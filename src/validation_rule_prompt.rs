use crate::types::{SourceFile, ValidationRuleMetadata};

pub const VALIDATION_RULE_SYSTEM_PROMPT: &str = r#"
You are an expert Salesforce developer and technical writer.
Your task is to generate rich, accurate documentation for Salesforce Validation Rules.

Always respond with a single valid JSON object matching this schema exactly:
{
  "rule_name": "string — the validation rule API name",
  "object_name": "string — the SObject this rule applies to",
  "summary": "string — one sentence describing what this rule does",
  "when_fires": "string — plain-English description of when this rule triggers (when the formula evaluates to true)",
  "what_protects": "string — what data quality issue or business rule this validation enforces",
  "formula_explanation": "string — step-by-step plain-English walkthrough of the formula logic",
  "edge_cases": ["string — a noteworthy edge case, exception, or gotcha in this formula"],
  "relationships": ["string — a related field, object, or Apex class referenced in this formula"]
}

Rules:
- Base your response only on the provided rule name, formula, error message, and object.
- when_fires should be understandable to a non-technical Salesforce admin.
- formula_explanation should break down each function/condition in the formula.
- Do not invent functionality not present in the formula.
- Keep descriptions concise and accurate.
"#;

pub fn build_validation_rule_prompt(
    file: &SourceFile,
    metadata: &ValidationRuleMetadata,
) -> String {
    let _ = file; // XML not sent raw; structured summary is sufficient

    let mut prompt = String::new();

    prompt.push_str(&format!(
        "# Validation Rule: {} on {}\n\n",
        metadata.rule_name, metadata.object_name
    ));
    prompt.push_str(&format!("**Object:** {}\n", metadata.object_name));
    prompt.push_str(&format!(
        "**Active:** {}\n",
        if metadata.active { "Yes" } else { "No" }
    ));

    if !metadata.description.is_empty() {
        prompt.push_str(&format!("**Description:** {}\n", metadata.description));
    }

    prompt.push_str(&format!("**Error Message:** {}\n", metadata.error_message));

    if !metadata.error_display_field.is_empty() {
        prompt.push_str(&format!(
            "**Error Display Field:** {}\n",
            metadata.error_display_field
        ));
    }

    prompt.push_str("\n## Error Condition Formula\n\n```\n");
    prompt.push_str(&metadata.error_condition_formula);
    prompt.push_str("\n```\n\n");

    prompt.push_str("Generate documentation JSON for this validation rule.");
    prompt
}
