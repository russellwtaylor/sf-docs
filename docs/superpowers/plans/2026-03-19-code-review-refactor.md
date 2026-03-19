# Code Review Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address all code review findings — security fixes, DRY violations, architecture improvements, and minor issues — to improve maintainability, safety, and extensibility.

**Architecture:** Extract a `DocClient` trait for AI providers, a shared `apex_common` module for duplicated constants/regexes, a `safe_truncate` helper for UTF-8-safe prompt truncation, a `DocumentationBundle` struct for renderer parameters, a `scan_component` helper for LWC/Aura scanners, and a `AllNames::all_known_names()` helper. Unify the 8 repeated `document_*` methods with a generic `document` helper on each client. Refactor `main.rs` generate logic to reduce per-type boilerplate (not a full generic pipeline — that would be over-engineering for this step, but eliminate the most obvious repetition).

**Tech Stack:** Rust, tokio, reqwest, serde, sha2, walkdir

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/apex_common.rs` | Create | `APEX_BUILTINS`, `re_type_ref()` shared between parsers |
| `src/doc_client.rs` | Create | `DocClient` trait + generic `document` helper |
| `src/main.rs` | Modify | Use trait object, reduce boilerplate, `DocumentationBundle` |
| `src/gemini.rs` | Modify | Implement `DocClient` trait, add generic `document` helper |
| `src/openai_compat.rs` | Modify | Implement `DocClient` trait, add generic `document` helper |
| `src/parser.rs` | Modify | Import from `apex_common` instead of local definitions |
| `src/trigger_parser.rs` | Modify | Import from `apex_common` instead of local definitions |
| `src/aura_prompt.rs` | Modify | Use `safe_truncate` instead of byte-slicing |
| `src/lwc_prompt.rs` | Modify | Use `safe_truncate` instead of byte-slicing |
| `src/types.rs` | Modify | Add `AllNames::all_known_names()` method |
| `src/scanner.rs` | Modify | Fix `follow_links` inconsistency, add file size limit, extract `scan_component` |
| `src/renderer.rs` | Modify | `DocumentationBundle`, `AllNames::all_known_names()` usage |
| `src/cache.rs` | Modify | Pre-allocate in `hash_source` |
| `src/providers.rs` | Modify | Replace `unreachable!()` with `Option` |
| `src/lib.rs` | Modify | Add `pub mod apex_common; pub mod doc_client;` |
| `Cargo.toml` | Modify | Fix description, move `test-util` to dev-deps, add `async-trait` |

---

### Task 1: Fix UTF-8 panic in prompt truncation (P0 security)

**Files:**
- Modify: `src/aura_prompt.rs:75-78`
- Modify: `src/lwc_prompt.rs:68-71`

- [ ] **Step 1: Write failing test for UTF-8 truncation**

Add a test in `src/aura_prompt.rs` that passes a source with multi-byte UTF-8 characters longer than 6000 bytes:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AuraMetadata, SourceFile};
    use std::path::PathBuf;

    #[test]
    fn build_aura_prompt_does_not_panic_on_multibyte_utf8() {
        // Each char is 3 bytes; 2500 chars = 7500 bytes > 6000
        let source = "a".repeat(5000) + &"\u{00e9}".repeat(2500);
        let file = SourceFile {
            path: PathBuf::from("test.cmp"),
            filename: "test.cmp".to_string(),
            raw_source: source,
        };
        let meta = AuraMetadata {
            component_name: "test".to_string(),
            ..Default::default()
        };
        // Should not panic
        let _ = build_aura_prompt(&file, &meta);
    }
}
```

- [ ] **Step 2: Run test to verify it panics**

Run: `cargo test --lib build_aura_prompt_does_not_panic -- --nocapture 2>&1`
Expected: panic at byte boundary

- [ ] **Step 3: Fix both truncation sites**

In `src/aura_prompt.rs`, replace the byte-slicing:
```rust
const MAX_SOURCE_CHARS: usize = 6_000;
if file.raw_source.len() > MAX_SOURCE_CHARS {
    let truncated: String = file.raw_source.chars().take(MAX_SOURCE_CHARS).collect();
    prompt.push_str(&truncated);
    prompt.push_str("\n// ... (truncated)\n");
} else {
    prompt.push_str(&file.raw_source);
}
```

Apply the same fix in `src/lwc_prompt.rs` (MAX_JS_CHARS).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib build_aura_prompt_does_not_panic build_lwc_prompt -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/aura_prompt.rs src/lwc_prompt.rs
git commit -m "fix: use chars().take() for UTF-8-safe prompt truncation"
```

---

### Task 2: Fix symlink inconsistency and add file size limit in scanner (P1 security)

**Files:**
- Modify: `src/scanner.rs:162-246`

- [ ] **Step 1: Write test for file size limit**

Add to `src/scanner.rs` tests:
```rust
#[test]
fn scanner_skips_files_over_size_limit() {
    let tmp = TempDir::new().unwrap();
    // Create a file just over 10 MB
    let big = "x".repeat(10 * 1024 * 1024 + 1);
    write_file(tmp.path(), "Huge.cls", &big);
    write_file(tmp.path(), "Small.cls", "public class Small {}");

    let files = ApexScanner.scan(tmp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Small.cls");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib scanner_skips_files_over_size_limit -- --nocapture 2>&1`
Expected: FAIL (finds 2 files)

- [ ] **Step 3: Add file size check and fix follow_links**

In `scan_by_extension`, add a size check before `read_to_string`:
```rust
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

// Before read_to_string:
if let Ok(metadata) = std::fs::metadata(path) {
    if metadata.len() > MAX_FILE_SIZE {
        eprintln!(
            "Warning: skipping {} ({:.1} MB exceeds {} MB limit)",
            path.display(),
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_FILE_SIZE / (1024 * 1024)
        );
        continue;
    }
}
```

Change `LwcScanner` and `AuraScanner` to use `follow_links(false)` instead of `follow_links(true)`.

Extract `scan_component` helper to deduplicate `LwcScanner`/`AuraScanner`:
```rust
fn scan_component(
    source_dir: &Path,
    suffix: &str,
    ancestor: &str,
) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(source_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_visit)
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() { continue; }
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !file_name.ends_with(suffix) { continue; }
        let in_dir = path.ancestors()
            .any(|a| a.file_name().and_then(|n| n.to_str()) == Some(ancestor));
        if !in_dir { continue; }

        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > MAX_FILE_SIZE {
                eprintln!("Warning: skipping {} ({:.1} MB exceeds limit)",
                    path.display(), meta.len() as f64 / (1024.0 * 1024.0));
                continue;
            }
        }

        let component_name = file_name.trim_end_matches(suffix);
        let raw_source = read_with_js_fallback(path, component_name, &file_name)?;

        files.push(SourceFile {
            path: path.to_path_buf(),
            filename: file_name,
            raw_source,
        });
    }
    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(files)
}
```

Then simplify LwcScanner/AuraScanner:
```rust
impl FileScanner for LwcScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        scan_component(source_dir, ".js-meta.xml", "lwc")
    }
}

impl FileScanner for AuraScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        scan_component(source_dir, ".cmp", "aura")
    }
}
```

- [ ] **Step 4: Run all scanner tests**

Run: `cargo test --lib scanner -- --nocapture 2>&1`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src/scanner.rs
git commit -m "fix: consistent follow_links(false), file size limit, deduplicate component scanners"
```

---

### Task 3: Extract shared `apex_common` module (DRY)

**Files:**
- Create: `src/apex_common.rs`
- Modify: `src/parser.rs:64-108`
- Modify: `src/trigger_parser.rs:24-67`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `apex_common.rs`**

```rust
use regex::Regex;
use std::sync::OnceLock;

pub fn re_type_ref() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][a-zA-Z0-9_]+)\b").unwrap())
}

pub const APEX_BUILTINS: &[&str] = &[
    "String", "Integer", "Long", "Double", "Decimal", "Boolean",
    "Date", "DateTime", "Time", "Blob", "Id", "Object",
    "List", "Map", "Set", "SObject", "Schema", "Database",
    "System", "Math", "JSON", "Type", "Exception", "DmlException",
    "QueryException", "Test", "ApexPages", "PageReference",
    "SelectOption", "Messaging", "Approval", "UserInfo", "Label",
    "Site", "Network", "ConnectApi", "Trigger",
];
```

Note: include "Trigger" in the shared list (trigger_parser had it).

- [ ] **Step 2: Update `parser.rs` to import from `apex_common`**

Remove local `re_type_ref()` and `APEX_BUILTINS` definitions. Add:
```rust
use crate::apex_common::{re_type_ref, APEX_BUILTINS};
```

- [ ] **Step 3: Update `trigger_parser.rs` to import from `apex_common`**

Remove local `re_type_ref()` and `APEX_BUILTINS` definitions. Add:
```rust
use crate::apex_common::{re_type_ref, APEX_BUILTINS};
```

- [ ] **Step 4: Add `pub mod apex_common;` to `lib.rs`**

- [ ] **Step 5: Run all parser tests**

Run: `cargo test --lib parser trigger_parser -- --nocapture 2>&1`
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add src/apex_common.rs src/parser.rs src/trigger_parser.rs src/lib.rs
git commit -m "refactor: extract shared APEX_BUILTINS and re_type_ref into apex_common module"
```

---

### Task 4: Extract `DocClient` trait and unify `document_*` methods (P1 DRY)

**Files:**
- Create: `src/doc_client.rs`
- Modify: `src/gemini.rs:202-342`
- Modify: `src/openai_compat.rs:194-335`
- Modify: `src/main.rs:33-126`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `async-trait` dependency**

Add to `Cargo.toml` under `[dependencies]`:
```toml
async-trait = "0.1"
```

- [ ] **Step 2: Create `src/doc_client.rs` with the trait**

```rust
use anyhow::Result;
use async_trait::async_trait;

use crate::types::*;

#[async_trait]
pub trait DocClient: Send + Sync {
    /// Send a (system, user) prompt and return the raw response text.
    async fn send_with_retry(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;

    /// Provider name for error messages.
    fn provider_name(&self) -> &str;
}
```

- [ ] **Step 3: Add a generic `document` free function**

In `src/doc_client.rs`:
```rust
use serde::de::DeserializeOwned;
use anyhow::Context;

pub async fn document<D: DeserializeOwned>(
    client: &dyn DocClient,
    system_prompt: &str,
    user_prompt: &str,
    entity_label: &str,
) -> Result<D> {
    let raw = client.send_with_retry(system_prompt, user_prompt).await?;
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "Failed to parse {} JSON for {}:\n{raw}",
            client.provider_name(),
            entity_label
        )
    })
}
```

- [ ] **Step 4: Implement `DocClient` trait for `GeminiClient`**

In `src/gemini.rs`:
- Make `send_with_retry` public (rename internal to `send_request`)
- Add `#[async_trait] impl DocClient for GeminiClient`
- Remove all 8 `document_*` methods
- The semaphore acquire stays in `send_with_retry` (called once per request)

Wait — the semaphore is currently in each `document_*` method, not in `send_with_retry`. Move the semaphore acquire into `send_with_retry` so the trait method handles concurrency:

```rust
#[async_trait]
impl crate::doc_client::DocClient for GeminiClient {
    async fn send_with_retry(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let _permit = self.semaphore.acquire().await?;
        self.send_request(system_prompt, user_prompt).await
    }

    fn provider_name(&self) -> &str {
        "Gemini"
    }
}
```

Rename the existing `send_with_retry` to `send_request` (private).

- [ ] **Step 5: Implement `DocClient` trait for `OpenAiCompatClient`**

Same pattern as Step 4:
```rust
#[async_trait]
impl crate::doc_client::DocClient for OpenAiCompatClient {
    async fn send_with_retry(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let _permit = self.semaphore.acquire().await?;
        self.send_request(system_prompt, user_prompt).await
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }
}
```

- [ ] **Step 6: Update `main.rs` to use `Arc<dyn DocClient>`**

Replace the `DocClient` enum with:
```rust
use sfdoc::doc_client::{self, DocClient};

// In the generate command:
let client: Arc<dyn DocClient> = match provider {
    Provider::Gemini => Arc::new(GeminiClient::new(api_key, &model, args.concurrency, args.rpm)?),
    _ => Arc::new(OpenAiCompatClient::new(
        api_key, &model, provider.base_url(), args.concurrency,
        provider.display_name(), args.rpm,
    )?),
};
```

Replace all `client.document_class(...)` calls with:
```rust
doc_client::document::<ClassDocumentation>(
    client.as_ref(),
    SYSTEM_PROMPT,
    &build_prompt(&files[idx], &class_meta[idx]),
    &class_meta[idx].class_name,
).await?
```

And similarly for all 8 types.

- [ ] **Step 7: Add `pub mod doc_client;` to `lib.rs`**

- [ ] **Step 8: Run all tests**

Run: `cargo test 2>&1`
Expected: all PASS

- [ ] **Step 9: Commit**

```bash
git add src/doc_client.rs src/gemini.rs src/openai_compat.rs src/main.rs src/lib.rs Cargo.toml
git commit -m "refactor: extract DocClient trait, eliminate 16 duplicate document_* methods"
```

---

### Task 5: Add `AllNames::all_known_names()` and `DocumentationBundle` (P2 DRY)

**Files:**
- Modify: `src/types.rs`
- Modify: `src/renderer.rs`

- [ ] **Step 1: Add `all_known_names()` to `AllNames`**

In `src/types.rs`:
```rust
impl AllNames {
    pub fn all_known_names(&self) -> HashSet<&str> {
        self.class_names.iter()
            .chain(self.trigger_names.iter())
            .chain(self.flow_names.iter())
            .chain(self.validation_rule_names.iter())
            .chain(self.object_names.iter())
            .chain(self.lwc_names.iter())
            .chain(self.flexipage_names.iter())
            .chain(self.aura_names.iter())
            .map(|s| s.as_str())
            .collect()
    }
}
```

- [ ] **Step 2: Replace all `let known: HashSet<&str>` blocks in `renderer.rs`**

Replace each instance (7 occurrences at lines 130, 290, 391, 550, 655, 1195, 1350) with:
```rust
let known = ctx.all_names.all_known_names();
```

- [ ] **Step 3: Add `DocumentationBundle` struct to `renderer.rs`**

```rust
pub struct DocumentationBundle<'a> {
    pub classes: &'a [RenderContext],
    pub triggers: &'a [TriggerRenderContext],
    pub flows: &'a [FlowRenderContext],
    pub validation_rules: &'a [ValidationRuleRenderContext],
    pub objects: &'a [ObjectRenderContext],
    pub lwc: &'a [LwcRenderContext],
    pub flexipages: &'a [FlexiPageRenderContext],
    pub custom_metadata: &'a [CustomMetadataRenderContext],
    pub aura: &'a [AuraRenderContext],
}
```

Update `write_output` signature to take `&DocumentationBundle` instead of 9 parameters. Remove `#[allow(clippy::too_many_arguments)]`.

Update `html_renderer::write_html_output` similarly (or just pass through the bundle fields).

- [ ] **Step 4: Update `main.rs` call site**

```rust
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
```

- [ ] **Step 5: Run all tests**

Run: `cargo test 2>&1`
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add src/types.rs src/renderer.rs src/html_renderer.rs src/main.rs
git commit -m "refactor: add AllNames::all_known_names(), DocumentationBundle to eliminate parameter bloat"
```

---

### Task 6: Fix `Provider::base_url()` unreachable and `Cargo.toml` issues (P2)

**Files:**
- Modify: `src/providers.rs:65-72`
- Modify: `Cargo.toml`

- [ ] **Step 1: Change `base_url()` to return `Option<&'static str>`**

```rust
pub fn base_url(&self) -> Option<&'static str> {
    match self {
        Provider::Gemini => None,
        Provider::Groq => Some("https://api.groq.com/openai/v1"),
        Provider::OpenAi => Some("https://api.openai.com/v1"),
        Provider::Ollama => Some("http://localhost:11434/v1"),
    }
}
```

Update the call site in `main.rs` to use `.unwrap()` (it's only called for non-Gemini providers, which is guaranteed by the match arm). Or better, use `.expect("non-Gemini provider must have base_url")`.

- [ ] **Step 2: Fix Cargo.toml**

Update description:
```toml
description = "Generate wiki-style documentation for Salesforce metadata (Apex, Flows, LWC, Objects, and more) using AI providers"
```

Move `test-util` to dev-dependencies:
```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "macros"] }

[dev-dependencies]
tokio = { version = "1", features = ["test-util"] }
```

- [ ] **Step 3: Run full build and tests**

Run: `cargo build && cargo test 2>&1`
Expected: all PASS

- [ ] **Step 4: Commit**

```bash
git add src/providers.rs src/main.rs Cargo.toml
git commit -m "fix: Provider::base_url returns Option, update Cargo.toml description, move test-util to dev-deps"
```

---

### Task 7: Pre-allocate in `hash_source` (P3 minor perf)

**Files:**
- Modify: `src/cache.rs:193-200`

- [ ] **Step 1: Update `hash_source` to pre-allocate**

```rust
pub fn hash_source(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}
```

- [ ] **Step 2: Run cache tests**

Run: `cargo test --lib cache -- --nocapture 2>&1`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add src/cache.rs
git commit -m "perf: pre-allocate hex string in hash_source"
```

---

### Task 8: Run full test suite and verify clippy

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -- -W clippy::all 2>&1`
Expected: no warnings

- [ ] **Step 2: Run full test suite including integration tests**

Run: `cargo test 2>&1`
Expected: all tests pass

- [ ] **Step 3: Run cargo fmt check**

Run: `cargo fmt -- --check 2>&1`
Expected: no formatting issues (run `cargo fmt` if needed)

- [ ] **Step 4: Final commit if fmt changes needed**

```bash
git add -A
git commit -m "style: apply cargo fmt"
```
