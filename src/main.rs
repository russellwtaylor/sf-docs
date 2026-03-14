mod cli;
mod config;
mod gemini;
mod openai_compat;
mod parser;
mod prompt;
mod providers;
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
use openai_compat::OpenAiCompatClient;
use providers::Provider;
use scanner::{ApexScanner, FileScanner};
use types::{ApexFile, ClassDocumentation, ClassMetadata};

/// Unified client enum for dispatch without dynamic dispatch overhead.
enum DocClient {
    Gemini(Arc<GeminiClient>),
    OpenAiCompat(Arc<OpenAiCompatClient>),
}

impl DocClient {
    async fn document_class(
        &self,
        file: &ApexFile,
        metadata: &ClassMetadata,
    ) -> Result<ClassDocumentation> {
        match self {
            DocClient::Gemini(c) => c.document_class(file, metadata).await,
            DocClient::OpenAiCompat(c) => c.document_class(file, metadata).await,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth(args) => {
            run_auth(&args.provider)?;
        }
        Commands::Status => {
            run_status();
        }
        Commands::Generate(args) => {
            let provider = &args.provider;
            let model = args
                .model
                .as_deref()
                .unwrap_or(provider.default_model())
                .to_string();
            let api_key = resolve_api_key(provider)?;

            if args.verbose {
                eprintln!("Provider:    {}", provider.display_name());
                eprintln!("Model:       {model}");
                eprintln!("Source dir:  {}", args.source_dir.display());
                eprintln!("Output dir:  {}", args.output.display());
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
            if args.verbose {
                for f in &files {
                    eprintln!("  {}", f.path.display());
                }
            }

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

            // Build the appropriate client
            let client = match provider {
                Provider::Gemini => DocClient::Gemini(Arc::new(GeminiClient::new(
                    api_key,
                    &model,
                    args.concurrency,
                ))),
                _ => DocClient::OpenAiCompat(Arc::new(OpenAiCompatClient::new(
                    api_key,
                    &model,
                    provider.base_url(),
                    args.concurrency,
                    provider.display_name(),
                ))),
            };
            let client = Arc::new(client);

            // Spawn concurrent documentation tasks
            let mut tasks = Vec::new();
            for (file, meta) in files.iter().zip(metadata.iter()) {
                let client = Arc::clone(&client);
                let file = file.clone();
                let meta = meta.clone();
                tasks.push(tokio::spawn(async move {
                    client.document_class(&file, &meta).await
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
    println!("{:<10} {:<20} {}", "Provider", "Name", "API Key");
    println!("{}", "-".repeat(60));

    for provider in Provider::all() {
        let status = if !provider.requires_api_key() {
            "not required".to_string()
        } else if provider
            .env_var()
            .and_then(|v| std::env::var(v).ok())
            .map_or(false, |v| !v.is_empty())
        {
            format!(
                "set (env: {})",
                provider.env_var().unwrap_or("")
            )
        } else if has_stored_key(provider) {
            "set (OS keychain)".to_string()
        } else {
            format!("not configured — run `sfdoc auth --provider {}`", provider.cli_name())
        };

        println!("{:<10} {:<20} {}", provider.cli_name(), provider.display_name(), status);
    }
}

fn run_auth(provider: &Provider) -> Result<()> {
    if !provider.requires_api_key() {
        println!(
            "{} runs locally and does not require an API key.",
            provider.display_name()
        );
        println!("Make sure Ollama is running: https://ollama.com");
        return Ok(());
    }

    if has_stored_key(provider) {
        println!(
            "An API key for {} is already stored in your OS keychain.",
            provider.display_name()
        );
        print!("Overwrite it? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
        delete_api_key(provider)?;
    }

    let prompt = format!("Enter your {} API key: ", provider.display_name());
    let key = rpassword::prompt_password(&prompt).context("Failed to read API key")?;

    if key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty.");
    }

    save_api_key(provider, key.trim())?;

    println!(
        "API key for {} saved to your OS keychain.",
        provider.display_name()
    );
    println!("You're all set — run `sfdoc generate --provider {}` to get started.", provider.cli_name());

    Ok(())
}
