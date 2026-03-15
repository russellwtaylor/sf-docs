use crate::types::{ApexFile, FlowMetadata};

pub const FLOW_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer.
Your task is to generate rich, accurate documentation for Salesforce Flows.

Always respond with a single valid JSON object matching this schema exactly:
{
  "api_name": "string — the flow API name",
  "label": "string — the flow label",
  "summary": "string — one sentence describing what this flow does",
  "description": "string — detailed description (2-5 sentences)",
  "business_process": "string — plain-English explanation of the business process this flow implements",
  "entry_criteria": "string — when or how this flow is triggered (e.g. record-triggered on Account insert, manually launched, scheduled)",
  "key_decisions": ["string — a key decision point or branching condition in the flow"],
  "admin_notes": ["string — operational note for admins (e.g. active status considerations, required permissions, dependencies)"],
  "relationships": ["string — name of a related Apex class, Flow, or object and the nature of the relationship"]
}

Rules:
- base your response only on the structural summary provided; do not invent elements not present.
- business_process should be understandable to a non-technical Salesforce admin.
- entry_criteria should describe how/when the flow fires based on processType and any trigger info.
- key_decisions should only list decisions that appear in the flow element summary.
- Keep descriptions concise and accurate.
"#;

pub fn build_flow_prompt(file: &ApexFile, metadata: &FlowMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("# Salesforce Flow: {}\n\n", metadata.api_name));
    prompt.push_str(&format!("**Label:** {}\n", metadata.label));
    prompt.push_str(&format!("**Process Type:** {}\n", metadata.process_type));

    if !metadata.description.is_empty() {
        prompt.push_str(&format!("**Description:** {}\n", metadata.description));
    }
    prompt.push('\n');

    // Variables
    let input_vars: Vec<&crate::types::FlowVariable> =
        metadata.variables.iter().filter(|v| v.is_input).collect();
    let output_vars: Vec<&crate::types::FlowVariable> =
        metadata.variables.iter().filter(|v| v.is_output).collect();

    if !input_vars.is_empty() {
        prompt.push_str("## Input Variables\n\n");
        for v in &input_vars {
            prompt.push_str(&format!("- `{}` ({})\n", v.name, v.data_type));
        }
        prompt.push('\n');
    }

    if !output_vars.is_empty() {
        prompt.push_str("## Output Variables\n\n");
        for v in &output_vars {
            prompt.push_str(&format!("- `{}` ({})\n", v.name, v.data_type));
        }
        prompt.push('\n');
    }

    // Element counts
    prompt.push_str("## Flow Elements\n\n");
    prompt.push_str(&format!("- Decisions: {}\n", metadata.decisions));
    prompt.push_str(&format!("- Loops: {}\n", metadata.loops));
    prompt.push_str(&format!("- Screens: {}\n", metadata.screens));
    prompt.push('\n');

    // Record operations
    if !metadata.record_operations.is_empty() {
        prompt.push_str("## Record Operations\n\n");
        for op in &metadata.record_operations {
            prompt.push_str(&format!("- {} on `{}`\n", op.operation, op.object));
        }
        prompt.push('\n');
    }

    // Action calls
    if !metadata.action_calls.is_empty() {
        prompt.push_str("## Action Calls\n\n");
        for action in &metadata.action_calls {
            prompt.push_str(&format!(
                "- `{}` (type: {})\n",
                action.name, action.action_type
            ));
        }
        prompt.push('\n');
    }

    // Raw XML for AI context (truncated to keep prompt manageable)
    let xml_preview: String = file.raw_source.chars().take(3000).collect();
    prompt.push_str("## XML Preview\n\n```xml\n");
    prompt.push_str(&xml_preview);
    if file.raw_source.len() > 3000 {
        prompt.push_str("\n... (truncated)");
    }
    prompt.push_str("\n```\n\n");

    prompt.push_str("Generate documentation JSON for this flow.");

    prompt
}
