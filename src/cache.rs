use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use crate::types::{
    ClassDocumentation, FlowDocumentation, LwcDocumentation, ObjectDocumentation,
    TriggerDocumentation, ValidationRuleDocumentation,
};

const CACHE_FILE: &str = ".sfdoc-cache.json";

// ---------------------------------------------------------------------------
// Cache types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
    /// Trigger entries are in a separate map so the field can be absent in
    /// cache files written before trigger support was added.
    #[serde(default)]
    trigger_entries: HashMap<String, TriggerCacheEntry>,
    /// Flow entries are in a separate map so the field can be absent in
    /// cache files written before flow support was added.
    #[serde(default)]
    flow_entries: HashMap<String, FlowCacheEntry>,
    /// Validation rule entries are in a separate map so the field can be absent in
    /// cache files written before validation rule support was added.
    #[serde(default)]
    validation_rule_entries: HashMap<String, ValidationRuleCacheEntry>,
    /// Object entries are in a separate map so the field can be absent in
    /// cache files written before object support was added.
    #[serde(default)]
    object_entries: HashMap<String, ObjectCacheEntry>,
    /// LWC entries are in a separate map so the field can be absent in
    /// cache files written before LWC support was added.
    #[serde(default)]
    lwc_entries: HashMap<String, LwcCacheEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TriggerCacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: TriggerDocumentation,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FlowCacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: FlowDocumentation,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidationRuleCacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: ValidationRuleDocumentation,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectCacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: ObjectDocumentation,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LwcCacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: LwcDocumentation,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub hash: String,
    pub model: String,
    pub documentation: ClassDocumentation,
}

impl Cache {
    /// Load the cache from the output directory. Returns an empty cache if the
    /// file doesn't exist or can't be parsed (e.g. after a format change).
    pub fn load(output_dir: &Path) -> Self {
        let path = output_dir.join(CACHE_FILE);
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!(
                    "Warning: could not read cache file at {}: {e}",
                    path.display()
                );
                return Self::default();
            }
        };
        match serde_json::from_str(&data) {
            Ok(cache) => cache,
            Err(e) => {
                eprintln!(
                    "Warning: cache file at {} is corrupt and will be ignored: {e}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Persist the cache to the output directory.
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(output_dir.join(CACHE_FILE), data)?;
        Ok(())
    }

    /// Returns the cached entry if the hash and model both match (i.e. the
    /// source file hasn't changed and was generated with the same model).
    pub fn get_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a CacheEntry> {
        self.entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update a class entry after a successful API call.
    pub fn update(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: ClassDocumentation,
    ) {
        self.entries.insert(
            key,
            CacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }

    /// Returns the cached trigger entry if hash and model both match.
    pub fn get_trigger_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a TriggerCacheEntry> {
        self.trigger_entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update a trigger entry after a successful API call.
    pub fn update_trigger(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: TriggerDocumentation,
    ) {
        self.trigger_entries.insert(
            key,
            TriggerCacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }

    /// Returns the cached flow entry if hash and model both match.
    pub fn get_flow_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a FlowCacheEntry> {
        self.flow_entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update a flow entry after a successful API call.
    pub fn update_flow(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: FlowDocumentation,
    ) {
        self.flow_entries.insert(
            key,
            FlowCacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }

    /// Returns the cached validation rule entry if hash and model both match.
    pub fn get_validation_rule_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a ValidationRuleCacheEntry> {
        self.validation_rule_entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update a validation rule entry after a successful API call.
    pub fn update_validation_rule(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: ValidationRuleDocumentation,
    ) {
        self.validation_rule_entries.insert(
            key,
            ValidationRuleCacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }

    /// Returns the cached object entry if hash and model both match.
    pub fn get_object_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a ObjectCacheEntry> {
        self.object_entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update an object entry after a successful API call.
    pub fn update_object(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: ObjectDocumentation,
    ) {
        self.object_entries.insert(
            key,
            ObjectCacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }

    /// Returns the cached LWC entry if hash and model both match.
    pub fn get_lwc_if_fresh<'a>(
        &'a self,
        key: &str,
        hash: &str,
        model: &str,
    ) -> Option<&'a LwcCacheEntry> {
        self.lwc_entries
            .get(key)
            .filter(|e| e.hash == hash && e.model == model)
    }

    /// Insert or update an LWC entry after a successful API call.
    pub fn update_lwc(
        &mut self,
        key: String,
        hash: String,
        model: &str,
        documentation: LwcDocumentation,
    ) {
        self.lwc_entries.insert(
            key,
            LwcCacheEntry {
                hash,
                model: model.to_owned(),
                documentation,
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Returns the SHA-256 hex digest of the given source string.
pub fn hash_source(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_source("public class Foo {}");
        let h2 = hash_source("public class Foo {}");
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_sources_produce_different_hashes() {
        let h1 = hash_source("public class Foo {}");
        let h2 = hash_source("public class Bar {}");
        assert_ne!(h1, h2);
    }

    #[test]
    fn get_if_fresh_returns_none_for_wrong_hash() {
        let mut cache = Cache::default();
        let doc = ClassDocumentation {
            class_name: "Foo".to_string(),
            summary: "".to_string(),
            description: "".to_string(),
            methods: vec![],
            properties: vec![],
            usage_examples: vec![],
            relationships: vec![],
        };
        cache.update("Foo.cls".to_string(), "abc".to_string(), "gpt-4o", doc);
        assert!(cache
            .get_if_fresh("Foo.cls", "different", "gpt-4o")
            .is_none());
        assert!(cache
            .get_if_fresh("Foo.cls", "abc", "other-model")
            .is_none());
        assert!(cache.get_if_fresh("Foo.cls", "abc", "gpt-4o").is_some());
    }

    #[test]
    fn save_and_load_round_trips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = ClassDocumentation {
            class_name: "Foo".to_string(),
            summary: "A foo class.".to_string(),
            description: "Detailed description.".to_string(),
            methods: vec![],
            properties: vec![],
            usage_examples: vec![],
            relationships: vec![],
        };
        cache.update(
            "Foo.cls".to_string(),
            "deadbeef".to_string(),
            "gemini-2.5-flash",
            doc,
        );
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        let entry = loaded
            .get_if_fresh("Foo.cls", "deadbeef", "gemini-2.5-flash")
            .unwrap();
        assert_eq!(entry.documentation.class_name, "Foo");
        assert_eq!(entry.documentation.summary, "A foo class.");
    }
}
