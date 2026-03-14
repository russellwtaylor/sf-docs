# sfdoc

A Rust CLI tool that generates rich, wiki-style documentation for Salesforce Apex classes and triggers using AI.

## Features

- Recursively discovers all `.cls` and `.trigger` files in your SFDX project
- Extracts structural metadata (class signatures, methods, properties, ApexDoc comments) without a full AST
- Sends source + metadata to your chosen AI provider for documentation generation
- Outputs interlinked Markdown pages — one per class/trigger, plus an `index.md`
- Also renders a self-contained HTML site (`--format html`) with sidebar navigation
- Cross-links related classes automatically
- Incremental builds — skips unchanged files using SHA-256 hashing
- Concurrent API calls with configurable rate limiting
- API keys stored securely in the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)

## Supported AI Providers

| Provider | Flag | Default Model | Notes |
|----------|------|---------------|-------|
| Google Gemini | `gemini` (default) | `gemini-2.5-flash` | Free tier available |
| Groq | `groq` | `llama-3.3-70b-versatile` | Free tier, very fast |
| OpenAI | `openai` | `gpt-4o-mini` | Paid |
| Ollama | `ollama` | `llama3.2` | Local, no API key needed |

## Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later
- An API key for your chosen provider (not needed for Ollama)

## Installation

### From source

```bash
git clone https://github.com/russellwtaylor/sf-docs
cd sf-docs
cargo install --path .
```

This installs the `sfdoc` binary to `~/.cargo/bin/`.

If `rustup` was installed normally, this directory is already on your `PATH`. Verify with:

```bash
sfdoc --version
```

If that returns `command not found`, add the following to your shell profile and restart your terminal:

```bash
# zsh (default on macOS)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc
```

### Build without installing

```bash
cargo build --release
# Binary is at ./target/release/sfdoc
```

## Verify installation

```bash
sfdoc status
```

Example output:

```
sfdoc 0.1.0

Providers:
  gemini   — API key: configured (keychain)
  groq     — API key: not configured
  openai   — API key: not configured
  ollama   — no API key required
```

## Configuration

### Save your API key (recommended)

```bash
# Gemini (default)
sfdoc auth

# Other providers
sfdoc auth --provider groq
sfdoc auth --provider openai
```

The key is stored encrypted in your OS keychain — it never touches disk in plaintext.

### Environment variables (CI/CD)

For automated pipelines, set the environment variable instead. It takes priority over the keychain:

```bash
export GEMINI_API_KEY=your_key_here
export GROQ_API_KEY=your_key_here
export OPENAI_API_KEY=your_key_here
```

Ollama runs locally and requires no API key.

## Usage

### Basic — generate docs for an SFDX project

Run from the root of your Salesforce project:

```bash
sfdoc generate
```

This uses the default source path (`force-app/main/default/classes/`), the Gemini provider, and writes Markdown output to `docs/`.

### Specify paths explicitly

```bash
sfdoc generate --source-dir path/to/classes --output path/to/output
```

### Choose a provider

```bash
sfdoc generate --provider groq
sfdoc generate --provider openai
sfdoc generate --provider ollama
```

### Choose a model

Override the provider's default model:

```bash
sfdoc generate --model gemini-2.5-pro
sfdoc generate --provider groq --model mixtral-8x7b-32768
```

### HTML output

Generate a self-contained static site instead of Markdown:

```bash
sfdoc generate --format html
```

Output is written to `docs/` with an `index.html` and one page per class/trigger. No external dependencies — works offline.

### Incremental builds

By default, `sfdoc` skips classes whose source hasn't changed since the last run (tracked via SHA-256 hashes in `.sfdoc-cache.json`). To force a full regeneration:

```bash
sfdoc generate --force
```

### Tune concurrency

Controls the maximum number of simultaneous API requests. Lower this if you hit rate limits:

```bash
sfdoc generate --concurrency 1
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
  --output <PATH>        Output directory for generated files [default: docs]
  --provider <PROVIDER>  AI provider to use [default: gemini]
                         [possible values: gemini, groq, openai, ollama]
  --model <MODEL>        Model to use (defaults to provider's recommended model)
  --concurrency <N>      Maximum number of parallel API requests [default: 3]
  --format <FORMAT>      Output format [default: markdown]
                         [possible values: markdown, html]
  --force                Regenerate all docs, ignoring the incremental build cache
  --verbose              Enable verbose logging
  -h, --help             Print help
  -V, --version          Print version

sfdoc auth [OPTIONS]

Options:
  --provider <PROVIDER>  Provider to authenticate [default: gemini]
```

## Output structure

### Markdown (default)

```
docs/
  index.md              # Home page — alphabetical class listing with summaries
  AccountService.md     # One page per class or trigger
  OrderTrigger.md
  ...
  .sfdoc-cache.json     # Incremental build cache (do not edit manually)
```

### HTML

```
docs/
  index.html            # Home page with sidebar navigation
  AccountService.html
  OrderTrigger.html
  ...
```

### Example class page

Each page includes:

- **Title + badges** — access modifier, abstract/virtual, extends/implements
- **Summary** — one-sentence AI-generated description
- **Table of contents**
- **Description** — detailed explanation of the class's purpose
- **Properties table** — name, type, description
- **Methods** — signature, description, parameters table, return value, exceptions
- **Usage examples** — Apex code snippets
- **See Also** — cross-links to related classes

## Example workflow

```bash
# 1. Install sfdoc
cargo install --path .

# 2. Save your API key
sfdoc auth

# 3. Verify everything is ready
sfdoc status

# 4. Navigate to your Salesforce project and generate docs
cd my-salesforce-project
sfdoc generate --verbose

# 5. Open the output
open docs/index.md
```

## Tips

- **Rate limits**: the default concurrency of 3 is conservative. If you're on a paid plan, raise it with `--concurrency 10` or higher.
- **Free & fast**: Groq offers a generous free tier with very low latency — great for large codebases.
- **Offline**: use `--provider ollama` with a locally running Ollama instance for fully air-gapped generation.
- **Incremental updates**: re-running `sfdoc generate` only processes files that have changed since the last run. Use `--force` to regenerate everything.
- **Custom source layouts**: use `--source-dir` if your classes aren't under the default SFDX path.
- **CI/CD integration**: set your provider's env var as a secret and add `sfdoc generate` as a pipeline step to keep docs up to date automatically.

## Project structure

```
src/
  main.rs             Entry point and pipeline orchestration
  cli.rs              clap CLI definitions
  config.rs           API key storage and resolution
  providers.rs        Provider enum and per-provider defaults
  scanner.rs          Recursive .cls and .trigger file discovery
  parser.rs           Regex-based Apex class structural parser
  trigger_parser.rs   Apex trigger structural parser
  prompt.rs           AI prompt construction for classes
  trigger_prompt.rs   AI prompt construction for triggers
  gemini.rs           Google Gemini API client
  openai_compat.rs    OpenAI-compatible client (Groq, OpenAI, Ollama)
  retry.rs            Retry logic with exponential backoff
  renderer.rs         Markdown generation and cross-linking
  html_renderer.rs    Self-contained HTML site generator
  cache.rs            SHA-256 incremental build cache
  types.rs            Shared data structures
```

## License

See [LICENSE](LICENSE).
