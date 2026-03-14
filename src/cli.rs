use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Save your Gemini API key to the OS keychain
    Auth,
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

    /// Gemini model to use (e.g. gemini-1.5-flash, gemini-1.5-pro, gemini-2.0-flash)
    #[arg(long, default_value = "gemini-1.5-flash")]
    pub model: String,

    /// Maximum number of parallel Gemini API requests
    #[arg(long, default_value_t = 5)]
    pub concurrency: usize,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}
