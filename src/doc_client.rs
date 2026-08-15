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
    let json = extract_json_payload(&raw);
    serde_json::from_str(json).with_context(|| {
        format!(
            "Failed to parse {} JSON for {}:\n{raw}",
            client.provider_name(),
            entity_label
        )
    })
}

/// Strip optional markdown fences around a JSON object so providers that
/// ignore `response_format` still deserialize.
pub fn extract_json_payload(raw: &str) -> &str {
    let trimmed = raw.trim();
    let body = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("```JSON") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest
    } else {
        return trimmed;
    };
    body.trim()
        .strip_suffix("```")
        .map(str::trim)
        .unwrap_or_else(|| body.trim())
}

#[cfg(test)]
mod tests {
    use super::extract_json_payload;

    #[test]
    fn extract_json_payload_passthrough() {
        assert_eq!(extract_json_payload(" {\"a\":1} "), "{\"a\":1}");
    }

    #[test]
    fn extract_json_payload_strips_json_fence() {
        let raw = "```json\n{\"class_name\":\"Foo\"}\n```";
        assert_eq!(extract_json_payload(raw), "{\"class_name\":\"Foo\"}");
    }

    #[test]
    fn extract_json_payload_strips_plain_fence() {
        let raw = "```\n{\"x\":1}\n```";
        assert_eq!(extract_json_payload(raw), "{\"x\":1}");
    }

    #[test]
    fn class_documentation_allows_missing_optional_arrays() {
        let json = r#"{"class_name":"Foo","summary":"s","description":"d"}"#;
        let doc: crate::types::ClassDocumentation = serde_json::from_str(json).unwrap();
        assert_eq!(doc.class_name, "Foo");
        assert!(doc.methods.is_empty());
        assert!(doc.relationships.is_empty());
    }

    #[test]
    fn documentation_types_default_missing_vec_fields() {
        let trigger: crate::types::TriggerDocumentation = serde_json::from_str(
            r#"{"trigger_name":"T","sobject":"Account","summary":"s","description":"d"}"#,
        )
        .unwrap();
        assert!(trigger.events.is_empty());
        assert!(trigger.relationships.is_empty());

        let flow: crate::types::FlowDocumentation =
            serde_json::from_str(r#"{"api_name":"F","label":"L","summary":"s","description":"d"}"#)
                .unwrap();
        assert!(flow.key_decisions.is_empty());
        assert!(flow.relationships.is_empty());

        let object: crate::types::ObjectDocumentation = serde_json::from_str(
            r#"{"object_name":"O","label":"L","summary":"s","description":"d"}"#,
        )
        .unwrap();
        assert!(object.key_fields.is_empty());
        assert!(object.relationships.is_empty());

        let vr: crate::types::ValidationRuleDocumentation =
            serde_json::from_str(r#"{"rule_name":"R","object_name":"Account","summary":"s"}"#)
                .unwrap();
        assert!(vr.edge_cases.is_empty());
        assert!(vr.relationships.is_empty());

        let lwc: crate::types::LwcDocumentation =
            serde_json::from_str(r#"{"component_name":"c","summary":"s","description":"d"}"#)
                .unwrap();
        assert!(lwc.api_props.is_empty());
        assert!(lwc.relationships.is_empty());

        let page: crate::types::FlexiPageDocumentation =
            serde_json::from_str(r#"{"api_name":"P","label":"L","summary":"s","description":"d"}"#)
                .unwrap();
        assert!(page.key_components.is_empty());
        assert!(page.relationships.is_empty());

        let aura: crate::types::AuraDocumentation =
            serde_json::from_str(r#"{"component_name":"a","summary":"s","description":"d"}"#)
                .unwrap();
        assert!(aura.attributes.is_empty());
        assert!(aura.relationships.is_empty());
    }
}
