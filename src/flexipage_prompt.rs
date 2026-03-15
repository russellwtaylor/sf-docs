use crate::types::{FlexiPageMetadata, SourceFile};

pub const FLEXIPAGE_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer specializing in Lightning Experience page configuration.
Your task is to generate rich, accurate documentation for a Salesforce FlexiPage (Lightning Page).

Always respond with a single valid JSON object matching this schema exactly:
{
  "api_name": "string — the API name of this lightning page",
  "label": "string — the human-readable label",
  "summary": "string — one sentence describing what this page is for",
  "description": "string — 2-4 sentences about the page's purpose and context",
  "usage_context": "string — who uses this page and when it is displayed (profiles, record types, app contexts)",
  "key_components": ["string — a brief description of a key component and its role on this page"],
  "relationships": ["string — a related component, flow, object, or process this page interacts with"]
}

Rules:
- Base your response only on the provided page type, label, SObject type, component names, and flow references.
- summary should be understandable to a Salesforce admin who hasn't seen this page before.
- usage_context should explain who sees this page and under what conditions.
- key_components should describe the purpose of each listed component.
- Do not invent components or behaviour not present in the metadata.
- Keep descriptions concise and accurate.
"#;

pub fn build_flexipage_prompt(file: &SourceFile, metadata: &FlexiPageMetadata) -> String {
    let _ = file; // not used directly; metadata contains all relevant info
    let mut prompt = String::new();

    prompt.push_str(&format!("# Lightning Page: {}\n\n", metadata.label));
    prompt.push_str(&format!("**API Name:** `{}`\n\n", metadata.api_name));
    prompt.push_str(&format!("**Page Type:** `{}`\n\n", metadata.page_type));

    if !metadata.sobject.is_empty() {
        prompt.push_str(&format!("**SObject:** `{}`\n\n", metadata.sobject));
    }

    if !metadata.description.is_empty() {
        prompt.push_str(&format!("**Description:** {}\n\n", metadata.description));
    }

    if !metadata.component_names.is_empty() {
        prompt.push_str("## Components on this Page\n\n");
        for comp in &metadata.component_names {
            prompt.push_str(&format!("- `{}`\n", comp));
        }
        prompt.push('\n');
    }

    if !metadata.flow_names.is_empty() {
        prompt.push_str("## Referenced Flows\n\n");
        for flow in &metadata.flow_names {
            prompt.push_str(&format!("- `{}`\n", flow));
        }
        prompt.push('\n');
    }

    prompt.push_str("Generate documentation JSON for this Lightning Page.");
    prompt
}
