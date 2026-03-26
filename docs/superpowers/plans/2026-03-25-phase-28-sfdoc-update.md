# Phase 28: `sfdoc update <file>` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `sfdoc update <target>` subcommand that re-documents a single file without a full rebuild.

**Architecture:** Extract reusable pipeline functions from `main.rs` into a new `update.rs` module. Add `UpdateArgs` to `cli.rs`. The update command resolves the target (path or name), determines metadata type, parses, calls AI, updates cache, renders the single page, and rebuilds the full index from cache. Both `generate` and `update` share the same parsing/prompting/rendering code paths.

**Tech Stack:** Rust, clap, tokio, anyhow, serde_json, sha2

---

### Task 1: Add `UpdateArgs` to CLI and wire up the subcommand

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write tests for CLI parsing of the update subcommand**

Add to the `#[cfg(test)] mod tests` block in `src/cli.rs`:

```rust
fn parse_update(args: &[&str]) -> UpdateArgs {
    let mut full = vec!["sfdoc", "update"];
    full.extend(args);
    let cli = Cli::try_parse_from(full).expect("CLI should parse");
    match cli.command {
        Commands::Update(u) => u,
        _ => panic!("expected Update command"),
    }
}

#[test]
fn update_requires_target() {
    let result = Cli::try_parse_from(["sfdoc", "update"]);
    assert!(result.is_err());
}

#[test]
fn update_parses_target_positional() {
    let args = parse_update(&["OrderService"]);
    assert_eq!(args.target, "OrderService");
}

#[test]
fn update_default_flags() {
    let args = parse_update(&["Foo"]);
    assert_eq!(args.source_dir, std::path::PathBuf::from("force-app/main/default"));
    assert!(args.output.is_none());
    assert_eq!(args.provider, crate::providers::Provider::Gemini);
    assert!(args.model.is_none());
    assert!(args.format.is_none());
    assert!(!args.verbose);
}

#[test]
fn update_accepts_all_flags() {
    let args = parse_update(&[
        "MyClass",
        "--source-dir", "src",
        "--output", "out",
        "--provider", "openai",
        "--model", "gpt-4o",
        "--format", "html",
        "--verbose",
    ]);
    assert_eq!(args.target, "MyClass");
    assert_eq!(args.source_dir, std::path::PathBuf::from("src"));
    assert_eq!(args.output, Some(std::path::PathBuf::from("out")));
    assert_eq!(args.format, Some(OutputFormat::Html));
    assert!(args.verbose);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli::tests -- update`
Expected: compilation errors — `UpdateArgs` and `Commands::Update` don't exist yet.

- [ ] **Step 3: Add `UpdateArgs` struct and `Commands::Update` variant**

In `src/cli.rs`, add the struct after `AuthArgs`:

```rust
#[derive(clap::Args, Debug)]
pub struct UpdateArgs {
    /// File path or name to re-document (e.g. 'OrderService' or 'force-app/.../OrderService.cls')
    pub target: String,

    /// Path to Apex source directory
    #[arg(long, default_value = "force-app/main/default")]
    pub source_dir: PathBuf,

    /// Output directory for generated files
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// AI provider to use for documentation generation
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,

    /// Model to use (defaults to the provider's recommended model if not set)
    #[arg(long)]
    pub model: Option<String>,

    /// Output format (auto-detected from existing output if not specified)
    #[arg(long)]
    pub format: Option<OutputFormat>,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}
```

Add the variant to `Commands`:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate documentation from Salesforce source files
    Generate(GenerateArgs),
    /// Re-document a single file without a full rebuild
    Update(UpdateArgs),
    /// Save an AI provider API key to the OS keychain
    Auth(AuthArgs),
    /// Show installation status and configuration
    Status,
}
```

- [ ] **Step 4: Add stub match arm in `main.rs`**

In the `match cli.command` block in `main.rs`, add:

```rust
Commands::Update(_args) => {
    anyhow::bail!("sfdoc update is not yet implemented");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib cli::tests -- update`
Expected: all 4 update tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add UpdateArgs CLI definition for sfdoc update subcommand"
```

---

### Task 2: Add `Cache` accessor methods for building `AllNames` from cache

The `update` command needs to rebuild the index from cache alone (it doesn't scan all files). We need methods to iterate over cache entries.

**Files:**
- Modify: `src/cache.rs`

- [ ] **Step 1: Write tests for cache accessor methods**

Add to `#[cfg(test)] mod tests` in `src/cache.rs`:

```rust
#[test]
fn class_entries_returns_all_class_docs() {
    let mut cache = Cache::default();
    let doc = ClassDocumentation {
        class_name: "Foo".to_string(),
        summary: "A foo.".to_string(),
        description: "".to_string(),
        methods: vec![],
        properties: vec![],
        usage_examples: vec![],
        relationships: vec![],
    };
    cache.update("Foo.cls".to_string(), "h1".to_string(), "m1", doc);
    let entries: Vec<_> = cache.class_entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1.documentation.class_name, "Foo");
}

#[test]
fn trigger_entries_returns_all_trigger_docs() {
    let mut cache = Cache::default();
    let doc = TriggerDocumentation {
        trigger_name: "AccTrig".to_string(),
        sobject: "Account".to_string(),
        summary: "".to_string(),
        description: "".to_string(),
        events: vec![],
        handler_classes: vec![],
        usage_notes: vec![],
        relationships: vec![],
    };
    cache.update_trigger("AccTrig.trigger".to_string(), "h1".to_string(), "m1", doc);
    let entries: Vec<_> = cache.trigger_entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1.documentation.trigger_name, "AccTrig");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cache::tests -- entries_returns`
Expected: compilation error — `class_entries()` and `trigger_entries()` don't exist.

- [ ] **Step 3: Add iterator accessor methods to `Cache`**

Add to the `impl Cache` block in `src/cache.rs`, after the `cache_accessors!` macro invocations:

```rust
/// Iterators over all cached entries, for rebuilding AllNames and the index.
pub fn class_entries(&self) -> impl Iterator<Item = (&String, &CacheEntry)> {
    self.entries.iter()
}

pub fn trigger_entries(&self) -> impl Iterator<Item = (&String, &TriggerCacheEntry)> {
    self.trigger_entries.iter()
}

pub fn flow_entries(&self) -> impl Iterator<Item = (&String, &FlowCacheEntry)> {
    self.flow_entries.iter()
}

pub fn validation_rule_entries(&self) -> impl Iterator<Item = (&String, &ValidationRuleCacheEntry)> {
    self.validation_rule_entries.iter()
}

pub fn object_entries(&self) -> impl Iterator<Item = (&String, &ObjectCacheEntry)> {
    self.object_entries.iter()
}

pub fn lwc_entries(&self) -> impl Iterator<Item = (&String, &LwcCacheEntry)> {
    self.lwc_entries.iter()
}

pub fn flexipage_entries(&self) -> impl Iterator<Item = (&String, &FlexiPageCacheEntry)> {
    self.flexipage_entries.iter()
}

pub fn aura_entries(&self) -> impl Iterator<Item = (&String, &AuraCacheEntry)> {
    self.aura_entries.iter()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib cache::tests -- entries_returns`
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/cache.rs
git commit -m "feat: add cache entry iterator methods for index rebuild"
```

---

### Task 3: Implement target resolution logic

**Files:**
- Create: `src/update.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write tests for target resolution**

Create `src/update.rs` with tests:

```rust
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::cli::MetadataType;
use crate::scanner::{
    ApexScanner, AuraScanner, CustomMetadataScanner, FileScanner, FlexiPageScanner, FlowScanner,
    LwcScanner, ObjectScanner, TriggerScanner, ValidationRuleScanner,
};
use crate::types::SourceFile;

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
        .unwrap_or("");
    for (ext, mt) in EXTENSION_MAP {
        if name.ends_with(ext) {
            return Ok(*mt);
        }
    }
    bail!(
        "Cannot determine metadata type for '{}'. Supported extensions: {}",
        path.display(),
        EXTENSION_MAP.iter().map(|(e, _)| *e).collect::<Vec<_>>().join(", ")
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
    let raw_source =
        std::fs::read_to_string(&path).with_context(|| format!("Failed to read '{}'", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
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
        (&ValidationRuleScanner, MetadataType::ValidationRules, ".validationRule-meta.xml"),
        (&ObjectScanner, MetadataType::Objects, ".object-meta.xml"),
        (&LwcScanner, MetadataType::Lwc, ".js-meta.xml"),
        (&FlexiPageScanner, MetadataType::Flexipages, ".flexipage-meta.xml"),
        (&CustomMetadataScanner, MetadataType::CustomMetadata, ".md-meta.xml"),
        (&AuraScanner, MetadataType::Aura, ".cmp"),
    ];

    let mut matches: Vec<(SourceFile, MetadataType)> = Vec::new();

    for (scanner, mt, suffix) in &scanners {
        if let Ok(files) = scanner.scan(source_dir) {
            for file in files {
                let stem = file.filename.strip_suffix(suffix).unwrap_or(&file.filename);
                if stem.eq_ignore_ascii_case(name) {
                    matches.push((file, *mt));
                }
            }
        }
    }

    match matches.len() {
        0 => {
            // Collect all known names for suggestions
            let mut all_names: Vec<String> = Vec::new();
            for (scanner, _, suffix) in &scanners {
                if let Ok(files) = scanner.scan(source_dir) {
                    for file in files {
                        let stem = file.filename.strip_suffix(suffix).unwrap_or(&file.filename);
                        all_names.push(stem.to_string());
                    }
                }
            }
            let suggestions = find_similar_names(name, &all_names);
            let mut msg = format!("No source file matching '{}' found in '{}'.", name, source_dir.display());
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
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
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
}
```

- [ ] **Step 2: Add `pub mod update;` to `src/lib.rs`**

Add the line to `src/lib.rs`:

```rust
pub mod update;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --lib update::tests`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/update.rs src/lib.rs
git commit -m "feat: add target resolution logic for sfdoc update"
```

---

### Task 4: Implement format auto-detection

**Files:**
- Modify: `src/update.rs`

- [ ] **Step 1: Write tests for format auto-detection**

Add to `mod tests` in `src/update.rs`:

```rust
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
    assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Markdown);
}

#[test]
fn detect_format_defaults_to_markdown() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Markdown);
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib update::tests::detect_format`
Expected: compilation error — `detect_output_format` doesn't exist.

- [ ] **Step 3: Implement `detect_output_format`**

Add to `src/update.rs`, before `mod tests`:

```rust
use crate::cli::OutputFormat;

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib update::tests::detect_format`
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/update.rs
git commit -m "feat: add format auto-detection for sfdoc update"
```

---

### Task 5: Implement the `run_update` function

This is the core orchestration that ties everything together: resolve target, parse, call AI, update cache, render page, rebuild index.

**Files:**
- Modify: `src/update.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add necessary imports and the `run_update` function to `src/update.rs`**

Add these imports at the top of `src/update.rs` (merging with existing imports):

```rust
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
use crate::types::{AllNames, SourceFile};

// Parser modules
use crate::{
    aura_parser, custom_metadata_parser, flexipage_parser, flow_parser, lwc_parser,
    object_parser, parser, trigger_parser, validation_rule_parser,
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
```

Then add the `run_update` function:

```rust
/// Entry point for `sfdoc update <target>`.
pub async fn run_update(args: &UpdateArgs) -> Result<()> {
    let provider = &args.provider;
    let model: String = args
        .model
        .clone()
        .unwrap_or_else(|| provider.default_model().to_string());

    // Determine output directory: explicit > format-based default.
    // We need to detect format first to know the default output dir,
    // but format detection needs the output dir. Break the cycle:
    // if --output is set, use it; otherwise try both defaults.
    let output_dir = if let Some(ref out) = args.output {
        out.clone()
    } else {
        // Check both default directories for an existing cache
        let docs_dir = PathBuf::from("docs");
        let site_dir = PathBuf::from("site");
        if site_dir.join(".sfdoc-cache.json").exists() {
            site_dir
        } else {
            docs_dir
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

    // Detect output format
    let format = detect_output_format(&output_dir, &args.format);

    // Resolve the target to a source file + metadata type
    let resolved = resolve_target(&args.target, &args.source_dir)?;
    let source_file = resolved.source_file;
    let metadata_type = resolved.metadata_type;

    let type_label = metadata_type.cli_name();
    let display_name = source_file
        .filename
        .strip_suffix(".cls")
        .or_else(|| source_file.filename.strip_suffix(".trigger"))
        .or_else(|| source_file.filename.strip_suffix(".flow-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".validationRule-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".object-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".js-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".flexipage-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".md-meta.xml"))
        .or_else(|| source_file.filename.strip_suffix(".cmp"))
        .unwrap_or(&source_file.filename);

    println!("Updating documentation for {} ({})...", display_name, type_label);

    if args.verbose {
        eprintln!("Resolved target: {}", source_file.path.display());
        eprintln!("Metadata type:   {}", type_label);
        eprintln!("Provider:        {}", provider.display_name());
        eprintln!("Model:           {}", model);
        eprintln!("Format:          {}", if format == OutputFormat::Html { "html" } else { "markdown" });
        eprintln!(
            "Format source:   {}",
            if args.format.is_some() { "explicit" } else { "auto-detected" }
        );
    }

    // Hash the source
    let hash = cache::hash_source(&source_file.raw_source);
    if args.verbose {
        eprintln!("Source hash:     {}", hash);
    }

    // Resolve API key and create client
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

    // Parse, generate docs, update cache, and render the page
    let cache_key = source_file.path.to_string_lossy().into_owned();
    let source_dir = &args.source_dir;

    match metadata_type {
        MetadataType::Apex => {
            let meta = parser::parse_apex_class(&source_file.raw_source)?;
            let doc = doc_client::document(
                client.as_ref(),
                SYSTEM_PROMPT,
                &build_prompt(&source_file, &meta),
                &meta.class_name,
            )
            .await?;
            cache.update(cache_key, hash, &model, doc.clone());
            // Render and write the page
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
            let doc = doc_client::document(
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
            let doc = doc_client::document(
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
            let doc = doc_client::document(
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
            write_single_page(&output_dir, &format, SinglePageContext::ValidationRule(&ctx))?;
        }
        MetadataType::Objects => {
            let meta =
                object_parser::parse_object(&source_file.path, &source_file.raw_source)?;
            let doc = doc_client::document(
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
            let doc = doc_client::document(
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
            let doc = doc_client::document(
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
            // Custom metadata records don't use AI — they're structural tables.
            // For update, we just re-parse and re-render.
            let _record = custom_metadata_parser::parse_custom_metadata_record(
                &source_file.path,
                &source_file.raw_source,
            )?;
            // Custom metadata doesn't have per-file cache entries or AI docs.
            // Just rebuild the index.
            eprintln!("Note: Custom metadata records don't use AI generation. Re-rendering index only.");
        }
        MetadataType::Aura => {
            let meta = aura_parser::parse_aura(&source_file.path, &source_file.raw_source)?;
            let doc = doc_client::document(
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

    // Rebuild the full index from cache
    rebuild_index_from_cache(&cache, &output_dir, &format)?;
    println!("\u{2713} Index regenerated");

    // Save cache
    cache.save(&output_dir)?;
    if args.verbose {
        eprintln!("Cache saved");
    }

    Ok(())
}
```

- [ ] **Step 2: Add helper functions used by `run_update`**

Add to `src/update.rs` before `run_update`:

```rust
/// Recomputes the folder (relative path from source_dir to file's parent).
fn compute_folder(file_path: &Path, source_dir: &Path) -> String {
    file_path
        .parent()
        .and_then(|p| p.strip_prefix(source_dir).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
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
        // For HTML, we need to rebuild the full site (the HTML renderer generates
        // the entire site with sidebar navigation). We'll handle this in the
        // index rebuild step instead.
        return Ok(());
    }

    match ctx {
        SinglePageContext::Class(c) => {
            let dir = output_dir.join("classes");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_class_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.class_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: classes/{}.md", renderer::sanitize_filename(&c.metadata.class_name));
        }
        SinglePageContext::Trigger(c) => {
            let dir = output_dir.join("triggers");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_trigger_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.trigger_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: triggers/{}.md", renderer::sanitize_filename(&c.metadata.trigger_name));
        }
        SinglePageContext::Flow(c) => {
            let dir = output_dir.join("flows");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_flow_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.api_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: flows/{}.md", renderer::sanitize_filename(&c.metadata.api_name));
        }
        SinglePageContext::ValidationRule(c) => {
            let dir = output_dir.join("validation-rules");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_validation_rule_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.rule_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: validation-rules/{}.md", renderer::sanitize_filename(&c.metadata.rule_name));
        }
        SinglePageContext::Object(c) => {
            let dir = output_dir.join("objects");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_object_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.object_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: objects/{}.md", renderer::sanitize_filename(&c.metadata.object_name));
        }
        SinglePageContext::Lwc(c) => {
            let dir = output_dir.join("lwc");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_lwc_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.component_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: lwc/{}.md", renderer::sanitize_filename(&c.metadata.component_name));
        }
        SinglePageContext::FlexiPage(c) => {
            let dir = output_dir.join("flexipages");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_flexipage_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.api_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: flexipages/{}.md", renderer::sanitize_filename(&c.metadata.api_name));
        }
        SinglePageContext::Aura(c) => {
            let dir = output_dir.join("aura");
            std::fs::create_dir_all(&dir)?;
            let page = renderer::render_aura_page(c);
            let filename = format!("{}.md", renderer::sanitize_filename(&c.metadata.component_name));
            std::fs::write(dir.join(filename), page)?;
            println!("\u{2713} Documentation updated: aura/{}.md", renderer::sanitize_filename(&c.metadata.component_name));
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Add `rebuild_index_from_cache` function**

Add to `src/update.rs`:

```rust
/// Rebuild the full index from cached documentation.
/// For HTML format, this rebuilds the entire HTML site since the HTML renderer
/// generates a self-contained site with sidebar navigation.
fn rebuild_index_from_cache(cache: &Cache, output_dir: &Path, format: &OutputFormat) -> Result<()> {
    let all_names = Arc::new(build_all_names_from_cache(cache));

    // Build render contexts from cache entries.
    // We need metadata + documentation for each entry. The cache stores documentation
    // but not parsed metadata. For the index, we only need the summary and name fields
    // which are in the documentation structs. We create minimal metadata stubs.

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
        // HTML renderer generates the full site with sidebar
        renderer::write_output(output_dir, format, &bundle)?;
    } else {
        // Markdown: only rewrite the index file
        let index = renderer::render_index(&bundle);
        std::fs::write(output_dir.join("index.md"), index)?;
    }

    Ok(())
}
```

- [ ] **Step 4: Wire up `run_update` in `main.rs`**

Replace the stub `Commands::Update` match arm in `src/main.rs`:

```rust
Commands::Update(args) => {
    sfdoc::update::run_update(&args).await?;
}
```

Add the import at the top of `main.rs` if not already present — since `update` is accessed via `sfdoc::update::run_update`, no new `use` is needed.

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: successful compilation.

- [ ] **Step 6: Commit**

```bash
git add src/update.rs src/main.rs
git commit -m "feat: implement run_update orchestration for sfdoc update command"
```

---

### Task 6: Ensure renderer functions are public

The `write_single_page` function in `update.rs` calls `render_class_page`, `render_trigger_page`, etc. These need to be `pub`.

**Files:**
- Modify: `src/renderer.rs` (only if needed)

- [ ] **Step 1: Check visibility of render functions**

Run: `grep -n "^fn render_\|^pub fn render_" src/renderer.rs`

If any of `render_class_page`, `render_trigger_page`, `render_flow_page`, `render_validation_rule_page`, `render_object_page`, `render_lwc_page`, `render_flexipage_page`, `render_aura_page` are not `pub`, make them `pub`.

- [ ] **Step 2: Make functions public if needed**

For each non-public render function, change `fn render_*_page` to `pub fn render_*_page`.

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: successful compilation.

- [ ] **Step 4: Commit (if changes were made)**

```bash
git add src/renderer.rs
git commit -m "refactor: make per-page render functions public for update command"
```

---

### Task 7: Ensure metadata types have `Default` implementations

The `rebuild_index_from_cache` function creates metadata stubs using `..Default::default()`. Some metadata types may not derive `Default`.

**Files:**
- Modify: `src/types.rs` (only if needed)

- [ ] **Step 1: Check which metadata types derive Default**

Run: `grep -B5 "pub struct.*Metadata" src/types.rs | grep -E "(Default|pub struct)"`

For any metadata struct used in `rebuild_index_from_cache` that doesn't derive `Default`, add it.

The types that need `Default`:
- `ClassMetadata` (already has it)
- `TriggerMetadata`
- `FlowMetadata`
- `ValidationRuleMetadata`
- `ObjectMetadata`
- `LwcMetadata`
- `FlexiPageMetadata`
- `AuraMetadata`

- [ ] **Step 2: Add `Default` derive where missing**

For each struct that's missing `Default`, add it to the derive list. For example, change:
```rust
#[derive(Debug, Clone)]
pub struct TriggerMetadata {
```
to:
```rust
#[derive(Debug, Clone, Default)]
pub struct TriggerMetadata {
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: successful compilation.

- [ ] **Step 4: Commit (if changes were made)**

```bash
git add src/types.rs
git commit -m "refactor: derive Default for metadata types used in index rebuild"
```

---

### Task 8: Add integration tests

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Write integration tests for the CLI parsing and error cases**

Add to `tests/integration.rs`:

```rust
#[test]
fn update_no_target_shows_error() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args(["update"])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // clap should report the missing required argument
    assert!(
        stderr.contains("required") || stderr.contains("Usage"),
        "Expected usage/required error, got: {stderr}"
    );
}

#[test]
fn update_no_cache_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "SomeClass",
            "--source-dir",
            tmp.path().to_str().unwrap(),
            "--output",
            tmp.path().join("out").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No existing documentation found") || stderr.contains("sfdoc generate"),
        "Expected 'run generate first' error, got: {stderr}"
    );
}

#[test]
fn update_file_not_found_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    // Create a cache file so the "no cache" check passes
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(
        out_dir.join(".sfdoc-cache.json"),
        r#"{"cache_version":1,"entries":{},"trigger_entries":{},"flow_entries":{},"validation_rule_entries":{},"object_entries":{},"lwc_entries":{},"flexipage_entries":{},"aura_entries":{}}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "/nonexistent/path/Foo.cls",
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Expected 'file not found' error, got: {stderr}"
    );
}

#[test]
fn update_name_not_found_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(
        out_dir.join(".sfdoc-cache.json"),
        r#"{"cache_version":1,"entries":{},"trigger_entries":{},"flow_entries":{},"validation_rule_entries":{},"object_entries":{},"lwc_entries":{},"flexipage_entries":{},"aura_entries":{}}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "NonexistentClass",
            "--source-dir",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No source file matching"),
        "Expected 'no source file' error, got: {stderr}"
    );
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test integration -- update`
Expected: all 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for sfdoc update error cases"
```

---

### Task 9: Update the project plan

**Files:**
- Modify: `project-plan.md`

- [ ] **Step 1: Mark Phase 28 as done**

In `project-plan.md`, change the Phase 28 entry status from `todo` to `done`:

```yaml
    - id: single-file-update
      content: "Phase 28: sfdoc update <file> — re-document a single file without a full rebuild"
      status: done
```

- [ ] **Step 2: Commit**

```bash
git add project-plan.md
git commit -m "docs: mark Phase 28 as done in project plan"
```
