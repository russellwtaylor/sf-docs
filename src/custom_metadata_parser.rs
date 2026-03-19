use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

use crate::types::CustomMetadataRecord;

/// Parse a custom metadata record from a `.md-meta.xml` file.
///
/// Filename format: `{TypeName}__mdt.{RecordName}.md-meta.xml`
pub fn parse_custom_metadata_record(path: &Path, source: &str) -> Result<CustomMetadataRecord> {
    // Derive type_name and record_name from the filename.
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .trim_end_matches(".md-meta.xml");

    let (type_name, record_name) = if let Some(dot_pos) = filename.find('.') {
        (
            filename[..dot_pos].to_string(),
            filename[dot_pos + 1..].to_string(),
        )
    } else {
        (filename.to_string(), String::new())
    };

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut label = String::new();
    let mut values: Vec<(String, String)> = Vec::new();

    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    // Per-value state
    let mut current_field: Option<String> = None;
    let mut current_value: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                if name == "values" {
                    current_field = None;
                    current_value = None;
                }
                stack.push(name);
            }
            Event::End(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                if name == "values" {
                    if let (Some(f), Some(v)) = (current_field.take(), current_value.take()) {
                        if !f.is_empty() {
                            values.push((f, v));
                        }
                    }
                    current_field = None;
                    current_value = None;
                }
                stack.pop();
            }
            Event::Text(e) => {
                let text = match e.unescape() {
                    Ok(t) => t.trim().to_string(),
                    Err(_) => continue,
                };
                if text.is_empty() {
                    continue;
                }

                let parent = stack.last().map(|s| s.as_str()).unwrap_or("");
                let grandparent = stack
                    .len()
                    .checked_sub(2)
                    .and_then(|i| stack.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                match (parent, grandparent) {
                    ("label", _) if label.is_empty() => {
                        label = text;
                    }
                    ("field", "values") => {
                        current_field = Some(text);
                    }
                    ("value", "values") => {
                        current_value = Some(text);
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(CustomMetadataRecord {
        type_name,
        record_name,
        label,
        values,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <label>Integration Settings Default</label>
    <protected>false</protected>
    <values>
        <field>Endpoint__c</field>
        <value xsi:type="xsd:string">https://api.example.com</value>
    </values>
    <values>
        <field>Timeout__c</field>
        <value xsi:type="xsd:double">30</value>
    </values>
</CustomMetadata>"#;

    fn sample_path() -> PathBuf {
        PathBuf::from("Integration_Settings__mdt.Default.md-meta.xml")
    }

    #[test]
    fn parses_type_name() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        assert_eq!(rec.type_name, "Integration_Settings__mdt");
    }

    #[test]
    fn parses_record_name() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        assert_eq!(rec.record_name, "Default");
    }

    #[test]
    fn parses_label() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        assert_eq!(rec.label, "Integration Settings Default");
    }

    #[test]
    fn parses_values() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        assert_eq!(rec.values.len(), 2);
        assert!(
            rec.values.iter().any(|(f, _)| f == "Endpoint__c"),
            "expected Endpoint__c field"
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn filename_without_dot_gives_empty_record_name() {
        let path = PathBuf::from("SinglePart.md-meta.xml");
        let src = r#"<?xml version="1.0"?><CustomMetadata><label>Test</label></CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.type_name, "SinglePart");
        assert!(rec.record_name.is_empty());
    }

    #[test]
    fn empty_xml_returns_defaults() {
        let path = PathBuf::from("Type__mdt.Record.md-meta.xml");
        let src = r#"<?xml version="1.0"?><CustomMetadata></CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.type_name, "Type__mdt");
        assert_eq!(rec.record_name, "Record");
        assert!(rec.label.is_empty());
        assert!(rec.values.is_empty());
    }

    #[test]
    fn value_with_empty_field_skipped() {
        let path = PathBuf::from("T__mdt.R.md-meta.xml");
        let src = r#"<?xml version="1.0"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Test</label>
    <values>
        <field></field>
        <value>should be skipped</value>
    </values>
    <values>
        <field>Valid__c</field>
        <value>kept</value>
    </values>
</CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.values.len(), 1);
        assert_eq!(rec.values[0].0, "Valid__c");
    }

    #[test]
    fn multiple_values_preserved_in_order() {
        let path = PathBuf::from("Config__mdt.Main.md-meta.xml");
        let src = r#"<?xml version="1.0"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Main Config</label>
    <values><field>First__c</field><value>1</value></values>
    <values><field>Second__c</field><value>2</value></values>
    <values><field>Third__c</field><value>3</value></values>
</CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.values.len(), 3);
        assert_eq!(rec.values[0].0, "First__c");
        assert_eq!(rec.values[1].0, "Second__c");
        assert_eq!(rec.values[2].0, "Third__c");
    }

    #[test]
    fn protected_field_does_not_affect_parsing() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        assert!(!rec.values.iter().any(|(f, _)| f == "protected"));
    }

    #[test]
    fn value_with_xsi_type_attribute_still_reads_text() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        let timeout = rec.values.iter().find(|(f, _)| f == "Timeout__c").unwrap();
        assert_eq!(timeout.1, "30");
    }
}
