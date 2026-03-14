use anyhow::{Context, Result};

use crate::providers::Provider;

const KEYRING_SERVICE: &str = "sfdoc";

/// Store the API key for a provider in the OS keychain.
pub fn save_api_key(provider: &Provider, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.keychain_key())
        .context("Failed to access keychain")?;
    entry
        .set_password(key)
        .context("Failed to save API key to keychain")
}

/// Delete the API key for a provider from the OS keychain.
pub fn delete_api_key(provider: &Provider) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.keychain_key())
        .context("Failed to access keychain")?;
    entry
        .delete_password()
        .context("Failed to delete API key from keychain")
}

/// Retrieve the stored API key for a provider, or None if not set.
pub fn load_api_key(provider: &Provider) -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.keychain_key())
        .context("Failed to access keychain")?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::Error::from(e).context("Failed to read API key from keychain")),
    }
}

/// Resolve the API key for a provider using priority: env var > OS keychain.
/// Returns an empty string for providers that don't require a key (Ollama).
pub fn resolve_api_key(provider: &Provider) -> Result<String> {
    if !provider.requires_api_key() {
        return Ok(String::new());
    }

    // 1. Environment variable takes priority (CI/CD, one-off overrides)
    if let Some(env_var) = provider.env_var() {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    // 2. OS keychain (set via `sfdoc auth`)
    if let Some(key) = load_api_key(provider)? {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 3. Helpful error
    let env_var = provider.env_var().unwrap_or("(none)");
    anyhow::bail!(
        "No API key found for {}.\n\
         Run `sfdoc auth --provider {}` to save your key, \
         or set the {} environment variable.",
        provider.display_name(),
        provider.cli_name(),
        env_var,
    )
}

/// Returns true if a key is stored in the keychain for this provider.
pub fn has_stored_key(provider: &Provider) -> bool {
    load_api_key(provider).ok().flatten().is_some()
}
