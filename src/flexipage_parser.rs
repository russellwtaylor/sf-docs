use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::types::FlexiPageMetadata;

pub fn parse_flexipage(api_name: &str, source: &str) -> Result<FlexiPageMetadata> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut meta = FlexiPageMetadata {
        api_name: api_name.to_string(),
        ..Default::default()
    };

    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                stack.push(name);
            }
            Event::End(_) => {
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

                match parent {
                    "type" if grandparent == "FlexiPage" || grandparent.is_empty() => {
                        meta.page_type = text;
                    }
                    "masterLabel" => {
                        if meta.label.is_empty() {
                            meta.label = text;
                        }
                    }
                    "sobjectType" => {
                        meta.sobject = text;
                    }
                    "description" if grandparent == "FlexiPage" || grandparent.is_empty() => {
                        if meta.description.is_empty() {
                            meta.description = text;
                        }
                    }
                    "componentName" => {
                        // Strip c__ namespace prefix if present, but keep as-is otherwise
                        let name = text.strip_prefix("c__").unwrap_or(&text).to_string();
                        if !meta.component_names.contains(&name) {
                            meta.component_names.push(name);
                        }
                    }
                    "actionName" => {
                        if !meta.flow_names.contains(&text) {
                            meta.flow_names.push(text);
                        }
                    }
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

    const SAMPLE_FLEXIPAGE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Account Record Page</masterLabel>
    <type>RecordPage</type>
    <sobjectType>Account</sobjectType>
    <description>Custom record page for Account objects</description>
    <flexiPageRegions>
        <componentInstances>
            <componentInstance>
                <componentName>c__accountDetails</componentName>
            </componentInstance>
            <componentInstance>
                <componentName>c__relatedContacts</componentName>
            </componentInstance>
        </componentInstances>
    </flexiPageRegions>
</FlexiPage>"#;

    #[test]
    fn parses_api_name() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert_eq!(meta.api_name, "Account_Record_Page");
    }

    #[test]
    fn parses_label() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert_eq!(meta.label, "Account Record Page");
    }

    #[test]
    fn parses_page_type() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert_eq!(meta.page_type, "RecordPage");
    }

    #[test]
    fn parses_sobject() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert_eq!(meta.sobject, "Account");
    }

    #[test]
    fn parses_description() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert_eq!(meta.description, "Custom record page for Account objects");
    }

    #[test]
    fn parses_component_names_strips_c_prefix() {
        let meta = parse_flexipage("Account_Record_Page", SAMPLE_FLEXIPAGE).unwrap();
        assert!(
            meta.component_names.contains(&"accountDetails".to_string()),
            "expected accountDetails, got {:?}",
            meta.component_names
        );
        assert!(meta
            .component_names
            .contains(&"relatedContacts".to_string()));
    }
}
