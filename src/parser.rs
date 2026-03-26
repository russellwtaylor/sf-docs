use anyhow::Result;
use regex::Regex;
use std::sync::OnceLock;

use crate::apex_common::{extract_tags, re_type_ref, APEX_BUILTINS};
use crate::types::{ClassMetadata, MethodMetadata, ParamMetadata, PropertyMetadata};

// ---------------------------------------------------------------------------
// Compiled regex patterns (lazy-initialized once)
// ---------------------------------------------------------------------------

fn re_class() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Matches both `class` and `interface` keyword declarations.
        // implements allows dots, angle brackets, and brackets for generic types like Database.Batchable<SObject>
        Regex::new(
            r"(?i)(?P<access>public|private|protected|global)\s+(?P<mods>(?:(?:abstract|virtual|with\s+sharing|without\s+sharing|inherited\s+sharing)\s+)*)(?P<keyword>class|interface)\s+(?P<name>\w+)(?:\s+extends\s+(?P<extends>\w+))?(?:\s+implements\s+(?P<implements>[^{]+?))?\s*\{",
        )
        .unwrap()
    })
}

fn re_method() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?P<access>public|private|protected|global|@\w+)\s+(?P<mods>(?:(?:static|override|virtual|abstract)\s+)*)(?P<return>[\w<>\[\],\s]+?)\s+(?P<name>[a-zA-Z_]\w*)\s*\((?P<params>[^)]*)\)\s*(?:\{|;)",
        )
        .unwrap()
    })
}

/// Matches interface method declarations that have no access modifier:
/// e.g. `void process(List<Account> accounts);`
/// Group `return` = return type, `name` = method name, `params` = raw params string.
/// Matches interface method declarations that have no access modifier:
/// e.g. `void process(List<Account> accounts);`
/// Uses `(?m)` so `^` matches the start of each line.
fn re_interface_method() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?im)^\s*(?P<return>(?:void|[\w<>\[\],\s]+?))\s+(?P<name>[a-zA-Z_]\w*)\s*\((?P<params>[^)]*)\)\s*;",
        )
        .unwrap()
    })
}

fn re_property() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?P<access>public|private|protected|global)\s+(?P<mods>(?:(?:static|final)\s+)*)(?P<type>[\w<>\[\],\s]+?)\s+(?P<name>[a-zA-Z_]\w*)\s*(?:=|;|\{)",
        )
        .unwrap()
    })
}

fn re_apexdoc() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/\*\*[\s\S]*?\*/").unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn parse_apex_class(source: &str) -> Result<ClassMetadata> {
    let mut meta = ClassMetadata::default();

    // Strip single-line and block comments before parsing structure,
    // but keep ApexDoc comments so we can extract them first.
    let apexdoc_comments = extract_apexdoc_comments(source);
    meta.existing_comments = apexdoc_comments;
    meta.tags = extract_tags(&meta.existing_comments);

    // Strip all comments for structural parsing
    let stripped = strip_comments(source);

    parse_class_declaration(&stripped, &mut meta);
    parse_methods(&stripped, &mut meta);
    parse_properties(&stripped, &mut meta);
    parse_references(source, &mut meta);

    Ok(meta)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_apexdoc_comments(source: &str) -> Vec<String> {
    re_apexdoc()
        .find_iter(source)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn re_strip_block() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/\*[\s\S]*?\*/").unwrap())
}

fn re_strip_line() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"//[^\n]*").unwrap())
}

fn strip_comments(source: &str) -> String {
    let no_block = re_strip_block().replace_all(source, " ");
    re_strip_line().replace_all(&no_block, "").to_string()
}

fn parse_class_declaration(source: &str, meta: &mut ClassMetadata) {
    if let Some(caps) = re_class().captures(source) {
        meta.class_name = caps.name("name").map_or("", |m| m.as_str()).to_string();
        meta.access_modifier = caps
            .name("access")
            .map_or("", |m| m.as_str())
            .to_lowercase();

        let keyword = caps
            .name("keyword")
            .map_or("class", |m| m.as_str())
            .to_lowercase();
        meta.is_interface = keyword == "interface";

        let mods = caps.name("mods").map_or("", |m| m.as_str()).to_lowercase();
        meta.is_abstract = mods.contains("abstract");
        meta.is_virtual = mods.contains("virtual");

        meta.extends = caps
            .name("extends")
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        meta.implements = caps
            .name("implements")
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
    }
}

fn parse_methods(source: &str, meta: &mut ClassMetadata) {
    for caps in re_method().captures_iter(source) {
        let name = caps.name("name").map_or("", |m| m.as_str()).to_string();

        // Skip common false-positives: control flow keywords, class declaration itself
        if matches!(
            name.as_str(),
            "if" | "else" | "for" | "while" | "catch" | "class" | "return" | "new"
        ) {
            continue;
        }
        // Skip the class declaration match (class name = method name)
        if name == meta.class_name {
            continue;
        }

        let mods = caps.name("mods").map_or("", |m| m.as_str()).to_lowercase();
        let params_raw = caps.name("params").map_or("", |m| m.as_str());

        let params = parse_params(params_raw);

        meta.methods.push(MethodMetadata {
            name,
            access_modifier: caps
                .name("access")
                .map_or("", |m| m.as_str())
                .to_lowercase(),
            return_type: caps
                .name("return")
                .map_or("", |m| m.as_str())
                .trim()
                .to_string(),
            is_static: mods.contains("static"),
            params,
        });
    }

    // For interfaces: method declarations have no access modifier, so the normal
    // regex misses them. Apply the interface-method pattern as a supplemental pass.
    if meta.is_interface {
        let existing_names: std::collections::HashSet<String> =
            meta.methods.iter().map(|m| m.name.clone()).collect();
        let mut extra: Vec<MethodMetadata> = Vec::new();
        for caps in re_interface_method().captures_iter(source) {
            let name = caps.name("name").map_or("", |m| m.as_str()).to_string();
            if existing_names.contains(&name) || name == meta.class_name {
                continue;
            }
            let params_raw = caps.name("params").map_or("", |m| m.as_str());
            extra.push(MethodMetadata {
                name,
                access_modifier: String::new(),
                return_type: caps
                    .name("return")
                    .map_or("", |m| m.as_str())
                    .trim()
                    .to_string(),
                is_static: false,
                params: parse_params(params_raw),
            });
        }
        meta.methods.extend(extra);
    }

    // Remove exact-duplicate signatures only. Two overloads that share a name
    // but differ in parameter types must both be kept.
    meta.methods.dedup_by(|a, b| {
        a.name == b.name
            && a.return_type == b.return_type
            && a.params.len() == b.params.len()
            && a.params
                .iter()
                .zip(b.params.iter())
                .all(|(pa, pb)| pa.param_type == pb.param_type)
    });
}

fn parse_params(params_raw: &str) -> Vec<ParamMetadata> {
    if params_raw.trim().is_empty() {
        return vec![];
    }
    params_raw
        .split(',')
        .filter_map(|p| {
            let parts: Vec<&str> = p.trim().splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                Some(ParamMetadata {
                    param_type: parts[0].trim().to_string(),
                    name: parts[1].trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn parse_properties(source: &str, meta: &mut ClassMetadata) {
    // We want to avoid matching method signatures as properties.
    // Strategy: only capture lines that do NOT contain a '(' before ';' or '{'.
    let method_names: std::collections::HashSet<&str> =
        meta.methods.iter().map(|m| m.name.as_str()).collect();

    for caps in re_property().captures_iter(source) {
        let name = caps.name("name").map_or("", |m| m.as_str()).to_string();

        // Skip if it matches a method name or is a keyword
        if method_names.contains(name.as_str())
            || matches!(
                name.as_str(),
                "if" | "else" | "for" | "while" | "catch" | "return" | "new" | "class"
            )
        {
            continue;
        }
        // Skip if the "type" looks like a return-type fragment from a method
        let type_str = caps
            .name("type")
            .map_or("", |m| m.as_str())
            .trim()
            .to_string();
        if type_str.is_empty() || type_str.contains('(') {
            continue;
        }

        let mods = caps.name("mods").map_or("", |m| m.as_str()).to_lowercase();

        meta.properties.push(PropertyMetadata {
            name,
            access_modifier: caps
                .name("access")
                .map_or("", |m| m.as_str())
                .to_lowercase(),
            property_type: type_str,
            is_static: mods.contains("static"),
        });
    }

    meta.properties
        .dedup_by(|a, b| a.name == b.name && a.property_type == b.property_type);
}

fn parse_references(source: &str, meta: &mut ClassMetadata) {
    let mut refs: Vec<String> = re_type_ref()
        .captures_iter(source)
        .map(|c| c[1].to_string())
        .filter(|name| {
            !APEX_BUILTINS.contains(&name.as_str()) && name != &meta.class_name && name.len() > 2
        })
        .collect();

    refs.sort();
    refs.dedup();
    meta.references = refs;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CLASS: &str = r#"
/**
 * Service class for account operations.
 */
public class AccountService extends BaseService implements Queueable, Database.Batchable<SObject> {

    private static final Integer MAX_RETRIES = 3;
    public String name;
    private AccountRepository repo;

    /**
     * Processes accounts.
     */
    public void processAccounts(List<Account> accounts, Boolean force) {
        // implementation
    }

    public static AccountService getInstance() {
        return new AccountService();
    }

    private Integer retry(Integer count) {
        return count + 1;
    }
}
"#;

    #[test]
    fn parses_class_name() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        assert_eq!(meta.class_name, "AccountService");
    }

    #[test]
    fn parses_access_modifier() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        assert_eq!(meta.access_modifier, "public");
    }

    #[test]
    fn parses_extends() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        assert_eq!(meta.extends.as_deref(), Some("BaseService"));
    }

    #[test]
    fn parses_implements() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        assert!(meta.implements.contains(&"Queueable".to_string()));
    }

    #[test]
    fn parses_methods() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        let method_names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(
            method_names.contains(&"processAccounts"),
            "missing processAccounts: {:?}",
            method_names
        );
        assert!(
            method_names.contains(&"getInstance"),
            "missing getInstance: {:?}",
            method_names
        );
        assert!(
            method_names.contains(&"retry"),
            "missing retry: {:?}",
            method_names
        );
    }

    #[test]
    fn parses_static_method() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        let get_instance = meta
            .methods
            .iter()
            .find(|m| m.name == "getInstance")
            .unwrap();
        assert!(get_instance.is_static);
    }

    #[test]
    fn parses_method_params() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        let process = meta
            .methods
            .iter()
            .find(|m| m.name == "processAccounts")
            .unwrap();
        assert_eq!(process.params.len(), 2);
        assert_eq!(process.params[0].name, "accounts");
        assert_eq!(process.params[1].name, "force");
    }

    #[test]
    fn extracts_apexdoc_comments() {
        let meta = parse_apex_class(SAMPLE_CLASS).unwrap();
        assert!(!meta.existing_comments.is_empty());
        assert!(meta.existing_comments[0].contains("Service class"));
    }

    #[test]
    fn parses_abstract_class() {
        let src = "public abstract class MyAbstract { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.is_abstract);
    }

    #[test]
    fn parses_virtual_class() {
        let src = "global virtual class MyVirtual { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.is_virtual);
        assert_eq!(meta.access_modifier, "global");
    }

    #[test]
    fn parses_interface() {
        let src = "public interface IAccountService { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(
            meta.is_interface,
            "expected is_interface=true for interface declaration"
        );
        assert_eq!(meta.class_name, "IAccountService");
    }

    #[test]
    fn interface_methods_not_empty() {
        let src = r#"
public interface IProcessor {
    void process(List<Account> accounts);
    Boolean validate(Account acc);
}
"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.is_interface);
        // Interface method declarations end with ';' — the parser should still pick them up.
        let method_names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(
            method_names.contains(&"process") || method_names.contains(&"validate"),
            "expected at least one interface method, got {:?}",
            method_names
        );
    }

    #[test]
    fn class_is_not_interface() {
        let src = "public class AccountService { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(
            !meta.is_interface,
            "regular class should not be flagged as interface"
        );
    }

    #[test]
    fn keeps_overloaded_methods_with_different_param_types() {
        let src = r#"
public class OverloadService {
    public void process(List<Account> accounts) {}
    public void process(Set<Id> ids) {}
}
"#;
        let meta = parse_apex_class(src).unwrap();
        let overloads: Vec<_> = meta
            .methods
            .iter()
            .filter(|m| m.name == "process")
            .collect();
        assert_eq!(
            overloads.len(),
            2,
            "both overloads should be kept: {:?}",
            overloads
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_source_returns_default_metadata() {
        let meta = parse_apex_class("").unwrap();
        assert!(meta.class_name.is_empty());
        assert!(meta.methods.is_empty());
        assert!(meta.properties.is_empty());
    }

    #[test]
    fn whitespace_only_source_returns_default_metadata() {
        let meta = parse_apex_class("   \n\t\n  ").unwrap();
        assert!(meta.class_name.is_empty());
    }

    #[test]
    fn parses_with_sharing_class() {
        let src = "public with sharing class MyService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "MyService");
        assert_eq!(meta.access_modifier, "public");
    }

    #[test]
    fn parses_without_sharing_class() {
        let src = "public without sharing class SystemService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "SystemService");
    }

    #[test]
    fn parses_inherited_sharing_class() {
        let src = "public inherited sharing class DelegateService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "DelegateService");
    }

    #[test]
    fn parses_global_access_modifier() {
        let src = "global class WebServiceHandler { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.access_modifier, "global");
    }

    #[test]
    fn parses_private_inner_class_style() {
        let src = "private class InnerHelper { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.access_modifier, "private");
        assert_eq!(meta.class_name, "InnerHelper");
    }

    #[test]
    fn no_extends_gives_none() {
        let src = "public class Simple { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.extends.is_none());
    }

    #[test]
    fn no_implements_gives_empty_vec() {
        let src = "public class Simple { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.implements.is_empty());
    }

    #[test]
    fn multiple_implements() {
        let src =
            "public class Multi implements Queueable, Schedulable, Database.Batchable<SObject> { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(
            meta.implements.len() >= 2,
            "expected at least 2 interfaces, got {:?}",
            meta.implements
        );
    }

    #[test]
    fn method_with_no_params() {
        let src = r#"public class Svc {
    public void doWork() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let m = meta.methods.iter().find(|m| m.name == "doWork").unwrap();
        assert!(m.params.is_empty());
    }

    #[test]
    fn strips_single_line_comments_before_parsing() {
        let src = r#"public class Svc {
    // public void hiddenMethod() { }
    public void realMethod() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"realMethod"));
        assert!(
            !names.contains(&"hiddenMethod"),
            "comment method should not be parsed: {:?}",
            names
        );
    }

    #[test]
    fn strips_block_comments_before_parsing() {
        let src = r#"public class Svc {
    /* public void hidden() { } */
    public void visible() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"visible"));
    }

    #[test]
    fn skips_control_flow_keywords_as_methods() {
        let src = r#"public class Svc {
    public void process() {
        if (true) { }
        for (Integer i = 0; i < 10; i++) { }
        while (true) { }
    }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(!names.contains(&"if"));
        assert!(!names.contains(&"for"));
        assert!(!names.contains(&"while"));
    }

    #[test]
    fn parses_override_method() {
        let src = r#"public class Child extends Parent {
    public override void process() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.methods.iter().any(|m| m.name == "process"));
    }

    #[test]
    fn parses_abstract_method_declaration() {
        let src = r#"public abstract class Base {
    public abstract void execute();
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.is_abstract);
    }

    #[test]
    fn references_exclude_builtin_types() {
        let src = r#"public class Svc {
    public String name;
    public Integer count;
    public CustomType__c custom;
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(!meta.references.contains(&"String".to_string()));
        assert!(!meta.references.contains(&"Integer".to_string()));
    }

    // -----------------------------------------------------------------------
    // @tag annotation tests
    // -----------------------------------------------------------------------

    #[test]
    fn parses_single_tag() {
        let source = r#"
    /**
     * @tag billing
     * Service class for orders
     */
    public class OrderService {
    }
    "#;
        let meta = parse_apex_class(source).unwrap();
        assert_eq!(meta.tags, vec!["billing"]);
    }

    #[test]
    fn parses_multiple_tags() {
        let source = r#"
    /**
     * @tag billing
     * @tag integration
     * Service class
     */
    public class OrderService {
    }
    "#;
        let meta = parse_apex_class(source).unwrap();
        assert_eq!(meta.tags, vec!["billing", "integration"]);
    }

    #[test]
    fn parses_hyphenated_tag() {
        let source = r#"
    /**
     * @tag order-management
     */
    public class OrderService {
    }
    "#;
        let meta = parse_apex_class(source).unwrap();
        assert_eq!(meta.tags, vec!["order-management"]);
    }

    #[test]
    fn no_tags_returns_empty() {
        let source = r#"
    /**
     * Service class
     */
    public class OrderService {
    }
    "#;
        let meta = parse_apex_class(source).unwrap();
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn references_exclude_self_class_name() {
        let src = r#"public class AccountService {
    public AccountService getInstance() { return new AccountService(); }
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(!meta.references.contains(&"AccountService".to_string()));
    }
}
