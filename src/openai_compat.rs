use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::flow_prompt::{build_flow_prompt, FLOW_SYSTEM_PROMPT};
use crate::lwc_prompt::{build_lwc_prompt, LWC_SYSTEM_PROMPT};
use crate::object_prompt::{build_object_prompt, OBJECT_SYSTEM_PROMPT};
use crate::prompt::{build_prompt, SYSTEM_PROMPT};
use crate::retry::{self, MAX_RETRIES};
use crate::trigger_prompt::{build_trigger_prompt, TRIGGER_SYSTEM_PROMPT};
use crate::types::{
    ClassDocumentation, ClassMetadata, FlowDocumentation, FlowMetadata, LwcDocumentation,
    LwcMetadata, ObjectDocumentation, ObjectMetadata, SourceFile, TriggerDocumentation,
    TriggerMetadata, ValidationRuleDocumentation, ValidationRuleMetadata,
};
use crate::validation_rule_prompt::{build_validation_rule_prompt, VALIDATION_RULE_SYSTEM_PROMPT};

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
    ) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            client,
            api_key,
            model: model.to_string(),
            base_url: base_url.to_string(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            provider_name: provider_name.to_string(),
        })
    }

    /// Send a single (system, user) prompt to the OpenAI-compatible endpoint with retry logic.
    /// Returns the raw JSON string from the first choice's message content.
    async fn send_with_retry(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
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
                    retry::sleep_for_retry("", attempt, &self.provider_name).await;
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
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

            if retry::should_retry(status.as_u16()) && attempt < MAX_RETRIES {
                retry::sleep_for_retry(&body, attempt, &self.provider_name).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("{} API error {status}: {body}", self.provider_name);
        }
    }

    pub async fn document_class(
        &self,
        file: &SourceFile,
        metadata: &ClassMetadata,
    ) -> Result<ClassDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(SYSTEM_PROMPT, &build_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for class '{}':\n{raw}",
                self.provider_name, metadata.class_name
            )
        })
    }

    pub async fn document_trigger(
        &self,
        file: &SourceFile,
        metadata: &TriggerMetadata,
    ) -> Result<TriggerDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(TRIGGER_SYSTEM_PROMPT, &build_trigger_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for trigger '{}':\n{raw}",
                self.provider_name, metadata.trigger_name
            )
        })
    }

    pub async fn document_flow(
        &self,
        file: &SourceFile,
        metadata: &FlowMetadata,
    ) -> Result<FlowDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(FLOW_SYSTEM_PROMPT, &build_flow_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for flow '{}':\n{raw}",
                self.provider_name, metadata.api_name
            )
        })
    }

    pub async fn document_validation_rule(
        &self,
        file: &SourceFile,
        metadata: &ValidationRuleMetadata,
    ) -> Result<ValidationRuleDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(
                VALIDATION_RULE_SYSTEM_PROMPT,
                &build_validation_rule_prompt(file, metadata),
            )
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for validation rule '{}':\n{raw}",
                self.provider_name, metadata.rule_name
            )
        })
    }

    pub async fn document_object(
        &self,
        file: &SourceFile,
        metadata: &ObjectMetadata,
    ) -> Result<ObjectDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(OBJECT_SYSTEM_PROMPT, &build_object_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for object '{}':\n{raw}",
                self.provider_name, metadata.object_name
            )
        })
    }

    pub async fn document_lwc(
        &self,
        file: &SourceFile,
        metadata: &LwcMetadata,
    ) -> Result<LwcDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(LWC_SYSTEM_PROMPT, &build_lwc_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse {} JSON for LWC component '{}':\n{raw}",
                self.provider_name, metadata.component_name
            )
        })
    }
}
