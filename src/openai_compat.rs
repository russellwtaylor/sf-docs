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
}

impl OpenAiCompatClient {
    pub fn new(
        api_key: String,
        model: &str,
        base_url: &str,
        concurrency: usize,
        provider_name: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            base_url: base_url.to_string(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            provider_name: provider_name.to_string(),
        }
    }

    pub async fn document_class(
        &self,
        file: &ApexFile,
        metadata: &ClassMetadata,
    ) -> Result<ClassDocumentation> {
        let _permit = self.semaphore.acquire().await?;

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: build_prompt(file, metadata),
                },
            ],
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
            temperature: 0.2,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let mut attempt = 0u32;
        loop {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&request)
                .send()
                .await
                .with_context(|| format!("Failed to send request to {} API", self.provider_name))?;

            if response.status().is_success() {
                let chat_response: ChatResponse = response
                    .json()
                    .await
                    .with_context(|| {
                        format!("Failed to deserialize {} response", self.provider_name)
                    })?;

                let raw_json = chat_response
                    .choices
                    .into_iter()
                    .next()
                    .map(|c| c.message.content)
                    .with_context(|| {
                        format!("{} returned an empty response", self.provider_name)
                    })?;

                let doc: ClassDocumentation = serde_json::from_str(&raw_json).with_context(
                    || {
                        format!(
                            "Failed to parse {} JSON for class '{}':\n{}",
                            self.provider_name, metadata.class_name, raw_json
                        )
                    },
                )?;

                return Ok(doc);
            }

            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 && attempt < MAX_RETRIES {
                let wait = retry_delay_secs(&body, attempt);
                eprintln!(
                    "Rate limited by {} — waiting {wait}s before retry {}/{MAX_RETRIES}...",
                    self.provider_name,
                    attempt + 1
                );
                tokio::time::sleep(Duration::from_secs(wait)).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("{} API error {status}: {body}", self.provider_name);
        }
    }
}

// ---------------------------------------------------------------------------
// Retry helpers
// ---------------------------------------------------------------------------

fn retry_delay_secs(body: &str, attempt: u32) -> u64 {
    if let Some(start) = body.find("retry in ") {
        let after = &body[start + 9..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                return (secs.ceil() as u64) + 1;
            }
        }
    }
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

    #[test]
    fn retry_delay_parses_suggested_seconds() {
        assert_eq!(retry_delay_secs("Please retry in 6.837s.", 0), 8);
    }

    #[test]
    fn retry_delay_falls_back_to_exponential_backoff() {
        assert_eq!(retry_delay_secs("no hint here", 0), 5);
        assert_eq!(retry_delay_secs("no hint here", 1), 10);
        assert_eq!(retry_delay_secs("no hint here", 2), 20);
    }
}
