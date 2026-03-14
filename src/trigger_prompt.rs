use crate::types::{ApexFile, TriggerMetadata};

pub const TRIGGER_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer.
Your task is to generate rich, accurate documentation for Apex triggers.

Always respond with a single valid JSON object matching this schema exactly:
{
  "trigger_name": "string",
  "sobject": "string — the SObject this trigger fires on (e.g. Account)",
  "summary": "string — one sentence describing what this trigger does",
  "description": "string — detailed description (2-5 sentences)",
  "events": [
    {
      "event": "string — one of: before insert, before update, before delete, after insert, after update, after delete, after undelete",
      "description": "string — what logic runs in this event context"
    }
  ],
  "handler_classes": ["string — name of a handler or service class invoked by this trigger"],
  "usage_notes": ["string — operational note about this trigger (e.g. recursion guard, bypass logic, order dependency)"],
  "relationships": ["string — name of a related class or trigger and the nature of the relationship"]
}

Rules:
- Only include events that appear in the trigger declaration.
- handler_classes should only include class names that actually appear in the source.
- usage_notes should capture non-obvious operational concerns visible in the code.
- Do not invent functionality that is not in the source.
- Keep descriptions concise and technical.
"#;

pub fn build_trigger_prompt(file: &ApexFile, metadata: &TriggerMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("# Apex Trigger: {}\n\n", metadata.trigger_name));
    prompt.push_str(&format!("**SObject:** {}\n", metadata.sobject));
    prompt.push_str(&format!(
        "**Events:** {}\n\n",
        metadata
            .events
            .iter()
            .map(|e| e.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    if !metadata.existing_comments.is_empty() {
        prompt.push_str("## Existing ApexDoc Comments\n\n");
        for comment in &metadata.existing_comments {
            prompt.push_str(comment);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str("## Full Source\n\n```apex\n");
    prompt.push_str(&file.raw_source);
    prompt.push_str("\n```\n\n");
    prompt.push_str("Generate documentation JSON for this trigger.");

    prompt
}
