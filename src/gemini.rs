use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::rate_limit::RpmLimiter;
use crate::retry::{self, MAX_RETRIES};

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// ---------------------------------------------------------------------------
// Gemini REST API request/response shapes
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "systemInstruction")]
    system_instruction: Content,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    temperature: f32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    semaphore: Arc<Semaphore>,
    rate_limiter: Option<Arc<RpmLimiter>>,
}

impl GeminiClient {
    pub fn new(api_key: String, model: &str, concurrency: usize, rpm: u32) -> Result<Self> {
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
            semaphore: Arc::new(Semaphore::new(concurrency)),
            rate_limiter,
        })
    }

    /// Send a single (system, user) prompt to Gemini with retry logic.
    /// Returns the raw JSON string from the first candidate's text part.
    async fn send_request_impl(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request = GenerateRequest {
            system_instruction: Content {
                role: None,
                parts: vec![Part {
                    text: system_prompt.to_string(),
                }],
            },
            contents: vec![Content {
                role: Some("user".to_string()),
                parts: vec![Part {
                    text: user_prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                temperature: 0.2,
            },
        };

        let url = format!("{}/{}:generateContent", GEMINI_BASE_URL, self.model);

        // Acquire a rate-limit token before starting (counts one logical call, not retries).
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        let mut attempt = 0u32;
        loop {
            let response = match self
                .client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) if retry::is_retryable_error(&e) && attempt < MAX_RETRIES => {
                    eprintln!(
                        "Network error calling Gemini API (attempt {}/{}): {e}",
                        attempt + 1,
                        MAX_RETRIES
                    );
                    retry::sleep_for_retry(None, "", attempt, "Gemini API").await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e).context("Failed to send request to Gemini API"),
            };

            if response.status().is_success() {
                let generate_response: GenerateResponse = response
                    .json()
                    .await
                    .context("Failed to deserialize Gemini response")?;

                return generate_response
                    .candidates
                    .into_iter()
                    .next()
                    .and_then(|c| c.content.parts.into_iter().next())
                    .map(|p| p.text)
                    .context("Gemini returned an empty response");
            }

            let status = response.status();
            // Extract Retry-After header before consuming the body.
            let retry_after = retry::parse_retry_after_header(response.headers());
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

            if retry::should_retry(status.as_u16()) && attempt < MAX_RETRIES {
                if status.as_u16() == 429 && is_quota_exhausted(&body) {
                    anyhow::bail!(
                        "Gemini API quota exhausted (free tier limit reached).\n\
                         Enable billing on your Google AI project to continue:\n\
                         https://aistudio.google.com/plan_information"
                    );
                }
                retry::sleep_for_retry(retry_after, &body, attempt, "Gemini API").await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("Gemini API error {status}: {body}");
        }
    }

}

#[async_trait]
impl crate::doc_client::DocClient for GeminiClient {
    async fn send_request(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let _permit = self.semaphore.acquire().await?;
        self.send_request_impl(system_prompt, user_prompt).await
    }

    fn provider_name(&self) -> &str {
        "Gemini"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns true if the 429 body indicates a hard quota exhaustion (limit: 0)
/// rather than a transient per-minute rate limit that is worth retrying.
fn is_quota_exhausted(body: &str) -> bool {
    // The API returns "limit: 0" in the message when the free tier is fully exhausted.
    body.contains("limit: 0")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::prompt::build_prompt;
    use crate::types::{ClassMetadata, MethodMetadata, ParamMetadata, SourceFile};
    use std::path::PathBuf;

    fn make_file(source: &str) -> SourceFile {
        SourceFile {
            path: PathBuf::from("AccountService.cls"),
            filename: "AccountService.cls".to_string(),
            raw_source: source.to_string(),
        }
    }

    fn make_metadata() -> ClassMetadata {
        ClassMetadata {
            class_name: "AccountService".to_string(),
            access_modifier: "public".to_string(),
            methods: vec![MethodMetadata {
                name: "processAccounts".to_string(),
                access_modifier: "public".to_string(),
                return_type: "void".to_string(),
                is_static: false,
                params: vec![ParamMetadata {
                    param_type: "List<Account>".to_string(),
                    name: "accounts".to_string(),
                }],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn prompt_contains_class_name() {
        let file = make_file("public class AccountService {}");
        let meta = make_metadata();
        let prompt = build_prompt(&file, &meta);
        assert!(prompt.contains("AccountService"));
        assert!(prompt.contains("processAccounts"));
        assert!(prompt.contains("```apex"));
    }

    #[test]
    fn prompt_includes_apexdoc_when_present() {
        let source = "/** Service for accounts. */\npublic class AccountService {}";
        let file = make_file(source);
        let mut meta = make_metadata();
        meta.existing_comments = vec!["/** Service for accounts. */".to_string()];
        let prompt = build_prompt(&file, &meta);
        assert!(prompt.contains("Service for accounts"));
    }
}
