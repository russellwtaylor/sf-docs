use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::types::{
    AllNames, AuraDocumentation, AuraMetadata, ClassDocumentation, ClassMetadata,
    CustomMetadataRecord, FlexiPageDocumentation, FlexiPageMetadata, FlowDocumentation,
    FlowMetadata, LwcDocumentation, LwcMetadata, ObjectDocumentation, ObjectMetadata,
    TriggerDocumentation, TriggerMetadata, ValidationRuleDocumentation, ValidationRuleMetadata,
};

pub struct RenderContext {
    pub metadata: ClassMetadata,
    pub documentation: ClassDocumentation,
    pub all_names: Arc<AllNames>,
    /// Relative path from the source directory to this file's parent directory.
    /// Used to group the index by namespace/folder (e.g. `"classes"`, `"classes/account"`).
    pub folder: String,
}

pub struct TriggerRenderContext {
    pub metadata: TriggerMetadata,
    pub documentation: TriggerDocumentation,
    pub all_names: Arc<AllNames>,
    /// Relative path from the source directory to this file's parent directory.
    pub folder: String,
}

pub struct FlowRenderContext {
    pub metadata: FlowMetadata,
    pub documentation: FlowDocumentation,
    pub all_names: Arc<AllNames>,
    pub folder: String,
}

pub struct ValidationRuleRenderContext {
    pub metadata: ValidationRuleMetadata,
    pub documentation: ValidationRuleDocumentation,
    pub all_names: Arc<AllNames>,
    /// Set to `object_name` for grouping by object in the index.
    pub folder: String,
}

pub struct ObjectRenderContext {
    pub metadata: ObjectMetadata,
    pub documentation: ObjectDocumentation,
    pub all_names: Arc<AllNames>,
    pub folder: String,
}

pub struct LwcRenderContext {
    pub metadata: LwcMetadata,
    pub documentation: LwcDocumentation,
    pub all_names: Arc<AllNames>,
    pub folder: String,
}

pub struct FlexiPageRenderContext {
    pub metadata: FlexiPageMetadata,
    pub documentation: FlexiPageDocumentation,
    pub all_names: Arc<AllNames>,
    pub folder: String,
}

pub struct CustomMetadataRenderContext {
    pub type_name: String,
    pub records: Vec<CustomMetadataRecord>,
}

pub struct AuraRenderContext {
    pub metadata: AuraMetadata,
    pub documentation: AuraDocumentation,
    pub all_names: Arc<AllNames>,
    pub folder: String,
}

/// Groups all documentation contexts for rendering, eliminating long parameter lists.
pub struct DocumentationBundle<'a> {
    pub classes: &'a [RenderContext],
    pub triggers: &'a [TriggerRenderContext],
    pub flows: &'a [FlowRenderContext],
    pub validation_rules: &'a [ValidationRuleRenderContext],
    pub objects: &'a [ObjectRenderContext],
    pub lwc: &'a [LwcRenderContext],
    pub flexipages: &'a [FlexiPageRenderContext],
    pub custom_metadata: &'a [CustomMetadataRenderContext],
    pub aura: &'a [AuraRenderContext],
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns the relative markdown link path from a page of `from_type` to a page named `name`.
///
/// `from_type` is one of `"class"`, `"trigger"`, `"flow"`, or `"validation_rule"`.
/// The function inspects `all_names` to determine which type `name` belongs to.
fn cross_link_md(name: &str, all_names: &AllNames, from_type: &str) -> String {
    let to_type = if all_names.class_names.contains(name) {
        "class"
    } else if all_names.trigger_names.contains(name) {
        "trigger"
    } else if all_names.flow_names.contains(name) {
        "flow"
    } else if all_names.validation_rule_names.contains(name) {
        "validation_rule"
    } else if all_names.object_names.contains(name) {
        "object"
    } else if all_names.lwc_names.contains(name) {
        "lwc"
    } else if all_names.flexipage_names.contains(name) {
        "flexipage"
    } else if all_names.aura_names.contains(name) {
        "aura"
    } else {
        // Unknown name — no link generated (caller filters via `known` set).
        return format!("{name}.md");
    };

    let type_dir = match to_type {
        "class" => "classes",
        "trigger" => "triggers",
        "flow" => "flows",
        "object" => "objects",
        "lwc" => "lwc",
        "flexipage" => "flexipages",
        "aura" => "aura",
        _ => "validation-rules",
    };

    if to_type == from_type {
        format!("{name}.md")
    } else {
        format!("../{type_dir}/{name}.md")
    }
}

pub fn render_class_page(ctx: &RenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    // Title + badges
    out.push_str(&format!("# {}\n\n", doc.class_name));
    out.push_str(&render_badges(meta));
    out.push('\n');

    if !ctx.metadata.tags.is_empty() {
        let tag_str: Vec<String> = ctx.metadata.tags.iter().map(|t| format!("`{t}`")).collect();
        out.push_str(&format!("**Tags:** {}\n\n", tag_str.join(", ")));
    }

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Table of contents
    out.push_str(&render_toc(doc));
    out.push('\n');

    // Description
    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    // Implemented By (for interfaces)
    if meta.is_interface {
        if let Some(implementors) = ctx.all_names.interface_implementors.get(&meta.class_name) {
            if !implementors.is_empty() {
                out.push_str("## Implemented By\n\n");
                for cls in implementors {
                    let link = cross_link_md(cls, &ctx.all_names, "class");
                    out.push_str(&format!("- [{cls}]({link})\n"));
                }
                out.push('\n');
            }
        }
    }

    // Properties
    if !doc.properties.is_empty() {
        out.push_str("## Properties\n\n");
        out.push_str("| Name | Type | Description |\n");
        out.push_str("|------|------|-------------|\n");
        for prop in &doc.properties {
            // Find type and static flag from metadata
            let prop_type = meta
                .properties
                .iter()
                .find(|p| p.name == prop.name)
                .map(|p| {
                    if p.is_static {
                        format!("static {}", p.property_type)
                    } else {
                        p.property_type.clone()
                    }
                })
                .unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                prop.name, prop_type, prop.description
            ));
        }
        out.push('\n');
    }

    // Methods
    if !doc.methods.is_empty() {
        out.push_str("## Methods\n\n");
        for method_doc in &doc.methods {
            // Find signature from metadata
            let sig = meta
                .methods
                .iter()
                .find(|m| m.name == method_doc.name)
                .map(|m| {
                    let params: Vec<String> = m
                        .params
                        .iter()
                        .map(|p| format!("{} {}", p.param_type, p.name))
                        .collect();
                    let static_kw = if m.is_static { "static " } else { "" };
                    format!(
                        "{} {}{}({}): {}",
                        m.access_modifier,
                        static_kw,
                        m.name,
                        params.join(", "),
                        m.return_type,
                    )
                })
                .unwrap_or_else(|| method_doc.name.clone());

            out.push_str(&format!("### `{}`\n\n", sig));
            out.push_str(&method_doc.description);
            out.push_str("\n\n");

            if !method_doc.params.is_empty() {
                out.push_str("**Parameters**\n\n");
                out.push_str("| Name | Description |\n");
                out.push_str("|------|-------------|\n");
                for param in &method_doc.params {
                    out.push_str(&format!("| `{}` | {} |\n", param.name, param.description));
                }
                out.push('\n');
            }

            if method_doc.returns != "void" && !method_doc.returns.is_empty() {
                out.push_str(&format!("**Returns:** {}\n\n", method_doc.returns));
            }

            if !method_doc.throws.is_empty() {
                out.push_str("**Throws**\n\n");
                for exc in &method_doc.throws {
                    out.push_str(&format!("- {}\n", exc));
                }
                out.push('\n');
            }
        }
    }

    // Usage examples
    if !doc.usage_examples.is_empty() {
        out.push_str("## Usage Examples\n\n");
        for example in &doc.usage_examples {
            out.push_str(example);
            out.push_str("\n\n");
        }
    }

    // See Also (cross-linked relationships)
    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            // Try to find a known class name in the relationship string
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "class");
                format!("[{}]({}) — {}", name, link, rel)
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {}\n", link));
        }
        out.push('\n');
    }

    out
}

pub fn render_trigger_page(ctx: &TriggerRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", doc.trigger_name));

    // Badges: trigger event list
    let events_str = meta
        .events
        .iter()
        .map(|e| format!("`{}`", e.as_str()))
        .collect::<Vec<_>>()
        .join(" · ");
    out.push_str(&format!(
        "`trigger` · `on {}` · {}\n\n",
        doc.sobject, events_str
    ));

    if !ctx.metadata.tags.is_empty() {
        let tag_str: Vec<String> = ctx.metadata.tags.iter().map(|t| format!("`{t}`")).collect();
        out.push_str(&format!("**Tags:** {}\n\n", tag_str.join(", ")));
    }

    out.push_str(&format!("{}\n\n", doc.summary));

    // ToC
    out.push_str("## Table of Contents\n\n");
    out.push_str("- [Description](#description)\n");
    if !doc.events.is_empty() {
        out.push_str("- [Event Handlers](#event-handlers)\n");
    }
    if !doc.handler_classes.is_empty() {
        out.push_str("- [Handler Classes](#handler-classes)\n");
    }
    if !doc.usage_notes.is_empty() {
        out.push_str("- [Usage Notes](#usage-notes)\n");
    }
    out.push('\n');

    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    if !doc.events.is_empty() {
        out.push_str("## Event Handlers\n\n");
        out.push_str("| Event | Description |\n");
        out.push_str("|-------|-------------|\n");
        for ev in &doc.events {
            out.push_str(&format!("| `{}` | {} |\n", ev.event, ev.description));
        }
        out.push('\n');
    }

    if !doc.handler_classes.is_empty() {
        out.push_str("## Handler Classes\n\n");
        for cls in &doc.handler_classes {
            if known.contains(cls.as_str()) {
                let link = cross_link_md(cls, &ctx.all_names, "trigger");
                out.push_str(&format!("- [{cls}]({link})\n"));
            } else {
                out.push_str(&format!("- `{cls}`\n"));
            }
        }
        out.push('\n');
    }

    if !doc.usage_notes.is_empty() {
        out.push_str("## Usage Notes\n\n");
        for note in &doc.usage_notes {
            out.push_str(&format!("- {note}\n"));
        }
        out.push('\n');
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "trigger");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_flow_page(ctx: &FlowRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    // Title + subtitle
    out.push_str(&format!("# {}\n\n", doc.label));
    out.push_str(&format!("`flow` · `{}`\n\n", meta.process_type));

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Table of Contents
    out.push_str("## Table of Contents\n\n");
    out.push_str("- [Description](#description)\n");
    out.push_str("- [Business Process](#business-process)\n");
    out.push_str("- [Entry Criteria](#entry-criteria)\n");
    if !meta.variables.is_empty() {
        out.push_str("- [Variables](#variables)\n");
    }
    if !meta.record_operations.is_empty() {
        out.push_str("- [Record Operations](#record-operations)\n");
    }
    if !meta.action_calls.is_empty() {
        out.push_str("- [Action Calls](#action-calls)\n");
    }
    if meta.decisions > 0 || meta.loops > 0 || meta.screens > 0 {
        out.push_str("- [Element Counts](#element-counts)\n");
    }
    if !doc.key_decisions.is_empty() {
        out.push_str("- [Key Decisions](#key-decisions)\n");
    }
    if !doc.admin_notes.is_empty() {
        out.push_str("- [Admin Notes](#admin-notes)\n");
    }
    out.push('\n');

    // Description
    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    // Business Process
    out.push_str("## Business Process\n\n");
    out.push_str(&doc.business_process);
    out.push_str("\n\n");

    // Entry Criteria
    out.push_str("## Entry Criteria\n\n");
    out.push_str(&doc.entry_criteria);
    out.push_str("\n\n");

    // Variables table
    if !meta.variables.is_empty() {
        out.push_str("## Variables\n\n");
        out.push_str("| Name | Type | Direction |\n");
        out.push_str("|------|------|-----------|\n");
        for v in &meta.variables {
            let direction = match (v.is_input, v.is_output) {
                (true, true) => "Input / Output",
                (true, false) => "Input",
                (false, true) => "Output",
                (false, false) => "Internal",
            };
            out.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                v.name, v.data_type, direction
            ));
        }
        out.push('\n');
    }

    // Record Operations table
    if !meta.record_operations.is_empty() {
        out.push_str("## Record Operations\n\n");
        out.push_str("| Operation | Object |\n");
        out.push_str("|-----------|--------|\n");
        for op in &meta.record_operations {
            out.push_str(&format!("| {} | `{}` |\n", op.operation, op.object));
        }
        out.push('\n');
    }

    // Action Calls table
    if !meta.action_calls.is_empty() {
        out.push_str("## Action Calls\n\n");
        out.push_str("| Action | Type |\n");
        out.push_str("|--------|------|\n");
        for action in &meta.action_calls {
            out.push_str(&format!("| `{}` | {} |\n", action.name, action.action_type));
        }
        out.push('\n');
    }

    // Element Counts
    if meta.decisions > 0 || meta.loops > 0 || meta.screens > 0 {
        out.push_str("## Element Counts\n\n");
        if meta.decisions > 0 {
            out.push_str(&format!("- Decisions: {}\n", meta.decisions));
        }
        if meta.loops > 0 {
            out.push_str(&format!("- Loops: {}\n", meta.loops));
        }
        if meta.screens > 0 {
            out.push_str(&format!("- Screens: {}\n", meta.screens));
        }
        out.push('\n');
    }

    // Key Decisions
    if !doc.key_decisions.is_empty() {
        out.push_str("## Key Decisions\n\n");
        for d in &doc.key_decisions {
            out.push_str(&format!("- {d}\n"));
        }
        out.push('\n');
    }

    // Admin Notes
    if !doc.admin_notes.is_empty() {
        out.push_str("## Admin Notes\n\n");
        for note in &doc.admin_notes {
            out.push_str(&format!("- {note}\n"));
        }
        out.push('\n');
    }

    // See Also (cross-linked relationships)
    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "flow");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_validation_rule_page(ctx: &ValidationRuleRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    // Title + subtitle
    out.push_str(&format!("# {}\n\n", meta.rule_name));
    out.push_str(&format!(
        "`validation-rule` · `on {}` · {}\n\n",
        meta.object_name,
        if meta.active {
            "`active`"
        } else {
            "`inactive`"
        }
    ));

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Table of Contents
    out.push_str("## Table of Contents\n\n");
    out.push_str("- [When It Fires](#when-it-fires)\n");
    out.push_str("- [What It Protects](#what-it-protects)\n");
    out.push_str("- [Error Condition Formula](#error-condition-formula)\n");
    out.push_str("- [Formula Explanation](#formula-explanation)\n");
    out.push_str("- [Error Message](#error-message)\n");
    if !doc.edge_cases.is_empty() {
        out.push_str("- [Edge Cases](#edge-cases)\n");
    }
    out.push('\n');

    // When It Fires
    out.push_str("## When It Fires\n\n");
    out.push_str(&doc.when_fires);
    out.push_str("\n\n");

    // What It Protects
    out.push_str("## What It Protects\n\n");
    out.push_str(&doc.what_protects);
    out.push_str("\n\n");

    // Error Condition Formula
    out.push_str("## Error Condition Formula\n\n");
    out.push_str("```\n");
    out.push_str(&meta.error_condition_formula);
    out.push_str("\n```\n\n");

    // Formula Explanation
    out.push_str("## Formula Explanation\n\n");
    out.push_str(&doc.formula_explanation);
    out.push_str("\n\n");

    // Error Message
    out.push_str("## Error Message\n\n");
    out.push_str(&format!("> {}\n\n", meta.error_message));
    if !meta.error_display_field.is_empty() {
        out.push_str(&format!(
            "**Displayed on field:** `{}`\n\n",
            meta.error_display_field
        ));
    }

    // Edge Cases
    if !doc.edge_cases.is_empty() {
        out.push_str("## Edge Cases\n\n");
        for case in &doc.edge_cases {
            out.push_str(&format!("- {case}\n"));
        }
        out.push('\n');
    }

    // See Also (cross-linked relationships)
    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "validation_rule");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_object_page(ctx: &ObjectRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    // Title + subtitle
    let label = if doc.label.is_empty() {
        meta.object_name.as_str()
    } else {
        doc.label.as_str()
    };
    out.push_str(&format!("# {label}\n\n"));
    out.push_str(&format!("`object` · `{}`\n\n", meta.object_name));

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Purpose
    if !doc.purpose.is_empty() {
        out.push_str("## Purpose\n\n");
        out.push_str(&doc.purpose);
        out.push_str("\n\n");
    }

    // Description
    if !doc.description.is_empty() {
        out.push_str("## Description\n\n");
        out.push_str(&doc.description);
        out.push_str("\n\n");
    }

    // Fields table
    if !meta.fields.is_empty() {
        out.push_str("## Fields\n\n");
        out.push_str("| API Name | Type | Label | Required |\n");
        out.push_str("|----------|------|-------|----------|\n");
        for field in &meta.fields {
            let type_str = if field.field_type == "Lookup" || field.field_type == "MasterDetail" {
                if field.reference_to.is_empty() {
                    field.field_type.clone()
                } else {
                    format!("{} → `{}`", field.field_type, field.reference_to)
                }
            } else {
                field.field_type.clone()
            };
            out.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                field.api_name,
                type_str,
                field.label,
                if field.required { "Yes" } else { "No" },
            ));
        }
        out.push('\n');
    }

    // Key Fields (AI-curated highlights)
    if !doc.key_fields.is_empty() {
        out.push_str("## Key Fields\n\n");
        for f in &doc.key_fields {
            out.push_str(&format!("- {f}\n"));
        }
        out.push('\n');
    }

    // Admin Notes
    if !doc.admin_notes.is_empty() {
        out.push_str("## Admin Notes\n\n");
        for note in &doc.admin_notes {
            out.push_str(&format!("- {note}\n"));
        }
        out.push('\n');
    }

    // See Also (cross-linked relationships)
    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "object");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_lwc_page(ctx: &LwcRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    // Title + subtitle
    out.push_str(&format!("# {}\n\n", meta.component_name));
    out.push_str("`lwc` · `Lightning Web Component`\n\n");

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Table of Contents
    out.push_str("## Table of Contents\n\n");
    out.push_str("- [Description](#description)\n");
    if !doc.api_props.is_empty() {
        out.push_str("- [Public API](#public-api)\n");
    }
    if !meta.slots.is_empty() {
        out.push_str("- [Slots](#slots)\n");
    }
    if !doc.usage_notes.is_empty() {
        out.push_str("- [Usage Notes](#usage-notes)\n");
    }
    out.push('\n');

    // Description
    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    // Public API table
    if !doc.api_props.is_empty() {
        out.push_str("## Public API\n\n");
        out.push_str("| Name | Kind | Description |\n");
        out.push_str("|------|------|-------------|\n");
        for prop_doc in &doc.api_props {
            let kind = meta
                .api_props
                .iter()
                .find(|p| p.name == prop_doc.name)
                .map(|p| if p.is_method { "method" } else { "property" })
                .unwrap_or("property");
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                prop_doc.name, kind, prop_doc.description
            ));
        }
        out.push('\n');
    }

    // Slots
    if !meta.slots.is_empty() {
        out.push_str("## Slots\n\n");
        out.push_str("| Slot | Description |\n");
        out.push_str("|------|-------------|\n");
        for slot in &meta.slots {
            let label = if slot == "default" {
                "_(default)_".to_string()
            } else {
                format!("`{slot}`")
            };
            out.push_str(&format!("| {label} | |\n"));
        }
        out.push('\n');
    }

    // Usage Notes
    if !doc.usage_notes.is_empty() {
        out.push_str("## Usage Notes\n\n");
        for note in &doc.usage_notes {
            out.push_str(&format!("- {note}\n"));
        }
        out.push('\n');
    }

    // See Also (cross-linked relationships)
    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "lwc");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_index(bundle: &DocumentationBundle) -> String {
    let class_contexts = bundle.classes;
    let trigger_contexts = bundle.triggers;
    let flow_contexts = bundle.flows;
    let validation_rule_contexts = bundle.validation_rules;
    let object_contexts = bundle.objects;
    let lwc_contexts = bundle.lwc;
    let flexipage_contexts = bundle.flexipages;
    let custom_metadata_contexts = bundle.custom_metadata;
    let aura_contexts = bundle.aura;
    let mut out = String::new();
    out.push_str("# Apex Documentation Index\n\n");
    out.push_str(&format!(
        "Generated documentation for {} class(es), {} trigger(s), {} flow(s), {} validation rule(s), {} object(s), {} LWC component(s), {} Lightning page(s), {} Custom Metadata type(s), and {} Aura component(s).\n\n",
        class_contexts.iter().filter(|c| !c.metadata.is_interface).count(),
        trigger_contexts.len(),
        flow_contexts.len(),
        validation_rule_contexts.len(),
        object_contexts.len(),
        lwc_contexts.len(),
        flexipage_contexts.len(),
        custom_metadata_contexts.len(),
        aura_contexts.len(),
    ));

    // Separate classes and interfaces
    let (interface_contexts, regular_class_contexts): (Vec<_>, Vec<_>) =
        class_contexts.iter().partition(|c| c.metadata.is_interface);

    // Group regular classes by folder, sorted alphabetically within each group.
    let mut class_by_folder: BTreeMap<&str, Vec<&RenderContext>> = BTreeMap::new();
    for ctx in &regular_class_contexts {
        class_by_folder
            .entry(ctx.folder.as_str())
            .or_default()
            .push(ctx);
    }
    for group in class_by_folder.values_mut() {
        group.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));
    }

    if !regular_class_contexts.is_empty() {
        out.push_str("## Classes\n\n");
        let multi_class_folder = class_by_folder.len() > 1;
        for (folder, classes) in &class_by_folder {
            if multi_class_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                out.push_str(&format!("### {label}\n\n"));
            }
            out.push_str("| Class | Summary |\n");
            out.push_str("|-------|---------|\n");
            for ctx in classes {
                out.push_str(&format!(
                    "| [{}](classes/{}.md) | {} |\n",
                    ctx.documentation.class_name,
                    ctx.documentation.class_name,
                    ctx.documentation.summary
                ));
            }
            out.push('\n');
        }
    }

    // Interfaces section
    if !interface_contexts.is_empty() {
        let mut iface_sorted: Vec<&&RenderContext> = interface_contexts.iter().collect();
        iface_sorted.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));
        out.push_str("## Interfaces\n\n");
        out.push_str("| Interface | Summary |\n");
        out.push_str("|-----------|--------|\n");
        for ctx in iface_sorted {
            out.push_str(&format!(
                "| [{}](classes/{}.md) | {} |\n",
                ctx.documentation.class_name,
                ctx.documentation.class_name,
                ctx.documentation.summary
            ));
        }
        out.push('\n');
    }

    if !trigger_contexts.is_empty() {
        // Group triggers by folder, sorted within each group.
        let mut trigger_by_folder: BTreeMap<&str, Vec<&TriggerRenderContext>> = BTreeMap::new();
        for ctx in trigger_contexts {
            trigger_by_folder
                .entry(ctx.folder.as_str())
                .or_default()
                .push(ctx);
        }
        for group in trigger_by_folder.values_mut() {
            group.sort_by(|a, b| {
                a.documentation
                    .trigger_name
                    .cmp(&b.documentation.trigger_name)
            });
        }

        out.push_str("## Triggers\n\n");
        let multi_trigger_folder = trigger_by_folder.len() > 1;
        for (folder, triggers) in &trigger_by_folder {
            if multi_trigger_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                out.push_str(&format!("### {label}\n\n"));
            }
            out.push_str("| Trigger | SObject | Summary |\n");
            out.push_str("|---------|---------|--------|\n");
            for ctx in triggers {
                out.push_str(&format!(
                    "| [{}](triggers/{}.md) | `{}` | {} |\n",
                    ctx.documentation.trigger_name,
                    ctx.documentation.trigger_name,
                    ctx.documentation.sobject,
                    ctx.documentation.summary,
                ));
            }
            out.push('\n');
        }
    }

    if !flow_contexts.is_empty() {
        // Group flows by folder, sorted within each group.
        let mut flow_by_folder: BTreeMap<&str, Vec<&FlowRenderContext>> = BTreeMap::new();
        for ctx in flow_contexts {
            flow_by_folder
                .entry(ctx.folder.as_str())
                .or_default()
                .push(ctx);
        }
        for group in flow_by_folder.values_mut() {
            group.sort_by(|a, b| a.documentation.label.cmp(&b.documentation.label));
        }

        out.push_str("## Flows\n\n");
        let multi_flow_folder = flow_by_folder.len() > 1;
        for (folder, flows) in &flow_by_folder {
            if multi_flow_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                out.push_str(&format!("### {label}\n\n"));
            }
            out.push_str("| Flow | Process Type | Summary |\n");
            out.push_str("|------|--------------|--------|\n");
            for ctx in flows {
                out.push_str(&format!(
                    "| [{}](flows/{}.md) | `{}` | {} |\n",
                    ctx.documentation.label,
                    ctx.metadata.api_name,
                    ctx.metadata.process_type,
                    ctx.documentation.summary,
                ));
            }
            out.push('\n');
        }
    }

    if !validation_rule_contexts.is_empty() {
        // Group validation rules by object_name (stored in folder), sorted within each group.
        let mut vr_by_folder: BTreeMap<&str, Vec<&ValidationRuleRenderContext>> = BTreeMap::new();
        for ctx in validation_rule_contexts {
            vr_by_folder
                .entry(ctx.folder.as_str())
                .or_default()
                .push(ctx);
        }
        for group in vr_by_folder.values_mut() {
            group.sort_by(|a, b| a.metadata.rule_name.cmp(&b.metadata.rule_name));
        }

        out.push_str("## Validation Rules\n\n");
        let multi_vr_folder = vr_by_folder.len() > 1;
        for (folder, rules) in &vr_by_folder {
            if multi_vr_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                out.push_str(&format!("### {label}\n\n"));
            }
            out.push_str("| Rule | Object | Status | Summary |\n");
            out.push_str("|------|--------|--------|---------|\n");
            for ctx in rules {
                let status = if ctx.metadata.active {
                    "active"
                } else {
                    "inactive"
                };
                out.push_str(&format!(
                    "| [{}](validation-rules/{}.md) | `{}` | {} | {} |\n",
                    ctx.metadata.rule_name,
                    sanitize_filename(&ctx.metadata.rule_name),
                    ctx.metadata.object_name,
                    status,
                    ctx.documentation.summary,
                ));
            }
            out.push('\n');
        }
    }

    if !object_contexts.is_empty() {
        let mut obj_sorted: Vec<&ObjectRenderContext> = object_contexts.iter().collect();
        obj_sorted.sort_by(|a, b| a.metadata.object_name.cmp(&b.metadata.object_name));

        out.push_str("## Objects\n\n");
        out.push_str("| Object | Label | Fields | Summary |\n");
        out.push_str("|--------|-------|--------|---------|\n");
        for ctx in obj_sorted {
            let label = if ctx.documentation.label.is_empty() {
                ctx.metadata.object_name.as_str()
            } else {
                ctx.documentation.label.as_str()
            };
            out.push_str(&format!(
                "| [{}](objects/{}.md) | {} | {} | {} |\n",
                ctx.metadata.object_name,
                sanitize_filename(&ctx.metadata.object_name),
                label,
                ctx.metadata.fields.len(),
                ctx.documentation.summary,
            ));
        }
        out.push('\n');
    }

    if !lwc_contexts.is_empty() {
        let mut lwc_by_folder: BTreeMap<&str, Vec<&LwcRenderContext>> = BTreeMap::new();
        for ctx in lwc_contexts {
            lwc_by_folder
                .entry(ctx.folder.as_str())
                .or_default()
                .push(ctx);
        }
        for group in lwc_by_folder.values_mut() {
            group.sort_by(|a, b| a.metadata.component_name.cmp(&b.metadata.component_name));
        }

        out.push_str("## Lightning Web Components\n\n");
        let multi_lwc_folder = lwc_by_folder.len() > 1;
        for (folder, components) in &lwc_by_folder {
            if multi_lwc_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                out.push_str(&format!("### {label}\n\n"));
            }
            out.push_str("| Component | @api Props | Summary |\n");
            out.push_str("|-----------|------------|---------|\n");
            for ctx in components {
                out.push_str(&format!(
                    "| [{}](lwc/{}.md) | {} | {} |\n",
                    ctx.metadata.component_name,
                    sanitize_filename(&ctx.metadata.component_name),
                    ctx.metadata.api_props.len(),
                    ctx.documentation.summary,
                ));
            }
            out.push('\n');
        }
    }

    // FlexiPages section
    if !flexipage_contexts.is_empty() {
        let mut fp_sorted: Vec<&FlexiPageRenderContext> = flexipage_contexts.iter().collect();
        fp_sorted.sort_by(|a, b| a.metadata.api_name.cmp(&b.metadata.api_name));
        out.push_str("## Lightning Pages\n\n");
        out.push_str("| Page | Type | Summary |\n");
        out.push_str("|------|------|---------|\n");
        for ctx in fp_sorted {
            let label = if ctx.documentation.label.is_empty() {
                ctx.metadata.api_name.as_str()
            } else {
                ctx.documentation.label.as_str()
            };
            out.push_str(&format!(
                "| [{}](flexipages/{}.md) | `{}` | {} |\n",
                label,
                sanitize_filename(&ctx.metadata.api_name),
                ctx.metadata.page_type,
                ctx.documentation.summary,
            ));
        }
        out.push('\n');
    }

    // Custom Metadata Types section
    if !custom_metadata_contexts.is_empty() {
        let mut cmt_sorted: Vec<&CustomMetadataRenderContext> =
            custom_metadata_contexts.iter().collect();
        cmt_sorted.sort_by(|a, b| a.type_name.cmp(&b.type_name));
        out.push_str("## Custom Metadata Types\n\n");
        out.push_str("| Type | Records |\n");
        out.push_str("|------|---------|\n");
        for ctx in cmt_sorted {
            out.push_str(&format!(
                "| [{}](custom-metadata/{}.md) | {} |\n",
                ctx.type_name,
                sanitize_filename(&ctx.type_name),
                ctx.records.len(),
            ));
        }
        out.push('\n');
    }

    // Aura Components section
    if !aura_contexts.is_empty() {
        let mut aura_sorted: Vec<&AuraRenderContext> = aura_contexts.iter().collect();
        aura_sorted.sort_by(|a, b| a.metadata.component_name.cmp(&b.metadata.component_name));
        out.push_str("## Aura Components\n\n");
        out.push_str("| Component | Attributes | Summary |\n");
        out.push_str("|-----------|------------|---------|\n");
        for ctx in aura_sorted {
            out.push_str(&format!(
                "| [{}](aura/{}.md) | {} | {} |\n",
                ctx.metadata.component_name,
                sanitize_filename(&ctx.metadata.component_name),
                ctx.metadata.attributes.len(),
                ctx.documentation.summary,
            ));
        }
        out.push('\n');
    }

    out
}

pub fn render_flexipage_page(ctx: &FlexiPageRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    let label = if doc.label.is_empty() {
        meta.api_name.as_str()
    } else {
        doc.label.as_str()
    };

    out.push_str(&format!("# {label}\n\n"));
    out.push_str(&format!(
        "`lightning-page` · `{}` · `{}`\n\n",
        meta.page_type,
        if meta.sobject.is_empty() {
            "—".to_string()
        } else {
            meta.sobject.clone()
        }
    ));

    out.push_str(&format!("{}\n\n", doc.summary));

    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    if !doc.usage_context.is_empty() {
        out.push_str("## Usage Context\n\n");
        out.push_str(&doc.usage_context);
        out.push_str("\n\n");
    }

    if !meta.component_names.is_empty() {
        out.push_str("## Components\n\n");
        out.push_str("| Component |\n");
        out.push_str("|-----------|\n");
        for comp in &meta.component_names {
            out.push_str(&format!("| `{}` |\n", comp));
        }
        out.push('\n');
    }

    if !doc.key_components.is_empty() {
        out.push_str("## Key Components\n\n");
        for comp_desc in &doc.key_components {
            out.push_str(&format!("- {}\n", comp_desc));
        }
        out.push('\n');
    }

    if !meta.flow_names.is_empty() {
        out.push_str("## Referenced Flows\n\n");
        for flow in &meta.flow_names {
            out.push_str(&format!("- `{}`\n", flow));
        }
        out.push('\n');
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "flexipage");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn render_custom_metadata_page(ctx: &CustomMetadataRenderContext) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", ctx.type_name));
    out.push_str("`custom-metadata-type`\n\n");
    out.push_str(&format!("{} record(s)\n\n", ctx.records.len()));

    if ctx.records.is_empty() {
        out.push_str("_No records found._\n");
        return out;
    }

    // Collect all unique field names across all records for table headers
    let mut all_fields: Vec<String> = Vec::new();
    for rec in &ctx.records {
        for (field, _) in &rec.values {
            if !all_fields.contains(field) {
                all_fields.push(field.clone());
            }
        }
    }
    all_fields.sort();

    // Records table
    out.push_str("## Records\n\n");

    // Build header
    let mut header = "| Record | Label".to_string();
    for f in &all_fields {
        header.push_str(&format!(" | {}", f));
    }
    header.push_str(" |\n");
    out.push_str(&header);

    let mut separator = "|--------|------".to_string();
    for _ in &all_fields {
        separator.push_str("|------");
    }
    separator.push_str("|\n");
    out.push_str(&separator);

    let mut sorted_records: Vec<&CustomMetadataRecord> = ctx.records.iter().collect();
    sorted_records.sort_by(|a, b| a.record_name.cmp(&b.record_name));

    for rec in sorted_records {
        let mut row = format!("| `{}` | {} ", rec.record_name, rec.label);
        for field_name in &all_fields {
            let val = rec
                .values
                .iter()
                .find(|(f, _)| f == field_name)
                .map(|(_, v)| v.as_str())
                .unwrap_or("—");
            row.push_str(&format!("| {} ", val));
        }
        row.push_str("|\n");
        out.push_str(&row);
    }
    out.push('\n');

    out
}

pub fn render_aura_page(ctx: &AuraRenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known = ctx.all_names.all_known_names();

    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", meta.component_name));
    let mut badges = vec!["`aura`".to_string(), "`Aura Component`".to_string()];
    if let Some(ref ext) = meta.extends {
        badges.push(format!("extends `{}`", ext));
    }
    out.push_str(&format!("{}\n\n", badges.join(" · ")));

    out.push_str(&format!("{}\n\n", doc.summary));

    out.push_str("## Table of Contents\n\n");
    out.push_str("- [Description](#description)\n");
    if !doc.attributes.is_empty() {
        out.push_str("- [Attributes](#attributes)\n");
    }
    if !meta.events_handled.is_empty() {
        out.push_str("- [Events](#events)\n");
    }
    if !doc.usage_notes.is_empty() {
        out.push_str("- [Usage Notes](#usage-notes)\n");
    }
    out.push('\n');

    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

    if !doc.attributes.is_empty() {
        out.push_str("## Attributes\n\n");
        out.push_str("| Name | Type | Default | Description |\n");
        out.push_str("|------|------|---------|-------------|\n");
        for attr_doc in &doc.attributes {
            let meta_attr = meta.attributes.iter().find(|a| a.name == attr_doc.name);
            let attr_type = meta_attr.map(|a| a.attr_type.as_str()).unwrap_or("—");
            let default = meta_attr
                .map(|a| {
                    if a.default.is_empty() {
                        "—".to_string()
                    } else {
                        format!("`{}`", a.default)
                    }
                })
                .unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                attr_doc.name, attr_type, default, attr_doc.description
            ));
        }
        out.push('\n');
    }

    if !meta.events_handled.is_empty() {
        out.push_str("## Events\n\n");
        for event in &meta.events_handled {
            out.push_str(&format!("- `{}`\n", event));
        }
        out.push('\n');
    }

    if !doc.usage_notes.is_empty() {
        out.push_str("## Usage Notes\n\n");
        for note in &doc.usage_notes {
            out.push_str(&format!("- {note}\n"));
        }
        out.push('\n');
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            known.iter().find(|&&name| rel.contains(name)).map(|&name| {
                let link = cross_link_md(name, &ctx.all_names, "aura");
                format!("[{name}]({link}) — {rel}")
            })
        })
        .collect();

    if !see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out
}

pub fn write_output(
    output_dir: &Path,
    bundle: &DocumentationBundle,
) -> Result<()> {
    let class_contexts = bundle.classes;
    let trigger_contexts = bundle.triggers;
    let flow_contexts = bundle.flows;
    let validation_rule_contexts = bundle.validation_rules;
    let object_contexts = bundle.objects;
    let lwc_contexts = bundle.lwc;
    let flexipage_contexts = bundle.flexipages;
    let custom_metadata_contexts = bundle.custom_metadata;
    let aura_contexts = bundle.aura;

    let classes_dir = output_dir.join("classes");
    let triggers_dir = output_dir.join("triggers");
    let flows_dir = output_dir.join("flows");
    let vr_dir = output_dir.join("validation-rules");
    let objects_dir = output_dir.join("objects");
    let lwc_dir = output_dir.join("lwc");
    let flexipages_dir = output_dir.join("flexipages");
    let custom_metadata_dir = output_dir.join("custom-metadata");
    let aura_dir = output_dir.join("aura");

    std::fs::create_dir_all(output_dir)?;
    if !class_contexts.is_empty() {
        std::fs::create_dir_all(&classes_dir)?;
    }
    if !trigger_contexts.is_empty() {
        std::fs::create_dir_all(&triggers_dir)?;
    }
    if !flow_contexts.is_empty() {
        std::fs::create_dir_all(&flows_dir)?;
    }
    if !validation_rule_contexts.is_empty() {
        std::fs::create_dir_all(&vr_dir)?;
    }
    if !object_contexts.is_empty() {
        std::fs::create_dir_all(&objects_dir)?;
    }
    if !lwc_contexts.is_empty() {
        std::fs::create_dir_all(&lwc_dir)?;
    }
    if !flexipage_contexts.is_empty() {
        std::fs::create_dir_all(&flexipages_dir)?;
    }
    if !custom_metadata_contexts.is_empty() {
        std::fs::create_dir_all(&custom_metadata_dir)?;
    }
    if !aura_contexts.is_empty() {
        std::fs::create_dir_all(&aura_dir)?;
    }

    for ctx in class_contexts {
        let page = render_class_page(ctx);
        std::fs::write(
            classes_dir.join(format!(
                "{}.md",
                sanitize_filename(&ctx.metadata.class_name)
            )),
            page,
        )?;
    }

    for ctx in trigger_contexts {
        let page = render_trigger_page(ctx);
        std::fs::write(
            triggers_dir.join(format!(
                "{}.md",
                sanitize_filename(&ctx.metadata.trigger_name)
            )),
            page,
        )?;
    }

    for ctx in flow_contexts {
        let page = render_flow_page(ctx);
        std::fs::write(
            flows_dir.join(format!("{}.md", sanitize_filename(&ctx.metadata.api_name))),
            page,
        )?;
    }

    for ctx in validation_rule_contexts {
        let page = render_validation_rule_page(ctx);
        std::fs::write(
            vr_dir.join(format!("{}.md", sanitize_filename(&ctx.metadata.rule_name))),
            page,
        )?;
    }

    for ctx in object_contexts {
        let page = render_object_page(ctx);
        std::fs::write(
            objects_dir.join(format!(
                "{}.md",
                sanitize_filename(&ctx.metadata.object_name)
            )),
            page,
        )?;
    }

    for ctx in lwc_contexts {
        let page = render_lwc_page(ctx);
        std::fs::write(
            lwc_dir.join(format!(
                "{}.md",
                sanitize_filename(&ctx.metadata.component_name)
            )),
            page,
        )?;
    }

    for ctx in flexipage_contexts {
        let page = render_flexipage_page(ctx);
        std::fs::write(
            flexipages_dir.join(format!("{}.md", sanitize_filename(&ctx.metadata.api_name))),
            page,
        )?;
    }

    for ctx in custom_metadata_contexts {
        let page = render_custom_metadata_page(ctx);
        std::fs::write(
            custom_metadata_dir.join(format!("{}.md", sanitize_filename(&ctx.type_name))),
            page,
        )?;
    }

    for ctx in aura_contexts {
        let page = render_aura_page(ctx);
        std::fs::write(
            aura_dir.join(format!(
                "{}.md",
                sanitize_filename(&ctx.metadata.component_name)
            )),
            page,
        )?;
    }

    let index = render_index(bundle);
    std::fs::write(output_dir.join("index.md"), index)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strips any path separators or traversal components from a name used as
/// an output filename. Only keeps `[a-zA-Z0-9_.-]` characters.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect()
}

fn render_badges(meta: &ClassMetadata) -> String {
    let mut badges = vec![format!("`{}`", meta.access_modifier)];
    if meta.is_interface {
        badges.push("`interface`".to_string());
    }
    if meta.is_abstract {
        badges.push("`abstract`".to_string());
    }
    if meta.is_virtual {
        badges.push("`virtual`".to_string());
    }
    if let Some(ref ext) = meta.extends {
        badges.push(format!("extends `{}`", ext));
    }
    if !meta.implements.is_empty() {
        badges.push(format!("implements `{}`", meta.implements.join("`, `")));
    }
    format!("{}\n", badges.join(" · "))
}

fn render_toc(doc: &ClassDocumentation) -> String {
    let mut toc = String::from("## Table of Contents\n\n");
    toc.push_str("- [Description](#description)\n");
    if !doc.properties.is_empty() {
        toc.push_str("- [Properties](#properties)\n");
    }
    if !doc.methods.is_empty() {
        toc.push_str("- [Methods](#methods)\n");
        for m in &doc.methods {
            let anchor = m.name.to_lowercase();
            toc.push_str(&format!("  - [`{}`](#{anchor})\n", m.name));
        }
    }
    if !doc.usage_examples.is_empty() {
        toc.push_str("- [Usage Examples](#usage-examples)\n");
    }
    toc
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AllNames, ClassDocumentation, ClassMetadata, MethodDocumentation, ParamDocumentation,
        PropertyDocumentation,
    };
    use std::collections::HashSet;
    use std::sync::Arc;

    fn sample_context() -> RenderContext {
        RenderContext {
            metadata: ClassMetadata {
                class_name: "AccountService".to_string(),
                access_modifier: "public".to_string(),
                is_abstract: false,
                is_virtual: false,
                is_interface: false,
                extends: Some("BaseService".to_string()),
                implements: vec!["Queueable".to_string()],
                methods: vec![crate::types::MethodMetadata {
                    name: "processAccounts".to_string(),
                    access_modifier: "public".to_string(),
                    return_type: "void".to_string(),
                    is_static: false,
                    params: vec![crate::types::ParamMetadata {
                        param_type: "List<Account>".to_string(),
                        name: "accounts".to_string(),
                    }],
                }],
                properties: vec![crate::types::PropertyMetadata {
                    name: "repo".to_string(),
                    access_modifier: "private".to_string(),
                    property_type: "AccountRepository".to_string(),
                    is_static: false,
                }],
                existing_comments: vec![],
                references: vec!["AccountRepository".to_string()],
                tags: vec![],
            },
            documentation: ClassDocumentation {
                class_name: "AccountService".to_string(),
                summary: "Handles account processing operations.".to_string(),
                description: "A service class that processes Salesforce Account records.".to_string(),
                methods: vec![MethodDocumentation {
                    name: "processAccounts".to_string(),
                    description: "Processes a list of accounts.".to_string(),
                    params: vec![ParamDocumentation {
                        name: "accounts".to_string(),
                        description: "The accounts to process.".to_string(),
                    }],
                    returns: "void".to_string(),
                    throws: vec![],
                }],
                properties: vec![PropertyDocumentation {
                    name: "repo".to_string(),
                    description: "Repository for account data access.".to_string(),
                }],
                usage_examples: vec!["```apex\nAccountService svc = new AccountService();\nsvc.processAccounts(accounts);\n```".to_string()],
                relationships: vec!["AccountRepository is used for data access".to_string()],
            },
            all_names: Arc::new(AllNames {
                class_names: ["AccountService".to_string(), "AccountRepository".to_string()]
                    .into_iter()
                    .collect(),
                trigger_names: HashSet::new(),
                flow_names: HashSet::new(),
                validation_rule_names: HashSet::new(),
                object_names: HashSet::new(),
                lwc_names: HashSet::new(),
                flexipage_names: HashSet::new(),
                aura_names: HashSet::new(),
                custom_metadata_type_names: HashSet::new(),
                interface_implementors: std::collections::HashMap::new(),
            }),
            folder: "classes".to_string(),
        }
    }

    #[test]
    fn class_page_contains_title() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("# AccountService"));
    }

    #[test]
    fn class_page_contains_method_section() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("processAccounts"));
        assert!(page.contains("## Methods"));
    }

    #[test]
    fn class_page_contains_properties_table() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("## Properties"));
        assert!(page.contains("repo"));
        assert!(page.contains("AccountRepository"));
    }

    #[test]
    fn class_page_has_see_also_with_link() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("## See Also"));
        // AccountRepository is also a class, so from a class page the link is same-dir
        assert!(page.contains("[AccountRepository](AccountRepository.md)"));
    }

    #[test]
    fn class_page_badges_include_extends() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("extends `BaseService`"));
    }

    #[test]
    fn index_contains_all_classes() {
        let ctx = sample_context();
        let bundle = DocumentationBundle {
            classes: &[ctx],
            triggers: &[],
            flows: &[],
            validation_rules: &[],
            objects: &[],
            lwc: &[],
            flexipages: &[],
            custom_metadata: &[],
            aura: &[],
        };
        let index = render_index(&bundle);
        assert!(index.contains("# Apex Documentation Index"));
        assert!(index.contains("[AccountService](classes/AccountService.md)"));
        assert!(index.contains("Handles account processing operations."));
    }

    #[test]
    fn static_method_shows_static_keyword() {
        let mut ctx = sample_context();
        ctx.metadata.methods[0].is_static = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("static processAccounts"));
    }

    #[test]
    fn static_property_shows_static_keyword() {
        let mut ctx = sample_context();
        ctx.metadata.properties[0].is_static = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("static AccountRepository"));
    }

    #[test]
    fn write_output_creates_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = sample_context();
        let bundle = DocumentationBundle {
            classes: &[ctx],
            triggers: &[],
            flows: &[],
            validation_rules: &[],
            objects: &[],
            lwc: &[],
            flexipages: &[],
            custom_metadata: &[],
            aura: &[],
        };
        write_output(tmp.path(), &bundle).unwrap();
        assert!(tmp.path().join("classes/AccountService.md").exists());
        assert!(tmp.path().join("index.md").exists());
    }

    // -----------------------------------------------------------------------
    // Edge cases & additional coverage
    // -----------------------------------------------------------------------

    #[test]
    fn class_page_with_no_methods_skips_methods_section() {
        let mut ctx = sample_context();
        ctx.metadata.methods.clear();
        ctx.documentation.methods.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Methods"));
    }

    #[test]
    fn class_page_with_no_properties_skips_properties_section() {
        let mut ctx = sample_context();
        ctx.metadata.properties.clear();
        ctx.documentation.properties.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Properties"));
    }

    #[test]
    fn class_page_with_no_usage_examples_skips_section() {
        let mut ctx = sample_context();
        ctx.documentation.usage_examples.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Usage Examples"));
    }

    #[test]
    fn class_page_interface_badge() {
        let mut ctx = sample_context();
        ctx.metadata.is_interface = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("interface"), "interface badge missing");
    }

    #[test]
    fn class_page_abstract_badge() {
        let mut ctx = sample_context();
        ctx.metadata.is_abstract = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("abstract"));
    }

    #[test]
    fn class_page_implements_badge() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("Queueable"));
    }

    #[test]
    fn index_with_empty_bundle() {
        let bundle = DocumentationBundle {
            classes: &[],
            triggers: &[],
            flows: &[],
            validation_rules: &[],
            objects: &[],
            lwc: &[],
            flexipages: &[],
            custom_metadata: &[],
            aura: &[],
        };
        let index = render_index(&bundle);
        assert!(index.contains("# Apex Documentation Index"));
    }

    #[test]
    fn sanitize_filename_removes_special_chars() {
        assert_eq!(sanitize_filename("Hello World"), "HelloWorld");
        assert_eq!(sanitize_filename("test/path"), "testpath");
        assert_eq!(sanitize_filename("normal"), "normal");
    }

    #[test]
    fn sanitize_filename_preserves_underscores_and_hyphens() {
        assert_eq!(sanitize_filename("my-file_name"), "my-file_name");
    }
}
