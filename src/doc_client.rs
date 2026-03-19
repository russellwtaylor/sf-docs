use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;

/// Trait for AI documentation providers.
///
/// Each provider implements `send_request` (the raw HTTP call with retry)
/// and `provider_name` (for error messages). The generic `document` function
/// handles semaphore-gated prompt dispatch and JSON deserialization.
#[async_trait]
pub trait DocClient: Send + Sync {
    /// Send a (system, user) prompt pair and return the raw response text.
    /// Implementations should handle retries, rate limiting, and concurrency.
    async fn send_request(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;

    /// Human-readable provider name for error messages.
    fn provider_name(&self) -> &str;
}

/// Generic document generation: send prompt, parse JSON response.
pub async fn document<D: DeserializeOwned>(
    client: &dyn DocClient,
    system_prompt: &str,
    user_prompt: &str,
    entity_label: &str,
) -> Result<D> {
    let raw = client.send_request(system_prompt, user_prompt).await?;
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "Failed to parse {} JSON for {}:\n{raw}",
            client.provider_name(),
            entity_label
        )
    })
}
