use std::path::PathBuf;

/// A discovered .cls file with its raw source content.
#[derive(Debug, Clone)]
pub struct ApexFile {
    pub path: PathBuf,
    pub filename: String,
    pub raw_source: String,
}

/// Structural metadata extracted from an Apex class by the parser.
#[derive(Debug, Clone, Default)]
pub struct ClassMetadata {
    pub class_name: String,
    pub access_modifier: String,
    pub is_abstract: bool,
    pub is_virtual: bool,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub methods: Vec<MethodMetadata>,
    pub properties: Vec<PropertyMetadata>,
    pub existing_comments: Vec<String>,
    /// Other class names referenced in this class (field types, param types, return types).
    pub references: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MethodMetadata {
    pub name: String,
    pub access_modifier: String,
    pub return_type: String,
    pub params: Vec<ParamMetadata>,
    pub is_static: bool,
    pub existing_comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParamMetadata {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone)]
pub struct PropertyMetadata {
    pub name: String,
    pub access_modifier: String,
    pub property_type: String,
    pub is_static: bool,
    pub existing_comment: Option<String>,
}

/// AI-generated documentation for a class, parsed from the Gemini response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClassDocumentation {
    pub class_name: String,
    pub summary: String,
    pub description: String,
    pub methods: Vec<MethodDocumentation>,
    pub properties: Vec<PropertyDocumentation>,
    pub usage_examples: Vec<String>,
    pub relationships: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MethodDocumentation {
    pub name: String,
    pub description: String,
    pub params: Vec<ParamDocumentation>,
    pub returns: String,
    pub throws: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ParamDocumentation {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PropertyDocumentation {
    pub name: String,
    pub description: String,
}
