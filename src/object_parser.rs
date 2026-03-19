use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

use crate::types::{ObjectField, ObjectMetadata};

pub fn parse_object(path: &Path, source: &str) -> Result<ObjectMetadata> {
    // Derive object_name from filename: strip ".object-meta.xml"
    let object_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".object-meta.xml"))
        .unwrap_or("")
        .to_string();

    let mut label = String::new();
    let mut description = String::new();

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("object XML contains invalid UTF-8 in tag name")?
                    .to_string();
                stack.push(name);
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Text(e) => {
                let text = match e.unescape() {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parent = stack.last().map(|s| s.as_str()).unwrap_or("");
                // Only pick up label/description at the top level (direct children of CustomObject)
                // by checking stack depth to avoid matching nested elements with the same tag name.
                match parent {
                    "label" if stack.len() == 2 => label = trimmed.to_string(),
                    "description" if stack.len() == 2 => description = trimmed.to_string(),
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    // Read field files from the sibling `fields/` directory.
    let mut fields = Vec::new();
    if let Some(object_dir) = path.parent() {
        let fields_dir = object_dir.join("fields");
        if fields_dir.is_dir() {
            let mut field_paths: Vec<_> = std::fs::read_dir(&fields_dir)
                .with_context(|| format!("Failed to read fields dir: {}", fields_dir.display()))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(".field-meta.xml"))
                })
                .collect();
            field_paths.sort();
            const MAX_FIELD_FILE_SIZE: u64 = 10 * 1024 * 1024;
            for field_path in field_paths {
                if let Ok(meta) = std::fs::metadata(&field_path) {
                    if meta.len() > MAX_FIELD_FILE_SIZE {
                        eprintln!(
                            "Warning: skipping oversized field file {} ({:.1} MB)",
                            field_path.display(),
                            meta.len() as f64 / (1024.0 * 1024.0)
                        );
                        continue;
                    }
                }
                let field_source = match std::fs::read_to_string(&field_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!(
                            "Warning: could not read field file {}: {e}",
                            field_path.display()
                        );
                        continue;
                    }
                };
                match parse_field(&field_path, &field_source) {
                    Ok(field) => fields.push(field),
                    Err(e) => eprintln!(
                        "Warning: could not parse field file {}: {e}",
                        field_path.display()
                    ),
                }
            }
        }
    }

    Ok(ObjectMetadata {
        object_name,
        label,
        description,
        fields,
    })
}

fn parse_field(path: &Path, source: &str) -> Result<ObjectField> {
    // Derive api_name from filename: strip ".field-meta.xml"
    let api_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".field-meta.xml"))
        .unwrap_or("")
        .to_string();

    let mut field = ObjectField {
        api_name,
        ..Default::default()
    };

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .context("field XML contains invalid UTF-8 in tag name")?
                    .to_string();
                stack.push(name);
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Text(e) => {
                let text = match e.unescape() {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parent = stack.last().map(|s| s.as_str()).unwrap_or("");
                match parent {
                    "label" => field.label = trimmed.to_string(),
                    "description" => field.description = trimmed.to_string(),
                    "inlineHelpText" => field.help_text = trimmed.to_string(),
                    "type" => field.field_type = trimmed.to_string(),
                    "referenceTo" => field.reference_to = trimmed.to_string(),
                    "required" => field.required = trimmed == "true",
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(field)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_object_xml(label: &str, description: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <description>{description}</description>
    <label>{label}</label>
    <pluralLabel>{label}s</pluralLabel>
    <nameField>
        <label>Name</label>
        <type>Text</type>
    </nameField>
</CustomObject>"#
        )
    }

    fn make_field_xml(label: &str, field_type: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Status__c</fullName>
    <label>{label}</label>
    <type>{field_type}</type>
    <required>false</required>
</CustomField>"#
        )
    }

    fn setup_object_dir(tmp: &TempDir, object_name: &str) -> std::path::PathBuf {
        let obj_dir = tmp.path().join(object_name);
        fs::create_dir_all(obj_dir.join("fields")).unwrap();
        obj_dir
    }

    #[test]
    fn parses_object_name_from_filename() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Account");
        let xml = make_object_xml("Account", "Standard Salesforce Account object");
        let path = obj_dir.join("Account.object-meta.xml");
        fs::write(&path, &xml).unwrap();

        let meta = parse_object(&path, &xml).unwrap();
        assert_eq!(meta.object_name, "Account");
    }

    #[test]
    fn parses_label_and_description() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Account");
        let xml = make_object_xml("Account", "Standard Salesforce Account object");
        let path = obj_dir.join("Account.object-meta.xml");
        fs::write(&path, &xml).unwrap();

        let meta = parse_object(&path, &xml).unwrap();
        assert_eq!(meta.label, "Account");
        assert_eq!(meta.description, "Standard Salesforce Account object");
    }

    #[test]
    fn reads_field_files_from_fields_dir() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "My_Object__c");
        let xml = make_object_xml("My Object", "A custom object");
        let obj_path = obj_dir.join("My_Object__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        let field_xml = make_field_xml("Status", "Picklist");
        fs::write(
            obj_dir.join("fields").join("Status__c.field-meta.xml"),
            &field_xml,
        )
        .unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert_eq!(meta.fields.len(), 1);
        assert_eq!(meta.fields[0].api_name, "Status__c");
        assert_eq!(meta.fields[0].label, "Status");
        assert_eq!(meta.fields[0].field_type, "Picklist");
    }

    #[test]
    fn no_fields_dir_gives_empty_fields() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("Account.object-meta.xml");
        let xml = make_object_xml("Account", "");
        fs::write(&path, &xml).unwrap();

        let meta = parse_object(&path, &xml).unwrap();
        assert!(meta.fields.is_empty());
    }

    #[test]
    fn parses_lookup_reference_to() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "My_Object__c");
        let xml = make_object_xml("My Object", "");
        let obj_path = obj_dir.join("My_Object__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        let field_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account__c</fullName>
    <label>Account</label>
    <type>Lookup</type>
    <referenceTo>Account</referenceTo>
    <required>true</required>
</CustomField>"#;
        fs::write(
            obj_dir.join("fields").join("Account__c.field-meta.xml"),
            field_xml,
        )
        .unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert_eq!(meta.fields.len(), 1);
        assert_eq!(meta.fields[0].field_type, "Lookup");
        assert_eq!(meta.fields[0].reference_to, "Account");
        assert!(meta.fields[0].required);
    }

    #[test]
    fn empty_description_gives_empty_string() {
        let tmp = TempDir::new().unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Account</label>
</CustomObject>"#;
        let path = tmp.path().join("Account.object-meta.xml");
        std::fs::write(&path, xml).unwrap();

        let meta = parse_object(&path, xml).unwrap();
        assert!(meta.description.is_empty());
    }
}
