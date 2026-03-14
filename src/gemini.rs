use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::types::{ApexFile, ClassDocumentation, ClassMetadata};

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
        let model_id = match model {
            "pro" => "gemini-2.0-pro-exp",
            _ => "gemini-2.0-flash",
        };
        Self {
            client: Client::new(),
            api_key,
            model: model_id.to_string(),
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

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error {status}: {body}");
        }

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

        Ok(doc)
    }
}

// ---------------------------------------------------------------------------
// Prompt engineering
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer.
Your task is to generate rich, accurate Markdown documentation for Apex classes.

Always respond with a single valid JSON object matching this schema exactly:
{
  "class_name": "string",
  "summary": "string — one sentence describing the class purpose",
  "description": "string — detailed description (2-5 sentences)",
  "methods": [
    {
      "name": "string",
      "description": "string",
      "params": [{"name": "string", "description": "string"}],
      "returns": "string — describe what is returned, or 'void'",
      "throws": ["string — exception type and condition"]
    }
  ],
  "properties": [
    {"name": "string", "description": "string"}
  ],
  "usage_examples": ["string — a short Apex code snippet showing how to use this class"],
  "relationships": ["string — name of related class and nature of relationship"]
}

Rules:
- Include only methods and properties that appear in the source.
- If a method throws no exceptions, use an empty array for "throws".
- Keep descriptions concise and technical.
- usage_examples should be valid Apex code snippets wrapped in a code fence (``` apex ... ```).
- Do not invent functionality that is not in the source.
"#;

fn build_prompt(file: &ApexFile, metadata: &ClassMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("# Apex Class: {}\n\n", metadata.class_name));

    if !metadata.existing_comments.is_empty() {
        prompt.push_str("## Existing ApexDoc Comments\n\n");
        for comment in &metadata.existing_comments {
            prompt.push_str(comment);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    if !metadata.methods.is_empty() {
        prompt.push_str("## Extracted Methods\n\n");
        for method in &metadata.methods {
            let params: Vec<String> = method
                .params
                .iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect();
            prompt.push_str(&format!(
                "- `{} {} {}({})`\n",
                method.access_modifier,
                method.return_type,
                method.name,
                params.join(", ")
            ));
        }
        prompt.push('\n');
    }

    if !metadata.properties.is_empty() {
        prompt.push_str("## Extracted Properties\n\n");
        for prop in &metadata.properties {
            prompt.push_str(&format!(
                "- `{} {} {}`\n",
                prop.access_modifier, prop.property_type, prop.name
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Full Source\n\n```apex\n");
    prompt.push_str(&file.raw_source);
    prompt.push_str("\n```\n\n");
    prompt.push_str("Generate documentation JSON for this class.");

    prompt
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
                existing_comment: None,
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
