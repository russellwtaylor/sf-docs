# Contributing to sfdoc

Thanks for your interest in contributing. This document explains how to get set up, run tests, and submit changes.

## Development setup

### Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later

### Build and test

```bash
git clone https://github.com/russellwtaylor/sf-docs
cd sf-docs

# Build
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --all -- --check

# Lint
cargo clippy -- -D warnings
```

To try the CLI against a local Salesforce project:

```bash
cargo run -- generate --source-dir /path/to/sf-project/force-app/main/default/classes --verbose
```

## Submitting changes

1. **Open an issue** (optional but helpful for larger changes)  
   Use [Bug report](.github/ISSUE_TEMPLATE/bug_report.md) or [Feature request](.github/ISSUE_TEMPLATE/feature_request.md) so we can align on the approach.

2. **Branch from `main`**  
   Use a short, descriptive branch name (e.g. `fix/cache-edge-case`, `feat/flow-docs`).

3. **Make your changes**  
   Keep the scope focused. Follow existing style: run `cargo fmt` and fix any `cargo clippy` warnings.

4. **Run the test suite**  
   Ensure `cargo test` and `cargo clippy -- -D warnings` pass locally.

5. **Open a pull request**  
   Target the `main` branch and fill in the [PR template](.github/PULL_REQUEST_TEMPLATE.md). CI will run build, test, format check, and Clippy; all must pass before merge.

## Code and project structure

- **Errors:** Use `anyhow` in application code; add context with `.context("...")` at each `?`.
- **Regex:** Compile once via `std::sync::OnceLock<Regex>` in a helper; never compile inside a hot loop.
- **Tests:** Unit tests live in the same file under `#[cfg(test)] mod tests`. Use `tempfile::TempDir` for filesystem tests.

Module overview:

| Module        | Role |
|---------------|------|
| `src/main.rs` | Entry point, pipeline orchestration |
| `src/cli.rs`  | CLI definitions (clap) |
| `src/parser.rs`, `src/trigger_parser.rs` | Structural parsing (regex-based) |
| `src/gemini.rs`, `src/openai_compat.rs` | AI provider clients |
| `src/renderer.rs`, `src/html_renderer.rs` | Output generation |
| `src/types.rs` | Shared data structures |

To add a new metadata type (e.g. another file kind): implement `FileScanner` in `scanner.rs`, add a parser and prompt module, extend `types.rs`, and wire it up in `main.rs`.

## Questions

Open a [GitHub Discussion](https://github.com/russellwtaylor/sf-docs/discussions) for questions or ideas that aren’t bugs or concrete feature requests.
