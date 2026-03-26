use crate::types::*;

// ---------------------------------------------------------------------------
// Apex Classes
// ---------------------------------------------------------------------------

pub const CLASS_SYSTEM_PROMPT: &str = r#"
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

pub fn build_class_prompt(file: &SourceFile, metadata: &ClassMetadata) -> String {
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

// ---------------------------------------------------------------------------
// Apex Triggers
// ---------------------------------------------------------------------------

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

pub fn build_trigger_prompt(file: &SourceFile, metadata: &TriggerMetadata) -> String {
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

// ---------------------------------------------------------------------------
// Flows
// ---------------------------------------------------------------------------

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

pub fn build_flow_prompt(file: &SourceFile, metadata: &FlowMetadata) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!("# Salesforce Flow: {}\n\n", metadata.api_name));
    prompt.push_str(&format!("**Label:** {}\n", metadata.label));
    prompt.push_str(&format!("**Process Type:** {}\n", metadata.process_type));

    if !metadata.description.is_empty() {
        prompt.push_str(&format!("**Description:** {}\n", metadata.description));
    }
    prompt.push('\n');

    // Variables
    let input_vars: Vec<&FlowVariable> =
        metadata.variables.iter().filter(|v| v.is_input).collect();
    let output_vars: Vec<&FlowVariable> =
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
    if file.raw_source.chars().count() > 3000 {
        prompt.push_str("\n... (truncated)");
    }
    prompt.push_str("\n```\n\n");

    prompt.push_str("Generate documentation JSON for this flow.");

    prompt
}

// ---------------------------------------------------------------------------
// Validation Rules
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Custom Objects
// ---------------------------------------------------------------------------

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

pub fn build_object_prompt(file: &SourceFile, metadata: &ObjectMetadata) -> String {
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
                "\u{2014}".to_string()
            } else {
                field.reference_to.clone()
            };
            let help = if field.help_text.is_empty() {
                "\u{2014}".to_string()
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

// ---------------------------------------------------------------------------
// Lightning Web Components (LWC)
// ---------------------------------------------------------------------------

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
        if file.raw_source.chars().count() > MAX_JS_CHARS {
            let truncated: String = file.raw_source.chars().take(MAX_JS_CHARS).collect();
            prompt.push_str(&truncated);
            prompt.push_str("\n// ... (truncated)\n");
        } else {
            prompt.push_str(&file.raw_source);
        }
        prompt.push_str("\n```\n\n");
    }

    prompt.push_str("Generate documentation JSON for this Lightning Web Component.");
    prompt
}

// ---------------------------------------------------------------------------
// FlexiPages (Lightning Pages)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Aura Components
// ---------------------------------------------------------------------------

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
                    "\u{2014}".to_string()
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
        if file.raw_source.chars().count() > MAX_SOURCE_CHARS {
            let truncated: String = file.raw_source.chars().take(MAX_SOURCE_CHARS).collect();
            prompt.push_str(&truncated);
            prompt.push_str("\n// ... (truncated)\n");
        } else {
            prompt.push_str(&file.raw_source);
        }
        prompt.push_str("\n```\n\n");
    }

    prompt.push_str("Generate documentation JSON for this Aura component.");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_aura_prompt_does_not_panic_on_multibyte_utf8() {
        // \u{00e9} is 2 bytes in UTF-8; 5000 ASCII + 2500 multi-byte = 10000 bytes > 6000
        let source = "a".repeat(5000) + &"\u{00e9}".repeat(2500);
        let file = SourceFile {
            path: PathBuf::from("test.cmp"),
            filename: "test.cmp".to_string(),
            raw_source: source,
        };
        let meta = AuraMetadata {
            component_name: "test".to_string(),
            ..Default::default()
        };
        // Should not panic
        let result = build_aura_prompt(&file, &meta);
        assert!(result.contains("(truncated)"));
    }
}
