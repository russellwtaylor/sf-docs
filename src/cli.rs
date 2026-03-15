use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::providers::Provider;

#[derive(clap::ValueEnum, Clone, Debug, Default, PartialEq)]
pub enum OutputFormat {
    /// Wiki-style Markdown files (default)
    #[default]
    Markdown,
    /// Self-contained HTML site with sidebar navigation
    Html,
}

#[derive(Parser, Debug)]
#[command(
    name = "sfdoc",
    about = "Generate wiki-style Markdown documentation for Salesforce source files",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate documentation from Salesforce source files
    Generate(GenerateArgs),
    /// Save an AI provider API key to the OS keychain
    Auth(AuthArgs),
    /// Show installation status and configuration
    Status,
}

#[derive(clap::Args, Debug)]
pub struct GenerateArgs {
    /// Path to Apex source directory
    #[arg(long, default_value = "force-app/main/default")]
    pub source_dir: PathBuf,

    /// Output directory for generated files.
    /// Defaults to `docs` for Markdown output and `site` for HTML output.
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// AI provider to use for documentation generation
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,

    /// Model to use (defaults to the provider's recommended model if not set)
    #[arg(long)]
    pub model: Option<String>,

    /// Maximum number of parallel API requests
    #[arg(long, default_value_t = 3, value_parser = parse_concurrency)]
    pub concurrency: usize,

    /// Output format
    #[arg(long, default_value = "markdown")]
    pub format: OutputFormat,

    /// Regenerate all documentation, ignoring the incremental build cache
    #[arg(long)]
    pub force: bool,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}

fn parse_concurrency(s: &str) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;
    if n == 0 {
        Err("--concurrency must be at least 1".to_string())
    } else {
        Ok(n)
    }
}

#[derive(clap::Args, Debug)]
pub struct AuthArgs {
    /// Provider to authenticate
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,
}
