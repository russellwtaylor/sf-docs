# sfdoc

A Rust CLI tool that generates rich, wiki-style Markdown documentation for Salesforce Apex classes using the Gemini AI API.

## Features

- Recursively discovers all `.cls` files in your SFDX project
- Extracts structural metadata (class signatures, methods, properties, ApexDoc comments) without a full AST
- Sends source + metadata to Gemini for AI-generated documentation
- Outputs interlinked Markdown pages — one per class, plus an `index.md`
- Cross-links related classes automatically
- Concurrent API calls with configurable rate limiting
- API key stored securely in the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)

## Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later
- A [Google Gemini API key](https://aistudio.google.com/app/apikey)

## Installation

### From source

```bash
git clone https://github.com/russellwtaylor/sf-docs
cd sfdoc
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

Alternatively, `rustup` installs a shell env file you can source instead:

```bash
source "$HOME/.cargo/env"
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

Gemini API key: not configured — run `sfdoc auth` to set it
```

## Configuration

### Save your API key (recommended)

```bash
sfdoc auth
# Enter your Gemini API key: ••••••••••••••••
# API key saved to your OS keychain.
# You're all set — run `sfdoc generate` to get started.
```

The key is stored encrypted in your OS keychain — it never touches disk in plaintext. On macOS you can verify it in **Keychain Access** by searching for `sfdoc`.

### Environment variable (CI/CD)

For automated pipelines, set the environment variable instead. It takes priority over the keychain:

```bash
export GEMINI_API_KEY=your_api_key_here
```

## Usage

### Basic — generate docs for an SFDX project

Run from the root of your Salesforce project:

```bash
sfdoc generate
```

This uses the default source path (`force-app/main/default/classes/`) and writes output to `docs/`.

### Specify paths explicitly

```bash
sfdoc generate --source-dir path/to/classes --output path/to/output
```

### Use the more powerful model

```bash
sfdoc generate --model pro
```

Available models:

| Value             | Gemini model                              |
| ----------------- | ----------------------------------------- |
| `flash` (default) | `gemini-1.5-flash` — fast, free tier available |
| `pro`             | `gemini-1.5-pro` — higher quality              |

### Tune concurrency

Controls the maximum number of simultaneous Gemini API requests. Lower this if you hit rate limits:

```bash
sfdoc generate --concurrency 3
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
  auth        Save your Gemini API key to the OS keychain
  status      Show installation status and configuration
  help        Print this message or the help of the given subcommand(s)

sfdoc generate [OPTIONS]

Options:
  --source-dir <PATH>    Path to Apex source directory
                         [default: force-app/main/default/classes]
  --output <PATH>        Output directory for generated Markdown files
                         [default: docs]
  --model <MODEL>        Gemini model to use [default: flash] [possible values: flash, pro]
  --concurrency <N>      Maximum number of parallel Gemini API requests [default: 5]
  --verbose              Enable verbose logging
  -h, --help             Print help
  -V, --version          Print version
```

## Output structure

```
docs/
  index.md              # Home page — alphabetical class listing with summaries
  AccountService.md     # One page per class
  ContactService.md
  ...
```

### Example class page

Each class page includes:

- **Title + badges** — access modifier, abstract/virtual, extends/implements
- **Summary** — one-sentence AI-generated description
- **Table of contents**
- **Description** — detailed explanation of the class's purpose
- **Properties table** — name, type, description
- **Methods** — signature, description, parameters table, return value, exceptions
- **Usage examples** — Apex code snippets
- **See Also** — cross-links to related classes

### Example index page

The `index.md` lists every class alphabetically with a one-line summary and a link to its detail page.

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

- **Large codebases**: increase `--concurrency` cautiously — the default of 5 is safe for most Gemini API quotas.
- **Incremental updates**: re-run `sfdoc generate` at any time; it overwrites existing output files.
- **Custom source layouts**: use `--source-dir` if your classes aren't under the default SFDX path.
- **CI/CD integration**: set `GEMINI_API_KEY` as a secret and add `sfdoc generate` as a step to keep docs up to date automatically.

## Project structure

```
src/
  main.rs       Entry point and pipeline orchestration
  cli.rs        clap CLI definitions
  config.rs     API key storage via OS keychain
  scanner.rs    Recursive .cls file discovery
  parser.rs     Regex-based Apex structural parser
  gemini.rs     Gemini API client with rate limiting
  renderer.rs   Markdown generation and cross-linking
  types.rs      Shared data structures
  error.rs      Custom error types
```

## License

See [LICENSE](LICENSE).
