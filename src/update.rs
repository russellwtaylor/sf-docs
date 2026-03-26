use std::sync::Arc;

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::cache::{self, Cache};
use crate::cli::{MetadataType, OutputFormat, UpdateArgs};
use crate::config::resolve_api_key;
use crate::doc_client::{self, DocClient};
use crate::gemini::GeminiClient;
use crate::openai_compat::OpenAiCompatClient;
use crate::providers::Provider;
use crate::renderer;
use crate::scanner::{
    ApexScanner, AuraScanner, CustomMetadataScanner, FileScanner, FlexiPageScanner, FlowScanner,
    LwcScanner, ObjectScanner, TriggerScanner, ValidationRuleScanner,
};
use crate::types::{
    AllNames, AuraDocumentation, ClassDocumentation, FlexiPageDocumentation, FlowDocumentation,
    LwcDocumentation, ObjectDocumentation, SourceFile, TriggerDocumentation,
    ValidationRuleDocumentation,
};

// Parser modules
use crate::{
    aura_parser, flexipage_parser, flow_parser, lwc_parser, object_parser, parser, trigger_parser,
    validation_rule_parser,
};

// Prompt modules
use crate::aura_prompt::{build_aura_prompt, AURA_SYSTEM_PROMPT};
use crate::flexipage_prompt::{build_flexipage_prompt, FLEXIPAGE_SYSTEM_PROMPT};
use crate::flow_prompt::{build_flow_prompt, FLOW_SYSTEM_PROMPT};
use crate::lwc_prompt::{build_lwc_prompt, LWC_SYSTEM_PROMPT};
use crate::object_prompt::{build_object_prompt, OBJECT_SYSTEM_PROMPT};
use crate::prompt::{build_prompt, SYSTEM_PROMPT};
use crate::trigger_prompt::{build_trigger_prompt, TRIGGER_SYSTEM_PROMPT};
use crate::validation_rule_prompt::{build_validation_rule_prompt, VALIDATION_RULE_SYSTEM_PROMPT};

/// Known file extensions and their metadata types.
const EXTENSION_MAP: &[(&str, MetadataType)] = &[
    (".cls", MetadataType::Apex),
    (".trigger", MetadataType::Triggers),
    (".flow-meta.xml", MetadataType::Flows),
    (".validationRule-meta.xml", MetadataType::ValidationRules),
    (".object-meta.xml", MetadataType::Objects),
    (".js-meta.xml", MetadataType::Lwc),
    (".flexipage-meta.xml", MetadataType::Flexipages),
    (".md-meta.xml", MetadataType::CustomMetadata),
    (".cmp", MetadataType::Aura),
];

/// Result of resolving a target string to a concrete source file.
#[derive(Debug)]
pub struct ResolvedTarget {
    pub source_file: SourceFile,
    pub metadata_type: MetadataType,
}

/// Returns true if the target looks like a file path rather than a bare name.
fn is_path_target(target: &str) -> bool {
    if target.contains('/') || target.contains('\\') {
        return true;
    }
    EXTENSION_MAP.iter().any(|(ext, _)| target.ends_with(ext))
}

/// Determines the metadata type from a file path's extension.
fn metadata_type_from_path(path: &Path) -> Result<MetadataType> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Cannot extract filename from '{}'", path.display()))?;
    for (ext, mt) in EXTENSION_MAP {
        if name.ends_with(ext) {
            return Ok(*mt);
        }
    }
    bail!(
        "Cannot determine metadata type for '{}'. Supported extensions: {}",
        path.display(),
        EXTENSION_MAP
            .iter()
            .map(|(e, _)| *e)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Resolves a CLI target (path or name) to a source file and its metadata type.
pub fn resolve_target(target: &str, source_dir: &Path) -> Result<ResolvedTarget> {
    if is_path_target(target) {
        resolve_path_target(target)
    } else {
        resolve_name_target(target, source_dir)
    }
}

fn resolve_path_target(target: &str) -> Result<ResolvedTarget> {
    let path = PathBuf::from(target);
    if !path.exists() {
        bail!("File not found: '{}'", path.display());
    }
    let metadata_type = metadata_type_from_path(&path)?;
    let raw_source = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Cannot extract filename from '{}'", path.display()))?
        .to_string();
    Ok(ResolvedTarget {
        source_file: SourceFile {
            path,
            filename,
            raw_source,
        },
        metadata_type,
    })
}

fn resolve_name_target(name: &str, source_dir: &Path) -> Result<ResolvedTarget> {
    let scanners: Vec<(&dyn FileScanner, MetadataType, &str)> = vec![
        (&ApexScanner, MetadataType::Apex, ".cls"),
        (&TriggerScanner, MetadataType::Triggers, ".trigger"),
        (&FlowScanner, MetadataType::Flows, ".flow-meta.xml"),
        (
            &ValidationRuleScanner,
            MetadataType::ValidationRules,
            ".validationRule-meta.xml",
        ),
        (&ObjectScanner, MetadataType::Objects, ".object-meta.xml"),
        (&LwcScanner, MetadataType::Lwc, ".js-meta.xml"),
        (
            &FlexiPageScanner,
            MetadataType::Flexipages,
            ".flexipage-meta.xml",
        ),
        (
            &CustomMetadataScanner,
            MetadataType::CustomMetadata,
            ".md-meta.xml",
        ),
        (&AuraScanner, MetadataType::Aura, ".cmp"),
    ];

    let mut matches: Vec<(SourceFile, MetadataType)> = Vec::new();
    let mut all_names: Vec<String> = Vec::new();

    for (scanner, mt, suffix) in &scanners {
        if let Ok(files) = scanner.scan(source_dir) {
            for file in files {
                let stem = file.filename.strip_suffix(suffix).unwrap_or(&file.filename);
                all_names.push(stem.to_string());
                if stem.eq_ignore_ascii_case(name) {
                    matches.push((file, *mt));
                }
            }
        }
    }

    match matches.len() {
        0 => {
            let suggestions = find_similar_names(name, &all_names);
            let mut msg = format!(
                "No source file matching '{}' found in '{}'.",
                name,
                source_dir.display()
            );
            if !suggestions.is_empty() {
                msg.push_str(&format!(" Did you mean: {}?", suggestions.join(", ")));
            }
            bail!(msg);
        }
        1 => {
            let (file, mt) = matches.remove(0);
            Ok(ResolvedTarget {
                source_file: file,
                metadata_type: mt,
            })
        }
        _ => {
            let list: Vec<String> = matches
                .iter()
                .map(|(f, _)| format!("  {}", f.path.display()))
                .collect();
            bail!(
                "'{}' matches multiple files:\n{}\nSpecify the full path instead.",
                name,
                list.join("\n")
            );
        }
    }
}

/// Returns names within Levenshtein distance <= 2 of the target.
fn find_similar_names(target: &str, candidates: &[String]) -> Vec<String> {
    let target_lower = target.to_lowercase();
    candidates
        .iter()
        .filter(|c| levenshtein_distance(&target_lower, &c.to_lowercase()) <= 2)
        .cloned()
        .collect()
}

/// Simple Levenshtein distance implementation.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

/// Detect the output format from the existing output directory.
/// If `explicit` is `Some`, uses that. Otherwise checks for index.html / index.md.
/// Falls back to Markdown.
pub fn detect_output_format(output_dir: &Path, explicit: &Option<OutputFormat>) -> OutputFormat {
    if let Some(fmt) = explicit {
        return fmt.clone();
    }
    if output_dir.join("index.html").exists() {
        OutputFormat::Html
    } else {
        OutputFormat::Markdown
    }
}

/// Recomputes the folder (relative path from source_dir to file's parent).
fn compute_folder(file_path: &Path, source_dir: &Path) -> String {
    crate::types::compute_folder(file_path, source_dir)
}

/// Build an AllNames cross-linking index from the cache's stored documentation.
fn build_all_names_from_cache(cache: &Cache) -> AllNames {
    AllNames {
        class_names: cache
            .class_entries()
            .map(|(_, e)| e.documentation.class_name.clone())
            .collect(),
        trigger_names: cache
            .trigger_entries()
            .map(|(_, e)| e.documentation.trigger_name.clone())
            .collect(),
        flow_names: cache
            .flow_entries()
            .map(|(_, e)| e.documentation.api_name.clone())
            .collect(),
        validation_rule_names: cache
            .validation_rule_entries()
            .map(|(_, e)| e.documentation.rule_name.clone())
            .collect(),
        object_names: cache
            .object_entries()
            .map(|(_, e)| e.documentation.object_name.clone())
            .collect(),
        lwc_names: cache
            .lwc_entries()
            .map(|(_, e)| e.documentation.component_name.clone())
            .collect(),
        flexipage_names: cache
            .flexipage_entries()
            .map(|(_, e)| e.documentation.api_name.clone())
            .collect(),
        aura_names: cache
            .aura_entries()
            .map(|(_, e)| e.documentation.component_name.clone())
            .collect(),
        // Not available from cache alone — would require storing parsed metadata.
        // Index cross-linking still works for named entities; only interface→implementor
        // links and custom metadata type groupings are missing.
        custom_metadata_type_names: std::collections::HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    }
}

/// Context enum for rendering a single page of any metadata type.
enum SinglePageContext<'a> {
    Class(&'a renderer::RenderContext),
    Trigger(&'a renderer::TriggerRenderContext),
    Flow(&'a renderer::FlowRenderContext),
    ValidationRule(&'a renderer::ValidationRuleRenderContext),
    Object(&'a renderer::ObjectRenderContext),
    Lwc(&'a renderer::LwcRenderContext),
    FlexiPage(&'a renderer::FlexiPageRenderContext),
    Aura(&'a renderer::AuraRenderContext),
}

/// Write a single documentation page to the output directory.
fn write_single_page(
    output_dir: &Path,
    format: &OutputFormat,
    ctx: SinglePageContext,
) -> Result<()> {
    if *format == OutputFormat::Html {
        // For HTML, the full site is rebuilt in rebuild_index_from_cache
        return Ok(());
    }

    match ctx {
        SinglePageContext::Class(c) => {
            let dir = output_dir.join("classes");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_class_page(c);
            let name = renderer::sanitize_filename(&c.metadata.class_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: classes/{name}.md");
        }
        SinglePageContext::Trigger(c) => {
            let dir = output_dir.join("triggers");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_trigger_page(c);
            let name = renderer::sanitize_filename(&c.metadata.trigger_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: triggers/{name}.md");
        }
        SinglePageContext::Flow(c) => {
            let dir = output_dir.join("flows");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_flow_page(c);
            let name = renderer::sanitize_filename(&c.metadata.api_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: flows/{name}.md");
        }
        SinglePageContext::ValidationRule(c) => {
            let dir = output_dir.join("validation-rules");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_validation_rule_page(c);
            let name = renderer::sanitize_filename(&c.metadata.rule_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: validation-rules/{name}.md");
        }
        SinglePageContext::Object(c) => {
            let dir = output_dir.join("objects");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_object_page(c);
            let name = renderer::sanitize_filename(&c.metadata.object_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: objects/{name}.md");
        }
        SinglePageContext::Lwc(c) => {
            let dir = output_dir.join("lwc");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_lwc_page(c);
            let name = renderer::sanitize_filename(&c.metadata.component_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: lwc/{name}.md");
        }
        SinglePageContext::FlexiPage(c) => {
            let dir = output_dir.join("flexipages");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_flexipage_page(c);
            let name = renderer::sanitize_filename(&c.metadata.api_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: flexipages/{name}.md");
        }
        SinglePageContext::Aura(c) => {
            let dir = output_dir.join("aura");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_aura_page(c);
            let name = renderer::sanitize_filename(&c.metadata.component_name);
            std::fs::write(dir.join(format!("{name}.md")), page)?;
            println!("\u{2713} Documentation updated: aura/{name}.md");
        }
    }
    Ok(())
}

/// Rebuild the full index from cached documentation.
fn rebuild_index_from_cache(cache: &Cache, output_dir: &Path, format: &OutputFormat) -> Result<()> {
    let all_names = Arc::new(build_all_names_from_cache(cache));

    let class_contexts: Vec<renderer::RenderContext> = cache
        .class_entries()
        .map(|(_, e)| renderer::RenderContext {
            metadata: crate::types::ClassMetadata {
                class_name: e.documentation.class_name.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let trigger_contexts: Vec<renderer::TriggerRenderContext> = cache
        .trigger_entries()
        .map(|(_, e)| renderer::TriggerRenderContext {
            metadata: crate::types::TriggerMetadata {
                trigger_name: e.documentation.trigger_name.clone(),
                sobject: e.documentation.sobject.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let flow_contexts: Vec<renderer::FlowRenderContext> = cache
        .flow_entries()
        .map(|(_, e)| renderer::FlowRenderContext {
            metadata: crate::types::FlowMetadata {
                api_name: e.documentation.api_name.clone(),
                label: e.documentation.label.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let vr_contexts: Vec<renderer::ValidationRuleRenderContext> = cache
        .validation_rule_entries()
        .map(|(_, e)| renderer::ValidationRuleRenderContext {
            folder: e.documentation.object_name.clone(),
            metadata: crate::types::ValidationRuleMetadata {
                rule_name: e.documentation.rule_name.clone(),
                object_name: e.documentation.object_name.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
        })
        .collect();

    let object_contexts: Vec<renderer::ObjectRenderContext> = cache
        .object_entries()
        .map(|(_, e)| renderer::ObjectRenderContext {
            metadata: crate::types::ObjectMetadata {
                object_name: e.documentation.object_name.clone(),
                label: e.documentation.label.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let lwc_contexts: Vec<renderer::LwcRenderContext> = cache
        .lwc_entries()
        .map(|(_, e)| renderer::LwcRenderContext {
            metadata: crate::types::LwcMetadata {
                component_name: e.documentation.component_name.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let flexipage_contexts: Vec<renderer::FlexiPageRenderContext> = cache
        .flexipage_entries()
        .map(|(_, e)| renderer::FlexiPageRenderContext {
            metadata: crate::types::FlexiPageMetadata {
                api_name: e.documentation.api_name.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let aura_contexts: Vec<renderer::AuraRenderContext> = cache
        .aura_entries()
        .map(|(_, e)| renderer::AuraRenderContext {
            metadata: crate::types::AuraMetadata {
                component_name: e.documentation.component_name.clone(),
                ..Default::default()
            },
            documentation: e.documentation.clone(),
            all_names: Arc::clone(&all_names),
            folder: String::new(),
        })
        .collect();

    let bundle = renderer::DocumentationBundle {
        classes: &class_contexts,
        triggers: &trigger_contexts,
        flows: &flow_contexts,
        validation_rules: &vr_contexts,
        objects: &object_contexts,
        lwc: &lwc_contexts,
        flexipages: &flexipage_contexts,
        custom_metadata: &[],
        aura: &aura_contexts,
    };

    if *format == OutputFormat::Html {
        // The HTML renderer generates a full self-contained site. Rebuilding from
        // cache alone produces a degraded site (missing structural metadata like
        // methods, modifiers, interface implementors). Only regenerate the index;
        // individual pages from the last `sfdoc generate` remain on disk.
        eprintln!("Note: For a full HTML site rebuild with complete metadata, run 'sfdoc generate --format html'.");
        // Still regenerate the index page for updated summaries
        renderer::write_output(output_dir, format, &bundle)?;
    } else {
        // Markdown: only rewrite index.md — individual pages are written by write_single_page
        let index = renderer::render_index(&bundle);
        std::fs::write(output_dir.join("index.md"), index)?;
    }

    Ok(())
}

/// Entry point for `sfdoc update <target>`.
pub async fn run_update(args: &UpdateArgs) -> Result<()> {
    let provider = &args.provider;
    let model: String = args
        .model
        .clone()
        .unwrap_or_else(|| provider.default_model().to_string());

    // Determine output directory
    let output_dir = if let Some(ref out) = args.output {
        out.clone()
    } else {
        let site_dir = PathBuf::from("site");
        if site_dir.join(".sfdoc-cache.json").exists() {
            site_dir
        } else {
            PathBuf::from("docs")
        }
    };

    // Check that a prior generate has been run
    let cache_path = output_dir.join(".sfdoc-cache.json");
    if !cache_path.exists() {
        bail!(
            "No existing documentation found in '{}'. Run 'sfdoc generate' first, then use 'sfdoc update' to refresh individual files.",
            output_dir.display()
        );
    }

    let mut cache = Cache::load(&output_dir);
    let format = detect_output_format(&output_dir, &args.format);

    // Resolve the target to a source file + metadata type
    let resolved = resolve_target(&args.target, &args.source_dir)?;
    let source_file = resolved.source_file;
    let metadata_type = resolved.metadata_type;

    let type_label = metadata_type.cli_name();
    let display_name = EXTENSION_MAP
        .iter()
        .find_map(|(ext, _)| source_file.filename.strip_suffix(ext))
        .unwrap_or(&source_file.filename);

    // Custom metadata records are structural tables without AI generation.
    // They can't be meaningfully "updated" via this command.
    if metadata_type == MetadataType::CustomMetadata {
        bail!(
            "Custom metadata records don't use AI documentation. Use 'sfdoc generate' to rebuild all docs including custom metadata."
        );
    }

    println!(
        "Updating documentation for {} ({})...",
        display_name, type_label
    );

    if args.verbose {
        eprintln!("Resolved target: {}", source_file.path.display());
        eprintln!("Metadata type:   {}", type_label);
        eprintln!("Provider:        {}", provider.display_name());
        eprintln!("Model:           {}", model);
        eprintln!(
            "Format:          {}",
            if format == OutputFormat::Html {
                "html"
            } else {
                "markdown"
            }
        );
        eprintln!(
            "Format source:   {}",
            if args.format.is_some() {
                "explicit"
            } else {
                "auto-detected"
            }
        );
    }

    // Hash the source and check if it's unchanged
    let hash = cache::hash_source(&source_file.raw_source);
    if args.verbose {
        eprintln!("Source hash:     {}", hash);
    }

    // Create AI client (single concurrency, no rate limit)
    let api_key = resolve_api_key(provider)?;
    let client: Arc<dyn DocClient> = match provider {
        Provider::Gemini => Arc::new(GeminiClient::new(api_key, &model, 1, 0)?),
        _ => Arc::new(OpenAiCompatClient::new(
            api_key,
            &model,
            provider
                .base_url()
                .expect("non-Gemini provider must have a base URL"),
            1,
            provider.display_name(),
            0,
        )?),
    };

    let cache_key = source_file.path.to_string_lossy().into_owned();
    let source_dir = &args.source_dir;

    // Inform user if source hasn't changed since last build
    let is_unchanged = match metadata_type {
        MetadataType::Apex => cache.get_if_fresh(&cache_key, &hash, &model).is_some(),
        MetadataType::Triggers => cache
            .get_trigger_if_fresh(&cache_key, &hash, &model)
            .is_some(),
        MetadataType::Flows => cache.get_flow_if_fresh(&cache_key, &hash, &model).is_some(),
        MetadataType::ValidationRules => cache
            .get_validation_rule_if_fresh(&cache_key, &hash, &model)
            .is_some(),
        MetadataType::Objects => cache
            .get_object_if_fresh(&cache_key, &hash, &model)
            .is_some(),
        MetadataType::Lwc => cache.get_lwc_if_fresh(&cache_key, &hash, &model).is_some(),
        MetadataType::Flexipages => cache
            .get_flexipage_if_fresh(&cache_key, &hash, &model)
            .is_some(),
        MetadataType::Aura => cache.get_aura_if_fresh(&cache_key, &hash, &model).is_some(),
        MetadataType::CustomMetadata => false,
    };
    if is_unchanged {
        eprintln!("Note: Source is unchanged since last build. Re-generating anyway.");
    }

    match metadata_type {
        MetadataType::Apex => {
            let meta = parser::parse_apex_class(&source_file.raw_source)?;
            let doc: ClassDocumentation = doc_client::document(
                client.as_ref(),
                SYSTEM_PROMPT,
                &build_prompt(&source_file, &meta),
                &meta.class_name,
            )
            .await?;
            cache.update(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::RenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Class(&ctx))?;
        }
        MetadataType::Triggers => {
            let meta = trigger_parser::parse_apex_trigger(&source_file.raw_source)?;
            let doc: TriggerDocumentation = doc_client::document(
                client.as_ref(),
                TRIGGER_SYSTEM_PROMPT,
                &build_trigger_prompt(&source_file, &meta),
                &meta.trigger_name,
            )
            .await?;
            cache.update_trigger(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::TriggerRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Trigger(&ctx))?;
        }
        MetadataType::Flows => {
            let api_name = source_file
                .filename
                .strip_suffix(".flow-meta.xml")
                .unwrap_or(&source_file.filename);
            let meta = flow_parser::parse_flow(api_name, &source_file.raw_source)?;
            let doc: FlowDocumentation = doc_client::document(
                client.as_ref(),
                FLOW_SYSTEM_PROMPT,
                &build_flow_prompt(&source_file, &meta),
                &meta.api_name,
            )
            .await?;
            cache.update_flow(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::FlowRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Flow(&ctx))?;
        }
        MetadataType::ValidationRules => {
            let meta = validation_rule_parser::parse_validation_rule(
                &source_file.path,
                &source_file.raw_source,
            )?;
            let doc: ValidationRuleDocumentation = doc_client::document(
                client.as_ref(),
                VALIDATION_RULE_SYSTEM_PROMPT,
                &build_validation_rule_prompt(&source_file, &meta),
                &meta.rule_name,
            )
            .await?;
            cache.update_validation_rule(cache_key, hash, &model, doc.clone());
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::ValidationRuleRenderContext {
                folder: meta.object_name.clone(),
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
            };
            write_single_page(
                &output_dir,
                &format,
                SinglePageContext::ValidationRule(&ctx),
            )?;
        }
        MetadataType::Objects => {
            let meta = object_parser::parse_object(&source_file.path, &source_file.raw_source)?;
            let doc: ObjectDocumentation = doc_client::document(
                client.as_ref(),
                OBJECT_SYSTEM_PROMPT,
                &build_object_prompt(&source_file, &meta),
                &meta.object_name,
            )
            .await?;
            cache.update_object(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::ObjectRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Object(&ctx))?;
        }
        MetadataType::Lwc => {
            let meta = lwc_parser::parse_lwc(&source_file.path, &source_file.raw_source)?;
            let doc: LwcDocumentation = doc_client::document(
                client.as_ref(),
                LWC_SYSTEM_PROMPT,
                &build_lwc_prompt(&source_file, &meta),
                &meta.component_name,
            )
            .await?;
            cache.update_lwc(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::LwcRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Lwc(&ctx))?;
        }
        MetadataType::Flexipages => {
            let api_name = source_file
                .filename
                .strip_suffix(".flexipage-meta.xml")
                .unwrap_or(&source_file.filename);
            let meta = flexipage_parser::parse_flexipage(api_name, &source_file.raw_source)?;
            let doc: FlexiPageDocumentation = doc_client::document(
                client.as_ref(),
                FLEXIPAGE_SYSTEM_PROMPT,
                &build_flexipage_prompt(&source_file, &meta),
                &meta.api_name,
            )
            .await?;
            cache.update_flexipage(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::FlexiPageRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::FlexiPage(&ctx))?;
        }
        MetadataType::CustomMetadata => {
            // Unreachable — we bail early above for custom metadata.
            unreachable!("custom metadata is handled before AI client creation");
        }
        MetadataType::Aura => {
            let meta = aura_parser::parse_aura(&source_file.path, &source_file.raw_source)?;
            let doc: AuraDocumentation = doc_client::document(
                client.as_ref(),
                AURA_SYSTEM_PROMPT,
                &build_aura_prompt(&source_file, &meta),
                &meta.component_name,
            )
            .await?;
            cache.update_aura(cache_key, hash, &model, doc.clone());
            let folder = compute_folder(&source_file.path, source_dir);
            let all_names = build_all_names_from_cache(&cache);
            let ctx = renderer::AuraRenderContext {
                metadata: meta,
                documentation: doc,
                all_names: Arc::new(all_names),
                folder,
            };
            write_single_page(&output_dir, &format, SinglePageContext::Aura(&ctx))?;
        }
    }

    // Rebuild the full index
    rebuild_index_from_cache(&cache, &output_dir, &format)?;
    println!("\u{2713} Index regenerated");

    // Save cache
    cache.save(&output_dir)?;
    if args.verbose {
        eprintln!("Cache saved");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_path_with_slash() {
        assert!(is_path_target("src/classes/Foo.cls"));
    }

    #[test]
    fn is_path_with_extension() {
        assert!(is_path_target("Foo.cls"));
        assert!(is_path_target("MyFlow.flow-meta.xml"));
        assert!(is_path_target("AccTrig.trigger"));
    }

    #[test]
    fn bare_name_is_not_path() {
        assert!(!is_path_target("OrderService"));
        assert!(!is_path_target("MyFlow"));
    }

    #[test]
    fn metadata_type_from_cls() {
        assert_eq!(
            metadata_type_from_path(Path::new("Foo.cls")).unwrap(),
            MetadataType::Apex
        );
    }

    #[test]
    fn metadata_type_from_trigger() {
        assert_eq!(
            metadata_type_from_path(Path::new("Foo.trigger")).unwrap(),
            MetadataType::Triggers
        );
    }

    #[test]
    fn metadata_type_from_flow() {
        assert_eq!(
            metadata_type_from_path(Path::new("My_Flow.flow-meta.xml")).unwrap(),
            MetadataType::Flows
        );
    }

    #[test]
    fn metadata_type_unknown_extension() {
        assert!(metadata_type_from_path(Path::new("Foo.java")).is_err());
    }

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn levenshtein_one_edit() {
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "axc"), 1);
    }

    #[test]
    fn levenshtein_two_edits() {
        assert_eq!(levenshtein_distance("abc", "a"), 2);
    }

    #[test]
    fn find_similar_names_finds_close_matches() {
        let candidates = vec!["OrderService".to_string(), "AccountHelper".to_string()];
        let result = find_similar_names("OrderServce", &candidates);
        assert_eq!(result, vec!["OrderService".to_string()]);
    }

    #[test]
    fn find_similar_names_no_matches() {
        let candidates = vec!["OrderService".to_string()];
        let result = find_similar_names("CompletelyDifferent", &candidates);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_path_target_file_not_found() {
        let result = resolve_path_target("/nonexistent/path/Foo.cls");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn detect_format_html_when_index_html_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.html"), "<html></html>").unwrap();
        assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Html);
    }

    #[test]
    fn detect_format_markdown_when_index_md_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.md"), "# Index").unwrap();
        assert_eq!(
            detect_output_format(tmp.path(), &None),
            OutputFormat::Markdown
        );
    }

    #[test]
    fn detect_format_defaults_to_markdown() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            detect_output_format(tmp.path(), &None),
            OutputFormat::Markdown
        );
    }

    #[test]
    fn detect_format_explicit_override() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.md"), "# Index").unwrap();
        assert_eq!(
            detect_output_format(tmp.path(), &Some(OutputFormat::Html)),
            OutputFormat::Html
        );
    }
}
