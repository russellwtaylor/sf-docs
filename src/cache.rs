use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use crate::types::{
    AuraDocumentation, ClassDocumentation, FlexiPageDocumentation, FlowDocumentation,
    LwcDocumentation, ObjectDocumentation, TriggerDocumentation, ValidationRuleDocumentation,
};

const CACHE_FILE: &str = ".sfdoc-cache.json";

// ---------------------------------------------------------------------------
// Cache types
// ---------------------------------------------------------------------------

/// Generic cache entry holding a content hash, model name, and AI-generated docs.
#[derive(Serialize, Deserialize, Clone)]
pub struct TypedEntry<D> {
    pub hash: String,
    pub model: String,
    pub documentation: D,
}

/// Type aliases for each documentation kind — keeps call-site names stable.
pub type CacheEntry = TypedEntry<ClassDocumentation>;
pub type TriggerCacheEntry = TypedEntry<TriggerDocumentation>;
pub type FlowCacheEntry = TypedEntry<FlowDocumentation>;
pub type ValidationRuleCacheEntry = TypedEntry<ValidationRuleDocumentation>;
pub type ObjectCacheEntry = TypedEntry<ObjectDocumentation>;
pub type LwcCacheEntry = TypedEntry<LwcDocumentation>;
pub type FlexiPageCacheEntry = TypedEntry<FlexiPageDocumentation>;
pub type AuraCacheEntry = TypedEntry<AuraDocumentation>;

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
    /// FlexiPage entries are in a separate map so the field can be absent in
    /// cache files written before FlexiPage support was added.
    #[serde(default)]
    flexipage_entries: HashMap<String, FlexiPageCacheEntry>,
    /// Aura entries are in a separate map so the field can be absent in
    /// cache files written before Aura support was added.
    #[serde(default)]
    aura_entries: HashMap<String, AuraCacheEntry>,
}

/// Generates a `get_*_if_fresh` / `update_*` pair for a given HashMap field.
///
/// Usage: `cache_accessors!(field_name, EntryType, DocType, get_fn_name, update_fn_name);`
macro_rules! cache_accessors {
    ($field:ident, $entry:ty, $doc:ty, $get_fn:ident, $update_fn:ident) => {
        pub fn $get_fn<'a>(&'a self, key: &str, hash: &str, model: &str) -> Option<&'a $entry> {
            self.$field
                .get(key)
                .filter(|e| e.hash == hash && e.model == model)
        }

        pub fn $update_fn(&mut self, key: String, hash: String, model: &str, documentation: $doc) {
            self.$field.insert(
                key,
                TypedEntry {
                    hash,
                    model: model.to_owned(),
                    documentation,
                },
            );
        }
    };
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

    cache_accessors!(
        entries,
        CacheEntry,
        ClassDocumentation,
        get_if_fresh,
        update
    );
    cache_accessors!(
        trigger_entries,
        TriggerCacheEntry,
        TriggerDocumentation,
        get_trigger_if_fresh,
        update_trigger
    );
    cache_accessors!(
        flow_entries,
        FlowCacheEntry,
        FlowDocumentation,
        get_flow_if_fresh,
        update_flow
    );
    cache_accessors!(
        validation_rule_entries,
        ValidationRuleCacheEntry,
        ValidationRuleDocumentation,
        get_validation_rule_if_fresh,
        update_validation_rule
    );
    cache_accessors!(
        object_entries,
        ObjectCacheEntry,
        ObjectDocumentation,
        get_object_if_fresh,
        update_object
    );
    cache_accessors!(
        lwc_entries,
        LwcCacheEntry,
        LwcDocumentation,
        get_lwc_if_fresh,
        update_lwc
    );
    cache_accessors!(
        flexipage_entries,
        FlexiPageCacheEntry,
        FlexiPageDocumentation,
        get_flexipage_if_fresh,
        update_flexipage
    );
    cache_accessors!(
        aura_entries,
        AuraCacheEntry,
        AuraDocumentation,
        get_aura_if_fresh,
        update_aura
    );
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Returns the SHA-256 hex digest of the given source string.
pub fn hash_source(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
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
