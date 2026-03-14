use anyhow::{Context, Result};

const KEYRING_SERVICE: &str = "sfdoc";
const KEYRING_USER: &str = "gemini_api_key";

/// Store the API key in the OS keychain.
pub fn save_api_key(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to access keychain")?;
    entry.set_password(key).context("Failed to save API key to keychain")
}

/// Delete the API key from the OS keychain.
pub fn delete_api_key() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to access keychain")?;
    entry.delete_password().context("Failed to delete API key from keychain")
}

/// Retrieve the API key from the OS keychain, or None if not stored.
fn load_api_key() -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to access keychain")?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::Error::from(e).context("Failed to read API key from keychain")),
    }
}

/// Resolve the API key using priority: env var > OS keychain.
pub fn resolve_api_key() -> Result<String> {
    // 1. Environment variable takes priority (CI/CD, one-off overrides)
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 2. OS keychain (set via `sfdoc auth`)
    if let Some(key) = load_api_key()? {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 3. Helpful error
    anyhow::bail!(
        "No Gemini API key found.\n\
         Run `sfdoc auth` to save your key, or set the GEMINI_API_KEY environment variable."
    )
}

/// Returns true if a key is already stored in the keychain.
pub fn has_stored_key() -> bool {
    load_api_key().ok().flatten().is_some()
}
