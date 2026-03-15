use crate::types::{LwcMetadata, SourceFile};

pub const LWC_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer specializing in Lightning Web Components (LWC).
Your task is to generate rich, accurate documentation for a Salesforce LWC component.

Always respond with a single valid JSON object matching this schema exactly:
{
  "component_name": "string — the camelCase component name",
  "summary": "string — one sentence describing what this component does",
  "description": "string — 2-4 sentences about the component's purpose and UI role",
  "api_props": [
    {
      "name": "string — the @api property or method name",
      "description": "string — what this property/method is for and how to use it"
    }
  ],
  "usage_notes": ["string — a usage tip, required parent component, or configuration note"],
  "relationships": ["string — a related Apex class, component, or object this component interacts with"]
}

Rules:
- Base your response only on the provided component name, @api properties, slots, and JS/HTML source.
- summary should be understandable to a developer who hasn't used this component before.
- api_props should document every @api property and method in the metadata.
- usage_notes should highlight important constraints or integration requirements.
- Do not invent properties or behaviour not present in the source.
- Keep descriptions concise and accurate.
"#;

pub fn build_lwc_prompt(file: &SourceFile, metadata: &LwcMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("# LWC Component: {}\n\n", metadata.component_name));

    if !metadata.api_props.is_empty() {
        prompt.push_str("## Public API\n\n");
        prompt.push_str("| Name | Kind |\n");
        prompt.push_str("|------|------|\n");
        for prop in &metadata.api_props {
            let kind = if prop.is_method { "method" } else { "property" };
            prompt.push_str(&format!("| `{}` | {} |\n", prop.name, kind));
        }
        prompt.push('\n');
    }

    if !metadata.slots.is_empty() {
        prompt.push_str("## Slots\n\n");
        for slot in &metadata.slots {
            prompt.push_str(&format!("- `{}`\n", slot));
        }
        prompt.push('\n');
    }

    if !metadata.referenced_components.is_empty() {
        prompt.push_str("## Referenced Components\n\n");
        for comp in &metadata.referenced_components {
            prompt.push_str(&format!("- `{}`\n", comp));
        }
        prompt.push('\n');
    }

    // Include JS source if available
    if !file.raw_source.is_empty() {
        prompt.push_str("## JavaScript Source\n\n");
        prompt.push_str("```javascript\n");
        // Truncate very long files to avoid overly large prompts
        const MAX_JS_CHARS: usize = 6_000;
        if file.raw_source.len() > MAX_JS_CHARS {
            prompt.push_str(&file.raw_source[..MAX_JS_CHARS]);
            prompt.push_str("\n// ... (truncated)\n");
        } else {
            prompt.push_str(&file.raw_source);
        }
        prompt.push_str("\n```\n\n");
    }

    prompt.push_str("Generate documentation JSON for this Lightning Web Component.");
    prompt
}
