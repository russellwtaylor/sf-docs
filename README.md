# sfdoc

> AI-powered documentation for Salesforce projects — Apex classes, triggers, Flows, and validation rules turned into rich, interlinked Markdown or HTML in seconds.

`sfdoc` is a Rust CLI tool that scans your SFDX project, extracts structural metadata from your Salesforce source files, and uses an AI provider of your choice to generate professional wiki-style documentation. It ships with an incremental build cache so that only changed files ever hit the API.

## How it works

```
SFDX project → scan → parse → AI provider → Markdown / HTML output
                                    ↑
                         skips unchanged files
                         (SHA-256 cache)
```

## Features

- Discovers `.cls`, `.trigger`, `.flow-meta.xml`, and `.validationRule-meta.xml` files recursively
- Extracts structural metadata (class signatures, methods, properties, ApexDoc comments, trigger events, flow elements, validation formulas) without a full AST
- Generates rich documentation pages with summaries, parameter tables, usage examples, and cross-links
- Outputs interlinked **Markdown** pages or a self-contained **HTML** site with sidebar navigation
- **Incremental builds** — tracks SHA-256 hashes and skips unchanged files; use `--force` to regenerate everything
- **Multi-provider** — Gemini (default), Groq, OpenAI, or Ollama; swap with a single flag
- API keys stored securely in the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)
- Concurrent API calls with configurable rate limiting

## Supported AI Providers

| Provider      | Flag                 | Default Model             | Notes                       |
| ------------- | -------------------- | ------------------------- | --------------------------- |
| Google Gemini | `gemini` _(default)_ | `gemini-2.5-flash`        | Free tier available         |
| Groq          | `groq`               | `llama-3.3-70b-versatile` | Free tier, very fast        |
| OpenAI        | `openai`             | `gpt-4o-mini`             | Paid                        |
| Ollama        | `ollama`             | `llama3.2`                | Local — no API key required |

## Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later
- An API key for your chosen provider (not required for Ollama)

## Installation

```bash
git clone https://github.com/russellwtaylor/sf-docs
cd sf-docs
cargo install --path .
```

This installs the `sfdoc` binary to `~/.cargo/bin/`. If that directory is not on your `PATH`, add it:

```bash
# zsh (default on macOS)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

Verify the installation:

```bash
sfdoc --version
```

### Build without installing

```bash
cargo build --release
# Binary is at ./target/release/sfdoc
```

## Setup

### 1. Save your API key

```bash
# Default provider (Gemini)
sfdoc auth

# Other providers
sfdoc auth --provider groq
sfdoc auth --provider openai
```

The key is stored encrypted in your OS keychain and never written to disk in plaintext.

### 2. Verify configuration

```bash
sfdoc status
```

Example output:

```
sfdoc 0.1.0

Provider   Name               API Key
------------------------------------------------------------
gemini     Google Gemini      set (env: GEMINI_API_KEY)
groq       Groq               set (OS keychain)
openai     OpenAI             not configured — run `sfdoc auth --provider openai`
ollama     Ollama (local)     not required
```

### 3. Generate documentation

Run from the root of your Salesforce project:

```bash
sfdoc generate
```

This recursively scans `--source-dir` (default: `force-app/main/default`) for `.cls`, `.trigger`, `.flow-meta.xml`, and `.validationRule-meta.xml` files, then writes Markdown output to `docs/`.

## Usage

### Specify paths

```bash
sfdoc generate --source-dir path/to/classes --output path/to/output
```

### Switch provider or model

```bash
# Use Groq with its default model
sfdoc generate --provider groq

# Use OpenAI with a specific model
sfdoc generate --provider openai --model gpt-4o

# Run locally with Ollama
sfdoc generate --provider ollama --model llama3.1
```

### HTML output

Generate a self-contained static site instead of Markdown:

```bash
sfdoc generate --format html
```

HTML output is written to `site/` by default (separate from the Markdown `docs/` directory). Override with `--output`:

```bash
sfdoc generate --format html --output public
```

No external dependencies — works fully offline and can be deployed to any static host.

### Incremental builds

By default, `sfdoc` skips files whose source hasn't changed since the last run (tracked via SHA-256 hashes in `.sfdoc-cache.json`). To force a full regeneration:

```bash
sfdoc generate --force
```

### Tune concurrency

```bash
# Lower if hitting rate limits; raise on a paid plan
sfdoc generate --concurrency 10
```

### Verbose output

```bash
sfdoc generate --verbose
```

### Full options reference

```
sfdoc <COMMAND>

Commands:
  generate    Generate documentation from Apex source files
  auth        Save an AI provider API key to the OS keychain
  status      Show installation status and configuration
  help        Print this message or the help of the given subcommand(s)

sfdoc generate [OPTIONS]

Options:
  --source-dir <PATH>    Path to Apex source directory
                         [default: force-app/main/default/classes]
  --output <PATH>        Output directory for generated files
                         [default: docs (markdown) | site (html)]
  --provider <PROVIDER>  AI provider [default: gemini]
                         [possible values: gemini, groq, openai, ollama]
  --model <MODEL>        Model override (uses provider default if omitted)
  --concurrency <N>      Maximum parallel API requests [default: 3]
  --format <FORMAT>      Output format [default: markdown]
                         [possible values: markdown, html]
  --force                Ignore the incremental build cache; regenerate all docs
  --verbose              Enable verbose logging
  -h, --help             Print help
  -V, --version          Print version

sfdoc auth [OPTIONS]

Options:
  --provider <PROVIDER>  Provider to authenticate [default: gemini]
```

## Output

### Markdown (default)

```
docs/
  index.md                                      # Home page — all types grouped by folder/object
  classes/
    AccountService.md                           # One page per class
  triggers/
    OrderTrigger.md                             # One page per trigger
  flows/
    Account_Onboarding_Flow.md                  # One page per flow
  validation-rules/
    Require_Start_Date.md                       # One page per validation rule
  .sfdoc-cache.json                             # Incremental build cache (do not edit manually)
```

### HTML

```
site/
  index.html                                    # Home page with sidebar navigation
  classes/
    AccountService.html
  triggers/
    OrderTrigger.html
  flows/
    Account_Onboarding_Flow.html
  validation-rules/
    Require_Start_Date.html
```

Markdown and HTML outputs default to separate directories (`docs/` and `site/`) so both formats can coexist without overwriting each other.

### What each page contains

| Section        | Details                                                           |
| -------------- | ----------------------------------------------------------------- |
| Title + badges | Access modifier, `abstract`/`virtual`, `extends`, `implements`    |
| Summary        | One-sentence AI-generated description                             |
| Description    | Full explanation of the class's purpose and behaviour             |
| Properties     | Name, type, description table                                     |
| Methods        | Signature, description, parameter table, return value, exceptions |
| Usage examples | Apex code snippets                                                |
| See Also       | Cross-links to related classes, triggers, flows, and validation rules |

### Flow pages additionally include

| Section          | Details                                                              |
| ---------------- | -------------------------------------------------------------------- |
| Process type     | AutoLaunchedFlow, Flow, Workflow, etc.                               |
| Business process | Plain-English explanation of the business logic for admins           |
| Entry criteria   | When/how the flow is triggered                                       |
| Variables        | Input and output variables with types                                |
| Record ops       | Objects the flow reads, creates, updates, or deletes                 |
| Action calls     | Invocable actions, Apex actions, email alerts, etc.                  |
| Key decisions    | Major branching conditions                                           |
| Admin notes      | Operational considerations for admins                                |

### Validation rule pages additionally include

| Section               | Details                                                          |
| --------------------- | ---------------------------------------------------------------- |
| Active/Inactive badge | Whether the rule is currently enforced                           |
| When it fires         | Plain-English description of the trigger condition for admins    |
| What it protects      | The data-quality or business rule being enforced                 |
| Error condition formula | Raw formula in a code block                                    |
| Formula explanation   | Step-by-step walkthrough of each function and condition          |
| Error message         | The message shown to the user, with the display field if set     |
| Edge cases            | Noteworthy exceptions or gotchas in the formula logic            |

## Example workflow

```bash
# 1. Install
cargo install --path .

# 2. Authenticate
sfdoc auth

# 3. Verify
sfdoc status

# 4. Generate docs
cd my-salesforce-project
sfdoc generate --verbose

# 5. Open the output
open docs/index.md
```

## Tips

- **Free tier** — Groq offers a generous free tier with very low latency; a good alternative to Gemini for large codebases.
- **Offline** — Use `--provider ollama` with a locally running Ollama instance for fully air-gapped generation.
- **Rate limits** — The default concurrency of 3 is conservative for free-tier plans. On a paid plan, `--concurrency 10` or higher is safe.
- **CI/CD** — Set your provider's environment variable as a secret (`GEMINI_API_KEY`, `GROQ_API_KEY`, `OPENAI_API_KEY`) and add `sfdoc generate` as a pipeline step. The incremental cache means only changed files are re-documented on each run.
- **Custom layouts** — Use `--source-dir` if your classes are not under the default SFDX path.

## Project structure

```
src/
  main.rs                       Entry point and pipeline orchestration
  cli.rs                        clap CLI definitions
  config.rs                     API key storage and resolution
  providers.rs                  Provider enum and per-provider defaults
  scanner.rs                    FileScanner trait and all scanner implementations
  parser.rs                     Regex-based Apex class structural parser
  trigger_parser.rs             Apex trigger structural parser
  flow_parser.rs                Salesforce Flow XML structural parser
  validation_rule_parser.rs     Salesforce Validation Rule XML structural parser
  prompt.rs                     AI prompt construction for classes
  trigger_prompt.rs             AI prompt construction for triggers
  flow_prompt.rs                AI prompt construction for flows
  validation_rule_prompt.rs     AI prompt construction for validation rules
  gemini.rs                     Google Gemini API client
  openai_compat.rs              OpenAI-compatible client (Groq, OpenAI, Ollama)
  retry.rs                      Retry logic with exponential backoff
  renderer.rs                   Markdown generation and cross-linking
  html_renderer.rs              Self-contained HTML site generator
  cache.rs                      SHA-256 incremental build cache
  types.rs                      Shared data structures
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with a local project
cargo run -- generate --source-dir /path/to/sf-project/force-app/main/default --verbose
```

All parser and renderer logic is unit-tested. To add a new metadata type, implement the `FileScanner` trait in `scanner.rs`, add a corresponding parser and prompt module, extend `types.rs` with the new metadata and documentation structs, and wire it up in `main.rs`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build, test, and submit changes. We use GitHub Issues and Pull Requests for bugs and features.

## License

See [LICENSE](LICENSE).
