use anyhow::Result;
use std::path::Path;

use crate::renderer::{RenderContext, TriggerRenderContext};

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
    "public", "private", "protected", "global", "static", "final",
    "abstract", "virtual", "override", "class", "interface", "enum",
    "trigger", "on", "new", "return", "if", "else", "for", "while",
    "do", "try", "catch", "finally", "throw", "void", "null", "true",
    "false", "this", "super", "extends", "implements",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn write_html_output(
    output_dir: &Path,
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    let class_names: Vec<&str> = class_contexts
        .iter()
        .map(|c| c.metadata.class_name.as_str())
        .collect();
    let trigger_names: Vec<&str> = trigger_contexts
        .iter()
        .map(|c| c.metadata.trigger_name.as_str())
        .collect();

    for ctx in class_contexts {
        let page = render_class_page(ctx, &class_names, &trigger_names);
        std::fs::write(
            output_dir.join(format!("{}.html", ctx.metadata.class_name)),
            page,
        )?;
    }

    for ctx in trigger_contexts {
        let page = render_trigger_page(ctx, &class_names, &trigger_names);
        std::fs::write(
            output_dir.join(format!("{}.html", ctx.metadata.trigger_name)),
            page,
        )?;
    }

    let index = render_index(class_contexts, trigger_contexts, &class_names, &trigger_names);
    std::fs::write(output_dir.join("index.html"), index)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Page renderers
// ---------------------------------------------------------------------------

fn render_class_page(
    ctx: &RenderContext,
    class_names: &[&str],
    trigger_names: &[&str],
) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.class_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&doc.class_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str(&format!("<span class=\"badge\">{}</span>\n", escape(&meta.access_modifier)));
    if meta.is_abstract { body.push_str("<span class=\"badge\">abstract</span>\n"); }
    if meta.is_virtual  { body.push_str("<span class=\"badge\">virtual</span>\n"); }
    if let Some(ref ext) = meta.extends {
        body.push_str(&format!("<span class=\"badge\">extends {}</span>\n", escape(ext)));
    }
    for iface in &meta.implements {
        body.push_str(&format!("<span class=\"badge\">implements {}</span>\n", escape(iface)));
    }
    body.push_str("</div>\n");

    body.push_str(&format!("<p class=\"summary\">{}</p>\n", escape(&doc.summary)));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.properties.is_empty() {
        body.push_str("<h2>Properties</h2>\n");
        body.push_str("<table><thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead><tbody>\n");
        for prop in &doc.properties {
            let prop_type = meta.properties.iter()
                .find(|p| p.name == prop.name)
                .map(|p| if p.is_static { format!("static {}", p.property_type) } else { p.property_type.clone() })
                .unwrap_or_else(|| "—".to_string());
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&prop.name), escape(&prop_type), escape(&prop.description)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.methods.is_empty() {
        body.push_str("<h2>Methods</h2>\n");
        for method_doc in &doc.methods {
            let sig = meta.methods.iter().find(|m| m.name == method_doc.name)
                .map(|m| {
                    let params: Vec<String> = m.params.iter()
                        .map(|p| format!("{} {}", p.param_type, p.name))
                        .collect();
                    let static_kw = if m.is_static { "static " } else { "" };
                    format!("{} {}{}({}): {}", m.access_modifier, static_kw, m.name, params.join(", "), m.return_type)
                })
                .unwrap_or_else(|| method_doc.name.clone());

            body.push_str(&format!("<h3>{}</h3>\n", escape(&sig)));
            body.push_str(&format!("<p>{}</p>\n", escape(&method_doc.description)));

            if !method_doc.params.is_empty() {
                body.push_str("<p><strong>Parameters</strong></p>\n");
                body.push_str("<table><thead><tr><th>Name</th><th>Description</th></tr></thead><tbody>\n");
                for param in &method_doc.params {
                    body.push_str(&format!(
                        "<tr><td><code>{}</code></td><td>{}</td></tr>\n",
                        escape(&param.name), escape(&param.description)
                    ));
                }
                body.push_str("</tbody></table>\n");
            }

            if method_doc.returns != "void" && !method_doc.returns.is_empty() {
                body.push_str(&format!("<p><strong>Returns:</strong> {}</p>\n", escape(&method_doc.returns)));
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
            body.push_str(&format!("<pre><code>{}</code></pre>\n", highlight_apex(&code)));
        }
    }

    let see_also: Vec<String> = doc.relationships.iter().filter_map(|rel| {
        class_names.iter()
            .find(|&&name| rel.contains(name))
            .map(|&name| format!("<a href=\"{}.html\">{}</a> — {}", name, escape(name), escape(rel)))
            .or_else(|| {
                trigger_names.iter()
                    .find(|&&name| rel.contains(name))
                    .map(|&name| format!("<a href=\"{}.html\">{}</a> — {}", name, escape(name), escape(rel)))
            })
    }).collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(&doc.class_name, "sfdoc", &body, active, class_names, trigger_names)
}

fn render_trigger_page(
    ctx: &TriggerRenderContext,
    class_names: &[&str],
    trigger_names: &[&str],
) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.trigger_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&doc.trigger_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str(&format!("<span class=\"badge badge-trigger\">trigger on {}</span>\n", escape(&doc.sobject)));
    for event in &meta.events {
        body.push_str(&format!("<span class=\"badge\">{}</span>\n", event.as_str()));
    }
    body.push_str("</div>\n");
    body.push_str(&format!("<p class=\"summary\">{}</p>\n", escape(&doc.summary)));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.events.is_empty() {
        body.push_str("<h2>Event Handlers</h2>\n");
        body.push_str("<table><thead><tr><th>Event</th><th>Description</th></tr></thead><tbody>\n");
        for ev in &doc.events {
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&ev.event), escape(&ev.description)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.handler_classes.is_empty() {
        body.push_str("<h2>Handler Classes</h2>\n<ul>\n");
        for cls in &doc.handler_classes {
            if class_names.contains(&cls.as_str()) {
                body.push_str(&format!("<li><a href=\"{}.html\">{}</a></li>\n", escape(cls), escape(cls)));
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

    let see_also: Vec<String> = doc.relationships.iter().filter_map(|rel| {
        class_names.iter()
            .find(|&&name| rel.contains(name))
            .map(|&name| format!("<a href=\"{}.html\">{}</a> — {}", name, escape(name), escape(rel)))
            .or_else(|| {
                trigger_names.iter()
                    .find(|&&name| rel.contains(name))
                    .map(|&name| format!("<a href=\"{}.html\">{}</a> — {}", name, escape(name), escape(rel)))
            })
    }).collect();

    if !see_also.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for link in see_also {
            body.push_str(&format!("<li>{}</li>\n", link));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(&doc.trigger_name, "sfdoc", &body, active, class_names, trigger_names)
}

fn render_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    class_names: &[&str],
    trigger_names: &[&str],
) -> String {
    let mut body = String::new();
    body.push_str("<h1>Apex Documentation</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">Generated documentation for {} class(es) and {} trigger(s).</p>\n",
        class_contexts.len(),
        trigger_contexts.len()
    ));

    if !class_contexts.is_empty() {
        body.push_str("<h2>Classes</h2>\n");
        body.push_str("<table><thead><tr><th>Class</th><th>Summary</th></tr></thead><tbody>\n");
        let mut sorted_classes: Vec<&RenderContext> = class_contexts.iter().collect();
        sorted_classes.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));
        for ctx in sorted_classes {
            body.push_str(&format!(
                "<tr><td><a href=\"{}.html\">{}</a></td><td>{}</td></tr>\n",
                escape(&ctx.metadata.class_name),
                escape(&ctx.documentation.class_name),
                escape(&ctx.documentation.summary),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !trigger_contexts.is_empty() {
        body.push_str("<h2>Triggers</h2>\n");
        body.push_str("<table><thead><tr><th>Trigger</th><th>SObject</th><th>Summary</th></tr></thead><tbody>\n");
        let mut sorted_triggers: Vec<&TriggerRenderContext> = trigger_contexts.iter().collect();
        sorted_triggers.sort_by(|a, b| a.documentation.trigger_name.cmp(&b.documentation.trigger_name));
        for ctx in sorted_triggers {
            body.push_str(&format!(
                "<tr><td><a href=\"{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&ctx.metadata.trigger_name),
                escape(&ctx.documentation.trigger_name),
                escape(&ctx.documentation.sobject),
                escape(&ctx.documentation.summary),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    wrap_page("Overview", "sfdoc", &body, "Overview", class_names, trigger_names)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn wrap_page(
    title: &str,
    brand: &str,
    body: &str,
    active: &str,
    class_names: &[&str],
    trigger_names: &[&str],
) -> String {
    let sidebar = render_sidebar(class_names, trigger_names, active);
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

fn render_sidebar(class_names: &[&str], trigger_names: &[&str], active: &str) -> String {
    let mut s = String::new();
    s.push_str("<nav class=\"sidebar\">\n");
    s.push_str("<a class=\"sidebar-brand\" href=\"index.html\">sfdoc</a>\n");

    if !class_names.is_empty() {
        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Classes</div>\n");
        s.push_str("<ul>\n");
        for name in class_names {
            let cls = if *name == active { " class=\"active\"" } else { "" };
            s.push_str(&format!(
                "<li><a href=\"{}.html\"{cls}>{}</a></li>\n",
                name,
                escape(name)
            ));
        }
        s.push_str("</ul>\n</div>\n");
    }

    if !trigger_names.is_empty() {
        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Triggers</div>\n");
        s.push_str("<ul>\n");
        for name in trigger_names {
            let cls = if *name == active { " class=\"active\"" } else { "" };
            s.push_str(&format!(
                "<li><a href=\"{}.html\"{cls}>{}</a></li>\n",
                name,
                escape(name)
            ));
        }
        s.push_str("</ul>\n</div>\n");
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
