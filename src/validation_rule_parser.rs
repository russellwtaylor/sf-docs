use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

use crate::types::ValidationRuleMetadata;

pub fn parse_validation_rule(path: &Path, source: &str) -> Result<ValidationRuleMetadata> {
    // Derive rule_name from filename
    let rule_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".validationRule-meta.xml"))
        .unwrap_or("")
        .to_string();

    // Derive object_name: path structure is .../objects/{ObjectName}/validationRules/{file}
    // so parent = validationRules dir, parent.parent = ObjectName dir
    let object_name = path
        .parent() // validationRules/
        .and_then(|p| p.parent()) // {ObjectName}/
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let mut meta = ValidationRuleMetadata {
        rule_name,
        object_name,
        active: true, // default to true if not specified
        ..Default::default()
    };

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    // For multi-line formula accumulation:
    let mut formula_buf = String::new();
    let mut in_formula = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("validation rule XML contains invalid UTF-8 in tag name")?
                    .to_string();
                if name == "errorConditionFormula" {
                    in_formula = true;
                    formula_buf.clear();
                }
                stack.push(name);
            }
            Event::End(e) => {
                let name_bytes = e.name();
                let name = std::str::from_utf8(name_bytes.as_ref())
                    .context("validation rule XML contains invalid UTF-8 in tag name")?;
                if name == "errorConditionFormula" {
                    meta.error_condition_formula = formula_buf.trim().to_string();
                    in_formula = false;
                }
                stack.pop();
            }
            Event::Text(e) => {
                let text = match e.unescape() {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };
                if in_formula {
                    formula_buf.push_str(&text);
                    continue;
                }
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parent = stack.last().map(|s| s.as_str()).unwrap_or("");
                match parent {
                    "active" => meta.active = trimmed == "true",
                    "description" => meta.description = trimmed.to_string(),
                    "errorDisplayField" => meta.error_display_field = trimmed.to_string(),
                    "errorMessage" => meta.error_message = trimmed.to_string(),
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(meta)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SAMPLE_RULE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Require_Start_Date</fullName>
    <active>true</active>
    <description>Ensures active records have a start date.</description>
    <errorConditionFormula>AND(
  ISPICKVAL(Status__c, "Active"),
  ISBLANK(StartDate__c)
)</errorConditionFormula>
    <errorDisplayField>StartDate__c</errorDisplayField>
    <errorMessage>Start Date is required for active records.</errorMessage>
</ValidationRule>"#;

    const INACTIVE_RULE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Old_Rule</fullName>
    <active>false</active>
    <errorConditionFormula>ISBLANK(Name)</errorConditionFormula>
    <errorMessage>Name is required.</errorMessage>
</ValidationRule>"#;

    fn make_path(object: &str, rule: &str) -> PathBuf {
        PathBuf::from(format!(
            "force-app/main/default/objects/{object}/validationRules/{rule}.validationRule-meta.xml"
        ))
    }

    #[test]
    fn parses_rule_name_from_filename() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert_eq!(meta.rule_name, "Require_Start_Date");
    }

    #[test]
    fn derives_object_name_from_path() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert_eq!(meta.object_name, "Account");
    }

    #[test]
    fn parses_active_true() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert!(meta.active);
    }

    #[test]
    fn parses_active_false() {
        let path = make_path("Contact", "Old_Rule");
        let meta = parse_validation_rule(&path, INACTIVE_RULE).unwrap();
        assert!(!meta.active);
    }

    #[test]
    fn parses_description() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert_eq!(meta.description, "Ensures active records have a start date.");
    }

    #[test]
    fn parses_multiline_formula() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert!(meta.error_condition_formula.contains("ISPICKVAL"));
        assert!(meta.error_condition_formula.contains("ISBLANK"));
    }

    #[test]
    fn parses_single_line_formula() {
        let path = make_path("Contact", "Old_Rule");
        let meta = parse_validation_rule(&path, INACTIVE_RULE).unwrap();
        assert_eq!(meta.error_condition_formula, "ISBLANK(Name)");
    }

    #[test]
    fn parses_error_display_field() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert_eq!(meta.error_display_field, "StartDate__c");
    }

    #[test]
    fn parses_error_message() {
        let path = make_path("Account", "Require_Start_Date");
        let meta = parse_validation_rule(&path, SAMPLE_RULE).unwrap();
        assert_eq!(
            meta.error_message,
            "Start Date is required for active records."
        );
    }

    #[test]
    fn no_description_gives_empty_string() {
        let path = make_path("Contact", "Old_Rule");
        let meta = parse_validation_rule(&path, INACTIVE_RULE).unwrap();
        assert!(meta.description.is_empty());
    }

    #[test]
    fn no_error_display_field_gives_empty_string() {
        let path = make_path("Contact", "Old_Rule");
        let meta = parse_validation_rule(&path, INACTIVE_RULE).unwrap();
        assert!(meta.error_display_field.is_empty());
    }
}
