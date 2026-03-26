use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::cli::{MetadataType, OutputFormat};
use crate::scanner::{
    ApexScanner, AuraScanner, CustomMetadataScanner, FileScanner, FlexiPageScanner, FlowScanner,
    LwcScanner, ObjectScanner, TriggerScanner, ValidationRuleScanner,
};
use crate::types::SourceFile;

/// Known file extensions and their metadata types.
const EXTENSION_MAP: &[(&str, MetadataType)] = &[
    (".cls", MetadataType::Apex),
    (".trigger", MetadataType::Triggers),
    (".flow-meta.xml", MetadataType::Flows),
    (".validationRule-meta.xml", MetadataType::ValidationRules),
    (".object-meta.xml", MetadataType::Objects),
    (".js-meta.xml", MetadataType::Lwc),
    (".flexipage-meta.xml", MetadataType::Flexipages),
    (".md-meta.xml", MetadataType::CustomMetadata),
    (".cmp", MetadataType::Aura),
];

/// Result of resolving a target string to a concrete source file.
#[derive(Debug)]
pub struct ResolvedTarget {
    pub source_file: SourceFile,
    pub metadata_type: MetadataType,
}

/// Returns true if the target looks like a file path rather than a bare name.
fn is_path_target(target: &str) -> bool {
    if target.contains('/') || target.contains('\\') {
        return true;
    }
    EXTENSION_MAP.iter().any(|(ext, _)| target.ends_with(ext))
}

/// Determines the metadata type from a file path's extension.
fn metadata_type_from_path(path: &Path) -> Result<MetadataType> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for (ext, mt) in EXTENSION_MAP {
        if name.ends_with(ext) {
            return Ok(*mt);
        }
    }
    bail!(
        "Cannot determine metadata type for '{}'. Supported extensions: {}",
        path.display(),
        EXTENSION_MAP
            .iter()
            .map(|(e, _)| *e)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Resolves a CLI target (path or name) to a source file and its metadata type.
pub fn resolve_target(target: &str, source_dir: &Path) -> Result<ResolvedTarget> {
    if is_path_target(target) {
        resolve_path_target(target)
    } else {
        resolve_name_target(target, source_dir)
    }
}

fn resolve_path_target(target: &str) -> Result<ResolvedTarget> {
    let path = PathBuf::from(target);
    if !path.exists() {
        bail!("File not found: '{}'", path.display());
    }
    let metadata_type = metadata_type_from_path(&path)?;
    let raw_source = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read '{}'", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    Ok(ResolvedTarget {
        source_file: SourceFile {
            path,
            filename,
            raw_source,
        },
        metadata_type,
    })
}

fn resolve_name_target(name: &str, source_dir: &Path) -> Result<ResolvedTarget> {
    let scanners: Vec<(&dyn FileScanner, MetadataType, &str)> = vec![
        (&ApexScanner, MetadataType::Apex, ".cls"),
        (&TriggerScanner, MetadataType::Triggers, ".trigger"),
        (&FlowScanner, MetadataType::Flows, ".flow-meta.xml"),
        (
            &ValidationRuleScanner,
            MetadataType::ValidationRules,
            ".validationRule-meta.xml",
        ),
        (
            &ObjectScanner,
            MetadataType::Objects,
            ".object-meta.xml",
        ),
        (&LwcScanner, MetadataType::Lwc, ".js-meta.xml"),
        (
            &FlexiPageScanner,
            MetadataType::Flexipages,
            ".flexipage-meta.xml",
        ),
        (
            &CustomMetadataScanner,
            MetadataType::CustomMetadata,
            ".md-meta.xml",
        ),
        (&AuraScanner, MetadataType::Aura, ".cmp"),
    ];

    let mut matches: Vec<(SourceFile, MetadataType)> = Vec::new();

    for (scanner, mt, suffix) in &scanners {
        if let Ok(files) = scanner.scan(source_dir) {
            for file in files {
                let stem = file
                    .filename
                    .strip_suffix(suffix)
                    .unwrap_or(&file.filename);
                if stem.eq_ignore_ascii_case(name) {
                    matches.push((file, *mt));
                }
            }
        }
    }

    match matches.len() {
        0 => {
            let mut all_names: Vec<String> = Vec::new();
            for (scanner, _, suffix) in &scanners {
                if let Ok(files) = scanner.scan(source_dir) {
                    for file in files {
                        let stem = file
                            .filename
                            .strip_suffix(suffix)
                            .unwrap_or(&file.filename);
                        all_names.push(stem.to_string());
                    }
                }
            }
            let suggestions = find_similar_names(name, &all_names);
            let mut msg = format!(
                "No source file matching '{}' found in '{}'.",
                name,
                source_dir.display()
            );
            if !suggestions.is_empty() {
                msg.push_str(&format!(" Did you mean: {}?", suggestions.join(", ")));
            }
            bail!(msg);
        }
        1 => {
            let (file, mt) = matches.remove(0);
            Ok(ResolvedTarget {
                source_file: file,
                metadata_type: mt,
            })
        }
        _ => {
            let list: Vec<String> = matches
                .iter()
                .map(|(f, _)| format!("  {}", f.path.display()))
                .collect();
            bail!(
                "'{}' matches multiple files:\n{}\nSpecify the full path instead.",
                name,
                list.join("\n")
            );
        }
    }
}

/// Returns names within Levenshtein distance <= 2 of the target.
fn find_similar_names(target: &str, candidates: &[String]) -> Vec<String> {
    let target_lower = target.to_lowercase();
    candidates
        .iter()
        .filter(|c| levenshtein_distance(&target_lower, &c.to_lowercase()) <= 2)
        .cloned()
        .collect()
}

/// Simple Levenshtein distance implementation.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

/// Detect the output format from the existing output directory.
/// If `explicit` is `Some`, uses that. Otherwise checks for index.html / index.md.
/// Falls back to Markdown.
pub fn detect_output_format(output_dir: &Path, explicit: &Option<OutputFormat>) -> OutputFormat {
    if let Some(fmt) = explicit {
        return fmt.clone();
    }
    if output_dir.join("index.html").exists() {
        OutputFormat::Html
    } else {
        OutputFormat::Markdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_path_with_slash() {
        assert!(is_path_target("src/classes/Foo.cls"));
    }

    #[test]
    fn is_path_with_extension() {
        assert!(is_path_target("Foo.cls"));
        assert!(is_path_target("MyFlow.flow-meta.xml"));
        assert!(is_path_target("AccTrig.trigger"));
    }

    #[test]
    fn bare_name_is_not_path() {
        assert!(!is_path_target("OrderService"));
        assert!(!is_path_target("MyFlow"));
    }

    #[test]
    fn metadata_type_from_cls() {
        assert_eq!(
            metadata_type_from_path(Path::new("Foo.cls")).unwrap(),
            MetadataType::Apex
        );
    }

    #[test]
    fn metadata_type_from_trigger() {
        assert_eq!(
            metadata_type_from_path(Path::new("Foo.trigger")).unwrap(),
            MetadataType::Triggers
        );
    }

    #[test]
    fn metadata_type_from_flow() {
        assert_eq!(
            metadata_type_from_path(Path::new("My_Flow.flow-meta.xml")).unwrap(),
            MetadataType::Flows
        );
    }

    #[test]
    fn metadata_type_unknown_extension() {
        assert!(metadata_type_from_path(Path::new("Foo.java")).is_err());
    }

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn levenshtein_one_edit() {
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "axc"), 1);
    }

    #[test]
    fn levenshtein_two_edits() {
        assert_eq!(levenshtein_distance("abc", "a"), 2);
    }

    #[test]
    fn find_similar_names_finds_close_matches() {
        let candidates = vec!["OrderService".to_string(), "AccountHelper".to_string()];
        let result = find_similar_names("OrderServce", &candidates);
        assert_eq!(result, vec!["OrderService".to_string()]);
    }

    #[test]
    fn find_similar_names_no_matches() {
        let candidates = vec!["OrderService".to_string()];
        let result = find_similar_names("CompletelyDifferent", &candidates);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_path_target_file_not_found() {
        let result = resolve_path_target("/nonexistent/path/Foo.cls");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn detect_format_html_when_index_html_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.html"), "<html></html>").unwrap();
        assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Html);
    }

    #[test]
    fn detect_format_markdown_when_index_md_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.md"), "# Index").unwrap();
        assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Markdown);
    }

    #[test]
    fn detect_format_defaults_to_markdown() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(detect_output_format(tmp.path(), &None), OutputFormat::Markdown);
    }

    #[test]
    fn detect_format_explicit_override() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("index.md"), "# Index").unwrap();
        assert_eq!(
            detect_output_format(tmp.path(), &Some(OutputFormat::Html)),
            OutputFormat::Html
        );
    }
}
