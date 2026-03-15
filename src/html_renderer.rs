use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

use crate::renderer::{
    sanitize_filename, FlowRenderContext, ObjectRenderContext, RenderContext, TriggerRenderContext,
    ValidationRuleRenderContext,
};

// ---------------------------------------------------------------------------
// Inline CSS — no external dependencies, works offline
// ---------------------------------------------------------------------------

const CSS: &str = r#"
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;font-size:15px;color:#24292e;background:#fff;display:flex;min-height:100vh}
a{color:#0366d6;text-decoration:none}
a:hover{text-decoration:underline}
.sidebar{width:220px;min-width:220px;background:#f6f8fa;border-right:1px solid #e1e4e8;padding:12px 0;overflow-y:auto;position:sticky;top:0;height:100vh;flex-shrink:0}
.sidebar-brand{font-weight:700;font-size:14px;color:#24292e;padding:8px 16px 12px;border-bottom:1px solid #e1e4e8;margin-bottom:8px;display:block}
.sidebar-section{margin-bottom:8px}
.sidebar-heading{font-size:11px;font-weight:600;color:#6a737d;text-transform:uppercase;letter-spacing:.5px;padding:6px 16px 2px}
.sidebar ul{list-style:none}
.sidebar li a{display:block;padding:3px 16px;font-size:13px;color:#24292e;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.sidebar li a:hover{background:#e1e4e8;text-decoration:none}
.sidebar li a.active{background:#0366d6;color:#fff;border-radius:0}
.sidebar-folder{font-size:10px;font-weight:600;color:#959da5;text-transform:uppercase;letter-spacing:.4px;padding:6px 16px 1px;margin-top:4px}
.content{flex:1;padding:32px 48px;max-width:900px;min-width:0}
h1{font-size:26px;margin-bottom:8px;line-height:1.3}
h2{font-size:18px;margin:28px 0 10px;padding-bottom:6px;border-bottom:1px solid #e1e4e8}
h3{font-size:14px;margin:16px 0 8px;font-family:'SFMono-Regular',Consolas,monospace;background:#f6f8fa;padding:8px 12px;border-radius:4px;font-weight:600;word-break:break-all}
p{margin-bottom:12px;line-height:1.6}
table{border-collapse:collapse;width:100%;margin-bottom:16px;font-size:13px}
th,td{border:1px solid #e1e4e8;padding:7px 12px;text-align:left}
th{background:#f6f8fa;font-weight:600}
code{background:#f6f8fa;padding:2px 5px;border-radius:3px;font-family:'SFMono-Regular',Consolas,monospace;font-size:12px}
pre{background:#1e1e1e;color:#d4d4d4;padding:16px;border-radius:6px;overflow-x:auto;margin-bottom:16px;font-size:13px;line-height:1.5}
pre code{background:none;padding:0;color:inherit;font-size:inherit}
.kw{color:#569cd6}
.badges{margin-bottom:16px;line-height:2}
.badge{display:inline-block;background:#f1f8ff;border:1px solid #c8e1ff;color:#0366d6;border-radius:12px;padding:2px 10px;font-size:12px;margin-right:4px;font-family:'SFMono-Regular',Consolas,monospace}
.badge-trigger{background:#fff8f0;border-color:#ffd3a3;color:#b07d00}
.summary{font-size:16px;color:#586069;margin-bottom:20px;line-height:1.5}
ul{padding-left:20px;margin-bottom:12px}
li{margin-bottom:4px;line-height:1.5}
"#;

const APEX_KEYWORDS: &[&str] = &[
    "public",
    "private",
    "protected",
    "global",
    "static",
    "final",
    "abstract",
    "virtual",
    "override",
    "class",
    "interface",
    "enum",
    "trigger",
    "on",
    "new",
    "return",
    "if",
    "else",
    "for",
    "while",
    "do",
    "try",
    "catch",
    "finally",
    "throw",
    "void",
    "null",
    "true",
    "false",
    "this",
    "super",
    "extends",
    "implements",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn write_html_output(
    output_dir: &Path,
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
) -> Result<()> {
    let classes_dir = output_dir.join("classes");
    let triggers_dir = output_dir.join("triggers");
    let flows_dir = output_dir.join("flows");
    let vr_dir = output_dir.join("validation-rules");
    let objects_dir = output_dir.join("objects");

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

    // (name, folder) pairs — used for sidebar grouping and cross-link generation.
    let class_items: Vec<(&str, &str)> = class_contexts
        .iter()
        .map(|c| (c.metadata.class_name.as_str(), c.folder.as_str()))
        .collect();
    let trigger_items: Vec<(&str, &str)> = trigger_contexts
        .iter()
        .map(|c| (c.metadata.trigger_name.as_str(), c.folder.as_str()))
        .collect();
    let flow_items: Vec<(&str, &str)> = flow_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str()))
        .collect();
    let vr_items: Vec<(&str, &str)> = validation_rule_contexts
        .iter()
        .map(|c| (c.metadata.rule_name.as_str(), c.folder.as_str()))
        .collect();
    let obj_items: Vec<(&str, &str)> = object_contexts
        .iter()
        .map(|c| (c.metadata.object_name.as_str(), c.folder.as_str()))
        .collect();

    for ctx in class_contexts {
        let page = render_class_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
        );
        std::fs::write(
            classes_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.class_name)
            )),
            page,
        )?;
    }

    for ctx in trigger_contexts {
        let page = render_trigger_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
        );
        std::fs::write(
            triggers_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.trigger_name)
            )),
            page,
        )?;
    }

    for ctx in flow_contexts {
        let page = render_flow_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
        );
        std::fs::write(
            flows_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.api_name)
            )),
            page,
        )?;
    }

    for ctx in validation_rule_contexts {
        let page = render_validation_rule_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
        );
        std::fs::write(
            vr_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.rule_name)
            )),
            page,
        )?;
    }

    for ctx in object_contexts {
        let page = render_object_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
        );
        std::fs::write(
            objects_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.object_name)
            )),
            page,
        )?;
    }

    let index = render_index(
        class_contexts,
        trigger_contexts,
        flow_contexts,
        validation_rule_contexts,
        object_contexts,
    );
    std::fs::write(output_dir.join("index.html"), index)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Page renderers
// ---------------------------------------------------------------------------

fn render_class_page(
    ctx: &RenderContext,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.class_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&doc.class_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str(&format!(
        "<span class=\"badge\">{}</span>\n",
        escape(&meta.access_modifier)
    ));
    if meta.is_abstract {
        body.push_str("<span class=\"badge\">abstract</span>\n");
    }
    if meta.is_virtual {
        body.push_str("<span class=\"badge\">virtual</span>\n");
    }
    if let Some(ref ext) = meta.extends {
        body.push_str(&format!(
            "<span class=\"badge\">extends {}</span>\n",
            escape(ext)
        ));
    }
    for iface in &meta.implements {
        body.push_str(&format!(
            "<span class=\"badge\">implements {}</span>\n",
            escape(iface)
        ));
    }
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.properties.is_empty() {
        body.push_str("<h2>Properties</h2>\n");
        body.push_str("<table><thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead><tbody>\n");
        for prop in &doc.properties {
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
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&prop.name),
                escape(&prop_type),
                escape(&prop.description)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.methods.is_empty() {
        body.push_str("<h2>Methods</h2>\n");
        for method_doc in &doc.methods {
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
                        m.return_type
                    )
                })
                .unwrap_or_else(|| method_doc.name.clone());

            body.push_str(&format!("<h3>{}</h3>\n", escape(&sig)));
            body.push_str(&format!("<p>{}</p>\n", escape(&method_doc.description)));

            if !method_doc.params.is_empty() {
                body.push_str("<p><strong>Parameters</strong></p>\n");
                body.push_str(
                    "<table><thead><tr><th>Name</th><th>Description</th></tr></thead><tbody>\n",
                );
                for param in &method_doc.params {
                    body.push_str(&format!(
                        "<tr><td><code>{}</code></td><td>{}</td></tr>\n",
                        escape(&param.name),
                        escape(&param.description)
                    ));
                }
                body.push_str("</tbody></table>\n");
            }

            if method_doc.returns != "void" && !method_doc.returns.is_empty() {
                body.push_str(&format!(
                    "<p><strong>Returns:</strong> {}</p>\n",
                    escape(&method_doc.returns)
                ));
            }

            if !method_doc.throws.is_empty() {
                body.push_str("<p><strong>Throws</strong></p><ul>\n");
                for exc in &method_doc.throws {
                    body.push_str(&format!("<li>{}</li>\n", escape(exc)));
                }
                body.push_str("</ul>\n");
            }
        }
    }

    if !doc.usage_examples.is_empty() {
        body.push_str("<h2>Usage Examples</h2>\n");
        for example in &doc.usage_examples {
            // Strip markdown code fences if present, then highlight
            let code = strip_code_fence(example);
            body.push_str(&format!(
                "<pre><code>{}</code></pre>\n",
                highlight_apex(&code)
            ));
        }
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            class_names
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| {
                    format!(
                        "<a href=\"{name}.html\">{}</a> — {}",
                        escape(name),
                        escape(rel)
                    )
                })
                .or_else(|| {
                    trigger_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../triggers/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    flow_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../flows/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    vr_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../validation-rules/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    obj_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../objects/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
        })
        .collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &doc.class_name,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
    )
}

fn render_trigger_page(
    ctx: &TriggerRenderContext,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.trigger_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&doc.trigger_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str(&format!(
        "<span class=\"badge badge-trigger\">trigger on {}</span>\n",
        escape(&doc.sobject)
    ));
    for event in &meta.events {
        body.push_str(&format!(
            "<span class=\"badge\">{}</span>\n",
            event.as_str()
        ));
    }
    body.push_str("</div>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.events.is_empty() {
        body.push_str("<h2>Event Handlers</h2>\n");
        body.push_str("<table><thead><tr><th>Event</th><th>Description</th></tr></thead><tbody>\n");
        for ev in &doc.events {
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&ev.event),
                escape(&ev.description)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.handler_classes.is_empty() {
        body.push_str("<h2>Handler Classes</h2>\n<ul>\n");
        for cls in &doc.handler_classes {
            if class_names.contains(&cls.as_str()) {
                body.push_str(&format!(
                    "<li><a href=\"../classes/{}.html\">{}</a></li>\n",
                    escape(cls),
                    escape(cls)
                ));
            } else {
                body.push_str(&format!("<li><code>{}</code></li>\n", escape(cls)));
            }
        }
        body.push_str("</ul>\n");
    }

    if !doc.usage_notes.is_empty() {
        body.push_str("<h2>Usage Notes</h2>\n<ul>\n");
        for note in &doc.usage_notes {
            body.push_str(&format!("<li>{}</li>\n", escape(note)));
        }
        body.push_str("</ul>\n");
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            class_names
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| {
                    format!(
                        "<a href=\"../classes/{name}.html\">{}</a> — {}",
                        escape(name),
                        escape(rel)
                    )
                })
                .or_else(|| {
                    trigger_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    flow_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../flows/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    vr_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../validation-rules/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    obj_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../objects/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
        })
        .collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &doc.trigger_name,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
    )
}

fn render_flow_page(
    ctx: &FlowRenderContext,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.api_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&doc.label)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">flow</span>\n");
    body.push_str(&format!(
        "<span class=\"badge\">{}</span>\n",
        escape(&meta.process_type)
    ));
    body.push_str("</div>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    body.push_str("<h2>Business Process</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.business_process)));

    body.push_str("<h2>Entry Criteria</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.entry_criteria)));

    if !meta.variables.is_empty() {
        body.push_str("<h2>Variables</h2>\n");
        body.push_str(
            "<table><thead><tr><th>Name</th><th>Type</th><th>Direction</th></tr></thead><tbody>\n",
        );
        for v in &meta.variables {
            let direction = match (v.is_input, v.is_output) {
                (true, true) => "Input / Output",
                (true, false) => "Input",
                (false, true) => "Output",
                (false, false) => "Internal",
            };
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&v.name),
                escape(&v.data_type),
                direction
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !meta.record_operations.is_empty() {
        body.push_str("<h2>Record Operations</h2>\n");
        body.push_str("<table><thead><tr><th>Operation</th><th>Object</th></tr></thead><tbody>\n");
        for op in &meta.record_operations {
            body.push_str(&format!(
                "<tr><td>{}</td><td><code>{}</code></td></tr>\n",
                escape(&op.operation),
                escape(&op.object)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !meta.action_calls.is_empty() {
        body.push_str("<h2>Action Calls</h2>\n");
        body.push_str("<table><thead><tr><th>Action</th><th>Type</th></tr></thead><tbody>\n");
        for action in &meta.action_calls {
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&action.name),
                escape(&action.action_type)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if meta.decisions > 0 || meta.loops > 0 || meta.screens > 0 {
        body.push_str("<h2>Element Counts</h2>\n<ul>\n");
        if meta.decisions > 0 {
            body.push_str(&format!("<li>Decisions: {}</li>\n", meta.decisions));
        }
        if meta.loops > 0 {
            body.push_str(&format!("<li>Loops: {}</li>\n", meta.loops));
        }
        if meta.screens > 0 {
            body.push_str(&format!("<li>Screens: {}</li>\n", meta.screens));
        }
        body.push_str("</ul>\n");
    }

    if !doc.key_decisions.is_empty() {
        body.push_str("<h2>Key Decisions</h2>\n<ul>\n");
        for d in &doc.key_decisions {
            body.push_str(&format!("<li>{}</li>\n", escape(d)));
        }
        body.push_str("</ul>\n");
    }

    if !doc.admin_notes.is_empty() {
        body.push_str("<h2>Admin Notes</h2>\n<ul>\n");
        for note in &doc.admin_notes {
            body.push_str(&format!("<li>{}</li>\n", escape(note)));
        }
        body.push_str("</ul>\n");
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            class_names
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| {
                    format!(
                        "<a href=\"../classes/{name}.html\">{}</a> — {}",
                        escape(name),
                        escape(rel)
                    )
                })
                .or_else(|| {
                    trigger_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../triggers/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    flow_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    vr_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../validation-rules/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    obj_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../objects/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
        })
        .collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &doc.label,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
    )
}

fn render_validation_rule_page(
    ctx: &ValidationRuleRenderContext,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.rule_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&meta.rule_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">validation-rule</span>\n");
    body.push_str(&format!(
        "<span class=\"badge\">on {}</span>\n",
        escape(&meta.object_name)
    ));
    if meta.active {
        body.push_str("<span class=\"badge\">active</span>\n");
    } else {
        body.push_str("<span class=\"badge\">inactive</span>\n");
    }
    body.push_str("</div>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>When It Fires</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.when_fires)));

    body.push_str("<h2>What It Protects</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.what_protects)));

    body.push_str("<h2>Error Condition Formula</h2>\n");
    body.push_str(&format!(
        "<pre><code>{}</code></pre>\n",
        escape(&meta.error_condition_formula)
    ));

    body.push_str("<h2>Formula Explanation</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.formula_explanation)));

    body.push_str("<h2>Error Message</h2>\n");
    body.push_str(&format!(
        "<blockquote><p>{}</p></blockquote>\n",
        escape(&meta.error_message)
    ));
    if !meta.error_display_field.is_empty() {
        body.push_str(&format!(
            "<p><strong>Displayed on field:</strong> <code>{}</code></p>\n",
            escape(&meta.error_display_field)
        ));
    }

    if !doc.edge_cases.is_empty() {
        body.push_str("<h2>Edge Cases</h2>\n<ul>\n");
        for case in &doc.edge_cases {
            body.push_str(&format!("<li>{}</li>\n", escape(case)));
        }
        body.push_str("</ul>\n");
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            class_names
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| {
                    format!(
                        "<a href=\"../classes/{name}.html\">{}</a> — {}",
                        escape(name),
                        escape(rel)
                    )
                })
                .or_else(|| {
                    trigger_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../triggers/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    flow_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../flows/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    vr_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    obj_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../objects/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
        })
        .collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &meta.rule_name,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
    )
}

fn render_object_page(
    ctx: &ObjectRenderContext,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.object_name;

    let label = if doc.label.is_empty() {
        meta.object_name.as_str()
    } else {
        doc.label.as_str()
    };

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(label)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">object</span>\n");
    body.push_str(&format!(
        "<span class=\"badge\">{}</span>\n",
        escape(&meta.object_name)
    ));
    body.push_str("</div>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    if !doc.purpose.is_empty() {
        body.push_str("<h2>Purpose</h2>\n");
        body.push_str(&format!("<p>{}</p>\n", escape(&doc.purpose)));
    }

    if !doc.description.is_empty() {
        body.push_str("<h2>Description</h2>\n");
        body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));
    }

    if !meta.fields.is_empty() {
        body.push_str("<h2>Fields</h2>\n");
        body.push_str("<table><thead><tr><th>API Name</th><th>Type</th><th>Label</th><th>Required</th></tr></thead><tbody>\n");
        for field in &meta.fields {
            let type_str = if (field.field_type == "Lookup" || field.field_type == "MasterDetail")
                && !field.reference_to.is_empty()
            {
                format!(
                    "{} → <code>{}</code>",
                    escape(&field.field_type),
                    escape(&field.reference_to)
                )
            } else {
                escape(&field.field_type)
            };
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape(&field.api_name),
                type_str,
                escape(&field.label),
                if field.required { "Yes" } else { "No" },
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.key_fields.is_empty() {
        body.push_str("<h2>Key Fields</h2>\n<ul>\n");
        for f in &doc.key_fields {
            body.push_str(&format!("<li>{}</li>\n", escape(f)));
        }
        body.push_str("</ul>\n");
    }

    if !doc.admin_notes.is_empty() {
        body.push_str("<h2>Admin Notes</h2>\n<ul>\n");
        for note in &doc.admin_notes {
            body.push_str(&format!("<li>{}</li>\n", escape(note)));
        }
        body.push_str("</ul>\n");
    }

    let see_also: Vec<String> = doc
        .relationships
        .iter()
        .filter_map(|rel| {
            class_names
                .iter()
                .find(|&&name| rel.contains(name))
                .map(|&name| {
                    format!(
                        "<a href=\"../classes/{name}.html\">{}</a> — {}",
                        escape(name),
                        escape(rel)
                    )
                })
                .or_else(|| {
                    trigger_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../triggers/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    flow_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../flows/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    vr_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../validation-rules/{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    obj_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"{name}.html\">{}</a> — {}",
                                escape(name),
                                escape(rel)
                            )
                        })
                })
        })
        .collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        label,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
    )
}

fn render_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
) -> String {
    let class_items: Vec<(&str, &str)> = class_contexts
        .iter()
        .map(|c| (c.metadata.class_name.as_str(), c.folder.as_str()))
        .collect();
    let trigger_items: Vec<(&str, &str)> = trigger_contexts
        .iter()
        .map(|c| (c.metadata.trigger_name.as_str(), c.folder.as_str()))
        .collect();
    let flow_items: Vec<(&str, &str)> = flow_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str()))
        .collect();
    let vr_items: Vec<(&str, &str)> = validation_rule_contexts
        .iter()
        .map(|c| (c.metadata.rule_name.as_str(), c.folder.as_str()))
        .collect();
    let obj_items: Vec<(&str, &str)> = object_contexts
        .iter()
        .map(|c| (c.metadata.object_name.as_str(), c.folder.as_str()))
        .collect();
    let mut body = String::new();
    body.push_str("<h1>Apex Documentation</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">Generated documentation for {} class(es), {} trigger(s), {} flow(s), {} validation rule(s), and {} object(s).</p>\n",
        class_contexts.len(),
        trigger_contexts.len(),
        flow_contexts.len(),
        validation_rule_contexts.len(),
        object_contexts.len(),
    ));

    if !class_contexts.is_empty() {
        // Group classes by folder.
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

        body.push_str("<h2>Classes</h2>\n");
        let multi_folder = class_by_folder.len() > 1;
        for (folder, classes) in &class_by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                body.push_str(&format!("<h3>{}</h3>\n", escape(label)));
            }
            body.push_str("<table><thead><tr><th>Class</th><th>Summary</th></tr></thead><tbody>\n");
            for ctx in classes {
                body.push_str(&format!(
                    "<tr><td><a href=\"classes/{}.html\">{}</a></td><td>{}</td></tr>\n",
                    escape(&ctx.metadata.class_name),
                    escape(&ctx.documentation.class_name),
                    escape(&ctx.documentation.summary),
                ));
            }
            body.push_str("</tbody></table>\n");
        }
    }

    if !trigger_contexts.is_empty() {
        // Group triggers by folder.
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

        body.push_str("<h2>Triggers</h2>\n");
        let multi_folder = trigger_by_folder.len() > 1;
        for (folder, triggers) in &trigger_by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                body.push_str(&format!("<h3>{}</h3>\n", escape(label)));
            }
            body.push_str("<table><thead><tr><th>Trigger</th><th>SObject</th><th>Summary</th></tr></thead><tbody>\n");
            for ctx in triggers {
                body.push_str(&format!(
                    "<tr><td><a href=\"triggers/{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td></tr>\n",
                    escape(&ctx.metadata.trigger_name),
                    escape(&ctx.documentation.trigger_name),
                    escape(&ctx.documentation.sobject),
                    escape(&ctx.documentation.summary),
                ));
            }
            body.push_str("</tbody></table>\n");
        }
    }

    if !flow_contexts.is_empty() {
        // Group flows by folder.
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

        body.push_str("<h2>Flows</h2>\n");
        let multi_folder = flow_by_folder.len() > 1;
        for (folder, flows) in &flow_by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                body.push_str(&format!("<h3>{}</h3>\n", escape(label)));
            }
            body.push_str("<table><thead><tr><th>Flow</th><th>Process Type</th><th>Summary</th></tr></thead><tbody>\n");
            for ctx in flows {
                body.push_str(&format!(
                    "<tr><td><a href=\"flows/{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td></tr>\n",
                    escape(&ctx.metadata.api_name),
                    escape(&ctx.documentation.label),
                    escape(&ctx.metadata.process_type),
                    escape(&ctx.documentation.summary),
                ));
            }
            body.push_str("</tbody></table>\n");
        }
    }

    if !validation_rule_contexts.is_empty() {
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

        body.push_str("<h2>Validation Rules</h2>\n");
        let multi_folder = vr_by_folder.len() > 1;
        for (folder, rules) in &vr_by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                body.push_str(&format!("<h3>{}</h3>\n", escape(label)));
            }
            body.push_str("<table><thead><tr><th>Rule</th><th>Object</th><th>Status</th><th>Summary</th></tr></thead><tbody>\n");
            for ctx in rules {
                let status = if ctx.metadata.active {
                    "active"
                } else {
                    "inactive"
                };
                body.push_str(&format!(
                    "<tr><td><a href=\"validation-rules/{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>\n",
                    escape(&ctx.metadata.rule_name),
                    escape(&ctx.metadata.rule_name),
                    escape(&ctx.metadata.object_name),
                    status,
                    escape(&ctx.documentation.summary),
                ));
            }
            body.push_str("</tbody></table>\n");
        }
    }

    if !object_contexts.is_empty() {
        let mut obj_sorted: Vec<&ObjectRenderContext> = object_contexts.iter().collect();
        obj_sorted.sort_by(|a, b| a.metadata.object_name.cmp(&b.metadata.object_name));

        body.push_str("<h2>Objects</h2>\n");
        body.push_str("<table><thead><tr><th>Object</th><th>Label</th><th>Fields</th><th>Summary</th></tr></thead><tbody>\n");
        for ctx in obj_sorted {
            let label = if ctx.documentation.label.is_empty() {
                ctx.metadata.object_name.as_str()
            } else {
                ctx.documentation.label.as_str()
            };
            body.push_str(&format!(
                "<tr><td><a href=\"objects/{}.html\">{}</a></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape(&ctx.metadata.object_name),
                escape(&ctx.metadata.object_name),
                escape(label),
                ctx.metadata.fields.len(),
                escape(&ctx.documentation.summary),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    wrap_page(
        "Overview",
        "sfdoc",
        &body,
        "Overview",
        "",
        &class_items,
        &trigger_items,
        &flow_items,
        &vr_items,
        &obj_items,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn wrap_page(
    title: &str,
    brand: &str,
    body: &str,
    active: &str,
    up_prefix: &str,
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
) -> String {
    let sidebar = render_sidebar(
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        active,
        up_prefix,
    );
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — {brand}</title>
<style>{CSS}</style>
</head>
<body>
{sidebar}
<main class="content">
{body}
</main>
</body>
</html>
"#,
        title = escape(title),
        brand = escape(brand),
    )
}

fn render_sidebar(
    class_items: &[(&str, &str)],
    trigger_items: &[(&str, &str)],
    flow_items: &[(&str, &str)],
    vr_items: &[(&str, &str)],
    obj_items: &[(&str, &str)],
    active: &str,
    up_prefix: &str,
) -> String {
    let mut s = String::new();
    s.push_str("<nav class=\"sidebar\">\n");
    s.push_str(&format!(
        "<a class=\"sidebar-brand\" href=\"{up_prefix}index.html\">sfdoc</a>\n"
    ));

    if !class_items.is_empty() {
        // Group by folder (BTreeMap gives alphabetical folder order).
        let mut by_folder: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for &(name, folder) in class_items {
            by_folder.entry(folder).or_default().push(name);
        }
        for names in by_folder.values_mut() {
            names.sort_unstable();
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Classes</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, names) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for name in names {
                let cls = if *name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<li><a href=\"{up_prefix}classes/{name}.html\"{cls}>{}</a></li>\n",
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !trigger_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for &(name, folder) in trigger_items {
            by_folder.entry(folder).or_default().push(name);
        }
        for names in by_folder.values_mut() {
            names.sort_unstable();
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Triggers</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, names) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for name in names {
                let cls = if *name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<li><a href=\"{up_prefix}triggers/{name}.html\"{cls}>{}</a></li>\n",
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !flow_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for &(name, folder) in flow_items {
            by_folder.entry(folder).or_default().push(name);
        }
        for names in by_folder.values_mut() {
            names.sort_unstable();
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Flows</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, names) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for name in names {
                let cls = if *name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<li><a href=\"{up_prefix}flows/{name}.html\"{cls}>{}</a></li>\n",
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !vr_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for &(name, folder) in vr_items {
            by_folder.entry(folder).or_default().push(name);
        }
        for names in by_folder.values_mut() {
            names.sort_unstable();
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Validation Rules</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, names) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for name in names {
                let cls = if *name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<li><a href=\"{up_prefix}validation-rules/{name}.html\"{cls}>{}</a></li>\n",
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !obj_items.is_empty() {
        let mut names: Vec<&str> = obj_items.iter().map(|&(n, _)| n).collect();
        names.sort_unstable();

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Objects</div>\n");
        s.push_str("<ul>\n");
        for name in names {
            let cls = if name == active {
                " class=\"active\""
            } else {
                ""
            };
            s.push_str(&format!(
                "<li><a href=\"{up_prefix}objects/{name}.html\"{cls}>{}</a></li>\n",
                escape(name)
            ));
        }
        s.push_str("</ul>\n");
        s.push_str("</div>\n");
    }

    s.push_str("</nav>\n");
    s
}

/// HTML-escape special characters.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Strip markdown code fences (```apex ... ``` or ``` ... ```) from a string.
fn strip_code_fence(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // skip the language identifier line
        let body = rest.trim_start_matches(|c: char| c.is_alphabetic());
        if let Some(code) = body.strip_suffix("```") {
            return code.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Wrap Apex keywords in `<span class="kw">` for syntax highlighting.
/// Input must already be HTML-escaped.
fn highlight_apex(source: &str) -> String {
    let escaped = escape(source);
    let mut result = escaped;
    for kw in APEX_KEYWORDS {
        // Replace whole-word occurrences only (avoid matching substrings)
        let pattern = format!(r"\b{kw}\b");
        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re
                .replace_all(&result, format!("<span class=\"kw\">{kw}</span>").as_str())
                .into_owned();
        }
    }
    result
}
