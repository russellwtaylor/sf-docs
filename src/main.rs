use sfdoc::{
    aura_parser, cache, cli, config, custom_metadata_parser, doc_client, flexipage_parser,
    flow_parser, gemini, lwc_parser, object_parser, openai_compat, parser, providers, renderer,
    scanner, validation_rule_parser,
};
use sfdoc::{trigger_parser, types};

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;

use cli::{Cli, Commands, MetadataType, OutputFormat};
use doc_client::DocClient;

use config::{delete_api_key, has_stored_key, resolve_api_key, save_api_key};
use gemini::GeminiClient;
use openai_compat::OpenAiCompatClient;
use providers::Provider;
use scanner::{
    ApexScanner, AuraScanner, CustomMetadataScanner, FileScanner, FlexiPageScanner, FlowScanner,
    LwcScanner, ObjectScanner, TriggerScanner, ValidationRuleScanner,
};
use types::{
    AllNames, AuraDocumentation, ClassDocumentation, FlexiPageDocumentation, FlowDocumentation,
    LwcDocumentation, ObjectDocumentation, TriggerDocumentation, ValidationRuleDocumentation,
};

// Prompt modules for building AI prompts per metadata type.
use sfdoc::aura_prompt::{build_aura_prompt, AURA_SYSTEM_PROMPT};
use sfdoc::flexipage_prompt::{build_flexipage_prompt, FLEXIPAGE_SYSTEM_PROMPT};
use sfdoc::flow_prompt::{build_flow_prompt, FLOW_SYSTEM_PROMPT};
use sfdoc::lwc_prompt::{build_lwc_prompt, LWC_SYSTEM_PROMPT};
use sfdoc::object_prompt::{build_object_prompt, OBJECT_SYSTEM_PROMPT};
use sfdoc::prompt::{build_prompt, SYSTEM_PROMPT};
use sfdoc::trigger_prompt::{build_trigger_prompt, TRIGGER_SYSTEM_PROMPT};
use sfdoc::validation_rule_prompt::{build_validation_rule_prompt, VALIDATION_RULE_SYSTEM_PROMPT};

/// Filters parallel file/metadata vectors, keeping only entries where the
/// metadata's tags match the CLI `--tag` filter.
fn filter_by_tags<M, F>(
    files: Vec<types::SourceFile>,
    meta: Vec<M>,
    get_tags: F,
    args: &cli::GenerateArgs,
) -> (Vec<types::SourceFile>, Vec<M>)
where
    F: Fn(&M) -> &[String],
{
    if args.tags.is_empty() {
        return (files, meta);
    }
    let mut kept_files = Vec::new();
    let mut kept_meta = Vec::new();
    for (f, m) in files.into_iter().zip(meta.into_iter()) {
        if args.tag_matches(get_tags(&m)) {
            kept_files.push(f);
            kept_meta.push(m);
        }
    }
    (kept_files, kept_meta)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Update(args) => {
            sfdoc::update::run_update(&args).await?;
        }
        Commands::Auth(args) => {
            run_auth(&args.provider)?;
        }
        Commands::Status => {
            run_status();
        }
        Commands::Generate(args) => {
            let provider = &args.provider;
            // Arc<str> so task closures get a cheap pointer copy instead of a String clone.
            let model: Arc<str> =
                Arc::from(args.model.as_deref().unwrap_or(provider.default_model()));

            // Default output directory depends on format: docs/ for Markdown, site/ for HTML.
            let output_dir = args.output.clone().unwrap_or_else(|| {
                if args.format == OutputFormat::Html {
                    std::path::PathBuf::from("site")
                } else {
                    std::path::PathBuf::from("docs")
                }
            });

            if args.verbose {
                eprintln!("Provider:    {}", provider.display_name());
                eprintln!("Model:       {model}");
                eprintln!("Source dir:  {}", args.source_dir.display());
                eprintln!("Output dir:  {}", output_dir.display());
                eprintln!("Concurrency: {}", args.concurrency);
                if !args.types.is_empty() {
                    let names: Vec<&str> = args.types.iter().map(|t| t.cli_name()).collect();
                    eprintln!("Types:       {}", names.join(", "));
                }
                if let Some(ref pattern) = args.name_filter {
                    eprintln!("Name filter: {}", pattern);
                }
                if !args.tags.is_empty() {
                    eprintln!("Tags:        {}", args.tags.join(", "));
                }
            }

            // Scan for supported source types, skipping those excluded by --type.
            let scan = |enabled, scanner: &dyn FileScanner, label: &str| -> Result<Vec<_>> {
                if enabled {
                    scanner.scan(&args.source_dir).with_context(|| {
                        format!("Failed to scan {label} in {}", args.source_dir.display())
                    })
                } else {
                    Ok(Vec::new())
                }
            };

            let files = scan(
                args.type_enabled(MetadataType::Apex),
                &ApexScanner,
                "Apex classes",
            )?;
            let trigger_files = scan(
                args.type_enabled(MetadataType::Triggers),
                &TriggerScanner,
                "triggers",
            )?;
            let flow_files = scan(
                args.type_enabled(MetadataType::Flows),
                &FlowScanner,
                "flows",
            )?;
            let vr_files = scan(
                args.type_enabled(MetadataType::ValidationRules),
                &ValidationRuleScanner,
                "validation rules",
            )?;
            let object_files = scan(
                args.type_enabled(MetadataType::Objects),
                &ObjectScanner,
                "objects",
            )?;
            let lwc_files = scan(args.type_enabled(MetadataType::Lwc), &LwcScanner, "LWC")?;
            let flexipage_files = scan(
                args.type_enabled(MetadataType::Flexipages),
                &FlexiPageScanner,
                "FlexiPages",
            )?;
            let custom_metadata_files = scan(
                args.type_enabled(MetadataType::CustomMetadata),
                &CustomMetadataScanner,
                "Custom Metadata",
            )?;
            let aura_files = scan(
                args.type_enabled(MetadataType::Aura),
                &AuraScanner,
                "Aura components",
            )?;

            // Apply --name-filter: drop files whose logical name doesn't match the glob.
            let name_filter = |files: Vec<types::SourceFile>| -> Vec<types::SourceFile> {
                if args.name_filter.is_none() {
                    return files;
                }
                files
                    .into_iter()
                    .filter(|f| args.name_matches(&f.filename))
                    .collect()
            };
            let files = name_filter(files);
            let trigger_files = name_filter(trigger_files);
            let flow_files = name_filter(flow_files);
            let vr_files = name_filter(vr_files);
            let object_files = name_filter(object_files);
            let lwc_files = name_filter(lwc_files);
            let flexipage_files = name_filter(flexipage_files);
            let custom_metadata_files = name_filter(custom_metadata_files);
            let aura_files = name_filter(aura_files);

            // Require at least one file of any enabled type.
            if files.is_empty()
                && trigger_files.is_empty()
                && flow_files.is_empty()
                && vr_files.is_empty()
                && object_files.is_empty()
                && lwc_files.is_empty()
                && flexipage_files.is_empty()
                && custom_metadata_files.is_empty()
                && aura_files.is_empty()
            {
                if args.types.is_empty() {
                    anyhow::bail!(
                        "No supported source files found in {} (expected .cls, .trigger, .flow-meta.xml, .validationRule-meta.xml, .object-meta.xml, .js-meta.xml, .flexipage-meta.xml, .md-meta.xml, or .cmp)",
                        args.source_dir.display()
                    );
                } else {
                    let names: Vec<&str> = args.types.iter().map(|t| t.cli_name()).collect();
                    anyhow::bail!(
                        "No source files found in {} for the selected types: {}",
                        args.source_dir.display(),
                        names.join(", ")
                    );
                }
            }

            if !files.is_empty() {
                println!("Found {} Apex class file(s)", files.len());
            }
            if args.verbose {
                for f in &files {
                    eprintln!("  {}", f.path.display());
                }
            }
            if !trigger_files.is_empty() {
                println!("Found {} Apex trigger file(s)", trigger_files.len());
            }
            if !flow_files.is_empty() {
                println!("Found {} Flow file(s)", flow_files.len());
            }
            if !vr_files.is_empty() {
                println!("Found {} Validation Rule file(s)", vr_files.len());
            }
            if !object_files.is_empty() {
                println!("Found {} Object file(s)", object_files.len());
            }
            if !lwc_files.is_empty() {
                println!("Found {} LWC component(s)", lwc_files.len());
            }
            if !flexipage_files.is_empty() {
                println!("Found {} Lightning Page(s)", flexipage_files.len());
            }
            if !custom_metadata_files.is_empty() {
                println!(
                    "Found {} Custom Metadata record(s)",
                    custom_metadata_files.len()
                );
            }
            if !aura_files.is_empty() {
                println!("Found {} Aura component(s)", aura_files.len());
            }

            // Parse classes, triggers, and flows in parallel using rayon.
            let class_meta: Vec<_> = files
                .par_iter()
                .map(|f| parser::parse_apex_class(&f.raw_source))
                .collect::<Result<_>>()?;
            let trigger_meta: Vec<_> = trigger_files
                .par_iter()
                .map(|f| trigger_parser::parse_apex_trigger(&f.raw_source))
                .collect::<Result<_>>()?;
            let flow_meta: Vec<_> = flow_files
                .par_iter()
                .map(|f| {
                    let api_name = f
                        .filename
                        .strip_suffix(".flow-meta.xml")
                        .unwrap_or(&f.filename);
                    flow_parser::parse_flow(api_name, &f.raw_source)
                })
                .collect::<Result<_>>()?;
            let vr_meta: Vec<_> = vr_files
                .par_iter()
                .map(|f| validation_rule_parser::parse_validation_rule(&f.path, &f.raw_source))
                .collect::<Result<_>>()?;
            let object_meta: Vec<_> = object_files
                .par_iter()
                .map(|f| object_parser::parse_object(&f.path, &f.raw_source))
                .collect::<Result<_>>()?;
            let lwc_meta: Vec<_> = lwc_files
                .par_iter()
                .map(|f| lwc_parser::parse_lwc(&f.path, &f.raw_source))
                .collect::<Result<_>>()?;
            let flexipage_meta: Vec<_> = flexipage_files
                .par_iter()
                .map(|f| {
                    let api_name = f
                        .filename
                        .strip_suffix(".flexipage-meta.xml")
                        .unwrap_or(&f.filename);
                    flexipage_parser::parse_flexipage(api_name, &f.raw_source)
                })
                .collect::<Result<_>>()?;
            let custom_metadata_records: Vec<_> = custom_metadata_files
                .par_iter()
                .map(|f| {
                    custom_metadata_parser::parse_custom_metadata_record(&f.path, &f.raw_source)
                })
                .collect::<Result<_>>()?;
            let aura_meta: Vec<_> = aura_files
                .par_iter()
                .map(|f| aura_parser::parse_aura(&f.path, &f.raw_source))
                .collect::<Result<_>>()?;

            // Apply --tag filter post-parse, pre-AI.
            let (files, class_meta) = filter_by_tags(files, class_meta, |m| &m.tags, &args);
            let (trigger_files, trigger_meta) =
                filter_by_tags(trigger_files, trigger_meta, |m| &m.tags, &args);

            // When --tag is active, exclude non-taggable metadata types entirely.
            let (flow_files, flow_meta) = if args.tags.is_empty() {
                (flow_files, flow_meta)
            } else {
                (Vec::new(), Vec::new())
            };
            let (vr_files, vr_meta) = if args.tags.is_empty() {
                (vr_files, vr_meta)
            } else {
                (Vec::new(), Vec::new())
            };
            let (object_files, object_meta) = if args.tags.is_empty() {
                (object_files, object_meta)
            } else {
                (Vec::new(), Vec::new())
            };
            let (lwc_files, lwc_meta) = if args.tags.is_empty() {
                (lwc_files, lwc_meta)
            } else {
                (Vec::new(), Vec::new())
            };
            let (flexipage_files, flexipage_meta) = if args.tags.is_empty() {
                (flexipage_files, flexipage_meta)
            } else {
                (Vec::new(), Vec::new())
            };
            let custom_metadata_records: Vec<types::CustomMetadataRecord> = if args.tags.is_empty()
            {
                custom_metadata_records
            } else {
                Vec::new()
            };
            let (aura_files, aura_meta) = if args.tags.is_empty() {
                (aura_files, aura_meta)
            } else {
                (Vec::new(), Vec::new())
            };

            // Wrap in Arc so task closures share the data without cloning raw_source.
            let files = Arc::new(files);
            let class_meta = Arc::new(class_meta);
            let trigger_files = Arc::new(trigger_files);
            let trigger_meta = Arc::new(trigger_meta);
            let flow_files = Arc::new(flow_files);
            let flow_meta = Arc::new(flow_meta);
            let vr_files = Arc::new(vr_files);
            let vr_meta = Arc::new(vr_meta);
            let object_files = Arc::new(object_files);
            let object_meta = Arc::new(object_meta);
            let lwc_files = Arc::new(lwc_files);
            let lwc_meta = Arc::new(lwc_meta);
            let flexipage_files = Arc::new(flexipage_files);
            let flexipage_meta = Arc::new(flexipage_meta);
            let aura_files = Arc::new(aura_files);
            let aura_meta = Arc::new(aura_meta);

            // Build interface_implementors map
            let mut interface_implementors: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for meta in class_meta.iter() {
                for iface in &meta.implements {
                    interface_implementors
                        .entry(iface.clone())
                        .or_default()
                        .push(meta.class_name.clone());
                }
            }

            // Group custom metadata records by type
            let mut cm_by_type: std::collections::BTreeMap<
                String,
                Vec<types::CustomMetadataRecord>,
            > = std::collections::BTreeMap::new();
            for record in custom_metadata_records {
                cm_by_type
                    .entry(record.type_name.clone())
                    .or_default()
                    .push(record);
            }

            // Shared cross-linking index
            let all_names = Arc::new(AllNames {
                class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
                trigger_names: trigger_meta
                    .iter()
                    .map(|m| m.trigger_name.clone())
                    .collect(),
                flow_names: flow_meta.iter().map(|m| m.api_name.clone()).collect(),
                validation_rule_names: vr_meta.iter().map(|m| m.rule_name.clone()).collect(),
                object_names: object_meta.iter().map(|m| m.object_name.clone()).collect(),
                lwc_names: lwc_meta.iter().map(|m| m.component_name.clone()).collect(),
                flexipage_names: flexipage_meta.iter().map(|m| m.api_name.clone()).collect(),
                aura_names: aura_meta.iter().map(|m| m.component_name.clone()).collect(),
                custom_metadata_type_names: cm_by_type.keys().cloned().collect(),
                interface_implementors,
            });

            // Load incremental build cache (empty if --force or first run)
            let mut cache = if args.force {
                cache::Cache::default()
            } else {
                cache::Cache::load(&output_dir)
            };

            // Hash every source file
            let class_hashes: Vec<String> = files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let trigger_hashes: Vec<String> = trigger_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let flow_hashes: Vec<String> = flow_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let vr_hashes: Vec<String> = vr_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let object_hashes: Vec<String> = object_files
                .par_iter()
                .map(|f| {
                    let mut combined = f.raw_source.clone();
                    if let Some(fields_dir) = f.path.parent().map(|p| p.join("fields")) {
                        if let Ok(entries) = std::fs::read_dir(&fields_dir) {
                            let mut field_contents: Vec<(String, String)> = entries
                                .filter_map(|e| e.ok())
                                .filter(|e| {
                                    e.file_name()
                                        .to_str()
                                        .is_some_and(|n| n.ends_with(".field-meta.xml"))
                                })
                                .filter_map(|e| {
                                    let path = e.path();
                                    let name = path.file_name()?.to_str()?.to_string();
                                    let content = std::fs::read_to_string(&path).ok()?;
                                    Some((name, content))
                                })
                                .collect();
                            field_contents.sort_by(|a, b| a.0.cmp(&b.0));
                            for (_, content) in field_contents {
                                combined.push_str(&content);
                            }
                        }
                    }
                    cache::hash_source(&combined)
                })
                .collect();
            let lwc_hashes: Vec<String> = lwc_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let flexipage_hashes: Vec<String> = flexipage_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let aura_hashes: Vec<String> = aura_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();

            // Partition into cached vs. needs-API
            let mut class_work: Vec<usize> = Vec::new();
            let mut class_docs: Vec<Option<ClassDocumentation>> = vec![None; files.len()];
            for (i, (f, h)) in files.iter().zip(class_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    class_docs[i] = Some(e.documentation.clone());
                } else {
                    class_work.push(i);
                }
            }

            let mut trigger_work: Vec<usize> = Vec::new();
            let mut trigger_docs: Vec<Option<TriggerDocumentation>> =
                vec![None; trigger_files.len()];
            for (i, (f, h)) in trigger_files.iter().zip(trigger_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_trigger_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    trigger_docs[i] = Some(e.documentation.clone());
                } else {
                    trigger_work.push(i);
                }
            }

            let mut flow_work: Vec<usize> = Vec::new();
            let mut flow_docs: Vec<Option<FlowDocumentation>> = vec![None; flow_files.len()];
            for (i, (f, h)) in flow_files.iter().zip(flow_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_flow_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    flow_docs[i] = Some(e.documentation.clone());
                } else {
                    flow_work.push(i);
                }
            }

            let mut vr_work: Vec<usize> = Vec::new();
            let mut vr_docs: Vec<Option<ValidationRuleDocumentation>> = vec![None; vr_files.len()];
            for (i, (f, h)) in vr_files.iter().zip(vr_hashes.iter()).enumerate() {
                if let Some(e) =
                    cache.get_validation_rule_if_fresh(&f.path.to_string_lossy(), h, &model)
                {
                    vr_docs[i] = Some(e.documentation.clone());
                } else {
                    vr_work.push(i);
                }
            }

            let mut object_work: Vec<usize> = Vec::new();
            let mut object_docs: Vec<Option<ObjectDocumentation>> = vec![None; object_files.len()];
            for (i, (f, h)) in object_files.iter().zip(object_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_object_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    object_docs[i] = Some(e.documentation.clone());
                } else {
                    object_work.push(i);
                }
            }

            let mut lwc_work: Vec<usize> = Vec::new();
            let mut lwc_docs: Vec<Option<LwcDocumentation>> = vec![None; lwc_files.len()];
            for (i, (f, h)) in lwc_files.iter().zip(lwc_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_lwc_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    lwc_docs[i] = Some(e.documentation.clone());
                } else {
                    lwc_work.push(i);
                }
            }

            let mut flexipage_work: Vec<usize> = Vec::new();
            let mut flexipage_docs: Vec<Option<FlexiPageDocumentation>> =
                vec![None; flexipage_files.len()];
            for (i, (f, h)) in flexipage_files
                .iter()
                .zip(flexipage_hashes.iter())
                .enumerate()
            {
                if let Some(e) = cache.get_flexipage_if_fresh(&f.path.to_string_lossy(), h, &model)
                {
                    flexipage_docs[i] = Some(e.documentation.clone());
                } else {
                    flexipage_work.push(i);
                }
            }

            let mut aura_work: Vec<usize> = Vec::new();
            let mut aura_docs: Vec<Option<AuraDocumentation>> = vec![None; aura_files.len()];
            for (i, (f, h)) in aura_files.iter().zip(aura_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_aura_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    aura_docs[i] = Some(e.documentation.clone());
                } else {
                    aura_work.push(i);
                }
            }

            let skipped = (files.len() - class_work.len())
                + (trigger_files.len() - trigger_work.len())
                + (flow_files.len() - flow_work.len())
                + (vr_files.len() - vr_work.len())
                + (object_files.len() - object_work.len())
                + (lwc_files.len() - lwc_work.len())
                + (flexipage_files.len() - flexipage_work.len())
                + (aura_files.len() - aura_work.len());
            if skipped > 0 {
                println!("{skipped} file(s) up-to-date — skipping API calls");
            }

            if !class_work.is_empty()
                || !trigger_work.is_empty()
                || !flow_work.is_empty()
                || !vr_work.is_empty()
                || !object_work.is_empty()
                || !lwc_work.is_empty()
                || !flexipage_work.is_empty()
                || !aura_work.is_empty()
            {
                let api_key = resolve_api_key(provider)?;
                let client: Arc<dyn DocClient> = match provider {
                    Provider::Gemini => Arc::new(GeminiClient::new(
                        api_key,
                        &model,
                        args.concurrency,
                        args.rpm,
                    )?),
                    _ => Arc::new(OpenAiCompatClient::new(
                        api_key,
                        &model,
                        provider
                            .base_url()
                            .expect("non-Gemini provider must have a base URL"),
                        args.concurrency,
                        provider.display_name(),
                        args.rpm,
                    )?),
                };

                let total_work = (class_work.len()
                    + trigger_work.len()
                    + flow_work.len()
                    + vr_work.len()
                    + object_work.len()
                    + lwc_work.len()
                    + flexipage_work.len()
                    + aura_work.len()) as u64;
                let pb = Arc::new(ProgressBar::new(total_work));
                if let Ok(style) = ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                ) {
                    pb.set_style(style.progress_chars("#>-"));
                }

                // Unified work queue: classes and triggers race for the same concurrency
                // slots, so the semaphore is never underutilised when one type dominates.
                enum WorkResult {
                    Class(usize, ClassDocumentation),
                    Trigger(usize, TriggerDocumentation),
                    Flow(usize, FlowDocumentation),
                    ValidationRule(usize, ValidationRuleDocumentation),
                    Object(usize, ObjectDocumentation),
                    Lwc(usize, LwcDocumentation),
                    FlexiPage(usize, FlexiPageDocumentation),
                    Aura(usize, AuraDocumentation),
                }

                let mut tasks: JoinSet<Result<WorkResult>> = JoinSet::new();
                let mut failures: Vec<String> = Vec::new();

                for &idx in &class_work {
                    let client = Arc::clone(&client);
                    let files = Arc::clone(&files);
                    let class_meta = Arc::clone(&class_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: ClassDocumentation = doc_client::document(
                            client.as_ref(),
                            SYSTEM_PROMPT,
                            &build_prompt(&files[idx], &class_meta[idx]),
                            &class_meta[idx].class_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Class(idx, doc))
                    });
                }

                for &idx in &trigger_work {
                    let client = Arc::clone(&client);
                    let trigger_files = Arc::clone(&trigger_files);
                    let trigger_meta = Arc::clone(&trigger_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: TriggerDocumentation = doc_client::document(
                            client.as_ref(),
                            TRIGGER_SYSTEM_PROMPT,
                            &build_trigger_prompt(&trigger_files[idx], &trigger_meta[idx]),
                            &trigger_meta[idx].trigger_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Trigger(idx, doc))
                    });
                }

                for &idx in &flow_work {
                    let client = Arc::clone(&client);
                    let flow_files = Arc::clone(&flow_files);
                    let flow_meta = Arc::clone(&flow_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: FlowDocumentation = doc_client::document(
                            client.as_ref(),
                            FLOW_SYSTEM_PROMPT,
                            &build_flow_prompt(&flow_files[idx], &flow_meta[idx]),
                            &flow_meta[idx].api_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Flow(idx, doc))
                    });
                }

                for &idx in &vr_work {
                    let client = Arc::clone(&client);
                    let vr_files = Arc::clone(&vr_files);
                    let vr_meta = Arc::clone(&vr_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: ValidationRuleDocumentation = doc_client::document(
                            client.as_ref(),
                            VALIDATION_RULE_SYSTEM_PROMPT,
                            &build_validation_rule_prompt(&vr_files[idx], &vr_meta[idx]),
                            &vr_meta[idx].rule_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::ValidationRule(idx, doc))
                    });
                }

                for &idx in &object_work {
                    let client = Arc::clone(&client);
                    let object_files = Arc::clone(&object_files);
                    let object_meta = Arc::clone(&object_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: ObjectDocumentation = doc_client::document(
                            client.as_ref(),
                            OBJECT_SYSTEM_PROMPT,
                            &build_object_prompt(&object_files[idx], &object_meta[idx]),
                            &object_meta[idx].object_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Object(idx, doc))
                    });
                }

                for &idx in &lwc_work {
                    let client = Arc::clone(&client);
                    let lwc_files = Arc::clone(&lwc_files);
                    let lwc_meta = Arc::clone(&lwc_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: LwcDocumentation = doc_client::document(
                            client.as_ref(),
                            LWC_SYSTEM_PROMPT,
                            &build_lwc_prompt(&lwc_files[idx], &lwc_meta[idx]),
                            &lwc_meta[idx].component_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Lwc(idx, doc))
                    });
                }

                for &idx in &flexipage_work {
                    let client = Arc::clone(&client);
                    let flexipage_files = Arc::clone(&flexipage_files);
                    let flexipage_meta = Arc::clone(&flexipage_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: FlexiPageDocumentation = doc_client::document(
                            client.as_ref(),
                            FLEXIPAGE_SYSTEM_PROMPT,
                            &build_flexipage_prompt(&flexipage_files[idx], &flexipage_meta[idx]),
                            &flexipage_meta[idx].api_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::FlexiPage(idx, doc))
                    });
                }

                for &idx in &aura_work {
                    let client = Arc::clone(&client);
                    let aura_files = Arc::clone(&aura_files);
                    let aura_meta = Arc::clone(&aura_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc: AuraDocumentation = doc_client::document(
                            client.as_ref(),
                            AURA_SYSTEM_PROMPT,
                            &build_aura_prompt(&aura_files[idx], &aura_meta[idx]),
                            &aura_meta[idx].component_name,
                        )
                        .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Aura(idx, doc))
                    });
                }

                // Collect results as they complete (not in spawn order).
                while let Some(res) = tasks.join_next().await {
                    match res {
                        Err(join_err) => {
                            failures.push(format!("task panicked: {join_err}"));
                        }
                        Ok(Err(api_err)) => {
                            failures.push(format!("{api_err:#}"));
                            pb.inc(1);
                        }
                        Ok(Ok(work_result)) => match work_result {
                            WorkResult::Class(idx, doc) => {
                                let key = files[idx].path.to_string_lossy().into_owned();
                                cache.update(key, class_hashes[idx].clone(), &model, doc.clone());
                                class_docs[idx] = Some(doc);
                            }
                            WorkResult::Trigger(idx, doc) => {
                                let key = trigger_files[idx].path.to_string_lossy().into_owned();
                                cache.update_trigger(
                                    key,
                                    trigger_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                trigger_docs[idx] = Some(doc);
                            }
                            WorkResult::Flow(idx, doc) => {
                                let key = flow_files[idx].path.to_string_lossy().into_owned();
                                cache.update_flow(
                                    key,
                                    flow_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                flow_docs[idx] = Some(doc);
                            }
                            WorkResult::ValidationRule(idx, doc) => {
                                let key = vr_files[idx].path.to_string_lossy().into_owned();
                                cache.update_validation_rule(
                                    key,
                                    vr_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                vr_docs[idx] = Some(doc);
                            }
                            WorkResult::Object(idx, doc) => {
                                let key = object_files[idx].path.to_string_lossy().into_owned();
                                cache.update_object(
                                    key,
                                    object_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                object_docs[idx] = Some(doc);
                            }
                            WorkResult::Lwc(idx, doc) => {
                                let key = lwc_files[idx].path.to_string_lossy().into_owned();
                                cache.update_lwc(key, lwc_hashes[idx].clone(), &model, doc.clone());
                                lwc_docs[idx] = Some(doc);
                            }
                            WorkResult::FlexiPage(idx, doc) => {
                                let key = flexipage_files[idx].path.to_string_lossy().into_owned();
                                cache.update_flexipage(
                                    key,
                                    flexipage_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                flexipage_docs[idx] = Some(doc);
                            }
                            WorkResult::Aura(idx, doc) => {
                                let key = aura_files[idx].path.to_string_lossy().into_owned();
                                cache.update_aura(
                                    key,
                                    aura_hashes[idx].clone(),
                                    &model,
                                    doc.clone(),
                                );
                                aura_docs[idx] = Some(doc);
                            }
                        },
                    }
                }

                pb.finish_with_message("Done");

                if !failures.is_empty() {
                    eprintln!(
                        "{} file(s) failed to generate documentation:",
                        failures.len()
                    );
                    for f in &failures {
                        eprintln!("  - {f}");
                    }
                    // Save progress so the next run can skip successful files
                    cache.save(&output_dir)?;
                    anyhow::bail!("{} file(s) failed; partial cache saved", failures.len());
                }
            }

            // Build render contexts (tasks are all done; Arc::try_unwrap reclaims the Vecs).
            const ARC_ERR: &str =
                "Internal error: could not reclaim resources after documentation generation. Please retry.";
            let files = Arc::try_unwrap(files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let class_meta = Arc::try_unwrap(class_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let trigger_files =
                Arc::try_unwrap(trigger_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let trigger_meta =
                Arc::try_unwrap(trigger_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let flow_files = Arc::try_unwrap(flow_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let flow_meta = Arc::try_unwrap(flow_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let vr_files = Arc::try_unwrap(vr_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let vr_meta = Arc::try_unwrap(vr_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let object_files =
                Arc::try_unwrap(object_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let object_meta = Arc::try_unwrap(object_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let lwc_files = Arc::try_unwrap(lwc_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let lwc_meta = Arc::try_unwrap(lwc_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let flexipage_files =
                Arc::try_unwrap(flexipage_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let flexipage_meta =
                Arc::try_unwrap(flexipage_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let aura_files = Arc::try_unwrap(aura_files).map_err(|_| anyhow::anyhow!(ARC_ERR))?;
            let aura_meta = Arc::try_unwrap(aura_meta).map_err(|_| anyhow::anyhow!(ARC_ERR))?;

            let class_contexts: Vec<renderer::RenderContext> = files
                .into_iter()
                .zip(class_meta)
                .zip(class_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::RenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let trigger_contexts: Vec<renderer::TriggerRenderContext> = trigger_files
                .into_iter()
                .zip(trigger_meta)
                .zip(trigger_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::TriggerRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let flow_contexts: Vec<renderer::FlowRenderContext> = flow_files
                .into_iter()
                .zip(flow_meta)
                .zip(flow_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::FlowRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let vr_contexts: Vec<renderer::ValidationRuleRenderContext> = vr_files
                .into_iter()
                .zip(vr_meta)
                .zip(vr_docs)
                .filter_map(|((_, meta), doc)| {
                    doc.map(|d| renderer::ValidationRuleRenderContext {
                        folder: meta.object_name.clone(),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let object_contexts: Vec<renderer::ObjectRenderContext> = object_files
                .into_iter()
                .zip(object_meta)
                .zip(object_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::ObjectRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let lwc_contexts: Vec<renderer::LwcRenderContext> = lwc_files
                .into_iter()
                .zip(lwc_meta)
                .zip(lwc_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::LwcRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            let flexipage_contexts: Vec<renderer::FlexiPageRenderContext> = flexipage_files
                .into_iter()
                .zip(flexipage_meta)
                .zip(flexipage_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::FlexiPageRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            // Custom metadata contexts: group by type_name.
            // Custom metadata records are purely structural (field-value tables) — no AI
            // documentation is generated for them, so there is no cache look-up or API call.
            let custom_metadata_contexts: Vec<renderer::CustomMetadataRenderContext> = cm_by_type
                .into_iter()
                .map(
                    |(type_name, records)| renderer::CustomMetadataRenderContext {
                        type_name,
                        records,
                    },
                )
                .collect();

            let aura_contexts: Vec<renderer::AuraRenderContext> = aura_files
                .into_iter()
                .zip(aura_meta)
                .zip(aura_docs)
                .filter_map(|((file, meta), doc)| {
                    doc.map(|d| renderer::AuraRenderContext {
                        folder: compute_folder(&file.path, &args.source_dir),
                        metadata: meta,
                        documentation: d,
                        all_names: Arc::clone(&all_names),
                    })
                })
                .collect();

            // Render and write output
            let bundle = renderer::DocumentationBundle {
                classes: &class_contexts,
                triggers: &trigger_contexts,
                flows: &flow_contexts,
                validation_rules: &vr_contexts,
                objects: &object_contexts,
                lwc: &lwc_contexts,
                flexipages: &flexipage_contexts,
                custom_metadata: &custom_metadata_contexts,
                aura: &aura_contexts,
            };
            renderer::write_output(&output_dir, &args.format, &bundle)?;
            println!("Documentation written to {}", output_dir.display());

            // Persist the updated cache — only reached if all API calls succeeded
            cache.save(&output_dir)?;
        }
    }

    Ok(())
}

/// Re-export from types for backward compatibility within main.
fn compute_folder(file_path: &Path, source_dir: &Path) -> String {
    types::compute_folder(file_path, source_dir)
}

fn run_status() {
    println!("sfdoc {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("{:<10} {:<20} API Key", "Provider", "Name");
    println!("{}", "-".repeat(60));

    for provider in Provider::all() {
        let status = if !provider.requires_api_key() {
            "not required".to_string()
        } else if provider
            .env_var()
            .and_then(|v| std::env::var(v).ok())
            .is_some_and(|v| !v.is_empty())
        {
            format!("set (env: {})", provider.env_var().unwrap_or(""))
        } else if has_stored_key(provider) {
            "set (OS keychain)".to_string()
        } else {
            format!(
                "not configured — run `sfdoc auth --provider {}`",
                provider.cli_name()
            )
        };

        println!(
            "{:<10} {:<20} {}",
            provider.cli_name(),
            provider.display_name(),
            status
        );
    }
}

fn run_auth(provider: &Provider) -> Result<()> {
    if !provider.requires_api_key() {
        println!(
            "{} runs locally and does not require an API key.",
            provider.display_name()
        );
        println!("Make sure Ollama is running: https://ollama.com");
        return Ok(());
    }

    if has_stored_key(provider) {
        println!(
            "An API key for {} is already stored in your OS keychain.",
            provider.display_name()
        );
        print!("Overwrite it? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
        delete_api_key(provider)?;
    }

    let prompt = format!("Enter your {} API key: ", provider.display_name());
    let key = rpassword::prompt_password(&prompt).context("Failed to read API key")?;

    if key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty.");
    }

    save_api_key(provider, key.trim())?;

    println!(
        "API key for {} saved to your OS keychain.",
        provider.display_name()
    );
    println!(
        "You're all set — run `sfdoc generate --provider {}` to get started.",
        provider.cli_name()
    );

    Ok(())
}
