use regex::Regex;
use std::sync::OnceLock;

/// Matches PascalCase identifiers that look like class/type names.
pub fn re_type_ref() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][a-zA-Z0-9_]+)\b").unwrap())
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
