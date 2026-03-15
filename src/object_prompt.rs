use crate::types::{ApexFile, ObjectMetadata};

pub const OBJECT_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer.
Your task is to generate rich, accurate documentation for Salesforce Custom Objects.

Always respond with a single valid JSON object matching this schema exactly:
{
  "object_name": "string — the object API name",
  "label": "string — the object label",
  "summary": "string — one sentence describing what this object stores or represents",
  "description": "string — 2-4 sentences about the object's purpose and role in the org",
  "purpose": "string — plain-English explanation of the business use case this object supports",
  "key_fields": ["string — a notable field and why it matters, e.g. 'Status__c (Picklist): tracks the current lifecycle stage'"],
  "relationships": ["string — a related object, Apex class, or flow that works with this object"],
  "admin_notes": ["string — an admin tip, gotcha, or configuration note"]
}

Rules:
- Base your response only on the provided object name, label, description, and field list.
- summary should be understandable to a non-technical Salesforce admin.
- key_fields should highlight the most important fields, not list every field.
- Do not invent fields or relationships not present in the metadata.
- Keep descriptions concise and accurate.
"#;

pub fn build_object_prompt(file: &ApexFile, metadata: &ObjectMetadata) -> String {
    let _ = file; // XML not sent raw; structured summary is sufficient

    let mut prompt = String::new();

    prompt.push_str(&format!("# Object: {}\n\n", metadata.object_name));

    if !metadata.label.is_empty() {
        prompt.push_str(&format!("**Label:** {}\n", metadata.label));
    }

    if !metadata.description.is_empty() {
        prompt.push_str(&format!("**Description:** {}\n", metadata.description));
    }

    prompt.push('\n');

    if !metadata.fields.is_empty() {
        prompt.push_str("## Fields\n\n");
        prompt.push_str("| API Name | Type | Label | Required | Reference To | Help Text |\n");
        prompt.push_str("|----------|------|-------|----------|--------------|-----------|\n");
        for field in &metadata.fields {
            let ref_to = if field.reference_to.is_empty() {
                "—".to_string()
            } else {
                field.reference_to.clone()
            };
            let help = if field.help_text.is_empty() {
                "—".to_string()
            } else {
                field.help_text.clone()
            };
            prompt.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} |\n",
                field.api_name,
                field.field_type,
                field.label,
                if field.required { "Yes" } else { "No" },
                ref_to,
                help,
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("Generate documentation JSON for this Salesforce object.");
    prompt
}
