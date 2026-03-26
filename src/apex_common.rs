use regex::Regex;
use std::sync::OnceLock;

/// Matches PascalCase identifiers that look like class/type names.
pub fn re_type_ref() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][a-zA-Z0-9_]+)\b").unwrap())
}

/// Matches ApexDoc block comments: `/** ... */`
pub fn re_apexdoc() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/\*\*[\s\S]*?\*/").unwrap())
}

fn re_tag() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@tag\s+(\w[\w-]*)").unwrap())
}

/// Extracts `@tag <label>` annotations from ApexDoc comment strings.
pub fn extract_tags(comments: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    for comment in comments {
        for caps in re_tag().captures_iter(comment) {
            tags.push(caps[1].to_string());
        }
    }
    tags
}

/// Apex primitive / built-in types to exclude from cross-reference lists.
pub const APEX_BUILTINS: &[&str] = &[
    "String",
    "Integer",
    "Long",
    "Double",
    "Decimal",
    "Boolean",
    "Date",
    "DateTime",
    "Time",
    "Blob",
    "Id",
    "Object",
    "List",
    "Map",
    "Set",
    "SObject",
    "Schema",
    "Database",
    "System",
    "Math",
    "JSON",
    "Type",
    "Exception",
    "DmlException",
    "QueryException",
    "Test",
    "ApexPages",
    "PageReference",
    "SelectOption",
    "Messaging",
    "Approval",
    "UserInfo",
    "Label",
    "Site",
    "Network",
    "ConnectApi",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_ref_regex_matches_pascal_case() {
        let text = "AccountService handler = new AccountService();";
        let matches: Vec<String> = re_type_ref()
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect();
        assert!(matches.contains(&"AccountService".to_string()));
    }

    #[test]
    fn type_ref_regex_does_not_match_lowercase() {
        let text = "string name = 'hello';";
        let matches: Vec<String> = re_type_ref()
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect();
        // Only PascalCase (starting uppercase) should match
        assert!(
            !matches.iter().any(|m| m == "string"),
            "lowercase 'string' should not match"
        );
    }

    #[test]
    fn builtins_list_contains_common_types() {
        assert!(APEX_BUILTINS.contains(&"String"));
        assert!(APEX_BUILTINS.contains(&"Integer"));
        assert!(APEX_BUILTINS.contains(&"Boolean"));
        assert!(APEX_BUILTINS.contains(&"List"));
        assert!(APEX_BUILTINS.contains(&"Map"));
        assert!(APEX_BUILTINS.contains(&"Set"));
        assert!(APEX_BUILTINS.contains(&"SObject"));
        assert!(APEX_BUILTINS.contains(&"Database"));
    }

    #[test]
    fn builtins_list_does_not_contain_custom_types() {
        assert!(!APEX_BUILTINS.contains(&"AccountService"));
        assert!(!APEX_BUILTINS.contains(&"MyCustomType"));
    }
}
