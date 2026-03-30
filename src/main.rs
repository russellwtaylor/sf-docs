use sfdoc::cli::{Cli, Commands};
use sfdoc::config::{delete_api_key, has_stored_key, save_api_key};
use sfdoc::providers::Provider;

use anyhow::{Context, Result};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Update(args) => {
            sfdoc::update::run_update(&args).await?;
        }
        Commands::Auth(args) => {
            run_auth(&args.provider)?;
        }
        Commands::Status => {
            run_status();
        }
        Commands::Generate(args) => {
            sfdoc::generate::run_generate(&args).await?;
        }
    }

    Ok(())
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
