mod cli;
mod config;
mod error;
mod gemini;
mod parser;
mod renderer;
mod scanner;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

use cli::{Cli, Commands};
use config::{delete_api_key, has_stored_key, resolve_api_key, save_api_key};
use gemini::GeminiClient;
use scanner::{ApexScanner, FileScanner};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => {
            run_auth()?;
        }
        Commands::Status => {
            run_status();
        }
        Commands::Generate(args) => {
            let api_key = resolve_api_key()?;

            if args.verbose {
                eprintln!("Source dir: {}", args.source_dir.display());
                eprintln!("Output dir: {}", args.output.display());
                eprintln!("Model: {}", args.model);
                eprintln!("Concurrency: {}", args.concurrency);
            }

            // Scan for .cls files
            let scanner = ApexScanner;
            let files = scanner
                .scan(&args.source_dir)
                .with_context(|| format!("Failed to scan {}", args.source_dir.display()))?;

            if files.is_empty() {
                anyhow::bail!("No .cls files found in {}", args.source_dir.display());
            }

            println!("Found {} Apex class file(s)", files.len());

            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
            );

            // Parse each file
            let metadata: Vec<_> = files
                .iter()
                .map(|f| parser::parse_apex_class(&f.raw_source))
                .collect::<Result<_>>()?;

            // Collect all class names for cross-linking
            let all_class_names: Vec<String> =
                metadata.iter().map(|m| m.class_name.clone()).collect();

            // Generate docs via Gemini API
            let gemini = Arc::new(GeminiClient::new(api_key, &args.model, args.concurrency));

            let mut tasks = Vec::new();
            for (file, meta) in files.iter().zip(metadata.iter()) {
                let gemini = Arc::clone(&gemini);
                let file = file.clone();
                let meta = meta.clone();
                tasks.push(tokio::spawn(async move {
                    gemini.document_class(&file, &meta).await
                }));
            }

            let mut contexts = Vec::new();
            for (task, meta) in tasks.into_iter().zip(metadata.into_iter()) {
                let documentation = task.await??;
                pb.inc(1);
                contexts.push(renderer::RenderContext {
                    metadata: meta,
                    documentation,
                    all_class_names: all_class_names.clone(),
                });
            }

            pb.finish_with_message("Done");

            // Render and write output
            renderer::write_output(&args.output, &contexts)?;
            println!("Documentation written to {}", args.output.display());
        }
    }

    Ok(())
}

fn run_status() {
    println!("sfdoc {}", env!("CARGO_PKG_VERSION"));
    println!();

    // API key source
    let key_source = if std::env::var("GEMINI_API_KEY").map_or(false, |v| !v.is_empty()) {
        "set (GEMINI_API_KEY environment variable)"
    } else if has_stored_key() {
        "set (OS keychain)"
    } else {
        "not configured — run `sfdoc auth` to set it"
    };
    println!("Gemini API key: {}", key_source);
}

fn run_auth() -> Result<()> {
    if has_stored_key() {
        println!("A Gemini API key is already stored in your OS keychain.");
        print!("Overwrite it? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
        delete_api_key()?;
    }

    let key = rpassword::prompt_password("Enter your Gemini API key: ")
        .context("Failed to read API key")?;

    if key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty.");
    }

    save_api_key(key.trim())?;

    println!("API key saved to your OS keychain.");
    println!("You're all set — run `sfdoc generate` to get started.");

    Ok(())
}
