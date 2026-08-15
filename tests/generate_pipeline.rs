/// Pipeline tests for `sfdoc generate`.
///
/// These drive `run_generate` against a tempfile Salesforce tree and an
/// httpmock OpenAI-compatible server so filter/overlay/prune behaviour is
/// locked without a live API key.
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use httpmock::prelude::*;
use sfdoc::cache::Cache;
use sfdoc::cli::{Cli, Commands, GenerateArgs};
use sfdoc::generate::run_generate;
use sfdoc::types::FlowDocumentation;

fn parse_generate(args: &[&str]) -> GenerateArgs {
    let mut full = vec!["sfdoc", "generate"];
    full.extend(args);
    match Cli::try_parse_from(full).expect("CLI should parse").command {
        Commands::Generate(g) => g,
        _ => panic!("expected Generate"),
    }
}

fn write_class(dir: &Path, name: &str) {
    let classes = dir.join("classes");
    fs::create_dir_all(&classes).unwrap();
    fs::write(
        classes.join(format!("{name}.cls")),
        format!("public class {name} {{\n    public void run() {{}}\n}}\n"),
    )
    .unwrap();
}

fn openai_class_response() -> String {
    let inner = serde_json::json!({
        "class_name": "Generated",
        "summary": "Generated summary.",
        "description": "Generated description.",
        "methods": [],
        "properties": [],
        "usage_examples": [],
        "relationships": []
    })
    .to_string();
    serde_json::json!({
        "choices": [{ "message": { "content": inner } }]
    })
    .to_string()
}

fn mock_openai_ok(server: &MockServer) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_class_response());
    })
}

fn generate_args(source: &Path, output: &Path, extra: &[&str]) -> GenerateArgs {
    let source = source.to_str().unwrap();
    let output = output.to_str().unwrap();
    let mut args = vec![
        "--provider",
        "ollama",
        "--source-dir",
        source,
        "--output",
        output,
        "--concurrency",
        "1",
    ];
    args.extend_from_slice(extra);
    parse_generate(&args)
}

fn with_mock_url(mut args: GenerateArgs, server: &MockServer) -> GenerateArgs {
    args.api_base_url = Some(server.base_url());
    args
}

async fn run(args: GenerateArgs) -> anyhow::Result<()> {
    run_generate(&args).await
}

#[tokio::test]
async fn name_filter_star_service_keeps_order_service_cls() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "OrderService");
    write_class(&source, "AccountHelper");
    let output = tmp.path().join("docs");

    let server = MockServer::start();
    let mock = mock_openai_ok(&server);

    run(with_mock_url(
        generate_args(
            &source,
            &output,
            &["--type", "apex", "--name-filter", "*Service"],
        ),
        &server,
    ))
    .await
    .unwrap();

    assert!(
        output.join("classes/OrderService.md").exists(),
        "OrderService.cls must match glob *Service"
    );
    assert!(
        !output.join("classes/AccountHelper.md").exists(),
        "AccountHelper.cls must not match glob *Service"
    );
    mock.assert_hits(1);
}

#[tokio::test]
async fn test_classes_skipped_unless_include_tests() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "AccountService");
    write_class(&source, "AccountServiceTest");
    let output = tmp.path().join("docs");

    let server = MockServer::start();
    let mock = mock_openai_ok(&server);

    run(with_mock_url(
        generate_args(&source, &output, &["--type", "apex"]),
        &server,
    ))
    .await
    .unwrap();

    assert!(output.join("classes/AccountService.md").exists());
    assert!(
        !output.join("classes/AccountServiceTest.md").exists(),
        "test classes are skipped by default"
    );
    mock.assert_hits(1);

    run(with_mock_url(
        generate_args(&source, &output, &["--type", "apex", "--include-tests"]),
        &server,
    ))
    .await
    .unwrap();

    assert!(output.join("classes/AccountServiceTest.md").exists());
}

#[tokio::test]
async fn partial_generate_overlays_index_from_cache() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "OrderService");
    let output = tmp.path().join("docs");
    fs::create_dir_all(&output).unwrap();

    let mut cache = Cache::default();
    cache.update_flow(
        "flows/Account_Onboarding.flow-meta.xml".into(),
        "hash".into(),
        "unused-model",
        FlowDocumentation {
            api_name: "Account_Onboarding".into(),
            label: "Account Onboarding".into(),
            summary: "Onboards accounts.".into(),
            description: "Flow description.".into(),
            ..Default::default()
        },
    );
    cache.save(&output).unwrap();

    let server = MockServer::start();
    let _mock = mock_openai_ok(&server);

    run(with_mock_url(
        generate_args(&source, &output, &["--type", "apex"]),
        &server,
    ))
    .await
    .unwrap();

    let index = fs::read_to_string(output.join("index.md")).unwrap();
    assert!(
        output.join("classes/OrderService.md").exists(),
        "current Apex page must be written"
    );
    assert!(
        index.contains("Account_Onboarding"),
        "cached Flow must remain in the index on --type apex:\n{index}"
    );
    assert!(
        index.contains("## Flows") && index.contains("## Classes"),
        "index should keep both sections:\n{index}"
    );
}

#[tokio::test]
async fn full_generate_prunes_deleted_class_pages() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "Keep");
    let output = tmp.path().join("docs");
    let classes = output.join("classes");
    fs::create_dir_all(&classes).unwrap();
    fs::write(classes.join("Gone.md"), "# Gone\n").unwrap();
    fs::write(classes.join("Keep.md"), "# stale Keep\n").unwrap();

    let server = MockServer::start();
    let _mock = mock_openai_ok(&server);

    run(with_mock_url(
        generate_args(&source, &output, &["--type", "apex"]),
        &server,
    ))
    .await
    .unwrap();

    assert!(output.join("classes/Keep.md").exists());
    assert!(
        !output.join("classes/Gone.md").exists(),
        "orphan class page must be pruned on a type-complete generate"
    );
}

#[tokio::test]
async fn name_filter_does_not_prune_sibling_class_pages() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "OrderService");
    write_class(&source, "AccountHelper");
    let output = tmp.path().join("docs");
    let classes = output.join("classes");
    fs::create_dir_all(&classes).unwrap();
    fs::write(classes.join("AccountHelper.md"), "# AccountHelper\n").unwrap();

    let server = MockServer::start();
    let _mock = mock_openai_ok(&server);

    run(with_mock_url(
        generate_args(
            &source,
            &output,
            &["--type", "apex", "--name-filter", "*Service"],
        ),
        &server,
    ))
    .await
    .unwrap();

    assert!(output.join("classes/OrderService.md").exists());
    assert!(
        output.join("classes/AccountHelper.md").exists(),
        "--name-filter must not delete sibling pages"
    );
}

#[tokio::test]
async fn partial_api_failure_still_writes_successful_pages() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = tmp.path().join("src");
    write_class(&source, "OrderService");
    write_class(&source, "AccountHelper");
    let output = tmp.path().join("docs");

    let server = MockServer::start();
    let _ok = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .body_contains("OrderService");
        then.status(200)
            .header("content-type", "application/json")
            .body(openai_class_response());
    });
    let _fail = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .body_contains("AccountHelper");
        then.status(400).body(r#"{"error":"boom"}"#);
    });

    let result = run(with_mock_url(
        generate_args(&source, &output, &["--type", "apex"]),
        &server,
    ))
    .await;

    assert!(result.is_err(), "expected generate to fail after a 500");
    assert!(
        output.join("classes/OrderService.md").exists(),
        "successful page must still be written; got {:?}",
        list_md(&output.join("classes"))
    );
}

fn list_md(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect()
}
