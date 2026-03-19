use anyhow::Result;
use regex::Regex;
use std::sync::OnceLock;

use crate::apex_common::{re_type_ref, APEX_BUILTINS};
use crate::types::{TriggerEvent, TriggerMetadata};

// ---------------------------------------------------------------------------
// Compiled regex patterns
// ---------------------------------------------------------------------------

fn re_trigger() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)trigger\s+(?P<name>\w+)\s+on\s+(?P<sobject>\w+)\s*\((?P<events>[^)]+)\)")
            .unwrap()
    })
}

fn re_apexdoc() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/\*\*[\s\S]*?\*/").unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn parse_apex_trigger(source: &str) -> Result<TriggerMetadata> {
    let mut meta = TriggerMetadata {
        existing_comments: re_apexdoc()
            .find_iter(source)
            .map(|m| m.as_str().to_string())
            .collect(),
        ..Default::default()
    };

    if let Some(caps) = re_trigger().captures(source) {
        meta.trigger_name = caps.name("name").map_or("", |m| m.as_str()).to_string();
        meta.sobject = caps.name("sobject").map_or("", |m| m.as_str()).to_string();
        meta.events = caps
            .name("events")
            .map(|m| parse_events(m.as_str()))
            .unwrap_or_default();
    }

    // Collect PascalCase class references from the trigger body
    let mut refs: Vec<String> = re_type_ref()
        .captures_iter(source)
        .map(|c| c[1].to_string())
        .filter(|name| {
            !APEX_BUILTINS.contains(&name.as_str())
                && name != "Trigger"
                && name != &meta.trigger_name
                && name != &meta.sobject
                && name.len() > 2
        })
        .collect();
    refs.sort();
    refs.dedup();
    meta.references = refs;

    Ok(meta)
}

fn parse_events(events_str: &str) -> Vec<TriggerEvent> {
    events_str
        .split(',')
        .filter_map(|token| match token.trim().to_lowercase().as_str() {
            "before insert" => Some(TriggerEvent::BeforeInsert),
            "before update" => Some(TriggerEvent::BeforeUpdate),
            "before delete" => Some(TriggerEvent::BeforeDelete),
            "after insert" => Some(TriggerEvent::AfterInsert),
            "after update" => Some(TriggerEvent::AfterUpdate),
            "after delete" => Some(TriggerEvent::AfterDelete),
            "after undelete" => Some(TriggerEvent::AfterUndelete),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TRIGGER: &str = r#"
/**
 * Handles Account DML events.
 */
trigger AccountTrigger on Account (before insert, before update, after insert, after update) {
    AccountTriggerHandler handler = new AccountTriggerHandler();
    handler.run();
}
"#;

    #[test]
    fn parses_trigger_name() {
        let meta = parse_apex_trigger(SAMPLE_TRIGGER).unwrap();
        assert_eq!(meta.trigger_name, "AccountTrigger");
    }

    #[test]
    fn parses_sobject() {
        let meta = parse_apex_trigger(SAMPLE_TRIGGER).unwrap();
        assert_eq!(meta.sobject, "Account");
    }

    #[test]
    fn parses_events() {
        let meta = parse_apex_trigger(SAMPLE_TRIGGER).unwrap();
        assert!(meta.events.contains(&TriggerEvent::BeforeInsert));
        assert!(meta.events.contains(&TriggerEvent::BeforeUpdate));
        assert!(meta.events.contains(&TriggerEvent::AfterInsert));
        assert!(meta.events.contains(&TriggerEvent::AfterUpdate));
        assert_eq!(meta.events.len(), 4);
    }

    #[test]
    fn parses_references() {
        let meta = parse_apex_trigger(SAMPLE_TRIGGER).unwrap();
        assert!(meta
            .references
            .contains(&"AccountTriggerHandler".to_string()));
    }

    #[test]
    fn extracts_apexdoc_comments() {
        let meta = parse_apex_trigger(SAMPLE_TRIGGER).unwrap();
        assert!(!meta.existing_comments.is_empty());
        assert!(meta.existing_comments[0].contains("Handles Account DML events"));
    }

    #[test]
    fn handles_single_event() {
        let src = "trigger ContactTrigger on Contact (before delete) {}";
        let meta = parse_apex_trigger(src).unwrap();
        assert_eq!(meta.events, vec![TriggerEvent::BeforeDelete]);
    }

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_source_returns_default_metadata() {
        let meta = parse_apex_trigger("").unwrap();
        assert!(meta.trigger_name.is_empty());
        assert!(meta.sobject.is_empty());
        assert!(meta.events.is_empty());
    }

    #[test]
    fn all_seven_events_parsed() {
        let src = "trigger T on Obj (before insert, before update, before delete, after insert, after update, after delete, after undelete) {}";
        let meta = parse_apex_trigger(src).unwrap();
        assert_eq!(meta.events.len(), 7);
        assert!(meta.events.contains(&TriggerEvent::BeforeInsert));
        assert!(meta.events.contains(&TriggerEvent::BeforeDelete));
        assert!(meta.events.contains(&TriggerEvent::AfterUndelete));
    }

    #[test]
    fn unknown_event_is_ignored() {
        let src = "trigger T on Obj (before insert, before merge) {}";
        let meta = parse_apex_trigger(src).unwrap();
        assert_eq!(meta.events.len(), 1);
        assert_eq!(meta.events[0], TriggerEvent::BeforeInsert);
    }

    #[test]
    fn references_exclude_trigger_keyword_and_sobject() {
        let src = r#"trigger AccountTrigger on Account (before insert) {
    AccountService svc = new AccountService();
}"#;
        let meta = parse_apex_trigger(src).unwrap();
        assert!(!meta.references.contains(&"Trigger".to_string()));
        assert!(!meta.references.contains(&"Account".to_string()));
        assert!(!meta.references.contains(&"AccountTrigger".to_string()));
        assert!(meta.references.contains(&"AccountService".to_string()));
    }

    #[test]
    fn trigger_event_as_str_values() {
        assert_eq!(TriggerEvent::BeforeInsert.as_str(), "before insert");
        assert_eq!(TriggerEvent::BeforeUpdate.as_str(), "before update");
        assert_eq!(TriggerEvent::BeforeDelete.as_str(), "before delete");
        assert_eq!(TriggerEvent::AfterInsert.as_str(), "after insert");
        assert_eq!(TriggerEvent::AfterUpdate.as_str(), "after update");
        assert_eq!(TriggerEvent::AfterDelete.as_str(), "after delete");
        assert_eq!(TriggerEvent::AfterUndelete.as_str(), "after undelete");
    }

    #[test]
    fn no_apexdoc_gives_empty_comments() {
        let src = "trigger T on Obj (after insert) { }";
        let meta = parse_apex_trigger(src).unwrap();
        assert!(meta.existing_comments.is_empty());
    }

    #[test]
    fn multiple_class_references_deduplicated() {
        let src = r#"trigger T on Account (before insert) {
    MyHelper h1 = new MyHelper();
    MyHelper h2 = new MyHelper();
}"#;
        let meta = parse_apex_trigger(src).unwrap();
        let count = meta
            .references
            .iter()
            .filter(|r| r.as_str() == "MyHelper")
            .count();
        assert_eq!(count, 1, "references should be deduplicated");
    }
}
