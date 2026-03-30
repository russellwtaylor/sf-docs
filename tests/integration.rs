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
use sfdoc::renderer::{self, RenderContext};
use sfdoc::scanner::{ApexScanner, FileScanner, FlowScanner, LwcScanner, TriggerScanner};
use sfdoc::trigger_parser;
use sfdoc::types::{
    AllNames, AuraDocumentation, ClassDocumentation, FlexiPageDocumentation, FlowDocumentation,
    LwcDocumentation, LwcPropDocumentation, MethodDocumentation, ObjectDocumentation,
    PropertyDocumentation, TriggerDocumentation, TriggerEventDocumentation,
    ValidationRuleDocumentation,
};

use sfdoc::aura_parser;
use sfdoc::custom_metadata_parser;
use sfdoc::flexipage_parser;
use sfdoc::object_parser;
use sfdoc::scanner::{
    AuraScanner, CustomMetadataScanner, FlexiPageScanner, ObjectScanner, ValidationRuleScanner,
};
use sfdoc::validation_rule_parser;

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

fn flow_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flows"))
}

fn object_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/objects"))
}

fn validation_rule_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validation-rules")
    })
}

fn lwc_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lwc"))
}

fn flexipage_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flexipages"))
}

fn aura_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aura"))
}

fn custom_metadata_fixtures_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/customMetadata"))
}

fn stub_flexipage_doc(api_name: &str) -> FlexiPageDocumentation {
    FlexiPageDocumentation {
        api_name: api_name.to_string(),
        label: format!("{} Label", api_name),
        summary: format!("Summary for {api_name}."),
        description: format!("Description for {api_name}."),
        usage_context: "Record pages".to_string(),
        key_components: vec![],
        relationships: vec![],
    }
}

fn stub_aura_doc(component_name: &str) -> AuraDocumentation {
    AuraDocumentation {
        component_name: component_name.to_string(),
        summary: format!("Summary for {component_name}."),
        description: format!("Description for {component_name}."),
        attributes: vec![],
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

    let class_contexts: Vec<_> = class_files
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

    let trigger_contexts: Vec<_> = trigger_files
        .iter()
        .zip(trigger_meta.iter())
        .map(|(file, meta)| {
            let folder = file
                .path
                .parent()
                .and_then(|p| p.strip_prefix(trigger_fixtures_dir()).ok())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            RenderContext {
                folder,
                metadata: meta.clone(),
                documentation: stub_trigger_doc(&meta.trigger_name, &meta.sobject),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    let bundle = renderer::DocumentationBundle {
        classes: &class_contexts,
        triggers: &trigger_contexts,
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(output_dir, &bundle).unwrap();

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

    let class_contexts: Vec<_> = class_files
        .iter()
        .zip(class_meta.iter())
        .map(|(_, meta)| RenderContext {
            folder: String::new(),
            metadata: meta.clone(),
            documentation: stub_class_doc(&meta.class_name),
            all_names: Arc::clone(&all_names),
        })
        .collect();

    let bundle = renderer::DocumentationBundle {
        classes: &class_contexts,
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(output_dir, &bundle).unwrap();

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

    let class_contexts: Vec<_> = class_files
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

    let bundle = renderer::DocumentationBundle {
        classes: &class_contexts,
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        aura: &[],
        custom_metadata: &[],
    };
    let index = renderer::render_index(&bundle);

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
        0,
    )
    .unwrap();

    let file = SourceFile {
        path: PathBuf::from("AccountService.cls"),
        filename: "AccountService.cls".to_string(),
        raw_source: std::fs::read_to_string(class_fixtures_dir().join("AccountService.cls"))
            .unwrap(),
    };
    let meta = parser::parse_apex_class(&file.raw_source).unwrap();
    let doc: sfdoc::types::ClassDocumentation = sfdoc::doc_client::document(
        &client,
        sfdoc::prompts::CLASS_SYSTEM_PROMPT,
        &sfdoc::prompts::build_class_prompt(&file, &meta),
        &meta.class_name,
    )
    .await
    .unwrap();

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
        0,
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
    let doc: sfdoc::types::TriggerDocumentation = sfdoc::doc_client::document(
        &client,
        sfdoc::prompts::TRIGGER_SYSTEM_PROMPT,
        &sfdoc::prompts::build_trigger_prompt(&apex_file, &meta),
        &meta.trigger_name,
    )
    .await
    .unwrap();

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
        0,
    )
    .unwrap();

    let file = SourceFile {
        path: PathBuf::from("AccountService.cls"),
        filename: "AccountService.cls".to_string(),
        raw_source: "public class AccountService {}".to_string(),
    };
    let meta = parser::parse_apex_class(&file.raw_source).unwrap();
    let result: Result<sfdoc::types::ClassDocumentation, _> = sfdoc::doc_client::document(
        &client,
        sfdoc::prompts::CLASS_SYSTEM_PROMPT,
        &sfdoc::prompts::build_class_prompt(&file, &meta),
        &meta.class_name,
    )
    .await;
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
            0,
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
        let doc: ClassDocumentation = sfdoc::doc_client::document(
            client.as_ref(),
            sfdoc::prompts::CLASS_SYSTEM_PROMPT,
            &sfdoc::prompts::build_class_prompt(file, meta),
            &meta.class_name,
        )
        .await
        .unwrap();
        class_docs.push(doc);
    }
    let mut trigger_docs = Vec::new();
    for (file, meta) in trigger_files.iter().zip(trigger_meta.iter()) {
        let doc: TriggerDocumentation = sfdoc::doc_client::document(
            client.as_ref(),
            sfdoc::prompts::TRIGGER_SYSTEM_PROMPT,
            &sfdoc::prompts::build_trigger_prompt(file, meta),
            &meta.trigger_name,
        )
        .await
        .unwrap();
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
    let class_contexts: Vec<_> = class_files
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
    let trigger_contexts: Vec<_> = trigger_files
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
            RenderContext {
                folder,
                metadata: meta.clone(),
                documentation: doc.clone(),
                all_names: Arc::clone(&all_names),
            }
        })
        .collect();

    // Render
    let tmp = tempfile::TempDir::new().unwrap();
    let bundle = renderer::DocumentationBundle {
        classes: &class_contexts,
        triggers: &trigger_contexts,
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(tmp.path(), &bundle).unwrap();

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

    let ctx = RenderContext {
        metadata: meta,
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[ctx],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

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

    let ctx = RenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: meta.object_name.clone(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[],
        validation_rules: &[ctx],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

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

    let ctx = RenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[ctx],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

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

    let ctx = RenderContext {
        metadata: meta.clone(),
        documentation: doc,
        all_names,
        folder: String::new(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[ctx],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

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

// ---------------------------------------------------------------------------
// Fixture-based scanner + parser tests for all metadata types
// ---------------------------------------------------------------------------

#[test]
fn flow_scanner_finds_flow_fixture() {
    let files = FlowScanner.scan(flow_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Account_Onboarding.flow-meta.xml");
}

#[test]
fn flow_fixture_parses_correctly() {
    let files = FlowScanner.scan(flow_fixtures_dir()).unwrap();
    let file = &files[0];
    let api_name = file.filename.trim_end_matches(".flow-meta.xml");
    let meta = flow_parser::parse_flow(api_name, &file.raw_source).unwrap();
    assert_eq!(meta.label, "Account Onboarding");
    assert_eq!(meta.process_type, "AutoLaunchedFlow");
    assert_eq!(meta.variables.len(), 1);
    assert_eq!(meta.decisions, 1);
    assert_eq!(meta.screens, 1);
    assert_eq!(meta.record_operations.len(), 2);
    assert_eq!(meta.action_calls.len(), 1);
}

#[test]
fn object_scanner_finds_object_fixture() {
    let files = ObjectScanner.scan(object_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Invoice__c.object-meta.xml");
}

#[test]
fn object_fixture_parses_with_fields() {
    let files = ObjectScanner.scan(object_fixtures_dir()).unwrap();
    let file = &files[0];
    let meta = object_parser::parse_object(&file.path, &file.raw_source).unwrap();
    assert_eq!(meta.object_name, "Invoice__c");
    assert_eq!(meta.label, "Invoice");
    assert!(meta.description.contains("invoices"));
    assert_eq!(meta.fields.len(), 2);
    let status = meta
        .fields
        .iter()
        .find(|f| f.api_name == "Status__c")
        .unwrap();
    assert_eq!(status.field_type, "Picklist");
    assert!(status.required);
    let account = meta
        .fields
        .iter()
        .find(|f| f.api_name == "Account__c")
        .unwrap();
    assert_eq!(account.field_type, "Lookup");
    assert_eq!(account.reference_to, "Account");
    assert!(!account.help_text.is_empty());
}

#[test]
fn validation_rule_scanner_finds_fixture() {
    let files = ValidationRuleScanner
        .scan(validation_rule_fixtures_dir())
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Require_Email.validationRule-meta.xml");
}

#[test]
fn validation_rule_fixture_parses_correctly() {
    let files = ValidationRuleScanner
        .scan(validation_rule_fixtures_dir())
        .unwrap();
    let file = &files[0];
    let meta = validation_rule_parser::parse_validation_rule(&file.path, &file.raw_source).unwrap();
    assert_eq!(meta.rule_name, "Require_Email");
    assert_eq!(meta.object_name, "Account");
    assert!(meta.active);
    assert!(meta.error_condition_formula.contains("ISPICKVAL"));
    assert!(meta.error_message.contains("Email is required"));
}

#[test]
fn lwc_scanner_finds_lwc_fixture() {
    let files = LwcScanner.scan(lwc_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "myButton.js-meta.xml");
}

#[test]
fn lwc_fixture_parses_api_props_and_slots() {
    let files = LwcScanner.scan(lwc_fixtures_dir()).unwrap();
    let file = &files[0];
    let meta = lwc_parser::parse_lwc(&file.path, &file.raw_source).unwrap();
    assert_eq!(meta.component_name, "myButton");
    let prop_names: Vec<&str> = meta
        .api_props
        .iter()
        .filter(|p| !p.is_method)
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        prop_names.contains(&"label"),
        "missing @api label: {:?}",
        prop_names
    );
    assert!(
        prop_names.contains(&"variant"),
        "missing @api variant: {:?}",
        prop_names
    );
    assert!(
        prop_names.contains(&"disabled"),
        "missing @api disabled: {:?}",
        prop_names
    );
    let method_names: Vec<&str> = meta
        .api_props
        .iter()
        .filter(|p| p.is_method)
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        method_names.contains(&"focus"),
        "missing @api focus(): {:?}",
        method_names
    );
    assert!(
        meta.slots.contains(&"icon".to_string()),
        "missing named slot 'icon': {:?}",
        meta.slots
    );
    assert!(
        meta.slots.contains(&"default".to_string()),
        "missing default slot: {:?}",
        meta.slots
    );
    assert!(
        meta.referenced_components.contains(&"tooltip".to_string()),
        "missing c-tooltip ref: {:?}",
        meta.referenced_components
    );
}

#[test]
fn flexipage_scanner_finds_fixture() {
    let files = FlexiPageScanner.scan(flexipage_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Account_Record_Page.flexipage-meta.xml");
}

#[test]
fn flexipage_fixture_parses_correctly() {
    let files = FlexiPageScanner.scan(flexipage_fixtures_dir()).unwrap();
    let file = &files[0];
    let api_name = file.filename.trim_end_matches(".flexipage-meta.xml");
    let meta = flexipage_parser::parse_flexipage(api_name, &file.raw_source).unwrap();
    assert_eq!(meta.label, "Account Record Page");
    assert_eq!(meta.page_type, "RecordPage");
    assert_eq!(meta.sobject, "Account");
    assert!(meta.component_names.contains(&"accountDetails".to_string()));
    assert!(meta
        .component_names
        .contains(&"relatedContacts".to_string()));
    assert!(meta
        .component_names
        .contains(&"force:detailPanel".to_string()));
}

#[test]
fn aura_scanner_finds_fixture() {
    let files = AuraScanner.scan(aura_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "myAuraComp.cmp");
}

#[test]
fn aura_fixture_parses_correctly() {
    let files = AuraScanner.scan(aura_fixtures_dir()).unwrap();
    let file = &files[0];
    let meta = aura_parser::parse_aura(&file.path, &file.raw_source).unwrap();
    assert_eq!(meta.component_name, "myAuraComp");
    assert_eq!(meta.extends.as_deref(), Some("c:baseComponent"));
    assert_eq!(meta.attributes.len(), 2);
    assert!(meta.attributes.iter().any(|a| a.name == "recordId"));
    assert!(meta
        .attributes
        .iter()
        .any(|a| a.name == "title" && a.default == "Details"));
    assert!(meta.events_handled.contains(&"onSave".to_string()));
}

#[test]
fn custom_metadata_scanner_finds_fixture() {
    let files = CustomMetadataScanner
        .scan(custom_metadata_fixtures_dir())
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].filename,
        "Integration_Settings__mdt.Default.md-meta.xml"
    );
}

#[test]
fn custom_metadata_fixture_parses_correctly() {
    let files = CustomMetadataScanner
        .scan(custom_metadata_fixtures_dir())
        .unwrap();
    let file = &files[0];
    let rec =
        custom_metadata_parser::parse_custom_metadata_record(&file.path, &file.raw_source).unwrap();
    assert_eq!(rec.type_name, "Integration_Settings__mdt");
    assert_eq!(rec.record_name, "Default");
    assert_eq!(rec.label, "Default Settings");
    assert_eq!(rec.values.len(), 3);
    assert!(rec
        .values
        .iter()
        .any(|(f, v)| f == "Endpoint__c" && v == "https://api.example.com"));
    assert!(rec
        .values
        .iter()
        .any(|(f, v)| f == "Timeout__c" && v == "30"));
    assert!(rec
        .values
        .iter()
        .any(|(f, v)| f == "Enabled__c" && v == "true"));
}

// ---------------------------------------------------------------------------
// Cross-linking and mixed-type rendering
// ---------------------------------------------------------------------------

#[test]
fn all_names_all_known_names_union() {
    let all = AllNames {
        class_names: ["ClassA".to_string()].into_iter().collect(),
        trigger_names: ["TriggerA".to_string()].into_iter().collect(),
        flow_names: ["FlowA".to_string()].into_iter().collect(),
        validation_rule_names: ["RuleA".to_string()].into_iter().collect(),
        object_names: ["ObjectA".to_string()].into_iter().collect(),
        lwc_names: ["lwcA".to_string()].into_iter().collect(),
        flexipage_names: ["PageA".to_string()].into_iter().collect(),
        aura_names: ["AuraA".to_string()].into_iter().collect(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    };
    let known = all.all_known_names();
    assert!(known.contains("ClassA"));
    assert!(known.contains("TriggerA"));
    assert!(known.contains("FlowA"));
    assert!(known.contains("RuleA"));
    assert!(known.contains("ObjectA"));
    assert!(known.contains("lwcA"));
    assert!(known.contains("PageA"));
    assert!(known.contains("AuraA"));
    assert_eq!(known.len(), 8);
}

#[test]
fn mixed_bundle_renders_index_with_all_sections() {
    let all_names = Arc::new(AllNames {
        class_names: ["AccountService".to_string()].into_iter().collect(),
        trigger_names: ["AccountTrigger".to_string()].into_iter().collect(),
        flow_names: ["My_Flow".to_string()].into_iter().collect(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: ["myButton".to_string()].into_iter().collect(),
        flexipage_names: HashSet::new(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let class_ctx = RenderContext {
        metadata: parser::parse_apex_class("public class AccountService { }").unwrap(),
        documentation: stub_class_doc("AccountService"),
        all_names: all_names.clone(),
        folder: "classes".to_string(),
    };

    let trigger_ctx = RenderContext {
        metadata: trigger_parser::parse_apex_trigger(
            "trigger AccountTrigger on Account (before insert) { }",
        )
        .unwrap(),
        documentation: stub_trigger_doc("AccountTrigger", "Account"),
        all_names: all_names.clone(),
        folder: "triggers".to_string(),
    };

    let flow_ctx = RenderContext {
        metadata: sfdoc::types::FlowMetadata {
            api_name: "My_Flow".to_string(),
            label: "My Flow".to_string(),
            ..Default::default()
        },
        documentation: stub_flow_doc("My_Flow"),
        all_names: all_names.clone(),
        folder: "flows".to_string(),
    };

    let lwc_ctx = RenderContext {
        metadata: sfdoc::types::LwcMetadata {
            component_name: "myButton".to_string(),
            ..Default::default()
        },
        documentation: stub_lwc_doc("myButton"),
        all_names: all_names.clone(),
        folder: "lwc".to_string(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[class_ctx],
        triggers: &[trigger_ctx],
        flows: &[flow_ctx],
        validation_rules: &[],
        objects: &[],
        lwc: &[lwc_ctx],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };

    let index = renderer::render_index(&bundle);
    assert!(index.contains("AccountService"), "index missing class");
    assert!(index.contains("AccountTrigger"), "index missing trigger");
    assert!(
        index.contains("My_Flow") || index.contains("My Flow"),
        "index missing flow"
    );
    assert!(index.contains("myButton"), "index missing LWC");
}

#[test]
fn mixed_bundle_writes_all_output_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let all_names = Arc::new(AllNames {
        class_names: ["Svc".to_string()].into_iter().collect(),
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
    let class_ctx = RenderContext {
        metadata: parser::parse_apex_class("public class Svc { }").unwrap(),
        documentation: stub_class_doc("Svc"),
        all_names: all_names.clone(),
        folder: "classes".to_string(),
    };
    let bundle = renderer::DocumentationBundle {
        classes: &[class_ctx],
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(tmp.path(), &bundle).unwrap();
    assert!(tmp.path().join("classes/Svc.md").exists());
    assert!(tmp.path().join("index.md").exists());
}

// ---------------------------------------------------------------------------
// FlexiPage pipeline rendering
// ---------------------------------------------------------------------------

#[test]
fn flexipage_pipeline_writes_markdown_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    let meta = sfdoc::types::FlexiPageMetadata {
        api_name: "Account_Record_Page".to_string(),
        label: "Account Record Page".to_string(),
        page_type: "RecordPage".to_string(),
        sobject: "Account".to_string(),
        description: String::new(),
        component_names: vec!["accountDetails".to_string()],
        flow_names: vec![],
    };

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: ["Account_Record_Page".to_string()].into_iter().collect(),
        aura_names: HashSet::new(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = RenderContext {
        metadata: meta,
        documentation: stub_flexipage_doc("Account_Record_Page"),
        all_names,
        folder: String::new(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[ctx],
        custom_metadata: &[],
        aura: &[],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

    assert!(
        output_dir
            .join("flexipages/Account_Record_Page.md")
            .exists(),
        "flexipage page not created"
    );
    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(
        index.contains("Account_Record_Page") || index.contains("Account Record Page"),
        "flexipage missing from index"
    );
}

// ---------------------------------------------------------------------------
// Aura pipeline rendering
// ---------------------------------------------------------------------------

#[test]
fn aura_pipeline_writes_markdown_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output_dir = tmp.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();

    let meta = sfdoc::types::AuraMetadata {
        component_name: "myAuraComp".to_string(),
        attributes: vec![],
        events_handled: vec!["onSave".to_string()],
        extends: Some("c:baseComponent".to_string()),
    };

    let all_names = Arc::new(AllNames {
        class_names: HashSet::new(),
        trigger_names: HashSet::new(),
        flow_names: HashSet::new(),
        validation_rule_names: HashSet::new(),
        object_names: HashSet::new(),
        lwc_names: HashSet::new(),
        flexipage_names: HashSet::new(),
        aura_names: ["myAuraComp".to_string()].into_iter().collect(),
        custom_metadata_type_names: HashSet::new(),
        interface_implementors: std::collections::HashMap::new(),
    });

    let ctx = RenderContext {
        metadata: meta,
        documentation: stub_aura_doc("myAuraComp"),
        all_names,
        folder: String::new(),
    };

    let bundle = renderer::DocumentationBundle {
        classes: &[],
        triggers: &[],
        flows: &[],
        validation_rules: &[],
        objects: &[],
        lwc: &[],
        flexipages: &[],
        custom_metadata: &[],
        aura: &[ctx],
    };
    renderer::write_output(&output_dir, &bundle).unwrap();

    assert!(
        output_dir.join("aura/myAuraComp.md").exists(),
        "aura page not created"
    );
    let index = std::fs::read_to_string(output_dir.join("index.md")).unwrap();
    assert!(
        index.contains("myAuraComp"),
        "aura component missing from index"
    );
}

// ---------------------------------------------------------------------------
// --name-filter and --tag integration tests
// ---------------------------------------------------------------------------

#[test]
fn name_filter_matches_glob_pattern() {
    use clap::Parser;

    // Parse args with --name-filter
    let cli =
        sfdoc::cli::Cli::try_parse_from(["sfdoc", "generate", "--name-filter", "Order*"]).unwrap();
    let args = match cli.command {
        sfdoc::cli::Commands::Generate(g) => g,
        _ => panic!("expected Generate"),
    };

    assert!(args.name_matches("OrderService"));
    assert!(args.name_matches("OrderHelper"));
    assert!(!args.name_matches("AccountService"));
}

#[test]
fn tag_parsing_from_apex_source() {
    let source = r#"
    /**
     * @tag billing
     * @tag integration
     */
    public class OrderService {
    }
    "#;
    let meta = sfdoc::parser::parse_apex_class(source).unwrap();
    assert_eq!(meta.tags, vec!["billing", "integration"]);
}

#[test]
fn tag_filter_matches_case_insensitive() {
    use clap::Parser;

    let cli =
        sfdoc::cli::Cli::try_parse_from(["sfdoc", "generate", "--tag", "billing,Integration"])
            .unwrap();
    let args = match cli.command {
        sfdoc::cli::Commands::Generate(g) => g,
        _ => panic!("expected Generate"),
    };

    assert!(args.tag_matches(&["billing".to_string()]));
    assert!(args.tag_matches(&["INTEGRATION".to_string()]));
    assert!(!args.tag_matches(&["unrelated".to_string()]));
}

#[test]
fn update_no_target_shows_error() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args(["update"])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("Usage"),
        "Expected usage/required error, got: {stderr}"
    );
}

#[test]
fn update_no_cache_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "SomeClass",
            "--source-dir",
            tmp.path().to_str().unwrap(),
            "--output",
            tmp.path().join("out").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No existing documentation found") || stderr.contains("sfdoc generate"),
        "Expected 'run generate first' error, got: {stderr}"
    );
}

#[test]
fn update_file_not_found_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(
        out_dir.join(".sfdoc-cache.json"),
        r#"{"cache_version":1,"entries":{},"trigger_entries":{},"flow_entries":{},"validation_rule_entries":{},"object_entries":{},"lwc_entries":{},"flexipage_entries":{},"aura_entries":{}}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "/nonexistent/path/Foo.cls",
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Expected 'file not found' error, got: {stderr}"
    );
}

#[test]
fn update_name_not_found_shows_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(
        out_dir.join(".sfdoc-cache.json"),
        r#"{"cache_version":1,"entries":{},"trigger_entries":{},"flow_entries":{},"validation_rule_entries":{},"object_entries":{},"lwc_entries":{},"flexipage_entries":{},"aura_entries":{}}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sfdoc"))
        .args([
            "update",
            "NonexistentClass",
            "--source-dir",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run sfdoc");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No source file matching"),
        "Expected 'no source file' error, got: {stderr}"
    );
}
