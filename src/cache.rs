use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;

use crate::cli::MetadataType;
use crate::types::{
    AuraDocumentation, ClassDocumentation, FlexiPageDocumentation, FlowDocumentation,
    LwcDocumentation, ObjectDocumentation, TriggerDocumentation, ValidationRuleDocumentation,
};

const CACHE_FILE: &str = ".sfdoc-cache.json";
const CACHE_TMP_FILE: &str = ".sfdoc-cache.json.tmp";

/// Bump this when the cache schema changes to force a full rebuild.
const CACHE_VERSION: u32 = 2;

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

#[derive(Serialize, Deserialize)]
pub struct Cache {
    /// Schema version — the entire cache is discarded when this doesn't match CACHE_VERSION.
    #[serde(default)]
    cache_version: u32,
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

impl Default for Cache {
    fn default() -> Self {
        Self {
            cache_version: CACHE_VERSION,
            entries: HashMap::default(),
            trigger_entries: HashMap::default(),
            flow_entries: HashMap::default(),
            validation_rule_entries: HashMap::default(),
            object_entries: HashMap::default(),
            lwc_entries: HashMap::default(),
            flexipage_entries: HashMap::default(),
            aura_entries: HashMap::default(),
        }
    }
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
        match serde_json::from_str::<Cache>(&data) {
            Ok(cache) if cache.cache_version == CACHE_VERSION => cache,
            Ok(cache) => {
                eprintln!(
                    "Warning: cache version mismatch (found {}, expected {}) — rebuilding",
                    cache.cache_version, CACHE_VERSION
                );
                Self::default()
            }
            Err(e) => {
                eprintln!(
                    "Warning: cache file at {} is corrupt and will be ignored: {e}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Persist the cache to the output directory via atomic write.
    ///
    /// Writes to a temporary file first, then renames it into place. This
    /// guarantees the cache file is always either the old or new version —
    /// never a partial write — even if the process crashes mid-save.
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let data = serde_json::to_string_pretty(self)?;
        let final_path = output_dir.join(CACHE_FILE);
        let tmp_path = output_dir.join(CACHE_TMP_FILE);
        std::fs::write(&tmp_path, &data)?;
        std::fs::rename(&tmp_path, &final_path)?;
        Ok(())
    }

    /// Check cache freshness for any AI-documented metadata type.
    /// CustomMetadata is not cached and always returns false.
    pub fn is_fresh(
        &self,
        metadata_type: MetadataType,
        key: &str,
        hash: &str,
        model: &str,
    ) -> bool {
        match metadata_type {
            MetadataType::Apex => self.get_if_fresh(key, hash, model).is_some(),
            MetadataType::Triggers => self.get_trigger_if_fresh(key, hash, model).is_some(),
            MetadataType::Flows => self.get_flow_if_fresh(key, hash, model).is_some(),
            MetadataType::ValidationRules => self
                .get_validation_rule_if_fresh(key, hash, model)
                .is_some(),
            MetadataType::Objects => self.get_object_if_fresh(key, hash, model).is_some(),
            MetadataType::Lwc => self.get_lwc_if_fresh(key, hash, model).is_some(),
            MetadataType::Flexipages => self.get_flexipage_if_fresh(key, hash, model).is_some(),
            MetadataType::Aura => self.get_aura_if_fresh(key, hash, model).is_some(),
            MetadataType::CustomMetadata => false,
        }
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

    /// Iterators over all cached entries, for rebuilding AllNames and the index.
    pub fn class_entries(&self) -> impl Iterator<Item = (&String, &CacheEntry)> {
        self.entries.iter()
    }

    pub fn trigger_entries(&self) -> impl Iterator<Item = (&String, &TriggerCacheEntry)> {
        self.trigger_entries.iter()
    }

    pub fn flow_entries(&self) -> impl Iterator<Item = (&String, &FlowCacheEntry)> {
        self.flow_entries.iter()
    }

    pub fn validation_rule_entries(
        &self,
    ) -> impl Iterator<Item = (&String, &ValidationRuleCacheEntry)> {
        self.validation_rule_entries.iter()
    }

    pub fn object_entries(&self) -> impl Iterator<Item = (&String, &ObjectCacheEntry)> {
        self.object_entries.iter()
    }

    pub fn lwc_entries(&self) -> impl Iterator<Item = (&String, &LwcCacheEntry)> {
        self.lwc_entries.iter()
    }

    pub fn flexipage_entries(&self) -> impl Iterator<Item = (&String, &FlexiPageCacheEntry)> {
        self.flexipage_entries.iter()
    }

    pub fn aura_entries(&self) -> impl Iterator<Item = (&String, &AuraCacheEntry)> {
        self.aura_entries.iter()
    }

    /// Drop cache entries of `metadata_type` whose keys are not in `keep`.
    /// Custom metadata is not cached, so this is a no-op for that type.
    pub fn retain_type(&mut self, metadata_type: MetadataType, keep: &HashSet<String>) {
        match metadata_type {
            MetadataType::Apex => self.entries.retain(|k, _| keep.contains(k)),
            MetadataType::Triggers => self.trigger_entries.retain(|k, _| keep.contains(k)),
            MetadataType::Flows => self.flow_entries.retain(|k, _| keep.contains(k)),
            MetadataType::ValidationRules => {
                self.validation_rule_entries.retain(|k, _| keep.contains(k))
            }
            MetadataType::Objects => self.object_entries.retain(|k, _| keep.contains(k)),
            MetadataType::Lwc => self.lwc_entries.retain(|k, _| keep.contains(k)),
            MetadataType::Flexipages => self.flexipage_entries.retain(|k, _| keep.contains(k)),
            MetadataType::Aura => self.aura_entries.retain(|k, _| keep.contains(k)),
            MetadataType::CustomMetadata => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Stable cache key: path relative to `source_dir`, forward slashes.
/// Falls back to the path as given when it is not under `source_dir`.
pub fn cache_key(path: &Path, source_dir: &Path) -> String {
    path.strip_prefix(source_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn append_sibling(combined: &mut String, path: &Path, filename: &str) {
    if let Some(parent) = path.parent() {
        if let Ok(content) = std::fs::read_to_string(parent.join(filename)) {
            combined.push('\0');
            combined.push_str(&content);
        }
    }
}

/// Hash an LWC component: JS source plus sibling HTML and js-meta.xml.
pub fn hash_lwc_source(meta_path: &Path, js_source: &str) -> String {
    let mut combined = js_source.to_string();
    let component_name = meta_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");
    append_sibling(&mut combined, meta_path, &format!("{component_name}.html"));
    if let Ok(meta_xml) = std::fs::read_to_string(meta_path) {
        combined.push('\0');
        combined.push_str(&meta_xml);
    }
    hash_source(&combined)
}

/// Hash an Aura component: .cmp markup plus sibling controller JS.
pub fn hash_aura_source(cmp_path: &Path, cmp_source: &str) -> String {
    let mut combined = cmp_source.to_string();
    let component_name = cmp_path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    append_sibling(&mut combined, cmp_path, &format!("{component_name}.js"));
    append_sibling(
        &mut combined,
        cmp_path,
        &format!("{component_name}Controller.js"),
    );
    append_sibling(
        &mut combined,
        cmp_path,
        &format!("{component_name}Helper.js"),
    );
    hash_source(&combined)
}

/// Hash a custom object: object XML plus sibling `fields/*.field-meta.xml`.
pub fn hash_object_source(path: &Path, object_xml: &str) -> String {
    let mut combined = object_xml.to_string();
    if let Some(fields_dir) = path.parent().map(|p| p.join("fields")) {
        if let Ok(entries) = std::fs::read_dir(&fields_dir) {
            let mut field_contents: Vec<(String, String)> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.ends_with(".field-meta.xml"))
                })
                .filter_map(|e| {
                    let path = e.path();
                    let name = path.file_name()?.to_str()?.to_string();
                    let content = std::fs::read_to_string(&path).ok()?;
                    Some((name, content))
                })
                .collect();
            field_contents.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, content) in field_contents {
                combined.push_str(&content);
            }
        }
    }
    hash_source(&combined)
}

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

    use crate::types::{
        FlowDocumentation, LwcDocumentation, ObjectDocumentation, TriggerDocumentation,
        ValidationRuleDocumentation,
    };

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn load_from_nonexistent_dir_returns_empty_cache() {
        let cache = Cache::load(std::path::Path::new(
            "/nonexistent/path/that/does/not/exist",
        ));
        assert!(cache.get_if_fresh("anything", "hash", "model").is_none());
    }

    #[test]
    fn load_corrupt_json_returns_empty_cache() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".sfdoc-cache.json"), "not valid json {{{}").unwrap();
        let cache = Cache::load(tmp.path());
        assert!(cache.get_if_fresh("anything", "hash", "model").is_none());
    }

    #[test]
    fn load_empty_json_object_returns_empty_cache() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".sfdoc-cache.json"), "{}").unwrap();
        let cache = Cache::load(tmp.path());
        assert!(cache.get_if_fresh("anything", "hash", "model").is_none());
    }

    #[test]
    fn backward_compatible_load_missing_trigger_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let old_cache = r#"{"entries":{}}"#;
        std::fs::write(tmp.path().join(".sfdoc-cache.json"), old_cache).unwrap();
        let cache = Cache::load(tmp.path());
        assert!(cache.get_trigger_if_fresh("any", "hash", "model").is_none());
        assert!(cache.get_flow_if_fresh("any", "hash", "model").is_none());
        assert!(cache.get_lwc_if_fresh("any", "hash", "model").is_none());
    }

    #[test]
    fn trigger_cache_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = TriggerDocumentation {
            trigger_name: "AccountTrigger".to_string(),
            sobject: "Account".to_string(),
            summary: "Handles account events.".to_string(),
            description: "Detailed desc.".to_string(),
            events: vec![],
            handler_classes: vec![],
            usage_notes: vec![],
            relationships: vec![],
        };
        cache.update_trigger(
            "AccountTrigger.trigger".to_string(),
            "abc123".to_string(),
            "model-1",
            doc,
        );
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        let entry = loaded
            .get_trigger_if_fresh("AccountTrigger.trigger", "abc123", "model-1")
            .unwrap();
        assert_eq!(entry.documentation.trigger_name, "AccountTrigger");
    }

    #[test]
    fn flow_cache_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = FlowDocumentation {
            api_name: "My_Flow".to_string(),
            label: "My Flow".to_string(),
            summary: "Does stuff.".to_string(),
            description: "Detailed.".to_string(),
            business_process: "Onboarding".to_string(),
            entry_criteria: "New account".to_string(),
            key_decisions: vec![],
            admin_notes: vec![],
            relationships: vec![],
        };
        cache.update_flow("My_Flow".to_string(), "hash1".to_string(), "model-1", doc);
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        assert!(loaded
            .get_flow_if_fresh("My_Flow", "hash1", "model-1")
            .is_some());
        assert!(loaded
            .get_flow_if_fresh("My_Flow", "wrong", "model-1")
            .is_none());
    }

    #[test]
    fn validation_rule_cache_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = ValidationRuleDocumentation {
            rule_name: "Rule1".to_string(),
            object_name: "Account".to_string(),
            summary: "Validates email.".to_string(),
            when_fires: "On save".to_string(),
            what_protects: "Data quality".to_string(),
            formula_explanation: "Checks email".to_string(),
            edge_cases: vec![],
            relationships: vec![],
        };
        cache.update_validation_rule("Rule1".to_string(), "h1".to_string(), "m1", doc);
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        assert!(loaded
            .get_validation_rule_if_fresh("Rule1", "h1", "m1")
            .is_some());
    }

    #[test]
    fn object_cache_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = ObjectDocumentation {
            object_name: "Invoice__c".to_string(),
            label: "Invoice".to_string(),
            summary: "Tracks invoices.".to_string(),
            description: "Detailed.".to_string(),
            purpose: "Billing".to_string(),
            key_fields: vec![],
            relationships: vec![],
            admin_notes: vec![],
        };
        cache.update_object("Invoice__c".to_string(), "h2".to_string(), "m2", doc);
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        assert!(loaded
            .get_object_if_fresh("Invoice__c", "h2", "m2")
            .is_some());
    }

    #[test]
    fn lwc_cache_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut cache = Cache::default();
        let doc = LwcDocumentation {
            component_name: "myButton".to_string(),
            summary: "A button.".to_string(),
            description: "Detailed.".to_string(),
            api_props: vec![],
            usage_notes: vec![],
            relationships: vec![],
        };
        cache.update_lwc("myButton".to_string(), "h3".to_string(), "m3", doc);
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        assert!(loaded.get_lwc_if_fresh("myButton", "h3", "m3").is_some());
    }

    #[test]
    fn hash_source_empty_string() {
        let h = hash_source("");
        assert_eq!(h.len(), 64);
        assert_eq!(h, hash_source(""));
    }

    #[test]
    fn hash_source_unicode_content() {
        let h = hash_source("public class Über {}");
        assert_eq!(h.len(), 64);
        assert_ne!(h, hash_source("public class Uber {}"));
    }

    #[test]
    fn class_entries_returns_all_class_docs() {
        let mut cache = Cache::default();
        let doc = ClassDocumentation {
            class_name: "Foo".to_string(),
            summary: "A foo.".to_string(),
            description: "".to_string(),
            methods: vec![],
            properties: vec![],
            usage_examples: vec![],
            relationships: vec![],
        };
        cache.update("Foo.cls".to_string(), "h1".to_string(), "m1", doc);
        let entries: Vec<_> = cache.class_entries().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.documentation.class_name, "Foo");
    }

    #[test]
    fn trigger_entries_returns_all_trigger_docs() {
        let mut cache = Cache::default();
        let doc = TriggerDocumentation {
            trigger_name: "AccTrig".to_string(),
            sobject: "Account".to_string(),
            summary: "".to_string(),
            description: "".to_string(),
            events: vec![],
            handler_classes: vec![],
            usage_notes: vec![],
            relationships: vec![],
        };
        cache.update_trigger("AccTrig.trigger".to_string(), "h1".to_string(), "m1", doc);
        let entries: Vec<_> = cache.trigger_entries().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.documentation.trigger_name, "AccTrig");
    }

    #[test]
    fn overwrite_existing_cache_entry() {
        let mut cache = Cache::default();
        let doc1 = ClassDocumentation {
            class_name: "Foo".to_string(),
            summary: "Version 1".to_string(),
            description: "".to_string(),
            methods: vec![],
            properties: vec![],
            usage_examples: vec![],
            relationships: vec![],
        };
        cache.update("Foo.cls".to_string(), "hash1".to_string(), "model", doc1);
        let doc2 = ClassDocumentation {
            class_name: "Foo".to_string(),
            summary: "Version 2".to_string(),
            description: "".to_string(),
            methods: vec![],
            properties: vec![],
            usage_examples: vec![],
            relationships: vec![],
        };
        cache.update("Foo.cls".to_string(), "hash2".to_string(), "model", doc2);

        assert!(cache.get_if_fresh("Foo.cls", "hash1", "model").is_none());
        assert_eq!(
            cache
                .get_if_fresh("Foo.cls", "hash2", "model")
                .unwrap()
                .documentation
                .summary,
            "Version 2"
        );
    }

    #[test]
    fn save_is_atomic_no_tmp_file_left() {
        let tmp = tempfile::TempDir::new().unwrap();
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
        cache.update("Foo.cls".to_string(), "h1".to_string(), "m1", doc);
        cache.save(tmp.path()).unwrap();

        // The final cache file exists
        assert!(tmp.path().join(CACHE_FILE).exists());
        // The temp file was renamed away — it should not exist
        assert!(!tmp.path().join(CACHE_TMP_FILE).exists());
    }

    #[test]
    fn cache_key_is_relative_to_source_dir() {
        let source = std::path::Path::new("/proj/force-app/main/default");
        let file = source.join("classes/AccountService.cls");
        assert_eq!(cache_key(&file, source), "classes/AccountService.cls");
    }

    #[test]
    fn cache_key_falls_back_when_not_under_source_dir() {
        let key = cache_key(
            std::path::Path::new("/other/Foo.cls"),
            std::path::Path::new("/proj"),
        );
        assert!(key.ends_with("Foo.cls"), "got {key}");
        assert!(!key.contains('\\'));
    }

    #[test]
    fn hash_lwc_changes_when_html_changes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comp = tmp.path().join("myButton");
        std::fs::create_dir_all(&comp).unwrap();
        let meta = comp.join("myButton.js-meta.xml");
        std::fs::write(&meta, "<LightningComponentBundle/>").unwrap();
        std::fs::write(comp.join("myButton.html"), "<template></template>").unwrap();
        let js = "export default class MyButton {}";
        let h1 = hash_lwc_source(&meta, js);
        std::fs::write(
            comp.join("myButton.html"),
            "<template><slot></slot></template>",
        )
        .unwrap();
        let h2 = hash_lwc_source(&meta, js);
        assert_ne!(h1, h2, "HTML change must invalidate the LWC cache hash");
    }

    #[test]
    fn hash_aura_changes_when_cmp_changes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comp = tmp.path().join("myComp");
        std::fs::create_dir_all(&comp).unwrap();
        let cmp = comp.join("myComp.cmp");
        std::fs::write(&cmp, "<aura:component></aura:component>").unwrap();
        std::fs::write(comp.join("myComp.js"), "({ helper: function() {} })").unwrap();
        let h1 = hash_aura_source(&cmp, "<aura:component></aura:component>");
        let h2 = hash_aura_source(
            &cmp,
            "<aura:component><aura:attribute name=\"x\" type=\"String\"/></aura:component>",
        );
        assert_ne!(h1, h2, "cmp change must invalidate the Aura cache hash");
    }

    #[test]
    fn hash_object_includes_field_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let obj_dir = tmp.path().join("Invoice__c");
        std::fs::create_dir_all(obj_dir.join("fields")).unwrap();
        let obj = obj_dir.join("Invoice__c.object-meta.xml");
        std::fs::write(&obj, "<CustomObject/>").unwrap();
        let xml = "<CustomObject/>";
        let h1 = hash_object_source(&obj, xml);
        std::fs::write(
            obj_dir.join("fields/Amount__c.field-meta.xml"),
            "<CustomField><fullName>Amount__c</fullName></CustomField>",
        )
        .unwrap();
        let h2 = hash_object_source(&obj, xml);
        assert_ne!(
            h1, h2,
            "new field file must invalidate the object cache hash"
        );
    }

    #[test]
    fn retain_type_drops_missing_keys() {
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
        cache.update("classes/Foo.cls".into(), "h1".into(), "m", doc.clone());
        cache.update("classes/Bar.cls".into(), "h1".into(), "m", doc);
        let mut keep = std::collections::HashSet::new();
        keep.insert("classes/Foo.cls".to_string());
        cache.retain_type(crate::cli::MetadataType::Apex, &keep);
        assert!(cache.get_if_fresh("classes/Foo.cls", "h1", "m").is_some());
        assert!(cache.get_if_fresh("classes/Bar.cls", "h1", "m").is_none());
    }

    #[test]
    fn load_discards_mismatched_cache_version() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(CACHE_FILE),
            r#"{"cache_version":1,"entries":{"Foo.cls":{"hash":"x","model":"m","documentation":{"class_name":"Foo","summary":"old","description":"d"}}}}"#,
        )
        .unwrap();
        let cache = Cache::load(tmp.path());
        assert!(
            cache.get_if_fresh("Foo.cls", "x", "m").is_none(),
            "v1 cache must be discarded when CACHE_VERSION is 2"
        );
    }
}
