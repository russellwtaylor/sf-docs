use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::prompt::{build_prompt, SYSTEM_PROMPT};
use crate::retry::{self, MAX_RETRIES};
use crate::types::{ApexFile, ClassDocumentation, ClassMetadata};

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
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
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
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

            if status.as_u16() == 429 && attempt < MAX_RETRIES {
                retry::sleep_for_retry(&body, attempt, &self.provider_name).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("{} API error {status}: {body}", self.provider_name);
        }
    }
}

