use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

use crate::types::{LwcApiProp, LwcMetadata};

// ---------------------------------------------------------------------------
// Compiled regexes (compiled once)
// ---------------------------------------------------------------------------

/// Matches `@api` followed immediately by a property declaration.
/// Captures: `@api propName` or `@api propName = ...` or `@api get propName()`.
/// Group 1 = property/getter name.
fn re_api_prop() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@api\s+(?:get\s+)?([a-zA-Z_$][a-zA-Z0-9_$]*)\s*[=;({]").unwrap())
}

/// Matches `@api methodName(` — an @api-decorated method.
/// Group 1 = method name.
fn re_api_method() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@api\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(").unwrap())
}

/// Matches a named `<slot name="...">` in HTML. Group 1 = slot name.
fn re_named_slot() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<slot\s[^>]*name=["']([^"']+)["']"#).unwrap())
}

/// Matches an anonymous `<slot>` or `<slot />` in HTML.
fn re_anon_slot() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<slot(\s|/|>)").unwrap())
}

/// Matches `<c-component-name` references in HTML. Group 1 = kebab name.
fn re_c_component() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<(c-[a-z][a-z0-9-]*)[\s/>]").unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse LWC metadata from the component's JS source (`raw_source`) and the
/// sibling HTML file (read from `path`'s directory).
///
/// `path` should be the `.js-meta.xml` path; the component name is derived
/// from the parent directory name.
pub fn parse_lwc(path: &Path, js_source: &str) -> Result<LwcMetadata> {
    // Derive component name from the parent directory (the component folder).
    let component_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // -----------------------------------------------------------------------
    // Parse JS source for @api props and methods
    // -----------------------------------------------------------------------
    let mut api_props: Vec<LwcApiProp> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Collect @api methods first (pattern: `@api name(`)
    for cap in re_api_method().captures_iter(js_source) {
        let name = cap[1].to_string();
        if seen_names.insert(name.clone()) {
            api_props.push(LwcApiProp {
                name,
                is_method: true,
            });
        }
    }

    // Then collect @api properties (`@api name =` or `@api name;` or `@api get name(`)
    for cap in re_api_prop().captures_iter(js_source) {
        let name = cap[1].to_string();
        // Check if the suffix after the name is `(` — that means it matched a
        // method-style getter, not a plain property; still model as a property.
        if seen_names.insert(name.clone()) {
            api_props.push(LwcApiProp {
                name,
                is_method: false,
            });
        }
    }

    // Sort for deterministic output
    api_props.sort_by(|a, b| a.name.cmp(&b.name));

    // -----------------------------------------------------------------------
    // Read sibling HTML file for slot and component reference extraction
    // -----------------------------------------------------------------------
    let html_content = read_sibling_html(path, &component_name);

    let mut slots: Vec<String> = Vec::new();
    let mut referenced_components: Vec<String> = Vec::new();

    if let Some(html) = &html_content {
        // Named slots
        for cap in re_named_slot().captures_iter(html) {
            let name = cap[1].to_string();
            if !slots.contains(&name) {
                slots.push(name);
            }
        }

        // Anonymous slot (if no named slot already added a default)
        if re_anon_slot().is_match(html) && !slots.iter().any(|s| s == "default") {
            slots.push("default".to_string());
        }

        // c-* component references — convert kebab-case to camelCase for cross-linking
        for cap in re_c_component().captures_iter(html) {
            let kebab = cap[1].trim_start_matches("c-").to_string();
            let camel = kebab_to_camel(&kebab);
            if !referenced_components.contains(&camel) {
                referenced_components.push(camel);
            }
        }
    }

    Ok(LwcMetadata {
        component_name,
        api_props,
        slots,
        referenced_components,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Attempts to read the `<componentName>.html` file from the same directory as
/// the `.js-meta.xml` file. Returns `None` if the file cannot be read.
fn read_sibling_html(meta_path: &Path, component_name: &str) -> Option<String> {
    let html_path = meta_path.parent()?.join(format!("{component_name}.html"));
    std::fs::read_to_string(html_path).ok()
}

/// Converts a kebab-case string to camelCase (e.g. `my-component` → `myComponent`).
fn kebab_to_camel(kebab: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in kebab.chars() {
        if ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_component(tmp: &TempDir, name: &str, js: &str, html: &str) -> std::path::PathBuf {
        let comp_dir = tmp.path().join(name);
        fs::create_dir_all(&comp_dir).unwrap();
        if !js.is_empty() {
            fs::write(comp_dir.join(format!("{name}.js")), js).unwrap();
        }
        if !html.is_empty() {
            fs::write(comp_dir.join(format!("{name}.html")), html).unwrap();
        }
        let meta_path = comp_dir.join(format!("{name}.js-meta.xml"));
        fs::write(&meta_path, "<LightningComponentBundle/>").unwrap();
        meta_path
    }

    #[test]
    fn extracts_component_name() {
        let tmp = TempDir::new().unwrap();
        let meta = setup_component(&tmp, "myComponent", "", "");
        let result = parse_lwc(&meta, "").unwrap();
        assert_eq!(result.component_name, "myComponent");
    }

    #[test]
    fn extracts_api_property() {
        let tmp = TempDir::new().unwrap();
        let js = "import { LightningElement, api } from 'lwc';\nexport default class MyComp extends LightningElement {\n    @api recordId;\n}\n";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        assert!(result.api_props.iter().any(|p| p.name == "recordId"));
    }

    #[test]
    fn extracts_api_method() {
        let tmp = TempDir::new().unwrap();
        let js = "@api focus() { this.template.querySelector('input').focus(); }";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        let method = result.api_props.iter().find(|p| p.name == "focus").unwrap();
        assert!(method.is_method);
    }

    #[test]
    fn extracts_named_slots_from_html() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template><slot name="header"></slot><slot name="body"></slot></template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert!(result.slots.contains(&"header".to_string()));
        assert!(result.slots.contains(&"body".to_string()));
    }

    #[test]
    fn extracts_anonymous_slot() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template><slot></slot></template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert!(result.slots.contains(&"default".to_string()));
    }

    #[test]
    fn extracts_c_component_references() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template><c-my-button label="Click"></c-my-button></template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert!(result
            .referenced_components
            .contains(&"myButton".to_string()));
    }

    #[test]
    fn kebab_to_camel_converts_correctly() {
        assert_eq!(kebab_to_camel("my-button"), "myButton");
        assert_eq!(kebab_to_camel("account-detail-card"), "accountDetailCard");
        assert_eq!(kebab_to_camel("simple"), "simple");
    }

    #[test]
    fn no_html_file_gives_empty_slots_and_references() {
        let tmp = TempDir::new().unwrap();
        // Component with JS but no HTML template
        let meta = setup_component(&tmp, "myComp", "@api recordId;", "");
        // Delete the html file that setup_component wrote (empty string → no file written)
        let result = parse_lwc(&meta, "@api recordId;").unwrap();
        assert!(result.slots.is_empty());
        assert!(result.referenced_components.is_empty());
    }

    #[test]
    fn duplicate_api_decorator_deduplicates() {
        let tmp = TempDir::new().unwrap();
        // Same property appears twice (e.g. copy-paste error in source)
        let js = "@api recordId;\n@api recordId;";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        assert_eq!(
            result
                .api_props
                .iter()
                .filter(|p| p.name == "recordId")
                .count(),
            1,
            "duplicate @api prop should appear only once"
        );
    }

    #[test]
    fn anonymous_slot_added_only_once() {
        let tmp = TempDir::new().unwrap();
        // Two anonymous slots in the template — default should only appear once
        let html = "<template><slot></slot><slot/></template>";
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert_eq!(
            result
                .slots
                .iter()
                .filter(|s| s.as_str() == "default")
                .count(),
            1,
            "default slot should appear exactly once"
        );
    }

    #[test]
    fn named_slot_not_duplicated() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template><slot name="header"></slot><slot name="header"></slot></template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert_eq!(
            result
                .slots
                .iter()
                .filter(|s| s.as_str() == "header")
                .count(),
            1,
            "named slot should appear exactly once even if repeated"
        );
    }

    #[test]
    fn api_getter_treated_as_property() {
        let tmp = TempDir::new().unwrap();
        let js = "@api get value() { return this._value; }";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        let prop = result.api_props.iter().find(|p| p.name == "value").unwrap();
        assert!(
            !prop.is_method,
            "@api getter should be treated as a property, not a method"
        );
    }

    #[test]
    fn multiple_c_component_refs_deduplicates() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template>
            <c-my-button label="A"></c-my-button>
            <c-my-button label="B"></c-my-button>
        </template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert_eq!(
            result
                .referenced_components
                .iter()
                .filter(|c| c.as_str() == "myButton")
                .count(),
            1,
            "same child component referenced twice should appear once"
        );
    }

    #[test]
    fn api_props_sorted_alphabetically() {
        let tmp = TempDir::new().unwrap();
        let js = "@api zebra;\n@api alpha;\n@api mango;";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        let names: Vec<&str> = result.api_props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"]);
    }
}
