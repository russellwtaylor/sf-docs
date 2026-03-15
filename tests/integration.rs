/// End-to-end integration tests for the sfdoc pipeline.
///
/// These tests exercise the full stack — scan → parse → (mock AI) → render —
/// using fixture source files under `tests/fixtures/`.  HTTP calls to the AI
/// provider are intercepted by a local `httpmock` server so no real API key is
/// required.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use httpmock::prelude::*;
use sfdoc::cache::{self, Cache};
use sfdoc::flow_parser;
use sfdoc::lwc_parser;
use sfdoc::parser;
use sfdoc::renderer::{self, RenderContext, TriggerRenderContext};
use sfdoc::renderer::{
    FlowRenderContext, LwcRenderContext, ObjectRenderContext, ValidationRuleRenderContext,
};
use sfdoc::scanner::{ApexScanner, FileScanner, FlowScanner, LwcScanner, TriggerScanner};
use sfdoc::trigger_parser;
use sfdoc::types::{
    AllNames, ClassDocumentation, FlowDocumentation, LwcDocumentation, LwcPropDocumentation,
    MethodDocumentation, ObjectDocumentation, PropertyDocumentation, TriggerDocumentation,
    TriggerEventDocumentation, ValidationRuleDocumentation,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
}

fn class_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/classes"))
}

fn trigger_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/triggers"))
}

/// Build a minimal `ClassDocumentation` suitable for rendering tests.
fn stub_class_doc(class_name: &str) -> ClassDocumentation {
    ClassDocumentation {
        class_name: class_name.to_string(),
        summary: format!("Summary for {class_name}."),
        description: format!("Description for {class_name}."),
        methods: vec![MethodDocumentation {
            name: "exampleMethod".to_string(),
            description: "Does something useful.".to_string(),
            params: vec![],
            returns: "void".to_string(),
            throws: vec![],
        }],
        properties: vec![PropertyDocumentation {
            name: "exampleProp".to_string(),
            description: "An example property.".to_string(),
        }],
        usage_examples: vec!["```apex\n// use it\n```".to_string()],
        relationships: vec![],
    }
}

/// Build a minimal `TriggerDocumentation`.
fn stub_trigger_doc(trigger_name: &str, sobject: &str) -> TriggerDocumentation {
    TriggerDocumentation {
        trigger_name: trigger_name.to_string(),
        sobject: sobject.to_string(),
        summary: format!("Summary for {trigger_name}."),
        description: format!("Description for {trigger_name}."),
        events: vec![TriggerEventDocumentation {
            event: "before insert".to_string(),
            description: "Runs before insert.".to_string(),
        }],
        handler_classes: vec!["AccountService".to_string()],
        usage_notes: vec![],
        relationships: vec![],
    }
}

// ---------------------------------------------------------------------------
// Scanner tests
// ---------------------------------------------------------------------------

#[test]
fn scan_finds_fixture_classes() {
    let scanner = ApexScanner;
    let files = scanner.scan(class_fixtures_dir()).unwrap();
    // Two fixture .cls files: AccountService.cls and account/OrderService.cls
    assert_eq!(files.len(), 2, "expected 2 fixture class files");
    let names: Vec<&str> = files.iter().map(|f| f.filename.as_str()).collect();
    assert!(names.contains(&"AccountService.cls"));
    assert!(names.contains(&"OrderService.cls"));
}

#[test]
fn scan_finds_fixture_triggers() {
    let scanner = TriggerScanner;
    let files = scanner.scan(trigger_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1, "expected 1 fixture trigger file");
    assert_eq!(files[0].filename, "AccountTrigger.trigger");
}

#[test]
fn scan_does_not_mix_types() {
    // The class scanner must not pick up .trigger files and vice-versa.
    let all_fixtures = fixtures_dir();
    let class_files = ApexScanner.scan(all_fixtures).unwrap();
    let trigger_files = TriggerScanner.scan(all_fixtures).unwrap();

    for f in &class_files {
        assert!(
            f.filename.ends_with(".cls"),
            "class scanner returned non-.cls file: {}",
            f.filename
        );
    }
    for f in &trigger_files {
        assert!(
            f.filename.ends_with(".trigger"),
            "trigger scanner returned non-.trigger file: {}",
            f.filename
        );
    }
}

// ---------------------------------------------------------------------------
// Parser tests on fixture files
// ---------------------------------------------------------------------------

#[test]
fn parse_account_service_fixture() {
    let files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let account_service = files
        .iter()
        .find(|f| f.filename == "AccountService.cls")
        .expect("AccountService.cls not found");

    let meta = parser::parse_apex_class(&account_service.raw_source).unwrap();

    assert_eq!(meta.class_name, "AccountService");
    assert_eq!(meta.access_modifier, "public");
    assert!(!meta.is_abstract);
    assert!(!meta.is_virtual);
    // Should find at least processAccounts and getActiveAccounts
    let method_names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(
        method_names.contains(&"processAccounts"),
        "missing processAccounts"
    );
    assert!(
        method_names.contains(&"getActiveAccounts"),
        "missing getActiveAccounts"
    );
}

#[test]
fn parse_order_service_fixture() {
    let files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let order_service = files
        .iter()
        .find(|f| f.filename == "OrderService.cls")
        .expect("OrderService.cls not found");

    let meta = parser::parse_apex_class(&order_service.raw_source).unwrap();

    assert_eq!(meta.class_name, "OrderService");
    assert_eq!(meta.extends, Some("BaseService".to_string()));
    let method_names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(
        method_names.contains(&"getOrdersForAccount"),
        "missing getOrdersForAccount"
    );
    assert!(method_names.contains(&"cancelOrder"), "missing cancelOrder");
}

#[test]
fn parse_account_trigger_fixture() {
    let files = TriggerScanner.scan(trigger_fixtures_dir()).unwrap();
    let meta = trigger_parser::parse_apex_trigger(&files[0].raw_source).unwrap();

    assert_eq!(meta.trigger_name, "AccountTrigger");
    assert_eq!(meta.sobject, "Account");
    assert!(!meta.events.is_empty());
}

// ---------------------------------------------------------------------------
// Render pipeline — Markdown output
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_writes_markdown_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let trigger_files = TriggerScanner.scan(trigger_fixtures_dir()).unwrap();

    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();
    let trigger_meta: Vec<_> = trigger_files
        .iter()
        .map(|f| trigger_parser::parse_apex_trigger(&f.raw_source).unwrap())
        .collect();

    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: trigger_meta
            .iter()
            .map(|m| m.trigger_name.clone())
            .collect(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(file, meta)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(class_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            RenderContext {
                folder,
                metadata: meta.clone(),
                documentation: stub_class_doc(&meta.class_name),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    let trigger_contexts: Vec<TriggerRenderContext> = trigger_files
        .iter()
        .zip(trigger_meta.iter())
        .map(|(file, meta)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(trigger_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            TriggerRenderContext {
                folder,
                metadata: meta.clone(),
                documentation: stub_trigger_doc(&meta.trigger_name, &meta.sobject),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    renderer::write_output(
        output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &class_contexts,
        &trigger_contexts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    // Every class and trigger gets its own page
    assert!(
        output_dir.join("classes/AccountService.md").exists(),
        "classes/AccountService.md missing"
    );
    assert!(
        output_dir.join("classes/OrderService.md").exists(),
        "classes/OrderService.md missing"
    );
    assert!(
        output_dir.join("triggers/AccountTrigger.md").exists(),
        "triggers/AccountTrigger.md missing"
    );
    assert!(output_dir.join("index.md").exists(), "index.md missing");
}

#[test]
fn markdown_class_page_contains_expected_sections() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();
    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(_, meta)| RenderContext {
            folder: String::new(),
            metadata: meta.clone(),
            documentation: stub_class_doc(&meta.class_name),
            all_names: Arc::clone(&all_names),
        })
        .collect();

    renderer::write_output(
        output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &class_contexts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    let content = std::fs::read_to_string(output_dir.join("classes/AccountService.md")).unwrap();
    assert!(content.contains("# AccountService"), "missing title");
    assert!(
        content.contains("## Description"),
        "missing description section"
    );
    assert!(
        content.contains("Summary for AccountService"),
        "missing summary"
    );
}

#[test]
fn markdown_index_groups_by_folder() {
    // AccountService is in the root; OrderService is in "account/" subfolder.
    // With two distinct folders the index should show ### headings.
    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();

    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(file, meta)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(class_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            RenderContext {
                folder,
                metadata: meta.clone(),
                documentation: stub_class_doc(&meta.class_name),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    let index = renderer::render_index(&class_contexts, &[], &[], &[], &[], &[], &[], &[], &[]);

    // Both classes should be linked with type-prefixed paths
    assert!(index.contains("[AccountService](classes/AccountService.md)"));
    assert!(index.contains("[OrderService](classes/OrderService.md)"));

    // Multi-folder project → folder headings
    assert!(
        index.contains("### account"),
        "expected a '### account' heading in index:\n{index}"
    );
    // The root folder is labelled "(root)" when the root is empty string
    assert!(
        index.contains("### (root)"),
        "expected a '### (root)' heading in index:\n{index}"
    );
}

// ---------------------------------------------------------------------------
// Render pipeline — HTML output
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_writes_html_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();
    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });
    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(_, meta)| RenderContext {
            folder: String::new(),
            metadata: meta.clone(),
            documentation: stub_class_doc(&meta.class_name),
            all_names: Arc::clone(&all_names),
        })
        .collect();

    renderer::write_output(
        output_dir,
        &sfdoc::cli::OutputFormat::Html,
        &class_contexts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    assert!(output_dir.join("index.html").exists(), "index.html missing");
    assert!(
        output_dir.join("classes/AccountService.html").exists(),
        "classes/AccountService.html missing"
    );
    assert!(
        output_dir.join("classes/OrderService.html").exists(),
        "classes/OrderService.html missing"
    );
}

#[test]
fn html_page_contains_sidebar_and_content() {
    let tmp = tempfile::TempDir::new().unwrap();
    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();
    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });
    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(_, meta)| RenderContext {
            folder: String::new(),
            metadata: meta.clone(),
            documentation: stub_class_doc(&meta.class_name),
            all_names: Arc::clone(&all_names),
        })
        .collect();

    renderer::write_output(
        tmp.path(),
        &sfdoc::cli::OutputFormat::Html,
        &class_contexts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    let html = std::fs::read_to_string(tmp.path().join("classes/AccountService.html")).unwrap();
    assert!(html.contains("<nav"), "missing nav sidebar");
    assert!(
        html.contains("AccountService"),
        "missing class name in HTML"
    );
    assert!(
        html.contains("Description for AccountService"),
        "missing description in HTML"
    );
}

// ---------------------------------------------------------------------------
// Incremental cache
// ---------------------------------------------------------------------------

#[test]
fn cache_skips_unchanged_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let model = "test-model";

    // Pre-populate the cache for each fixture file with the current hash.
    let mut cache = Cache::default();
    for file in &class_files {
        let hash = cache::hash_source(&file.raw_source);
        let doc = stub_class_doc("AccountService");
        cache.update(file.path.to_string_lossy().into_owned(), hash, model, doc);
    }
    cache.save(output_dir).unwrap();

    // Load cache and check every file is considered fresh.
    let loaded = Cache::load(output_dir);
    let mut work_count = 0usize;
    for file in &class_files {
        let hash = cache::hash_source(&file.raw_source);
        if loaded
            .get_if_fresh(&file.path.to_string_lossy(), &hash, model)
            .is_none()
        {
            work_count += 1;
        }
    }
    assert_eq!(
        work_count, 0,
        "expected 0 files requiring API calls after cache population"
    );
}

#[test]
fn cache_marks_modified_file_as_stale() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let model = "test-model";

    // Store an *old* hash so the file looks changed.
    let mut cache = Cache::default();
    let doc = stub_class_doc("AccountService");
    let file = &class_files[0];
    cache.update(
        file.path.to_string_lossy().into_owned(),
        "old-hash-that-doesnt-match".to_string(),
        model,
        doc,
    );
    cache.save(output_dir).unwrap();

    let loaded = Cache::load(output_dir);
    let actual_hash = cache::hash_source(&file.raw_source);
    let entry = loaded.get_if_fresh(&file.path.to_string_lossy(), &actual_hash, model);
    assert!(
        entry.is_none(),
        "stale cache entry should be treated as a miss"
    );
}

#[test]
fn cache_invalidated_on_model_change() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path();

    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let file = &class_files[0];
    let hash = cache::hash_source(&file.raw_source);

    let mut cache = Cache::default();
    cache.update(
        file.path.to_string_lossy().into_owned(),
        hash.clone(),
        "old-model",
        stub_class_doc("AccountService"),
    );
    cache.save(output_dir).unwrap();

    let loaded = Cache::load(output_dir);
    let entry = loaded.get_if_fresh(&file.path.to_string_lossy(), &hash, "new-model");
    assert!(
        entry.is_none(),
        "cache entry from a different model must be a miss"
    );
}

#[test]
fn force_flag_simulated_by_empty_cache() {
    // --force is implemented by loading Cache::default() instead of Cache::load().
    // Verify that an empty cache causes every file to be marked as needing work.
    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let cache = Cache::default();
    let model = "any-model";

    let work_count = class_files
        .iter()
        .filter(|f| {
            let hash = cache::hash_source(&f.raw_source);
            cache
                .get_if_fresh(&f.path.to_string_lossy(), &hash, model)
                .is_none()
        })
        .count();

    assert_eq!(
        work_count,
        class_files.len(),
        "with an empty cache (--force), all files should require API calls"
    );
}

// ---------------------------------------------------------------------------
// HTTP client with mock server (OpenAI-compatible endpoint)
// ---------------------------------------------------------------------------

/// Builds the JSON body that the OpenAI-compatible API returns for a class.
fn openai_class_response(doc: &ClassDocumentation) -> String {
    let inner = serde_json::to_string(doc).unwrap();
    format!(
        r#"{{"choices":[{{"message":{{"content":{inner_escaped}}}}}]}}"#,
        inner_escaped = serde_json::Value::String(inner)
    )
}

/// Builds the JSON body for a trigger documentation response.
fn openai_trigger_response(doc: &TriggerDocumentation) -> String {
    let inner = serde_json::to_string(doc).unwrap();
    format!(
        r#"{{"choices":[{{"message":{{"content":{inner_escaped}}}}}]}}"#,
        inner_escaped = serde_json::Value::String(inner)
    )
}

#[tokio::test]
async fn openai_compat_client_documents_class() {
    use sfdoc::openai_compat::OpenAiCompatClient;
    use sfdoc::types::SourceFile;

    let server = MockServer::start();
    let expected_doc = stub_class_doc("AccountService");

    let _mock = server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_class_response(&expected_doc));
    });

    let client = OpenAiCompatClient::new(
        "test-key".to_string(),
        "test-model",
        &server.base_url(),
        1,
        "TestProvider",
    )
    .unwrap();

    let file = SourceFile {
        path: PathBuf::from("AccountService.cls"),
        filename: "AccountService.cls".to_string(),
        raw_source: std::fs::read_to_string(class_fixtures_dir().join("AccountService.cls"))
            .unwrap(),
    };
    let meta = parser::parse_apex_class(&file.raw_source).unwrap();
    let doc = client.document_class(&file, &meta).await.unwrap();

    assert_eq!(doc.class_name, "AccountService");
    assert_eq!(doc.summary, expected_doc.summary);
}

#[tokio::test]
async fn openai_compat_client_documents_trigger() {
    use sfdoc::openai_compat::OpenAiCompatClient;
    use sfdoc::types::SourceFile;

    let server = MockServer::start();
    let expected_doc = stub_trigger_doc("AccountTrigger", "Account");

    let _mock = server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_trigger_response(&expected_doc));
    });

    let client = OpenAiCompatClient::new(
        "test-key".to_string(),
        "test-model",
        &server.base_url(),
        1,
        "TestProvider",
    )
    .unwrap();

    let trigger_files = TriggerScanner.scan(trigger_fixtures_dir()).unwrap();
    let file = &trigger_files[0];
    let apex_file = SourceFile {
        path: file.path.clone(),
        filename: file.filename.clone(),
        raw_source: file.raw_source.clone(),
    };
    let meta = trigger_parser::parse_apex_trigger(&file.raw_source).unwrap();
    let doc = client.document_trigger(&apex_file, &meta).await.unwrap();

    assert_eq!(doc.trigger_name, "AccountTrigger");
    assert_eq!(doc.sobject, "Account");
}

#[tokio::test]
async fn openai_compat_client_returns_error_on_non_200() {
    use sfdoc::openai_compat::OpenAiCompatClient;
    use sfdoc::types::SourceFile;

    let server = MockServer::start();
    let _mock = server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(400).body(r#"{"error":"bad request"}"#);
    });

    let client = OpenAiCompatClient::new(
        "test-key".to_string(),
        "test-model",
        &server.base_url(),
        1,
        "TestProvider",
    )
    .unwrap();

    let file = SourceFile {
        path: PathBuf::from("AccountService.cls"),
        filename: "AccountService.cls".to_string(),
        raw_source: "public class AccountService {}".to_string(),
    };
    let meta = parser::parse_apex_class(&file.raw_source).unwrap();
    let result = client.document_class(&file, &meta).await;
    assert!(result.is_err(), "expected error on HTTP 400");
}

// ---------------------------------------------------------------------------
// Full end-to-end pipeline with mock HTTP server
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_scan_parse_ai_render_markdown() {
    use sfdoc::openai_compat::OpenAiCompatClient;

    let server = MockServer::start();

    // The mock server returns a canned doc for every POST regardless of body.
    let account_doc = stub_class_doc("AccountService");
    let order_doc = stub_class_doc("OrderService");
    let trigger_doc = stub_trigger_doc("AccountTrigger", "Account");

    // The system prompts differ: class calls include "Apex classes" and trigger
    // calls include "Apex triggers".  Use these unique phrases to route the
    // mock responses reliably, regardless of what class names appear in the
    // source code (the trigger source code references AccountService, so
    // matching on class names alone would be ambiguous).
    //
    // Both class mocks match on "Apex classes".  httpmock uses first-match
    // wins, so AccountService (registered first) is consumed first, then
    // OrderService on the second call.
    // The class prompt body starts with "# Apex Class: {name}", which is unique
    // per class and won't appear in other classes' prompts or in trigger prompts.
    let _account_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .body_contains("# Apex Class: AccountService");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_class_response(&account_doc));
    });
    let _order_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .body_contains("# Apex Class: OrderService");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_class_response(&order_doc));
    });
    let _trigger_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .body_contains("Apex triggers");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_trigger_response(&trigger_doc));
    });

    let client = Arc::new(
        OpenAiCompatClient::new(
            "test-key".to_string(),
            "test-model",
            &server.base_url(),
            3,
            "TestProvider",
        )
        .unwrap(),
    );

    // Scan
    let class_files = ApexScanner.scan(class_fixtures_dir()).unwrap();
    let trigger_files = TriggerScanner.scan(trigger_fixtures_dir()).unwrap();

    // Parse
    let class_meta: Vec<_> = class_files
        .iter()
        .map(|f| parser::parse_apex_class(&f.raw_source).unwrap())
        .collect();
    let trigger_meta: Vec<_> = trigger_files
        .iter()
        .map(|f| trigger_parser::parse_apex_trigger(&f.raw_source).unwrap())
        .collect();

    // Call AI
    let mut class_docs = Vec::new();
    for (file, meta) in class_files.iter().zip(class_meta.iter()) {
        let doc = client.document_class(file, meta).await.unwrap();
        class_docs.push(doc);
    }
    let mut trigger_docs = Vec::new();
    for (file, meta) in trigger_files.iter().zip(trigger_meta.iter()) {
        let doc = client.document_trigger(file, meta).await.unwrap();
        trigger_docs.push(doc);
    }

    // Build render contexts
    let all_names = Arc::new(AllNames {
        class_names: class_meta.iter().map(|m| m.class_name.clone()).collect(),
        trigger_names: trigger_meta
            .iter()
            .map(|m| m.trigger_name.clone())
            .collect(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });
    let class_contexts: Vec<RenderContext> = class_files
        .iter()
        .zip(class_meta.iter())
        .zip(class_docs.iter())
        .map(|((file, meta), doc)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(class_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            RenderContext {
                folder,
                metadata: meta.clone(),
                documentation: doc.clone(),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();
    let trigger_contexts: Vec<TriggerRenderContext> = trigger_files
        .iter()
        .zip(trigger_meta.iter())
        .zip(trigger_docs.iter())
        .map(|((file, meta), doc)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(trigger_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            TriggerRenderContext {
                folder,
                metadata: meta.clone(),
                documentation: doc.clone(),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    // Render
    let tmp = tempfile::TempDir::new().unwrap();
    renderer::write_output(
        tmp.path(),
        &sfdoc::cli::OutputFormat::Markdown,
        &class_contexts,
        &trigger_contexts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    // Assert
    assert!(tmp.path().join("classes/AccountService.md").exists());
    assert!(tmp.path().join("classes/OrderService.md").exists());
    assert!(tmp.path().join("triggers/AccountTrigger.md").exists());
    assert!(tmp.path().join("index.md").exists());

    let index = std::fs::read_to_string(tmp.path().join("index.md")).unwrap();
    assert!(index.contains("AccountService"));
    assert!(index.contains("AccountTrigger"));

    let account_page =
        std::fs::read_to_string(tmp.path().join("classes/AccountService.md")).unwrap();
    assert!(account_page.contains("Summary for AccountService"));
}

// ---------------------------------------------------------------------------
// Flow pipeline
// ---------------------------------------------------------------------------

fn stub_flow_doc(api_name: &str) -> FlowDocumentation {
    FlowDocumentation {
        api_name: api_name.to_string(),
        label: api_name.replace('_', " "),
        summary: format!("Summary for {api_name}."),
        description: format!("Description for {api_name}."),
        business_process: "The business process.".to_string(),
        entry_criteria: "When record is created.".to_string(),
        key_decisions: vec![],
        admin_notes: vec![],
        relationships: vec![],
    }
}

#[test]
fn flow_scanner_finds_flow_fixture_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let flows_dir = tmp.path().join("flows");
    std::fs::create_dir_all(&flows_dir).unwrap();
    std::fs::write(
        flows_dir.join("Account_Flow.flow-meta.xml"),
        r#"<?xml version="1.0"?><Flow><label>Account Flow</label><processType>AutoLaunchedFlow</processType></Flow>"#,
    )
    .unwrap();

    let files = FlowScanner.scan(tmp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Account_Flow.flow-meta.xml");
}

#[test]
fn flow_pipeline_writes_markdown_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    let flow_xml = r#"<?xml version="1.0"?><Flow><label>My Flow</label><processType>AutoLaunchedFlow</processType><description>Does things.</description></Flow>"#;
    let meta = flow_parser::parse_flow("My_Flow", flow_xml).unwrap();
    let doc = stub_flow_doc("My_Flow");

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: ["My_Flow".to_string()].into_iter().collect(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = FlowRenderContext {
        metadata: meta,
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    renderer::write_output(
        &output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &[],
        &[],
        &[ctx],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    assert!(
        output_dir.join("flows/My_Flow.md").exists(),
        "flow page not created"
    );
    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(index.contains("My_Flow"), "flow missing from index");
}

#[test]
fn flow_page_contains_expected_content() {
    let flow_xml = r#"<?xml version="1.0"?><Flow>
        <label>Account Onboarding</label>
        <processType>AutoLaunchedFlow</processType>
        <description>Onboards new accounts.</description>
        <variables>
            <name>inputAccountId</name>
            <dataType>String</dataType>
            <isInput>true</isInput>
            <isOutput>false</isOutput>
        </variables>
    </Flow>"#;
    let meta = flow_parser::parse_flow("Account_Onboarding", flow_xml).unwrap();
    assert_eq!(meta.label, "Account Onboarding");
    assert_eq!(meta.process_type, "AutoLaunchedFlow");
    assert_eq!(meta.variables.len(), 1);
    assert!(meta.variables[0].is_input);
}

// ---------------------------------------------------------------------------
// Validation rule pipeline
// ---------------------------------------------------------------------------

fn stub_vr_doc(rule_name: &str, object_name: &str) -> ValidationRuleDocumentation {
    ValidationRuleDocumentation {
        rule_name: rule_name.to_string(),
        object_name: object_name.to_string(),
        summary: format!("Summary for {rule_name}."),
        when_fires: "When field is blank.".to_string(),
        what_protects: "Data quality.".to_string(),
        formula_explanation: "Checks that Name is not blank.".to_string(),
        edge_cases: vec![],
        relationships: vec![],
    }
}

#[test]
fn validation_rule_pipeline_writes_markdown_output() {
    use sfdoc::validation_rule_parser::parse_validation_rule;

    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    // Create the validation rule file in the expected path structure:
    // objects/{ObjectName}/validationRules/{file}.validationRule-meta.xml
    let vr_dir = tmp
        .path()
        .join("objects")
        .join("Account")
        .join("validationRules");
    std::fs::create_dir_all(&vr_dir).unwrap();
    let vr_path = vr_dir.join("Require_Name.validationRule-meta.xml");
    let vr_xml = r#"<?xml version="1.0"?><ValidationRule>
        <active>true</active>
        <description>Name must not be blank.</description>
        <errorConditionFormula>ISBLANK(Name)</errorConditionFormula>
        <errorMessage>Name is required.</errorMessage>
    </ValidationRule>"#;
    std::fs::write(&vr_path, vr_xml).unwrap();

    let meta = parse_validation_rule(&vr_path, vr_xml).unwrap();
    let doc = stub_vr_doc(&meta.rule_name, &meta.object_name);

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: [meta.rule_name.clone()].into_iter().collect(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = ValidationRuleRenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: meta.object_name.clone(),
    };

    renderer::write_output(
        &output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &[],
        &[],
        &[],
        &[ctx],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    let expected_path = output_dir
        .join("validation-rules")
        .join(format!("{}.md", meta.rule_name));
    assert!(
        expected_path.exists(),
        "validation rule page not created at {expected_path:?}"
    );

    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(
        index.contains(&meta.rule_name),
        "validation rule missing from index"
    );
}

// ---------------------------------------------------------------------------
// Object pipeline
// ---------------------------------------------------------------------------

fn stub_object_doc(object_name: &str) -> ObjectDocumentation {
    ObjectDocumentation {
        object_name: object_name.to_string(),
        label: object_name.replace("__c", ""),
        summary: format!("Summary for {object_name}."),
        description: format!("Description for {object_name}."),
        purpose: "Tracks business data.".to_string(),
        key_fields: vec![],
        relationships: vec![],
        admin_notes: vec![],
    }
}

#[test]
fn object_pipeline_writes_markdown_output() {
    use sfdoc::object_parser::parse_object;

    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    let obj_dir = tmp.path().join("objects").join("My_Object__c");
    std::fs::create_dir_all(&obj_dir).unwrap();
    let obj_path = obj_dir.join("My_Object__c.object-meta.xml");
    let obj_xml = r#"<?xml version="1.0"?><CustomObject>
        <label>My Object</label>
        <description>A test object.</description>
    </CustomObject>"#;
    std::fs::write(&obj_path, obj_xml).unwrap();

    let meta = parse_object(&obj_path, obj_xml).unwrap();
    let doc = stub_object_doc(&meta.object_name);

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: [meta.object_name.clone()].into_iter().collect(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = ObjectRenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    renderer::write_output(
        &output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &[],
        &[],
        &[],
        &[],
        &[ctx],
        &[],
        &[],
        &[],
        &[],
    )
    .unwrap();

    assert!(
        output_dir
            .join(format!("objects/{}.md", meta.object_name))
            .exists(),
        "object page not created"
    );
    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(
        index.contains(&meta.object_name),
        "object missing from index"
    );
}

// ---------------------------------------------------------------------------
// LWC pipeline
// ---------------------------------------------------------------------------

fn stub_lwc_doc(component_name: &str) -> LwcDocumentation {
    LwcDocumentation {
        component_name: component_name.to_string(),
        summary: format!("Summary for {component_name}."),
        description: format!("Description for {component_name}."),
        api_props: vec![LwcPropDocumentation {
            name: "recordId".to_string(),
            description: "The record Id.".to_string(),
        }],
        usage_notes: vec!["Use inside a record page.".to_string()],
        relationships: vec![],
    }
}

#[test]
fn lwc_scanner_finds_lwc_fixtures() {
    let tmp = tempfile::TempDir::new().unwrap();
    let comp_dir = tmp.path().join("lwc").join("myComp");
    std::fs::create_dir_all(&comp_dir).unwrap();
    std::fs::write(
        comp_dir.join("myComp.js-meta.xml"),
        "<LightningComponentBundle/>",
    )
    .unwrap();
    std::fs::write(
        comp_dir.join("myComp.js"),
        "import { LightningElement, api } from 'lwc';\nexport default class MyComp extends LightningElement {\n    @api recordId;\n}",
    )
    .unwrap();

    let files = LwcScanner.scan(tmp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].raw_source.contains("recordId"));
}

#[test]
fn lwc_pipeline_writes_markdown_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    // Set up a temp LWC component and parse it
    let comp_tmp = tempfile::TempDir::new().unwrap();
    let comp_dir = comp_tmp.path().join("lwc").join("myButton");
    std::fs::create_dir_all(&comp_dir).unwrap();
    let js = "import { LightningElement, api } from 'lwc';\nexport default class MyButton extends LightningElement {\n    @api label;\n}";
    let meta_path = comp_dir.join("myButton.js-meta.xml");
    std::fs::write(&meta_path, "<LightningComponentBundle/>").unwrap();
    std::fs::write(comp_dir.join("myButton.js"), js).unwrap();

    let meta = lwc_parser::parse_lwc(&meta_path, js).unwrap();
    let doc = stub_lwc_doc(&meta.component_name);

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: [meta.component_name.clone()].into_iter().collect(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = LwcRenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    renderer::write_output(
        &output_dir,
        &sfdoc::cli::OutputFormat::Markdown,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[ctx],
        &[],
        &[],
        &[],
    )
    .unwrap();

    assert!(
        output_dir
            .join(format!("lwc/{}.md", meta.component_name))
            .exists(),
        "LWC page not created"
    );
    let content =
        std::fs::read_to_string(output_dir.join(format!("lwc/{}.md", meta.component_name)))
            .unwrap();
    assert!(
        content.contains("Summary for myButton"),
        "summary missing from LWC page"
    );
    assert!(
        content.contains("recordId") || content.contains("label"),
        "api prop missing from LWC page"
    );

    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(
        index.contains(&meta.component_name),
        "LWC component missing from index"
    );
}

#[test]
fn lwc_pipeline_writes_html_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    let comp_tmp = tempfile::TempDir::new().unwrap();
    let comp_dir = comp_tmp.path().join("lwc").join("myCard");
    std::fs::create_dir_all(&comp_dir).unwrap();
    let js = "import { LightningElement, api } from 'lwc';\nexport default class MyCard extends LightningElement {\n    @api title;\n}";
    let meta_path = comp_dir.join("myCard.js-meta.xml");
    std::fs::write(&meta_path, "<LightningComponentBundle/>").unwrap();
    std::fs::write(comp_dir.join("myCard.js"), js).unwrap();

    let meta = lwc_parser::parse_lwc(&meta_path, js).unwrap();
    let doc = stub_lwc_doc(&meta.component_name);

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: [meta.component_name.clone()].into_iter().collect(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = LwcRenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    renderer::write_output(
        &output_dir,
        &sfdoc::cli::OutputFormat::Html,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[ctx],
        &[],
        &[],
        &[],
    )
    .unwrap();

    assert!(
        output_dir
            .join(format!("lwc/{}.html", meta.component_name))
            .exists(),
        "LWC HTML page not created"
    );
    let html =
        std::fs::read_to_string(output_dir.join(format!("lwc/{}.html", meta.component_name)))
            .unwrap();
    assert!(html.contains("myCard"), "component name missing from HTML");
    assert!(html.contains("<nav"), "sidebar missing from HTML");
    assert!(
        html.contains("LWC") || html.contains("Components"),
        "LWC sidebar section missing"
    );
}
