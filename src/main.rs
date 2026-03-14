mod cache;
mod cli;
mod config;
mod gemini;
mod html_renderer;
mod openai_compat;
mod parser;
mod prompt;
mod providers;
mod renderer;
mod retry;
mod scanner;
mod trigger_parser;
mod trigger_prompt;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;

use cli::{Cli, Commands, OutputFormat};

use config::{delete_api_key, has_stored_key, resolve_api_key, save_api_key};
use gemini::GeminiClient;
use openai_compat::OpenAiCompatClient;
use providers::Provider;
use scanner::{ApexScanner, FileScanner, TriggerScanner};
use types::{AllNames, ApexFile, ClassDocumentation, TriggerDocumentation};

/// Unified client enum for dispatch without dynamic dispatch overhead.
enum DocClient {
    Gemini(Arc<GeminiClient>),
    OpenAiCompat(Arc<OpenAiCompatClient>),
}

impl DocClient {
    async fn document_class(
        &self,
        file: &ApexFile,
        metadata: &types::ClassMetadata,
    ) -> Result<ClassDocumentation> {
        match self {
            DocClient::Gemini(c) => c.document_class(file, metadata).await,
            DocClient::OpenAiCompat(c) => c.document_class(file, metadata).await,
        }
    }

    async fn document_trigger(
        &self,
        file: &ApexFile,
        metadata: &types::TriggerMetadata,
    ) -> Result<TriggerDocumentation> {
        match self {
            DocClient::Gemini(c) => c.document_trigger(file, metadata).await,
            DocClient::OpenAiCompat(c) => c.document_trigger(file, metadata).await,
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
            // Arc<str> so task closures get a cheap pointer copy instead of a String clone.
            let model: Arc<str> =
                Arc::from(args.model.as_deref().unwrap_or(provider.default_model()));

            // Default output directory depends on format: docs/ for Markdown, site/ for HTML.
            let output_dir = args.output.clone().unwrap_or_else(|| {
                if args.format == OutputFormat::Html {
                    std::path::PathBuf::from("site")
                } else {
                    std::path::PathBuf::from("docs")
                }
            });

            if args.verbose {
                eprintln!("Provider:    {}", provider.display_name());
                eprintln!("Model:       {model}");
                eprintln!("Source dir:  {}", args.source_dir.display());
                eprintln!("Output dir:  {}", output_dir.display());
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

            // Scan for .trigger files (non-fatal if none found)
            let trigger_files = TriggerScanner.scan(&args.source_dir).with_context(|| {
                format!("Failed to scan triggers in {}", args.source_dir.display())
            })?;
            if !trigger_files.is_empty() {
                println!("Found {} Apex trigger file(s)", trigger_files.len());
            }

            // Parse classes and triggers in parallel using rayon.
            let class_meta: Vec<_> = files
                .par_iter()
                .map(|f| parser::parse_apex_class(&f.raw_source))
                .collect::<Result<_>>()?;
            let trigger_meta: Vec<_> = trigger_files
                .par_iter()
                .map(|f| trigger_parser::parse_apex_trigger(&f.raw_source))
                .collect::<Result<_>>()?;

            // Wrap in Arc so task closures share the data without cloning raw_source.
            let files = Arc::new(files);
            let class_meta = Arc::new(class_meta);
            let trigger_files = Arc::new(trigger_files);
            let trigger_meta = Arc::new(trigger_meta);

            // Shared cross-linking index
            let all_names = Arc::new(AllNames {
                class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
                trigger_names: trigger_meta
                    .iter()
                    .map(|m| m.trigger_name.clone())
                    .collect(),
            });

            // Load incremental build cache (empty if --force or first run)
            let mut cache = if args.force {
                cache::Cache::default()
            } else {
                cache::Cache::load(&output_dir)
            };

            // Hash every source file
            let class_hashes: Vec<String> = files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();
            let trigger_hashes: Vec<String> = trigger_files
                .par_iter()
                .map(|f| cache::hash_source(&f.raw_source))
                .collect();

            // Partition into cached vs. needs-API
            let mut class_work: Vec<usize> = Vec::new();
            let mut class_docs: Vec<Option<ClassDocumentation>> = vec![None; files.len()];
            for (i, (f, h)) in files.iter().zip(class_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    class_docs[i] = Some(e.documentation.clone());
                } else {
                    class_work.push(i);
                }
            }

            let mut trigger_work: Vec<usize> = Vec::new();
            let mut trigger_docs: Vec<Option<TriggerDocumentation>> =
                vec![None; trigger_files.len()];
            for (i, (f, h)) in trigger_files.iter().zip(trigger_hashes.iter()).enumerate() {
                if let Some(e) = cache.get_trigger_if_fresh(&f.path.to_string_lossy(), h, &model) {
                    trigger_docs[i] = Some(e.documentation.clone());
                } else {
                    trigger_work.push(i);
                }
            }

            let skipped =
                (files.len() - class_work.len()) + (trigger_files.len() - trigger_work.len());
            if skipped > 0 {
                println!("{skipped} file(s) up-to-date — skipping API calls");
            }

            if !class_work.is_empty() || !trigger_work.is_empty() {
                let api_key = resolve_api_key(provider)?;
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

                let total_work = (class_work.len() + trigger_work.len()) as u64;
                let pb = Arc::new(ProgressBar::new(total_work));
                pb.set_style(
                    ProgressStyle::with_template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                    )
                    .expect("invalid progress bar template")
                    .progress_chars("#>-"),
                );

                // Unified work queue: classes and triggers race for the same concurrency
                // slots, so the semaphore is never underutilised when one type dominates.
                enum WorkResult {
                    Class(usize, ClassDocumentation),
                    Trigger(usize, TriggerDocumentation),
                }

                let mut tasks: JoinSet<Result<WorkResult>> = JoinSet::new();

                for &idx in &class_work {
                    let client = Arc::clone(&client);
                    let files = Arc::clone(&files);
                    let class_meta = Arc::clone(&class_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc = client.document_class(&files[idx], &class_meta[idx]).await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Class(idx, doc))
                    });
                }

                for &idx in &trigger_work {
                    let client = Arc::clone(&client);
                    let trigger_files = Arc::clone(&trigger_files);
                    let trigger_meta = Arc::clone(&trigger_meta);
                    let pb_task = Arc::clone(&pb);
                    tasks.spawn(async move {
                        let doc = client
                            .document_trigger(&trigger_files[idx], &trigger_meta[idx])
                            .await?;
                        pb_task.inc(1);
                        Ok(WorkResult::Trigger(idx, doc))
                    });
                }

                // Collect results as they complete (not in spawn order).
                while let Some(res) = tasks.join_next().await {
                    match res?? {
                        WorkResult::Class(idx, doc) => {
                            let key = files[idx].path.to_string_lossy().into_owned();
                            cache.update(key, class_hashes[idx].clone(), &model, doc.clone());
                            class_docs[idx] = Some(doc);
                        }
                        WorkResult::Trigger(idx, doc) => {
                            let key = trigger_files[idx].path.to_string_lossy().into_owned();
                            cache.update_trigger(
                                key,
                                trigger_hashes[idx].clone(),
                                &model,
                                doc.clone(),
                            );
                            trigger_docs[idx] = Some(doc);
                        }
                    }
                }

                pb.finish_with_message("Done");
            }

            // Build render contexts (tasks are all done; Arc::try_unwrap reclaims the Vecs).
            let files = Arc::try_unwrap(files).expect("no other Arc holders after tasks join");
            let class_meta = Arc::try_unwrap(class_meta).expect("no other Arc holders");
            let trigger_files = Arc::try_unwrap(trigger_files).expect("no other Arc holders");
            let trigger_meta = Arc::try_unwrap(trigger_meta).expect("no other Arc holders");

            let class_contexts: Vec<renderer::RenderContext> = files
                .into_iter()
                .zip(class_meta)
                .zip(class_docs)
                .map(|((file, meta), doc)| renderer::RenderContext {
                    folder: compute_folder(&file.path, &args.source_dir),
                    metadata: meta,
                    documentation: doc.expect("every class must have documentation"),
                    all_names: Arc::clone(&all_names),
                })
                .collect();

            let trigger_contexts: Vec<renderer::TriggerRenderContext> = trigger_files
                .into_iter()
                .zip(trigger_meta)
                .zip(trigger_docs)
                .map(|((file, meta), doc)| renderer::TriggerRenderContext {
                    folder: compute_folder(&file.path, &args.source_dir),
                    metadata: meta,
                    documentation: doc.expect("every trigger must have documentation"),
                    all_names: Arc::clone(&all_names),
                })
                .collect();

            // Render and write output
            renderer::write_output(
                &output_dir,
                &args.format,
                &class_contexts,
                &trigger_contexts,
            )?;
            println!("Documentation written to {}", output_dir.display());

            // Persist the updated cache — only reached if all API calls succeeded
            cache.save(&output_dir)?;
        }
    }

    Ok(())
}

/// Returns the relative path from `source_dir` to `file_path`'s parent directory,
/// using forward slashes regardless of platform. Used to group the index by
/// namespace/folder (e.g. `"classes"`, `"classes/account"`).
fn compute_folder(file_path: &Path, source_dir: &Path) -> String {
    file_path
        .parent()
        .and_then(|p| p.strip_prefix(source_dir).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

fn run_status() {
    println!("sfdoc {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("{:<10} {:<20} API Key", "Provider", "Name");
    println!("{}", "-".repeat(60));

    for provider in Provider::all() {
        let status = if !provider.requires_api_key() {
            "not required".to_string()
        } else if provider
            .env_var()
            .and_then(|v| std::env::var(v).ok())
            .is_some_and(|v| !v.is_empty())
        {
            format!("set (env: {})", provider.env_var().unwrap_or(""))
        } else if has_stored_key(provider) {
            "set (OS keychain)".to_string()
        } else {
            format!(
                "not configured — run `sfdoc auth --provider {}`",
                provider.cli_name()
            )
        };

        println!(
            "{:<10} {:<20} {}",
            provider.cli_name(),
            provider.display_name(),
            status
        );
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
    println!(
        "You're all set — run `sfdoc generate --provider {}` to get started.",
        provider.cli_name()
    );

    Ok(())
}
