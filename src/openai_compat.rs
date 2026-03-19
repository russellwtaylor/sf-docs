use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::rate_limit::RpmLimiter;
use crate::retry::{self, MAX_RETRIES};

// ---------------------------------------------------------------------------
// OpenAI-compatible API shapes (Groq, OpenAI, Ollama)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(rename = "response_format")]
    response_format: ResponseFormat,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct OpenAiCompatClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    semaphore: Arc<Semaphore>,
    provider_name: String,
    rate_limiter: Option<Arc<RpmLimiter>>,
}

impl OpenAiCompatClient {
    pub fn new(
        api_key: String,
        model: &str,
        base_url: &str,
        concurrency: usize,
        provider_name: &str,
        rpm: u32,
    ) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to build HTTP client")?;
        let rate_limiter = (rpm > 0).then(|| Arc::new(RpmLimiter::new(rpm)));
        Ok(Self {
            client,
            api_key,
            model: model.to_string(),
            base_url: base_url.to_string(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            provider_name: provider_name.to_string(),
            rate_limiter,
        })
    }

    /// Send a single (system, user) prompt to the OpenAI-compatible endpoint with retry logic.
    /// Returns the raw JSON string from the first choice's message content.
    async fn send_request_impl(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
            temperature: 0.2,
        };

        let url = format!("{}/chat/completions", self.base_url);

        // Acquire a rate-limit token before starting (counts one logical call, not retries).
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        let mut attempt = 0u32;
        loop {
            let response = match self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) if retry::is_retryable_error(&e) && attempt < MAX_RETRIES => {
                    eprintln!(
                        "Network error calling {} API (attempt {}/{}): {e}",
                        self.provider_name,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    retry::sleep_for_retry(None, "", attempt, &self.provider_name).await;
                    attempt += 1;
                    continue;
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("Failed to send request to {} API", self.provider_name)
                    })
                }
            };

            if response.status().is_success() {
                let chat_response: ChatResponse = response.json().await.with_context(|| {
                    format!("Failed to deserialize {} response", self.provider_name)
                })?;

                return chat_response
                    .choices
                    .into_iter()
                    .next()
                    .map(|c| c.message.content)
                    .with_context(|| format!("{} returned an empty response", self.provider_name));
            }

            let status = response.status();
            // Extract Retry-After header before consuming the body.
            let retry_after = retry::parse_retry_after_header(response.headers());
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

            if retry::should_retry(status.as_u16()) && attempt < MAX_RETRIES {
                retry::sleep_for_retry(retry_after, &body, attempt, &self.provider_name).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("{} API error {status}: {body}", self.provider_name);
        }
    }
}

#[async_trait]
impl crate::doc_client::DocClient for OpenAiCompatClient {
    async fn send_request(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let _permit = self.semaphore.acquire().await?;
        self.send_request_impl(system_prompt, user_prompt).await
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }
}
