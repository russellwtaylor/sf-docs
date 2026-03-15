use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::aura_prompt::{build_aura_prompt, AURA_SYSTEM_PROMPT};
use crate::flexipage_prompt::{build_flexipage_prompt, FLEXIPAGE_SYSTEM_PROMPT};
use crate::flow_prompt::{build_flow_prompt, FLOW_SYSTEM_PROMPT};
use crate::lwc_prompt::{build_lwc_prompt, LWC_SYSTEM_PROMPT};
use crate::object_prompt::{build_object_prompt, OBJECT_SYSTEM_PROMPT};
use crate::prompt::{build_prompt, SYSTEM_PROMPT};
use crate::retry::{self, MAX_RETRIES};
use crate::trigger_prompt::{build_trigger_prompt, TRIGGER_SYSTEM_PROMPT};
use crate::types::{
    AuraDocumentation, AuraMetadata, ClassDocumentation, ClassMetadata, FlexiPageDocumentation,
    FlexiPageMetadata, FlowDocumentation, FlowMetadata, LwcDocumentation, LwcMetadata,
    ObjectDocumentation, ObjectMetadata, SourceFile, TriggerDocumentation, TriggerMetadata,
    ValidationRuleDocumentation, ValidationRuleMetadata,
};
use crate::validation_rule_prompt::{build_validation_rule_prompt, VALIDATION_RULE_SYSTEM_PROMPT};

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
}

impl GeminiClient {
    pub fn new(api_key: String, model: &str, concurrency: usize) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            client,
            api_key,
            model: model.to_string(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        })
    }

    /// Send a single (system, user) prompt to Gemini with retry logic.
    /// Returns the raw JSON string from the first candidate's text part.
    async fn send_with_retry(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
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
                    retry::sleep_for_retry("", attempt, "Gemini API").await;
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
                retry::sleep_for_retry(&body, attempt, "Gemini API").await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("Gemini API error {status}: {body}");
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
                "Failed to parse Gemini JSON for class '{}':\n{raw}",
                metadata.class_name
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
                "Failed to parse Gemini JSON for trigger '{}':\n{raw}",
                metadata.trigger_name
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
                "Failed to parse Gemini JSON for flow '{}':\n{raw}",
                metadata.api_name
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
                "Failed to parse Gemini JSON for validation rule '{}':\n{raw}",
                metadata.rule_name
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
                "Failed to parse Gemini JSON for object '{}':\n{raw}",
                metadata.object_name
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
                "Failed to parse Gemini JSON for LWC component '{}':\n{raw}",
                metadata.component_name
            )
        })
    }

    pub async fn document_flexipage(
        &self,
        file: &SourceFile,
        metadata: &FlexiPageMetadata,
    ) -> Result<FlexiPageDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(
                FLEXIPAGE_SYSTEM_PROMPT,
                &build_flexipage_prompt(file, metadata),
            )
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse Gemini JSON for FlexiPage '{}':\n{raw}",
                metadata.api_name
            )
        })
    }

    pub async fn document_aura(
        &self,
        file: &SourceFile,
        metadata: &AuraMetadata,
    ) -> Result<AuraDocumentation> {
        let _permit = self.semaphore.acquire().await?;
        let raw = self
            .send_with_retry(AURA_SYSTEM_PROMPT, &build_aura_prompt(file, metadata))
            .await?;
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse Gemini JSON for Aura component '{}':\n{raw}",
                metadata.component_name
            )
        })
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
    use super::*;
    use crate::types::{MethodMetadata, ParamMetadata};
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
