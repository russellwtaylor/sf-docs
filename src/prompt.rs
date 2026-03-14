use crate::types::{ApexFile, ClassMetadata};

pub const SYSTEM_PROMPT: &str = r#"
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

pub fn build_prompt(file: &ApexFile, metadata: &ClassMetadata) -> String {
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
            let static_kw = if method.is_static { "static " } else { "" };
            prompt.push_str(&format!(
                "- `{} {}{}({}): {}`\n",
                method.access_modifier,
                static_kw,
                method.name,
                params.join(", "),
                method.return_type,
            ));
        }
        prompt.push('\n');
    }

    if !metadata.properties.is_empty() {
        prompt.push_str("## Extracted Properties\n\n");
        for prop in &metadata.properties {
            let static_kw = if prop.is_static { "static " } else { "" };
            prompt.push_str(&format!(
                "- `{} {}{} {}`\n",
                prop.access_modifier, static_kw, prop.property_type, prop.name
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
