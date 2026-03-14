use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::prompt::{build_prompt, SYSTEM_PROMPT};
use crate::types::{ApexFile, ClassDocumentation, ClassMetadata};

const MAX_RETRIES: u32 = 4;
const BASE_BACKOFF_SECS: u64 = 5;

const GEMINI_BASE_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";

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
}

impl GeminiClient {
    pub fn new(api_key: String, model: &str, concurrency: usize) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }

    pub async fn document_class(
        &self,
        file: &ApexFile,
        metadata: &ClassMetadata,
    ) -> Result<ClassDocumentation> {
        let _permit = self.semaphore.acquire().await?;

        let prompt = build_prompt(file, metadata);
        let request = GenerateRequest {
            system_instruction: Content {
                role: None,
                parts: vec![Part {
                    text: SYSTEM_PROMPT.to_string(),
                }],
            },
            contents: vec![Content {
                role: Some("user".to_string()),
                parts: vec![Part { text: prompt }],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                temperature: 0.2,
            },
        };

        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_BASE_URL, self.model, self.api_key
        );

        let mut attempt = 0u32;
        loop {
            let response = self
                .client
                .post(&url)
                .json(&request)
                .send()
                .await
                .context("Failed to send request to Gemini API")?;

            if response.status().is_success() {
                let generate_response: GenerateResponse = response
                    .json()
                    .await
                    .context("Failed to deserialize Gemini response")?;

                let raw_json = generate_response
                    .candidates
                    .into_iter()
                    .next()
                    .and_then(|c| c.content.parts.into_iter().next())
                    .map(|p| p.text)
                    .context("Gemini returned an empty response")?;

                let doc: ClassDocumentation = serde_json::from_str(&raw_json)
                    .with_context(|| format!("Failed to parse Gemini JSON for class '{}':\n{}", metadata.class_name, raw_json))?;

                return Ok(doc);
            }

            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // 429: rate limited — retry with backoff if we have attempts left
            if status.as_u16() == 429 && attempt < MAX_RETRIES {
                // Check if this is a hard quota exhaustion (limit: 0) rather than
                // a transient per-minute spike. If so, fail fast with a helpful message.
                if is_quota_exhausted(&body) {
                    anyhow::bail!(
                        "Gemini API quota exhausted (free tier limit reached).\n\
                         Enable billing on your Google AI project to continue:\n\
                         https://aistudio.google.com/plan_information"
                    );
                }

                let wait = retry_delay_secs(&body, attempt);
                eprintln!(
                    "Rate limited by Gemini API — waiting {wait}s before retry {}/{MAX_RETRIES}...",
                    attempt + 1
                );
                tokio::time::sleep(Duration::from_secs(wait)).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("Gemini API error {status}: {body}");
        }
    }
}

// ---------------------------------------------------------------------------
// Retry helpers
// ---------------------------------------------------------------------------

/// Returns true if the 429 body indicates a hard quota exhaustion (limit: 0)
/// rather than a transient per-minute rate limit that is worth retrying.
fn is_quota_exhausted(body: &str) -> bool {
    // The API returns "limit: 0" in the message when the free tier is fully exhausted.
    body.contains("limit: 0")
}

/// Parses the suggested retry delay from the 429 response body, falling back
/// to exponential backoff based on the attempt number.
fn retry_delay_secs(body: &str, attempt: u32) -> u64 {
    // The API includes e.g. "Please retry in 6.837885891s." in the message.
    // Try to extract the number of seconds from that string.
    if let Some(start) = body.find("retry in ") {
        let after = &body[start + 9..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                // Add a small buffer on top of the suggested delay
                return (secs.ceil() as u64) + 1;
            }
        }
    }
    // Exponential backoff fallback: 5s, 10s, 20s, 40s
    BASE_BACKOFF_SECS * (2u64.pow(attempt))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MethodMetadata, ParamMetadata};
    use std::path::PathBuf;

    fn make_file(source: &str) -> ApexFile {
        ApexFile {
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
