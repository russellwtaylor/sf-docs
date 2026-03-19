use std::collections::HashSet;
use std::path::PathBuf;

/// A discovered source file with its raw content (used for all file types).
#[derive(Debug, Clone)]
pub struct SourceFile {
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
    /// True when the declaration uses `interface` keyword instead of `class`.
    pub is_interface: bool,
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
    pub class_names: HashSet<String>,
    pub trigger_names: HashSet<String>,
    pub flow_names: HashSet<String>,
    pub validation_rule_names: HashSet<String>,
    pub object_names: HashSet<String>,
    pub lwc_names: HashSet<String>,
    pub flexipage_names: HashSet<String>,
    pub aura_names: HashSet<String>,
    pub custom_metadata_type_names: HashSet<String>,
    /// Maps interface name → Vec of class names that implement it.
    pub interface_implementors: std::collections::HashMap<String, Vec<String>>,
}

impl AllNames {
    /// Returns the union of all known entity names across all metadata types.
    pub fn all_known_names(&self) -> HashSet<&str> {
        self.class_names
            .iter()
            .chain(self.trigger_names.iter())
            .chain(self.flow_names.iter())
            .chain(self.validation_rule_names.iter())
            .chain(self.object_names.iter())
            .chain(self.lwc_names.iter())
            .chain(self.flexipage_names.iter())
            .chain(self.aura_names.iter())
            .map(|s| s.as_str())
            .collect()
    }
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

// ---------------------------------------------------------------------------
// Validation Rule types
// ---------------------------------------------------------------------------

/// Structural metadata extracted from a Salesforce Validation Rule XML file.
#[derive(Debug, Clone, Default)]
pub struct ValidationRuleMetadata {
    pub rule_name: String,
    pub object_name: String,
    pub active: bool,
    pub description: String,
    pub error_condition_formula: String,
    pub error_display_field: String,
    pub error_message: String,
}

// ---------------------------------------------------------------------------
// Object types
// ---------------------------------------------------------------------------

/// A single field on a Salesforce custom object.
#[derive(Debug, Clone, Default)]
pub struct ObjectField {
    pub api_name: String,
    pub label: String,
    pub field_type: String,
    pub description: String,
    pub help_text: String,
    /// For Lookup/MasterDetail fields: the target object API name.
    pub reference_to: String,
    pub required: bool,
}

/// Structural metadata extracted from a Salesforce Custom Object.
#[derive(Debug, Clone, Default)]
pub struct ObjectMetadata {
    pub object_name: String,
    pub label: String,
    pub description: String,
    pub fields: Vec<ObjectField>,
}

/// AI-generated documentation for a Salesforce Custom Object.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ObjectDocumentation {
    pub object_name: String,
    pub label: String,
    pub summary: String,
    pub description: String,
    pub purpose: String,
    pub key_fields: Vec<String>,
    pub relationships: Vec<String>,
    pub admin_notes: Vec<String>,
}

/// AI-generated documentation for a Salesforce Validation Rule.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ValidationRuleDocumentation {
    pub rule_name: String,
    pub object_name: String,
    pub summary: String,
    pub when_fires: String,
    pub what_protects: String,
    pub formula_explanation: String,
    pub edge_cases: Vec<String>,
    pub relationships: Vec<String>,
}

// ---------------------------------------------------------------------------
// LWC types
// ---------------------------------------------------------------------------

/// A single `@api`-decorated property or method on a Lightning Web Component.
#[derive(Debug, Clone, Default)]
pub struct LwcApiProp {
    pub name: String,
    /// `true` if this is an `@api` method rather than a property.
    pub is_method: bool,
}

/// Structural metadata extracted from a Lightning Web Component.
#[derive(Debug, Clone, Default)]
pub struct LwcMetadata {
    pub component_name: String,
    /// All `@api` properties and methods exposed by the component.
    pub api_props: Vec<LwcApiProp>,
    /// Slot names declared in the HTML template (`"default"` for anonymous slots).
    pub slots: Vec<String>,
    /// camelCase names of child `<c-*>` components referenced in the template.
    pub referenced_components: Vec<String>,
}

/// AI-generated documentation for a Lightning Web Component property or method.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LwcPropDocumentation {
    pub name: String,
    pub description: String,
}

/// AI-generated documentation for a Lightning Web Component.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LwcDocumentation {
    pub component_name: String,
    pub summary: String,
    pub description: String,
    pub api_props: Vec<LwcPropDocumentation>,
    pub usage_notes: Vec<String>,
    pub relationships: Vec<String>,
}

// ---------------------------------------------------------------------------
// FlexiPage types
// ---------------------------------------------------------------------------

/// Structural metadata extracted from a Salesforce FlexiPage XML file.
#[derive(Debug, Clone, Default)]
pub struct FlexiPageMetadata {
    pub api_name: String,
    pub label: String,
    pub page_type: String,
    /// SObject type for record pages; empty for other page types.
    pub sobject: String,
    pub description: String,
    /// LWC component names referenced in flexiPageRegions.
    pub component_names: Vec<String>,
    /// Flow API names referenced in action components.
    pub flow_names: Vec<String>,
}

/// AI-generated documentation for a Salesforce FlexiPage.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FlexiPageDocumentation {
    pub api_name: String,
    pub label: String,
    pub summary: String,
    pub description: String,
    /// Who/when this page is shown.
    pub usage_context: String,
    /// AI-described purpose of each key component.
    pub key_components: Vec<String>,
    pub relationships: Vec<String>,
}

// ---------------------------------------------------------------------------
// Custom Metadata Record types
// ---------------------------------------------------------------------------

/// A single custom metadata record parsed from a `.md-meta.xml` file.
#[derive(Debug, Clone, Default)]
pub struct CustomMetadataRecord {
    pub type_name: String,
    pub record_name: String,
    pub label: String,
    pub values: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Aura Component types
// ---------------------------------------------------------------------------

/// A single attribute declared on an Aura component.
#[derive(Debug, Clone, Default)]
pub struct AuraAttributeMetadata {
    pub name: String,
    pub attr_type: String,
    pub default: String,
    pub description: String,
}

/// Structural metadata extracted from an Aura component `.cmp` file.
#[derive(Debug, Clone, Default)]
pub struct AuraMetadata {
    pub component_name: String,
    pub attributes: Vec<AuraAttributeMetadata>,
    pub events_handled: Vec<String>,
    pub extends: Option<String>,
}

/// AI-generated documentation for a single Aura attribute.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuraAttributeDocumentation {
    pub name: String,
    pub description: String,
}

/// AI-generated documentation for an Aura component.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuraDocumentation {
    pub component_name: String,
    pub summary: String,
    pub description: String,
    pub attributes: Vec<AuraAttributeDocumentation>,
    pub usage_notes: Vec<String>,
    pub relationships: Vec<String>,
}
