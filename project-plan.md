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
      status: pending
    - id: flexipages
      content: "Phase 21: FlexiPages / Lightning pages — XML parser, component list per page, link to LWC/Flows"
      status: pending
    - id: custom-metadata
      content: "Phase 22: Custom Metadata Types — list types and records, optional AI descriptions (lower priority)"
      status: pending
    - id: aura
      content: "Phase 23: Aura components — scanner, parser, docs (optional; only if Aura-heavy codebase)"
      status: pending
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
    CLI["CLI Layer\n(clap)"] --> Scanner["File Scanners\n(Apex · Trigger · Flow · VR · Object · LWC)"]
    Scanner --> Parser["Parsers\n(parser · trigger_parser · flow_parser\nvalidation_rule_parser · object_parser · lwc_parser)"]
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
    end

    Cls & Trg & Flw & VR & Obj & LWC --> Scanner
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
