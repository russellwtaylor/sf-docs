use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::renderer::{
    sanitize_filename, AuraRenderContext, CustomMetadataRenderContext, FlexiPageRenderContext,
    FlowRenderContext, LwcRenderContext, ObjectRenderContext, RenderContext, TriggerRenderContext,
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
.badge-tag{background:#f0fff4;border-color:#a3d9a5;color:#22863a;cursor:pointer}
.badge-tag:hover{background:#dcffe4}
.summary{font-size:16px;color:#586069;margin-bottom:20px;line-height:1.5}
ul{padding-left:20px;margin-bottom:12px}
li{margin-bottom:4px;line-height:1.5}
.sidebar-search{padding:8px 12px}
.sidebar-search input{width:100%;padding:5px 8px;font-size:13px;border:1px solid #e1e4e8;border-radius:4px;outline:none}
.sidebar-search input:focus{border-color:#0366d6;box-shadow:0 0 0 2px rgba(3,102,214,.15)}
#sfdoc-search-results{display:none}
#sfdoc-search-results ul{list-style:none;padding:0}
#sfdoc-search-results li a{display:block;padding:3px 16px;font-size:13px;color:#24292e;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#sfdoc-search-results li a:hover{background:#e1e4e8;text-decoration:none}
"#;

const FUSE_JS: &str = include_str!("fuse.min.js");

const SEARCH_JS: &str = r#"
(function() {
  var scriptEl = document.currentScript;
  var base = scriptEl.src.replace(/search\.js$/, '');

  var sidebar = document.querySelector('.sidebar');
  var navSections = sidebar.querySelectorAll('.sidebar-section');
  var searchInput = document.getElementById('sfdoc-search');
  var resultsContainer = document.getElementById('sfdoc-search-results');
  var debounceTimer;

  fetch(base + 'search-index.json')
    .then(function(r) { return r.json(); })
    .then(function(data) {
      var fuse = new Fuse(data, {
        keys: ['title', 'summary', 'tags'],
        threshold: 0.3,
        includeScore: true
      });

      searchInput.addEventListener('input', function() {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(function() {
          var query = searchInput.value.trim();
          if (!query) {
            resultsContainer.style.display = 'none';
            navSections.forEach(function(s) { s.style.display = ''; });
            return;
          }
          var results = fuse.search(query).slice(0, 20);
          navSections.forEach(function(s) { s.style.display = 'none'; });
          resultsContainer.style.display = '';
          resultsContainer.innerHTML = '<ul>' + results.map(function(r) {
            var item = r.item;
            var tagHtml = (item.tags || []).map(function(t) {
              return '<span class="badge badge-tag" style="font-size:10px;padding:1px 5px">' + t + '</span>';
            }).join(' ');
            return '<li><a href="' + base + item.url + '">' + item.title +
              ' <span style="color:#6a737d;font-size:11px">' + item.type + '</span>' +
              (tagHtml ? ' ' + tagHtml : '') + '</a></li>';
          }).join('') + '</ul>';
        }, 200);
      });
    });

  // Tag pill click handler: filter sidebar by tag
  document.addEventListener('click', function(e) {
    if (e.target.classList.contains('badge-tag')) {
      var tag = e.target.textContent.trim();
      searchInput.value = tag;
      searchInput.dispatchEvent(new Event('input'));
    }
  });
})();
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

#[allow(clippy::too_many_arguments)]
pub fn write_html_output(
    output_dir: &Path,
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
    lwc_contexts: &[LwcRenderContext],
    flexipage_contexts: &[FlexiPageRenderContext],
    custom_metadata_contexts: &[CustomMetadataRenderContext],
    aura_contexts: &[AuraRenderContext],
) -> Result<()> {
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

    // (name, folder, tags) triples — used for sidebar grouping, cross-link generation, and tag filtering.
    let class_tag_strings: Vec<String> = class_contexts
        .iter()
        .map(|c| c.metadata.tags.join(","))
        .collect();
    let trigger_tag_strings: Vec<String> = trigger_contexts
        .iter()
        .map(|c| c.metadata.tags.join(","))
        .collect();
    let class_items: Vec<(&str, &str, &str)> = class_contexts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                c.metadata.class_name.as_str(),
                c.folder.as_str(),
                class_tag_strings[i].as_str(),
            )
        })
        .collect();
    let trigger_items: Vec<(&str, &str, &str)> = trigger_contexts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                c.metadata.trigger_name.as_str(),
                c.folder.as_str(),
                trigger_tag_strings[i].as_str(),
            )
        })
        .collect();
    let flow_items: Vec<(&str, &str, &str)> = flow_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let vr_items: Vec<(&str, &str, &str)> = validation_rule_contexts
        .iter()
        .map(|c| (c.metadata.rule_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let obj_items: Vec<(&str, &str, &str)> = object_contexts
        .iter()
        .map(|c| (c.metadata.object_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let lwc_items: Vec<(&str, &str, &str)> = lwc_contexts
        .iter()
        .map(|c| (c.metadata.component_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let flexipage_items: Vec<(&str, &str, &str)> = flexipage_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let aura_items: Vec<(&str, &str, &str)> = aura_contexts
        .iter()
        .map(|c| (c.metadata.component_name.as_str(), c.folder.as_str(), ""))
        .collect();

    for ctx in class_contexts {
        let page = render_class_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
            &lwc_items,
            &flexipage_items,
            &aura_items,
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
            &lwc_items,
            &flexipage_items,
            &aura_items,
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
            &lwc_items,
            &flexipage_items,
            &aura_items,
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
            &lwc_items,
            &flexipage_items,
            &aura_items,
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
            &lwc_items,
            &flexipage_items,
            &aura_items,
        );
        std::fs::write(
            objects_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.object_name)
            )),
            page,
        )?;
    }

    for ctx in lwc_contexts {
        let page = render_lwc_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
            &lwc_items,
            &flexipage_items,
            &aura_items,
        );
        std::fs::write(
            lwc_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.component_name)
            )),
            page,
        )?;
    }

    for ctx in flexipage_contexts {
        let page = render_flexipage_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
            &lwc_items,
            &flexipage_items,
            &aura_items,
        );
        std::fs::write(
            flexipages_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.api_name)
            )),
            page,
        )?;
    }

    for ctx in custom_metadata_contexts {
        let page = render_custom_metadata_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
            &lwc_items,
            &flexipage_items,
            &aura_items,
        );
        std::fs::write(
            custom_metadata_dir.join(format!("{}.html", sanitize_filename(&ctx.type_name))),
            page,
        )?;
    }

    for ctx in aura_contexts {
        let page = render_aura_page(
            ctx,
            &class_items,
            &trigger_items,
            &flow_items,
            &vr_items,
            &obj_items,
            &lwc_items,
            &flexipage_items,
            &aura_items,
        );
        std::fs::write(
            aura_dir.join(format!(
                "{}.html",
                sanitize_filename(&ctx.metadata.component_name)
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
        lwc_contexts,
        flexipage_contexts,
        custom_metadata_contexts,
        aura_contexts,
    );
    std::fs::write(output_dir.join("index.html"), index)?;

    // Write search assets
    let search_index = generate_search_index(
        class_contexts,
        trigger_contexts,
        flow_contexts,
        validation_rule_contexts,
        object_contexts,
        lwc_contexts,
        flexipage_contexts,
        custom_metadata_contexts,
        aura_contexts,
    );
    std::fs::write(output_dir.join("search-index.json"), search_index)?;
    std::fs::write(
        output_dir.join("search.js"),
        format!("{}\n{}", FUSE_JS, SEARCH_JS),
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Page renderers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_class_page(
    ctx: &RenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();
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
    if meta.is_interface {
        body.push_str("<span class=\"badge\">interface</span>\n");
    }
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
    for tag in &ctx.metadata.tags {
        body.push_str(&format!(
            "<span class=\"badge badge-tag\">{}</span>",
            escape(tag)
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
                        "<a href=\"{}.html\">{}</a> — {}",
                        escape(name),
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
                                "<a href=\"../triggers/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../flows/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../validation-rules/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../objects/{}.html\">{}</a> — {}",
                                escape(name),
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    lwc_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../lwc/{}.html\">{}</a> — {}",
                                escape(name),
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

    // Implemented By (for interfaces)
    if meta.is_interface {
        if let Some(implementors) = ctx.all_names.interface_implementors.get(&meta.class_name) {
            if !implementors.is_empty() {
                body.push_str("<h2>Implemented By</h2>\n<ul>\n");
                for cls in implementors {
                    body.push_str(&format!(
                        "<li><a href=\"{}.html\">{}</a></li>\n",
                        escape(cls),
                        escape(cls)
                    ));
                }
                body.push_str("</ul>\n");
            }
        }
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
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_trigger_page(
    ctx: &TriggerRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();
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
    for tag in &ctx.metadata.tags {
        body.push_str(&format!(
            "<span class=\"badge badge-tag\">{}</span>",
            escape(tag)
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
                        "<a href=\"../classes/{}.html\">{}</a> — {}",
                        escape(name),
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
                                "<a href=\"{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../flows/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../validation-rules/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../objects/{}.html\">{}</a> — {}",
                                escape(name),
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    lwc_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../lwc/{}.html\">{}</a> — {}",
                                escape(name),
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
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_flow_page(
    ctx: &FlowRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();
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
                        "<a href=\"../classes/{}.html\">{}</a> — {}",
                        escape(name),
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
                                "<a href=\"../triggers/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../validation-rules/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../objects/{}.html\">{}</a> — {}",
                                escape(name),
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    lwc_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../lwc/{}.html\">{}</a> — {}",
                                escape(name),
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
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_validation_rule_page(
    ctx: &ValidationRuleRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();
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
                        "<a href=\"../classes/{}.html\">{}</a> — {}",
                        escape(name),
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
                                "<a href=\"../triggers/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../flows/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../objects/{}.html\">{}</a> — {}",
                                escape(name),
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    lwc_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../lwc/{}.html\">{}</a> — {}",
                                escape(name),
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
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_object_page(
    ctx: &ObjectRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();
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
                        "<a href=\"../classes/{}.html\">{}</a> — {}",
                        escape(name),
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
                                "<a href=\"../triggers/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../flows/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"../validation-rules/{}.html\">{}</a> — {}",
                                escape(name),
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
                                "<a href=\"{}.html\">{}</a> — {}",
                                escape(name),
                                escape(name),
                                escape(rel)
                            )
                        })
                })
                .or_else(|| {
                    lwc_names
                        .iter()
                        .find(|&&name| rel.contains(name))
                        .map(|&name| {
                            format!(
                                "<a href=\"../lwc/{}.html\">{}</a> — {}",
                                escape(name),
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
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_lwc_page(
    ctx: &LwcRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let class_names: Vec<&str> = class_items.iter().map(|&(n, _, _)| n).collect();
    let trigger_names: Vec<&str> = trigger_items.iter().map(|&(n, _, _)| n).collect();
    let flow_names: Vec<&str> = flow_items.iter().map(|&(n, _, _)| n).collect();
    let vr_names: Vec<&str> = vr_items.iter().map(|&(n, _, _)| n).collect();
    let obj_names: Vec<&str> = obj_items.iter().map(|&(n, _, _)| n).collect();
    let lwc_names: Vec<&str> = lwc_items.iter().map(|&(n, _, _)| n).collect();

    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.component_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&meta.component_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">lwc</span>\n");
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.api_props.is_empty() {
        body.push_str("<h2>Public API</h2>\n");
        body.push_str("<table><thead><tr><th>Name</th><th>Kind</th><th>Description</th></tr></thead><tbody>\n");
        for prop_doc in &doc.api_props {
            let kind = meta
                .api_props
                .iter()
                .find(|p| p.name == prop_doc.name)
                .map(|p| if p.is_method { "method" } else { "property" })
                .unwrap_or("property");
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>\n",
                escape(&prop_doc.name),
                escape(kind),
                escape(&prop_doc.description),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !meta.slots.is_empty() {
        body.push_str("<h2>Slots</h2>\n");
        body.push_str("<table><thead><tr><th>Slot</th></tr></thead><tbody>\n");
        for slot in &meta.slots {
            let label = if slot == "default" {
                "<em>(default)</em>".to_string()
            } else {
                format!("<code>{}</code>", escape(slot))
            };
            body.push_str(&format!("<tr><td>{label}</td></tr>\n"));
        }
        body.push_str("</tbody></table>\n");
    }

    if !doc.usage_notes.is_empty() {
        body.push_str("<h2>Usage Notes</h2>\n");
        body.push_str("<ul>\n");
        for note in &doc.usage_notes {
            body.push_str(&format!("<li>{}</li>\n", escape(note)));
        }
        body.push_str("</ul>\n");
    }

    if !doc.relationships.is_empty() {
        body.push_str("<h2>See Also</h2>\n");
        body.push_str("<ul>\n");
        for rel in &doc.relationships {
            let linked =
                class_names
                    .iter()
                    .find(|&&name| rel.contains(name))
                    .and_then(|&name| {
                        class_items
                            .iter()
                            .find(|&&(n, _, _)| n == name)
                            .map(|&(_, folder, _)| {
                                format!(
                                    "<a href=\"../classes/{}/{}.html\">{}</a>",
                                    escape(folder),
                                    escape(name),
                                    escape(rel)
                                )
                            })
                    })
                    .or_else(|| {
                        trigger_names
                            .iter()
                            .find(|&&name| rel.contains(name))
                            .and_then(|&name| {
                                trigger_items.iter().find(|&&(n, _, _)| n == name).map(
                                    |&(_, folder, _)| {
                                        format!(
                                            "<a href=\"../triggers/{}/{}.html\">{}</a>",
                                            escape(folder),
                                            escape(name),
                                            escape(rel)
                                        )
                                    },
                                )
                            })
                    })
                    .or_else(|| {
                        flow_names
                            .iter()
                            .find(|&&name| rel.contains(name))
                            .and_then(|&name| {
                                flow_items.iter().find(|&&(n, _, _)| n == name).map(
                                    |&(_, folder, _)| {
                                        format!(
                                            "<a href=\"../flows/{}/{}.html\">{}</a>",
                                            escape(folder),
                                            escape(name),
                                            escape(rel)
                                        )
                                    },
                                )
                            })
                    })
                    .or_else(|| {
                        vr_names
                            .iter()
                            .find(|&&name| rel.contains(name))
                            .and_then(|&name| {
                                vr_items.iter().find(|&&(n, _, _)| n == name).map(
                                    |&(_, folder, _)| {
                                        format!(
                                            "<a href=\"../validation-rules/{}/{}.html\">{}</a>",
                                            escape(folder),
                                            escape(name),
                                            escape(rel)
                                        )
                                    },
                                )
                            })
                    })
                    .or_else(|| {
                        obj_names
                            .iter()
                            .find(|&&name| rel.contains(name))
                            .and_then(|&name| {
                                obj_items.iter().find(|&&(n, _, _)| n == name).map(
                                    |&(_, folder, _)| {
                                        format!(
                                            "<a href=\"../objects/{}/{}.html\">{}</a>",
                                            escape(folder),
                                            escape(name),
                                            escape(rel)
                                        )
                                    },
                                )
                            })
                    })
                    .or_else(|| {
                        lwc_names
                            .iter()
                            .find(|&&name| rel.contains(name))
                            .and_then(|&name| {
                                lwc_items.iter().find(|&&(n, _, _)| n == name).map(
                                    |&(_, folder, _)| {
                                        format!(
                                            "<a href=\"../lwc/{}/{}.html\">{}</a>",
                                            escape(folder),
                                            escape(name),
                                            escape(rel)
                                        )
                                    },
                                )
                            })
                    })
                    .unwrap_or_else(|| escape(rel));
            body.push_str(&format!("<li>{linked}</li>\n"));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &meta.component_name,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_flexipage_page(
    ctx: &FlexiPageRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.api_name;

    let mut body = String::new();

    let title = if !meta.label.is_empty() {
        meta.label.as_str()
    } else {
        meta.api_name.as_str()
    };

    body.push_str(&format!("<h1>{}</h1>\n", escape(title)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">lightning-page</span>\n");
    if !meta.page_type.is_empty() {
        body.push_str(&format!(
            "<span class=\"badge\">{}</span>\n",
            escape(&meta.page_type)
        ));
    }
    if !meta.sobject.is_empty() {
        body.push_str(&format!(
            "<span class=\"badge\">on {}</span>\n",
            escape(&meta.sobject)
        ));
    }
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.usage_context.is_empty() {
        body.push_str("<h2>Usage Context</h2>\n");
        body.push_str(&format!("<p>{}</p>\n", escape(&doc.usage_context)));
    }

    if !meta.component_names.is_empty() {
        body.push_str("<h2>Components</h2>\n<ul>\n");
        for comp in &meta.component_names {
            let name_cell = if lwc_items.iter().any(|&(n, _, _)| n == comp.as_str()) {
                format!(
                    "<a href=\"../lwc/{}.html\"><code>{}</code></a>",
                    escape(comp),
                    escape(comp)
                )
            } else if aura_items.iter().any(|&(n, _, _)| n == comp.as_str()) {
                format!(
                    "<a href=\"../aura/{}.html\"><code>{}</code></a>",
                    escape(comp),
                    escape(comp)
                )
            } else {
                format!("<code>{}</code>", escape(comp))
            };
            body.push_str(&format!("<li>{}</li>\n", name_cell));
        }
        body.push_str("</ul>\n");
    }

    if !meta.flow_names.is_empty() {
        body.push_str("<h2>Flows</h2>\n<ul>\n");
        for flow in &meta.flow_names {
            let name_cell = if flow_items.iter().any(|&(n, _, _)| n == flow.as_str()) {
                format!(
                    "<a href=\"../flows/{}.html\"><code>{}</code></a>",
                    escape(flow),
                    escape(flow)
                )
            } else {
                format!("<code>{}</code>", escape(flow))
            };
            body.push_str(&format!("<li>{}</li>\n", name_cell));
        }
        body.push_str("</ul>\n");
    }

    if !doc.key_components.is_empty() {
        body.push_str("<h2>Key Components</h2>\n<ul>\n");
        for comp in &doc.key_components {
            body.push_str(&format!("<li>{}</li>\n", escape(comp)));
        }
        body.push_str("</ul>\n");
    }

    if !doc.relationships.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for rel in &doc.relationships {
            body.push_str(&format!("<li>{}</li>\n", escape(rel)));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        title,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_custom_metadata_page(
    ctx: &CustomMetadataRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&ctx.type_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">custom-metadata-type</span>\n");
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p class=\"summary\">{} record(s) defined.</p>\n",
        ctx.records.len()
    ));

    if !ctx.records.is_empty() {
        // Collect all unique field names across records
        let mut field_names: Vec<String> = Vec::new();
        for record in &ctx.records {
            for (field, _) in &record.values {
                if !field_names.contains(field) {
                    field_names.push(field.clone());
                }
            }
        }
        field_names.sort();

        body.push_str("<h2>Records</h2>\n");
        body.push_str("<table><thead><tr><th>Record</th><th>Label</th>");
        for field in &field_names {
            body.push_str(&format!("<th>{}</th>", escape(field)));
        }
        body.push_str("</tr></thead><tbody>\n");

        let mut sorted_records: Vec<_> = ctx.records.iter().collect();
        sorted_records.sort_by(|a, b| a.record_name.cmp(&b.record_name));

        for record in sorted_records {
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td>",
                escape(&record.record_name),
                escape(&record.label)
            ));
            for field in &field_names {
                let val = record
                    .values
                    .iter()
                    .find(|(f, _)| f == field)
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                body.push_str(&format!("<td>{}</td>", escape(val)));
            }
            body.push_str("</tr>\n");
        }
        body.push_str("</tbody></table>\n");
    }

    wrap_page(
        &ctx.type_name,
        "sfdoc",
        &body,
        &ctx.type_name,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_aura_page(
    ctx: &AuraRenderContext,
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let doc = &ctx.documentation;
    let meta = &ctx.metadata;
    let active = &meta.component_name;

    let mut body = String::new();

    body.push_str(&format!("<h1>{}</h1>\n", escape(&meta.component_name)));
    body.push_str("<div class=\"badges\">\n");
    body.push_str("<span class=\"badge\">aura</span>\n");
    if let Some(ref ext) = meta.extends {
        body.push_str(&format!(
            "<span class=\"badge\">extends {}</span>\n",
            escape(ext)
        ));
    }
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p class=\"summary\">{}</p>\n",
        escape(&doc.summary)
    ));

    body.push_str("<h2>Description</h2>\n");
    body.push_str(&format!("<p>{}</p>\n", escape(&doc.description)));

    if !doc.attributes.is_empty() {
        body.push_str("<h2>Attributes</h2>\n");
        body.push_str("<table><thead><tr><th>Name</th><th>Type</th><th>Default</th><th>Description</th></tr></thead><tbody>\n");
        for attr_doc in &doc.attributes {
            let (attr_type, default_val) = meta
                .attributes
                .iter()
                .find(|a| a.name == attr_doc.name)
                .map(|a| (a.attr_type.as_str(), a.default.as_str()))
                .unwrap_or(("", ""));
            body.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>\n",
                escape(&attr_doc.name),
                escape(attr_type),
                escape(default_val),
                escape(&attr_doc.description)
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !meta.events_handled.is_empty() {
        body.push_str("<h2>Events Handled</h2>\n<ul>\n");
        for event in &meta.events_handled {
            body.push_str(&format!("<li><code>{}</code></li>\n", escape(event)));
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

    if !doc.relationships.is_empty() {
        body.push_str("<h2>See Also</h2>\n<ul>\n");
        for rel in &doc.relationships {
            body.push_str(&format!("<li>{}</li>\n", escape(rel)));
        }
        body.push_str("</ul>\n");
    }

    wrap_page(
        &meta.component_name,
        "sfdoc",
        &body,
        active,
        "../",
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        lwc_items,
        flexipage_items,
        aura_items,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
    lwc_contexts: &[LwcRenderContext],
    flexipage_contexts: &[FlexiPageRenderContext],
    custom_metadata_contexts: &[CustomMetadataRenderContext],
    aura_contexts: &[AuraRenderContext],
) -> String {
    let class_tag_strings: Vec<String> = class_contexts
        .iter()
        .map(|c| c.metadata.tags.join(","))
        .collect();
    let trigger_tag_strings: Vec<String> = trigger_contexts
        .iter()
        .map(|c| c.metadata.tags.join(","))
        .collect();
    let class_items: Vec<(&str, &str, &str)> = class_contexts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                c.metadata.class_name.as_str(),
                c.folder.as_str(),
                class_tag_strings[i].as_str(),
            )
        })
        .collect();
    let trigger_items: Vec<(&str, &str, &str)> = trigger_contexts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                c.metadata.trigger_name.as_str(),
                c.folder.as_str(),
                trigger_tag_strings[i].as_str(),
            )
        })
        .collect();
    let flow_items: Vec<(&str, &str, &str)> = flow_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let vr_items: Vec<(&str, &str, &str)> = validation_rule_contexts
        .iter()
        .map(|c| (c.metadata.rule_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let obj_items: Vec<(&str, &str, &str)> = object_contexts
        .iter()
        .map(|c| (c.metadata.object_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let lwc_items: Vec<(&str, &str, &str)> = lwc_contexts
        .iter()
        .map(|c| (c.metadata.component_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let flexipage_items: Vec<(&str, &str, &str)> = flexipage_contexts
        .iter()
        .map(|c| (c.metadata.api_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let aura_items: Vec<(&str, &str, &str)> = aura_contexts
        .iter()
        .map(|c| (c.metadata.component_name.as_str(), c.folder.as_str(), ""))
        .collect();
    let mut body = String::new();
    body.push_str("<h1>Salesforce Documentation</h1>\n");
    let interface_count = class_contexts
        .iter()
        .filter(|c| c.metadata.is_interface)
        .count();
    let class_count = class_contexts.len() - interface_count;
    body.push_str(&format!(
        "<p class=\"summary\">Generated documentation for {} class(es), {} interface(s), {} trigger(s), {} flow(s), {} validation rule(s), {} object(s), {} LWC component(s), {} Lightning page(s), {} custom metadata type(s), and {} Aura component(s).</p>\n",
        class_count,
        interface_count,
        trigger_contexts.len(),
        flow_contexts.len(),
        validation_rule_contexts.len(),
        object_contexts.len(),
        lwc_contexts.len(),
        flexipage_contexts.len(),
        custom_metadata_contexts.len(),
        aura_contexts.len(),
    ));

    // Partition classes and interfaces
    let concrete_classes: Vec<&RenderContext> = class_contexts
        .iter()
        .filter(|c| !c.metadata.is_interface)
        .collect();
    let interfaces: Vec<&RenderContext> = class_contexts
        .iter()
        .filter(|c| c.metadata.is_interface)
        .collect();

    if !concrete_classes.is_empty() {
        // Group classes by folder.
        let mut class_by_folder: BTreeMap<&str, Vec<&&RenderContext>> = BTreeMap::new();
        for ctx in &concrete_classes {
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
                let tag_html: String = ctx
                    .metadata
                    .tags
                    .iter()
                    .map(|t| {
                        format!(
                            " <span class=\"badge badge-tag\" style=\"font-size:10px\">{}</span>",
                            escape(t)
                        )
                    })
                    .collect();
                body.push_str(&format!(
                    "<tr><td><a href=\"classes/{}.html\">{}</a></td><td>{}{}</td></tr>\n",
                    escape(&ctx.metadata.class_name),
                    escape(&ctx.documentation.class_name),
                    escape(&ctx.documentation.summary),
                    tag_html,
                ));
            }
            body.push_str("</tbody></table>\n");
        }
    }

    if !interfaces.is_empty() {
        let mut iface_by_folder: BTreeMap<&str, Vec<&&RenderContext>> = BTreeMap::new();
        for ctx in &interfaces {
            iface_by_folder
                .entry(ctx.folder.as_str())
                .or_default()
                .push(ctx);
        }
        for group in iface_by_folder.values_mut() {
            group.sort_by(|a, b| a.documentation.class_name.cmp(&b.documentation.class_name));
        }

        body.push_str("<h2>Interfaces</h2>\n");
        let multi_folder = iface_by_folder.len() > 1;
        for (folder, ifaces) in &iface_by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                body.push_str(&format!("<h3>{}</h3>\n", escape(label)));
            }
            body.push_str(
                "<table><thead><tr><th>Interface</th><th>Summary</th></tr></thead><tbody>\n",
            );
            for ctx in ifaces {
                let tag_html: String = ctx
                    .metadata
                    .tags
                    .iter()
                    .map(|t| {
                        format!(
                            " <span class=\"badge badge-tag\" style=\"font-size:10px\">{}</span>",
                            escape(t)
                        )
                    })
                    .collect();
                body.push_str(&format!(
                    "<tr><td><a href=\"classes/{}.html\">{}</a></td><td>{}{}</td></tr>\n",
                    escape(&ctx.metadata.class_name),
                    escape(&ctx.documentation.class_name),
                    escape(&ctx.documentation.summary),
                    tag_html,
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
                let tag_html: String = ctx
                    .metadata
                    .tags
                    .iter()
                    .map(|t| {
                        format!(
                            " <span class=\"badge badge-tag\" style=\"font-size:10px\">{}</span>",
                            escape(t)
                        )
                    })
                    .collect();
                body.push_str(&format!(
                    "<tr><td><a href=\"triggers/{}.html\">{}</a></td><td><code>{}</code></td><td>{}{}</td></tr>\n",
                    escape(&ctx.metadata.trigger_name),
                    escape(&ctx.documentation.trigger_name),
                    escape(&ctx.documentation.sobject),
                    escape(&ctx.documentation.summary),
                    tag_html,
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

    if !lwc_contexts.is_empty() {
        let mut lwc_sorted: Vec<&LwcRenderContext> = lwc_contexts.iter().collect();
        lwc_sorted.sort_by(|a, b| a.metadata.component_name.cmp(&b.metadata.component_name));

        body.push_str("<h2>Lightning Web Components</h2>\n");
        body.push_str("<table><thead><tr><th>Component</th><th>@api Props</th><th>Summary</th></tr></thead><tbody>\n");
        for ctx in lwc_sorted {
            body.push_str(&format!(
                "<tr><td><a href=\"lwc/{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
                escape(&ctx.metadata.component_name),
                escape(&ctx.metadata.component_name),
                ctx.metadata.api_props.len(),
                escape(&ctx.documentation.summary),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !flexipage_contexts.is_empty() {
        let mut fp_sorted: Vec<&FlexiPageRenderContext> = flexipage_contexts.iter().collect();
        fp_sorted.sort_by(|a, b| a.metadata.api_name.cmp(&b.metadata.api_name));

        body.push_str("<h2>Lightning Pages</h2>\n");
        body.push_str(
            "<table><thead><tr><th>Page</th><th>Type</th><th>Summary</th></tr></thead><tbody>\n",
        );
        for ctx in fp_sorted {
            let label = if !ctx.metadata.label.is_empty() {
                ctx.metadata.label.as_str()
            } else {
                ctx.metadata.api_name.as_str()
            };
            body.push_str(&format!(
                "<tr><td><a href=\"flexipages/{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td></tr>\n",
                escape(&ctx.metadata.api_name),
                escape(label),
                escape(&ctx.metadata.page_type),
                escape(&ctx.documentation.summary),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !custom_metadata_contexts.is_empty() {
        let mut cm_sorted: Vec<&CustomMetadataRenderContext> =
            custom_metadata_contexts.iter().collect();
        cm_sorted.sort_by(|a, b| a.type_name.cmp(&b.type_name));

        body.push_str("<h2>Custom Metadata Types</h2>\n");
        body.push_str("<table><thead><tr><th>Type</th><th>Records</th></tr></thead><tbody>\n");
        for ctx in cm_sorted {
            body.push_str(&format!(
                "<tr><td><a href=\"custom-metadata/{}.html\"><code>{}</code></a></td><td>{}</td></tr>\n",
                escape(&ctx.type_name),
                escape(&ctx.type_name),
                ctx.records.len(),
            ));
        }
        body.push_str("</tbody></table>\n");
    }

    if !aura_contexts.is_empty() {
        let mut aura_sorted: Vec<&AuraRenderContext> = aura_contexts.iter().collect();
        aura_sorted.sort_by(|a, b| a.metadata.component_name.cmp(&b.metadata.component_name));

        body.push_str("<h2>Aura Components</h2>\n");
        body.push_str("<table><thead><tr><th>Component</th><th>Attributes</th><th>Summary</th></tr></thead><tbody>\n");
        for ctx in aura_sorted {
            body.push_str(&format!(
                "<tr><td><a href=\"aura/{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
                escape(&ctx.metadata.component_name),
                escape(&ctx.metadata.component_name),
                ctx.metadata.attributes.len(),
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
        &lwc_items,
        &flexipage_items,
        &aura_items,
    )
}

// ---------------------------------------------------------------------------
// Search index
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn generate_search_index(
    class_contexts: &[RenderContext],
    trigger_contexts: &[TriggerRenderContext],
    flow_contexts: &[FlowRenderContext],
    validation_rule_contexts: &[ValidationRuleRenderContext],
    object_contexts: &[ObjectRenderContext],
    lwc_contexts: &[LwcRenderContext],
    flexipage_contexts: &[FlexiPageRenderContext],
    custom_metadata_contexts: &[CustomMetadataRenderContext],
    aura_contexts: &[AuraRenderContext],
) -> String {
    let mut entries = Vec::new();

    for ctx in class_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.class_name,
            "type": if ctx.metadata.is_interface { "interface" } else { "class" },
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("classes/{}.html", sanitize_filename(&ctx.metadata.class_name)),
            "tags": ctx.metadata.tags,
        }));
    }
    for ctx in trigger_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.trigger_name,
            "type": "trigger",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("triggers/{}.html", sanitize_filename(&ctx.metadata.trigger_name)),
            "tags": ctx.metadata.tags,
        }));
    }
    for ctx in flow_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.label,
            "type": "flow",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("flows/{}.html", sanitize_filename(&ctx.metadata.api_name)),
            "tags": [],
        }));
    }
    for ctx in validation_rule_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.rule_name,
            "type": "validation-rule",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("validation-rules/{}.html", sanitize_filename(&ctx.metadata.rule_name)),
            "tags": [],
        }));
    }
    for ctx in object_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.object_name,
            "type": "object",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("objects/{}.html", sanitize_filename(&ctx.metadata.object_name)),
            "tags": [],
        }));
    }
    for ctx in lwc_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.component_name,
            "type": "lwc",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("lwc/{}.html", sanitize_filename(&ctx.metadata.component_name)),
            "tags": [],
        }));
    }
    for ctx in flexipage_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.label,
            "type": "flexipage",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("flexipages/{}.html", sanitize_filename(&ctx.metadata.api_name)),
            "tags": [],
        }));
    }
    for ctx in custom_metadata_contexts {
        entries.push(serde_json::json!({
            "title": ctx.type_name,
            "type": "custom-metadata",
            "folder": "",
            "summary": format!("{} records", ctx.records.len()),
            "url": format!("custom-metadata/{}.html", sanitize_filename(&ctx.type_name)),
            "tags": [],
        }));
    }
    for ctx in aura_contexts {
        entries.push(serde_json::json!({
            "title": ctx.documentation.component_name,
            "type": "aura",
            "folder": ctx.folder,
            "summary": ctx.documentation.summary,
            "url": format!("aura/{}.html", sanitize_filename(&ctx.metadata.component_name)),
            "tags": [],
        }));
    }

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
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
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
) -> String {
    let sidebar = render_sidebar(
        class_items,
        trigger_items,
        flow_items,
        vr_items,
        obj_items,
        lwc_items,
        flexipage_items,
        aura_items,
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
<script src="{up_prefix}search.js"></script>
</body>
</html>
"#,
        title = escape(title),
        brand = escape(brand),
    )
}

#[allow(clippy::too_many_arguments)]
fn render_sidebar(
    class_items: &[(&str, &str, &str)],
    trigger_items: &[(&str, &str, &str)],
    flow_items: &[(&str, &str, &str)],
    vr_items: &[(&str, &str, &str)],
    obj_items: &[(&str, &str, &str)],
    lwc_items: &[(&str, &str, &str)],
    flexipage_items: &[(&str, &str, &str)],
    aura_items: &[(&str, &str, &str)],
    active: &str,
    up_prefix: &str,
) -> String {
    let mut s = String::new();
    s.push_str("<nav class=\"sidebar\">\n");
    s.push_str(&format!(
        "<a class=\"sidebar-brand\" href=\"{up_prefix}index.html\">sfdoc</a>\n"
    ));
    s.push_str("<div class=\"sidebar-search\"><input type=\"text\" id=\"sfdoc-search\" placeholder=\"Search...\" autocomplete=\"off\"></div>\n");
    s.push_str("<div id=\"sfdoc-search-results\"></div>\n");

    if !class_items.is_empty() {
        // Group by folder (BTreeMap gives alphabetical folder order).
        let mut by_folder: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for &(name, folder, tags) in class_items {
            by_folder.entry(folder).or_default().push((name, tags));
        }
        for entries in by_folder.values_mut() {
            entries.sort_unstable_by_key(|&(name, _)| name);
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Classes</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, entries) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for &(name, tags) in entries {
                let cls = if name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                let tag_attr = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" data-tags=\"{}\"", escape(tags))
                };
                s.push_str(&format!(
                    "<li{tag_attr}><a href=\"{up_prefix}classes/{}.html\"{cls}>{}</a></li>\n",
                    escape(name),
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !trigger_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for &(name, folder, tags) in trigger_items {
            by_folder.entry(folder).or_default().push((name, tags));
        }
        for entries in by_folder.values_mut() {
            entries.sort_unstable_by_key(|&(name, _)| name);
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Triggers</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, entries) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for &(name, tags) in entries {
                let cls = if name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                let tag_attr = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" data-tags=\"{}\"", escape(tags))
                };
                s.push_str(&format!(
                    "<li{tag_attr}><a href=\"{up_prefix}triggers/{}.html\"{cls}>{}</a></li>\n",
                    escape(name),
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !flow_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for &(name, folder, tags) in flow_items {
            by_folder.entry(folder).or_default().push((name, tags));
        }
        for entries in by_folder.values_mut() {
            entries.sort_unstable_by_key(|&(name, _)| name);
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Flows</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, entries) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for &(name, tags) in entries {
                let cls = if name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                let tag_attr = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" data-tags=\"{}\"", escape(tags))
                };
                s.push_str(&format!(
                    "<li{tag_attr}><a href=\"{up_prefix}flows/{}.html\"{cls}>{}</a></li>\n",
                    escape(name),
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !vr_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for &(name, folder, tags) in vr_items {
            by_folder.entry(folder).or_default().push((name, tags));
        }
        for entries in by_folder.values_mut() {
            entries.sort_unstable_by_key(|&(name, _)| name);
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Validation Rules</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, entries) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for &(name, tags) in entries {
                let cls = if name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                let tag_attr = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" data-tags=\"{}\"", escape(tags))
                };
                s.push_str(&format!(
                    "<li{tag_attr}><a href=\"{up_prefix}validation-rules/{}.html\"{cls}>{}</a></li>\n",
                    escape(name),
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !obj_items.is_empty() {
        let mut entries: Vec<(&str, &str)> =
            obj_items.iter().map(|&(n, _, tags)| (n, tags)).collect();
        entries.sort_unstable_by_key(|&(name, _)| name);

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Objects</div>\n");
        s.push_str("<ul>\n");
        for (name, tags) in &entries {
            let cls = if *name == active {
                " class=\"active\""
            } else {
                ""
            };
            let tag_attr = if tags.is_empty() {
                String::new()
            } else {
                format!(" data-tags=\"{}\"", escape(tags))
            };
            s.push_str(&format!(
                "<li{tag_attr}><a href=\"{up_prefix}objects/{}.html\"{cls}>{}</a></li>\n",
                escape(name),
                escape(name)
            ));
        }
        s.push_str("</ul>\n");
        s.push_str("</div>\n");
    }

    if !lwc_items.is_empty() {
        let mut by_folder: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for &(name, folder, tags) in lwc_items {
            by_folder.entry(folder).or_default().push((name, tags));
        }
        for entries in by_folder.values_mut() {
            entries.sort_unstable_by_key(|&(name, _)| name);
        }

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">LWC</div>\n");
        let multi_folder = by_folder.len() > 1;
        for (folder, entries) in &by_folder {
            if multi_folder {
                let label = if folder.is_empty() { "(root)" } else { folder };
                s.push_str(&format!(
                    "<div class=\"sidebar-folder\">{}</div>\n",
                    escape(label)
                ));
            }
            s.push_str("<ul>\n");
            for &(name, tags) in entries {
                let cls = if name == active {
                    " class=\"active\""
                } else {
                    ""
                };
                let tag_attr = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" data-tags=\"{}\"", escape(tags))
                };
                s.push_str(&format!(
                    "<li{tag_attr}><a href=\"{up_prefix}lwc/{}.html\"{cls}>{}</a></li>\n",
                    escape(name),
                    escape(name)
                ));
            }
            s.push_str("</ul>\n");
        }
        s.push_str("</div>\n");
    }

    if !flexipage_items.is_empty() {
        let mut entries: Vec<(&str, &str)> = flexipage_items
            .iter()
            .map(|&(n, _, tags)| (n, tags))
            .collect();
        entries.sort_unstable_by_key(|&(name, _)| name);

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Lightning Pages</div>\n");
        s.push_str("<ul>\n");
        for (name, tags) in &entries {
            let cls = if *name == active {
                " class=\"active\""
            } else {
                ""
            };
            let tag_attr = if tags.is_empty() {
                String::new()
            } else {
                format!(" data-tags=\"{}\"", escape(tags))
            };
            s.push_str(&format!(
                "<li{tag_attr}><a href=\"{up_prefix}flexipages/{}.html\"{cls}>{}</a></li>\n",
                escape(name),
                escape(name)
            ));
        }
        s.push_str("</ul>\n");
        s.push_str("</div>\n");
    }

    if !aura_items.is_empty() {
        let mut entries: Vec<(&str, &str)> =
            aura_items.iter().map(|&(n, _, tags)| (n, tags)).collect();
        entries.sort_unstable_by_key(|&(name, _)| name);

        s.push_str("<div class=\"sidebar-section\">\n");
        s.push_str("<div class=\"sidebar-heading\">Aura</div>\n");
        s.push_str("<ul>\n");
        for (name, tags) in &entries {
            let cls = if *name == active {
                " class=\"active\""
            } else {
                ""
            };
            let tag_attr = if tags.is_empty() {
                String::new()
            } else {
                format!(" data-tags=\"{}\"", escape(tags))
            };
            s.push_str(&format!(
                "<li{tag_attr}><a href=\"{up_prefix}aura/{}.html\"{cls}>{}</a></li>\n",
                escape(name),
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

/// Returns compiled keyword regexes paired with their replacement strings, compiled once.
fn keyword_highlighters() -> &'static Vec<(regex::Regex, String)> {
    static HIGHLIGHTERS: OnceLock<Vec<(regex::Regex, String)>> = OnceLock::new();
    HIGHLIGHTERS.get_or_init(|| {
        APEX_KEYWORDS
            .iter()
            .filter_map(|&kw| {
                regex::Regex::new(&format!(r"\b{kw}\b"))
                    .ok()
                    .map(|re| (re, format!("<span class=\"kw\">{kw}</span>")))
            })
            .collect()
    })
}

/// Wrap Apex keywords in `<span class="kw">` for syntax highlighting.
/// Input must already be HTML-escaped.
fn highlight_apex(source: &str) -> String {
    let mut result = escape(source);
    for (re, replacement) in keyword_highlighters() {
        result = re.replace_all(&result, replacement.as_str()).into_owned();
    }
    result
}
