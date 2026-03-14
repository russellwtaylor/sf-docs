use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::providers::Provider;

#[derive(Parser, Debug)]
#[command(
    name = "sfdoc",
    about = "Generate wiki-style Markdown documentation for Salesforce Apex classes",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate documentation from Apex source files
    Generate(GenerateArgs),
    /// Save an AI provider API key to the OS keychain
    Auth(AuthArgs),
    /// Show installation status and configuration
    Status,
}

#[derive(clap::Args, Debug)]
pub struct GenerateArgs {
    /// Path to Apex source directory
    #[arg(long, default_value = "force-app/main/default/classes")]
    pub source_dir: PathBuf,

    /// Output directory for generated Markdown files
    #[arg(long, short, default_value = "docs")]
    pub output: PathBuf,

    /// AI provider to use for documentation generation
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,

    /// Model to use (defaults to the provider's recommended model if not set)
    #[arg(long)]
    pub model: Option<String>,

    /// Maximum number of parallel API requests
    #[arg(long, default_value_t = 3)]
    pub concurrency: usize,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(clap::Args, Debug)]
pub struct AuthArgs {
    /// Provider to authenticate
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,
}
