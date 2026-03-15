use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

use crate::types::{AuraAttributeMetadata, AuraMetadata};

// ---------------------------------------------------------------------------
// Compiled regexes (compiled once)
// ---------------------------------------------------------------------------

/// Matches `<aura:attribute name="..." type="..." .../>` or `<aura:attribute ... >`.
/// Captures named attributes from within the opening tag.
fn re_aura_attribute() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<aura:attribute\s+([^>]*?)/?>"#).unwrap())
}

/// Extracts a named attribute value from a tag's attribute string.
/// e.g. `name="foo"` or `name='foo'`
fn re_attr_value(attr_name: &str) -> Regex {
    Regex::new(&format!(
        r#"(?:^|\s){}=["']([^"']*)["']"#,
        regex::escape(attr_name)
    ))
    .unwrap()
}

/// Matches `<aura:registerEvent .../>` or `<aura:handler .../>`.
fn re_aura_event() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<aura:(?:registerEvent|handler)\s+([^>]*?)/?>"#).unwrap())
}

/// Matches `<aura:component ...>` opening tag.
fn re_aura_component() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<aura:component\s*([^>]*)>"#).unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse Aura component metadata from the component's `.cmp` source.
///
/// `path` should point to the `.cmp` file; the component name is derived from
/// the parent directory name.
pub fn parse_aura(path: &Path, cmp_source: &str) -> Result<AuraMetadata> {
    let component_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // -----------------------------------------------------------------------
    // Parse <aura:component> opening tag for `extends`
    // -----------------------------------------------------------------------
    let extends = re_aura_component().captures(cmp_source).and_then(|caps| {
        let tag_attrs = &caps[1];
        let re = re_attr_value("extends");
        re.captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .filter(|s| !s.is_empty())
    });

    // -----------------------------------------------------------------------
    // Parse <aura:attribute> declarations
    // -----------------------------------------------------------------------
    let mut attributes: Vec<AuraAttributeMetadata> = Vec::new();

    for caps in re_aura_attribute().captures_iter(cmp_source) {
        let tag_attrs = &caps[1];

        let name = re_attr_value("name")
            .captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        let attr_type = re_attr_value("type")
            .captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        let default = re_attr_value("default")
            .captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        let description = re_attr_value("description")
            .captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_default();

        if !name.is_empty() {
            attributes.push(AuraAttributeMetadata {
                name,
                attr_type,
                default,
                description,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Parse event registrations / handlers
    // -----------------------------------------------------------------------
    let mut events_handled: Vec<String> = Vec::new();

    for caps in re_aura_event().captures_iter(cmp_source) {
        let tag_attrs = &caps[1];

        // Prefer `name` attribute first, fall back to `event` or `type`
        let event_name = re_attr_value("name")
            .captures(tag_attrs)
            .map(|c| c[1].trim().to_string())
            .or_else(|| {
                re_attr_value("event")
                    .captures(tag_attrs)
                    .map(|c| c[1].trim().to_string())
            })
            .or_else(|| {
                re_attr_value("type")
                    .captures(tag_attrs)
                    .map(|c| c[1].trim().to_string())
            })
            .unwrap_or_default();

        if !event_name.is_empty() && !events_handled.contains(&event_name) {
            events_handled.push(event_name);
        }
    }

    Ok(AuraMetadata {
        component_name,
        attributes,
        events_handled,
        extends,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_component(tmp: &TempDir, name: &str, cmp: &str) -> std::path::PathBuf {
        let comp_dir = tmp.path().join(name);
        fs::create_dir_all(&comp_dir).unwrap();
        let cmp_path = comp_dir.join(format!("{name}.cmp"));
        fs::write(&cmp_path, cmp).unwrap();
        cmp_path
    }

    #[test]
    fn extracts_component_name() {
        let tmp = TempDir::new().unwrap();
        let path = setup_component(
            &tmp,
            "myComp",
            "<aura:component><aura:attribute name=\"title\" type=\"String\"/></aura:component>",
        );
        let result = parse_aura(
            &path,
            "<aura:component><aura:attribute name=\"title\" type=\"String\"/></aura:component>",
        )
        .unwrap();
        assert_eq!(result.component_name, "myComp");
    }

    #[test]
    fn extracts_attribute() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="recordId" type="Id" description="The record Id"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes.len(), 1);
        assert_eq!(result.attributes[0].name, "recordId");
        assert_eq!(result.attributes[0].attr_type, "Id");
        assert_eq!(result.attributes[0].description, "The record Id");
    }

    #[test]
    fn extracts_multiple_attributes() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="title" type="String" default="Default Title"/>
    <aura:attribute name="count" type="Integer"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes.len(), 2);
        assert_eq!(result.attributes[0].default, "Default Title");
    }

    #[test]
    fn extracts_extends() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component extends="c:baseComponent">
    <aura:attribute name="value" type="String"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.extends.as_deref(), Some("c:baseComponent"));
    }

    #[test]
    fn extracts_events_handled() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:registerEvent name="onSave" type="c:saveEvent"/>
    <aura:handler event="c:updateEvent" action="{!c.handleUpdate}"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert!(result.events_handled.contains(&"onSave".to_string()));
        assert!(
            result.events_handled.contains(&"c:updateEvent".to_string()),
            "expected c:updateEvent, got {:?}",
            result.events_handled
        );
    }

    #[test]
    fn no_extends_when_absent() {
        let tmp = TempDir::new().unwrap();
        let cmp = "<aura:component></aura:component>";
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert!(result.extends.is_none());
    }
}
