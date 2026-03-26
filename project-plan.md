---
name: sfdoc CLI Tool
overview: >
    Build `sfdoc`, a Rust CLI tool that scans Salesforce project directories
    for Apex, trigger, Flow, validation-rule, custom object, LWC, and other
    metadata source files, uses an AI provider to generate rich documentation,
    and outputs interlinked wiki-style Markdown or self-contained HTML pages.
todos:
    - id: scaffold
      content: "Phase 1: Scaffold Rust project (Cargo.toml, module structure, clap CLI definitions)"
      status: done
    - id: scanner
      content: "Phase 2: Implement file scanner — recursive .cls file discovery with SFDX-aware defaults"
      status: done
    - id: parser
      content: "Phase 3: Build lightweight Apex structural parser (regex-based class/method/property extraction)"
      status: done
    - id: ai-client
      content: "Phase 4: Implement Gemini API client with prompt engineering, rate limiting, and structured response parsing"
      status: done
    - id: renderer
      content: "Phase 5: Build Markdown renderer with cross-linking, index generation, and wiki-style page layout"
      status: done
    - id: auth
      content: "Phase 7: sfdoc auth — secure API key storage via OS keychain (keyring crate)"
      status: done
    - id: status
      content: "Phase 8: sfdoc status — installation check and configuration summary"
      status: done
    - id: incremental
      content: "Phase 9: Incremental builds — SHA-256 file hashing, skip unchanged files, --force flag"
      status: done
    - id: triggers
      content: "Phase 10: Apex Trigger support (.trigger files) — dedicated scanner, parser, prompt, and renderer"
      status: done
    - id: html-output
      content: "Phase 11: HTML output mode — self-contained static site with sidebar navigation"
      status: done
    - id: multi-provider
      content: "Phase 12: Multi-provider AI support — Gemini, Groq, OpenAI, Ollama via unified DocClient"
      status: done
    - id: efficiency
      content: "Phase 13: Performance improvements — rayon parallel parsing, Arc sharing, JoinSet unified queue"
      status: done
    - id: namespace-grouping
      content: "Phase 14: Group index by namespace/folder structure instead of flat alphabetical list"
      status: done
    - id: e2e-test
      content: "Phase 15: End-to-end integration tests — fixture .cls files, mock HTTP server, full pipeline assertions"
      status: done
    - id: flows
      content: "Phase 16: Salesforce Flow documentation — XML scanner, structural parser, AI prompt, flow renderer"
      status: done
    - id: validation-rules
      content: "Phase 17: Validation Rule documentation — XML scanner/parser, formula explanation, per-object grouping"
      status: done
    - id: custom-objects
      content: "Phase 18: Custom Object documentation — object/field metadata, link validation rules, flows, Apex"
      status: done
    - id: lwc
      content: "Phase 19: Lightning Web Components — scanner, parser (@api, structure), AI prompt, renderer, cross-link to Apex"
      status: done
    - id: apex-interfaces
      content: "Phase 20: Apex interface support — distinguish interface vs class in parser, index section, implementors list"
      status: done
    - id: flexipages
      content: "Phase 21: FlexiPages / Lightning pages — XML parser, component list per page, link to LWC/Flows"
      status: done
    - id: custom-metadata
      content: "Phase 22: Custom Metadata Types — list types and records, optional AI descriptions (lower priority)"
      status: done
    - id: aura
      content: "Phase 23: Aura components — scanner, parser, docs (optional; only if Aura-heavy codebase)"
      status: done
    - id: type-filter
      content: "Phase 24: --type filter flag — generate docs only for selected metadata types (e.g. --type lwc,apex,flows)"
      status: done
    - id: name-filter
      content: "Phase 25: --name-filter / --glob flag — filter by name pattern (e.g. --name-filter 'Order*')"
      status: done
    - id: html-search
      content: "Phase 26: Client-side search in HTML site — inline lunr.js or fuse.js for searching by class/method/field name"
      status: done
    - id: tag-filter
      content: "Phase 27: Tag/label filtering — support @tag annotations in ApexDoc; filter index by tag in HTML site"
      status: done
    - id: single-file-update
      content: "Phase 28: sfdoc update <file> — re-document a single file without a full rebuild"
      status: todo
    - id: diff-aware-docs
      content: "Phase 29: Diff-aware summaries — include git diff in AI prompt so docs describe what changed"
      status: todo
    - id: quality-scores
      content: "Phase 30: Confidence/quality scores — AI rates doc completeness; flag low-context methods"
      status: todo
    - id: dependency-graph
      content: "Phase 31: Dependency graph — Mermaid/DOT visualization of class-call, flow-object, and LWC-Apex relationships"
      status: todo
    - id: impact-analysis
      content: "Phase 32: sfdoc impact <Name> — CLI command listing all metadata that references a given class/object/flow"
      status: todo
    - id: dead-code-hints
      content: "Phase 33: Dead code hints — flag classes/methods with no incoming cross-references across the documented org"
      status: todo
    - id: watch-mode
      content: "Phase 34: Watch mode — sfdoc watch auto-rebuilds on file save during development"
      status: todo
    - id: confluence-deploy
      content: "Phase 35: Deploy to Confluence/Notion — push generated docs to a team wiki via API"
      status: todo
    - id: ci-deploy
      content: "Phase 36: GitHub Pages / Netlify CI integration — example workflow YAML to publish docs on push to main"
      status: todo
    - id: dark-mode
      content: "Phase 37: Dark mode — CSS dark theme for the HTML output site"
      status: todo
    - id: sfdoc-diff
      content: "Phase 38: sfdoc diff — show which files have changed since last build without regenerating"
      status: todo
    - id: sfdoc-validate
      content: "Phase 39: sfdoc validate — verify all cross-links resolve; catch renamed classes that break doc links (CI-friendly)"
      status: todo
    - id: progress-resumption
      content: "Phase 40: Progress resumption — resume interrupted sfdoc generate runs from the last completed file"
      status: todo
isProject: true
---

# sfdoc — Salesforce Documentation Generator

`sfdoc` is a Rust CLI tool that turns Salesforce metadata into rich,
AI-generated documentation. It scans your SFDX project, extracts structural
metadata from Apex classes, triggers, Flows, and validation rules, sends that
context to an AI provider, and writes interlinked Markdown or HTML pages that
stay up to date automatically through incremental builds.

---

## Architecture Overview

```mermaid
flowchart LR
    CLI["CLI Layer\n(clap)"] --> Scanner["File Scanners\n(Apex · Trigger · Flow · VR · Object · LWC · FlexiPage · CustomMetadata · Aura)"]
    Scanner --> Parser["Parsers\n(parser · trigger_parser · flow_parser · validation_rule_parser\nobject_parser · lwc_parser · flexipage_parser · custom_metadata_parser · aura_parser)"]
    Parser --> Cache["Incremental Cache\n(SHA-256 hashing)"]
    Cache -->|stale files only| AI["AI Providers\n(Gemini · Groq · OpenAI · Ollama)"]
    AI --> Renderer["Renderers\n(Markdown · HTML)"]
    Renderer --> Output["Output\n(index.md / index.html\nper-file pages)"]

    subgraph input [Input — SFDX Project]
        Cls[".cls files"]
        Trg[".trigger files"]
        Flw[".flow-meta.xml"]
        VR[".validationRule-meta.xml"]
        Obj[".object-meta.xml"]
        LWC[".js-meta.xml"]
        FP[".flexipage-meta.xml"]
        CMD[".md-meta.xml"]
        Aura[".cmp files"]
    end

    Cls & Trg & Flw & VR & Obj & LWC & FP & CMD & Aura --> Scanner
```

---

## Project Structure

```
sfdoc/
  Cargo.toml
  src/
    main.rs               # Entry point — CLI dispatch, pipeline orchestration
    cli.rs                # clap CLI definitions (Commands, GenerateArgs, AuthArgs)
    config.rs             # API key storage and resolution (keychain + env var)
    providers.rs          # Provider enum — default models, env vars, base URLs
    scanner.rs                    # File discovery (FileScanner trait and all scanners)
    parser.rs                     # Regex-based Apex class structural parser
    trigger_parser.rs             # Apex trigger structural parser
    flow_parser.rs                # Salesforce Flow XML structural parser (quick-xml)
    validation_rule_parser.rs     # Salesforce Validation Rule XML structural parser
    object_parser.rs              # Custom Object XML structural parser
    lwc_parser.rs                 # LWC @api/slot/component-ref parser (regex-based)
    prompt.rs                     # AI prompt construction for Apex classes
    trigger_prompt.rs             # AI prompt construction for Apex triggers
    flow_prompt.rs                # AI prompt construction for Flows
    validation_rule_prompt.rs     # AI prompt construction for Validation Rules
    object_prompt.rs              # AI prompt construction for Custom Objects
    lwc_prompt.rs                 # AI prompt construction for LWC components
    gemini.rs                     # Google Gemini API client
    openai_compat.rs              # OpenAI-compatible client (Groq, OpenAI, Ollama)
    retry.rs                      # Exponential backoff with Retry-After header support
    renderer.rs                   # Markdown generation and cross-linking
    html_renderer.rs              # Self-contained HTML site generator
    cache.rs                      # SHA-256 incremental build cache (.sfdoc-cache.json)
    types.rs                      # Shared data structures (ApexFile, ClassMetadata, etc.)
  README.md
  project-plan.md
```

---

## Key Dependencies

| Crate                  | Purpose                                 |
| ---------------------- | --------------------------------------- |
| `clap` (derive)        | CLI argument parsing                    |
| `tokio`                | Async runtime for concurrent API calls  |
| `reqwest`              | HTTP client for AI provider APIs        |
| `serde` / `serde_json` | JSON serialization                      |
| `walkdir`              | Recursive directory traversal           |
| `rayon`                | CPU-parallel file parsing               |
| `indicatif`            | Terminal progress bars                  |
| `sha2`                 | SHA-256 hashing for incremental cache   |
| `anyhow`               | Ergonomic error handling                |
| `regex`                | Apex structural parsing                 |
| `keyring`              | OS keychain integration                 |
| `rpassword`            | Masked terminal input for API key entry |

---

## Completed Phases

### Phase 1: Project Scaffolding and CLI ✅

- `Cargo.toml` with all dependencies
- Full module structure established
- `sfdoc generate` subcommand with `--source-dir`, `--output`, `--model`, `--concurrency`, `--verbose`
- API key sourced from environment variable

### Phase 2: File Scanner ✅

- `FileScanner` trait for extensibility across metadata types
- `ApexScanner` implementation using `walkdir`
- Skips `-meta.xml` companion files automatically
- Deterministic sorted output
- `filter_entry` pruning of `.git`, `.sfdx`, `node_modules`, `target`

### Phase 3: Apex Structural Parser ✅

- `OnceLock<Regex>` patterns — compiled once, never in hot loops
- Extracts: class name, access modifier, `abstract`/`virtual`, `extends`, `implements` (including complex generics like `Database.Batchable<SObject>`)
- Extracts method signatures: name, access modifier, return type, parameters, `static` flag
- Extracts properties: name, access modifier, type, `static` flag
- Extracts existing ApexDoc/Javadoc block comments to include in AI context
- Builds a cross-reference list of PascalCase class names (filtering out Apex built-ins)

### Phase 4: AI Client (Gemini) ✅

- Async `reqwest` client against the Gemini `generativelanguage.googleapis.com` API
- `tokio::sync::Semaphore` rate limiting (configurable `--concurrency`)
- Structured prompt: sends full source + extracted metadata + existing ApexDoc comments
- `responseMimeType: application/json` — AI returns JSON directly, no prose scraping
- Parses response into typed `ClassDocumentation` struct

### Phase 5: Markdown Renderer ✅

- One `.md` per class: title, access badges, ToC, description, properties table, method sections with parameter tables, usage examples, See Also cross-links
- `index.md` with alphabetical class listing and one-line summaries
- Cross-linking: scans `relationships` text for known class names, emits relative Markdown links
- `write_output()` creates the output directory and writes all files

### Phase 6: Secure API Key Storage ✅

- `sfdoc auth` subcommand — prompts with masked input via `rpassword`, stores key in OS keychain via `keyring`
- Key is encrypted at rest; never written to disk in plaintext
- Prompts before overwriting an existing key
- `resolve_api_key()` checks environment variable first (CI/CD), then keychain

### Phase 7: Status Command ✅

- `sfdoc status` subcommand — prints version and API key status per provider
- Surfaces whether each key is set via environment variable, keychain, or not configured

### Phase 8: Incremental Builds ✅

- `cache.rs` — SHA-256 hashes each source file before calling the AI
- `.sfdoc-cache.json` persisted in the output directory; maps file paths to `{ hash, model, documentation }`
- Cache is invalidated per file if: source changed, or `--model` changed
- `--force` flag bypasses the cache for a full regeneration
- Separate cache maps for classes and triggers; backward-compatible with old cache files

### Phase 9: Apex Trigger Support ✅

- `TriggerScanner` — discovers `.trigger` files via the same `FileScanner` trait
- `trigger_parser.rs` — parses trigger declaration syntax (`trigger Foo on SObject (events)`)
- `trigger_prompt.rs` — dedicated AI prompt for triggers covering event handlers and handler classes
- Trigger-specific renderer template: event handler table, handler class cross-links, usage notes
- Triggers included in `index.md` / `index.html` alongside classes

### Phase 10: HTML Output Mode ✅

- `--format html` flag on `sfdoc generate`
- `html_renderer.rs` — self-contained static site; all CSS inlined, no external dependencies
- Sidebar navigation listing all classes and triggers
- Works fully offline; suitable for deployment to any static host

### Phase 11: Multi-Provider AI Support ✅

- `providers.rs` — `Provider` enum (Gemini, Groq, OpenAI, Ollama) with per-provider defaults
- `openai_compat.rs` — shared OpenAI-compatible client for Groq, OpenAI, and Ollama
- `DocClient` enum in `main.rs` — static dispatch, zero `dyn` overhead
- `sfdoc auth --provider <name>` — per-provider keychain storage
- Environment variable override per provider (`GEMINI_API_KEY`, `GROQ_API_KEY`, etc.)
- Ollama requires no API key; runs fully locally

### Phase 12: Performance & Efficiency ✅

- **Parallel parsing** — rayon `par_iter()` across class and trigger files
- **`Arc<Vec<_>>` sharing** — task closures receive a pointer clone, not a full `raw_source` clone
- **Unified `JoinSet` work queue** — class and trigger API calls share concurrency slots; results processed as they arrive
- **`Arc<str>` for model string** — cheap pointer copies into task closures
- **`cache.update` takes `&str`** — removes one redundant `String` allocation per file processed
- **Scanner `filter_entry`** — prunes known noise directories early in directory traversal

### Phase 13: Namespace / Folder Grouping in Index ✅

- `folder: String` added to `RenderContext` and `TriggerRenderContext` — derived at build time from the relative path between `--source-dir` and each file's parent directory
- Markdown index groups classes and triggers by folder under `###` subsections, sorted alphabetically; single-folder projects render flat (no extra heading)
- HTML index groups with `<h3>` headings per folder group
- HTML sidebar groups entries by folder with subtle uppercase folder labels; single-folder projects remain flat
- Markdown output defaults to `docs/`, HTML output defaults to `site/` — both formats coexist without overwriting each other; `--output` overrides the default for either format

### Phase 17: Validation Rule Documentation ✅

- `ValidationRuleScanner` — discovers `*.validationRule-meta.xml` under the `objects/` subtree; sorted by path for deterministic output
- `validation_rule_parser.rs` — streaming quick-xml parser that extracts rule name (from filename), object name (from directory structure), active status, description, error condition formula (including multi-line), error display field, and error message; returns proper errors on invalid UTF-8 instead of silently substituting empty strings
- `ValidationRuleMetadata` and `ValidationRuleDocumentation` types in `types.rs`
- `validation_rule_prompt.rs` — dedicated AI prompt asking for: one-sentence summary, when the rule fires (admin-friendly), what data-quality concern it protects, step-by-step formula explanation, edge cases, and referenced fields/objects
- Renderer template per rule: active/inactive badge, formula code block, error message, AI description, edge cases, See Also cross-links
- Per-object grouping in the Markdown and HTML index; inactive rules clearly flagged; output to `validation-rules/` subdirectory
- `AllNames.validation_rule_names` added for full cross-linking across all four asset types
- Cache support: `validation_rule_entries` map with hash + model invalidation, backward-compatible with old cache files

### Phase 18: Custom Object Documentation ✅

- `ObjectScanner` — discovers `*.object-meta.xml` under `objects/` subtrees using `walkdir`; sorted for deterministic output
- `object_parser.rs` — streaming quick-xml parser extracting label, plural label, description, `deploymentStatus`, `enableHistory`, `enableReports` from the object file; also scans a sibling `fields/` directory to parse field files (type, label, description, required flag, lookup reference)
- `ObjectMetadata`, `ObjectFieldMetadata`, `ObjectDocumentation`, `ObjectFieldDocumentation` types in `types.rs`
- `object_prompt.rs` — AI prompt providing full field list with types and any existing descriptions, asking for: label, one-sentence summary, detailed description, per-field descriptions, usage notes, and relationships to Apex/Flows/Triggers
- Renderer: object page with field table (name/type/required/description), usage notes, cross-linked See Also; index grouped by folder
- Cross-linking: `AllNames.object_names` added; object pages link to Apex classes, triggers, flows, validation rules, and other objects
- Cache: `object_entries` map with hash + model invalidation

### Phase 20: Apex Interface Support ✅

- Updated `re_class()` regex to capture both `class` and `interface` declarations
- Added `re_interface_method()` regex (with `(?m)` multiline) to parse interface method declarations lacking access modifiers
- `ClassMetadata.is_interface: bool` field — set automatically by the parser
- `interface_implementors: HashMap<String, Vec<String>>` added to `AllNames` — built in `main.rs` after parsing by iterating class `implements` lists
- Markdown renderer: "interface" badge on interface pages; "Implemented By:" section; separate "## Interfaces" section in index
- HTML renderer: same badge and section logic
- Unit tests: `parses_interface`, `interface_methods_not_empty`, `class_is_not_interface`

### Phase 21: FlexiPages / Lightning Pages ✅

- `FlexiPageScanner` — discovers `*.flexipage-meta.xml` files; sorted by filename
- `flexipage_parser.rs` — streaming quick-xml parser extracting page type, master label, sobject type (for record pages), description, LWC component names (stripping `c__` prefix), and referenced Flow names
- `FlexiPageMetadata`, `FlexiPageDocumentation` types in `types.rs`
- `flexipage_prompt.rs` — AI prompt asking for summary, description, usage context, key component descriptions, and relationships
- Markdown renderer: one page per flexipage with type badge, component list, flow list, AI description; "## Lightning Pages" section in index
- HTML renderer: full HTML page with sidebar "Lightning Pages" section
- Cache: `flexipage_entries` map with hash + model invalidation
- `flexipage_names: Vec<String>` added to `AllNames` for cross-linking

### Phase 22: Custom Metadata Types ✅

- `CustomMetadataScanner` — discovers `customMetadata/*.md-meta.xml` record files; sorted by filename
- `custom_metadata_parser.rs` — streaming quick-xml parser extracting type name and record developer name (from filename), label, and all field values from `<values>` elements
- `CustomMetadataRecord` type in `types.rs`; no AI call — metadata-only listing
- Markdown renderer: one page per custom metadata type listing all records in a table; "## Custom Metadata Types" section in index; output to `custom-metadata/` subdirectory
- HTML renderer: equivalent HTML output with sidebar section
- No cache needed (no AI call)
- `custom_metadata_type_names: Vec<String>` added to `AllNames`

### Phase 23: Aura Components ✅

- `AuraScanner` — discovers `*.cmp` files under `aura/` directories; uses sibling `.js` file as `raw_source` when available; sorted by filename
- `aura_parser.rs` — regex-based parser using `OnceLock<Regex>` patterns; extracts component name, `aura:attribute` elements (name, type, default, description), `aura:handler` events, and `extends` attribute
- `AuraMetadata`, `AuraAttributeMetadata`, `AuraDocumentation`, `AuraAttributeDocumentation` types in `types.rs`
- `aura_prompt.rs` — AI prompt including attribute table and events, asking for component purpose, usage notes, and cross-links
- Markdown renderer: one page per Aura component with attributes table, events, usage notes, See Also; "## Aura Components" section in index; output to `aura/` subdirectory
- HTML renderer: full HTML page with sidebar "Aura" section
- Cache: `aura_entries` map with hash + model invalidation
- `aura_names: Vec<String>` added to `AllNames` for cross-linking

### Phase 19: Lightning Web Components (LWC) ✅

- `LwcScanner` — discovers `*.js-meta.xml` files inside `lwc/` directories; reads sibling `.js` file as `raw_source` for hashing and AI prompting
- `lwc_parser.rs` — regex-based parser using `OnceLock<Regex>` patterns; extracts `@api` properties and methods from JS source; reads sibling HTML template to extract named/anonymous slots and `<c-*>` component references (converted from kebab-case to camelCase)
- `LwcMetadata`, `LwcApiProp`, `LwcDocumentation`, `LwcPropDocumentation` types in `types.rs`
- `lwc_prompt.rs` — AI prompt including Public API table, Slots list, Referenced Components list, and truncated JS source; structured JSON schema for component_name, summary, description, api_props, usage_notes, relationships
- Renderer: LWC page with Public API table (name/kind/description), Slots table, Usage Notes, cross-linked See Also; index section with folder grouping
- HTML renderer: full HTML page for LWC with sidebar including "Components" section; cross-linking to/from Apex classes, triggers, flows, validation rules, and objects
- `AllNames.lwc_names` added for cross-linking across all asset types
- Cache: `lwc_entries` map with hash + model invalidation, backward-compatible

---

## Upcoming Phases

### Phase 15: End-to-End Integration Tests

**Goal:** Provide a full-pipeline integration test suite that catches regressions without requiring a live AI API.

**Design:**

- `tests/fixtures/` — a curated set of realistic `.cls` and `.trigger` files covering edge cases: abstract classes, interfaces, inner classes, complex generics, full ApexDoc comments, no ApexDoc
- Mock HTTP server (e.g. `wiremock` crate) returning canned JSON responses for each fixture
- Integration tests assert on specific content in generated `.md` files: class names present, method sections rendered, cross-links correct, index entries accurate
- One test exercises `--force` flag to verify cache bypass; one exercises incremental mode to verify unchanged files are skipped

**Impact:** Prevents silent regressions in the parser, prompt, or renderer without live API calls.

---

### Phase 16: Salesforce Flow Documentation

**Goal:** Generate AI documentation for Salesforce Flows (`.flow-meta.xml` files), making the tool useful beyond Apex.

**Salesforce Flow metadata overview:**
Flows are stored as XML under `force-app/main/default/flows/`. A flow definition contains a label, process type (e.g. `AutoLaunchedFlow`, `Flow`, `Workflow`), optional description, and a graph of elements including variables, decisions, loops, assignments, screens (for screen flows), record operations, and action calls.

**Design:**

- `FlowScanner` — scans for `*.flow-meta.xml` files; skips non-flow XML
- `flow_parser.rs` — XML parser (using the `quick-xml` crate) that extracts:
    - Flow label, API name, process type, description
    - Input/output variables (name, type, direction)
    - Screen elements (for screen flows): field names and labels
    - Decision elements: condition counts
    - Record operations: object names and operation types (lookup, create, update, delete)
    - Action calls: action names and types (invocable actions, Apex, email alerts, etc.)
- `FlowMetadata` and `FlowDocumentation` types in `types.rs`
- `flow_prompt.rs` — AI prompt that sends the structural summary and asks for: plain-English description, explanation of the business process, entry criteria, key decision points, and considerations for administrators
- Flow renderer template: process type badge, trigger/entry section, element summary table, AI description, usage notes
- Flows included in index alongside classes and triggers, grouped by process type

**Key implementation notes:**

- `quick-xml` is a streaming parser; extract only needed elements rather than deserialising the full schema
- Flow XML can be large (hundreds of elements); the prompt should send the structural summary, not the raw XML
- Cross-links: if a flow calls an Apex action or references a class, link to that class's page

**New CLI behaviour:**

```
sfdoc generate --source-dir force-app/main/default
```

With no flags, scans for `.cls`, `.trigger`, and `.flow-meta.xml` files automatically.

---

### Phase 17: Validation Rule Documentation

**Goal:** Generate AI documentation for Salesforce Validation Rules, giving admins and developers a plain-English explanation of each rule's formula and business intent.

**Salesforce Validation Rule metadata overview:**
Validation rules are stored per-object under `force-app/main/default/objects/{ObjectName}/validationRules/{RuleName}.validationRule-meta.xml`. Each rule contains an `active` flag, optional `description`, an `errorConditionFormula` (Salesforce formula syntax that evaluates to `true` when the record is invalid), an optional `errorDisplayField`, and an `errorMessage` shown to the user.

**Design:**

- `ValidationRuleScanner` — walks the `objects/` subtree and collects all `*.validationRule-meta.xml` files, grouping by object name
- `validation_rule_parser.rs` — XML parser that extracts:
    - Rule name, active status, description
    - Error condition formula (raw)
    - Error display field, error message
    - Parent object name (derived from directory structure)
- `ValidationRuleMetadata` and `ValidationRuleDocumentation` types in `types.rs`
- `validation_rule_prompt.rs` — AI prompt that sends the rule name, formula, error message, and object name, and asks for: plain-English explanation of when the rule fires, what it is protecting, and any edge cases in the formula
- Renderer template per validation rule: active badge, formula block, error message, AI description
- Per-object grouping in the index: all rules for `Account` listed under an Account section, etc.

**Key implementation notes:**

- Validation rules do not require the full Salesforce formula evaluator; the raw formula is sent to the AI for explanation
- Inactive rules (`<active>false</active>`) should be flagged with an "Inactive" badge in the rendered output
- The `errorConditionFormula` can span multiple lines and contain nested functions; render it in a code block
- Cross-links: if the formula references a field on a related object, note it in the "See Also" section

---

### Phase 18: Custom Object Documentation

**Goal:** Document custom (and standard) objects and their fields so validation rules, flows, and Apex can be linked to the object layer. Object-centric docs tie the rest of the metadata together.

**Salesforce Object metadata overview:** Objects live under `force-app/main/default/objects/{ObjectName}/`. Each object has an `*.object-meta.xml` and a `fields/` directory with `*.field-meta.xml` files. Metadata includes label, description, field names, types, help text, and relationships.

**Design:**

- `ObjectScanner` — discovers object definitions and field metadata from the `objects/` subtree
- `object_parser.rs` (or equivalent) — extract object name, label, description; per field: name, type, label, help text, reference target (for lookups)
- `ObjectMetadata` and `ObjectDocumentation` types; optional AI pass for richer descriptions
- One page per object: purpose, key fields table, then **Validation rules**, **Flows**, and **Apex** that reference this object (cross-links from the existing index/cache)
- Index can group by object or list objects alongside classes/triggers/flows

**Impact:** Single place to see “what is Account / MyObject\_\_c?” and which automation touches it.

---

### Phase 19: Lightning Web Components (LWC)

**Goal:** Document the UI layer so the doc set covers full stack: Apex, triggers, flows, validation rules, and LWC.

**SFDX layout:** `lwc/<componentName>/` contains `*.js`, `*.html`, `*.css`, and `*.xml` (meta). Public API is via `@api` properties and methods.

**Design:**

- `LwcScanner` — discovers LWC directories under `lwc/`, reads `*.js`, `*.html`, `*.xml`
- `lwc_parser.rs` — extract component name, `@api` props, public methods, slots from JS/HTML; optional meta.xml for description
- `LwcMetadata` and `LwcDocumentation` types
- `lwc_prompt.rs` — AI prompt for component purpose, usage notes, and cross-links to Apex they call
- Renderer: one page per component (props table, slots, usage), “See Also” to Apex/Flows
- Index: LWC section alongside Classes, Triggers, Flows

**Impact:** Onboarding and impact analysis for UI and backend in one place.

---

### Phase 20: Apex Interface Support

**Goal:** Treat Apex interfaces as first-class so “implementors” and “implements” relationships are easy to follow.

**Design:**

- In `parser.rs`: detect `interface` vs `class` in the declaration regex; set an `is_interface: bool` (or kind enum) on `ClassMetadata` (or introduce `InterfaceMetadata` if preferred)
- Index: add an “Interfaces” section; on each interface page list all classes that `implements` it
- Reuse existing class pipeline; no new scanner. Optional: link from class pages to “Implements: IMyService” with a link to the interface page

**Impact:** Better navigation for interface-based design; small parser and renderer change.

---

### Phase 21: FlexiPages / Lightning App and Record Pages

**Goal:** Document what appears on each Lightning page (App, Record, Home) for onboarding and change impact.

**SFDX:** `flexiPages/*.flexipage-meta.xml` (and similar for app/record pages).

**Design:**

- `FlexiPageScanner` — collect `*.flexipage-meta.xml` (and related page types)
- `flexipage_parser.rs` — XML parser that extracts page type, label, and a simplified list of regions/components (e.g. LWC names, Flow names)
- One page per FlexiPage: type, object (if record page), list of components with links to LWC/Flow docs
- Optional AI: short summary of the page’s purpose

**Impact:** Answers “what’s on this app or record page?” and links to LWC and Flows.

---

### Phase 22: Custom Metadata Types (Optional)

**Goal:** List custom metadata types and optionally their records so configuration is documented.

**SFDX:** `customMetadata/*.customMetadata-meta.xml` and object definitions for custom metadata types.

**Design:**

- Scanner/parser for custom metadata type definitions and, if desired, record files
- Index or section listing types and records; optional one-line AI description per type
- Lower priority unless the org relies heavily on custom metadata for config

---

### Phase 23: Aura Components (Optional)

**Goal:** If the codebase still uses Aura, document components alongside LWC.

**SFDX:** `aura/<componentName>/*.cmp`, `*.auradoc`, etc.

**Design:**

- `AuraScanner` and parser for component markup and docs
- Same pattern as LWC: one page per component, public API, cross-links
- Only worth adding if the project has substantial Aura usage

---

### Phase 24: `--type` Filter Flag ✅

- `MetadataType` enum with `clap::ValueEnum` — nine variants: `apex`, `triggers`, `flows`, `validation-rules`, `objects`, `lwc`, `flexipages`, `custom-metadata`, `aura`
- `--type` CLI argument on `GenerateArgs` with `value_delimiter = ','` — accepts comma-separated values or repeated flags (`--type apex --type lwc`)
- `GenerateArgs::type_enabled()` helper: returns `true` when no types are specified (all enabled) or the type is in the selected set
- Scanner dispatch in `main.rs` gated via a `scan()` closure that returns `Vec::new()` for excluded types
- Error message distinguishes "no files found at all" from "no files for the selected types"
- Verbose mode prints the selected type list
- Invalid type names rejected by clap with the full list of valid values
- 6 unit tests covering: no flag (all enabled), single type, comma-separated, hyphenated names, invalid type rejection, repeated flag

---

### Phase 25: `--name-filter` / `--glob` Flag ✅

**Goal:** Narrow docs to files matching a name pattern, useful for documenting a single feature area (e.g. all `Order*` classes).

**Design:**

- `--name-filter <pattern>` accepting shell glob syntax (e.g. `Order*`, `*Service`)
- Applied after scanning, before parsing/AI — skips non-matching files entirely
- Works across all metadata types; matches against the logical name (class name, object name, flow label)
- Can be combined with `--type`

**Impact:** Low implementation cost; dramatically improves UX for partial rebuilds on large orgs.

**Implemented:** `--name-filter` added to `GenerateArgs` using `globset` crate. `GenerateArgs::name_matches()` helper compiles the pattern and matches against filename stems. Filter applied in `main.rs` for all metadata types after scanning. Unit tests in `cli.rs` cover no-filter, prefix glob, suffix glob, and contains glob cases.

---

### Phase 26: Client-Side Search in HTML Site ✅

**Goal:** Make the generated HTML site searchable by class, method, field, and flow name without a backend.

**Design:**

- Inline a minified search library (lunr.js or fuse.js) into the HTML output — no external CDN dependency
- Build a JSON search index at generation time covering: page title, metadata type, short summary, and all method/field/property names
- Search bar in the sidebar; results appear as a filtered list with links
- Index JSON can be inlined in a `<script>` tag or written as a companion `search-index.json`

**Impact:** High visibility improvement; makes large doc sites usable without knowing exact names up front.

**Implemented:** fuse.js (minified, inlined) used as the search engine. `html_renderer.rs` builds a JSON search index at generation time and embeds it in a `<script>` tag in `index.html`. A search input in the sidebar filters the entry list in real time using fuse.js fuzzy matching across title, summary, and method names. No external network requests required.

---

### Phase 27: Tag / Label Filtering ✅

**Goal:** Allow teams to categorise docs with `@tag` annotations and filter the index by tag.

**Design:**

- Recognise `@tag <label>` in ApexDoc block comments during parsing; store as `Vec<String>` on `ClassMetadata`
- HTML index: tag pills on each entry; clicking a tag filters the sidebar to matching entries (JS, no rebuild needed)
- `--tag <label>` CLI flag to generate a filtered doc set containing only tagged files
- Tags propagate to the page header as badges

**Impact:** Enables domain-based organisation (e.g. `@tag billing`, `@tag integration`) on top of the folder structure.

**Implemented:** `extract_tags()` in `apex_common.rs` parses `@tag` annotations from ApexDoc comments. `tags: Vec<String>` added to `ClassMetadata` and `TriggerMetadata`. `GenerateArgs::tag_matches()` applies OR logic with case-insensitive comparison. HTML renderer emits tag badges on class/trigger pages and `data-tags` attributes on sidebar items; clicking a tag pill filters the sidebar via JS. Markdown renderer emits tag badges in the page header. `--tag` filter applied in `main.rs` after parsing, before AI calls.

---

### Phase 28: `sfdoc update <file>`

**Goal:** Re-document a single file without scanning and rebuilding the entire project.

**Design:**

- New `sfdoc update <path>` subcommand
- Reads, parses, and re-sends just the specified file to the AI provider
- Updates the cache entry and rewrites only that file's output page
- Re-generates the index to reflect any changed summary

**Impact:** Fast inner loop for developers actively editing Apex; avoids full rebuild just to refresh one class.

---

### Phase 29: Diff-Aware Summaries

**Goal:** When a file has changed since the last build, tell the AI what changed so the docs reflect the delta explicitly.

**Design:**

- Run `git diff HEAD <file>` (or compare against the cached source snapshot) to produce a unified diff
- Include the diff in the AI prompt alongside the full source: "The following changes were made since the last documentation run: …"
- AI is asked to note breaking changes, new methods, or removed functionality in the description
- Falls back to standard prompt if git is unavailable or file is new

**Impact:** Docs stay meaningful through refactors; surfaces breaking API changes automatically.

---

### Phase 30: Confidence / Quality Scores

**Goal:** Surface where generated documentation is likely to be low-quality so teams can prioritise adding ApexDoc comments.

**Design:**

- AI returns an optional `confidence: "high" | "medium" | "low"` field alongside existing doc fields
- Low confidence triggered by: no existing ApexDoc, very short source, complex generic types with no context
- HTML output: small badge on each page and index entry; low-confidence entries highlighted in the index
- Optional `--min-confidence medium` flag to skip low-confidence files and log them as warnings instead

**Impact:** Helps teams know where to invest in adding comments; avoids surfacing misleading AI-generated docs.

---

### Phase 31: Dependency Graph

**Goal:** Visualise how the org's metadata connects — which classes call which, which flows reference which objects.

**Design:**

- Post-render pass over `AllNames` cross-reference data already collected during the build
- Emit a `dependency-graph.md` with a Mermaid `graph LR` diagram and a `dependency-graph.html` with an interactive D3 or vis.js graph
- Nodes: classes, triggers, flows, objects, LWC; edges: calls, implements, references, handles
- `--graph-depth <n>` flag to limit graph to N hops from a given entry point (pairs well with `--name-filter`)

**Impact:** Immediate visual for onboarding and impact analysis; built entirely from data already gathered.

---

### Phase 32: `sfdoc impact <Name>`

**Goal:** Answer "if I change X, what else breaks?" as a CLI command.

**Design:**

- New `sfdoc impact <Name>` subcommand; reads the existing cache/output — no re-scan needed
- Traverses the cross-reference graph in reverse: finds all pages that link to `<Name>`
- Prints a grouped list: Apex classes, triggers, flows, validation rules, LWC, FlexiPages that reference the target
- `--json` flag for CI/scripting use; exit code 1 if any references found (useful as a pre-merge gate)

**Impact:** Instant impact analysis without opening a sandbox; works offline from the cached doc set.

---

### Phase 33: Dead Code Hints

**Goal:** Flag classes, methods, and flows that have no incoming cross-references anywhere in the documented org.

**Design:**

- Post-render pass: for each documented entity, check whether any other page links to it
- Emit a `dead-code-report.md` / section in the index listing unreferenced entities grouped by type
- Distinguish "no references in this project" from "referenced externally" (managed packages, unscanned dirs) via a `--exclude-from-dead-code <glob>` flag
- Integrate with `sfdoc validate` (Phase 39) to surface both link rot and dead code in one pass

**Impact:** Helps identify technical debt and unused automation; especially useful before major refactors.

---

### Phase 34: Watch Mode

**Goal:** Auto-rebuild docs on file save during active development.

**Design:**

- New `sfdoc watch` subcommand using the `notify` crate for cross-platform filesystem events
- On change: re-runs the pipeline for only the modified file (same logic as Phase 28 `sfdoc update`)
- Debounce rapid saves (300 ms); print a concise rebuild summary to stdout
- Ctrl-C to stop; compatible with `--format html` for live-preview workflows

**Impact:** Eliminates the manual rebuild step during development; natural complement to a local HTTP server serving the `site/` directory.

---

### Phase 35: Deploy to Confluence / Notion

**Goal:** Push generated docs to a team wiki so they live alongside existing documentation.

**Design:**

- New `sfdoc deploy --target confluence` / `--target notion` subcommand
- Confluence: REST API (`/wiki/rest/api/content`) — create or update pages under a configured space key; map sfdoc page hierarchy to Confluence page tree
- Notion: Notion API — create/update database entries or pages under a configured parent page ID
- Auth: `sfdoc auth --provider confluence` stores the token/URL in the keychain (same pattern as AI providers)
- `--dry-run` flag to preview what would be created/updated without writing

**Impact:** Docs reach teams who don't browse static HTML; integrates with existing wiki workflows.

---

### Phase 36: GitHub Pages / Netlify CI Integration

**Goal:** Make it easy to publish docs automatically on every push.

**Design:**

- Ship an example `sfdoc-docs.yml` GitHub Actions workflow that: installs `sfdoc`, runs `sfdoc generate --format html`, and deploys `site/` to GitHub Pages
- Ship a `netlify.toml` example for Netlify continuous deployment
- Add a `sfdoc ci` subcommand that exits non-zero if any files changed and were not regenerated (stale docs gate)
- Document in README with one-click "Deploy to Netlify" badge

**Impact:** Zero-configuration publishing for teams already using GitHub or Netlify; docs stay current automatically.

---

### Phase 37: Dark Mode

**Goal:** Provide a comfortable reading experience in low-light environments.

**Design:**

- CSS `@media (prefers-color-scheme: dark)` block in the inlined stylesheet — no JS required
- Colour palette: dark background (`#1e1e2e`), muted sidebar, high-contrast headings and code blocks
- Optional manual toggle button (moon/sun icon) that writes preference to `localStorage`
- All existing badge colours adjusted for WCAG AA contrast on dark backgrounds

**Impact:** Low implementation effort; significant comfort improvement for engineers who use dark mode system-wide.

---

### Phase 38: `sfdoc diff`

**Goal:** Show which files have changed since the last build without regenerating anything.

**Design:**

- New `sfdoc diff` subcommand; reads `.sfdoc-cache.json` and re-hashes source files in parallel
- Prints a grouped list of: new files (not in cache), changed files (hash mismatch), and deleted files (in cache but gone)
- `--json` flag for scripting; `--exit-code` flag to exit 1 if any changes detected (CI gate)
- Runs in milliseconds since no AI calls are made

**Impact:** Fast "is the doc set stale?" check for CI pipelines; also useful before running a full regeneration to preview scope.

---

### Phase 39: `sfdoc validate`

**Goal:** Verify that all cross-links in the generated output resolve to real pages; catch renamed classes that silently break navigation.

**Design:**

- New `sfdoc validate` subcommand; reads the output directory (Markdown or HTML)
- Parses all internal links and checks that the target file exists
- Reports broken links grouped by source page with the specific anchor/path that failed
- `--strict` flag to exit non-zero on any broken link (suitable as a CI step after `sfdoc generate`)
- Can also validate that every entry in `AllNames` has a corresponding output page

**Impact:** Prevents silent link rot after refactors; pairs with `sfdoc diff` and `sfdoc generate` for a complete CI documentation pipeline.

---

### Phase 40: Progress Resumption

**Goal:** Allow an interrupted `sfdoc generate` run to resume from where it left off rather than starting over.

**Design:**

- Write a `partial` state to the cache file as each file completes (not just at the end of the run)
- On next run without `--force`: files already present in the cache with the correct hash are skipped as today; partially-written output files are detected and re-generated
- Handle SIGINT gracefully: flush cache before exit so completed work is not lost
- `--resume` flag (explicit) vs. automatic resume based on cache state

**Impact:** Critical for large orgs (hundreds of files) where an interrupted run currently wastes all AI credits spent so far.

---

## Data Flow

```mermaid
flowchart TD
    Start["sfdoc generate"] --> Scan["Scan source directory\n(classes · triggers · flows · validation rules)"]
    Scan --> Parse["Parse each file in parallel\n(rayon)"]
    Parse --> HashCheck["Hash source files\nLoad incremental cache"]
    HashCheck -->|unchanged| CachedDocs["Use cached documentation"]
    HashCheck -->|changed| AI["Send to AI provider\n(unified JoinSet, semaphore-limited)"]
    AI --> NewDocs["New documentation"]
    CachedDocs & NewDocs --> Render["Render pages\n(Markdown or HTML)"]
    Render --> Index["Generate index"]
    Index --> Write["Write output directory"]
    Write --> SaveCache["Persist updated cache"]
```
