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
}

/// Shared cross-linking index passed to every render context.
pub struct AllNames {
    pub class_names: Vec<String>,
    pub trigger_names: Vec<String>,
    pub flow_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Trigger types
// ---------------------------------------------------------------------------

/// Structural metadata extracted from an Apex trigger.
#[derive(Debug, Clone, Default)]
pub struct TriggerMetadata {
    pub trigger_name: String,
    pub sobject: String,
    pub events: Vec<TriggerEvent>,
    pub existing_comments: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerEvent {
    BeforeInsert,
    BeforeUpdate,
    BeforeDelete,
    AfterInsert,
    AfterUpdate,
    AfterDelete,
    AfterUndelete,
}

impl TriggerEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerEvent::BeforeInsert => "before insert",
            TriggerEvent::BeforeUpdate => "before update",
            TriggerEvent::BeforeDelete => "before delete",
            TriggerEvent::AfterInsert => "after insert",
            TriggerEvent::AfterUpdate => "after update",
            TriggerEvent::AfterDelete => "after delete",
            TriggerEvent::AfterUndelete => "after undelete",
        }
    }
}

/// AI-generated documentation for an Apex trigger.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TriggerDocumentation {
    pub trigger_name: String,
    pub sobject: String,
    pub summary: String,
    pub description: String,
    pub events: Vec<TriggerEventDocumentation>,
    pub handler_classes: Vec<String>,
    pub usage_notes: Vec<String>,
    pub relationships: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TriggerEventDocumentation {
    pub event: String,
    pub description: String,
}

// ---------------------------------------------------------------------------

/// AI-generated documentation for a class, parsed from the AI response.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ClassDocumentation {
    pub class_name: String,
    pub summary: String,
    pub description: String,
    pub methods: Vec<MethodDocumentation>,
    pub properties: Vec<PropertyDocumentation>,
    pub usage_examples: Vec<String>,
    pub relationships: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MethodDocumentation {
    pub name: String,
    pub description: String,
    pub params: Vec<ParamDocumentation>,
    pub returns: String,
    pub throws: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ParamDocumentation {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PropertyDocumentation {
    pub name: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Flow types
// ---------------------------------------------------------------------------

/// Structural metadata extracted from a Salesforce Flow XML file.
#[derive(Debug, Clone, Default)]
pub struct FlowMetadata {
    /// API name derived from the filename (e.g. `"My_Flow"` from `My_Flow.flow-meta.xml`).
    pub api_name: String,
    pub label: String,
    pub process_type: String,
    pub description: String,
    pub variables: Vec<FlowVariable>,
    pub decisions: usize,
    pub loops: usize,
    pub screens: usize,
    pub record_operations: Vec<FlowRecordOperation>,
    pub action_calls: Vec<FlowActionCall>,
}

#[derive(Debug, Clone, Default)]
pub struct FlowVariable {
    pub name: String,
    pub data_type: String,
    pub is_input: bool,
    pub is_output: bool,
}

#[derive(Debug, Clone)]
pub struct FlowRecordOperation {
    /// One of: `lookup`, `create`, `update`, `delete`.
    pub operation: String,
    pub object: String,
}

#[derive(Debug, Clone)]
pub struct FlowActionCall {
    pub name: String,
    pub action_type: String,
}

/// AI-generated documentation for a Salesforce Flow.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FlowDocumentation {
    pub api_name: String,
    pub label: String,
    pub summary: String,
    pub description: String,
    pub business_process: String,
    pub entry_criteria: String,
    pub key_decisions: Vec<String>,
    pub admin_notes: Vec<String>,
    pub relationships: Vec<String>,
}
