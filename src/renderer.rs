use anyhow::Result;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use crate::cli::OutputFormat;
use crate::types::{
    AllNames, ClassDocumentation, ClassMetadata, FlowDocumentation, FlowMetadata,
    TriggerDocumentation, TriggerMetadata,
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn render_class_page(ctx: &RenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known: HashSet<&str> = ctx
        .all_names
        .class_names
        .iter()
        .chain(ctx.all_names.trigger_names.iter())
        .chain(ctx.all_names.flow_names.iter())
        .map(|s| s.as_str())
        .collect();

    let mut out = String::new();

    // Title + badges
    out.push_str(&format!("# {}\n\n", doc.class_name));
    out.push_str(&render_badges(meta));
    out.push('\n');

    // Summary
    out.push_str(&format!("{}\n\n", doc.summary));

    // Table of contents
    out.push_str(&render_toc(doc));
    out.push('\n');

    // Description
    out.push_str("## Description\n\n");
    out.push_str(&doc.description);
    out.push_str("\n\n");

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
            known
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| format!("[{}]({}.md) — {}", name, name, rel))
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
    let known: HashSet<&str> = ctx
        .all_names
        .class_names
        .iter()
        .chain(ctx.all_names.trigger_names.iter())
        .chain(ctx.all_names.flow_names.iter())
        .map(|s| s.as_str())
        .collect();

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
                out.push_str(&format!("- [{cls}]({cls}.md)\n"));
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
            known
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| format!("[{name}]({name}.md) — {rel}"))
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
    let known: HashSet<&str> = ctx
        .all_names
        .class_names
        .iter()
        .chain(ctx.all_names.trigger_names.iter())
        .chain(ctx.all_names.flow_names.iter())
        .map(|s| s.as_str())
        .collect();

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
            known
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| format!("[{name}]({name}.md) — {rel}"))
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

pub fn render_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
) -> String {
    let mut out = String::new();
    out.push_str("# Apex Documentation Index\n\n");
    out.push_str(&format!(
        "Generated documentation for {} class(es), {} trigger(s), and {} flow(s).\n\n",
        class_contexts.len(),
        trigger_contexts.len(),
        flow_contexts.len(),
    ));

    // Group classes by folder, sorted alphabetically within each group.
    let mut class_by_folder: BTreeMap<&str, Vec<&RenderContext>> = BTreeMap::new();
    for ctx in class_contexts {
        class_by_folder
            .entry(ctx.folder.as_str())
            .or_default()
            .push(ctx);
    }
    for group in class_by_folder.values_mut() {
        group.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));
    }

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
                "| [{}]({}.md) | {} |\n",
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
                    "| [{}]({}.md) | `{}` | {} |\n",
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
                    "| [{}]({}.md) | `{}` | {} |\n",
                    ctx.documentation.label,
                    ctx.metadata.api_name,
                    ctx.metadata.process_type,
                    ctx.documentation.summary,
                ));
            }
            out.push('\n');
        }
    }

    out
}

pub fn write_output(
    output_dir: &Path,
    format: &OutputFormat,
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
) -> Result<()> {
    if *format == OutputFormat::Html {
        return crate::html_renderer::write_html_output(
            output_dir,
            class_contexts,
            trigger_contexts,
            flow_contexts,
        );
    }

    std::fs::create_dir_all(output_dir)?;

    for ctx in class_contexts {
        let page = render_class_page(ctx);
        std::fs::write(
            output_dir.join(format!("{}.md", ctx.metadata.class_name)),
            page,
        )?;
    }

    for ctx in trigger_contexts {
        let page = render_trigger_page(ctx);
        std::fs::write(
            output_dir.join(format!("{}.md", ctx.metadata.trigger_name)),
            page,
        )?;
    }

    for ctx in flow_contexts {
        let page = render_flow_page(ctx);
        std::fs::write(
            output_dir.join(format!("{}.md", ctx.metadata.api_name)),
            page,
        )?;
    }

    let index = render_index(class_contexts, trigger_contexts, flow_contexts);
    std::fs::write(output_dir.join("index.md"), index)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn render_badges(meta: &ClassMetadata) -> String {
    let mut badges = vec![format!("`{}`", meta.access_modifier)];
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
    use std::sync::Arc;

    fn sample_context() -> RenderContext {
        RenderContext {
            metadata: ClassMetadata {
                class_name: "AccountService".to_string(),
                access_modifier: "public".to_string(),
                is_abstract: false,
                is_virtual: false,
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
                class_names: vec!["AccountService".to_string(), "AccountRepository".to_string()],
                trigger_names: vec![],
                flow_names: vec![],
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
        let index = render_index(&[ctx], &[], &[]);
        assert!(index.contains("# Apex Documentation Index"));
        assert!(index.contains("[AccountService](AccountService.md)"));
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
        write_output(
            tmp.path(),
            &crate::cli::OutputFormat::Markdown,
            &[ctx],
            &[],
            &[],
        )
        .unwrap();
        assert!(tmp.path().join("AccountService.md").exists());
        assert!(tmp.path().join("index.md").exists());
    }
}
