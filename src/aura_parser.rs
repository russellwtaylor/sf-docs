use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
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

/// Extracts all `key="value"` or `key='value'` pairs from an XML attribute string in a single pass.
fn parse_attrs(tag_attrs: &str) -> HashMap<&str, &str> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"(\w+)=["']([^"']*)["']"#).unwrap());
    re.captures_iter(tag_attrs)
        .filter_map(|c| {
            let key = c.get(1)?.as_str();
            let val = c.get(2)?.as_str();
            Some((key, val))
        })
        .collect()
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
        let attrs = parse_attrs(&caps[1]);
        attrs
            .get("extends")
            .map(|v| v.trim().to_string())
            .filter(|s| !s.is_empty())
    });

    // -----------------------------------------------------------------------
    // Parse <aura:attribute> declarations
    // -----------------------------------------------------------------------
    let mut attributes: Vec<AuraAttributeMetadata> = Vec::new();

    for caps in re_aura_attribute().captures_iter(cmp_source) {
        let attrs = parse_attrs(&caps[1]);

        let name = attrs
            .get("name")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();

        let attr_type = attrs
            .get("type")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();

        let default = attrs
            .get("default")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();

        let description = attrs
            .get("description")
            .map(|v| v.trim().to_string())
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
        let attrs = parse_attrs(&caps[1]);

        // Prefer `name` attribute first, fall back to `event` or `type`
        let event_name = attrs
            .get("name")
            .or_else(|| attrs.get("event"))
            .or_else(|| attrs.get("type"))
            .map(|v| v.trim().to_string())
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

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_component_tag_returns_defaults() {
        let tmp = TempDir::new().unwrap();
        let cmp = "<aura:component></aura:component>";
        let path = setup_component(&tmp, "emptyComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.component_name, "emptyComp");
        assert!(result.attributes.is_empty());
        assert!(result.events_handled.is_empty());
        assert!(result.extends.is_none());
    }

    #[test]
    fn attribute_with_all_fields() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="mode" type="String" default="view" description="Display mode: view or edit"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes[0].name, "mode");
        assert_eq!(result.attributes[0].attr_type, "String");
        assert_eq!(result.attributes[0].default, "view");
        assert_eq!(result.attributes[0].description, "Display mode: view or edit");
    }

    #[test]
    fn attribute_without_optional_fields() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="items" type="List"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes[0].name, "items");
        assert_eq!(result.attributes[0].attr_type, "List");
        assert!(result.attributes[0].default.is_empty());
        assert!(result.attributes[0].description.is_empty());
    }

    #[test]
    fn attribute_with_empty_name_skipped() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="" type="String"/>
    <aura:attribute name="valid" type="String"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes.len(), 1);
        assert_eq!(result.attributes[0].name, "valid");
    }

    #[test]
    fn duplicate_events_deduplicated() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:handler event="c:myEvent" action="{!c.handle1}"/>
    <aura:handler event="c:myEvent" action="{!c.handle2}"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.events_handled.iter().filter(|e| e.as_str() == "c:myEvent").count(), 1);
    }

    #[test]
    fn extends_with_namespace() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component extends="lightning:baseComponent"></aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.extends.as_deref(), Some("lightning:baseComponent"));
    }
}
