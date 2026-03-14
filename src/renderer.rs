use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::types::{ClassDocumentation, ClassMetadata};

pub struct RenderContext {
    pub metadata: ClassMetadata,
    pub documentation: ClassDocumentation,
    /// Names of all classes in the project (for cross-linking)
    pub all_class_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn render_class_page(ctx: &RenderContext) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let known: HashSet<&str> = ctx.all_class_names.iter().map(|s| s.as_str()).collect();

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

pub fn render_index(contexts: &[RenderContext]) -> String {
    let mut out = String::new();
    out.push_str("# Apex Documentation Index\n\n");
    out.push_str(&format!(
        "Generated documentation for {} class(es).\n\n",
        contexts.len()
    ));

    // Group by folder (first segment of path relative to source dir)
    // For simplicity in this version, list alphabetically
    let mut sorted: Vec<&RenderContext> = contexts.iter().collect();
    sorted.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));

    out.push_str("## Classes\n\n");
    out.push_str("| Class | Summary |\n");
    out.push_str("|-------|---------|\n");
    for ctx in &sorted {
        out.push_str(&format!(
            "| [{}]({}.md) | {} |\n",
            ctx.documentation.class_name,
            ctx.documentation.class_name,
            ctx.documentation.summary
        ));
    }
    out.push('\n');

    out
}

pub fn write_output(output_dir: &Path, contexts: &[RenderContext]) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    // Write individual class pages
    for ctx in contexts {
        let page = render_class_page(ctx);
        let file_name = format!("{}.md", ctx.documentation.class_name);
        std::fs::write(output_dir.join(&file_name), page)?;
    }

    // Write index
    let index = render_index(contexts);
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
        ClassDocumentation, ClassMetadata, MethodDocumentation, ParamDocumentation,
        PropertyDocumentation,
    };

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
            all_class_names: vec!["AccountService".to_string(), "AccountRepository".to_string()],
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
        let index = render_index(&[ctx]);
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
        write_output(tmp.path(), &[ctx]).unwrap();
        assert!(tmp.path().join("AccountService.md").exists());
        assert!(tmp.path().join("index.md").exists());
    }
}
