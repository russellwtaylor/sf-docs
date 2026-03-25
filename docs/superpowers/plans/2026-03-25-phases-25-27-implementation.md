# Phases 25-27 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--name-filter` glob flag, client-side fuzzy search in HTML output, and `@tag` annotation support with CLI/UI filtering.

**Architecture:** Three layered filters applied at different pipeline stages: `--type` gates scanners (existing), `--name-filter` filters post-scan/pre-parse, `--tag` filters post-parse/pre-AI. Search is orthogonal — build-time index generation + static JS assets.

**Tech Stack:** Rust, clap, globset, fuse.js (embedded), regex

**Spec:** `docs/superpowers/specs/2026-03-25-phases-25-27-design.md`

---

### Task 1: Add `globset` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add globset to Cargo.toml**

Add under `[dependencies]`:

```toml
globset = "0.4"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add globset crate for name-filter glob matching"
```

---

### Task 2: Add `--name-filter` CLI arg and helper

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write failing tests for `--name-filter`**

Add these tests to the existing `#[cfg(test)] mod tests` block in `src/cli.rs`:

```rust
#[test]
fn name_filter_not_set_matches_all() {
    let args = parse_generate(&[]);
    assert!(args.name_matches("OrderService"));
    assert!(args.name_matches("anything"));
}

#[test]
fn name_filter_matches_glob() {
    let args = parse_generate(&["--name-filter", "Order*"]);
    assert!(args.name_matches("OrderService"));
    assert!(args.name_matches("OrderHelper"));
    assert!(!args.name_matches("AccountService"));
}

#[test]
fn name_filter_suffix_glob() {
    let args = parse_generate(&["--name-filter", "*Service"]);
    assert!(args.name_matches("OrderService"));
    assert!(!args.name_matches("OrderHelper"));
}

#[test]
fn name_filter_contains_glob() {
    let args = parse_generate(&["--name-filter", "*Order*"]);
    assert!(args.name_matches("OrderService"));
    assert!(args.name_matches("MyOrderHelper"));
    assert!(!args.name_matches("AccountService"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli::tests`
Expected: FAIL — `name_matches` method not found, `--name-filter` arg not recognized

- [ ] **Step 3: Add the `--name-filter` arg and `name_matches` method**

In `src/cli.rs`, add the arg to `GenerateArgs` after the `types` field:

```rust
/// Only document files whose name matches this glob pattern (e.g. 'Order*', '*Service').
/// Applied across all metadata types against the logical filename.
#[arg(long)]
pub name_filter: Option<String>,
```

Add the `name_matches` method to the `impl GenerateArgs` block:

```rust
/// Returns `true` if the given filename stem matches the `--name-filter` glob,
/// or if no filter was specified. Compiles the glob once via OnceLock for efficiency.
pub fn name_matches(&self, filename_stem: &str) -> bool {
    match &self.name_filter {
        None => true,
        Some(pattern) => {
            // Note: For CLI usage this is called per-file across all types.
            // The glob is cheap to compile but for hot paths consider
            // compiling once in the caller. globset::Glob::new returns
            // an error for invalid patterns; we fall back to match-all.
            let glob = globset::Glob::new(pattern)
                .unwrap_or_else(|_| globset::Glob::new("*").unwrap());
            glob.compile_matcher().is_match(filename_stem)
        }
    }
}
```

Note: The glob recompiles on each call. This is acceptable for CLI usage (called once per scanned file, <1ms each). If profiling shows this as a bottleneck, compile the glob once in the `name_filter` closure in Task 3 instead.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib cli::tests`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --name-filter CLI arg with glob matching"
```

---

### Task 3: Apply `--name-filter` in the main pipeline

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add name filter after each scan call**

In `src/main.rs`, after the block of `scan()` calls (around line 131, after `aura_files`), add a filtering closure and apply it to each file vec. Insert this code right after the `aura_files` scan call and before the "Require at least one file" check:

```rust
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
```

- [ ] **Step 2: Add verbose logging for the filter**

In the verbose block (around line 74-77), after the types logging, add:

```rust
if let Some(ref pattern) = args.name_filter {
    eprintln!("Name filter: {}", pattern);
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: apply --name-filter post-scan to narrow documented files"
```

---

### Task 4: Add `tags` field to `ClassMetadata` and `TriggerMetadata`

**Files:**
- Modify: `src/types.rs`

- [ ] **Step 1: Add `tags` field to `ClassMetadata`**

In `src/types.rs`, add after the `references` field in `ClassMetadata`:

```rust
/// Tags extracted from `@tag` annotations in ApexDoc comments.
pub tags: Vec<String>,
```

- [ ] **Step 2: Add `tags` field to `TriggerMetadata`**

In `src/types.rs`, add after the `references` field in `TriggerMetadata`:

```rust
/// Tags extracted from `@tag` annotations in ApexDoc comments.
pub tags: Vec<String>,
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles (both structs derive `Default`, so `Vec<String>` defaults to empty)

- [ ] **Step 4: Commit**

```bash
git add src/types.rs
git commit -m "feat: add tags field to ClassMetadata and TriggerMetadata"
```

---

### Task 5: Parse `@tag` annotations from ApexDoc comments

**Files:**
- Modify: `src/parser.rs`
- Modify: `src/trigger_parser.rs`

- [ ] **Step 1: Write failing tests for `@tag` parsing in `parser.rs`**

Add to the existing `#[cfg(test)] mod tests` block in `src/parser.rs`:

```rust
#[test]
fn parses_single_tag() {
    let source = r#"
    /**
     * @tag billing
     * Service class for orders
     */
    public class OrderService {
    }
    "#;
    let meta = parse_apex_class(source).unwrap();
    assert_eq!(meta.tags, vec!["billing"]);
}

#[test]
fn parses_multiple_tags() {
    let source = r#"
    /**
     * @tag billing
     * @tag integration
     * Service class
     */
    public class OrderService {
    }
    "#;
    let meta = parse_apex_class(source).unwrap();
    assert_eq!(meta.tags, vec!["billing", "integration"]);
}

#[test]
fn parses_hyphenated_tag() {
    let source = r#"
    /**
     * @tag order-management
     */
    public class OrderService {
    }
    "#;
    let meta = parse_apex_class(source).unwrap();
    assert_eq!(meta.tags, vec!["order-management"]);
}

#[test]
fn no_tags_returns_empty() {
    let source = r#"
    /**
     * Service class
     */
    public class OrderService {
    }
    "#;
    let meta = parse_apex_class(source).unwrap();
    assert!(meta.tags.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parser::tests`
Expected: FAIL — `tags` field is empty in all cases

- [ ] **Step 3: Add `extract_tags` to `src/apex_common.rs` (shared by both parsers)**

Add to `src/apex_common.rs`:

```rust
use regex::Regex;
use std::sync::OnceLock;

fn re_tag() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@tag\s+(\w[\w-]*)").unwrap())
}

/// Extracts `@tag <label>` annotations from ApexDoc comment strings.
pub fn extract_tags(comments: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    for comment in comments {
        for caps in re_tag().captures_iter(comment) {
            tags.push(caps[1].to_string());
        }
    }
    tags
}
```

Note: `regex::Regex` and `OnceLock` may already be imported in `apex_common.rs`. Check existing imports and add only what's missing.

- [ ] **Step 4: Use `extract_tags` in `parser.rs`**

In `src/parser.rs`, add the import:

```rust
use crate::apex_common::extract_tags;
```

In `parse_apex_class`, after `meta.existing_comments = apexdoc_comments;` add:

```rust
meta.tags = extract_tags(&meta.existing_comments);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib parser::tests`
Expected: all PASS

- [ ] **Step 5: Write failing tests for `@tag` parsing in `trigger_parser.rs`**

Add to the existing test module in `src/trigger_parser.rs`:

```rust
#[test]
fn parses_trigger_tags() {
    let source = r#"
    /**
     * @tag billing
     * @tag automation
     */
    trigger OrderTrigger on Order__c (before insert, after update) {
    }
    "#;
    let meta = parse_apex_trigger(source).unwrap();
    assert_eq!(meta.tags, vec!["billing", "automation"]);
}

#[test]
fn trigger_no_tags_returns_empty() {
    let source = r#"
    trigger OrderTrigger on Order__c (before insert) {
    }
    "#;
    let meta = parse_apex_trigger(source).unwrap();
    assert!(meta.tags.is_empty());
}
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test --lib trigger_parser::tests`
Expected: FAIL — `tags` field is empty

- [ ] **Step 7: Use shared `extract_tags` in `trigger_parser.rs`**

In `src/trigger_parser.rs`, add the import:

```rust
use crate::apex_common::extract_tags;
```

In `parse_apex_trigger`, after the `TriggerMetadata` struct initialization (around line 36, after `..Default::default()` closes), add:

```rust
meta.tags = extract_tags(&meta.existing_comments);
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --lib trigger_parser::tests`
Expected: all PASS

- [ ] **Step 9: Commit**

```bash
git add src/parser.rs src/trigger_parser.rs
git commit -m "feat: parse @tag annotations from ApexDoc comments"
```

---

### Task 6: Add `--tag` CLI arg and helper

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write failing tests**

Add to the test module in `src/cli.rs`:

```rust
#[test]
fn no_tag_flag_matches_all() {
    let args = parse_generate(&[]);
    assert!(args.tag_matches(&["billing".to_string()]));
    assert!(args.tag_matches(&[]));
}

#[test]
fn tag_flag_matches_or_logic() {
    let args = parse_generate(&["--tag", "billing,integration"]);
    assert!(args.tag_matches(&["billing".to_string()]));
    assert!(args.tag_matches(&["integration".to_string()]));
    assert!(args.tag_matches(&["billing".to_string(), "other".to_string()]));
    assert!(!args.tag_matches(&["unrelated".to_string()]));
    assert!(!args.tag_matches(&[]));
}

#[test]
fn tag_flag_case_insensitive() {
    let args = parse_generate(&["--tag", "Billing"]);
    assert!(args.tag_matches(&["billing".to_string()]));
    assert!(args.tag_matches(&["BILLING".to_string()]));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli::tests`
Expected: FAIL — `--tag` not recognized, `tag_matches` not found

- [ ] **Step 3: Add the `--tag` arg and `tag_matches` method**

In `src/cli.rs`, add to `GenerateArgs` after `name_filter`:

```rust
/// Only document items tagged with at least one of these labels (comma-separated).
/// Tags are extracted from @tag annotations in ApexDoc comments.
/// When --tag is specified, non-taggable metadata types (flows, objects, etc.) are excluded.
#[arg(long, value_delimiter = ',')]
pub tags: Vec<String>,
```

Add `tag_matches` to the `impl GenerateArgs` block:

```rust
/// Returns `true` if the item's tags overlap with the `--tag` filter (OR logic, case-insensitive).
/// Returns `true` when `--tag` is not specified.
/// Returns `false` when `--tag` is specified but the item has no tags.
pub fn tag_matches(&self, item_tags: &[String]) -> bool {
    if self.tags.is_empty() {
        return true;
    }
    item_tags.iter().any(|t| {
        self.tags
            .iter()
            .any(|f| f.eq_ignore_ascii_case(t))
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib cli::tests`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --tag CLI arg with case-insensitive OR matching"
```

---

### Task 7: Apply `--tag` filter in the main pipeline

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Apply tag filter post-parse, pre-AI**

In `src/main.rs`, after the parsing block (around line 245, after `aura_meta`) and **before** the `Arc::new(files)` block, add tag filtering.

**Important:** The variables `files`, `class_meta`, `trigger_files`, `trigger_meta`, `flow_files`, `flow_meta`, etc. are used later in the function (wrapped in `Arc`, hashed, sent to API, etc.). The filter must produce new variables that **shadow** the originals at the same scope level — NOT inside an `if` block (which would scope-limit the bindings). Use a helper function that returns filtered pairs:

```rust
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
```

Add this helper function above `main()` or as a standalone function. Then in the pipeline, after parsing and before `Arc::new()`:

```rust
// Apply --tag filter post-parse, pre-AI.
let (files, class_meta) =
    filter_by_tags(files, class_meta, |m| &m.tags, &args);
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
let custom_metadata_records: Vec<types::CustomMetadataRecord> = if args.tags.is_empty() {
    custom_metadata_records
} else {
    Vec::new()
};
let (aura_files, aura_meta) = if args.tags.is_empty() {
    (aura_files, aura_meta)
} else {
    (Vec::new(), Vec::new())
};
```

These shadow bindings are at the same scope level as the originals, so they persist for the rest of the function. Note: `custom_metadata_records` is a `Vec<CustomMetadataRecord>` (not a separate files+meta pair), handle it separately.

- [ ] **Step 2: Add verbose logging**

In the verbose block, after the name-filter logging:

```rust
if !args.tags.is_empty() {
    eprintln!("Tags:        {}", args.tags.join(", "));
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: apply --tag filter post-parse to narrow documented files"
```

---

### Task 8: Render tag badges in HTML output

**Files:**
- Modify: `src/html_renderer.rs`

- [ ] **Step 1: Add CSS for tag badges**

In `src/html_renderer.rs`, add to the `CSS` const string (after the `.badge-trigger` line):

```css
.badge-tag{background:#f0fff4;border-color:#a3d9a5;color:#22863a;cursor:pointer}
.badge-tag:hover{background:#dcffe4}
```

- [ ] **Step 2: Add tag badge rendering to class pages**

Find the `render_class_page` function. In the badges section (where it renders access modifier, abstract, virtual, extends, implements badges), add tag badges after the existing badges:

```rust
for tag in &ctx.metadata.tags {
    body.push_str(&format!(
        "<span class=\"badge badge-tag\">{}</span>",
        escape(tag)
    ));
}
```

- [ ] **Step 3: Add tag badge rendering to trigger pages**

Find the `render_trigger_page` function. Add similar tag badge rendering after existing badges:

```rust
for tag in &ctx.metadata.tags {
    body.push_str(&format!(
        "<span class=\"badge badge-tag\">{}</span>",
        escape(tag)
    ));
}
```

- [ ] **Step 4: Add tag pills to index page entries**

In the `render_index` function, for class table rows and trigger table rows, append tag badges after the summary text. In the class table row format string, add tag pills:

For class entries (in the `render_index` class table), after the summary `<td>`, append tag badges inline. Modify the row rendering to include tags from `ctx.metadata.tags`.

For trigger entries, do the same with `ctx.metadata.tags`.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/html_renderer.rs
git commit -m "feat: render tag badges in HTML class/trigger pages and index"
```

---

### Task 9: Render tags in Markdown output

**Files:**
- Modify: `src/renderer.rs`

- [ ] **Step 1: Add tag rendering to Markdown class pages**

Find the `render_class_page` function in `src/renderer.rs`. After the title/badges section and before the summary, add:

```rust
if !ctx.metadata.tags.is_empty() {
    let tag_str: Vec<String> = ctx.metadata.tags.iter().map(|t| format!("`{t}`")).collect();
    page.push_str(&format!("**Tags:** {}\n\n", tag_str.join(", ")));
}
```

- [ ] **Step 2: Add tag rendering to Markdown trigger pages**

Find the `render_trigger_page` function. Add similar tag rendering:

```rust
if !ctx.metadata.tags.is_empty() {
    let tag_str: Vec<String> = ctx.metadata.tags.iter().map(|t| format!("`{t}`")).collect();
    page.push_str(&format!("**Tags:** {}\n\n", tag_str.join(", ")));
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/renderer.rs
git commit -m "feat: render tag labels in Markdown class and trigger pages"
```

---

### Task 10: Generate search index and search.js for HTML output

**Files:**
- Modify: `src/html_renderer.rs`

- [ ] **Step 1: Download fuse.js minified and embed as a const**

Download fuse.js v7.x minified from https://cdn.jsdelivr.net/npm/fuse.js/dist/fuse.min.js and save it as `src/fuse.min.js`. Then add to `src/html_renderer.rs`:

```rust
const FUSE_JS: &str = include_str!("fuse.min.js");
```

- [ ] **Step 2: Create the search JS code**

Add to `src/html_renderer.rs`:

```rust
const SEARCH_JS: &str = r#"
(function() {
  var scriptEl = document.currentScript;
  var base = scriptEl.src.replace(/search\.js$/, '');

  var sidebar = document.querySelector('.sidebar');
  var navSections = sidebar.querySelectorAll('.sidebar-section');
  var searchInput = document.getElementById('sfdoc-search');
  var resultsContainer = document.getElementById('sfdoc-search-results');
  var debounceTimer;

  fetch(base + 'search-index.json')
    .then(function(r) { return r.json(); })
    .then(function(data) {
      var fuse = new Fuse(data, {
        keys: ['title', 'summary', 'tags'],
        threshold: 0.3,
        includeScore: true
      });

      searchInput.addEventListener('input', function() {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(function() {
          var query = searchInput.value.trim();
          if (!query) {
            resultsContainer.style.display = 'none';
            navSections.forEach(function(s) { s.style.display = ''; });
            return;
          }
          var results = fuse.search(query).slice(0, 20);
          navSections.forEach(function(s) { s.style.display = 'none'; });
          resultsContainer.style.display = '';
          resultsContainer.innerHTML = '<ul>' + results.map(function(r) {
            var item = r.item;
            var tagHtml = (item.tags || []).map(function(t) {
              return '<span class="badge badge-tag" style="font-size:10px;padding:1px 5px">' + t + '</span>';
            }).join(' ');
            return '<li><a href="' + base + item.url + '">' + item.title +
              ' <span style="color:#6a737d;font-size:11px">' + item.type + '</span>' +
              (tagHtml ? ' ' + tagHtml : '') + '</a></li>';
          }).join('') + '</ul>';
        }, 200);
      });
    });

  // Tag pill click handler: filter sidebar by tag
  document.addEventListener('click', function(e) {
    if (e.target.classList.contains('badge-tag')) {
      var tag = e.target.textContent.trim();
      searchInput.value = tag;
      searchInput.dispatchEvent(new Event('input'));
    }
  });
})();
"#;
```

- [ ] **Step 3: Add search index generation function**

First, add the import at the top of `src/html_renderer.rs`:

```rust
use serde_json;
```

Then add a function to `src/html_renderer.rs`:

```rust
fn generate_search_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
    lwc_contexts: &[LwcRenderContext],
    flexipage_contexts: &[FlexiPageRenderContext],
    custom_metadata_contexts: &[CustomMetadataRenderContext],
    aura_contexts: &[AuraRenderContext],
) -> String {
    let mut entries = Vec::new();

    for ctx in class_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.class_name,
            "type": if ctx.metadata.is_interface { "interface" } else { "class" },
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("classes/{}.html", sanitize_filename(&ctx.metadata.class_name)),
            "tags": ctx.metadata.tags,
        }));
    }
    for ctx in trigger_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.trigger_name,
            "type": "trigger",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("triggers/{}.html", sanitize_filename(&ctx.metadata.trigger_name)),
            "tags": ctx.metadata.tags,
        }));
    }
    for ctx in flow_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.label,
            "type": "flow",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("flows/{}.html", sanitize_filename(&ctx.metadata.api_name)),
            "tags": [],
        }));
    }
    for ctx in validation_rule_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.rule_name,
            "type": "validation-rule",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("validation-rules/{}.html", sanitize_filename(&ctx.metadata.rule_name)),
            "tags": [],
        }));
    }
    for ctx in object_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.object_name,
            "type": "object",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("objects/{}.html", sanitize_filename(&ctx.metadata.object_name)),
            "tags": [],
        }));
    }
    for ctx in lwc_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.component_name,
            "type": "lwc",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("lwc/{}.html", sanitize_filename(&ctx.metadata.component_name)),
            "tags": [],
        }));
    }
    for ctx in flexipage_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.label,
            "type": "flexipage",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("flexipages/{}.html", sanitize_filename(&ctx.metadata.api_name)),
            "tags": [],
        }));
    }
    for ctx in custom_metadata_contexts {
        entries.push(serde_json::json!({
            "title": ctx.type_name,
            "type": "custom-metadata",
            "folder": "",
            "summary": format!("{} records", ctx.records.len()),
            "url": format!("custom-metadata/{}.html", sanitize_filename(&ctx.type_name)),
            "tags": [],
        }));
    }
    for ctx in aura_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.component_name,
            "type": "aura",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("aura/{}.html", sanitize_filename(&ctx.metadata.component_name)),
            "tags": [],
        }));
    }

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}
```

- [ ] **Step 4: Write search assets in `write_html_output`**

At the end of `write_html_output`, before `Ok(())`, add:

```rust
// Write search assets
let search_index = generate_search_index(
    class_contexts,
    trigger_contexts,
    flow_contexts,
    validation_rule_contexts,
    object_contexts,
    lwc_contexts,
    flexipage_contexts,
    custom_metadata_contexts,
    aura_contexts,
);
std::fs::write(output_dir.join("search-index.json"), search_index)?;
std::fs::write(
    output_dir.join("search.js"),
    format!("{}\n{}", FUSE_JS, SEARCH_JS),
)?;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/html_renderer.rs src/fuse.min.js
git commit -m "feat: generate search-index.json and search.js for HTML output"
```

---

### Task 11: Add search input to HTML sidebar

**Files:**
- Modify: `src/html_renderer.rs`

- [ ] **Step 1: Add search CSS**

Add to the `CSS` const:

```css
.sidebar-search{padding:8px 12px}
.sidebar-search input{width:100%;padding:5px 8px;font-size:13px;border:1px solid #e1e4e8;border-radius:4px;outline:none}
.sidebar-search input:focus{border-color:#0366d6;box-shadow:0 0 0 2px rgba(3,102,214,.15)}
#sfdoc-search-results{display:none}
#sfdoc-search-results ul{list-style:none;padding:0}
#sfdoc-search-results li a{display:block;padding:3px 16px;font-size:13px;color:#24292e;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#sfdoc-search-results li a:hover{background:#e1e4e8;text-decoration:none}
```

- [ ] **Step 2: Add search input and results container to sidebar**

In the `render_sidebar` function, after the sidebar-brand `<a>` tag, add:

```rust
s.push_str("<div class=\"sidebar-search\"><input type=\"text\" id=\"sfdoc-search\" placeholder=\"Search...\" autocomplete=\"off\"></div>\n");
s.push_str("<div id=\"sfdoc-search-results\"></div>\n");
```

- [ ] **Step 3: Add script tag to `wrap_page`**

In the `wrap_page` function, modify the HTML template to add a script tag before `</body>`:

Change:
```
</main>
</body>
```

To:
```
</main>
<script src="{up_prefix}search.js"></script>
</body>
```

Make sure `{up_prefix}` is interpolated in the format string.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/html_renderer.rs
git commit -m "feat: add search input to HTML sidebar with script tag"
```

---

### Task 12: Add `data-tags` attributes to sidebar items

**Files:**
- Modify: `src/html_renderer.rs`

The sidebar currently uses `(&str, &str)` tuples for `(name, folder)`. To avoid a massive signature cascade (changing `render_sidebar`, `wrap_page`, and all 9 `render_*_page` functions), we'll change the tuple type to `(&str, &str, &str)` where the third element is a pre-formatted comma-separated tags string (empty string if no tags).

- [ ] **Step 1: Change sidebar item type from `(&str, &str)` to `(&str, &str, &str)`**

In `write_html_output`, update all item-building code. The third element is a pre-joined tags string:

```rust
// Build comma-separated tag strings for class and trigger items
let class_tag_strings: Vec<String> = class_contexts
    .iter()
    .map(|c| c.metadata.tags.join(","))
    .collect();
let trigger_tag_strings: Vec<String> = trigger_contexts
    .iter()
    .map(|c| c.metadata.tags.join(","))
    .collect();

let class_items: Vec<(&str, &str, &str)> = class_contexts
    .iter()
    .enumerate()
    .map(|(i, c)| (c.metadata.class_name.as_str(), c.folder.as_str(), class_tag_strings[i].as_str()))
    .collect();
let trigger_items: Vec<(&str, &str, &str)> = trigger_contexts
    .iter()
    .enumerate()
    .map(|(i, c)| (c.metadata.trigger_name.as_str(), c.folder.as_str(), trigger_tag_strings[i].as_str()))
    .collect();
```

For other types (flow, vr, object, lwc, flexipage, aura), the third element is always `""`:

```rust
let flow_items: Vec<(&str, &str, &str)> = flow_contexts
    .iter()
    .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str(), ""))
    .collect();
// ... same pattern for all other types
```

- [ ] **Step 2: Update `render_sidebar` to use the new tuple type and emit `data-tags`**

Change all `&[(&str, &str)]` params to `&[(&str, &str, &str)]`. In the `<li>` generation for all types, add the `data-tags` attribute when non-empty:

```rust
for &(name, _folder, tags) in class_items {
    // ... existing active class logic ...
    let tag_attr = if tags.is_empty() {
        String::new()
    } else {
        format!(" data-tags=\"{}\"", escape(tags))
    };
    s.push_str(&format!(
        "<li{tag_attr}><a href=\"{up_prefix}classes/{}.html\"{cls}>{}</a></li>\n",
        escape(name),
        escape(name)
    ));
}
```

- [ ] **Step 3: Update `wrap_page` and all `render_*_page` functions**

Change all `&[(&str, &str)]` to `&[(&str, &str, &str)]` in:
- `wrap_page` signature
- `render_class_page`, `render_trigger_page`, `render_flow_page`, `render_validation_rule_page`, `render_object_page`, `render_lwc_page`, `render_flexipage_page`, `render_custom_metadata_page`, `render_aura_page`
- `render_index`

This is a mechanical find-and-replace of the type signature. The tuple destructuring in `render_index` (which uses `(name, folder)`) needs to change to `(name, folder, _tags)` where tags aren't used, or `(name, folder, tags)` where tag pills are rendered.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/html_renderer.rs
git commit -m "feat: add data-tags attributes to sidebar items for tag filtering"
```

---

### Task 13: Update README documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add `--name-filter` documentation**

In the README, add a section after "Filter by metadata type" (or equivalent):

```markdown
### Filter by name

Generate docs only for files matching a glob pattern:

```bash
sfdoc generate --name-filter 'Order*'
sfdoc generate --name-filter '*Service'
sfdoc generate --type apex --name-filter 'Order*'
```
```

- [ ] **Step 2: Add `--tag` documentation**

```markdown
### Filter by tag

Add `@tag` annotations to your ApexDoc comments:

```apex
/**
 * @tag billing
 * @tag integration
 */
public class OrderService {
    // ...
}
```

Then filter by tag:

```bash
sfdoc generate --tag billing
sfdoc generate --tag billing,integration
sfdoc generate --type apex --name-filter 'Order*' --tag billing
```

When `--tag` is specified, only Apex classes and triggers with matching tags are included.
```

- [ ] **Step 3: Add search documentation**

```markdown
### Search

HTML output includes a built-in search bar powered by fuse.js. Search by class name, method name, or summary text. No server required — search runs entirely in the browser.
```

- [ ] **Step 4: Update CLI options reference**

Add `--name-filter <PATTERN>` and `--tag <LABELS>` to the CLI options table/list.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: add --name-filter, --tag, and search documentation"
```

---

### Task 14: Update project plan

**Files:**
- Modify: `project-plan.md`

- [ ] **Step 1: Mark phases 25, 26, 27 as done**

Update the status of phases 25, 26, and 27 from `todo` to `done` in the project-plan.md status table.

- [ ] **Step 2: Update phase detail sections**

Replace the design sections for phases 25, 26, 27 with brief implementation notes (same pattern used for phase 24).

- [ ] **Step 3: Commit**

```bash
git add project-plan.md
git commit -m "docs: mark phases 25-27 as done in project plan"
```

---

### Task 15: Integration tests

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Write integration test for `--name-filter`**

This test should verify that the name filter correctly narrows scan results. Create a temp dir with multiple `.cls` files, run the pipeline with `--name-filter 'Order*'`, and verify only matching files appear in the output.

Since integration tests in this project use `tempfile::TempDir` and test the pipeline output, follow the existing pattern in `tests/integration.rs`.

- [ ] **Step 2: Write integration test for `@tag` parsing**

Create a `.cls` file with `@tag billing` in the ApexDoc comment. Parse it and verify `tags` contains `"billing"`.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: all PASS

- [ ] **Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for name-filter and tag features"
```
