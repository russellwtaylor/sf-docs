use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

use crate::types::ApexFile;

pub trait FileScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>>;
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
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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

            files.push(ApexFile {
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
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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
            files.push(ApexFile {
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
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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
            files.push(ApexFile {
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
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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
            files.push(ApexFile {
                path: path.to_path_buf(),
                filename: file_name,
                raw_source,
            });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}

impl FileScanner for ObjectScanner {
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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
            files.push(ApexFile {
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
    fn scan(&self, source_dir: &Path) -> Result<Vec<ApexFile>> {
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
            let raw_source = js_path
                .as_deref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .or_else(|| std::fs::read_to_string(path).ok())
                .unwrap_or_default();

            files.push(ApexFile {
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
}
