use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

use crate::types::SourceFile;

pub trait FileScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>>;
}

pub struct ApexScanner;

/// Scans a directory tree for Apex trigger files (`.trigger`).
pub struct TriggerScanner;

/// Scans a directory tree for Salesforce Flow files (`.flow-meta.xml`).
pub struct FlowScanner;

/// Scans a directory tree for Salesforce Validation Rule files (`.validationRule-meta.xml`).
pub struct ValidationRuleScanner;

/// Scans a directory tree for Salesforce Custom Object files (`.object-meta.xml`).
pub struct ObjectScanner;

/// Scans a directory tree for Lightning Web Component roots (`*.js-meta.xml` under `lwc/`).
pub struct LwcScanner;

/// Returns `true` if WalkDir should descend into (or keep) this entry.
/// Prunes common noise directories to reduce unnecessary syscalls.
fn should_visit(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_dir() {
        let name = entry.file_name().to_str().unwrap_or("");
        !matches!(name, ".git" | ".sfdx" | "node_modules" | "target")
    } else {
        true
    }
}

impl FileScanner for ApexScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Only process .cls files; skip -meta.xml companion files
            if !file_name.ends_with(".cls") || file_name.contains("-meta.xml") {
                continue;
            }

            let raw_source = std::fs::read_to_string(path)?;

            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }

        // Sort for deterministic output order
        files.sort_by(|a, b| a.filename.cmp(&b.filename));

        Ok(files)
    }
}

impl FileScanner for TriggerScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".trigger") || file_name.contains("-meta.xml") {
                continue;
            }
            let raw_source = std::fs::read_to_string(path)?;
            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }

        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }
}

impl FileScanner for FlowScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".flow-meta.xml") {
                continue;
            }
            let raw_source = std::fs::read_to_string(path)?;
            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }
}

impl FileScanner for ValidationRuleScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".validationRule-meta.xml") {
                continue;
            }
            let raw_source = std::fs::read_to_string(path)?;
            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }
}

impl FileScanner for ObjectScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".object-meta.xml") {
                continue;
            }
            let raw_source = std::fs::read_to_string(path)?;
            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }
}

impl FileScanner for LwcScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(should_visit)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            // Only process LWC meta files
            if !file_name.ends_with(".js-meta.xml") {
                continue;
            }
            // Ensure the parent directory is inside an `lwc/` directory
            let in_lwc_dir = path
                .ancestors()
                .any(|a| a.file_name().and_then(|n| n.to_str()) == Some("lwc"));
            if !in_lwc_dir {
                continue;
            }

            // Use the sibling .js file as raw_source if it exists (for cache hashing and AI prompt).
            // Fall back to the meta.xml content when there is no JS file.
            let component_name = file_name.trim_end_matches(".js-meta.xml");
            let js_path = path
                .parent()
                .map(|p| p.join(format!("{component_name}.js")));
            let raw_source = match js_path
                .as_deref()
                .and_then(|p| std::fs::read_to_string(p).ok())
            {
                Some(js) => js,
                None => std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read LWC source for {}", file_name))?,
            };

            files.push(SourceFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn finds_cls_files() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "AccountService.cls",
            "public class AccountService {}",
        );
        write_file(tmp.path(), "AccountService.cls-meta.xml", "<ApexClass/>");
        write_file(tmp.path(), "README.md", "docs");

        let scanner = ApexScanner;
        let files = scanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "AccountService.cls");
    }

    #[test]
    fn recurses_into_subdirectories() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("triggers");
        fs::create_dir(&sub).unwrap();
        write_file(
            tmp.path(),
            "AccountService.cls",
            "public class AccountService {}",
        );
        write_file(
            &sub,
            "AccountTrigger.cls",
            "trigger AccountTrigger on Account (before insert) {}",
        );

        let scanner = ApexScanner;
        let files = scanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn finds_trigger_files() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "AccountTrigger.trigger",
            "trigger AccountTrigger on Account (before insert) {}",
        );
        write_file(
            tmp.path(),
            "AccountTrigger.trigger-meta.xml",
            "<ApexTrigger/>",
        );
        write_file(
            tmp.path(),
            "AccountService.cls",
            "public class AccountService {}",
        );

        let scanner = TriggerScanner;
        let files = scanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "AccountTrigger.trigger");
    }

    #[test]
    fn returns_sorted_filenames() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Zebra.cls", "public class Zebra {}");
        write_file(tmp.path(), "Alpha.cls", "public class Alpha {}");

        let scanner = ApexScanner;
        let files = scanner.scan(tmp.path()).unwrap();

        assert_eq!(files[0].filename, "Alpha.cls");
        assert_eq!(files[1].filename, "Zebra.cls");
    }

    // -----------------------------------------------------------------------
    // FlowScanner
    // -----------------------------------------------------------------------

    #[test]
    fn flow_scanner_finds_flow_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Account_Flow.flow-meta.xml", "<Flow/>");
        write_file(
            tmp.path(),
            "AccountService.cls",
            "public class AccountService {}",
        );
        write_file(
            tmp.path(),
            "AccountTrigger.trigger",
            "trigger AccountTrigger on Account (before insert) {}",
        );

        let files = FlowScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Account_Flow.flow-meta.xml");
    }

    #[test]
    fn flow_scanner_returns_sorted_output() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Zebra_Flow.flow-meta.xml", "<Flow/>");
        write_file(tmp.path(), "Alpha_Flow.flow-meta.xml", "<Flow/>");

        let files = FlowScanner.scan(tmp.path()).unwrap();

        assert_eq!(files[0].filename, "Alpha_Flow.flow-meta.xml");
        assert_eq!(files[1].filename, "Zebra_Flow.flow-meta.xml");
    }

    // -----------------------------------------------------------------------
    // ValidationRuleScanner
    // -----------------------------------------------------------------------

    #[test]
    fn validation_rule_scanner_finds_vr_files() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = tmp.path().join("objects").join("Account");
        fs::create_dir_all(&obj_dir).unwrap();
        write_file(
            &obj_dir,
            "Require_Name.validationRule-meta.xml",
            "<ValidationRule/>",
        );
        // Other file types should be ignored
        write_file(
            tmp.path(),
            "AccountService.cls",
            "public class AccountService {}",
        );

        let files = ValidationRuleScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Require_Name.validationRule-meta.xml");
    }

    // -----------------------------------------------------------------------
    // ObjectScanner
    // -----------------------------------------------------------------------

    #[test]
    fn object_scanner_finds_object_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Account__c.object-meta.xml", "<CustomObject/>");
        // Other types should not be picked up
        write_file(tmp.path(), "Account__c.cls", "public class Account__c {}");

        let files = ObjectScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Account__c.object-meta.xml");
    }

    #[test]
    fn object_scanner_returns_sorted_output() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Zebra__c.object-meta.xml", "<CustomObject/>");
        write_file(tmp.path(), "Alpha__c.object-meta.xml", "<CustomObject/>");

        let files = ObjectScanner.scan(tmp.path()).unwrap();

        assert_eq!(files[0].filename, "Alpha__c.object-meta.xml");
        assert_eq!(files[1].filename, "Zebra__c.object-meta.xml");
    }

    // -----------------------------------------------------------------------
    // LwcScanner
    // -----------------------------------------------------------------------

    #[test]
    fn lwc_scanner_finds_meta_files_under_lwc_dir() {
        let tmp = TempDir::new().unwrap();
        let comp_dir = tmp.path().join("lwc").join("myButton");
        fs::create_dir_all(&comp_dir).unwrap();
        write_file(
            &comp_dir,
            "myButton.js-meta.xml",
            "<LightningComponentBundle/>",
        );
        write_file(
            &comp_dir,
            "myButton.js",
            "import { LightningElement } from 'lwc';",
        );
        // A .js-meta.xml outside lwc/ should be ignored
        write_file(
            tmp.path(),
            "other.js-meta.xml",
            "<LightningComponentBundle/>",
        );

        let files = LwcScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "myButton.js-meta.xml");
    }

    #[test]
    fn lwc_scanner_uses_sibling_js_as_raw_source() {
        let tmp = TempDir::new().unwrap();
        let comp_dir = tmp.path().join("lwc").join("myButton");
        fs::create_dir_all(&comp_dir).unwrap();
        write_file(
            &comp_dir,
            "myButton.js-meta.xml",
            "<LightningComponentBundle/>",
        );
        write_file(&comp_dir, "myButton.js", "export default class MyButton {}");

        let files = LwcScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].raw_source, "export default class MyButton {}");
    }

    #[test]
    fn lwc_scanner_falls_back_to_meta_xml_when_no_js() {
        let tmp = TempDir::new().unwrap();
        let comp_dir = tmp.path().join("lwc").join("myButton");
        fs::create_dir_all(&comp_dir).unwrap();
        write_file(
            &comp_dir,
            "myButton.js-meta.xml",
            "<LightningComponentBundle/>",
        );
        // No .js file present

        let files = LwcScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].raw_source, "<LightningComponentBundle/>");
    }

    #[test]
    fn lwc_scanner_ignores_meta_xml_outside_lwc_dir() {
        let tmp = TempDir::new().unwrap();
        // A .js-meta.xml directly under a non-lwc directory should be skipped
        let other_dir = tmp.path().join("aura").join("myComp");
        fs::create_dir_all(&other_dir).unwrap();
        write_file(&other_dir, "myComp.js-meta.xml", "<AuraDefinitionBundle/>");

        let files = LwcScanner.scan(tmp.path()).unwrap();

        assert!(
            files.is_empty(),
            "expected no files, got {:?}",
            files.iter().map(|f| &f.filename).collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // Excluded directories
    // -----------------------------------------------------------------------

    #[test]
    fn scanner_skips_dot_git_directory() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        write_file(&git_dir, "SomeClass.cls", "public class SomeClass {}");
        write_file(tmp.path(), "RealClass.cls", "public class RealClass {}");

        let files = ApexScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "RealClass.cls");
    }

    #[test]
    fn scanner_skips_node_modules_directory() {
        let tmp = TempDir::new().unwrap();
        let nm_dir = tmp.path().join("node_modules").join("some-package");
        fs::create_dir_all(&nm_dir).unwrap();
        write_file(&nm_dir, "Hidden.cls", "public class Hidden {}");
        write_file(tmp.path(), "Visible.cls", "public class Visible {}");

        let files = ApexScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Visible.cls");
    }

    #[test]
    fn scanner_skips_target_directory() {
        let tmp = TempDir::new().unwrap();
        let target_dir = tmp.path().join("target").join("debug");
        fs::create_dir_all(&target_dir).unwrap();
        write_file(&target_dir, "Generated.cls", "public class Generated {}");
        write_file(tmp.path(), "Real.cls", "public class Real {}");

        let files = ApexScanner.scan(tmp.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Real.cls");
    }
}
