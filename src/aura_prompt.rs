use crate::types::{AuraMetadata, SourceFile};

pub const AURA_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer specializing in Aura Components (Lightning Components).
Your task is to generate rich, accurate documentation for a Salesforce Aura component.

Always respond with a single valid JSON object matching this schema exactly:
{
  "component_name": "string — the component name",
  "summary": "string — one sentence describing what this component does",
  "description": "string — 2-4 sentences about the component's purpose and UI role",
  "attributes": [
    {
      "name": "string — the attribute name",
      "description": "string — what this attribute is for and how to use it"
    }
  ],
  "usage_notes": ["string — a usage tip, required parent component, or configuration note"],
  "relationships": ["string — a related Apex class, component, object, or flow this component interacts with"]
}

Rules:
- Base your response only on the provided component name, attributes, events, and source code.
- summary should be understandable to a developer who hasn't used this component before.
- attributes should document every attribute listed in the metadata.
- usage_notes should highlight important constraints or integration requirements.
- Do not invent attributes or behaviour not present in the source.
- Keep descriptions concise and accurate.
"#;

pub fn build_aura_prompt(file: &SourceFile, metadata: &AuraMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "# Aura Component: {}\n\n",
        metadata.component_name
    ));

    if let Some(ref ext) = metadata.extends {
        prompt.push_str(&format!("**Extends:** `{}`\n\n", ext));
    }

    if !metadata.attributes.is_empty() {
        prompt.push_str("## Attributes\n\n");
        prompt.push_str("| Name | Type | Default | Description |\n");
        prompt.push_str("|------|------|---------|-------------|\n");
        for attr in &metadata.attributes {
            prompt.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                attr.name,
                attr.attr_type,
                if attr.default.is_empty() {
                    "—".to_string()
                } else {
                    format!("`{}`", attr.default)
                },
                attr.description
            ));
        }
        prompt.push('\n');
    }

    if !metadata.events_handled.is_empty() {
        prompt.push_str("## Events\n\n");
        for event in &metadata.events_handled {
            prompt.push_str(&format!("- `{}`\n", event));
        }
        prompt.push('\n');
    }

    // Include JS/CMP source if available
    if !file.raw_source.is_empty() {
        prompt.push_str("## Source\n\n");
        prompt.push_str("```javascript\n");
        const MAX_SOURCE_CHARS: usize = 6_000;
        if file.raw_source.len() > MAX_SOURCE_CHARS {
            prompt.push_str(&file.raw_source[..MAX_SOURCE_CHARS]);
            prompt.push_str("\n// ... (truncated)\n");
        } else {
            prompt.push_str(&file.raw_source);
        }
        prompt.push_str("\n```\n\n");
    }

    prompt.push_str("Generate documentation JSON for this Aura component.");
    prompt
}
