# Comprehensive Test Suite Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add thorough positive, negative, and edge-case tests for every module in the sfdoc CLI tool to catch breaking changes and ensure all metadata file types work correctly.

**Architecture:** Expand existing inline `#[cfg(test)]` blocks with new test functions, add fixture files for all metadata types, and expand the integration test suite. All tests are self-contained with no external API calls.

**Tech Stack:** Rust, `cargo test`, `tempfile`, `httpmock`, `serde_json`

---

## File Structure

### New Fixture Files
- `tests/fixtures/flows/Account_Onboarding.flow-meta.xml` — sample Flow
- `tests/fixtures/objects/Invoice__c/Invoice__c.object-meta.xml` — sample Object
- `tests/fixtures/objects/Invoice__c/fields/Status__c.field-meta.xml` — sample field
- `tests/fixtures/objects/Invoice__c/fields/Account__c.field-meta.xml` — sample lookup field
- `tests/fixtures/validation-rules/objects/Account/validationRules/Require_Email.validationRule-meta.xml`
- `tests/fixtures/lwc/myButton/myButton.js-meta.xml` — sample LWC meta
- `tests/fixtures/lwc/myButton/myButton.js` — sample LWC JS
- `tests/fixtures/lwc/myButton/myButton.html` — sample LWC HTML
- `tests/fixtures/flexipages/Account_Record_Page.flexipage-meta.xml` — sample FlexiPage
- `tests/fixtures/aura/myAuraComp/myAuraComp.cmp` — sample Aura component
- `tests/fixtures/customMetadata/Integration_Settings__mdt.Default.md-meta.xml` — sample custom metadata

### Modified Files (inline test additions)
- `src/parser.rs` — add ~15 tests for edge cases
- `src/trigger_parser.rs` — add ~8 tests for edge cases
- `src/flow_parser.rs` — add ~8 tests for edge cases
- `src/validation_rule_parser.rs` — add ~6 tests for edge cases
- `src/object_parser.rs` — add ~6 tests for edge cases
- `src/lwc_parser.rs` — add ~6 tests for edge cases
- `src/flexipage_parser.rs` — add ~6 tests for edge cases
- `src/aura_parser.rs` — add ~6 tests for edge cases
- `src/custom_metadata_parser.rs` — add ~6 tests for edge cases
- `src/scanner.rs` — add ~8 tests for edge cases
- `src/cache.rs` — add ~10 tests for edge cases
- `src/providers.rs` — add ~8 tests
- `src/renderer.rs` — add ~12 tests for edge cases
- `src/retry.rs` — add ~4 tests for edge cases
- `src/apex_common.rs` — add ~4 tests
- `tests/integration.rs` — add ~20 tests for full pipeline coverage of all metadata types

---

### Task 1: Add Fixture Files for All Metadata Types

**Files:**
- Create: `tests/fixtures/flows/Account_Onboarding.flow-meta.xml`
- Create: `tests/fixtures/objects/Invoice__c/Invoice__c.object-meta.xml`
- Create: `tests/fixtures/objects/Invoice__c/fields/Status__c.field-meta.xml`
- Create: `tests/fixtures/objects/Invoice__c/fields/Account__c.field-meta.xml`
- Create: `tests/fixtures/validation-rules/objects/Account/validationRules/Require_Email.validationRule-meta.xml`
- Create: `tests/fixtures/lwc/myButton/myButton.js-meta.xml`
- Create: `tests/fixtures/lwc/myButton/myButton.js`
- Create: `tests/fixtures/lwc/myButton/myButton.html`
- Create: `tests/fixtures/flexipages/Account_Record_Page.flexipage-meta.xml`
- Create: `tests/fixtures/aura/myAuraComp/myAuraComp.cmp`
- Create: `tests/fixtures/customMetadata/Integration_Settings__mdt.Default.md-meta.xml`

- [ ] **Step 1: Create flow fixture**

```xml
<!-- tests/fixtures/flows/Account_Onboarding.flow-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>59.0</apiVersion>
    <description>Onboards new accounts automatically.</description>
    <label>Account Onboarding</label>
    <processType>AutoLaunchedFlow</processType>
    <variables>
        <name>varAccountId</name>
        <dataType>String</dataType>
        <isInput>true</isInput>
        <isOutput>false</isOutput>
    </variables>
    <decisions>
        <name>Check_Status</name>
        <rules><name>Is_Active</name></rules>
    </decisions>
    <recordLookups>
        <name>Get_Account</name>
        <object>Account</object>
    </recordLookups>
    <recordUpdates>
        <name>Update_Account</name>
        <object>Account</object>
    </recordUpdates>
    <screens>
        <name>Welcome_Screen</name>
    </screens>
    <actionCalls>
        <name>Send_Welcome</name>
        <actionName>sendWelcomeEmail</actionName>
        <actionType>emailAlert</actionType>
    </actionCalls>
</Flow>
```

- [ ] **Step 2: Create object fixture with fields**

```xml
<!-- tests/fixtures/objects/Invoice__c/Invoice__c.object-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <description>Tracks invoices for customer billing.</description>
    <label>Invoice</label>
    <pluralLabel>Invoices</pluralLabel>
    <nameField>
        <label>Invoice Number</label>
        <type>AutoNumber</type>
    </nameField>
</CustomObject>
```

```xml
<!-- tests/fixtures/objects/Invoice__c/fields/Status__c.field-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Status__c</fullName>
    <label>Status</label>
    <type>Picklist</type>
    <required>true</required>
    <description>Current invoice status.</description>
</CustomField>
```

```xml
<!-- tests/fixtures/objects/Invoice__c/fields/Account__c.field-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account__c</fullName>
    <label>Account</label>
    <type>Lookup</type>
    <referenceTo>Account</referenceTo>
    <required>false</required>
    <inlineHelpText>The account this invoice belongs to.</inlineHelpText>
</CustomField>
```

- [ ] **Step 3: Create validation rule fixture**

```xml
<!-- tests/fixtures/validation-rules/objects/Account/validationRules/Require_Email.validationRule-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Require_Email</fullName>
    <active>true</active>
    <description>Ensures all accounts have an email address.</description>
    <errorConditionFormula>AND(
  ISPICKVAL(Type, "Customer"),
  ISBLANK(Email__c)
)</errorConditionFormula>
    <errorDisplayField>Email__c</errorDisplayField>
    <errorMessage>Email is required for customer accounts.</errorMessage>
</ValidationRule>
```

- [ ] **Step 4: Create LWC fixture (meta, JS, HTML)**

```xml
<!-- tests/fixtures/lwc/myButton/myButton.js-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<LightningComponentBundle xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>59.0</apiVersion>
    <isExposed>true</isExposed>
    <targets>
        <target>lightning__RecordPage</target>
    </targets>
</LightningComponentBundle>
```

```javascript
// tests/fixtures/lwc/myButton/myButton.js
import { LightningElement, api } from 'lwc';

export default class MyButton extends LightningElement {
    @api label = 'Click Me';
    @api variant;
    @api disabled;

    @api focus() {
        this.template.querySelector('button').focus();
    }

    handleClick() {
        this.dispatchEvent(new CustomEvent('press'));
    }
}
```

```html
<!-- tests/fixtures/lwc/myButton/myButton.html -->
<template>
    <button onclick={handleClick} disabled={disabled}>
        <slot name="icon"></slot>
        {label}
        <slot></slot>
    </button>
    <c-tooltip text="Help text"></c-tooltip>
</template>
```

- [ ] **Step 5: Create FlexiPage fixture**

```xml
<!-- tests/fixtures/flexipages/Account_Record_Page.flexipage-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Account Record Page</masterLabel>
    <type>RecordPage</type>
    <sobjectType>Account</sobjectType>
    <description>Custom record page for accounts.</description>
    <flexiPageRegions>
        <componentInstances>
            <componentInstance>
                <componentName>c__accountDetails</componentName>
            </componentInstance>
            <componentInstance>
                <componentName>c__relatedContacts</componentName>
            </componentInstance>
            <componentInstance>
                <componentName>force:detailPanel</componentName>
            </componentInstance>
        </componentInstances>
    </flexiPageRegions>
</FlexiPage>
```

- [ ] **Step 6: Create Aura component fixture**

```xml
<!-- tests/fixtures/aura/myAuraComp/myAuraComp.cmp -->
<aura:component extends="c:baseComponent" implements="flexipage:availableForRecordHome">
    <aura:attribute name="recordId" type="Id" description="The record to display"/>
    <aura:attribute name="title" type="String" default="Details"/>
    <aura:registerEvent name="onSave" type="c:saveEvent"/>
    <aura:handler event="c:refreshEvent" action="{!c.handleRefresh}"/>
    <div class="slds-card">
        <h2>{!v.title}</h2>
    </div>
</aura:component>
```

- [ ] **Step 7: Create custom metadata fixture**

```xml
<!-- tests/fixtures/customMetadata/Integration_Settings__mdt.Default.md-meta.xml -->
<?xml version="1.0" encoding="UTF-8"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <label>Default Settings</label>
    <protected>false</protected>
    <values>
        <field>Endpoint__c</field>
        <value xsi:type="xsd:string">https://api.example.com</value>
    </values>
    <values>
        <field>Timeout__c</field>
        <value xsi:type="xsd:double">30</value>
    </values>
    <values>
        <field>Enabled__c</field>
        <value xsi:type="xsd:boolean">true</value>
    </values>
</CustomMetadata>
```

- [ ] **Step 8: Run `cargo test` to confirm existing tests still pass**

Run: `cargo test`
Expected: All existing tests pass, new fixtures don't break anything.

- [ ] **Step 9: Commit fixture files**

```bash
git add tests/fixtures/
git commit -m "test: add fixture files for all metadata types (flows, objects, LWC, flexipages, aura, custom metadata, validation rules)"
```

---

### Task 2: Apex Class Parser Edge Cases

**Files:**
- Modify: `src/parser.rs` (tests section starting at line 310)

Add these tests to the existing `#[cfg(test)] mod tests` block:

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_source_returns_default_metadata() {
        let meta = parse_apex_class("").unwrap();
        assert!(meta.class_name.is_empty());
        assert!(meta.methods.is_empty());
        assert!(meta.properties.is_empty());
    }

    #[test]
    fn whitespace_only_source_returns_default_metadata() {
        let meta = parse_apex_class("   \n\t\n  ").unwrap();
        assert!(meta.class_name.is_empty());
    }

    #[test]
    fn parses_with_sharing_class() {
        let src = "public with sharing class MyService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "MyService");
        assert_eq!(meta.access_modifier, "public");
    }

    #[test]
    fn parses_without_sharing_class() {
        let src = "public without sharing class SystemService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "SystemService");
    }

    #[test]
    fn parses_inherited_sharing_class() {
        let src = "public inherited sharing class DelegateService { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.class_name, "DelegateService");
    }

    #[test]
    fn parses_global_access_modifier() {
        let src = "global class WebServiceHandler { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.access_modifier, "global");
    }

    #[test]
    fn parses_private_inner_class_style() {
        let src = "private class InnerHelper { }";
        let meta = parse_apex_class(src).unwrap();
        assert_eq!(meta.access_modifier, "private");
        assert_eq!(meta.class_name, "InnerHelper");
    }

    #[test]
    fn no_extends_gives_none() {
        let src = "public class Simple { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.extends.is_none());
    }

    #[test]
    fn no_implements_gives_empty_vec() {
        let src = "public class Simple { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.implements.is_empty());
    }

    #[test]
    fn multiple_implements() {
        let src = "public class Multi implements Queueable, Schedulable, Database.Batchable<SObject> { }";
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.implements.len() >= 2, "expected at least 2 interfaces, got {:?}", meta.implements);
    }

    #[test]
    fn method_with_no_params() {
        let src = r#"public class Svc {
    public void doWork() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let m = meta.methods.iter().find(|m| m.name == "doWork").unwrap();
        assert!(m.params.is_empty());
    }

    #[test]
    fn strips_single_line_comments_before_parsing() {
        let src = r#"public class Svc {
    // public void hiddenMethod() { }
    public void realMethod() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"realMethod"));
        // hiddenMethod is inside a comment — it should not be parsed
        assert!(!names.contains(&"hiddenMethod"), "comment method should not be parsed: {:?}", names);
    }

    #[test]
    fn strips_block_comments_before_parsing() {
        let src = r#"public class Svc {
    /* public void hidden() { } */
    public void visible() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"visible"));
    }

    #[test]
    fn skips_control_flow_keywords_as_methods() {
        let src = r#"public class Svc {
    public void process() {
        if (true) { }
        for (Integer i = 0; i < 10; i++) { }
        while (true) { }
    }
}"#;
        let meta = parse_apex_class(src).unwrap();
        let names: Vec<&str> = meta.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(!names.contains(&"if"));
        assert!(!names.contains(&"for"));
        assert!(!names.contains(&"while"));
    }

    #[test]
    fn parses_override_method() {
        let src = r#"public class Child extends Parent {
    public override void process() { }
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.methods.iter().any(|m| m.name == "process"));
    }

    #[test]
    fn parses_abstract_method() {
        let src = r#"public abstract class Base {
    public abstract void execute();
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(meta.is_abstract);
    }

    #[test]
    fn references_exclude_builtin_types() {
        let src = r#"public class Svc {
    public String name;
    public Integer count;
    public CustomType__c custom;
}"#;
        let meta = parse_apex_class(src).unwrap();
        // String and Integer are builtins — should not appear in references
        assert!(!meta.references.contains(&"String".to_string()));
        assert!(!meta.references.contains(&"Integer".to_string()));
    }

    #[test]
    fn references_exclude_self_class_name() {
        let src = r#"public class AccountService {
    public AccountService getInstance() { return new AccountService(); }
}"#;
        let meta = parse_apex_class(src).unwrap();
        assert!(!meta.references.contains(&"AccountService".to_string()));
    }
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --lib parser::tests`
Expected: All new and existing tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/parser.rs
git commit -m "test: add edge case and negative tests for Apex class parser"
```

---

### Task 3: Trigger Parser Edge Cases

**Files:**
- Modify: `src/trigger_parser.rs` (tests section starting at line 86)

- [ ] **Step 1: Write edge case tests**

```rust
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
        // "before merge" is not a valid trigger event — should be silently ignored
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
        let count = meta.references.iter().filter(|r| r.as_str() == "MyHelper").count();
        assert_eq!(count, 1, "references should be deduplicated");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib trigger_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/trigger_parser.rs
git commit -m "test: add edge case and negative tests for trigger parser"
```

---

### Task 4: Flow Parser Edge Cases

**Files:**
- Modify: `src/flow_parser.rs` (tests section starting at line 194)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_xml_returns_defaults() {
        let src = r#"<?xml version="1.0"?><Flow></Flow>"#;
        let meta = parse_flow("Empty_Flow", src).unwrap();
        assert_eq!(meta.api_name, "Empty_Flow");
        assert_eq!(meta.label, "Empty Flow"); // fallback
        assert!(meta.process_type.is_empty());
        assert!(meta.variables.is_empty());
        assert_eq!(meta.decisions, 0);
        assert_eq!(meta.loops, 0);
        assert_eq!(meta.screens, 0);
    }

    #[test]
    fn counts_loops_and_screens() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Loop Flow</label>
    <processType>Flow</processType>
    <loops><name>Loop1</name></loops>
    <loops><name>Loop2</name></loops>
    <screens><name>Screen1</name></screens>
    <screens><name>Screen2</name></screens>
    <screens><name>Screen3</name></screens>
</Flow>"#;
        let meta = parse_flow("Loop_Flow", src).unwrap();
        assert_eq!(meta.loops, 2);
        assert_eq!(meta.screens, 3);
    }

    #[test]
    fn record_delete_operation_parsed() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Delete Flow</label>
    <recordDeletes>
        <name>Delete_Old</name>
        <object>Task</object>
    </recordDeletes>
</Flow>"#;
        let meta = parse_flow("Delete_Flow", src).unwrap();
        assert_eq!(meta.record_operations.len(), 1);
        assert_eq!(meta.record_operations[0].operation, "delete");
        assert_eq!(meta.record_operations[0].object, "Task");
    }

    #[test]
    fn record_create_operation_parsed() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Create Flow</label>
    <recordCreates>
        <name>Create_Task</name>
        <object>Task</object>
    </recordCreates>
</Flow>"#;
        let meta = parse_flow("Create_Flow", src).unwrap();
        assert_eq!(meta.record_operations.len(), 1);
        assert_eq!(meta.record_operations[0].operation, "create");
    }

    #[test]
    fn variable_with_no_input_output_flags_defaults_to_false() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Var Flow</label>
    <variables>
        <name>localVar</name>
        <dataType>String</dataType>
    </variables>
</Flow>"#;
        let meta = parse_flow("Var_Flow", src).unwrap();
        assert_eq!(meta.variables.len(), 1);
        assert!(!meta.variables[0].is_input);
        assert!(!meta.variables[0].is_output);
    }

    #[test]
    fn variable_with_empty_name_is_skipped() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Skip Flow</label>
    <variables>
        <name></name>
        <dataType>String</dataType>
    </variables>
</Flow>"#;
        let meta = parse_flow("Skip_Flow", src).unwrap();
        assert!(meta.variables.is_empty());
    }

    #[test]
    fn action_call_with_empty_name_is_skipped() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Skip Action</label>
    <actionCalls>
        <name>Valid_Action</name>
        <actionName></actionName>
        <actionType>apex</actionType>
    </actionCalls>
</Flow>"#;
        let meta = parse_flow("Skip_Action", src).unwrap();
        // actionName is empty — the action should still be included since "name" (Valid_Action) is unused;
        // but actionName maps to action.name which is empty, so action is skipped
        assert!(meta.action_calls.is_empty());
    }

    #[test]
    fn multiple_decisions_counted() {
        let src = r#"<?xml version="1.0"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Multi Decision</label>
    <decisions><name>D1</name></decisions>
    <decisions><name>D2</name></decisions>
    <decisions><name>D3</name></decisions>
</Flow>"#;
        let meta = parse_flow("Multi_Decision", src).unwrap();
        assert_eq!(meta.decisions, 3);
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib flow_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/flow_parser.rs
git commit -m "test: add edge case and negative tests for flow parser"
```

---

### Task 5: Validation Rule Parser Edge Cases

**Files:**
- Modify: `src/validation_rule_parser.rs` (tests section starting at line 100)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_xml_returns_defaults() {
        let path = make_path("Account", "Empty_Rule");
        let src = r#"<?xml version="1.0"?><ValidationRule></ValidationRule>"#;
        let meta = parse_validation_rule(&path, src).unwrap();
        assert_eq!(meta.rule_name, "Empty_Rule");
        assert_eq!(meta.object_name, "Account");
        assert!(meta.active); // default is true
        assert!(meta.description.is_empty());
        assert!(meta.error_condition_formula.is_empty());
    }

    #[test]
    fn rule_name_derived_from_deep_nested_path() {
        let path = PathBuf::from("force-app/main/default/objects/My_Object__c/validationRules/Complex_Rule_Name.validationRule-meta.xml");
        let src = r#"<?xml version="1.0"?><ValidationRule><active>true</active></ValidationRule>"#;
        let meta = parse_validation_rule(&path, src).unwrap();
        assert_eq!(meta.rule_name, "Complex_Rule_Name");
        assert_eq!(meta.object_name, "My_Object__c");
    }

    #[test]
    fn formula_with_html_entities() {
        let path = make_path("Account", "Entity_Rule");
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <active>true</active>
    <errorConditionFormula>Amount__c &lt; 0</errorConditionFormula>
    <errorMessage>Amount must be &gt;= 0.</errorMessage>
</ValidationRule>"#;
        let meta = parse_validation_rule(&path, src).unwrap();
        assert!(meta.error_condition_formula.contains("<"), "XML entities should be unescaped: {}", meta.error_condition_formula);
        assert!(meta.error_message.contains(">="), "XML entities should be unescaped: {}", meta.error_message);
    }

    #[test]
    fn path_with_no_parent_gives_unknown_object() {
        let path = PathBuf::from("Orphan_Rule.validationRule-meta.xml");
        let src = r#"<?xml version="1.0"?><ValidationRule><active>false</active></ValidationRule>"#;
        let meta = parse_validation_rule(&path, src).unwrap();
        // With no parent directories, object_name falls back to "Unknown" or similar
        assert_eq!(meta.rule_name, "Orphan_Rule");
    }

    #[test]
    fn minimal_xml_has_empty_description() {
        let path = make_path("Contact", "No_Desc");
        let src = r#"<?xml version="1.0"?><ValidationRule><active>true</active><errorMessage>Required</errorMessage></ValidationRule>"#;
        let meta = parse_validation_rule(&path, src).unwrap();
        assert!(meta.description.is_empty());
        assert!(meta.error_display_field.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib validation_rule_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/validation_rule_parser.rs
git commit -m "test: add edge case and negative tests for validation rule parser"
```

---

### Task 6: Object Parser Edge Cases

**Files:**
- Modify: `src/object_parser.rs` (tests section starting at line 180)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_xml_returns_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("Empty__c.object-meta.xml");
        let xml = r#"<?xml version="1.0"?><CustomObject></CustomObject>"#;
        fs::write(&path, xml).unwrap();

        let meta = parse_object(&path, xml).unwrap();
        assert_eq!(meta.object_name, "Empty__c");
        assert!(meta.label.is_empty());
        assert!(meta.description.is_empty());
        assert!(meta.fields.is_empty());
    }

    #[test]
    fn multiple_fields_sorted_by_filename() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Multi__c");
        let xml = make_object_xml("Multi", "Multiple fields");
        let obj_path = obj_dir.join("Multi__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        // Create fields in reverse alphabetical order
        fs::write(
            obj_dir.join("fields").join("Zebra__c.field-meta.xml"),
            make_field_xml("Zebra", "Text"),
        ).unwrap();
        fs::write(
            obj_dir.join("fields").join("Alpha__c.field-meta.xml"),
            make_field_xml("Alpha", "Number"),
        ).unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert_eq!(meta.fields.len(), 2);
        assert_eq!(meta.fields[0].api_name, "Alpha__c");
        assert_eq!(meta.fields[1].api_name, "Zebra__c");
    }

    #[test]
    fn field_with_help_text() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Help__c");
        let xml = make_object_xml("Help", "");
        let obj_path = obj_dir.join("Help__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        let field_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Tip__c</fullName>
    <label>Tip</label>
    <type>Text</type>
    <inlineHelpText>Enter a helpful tip here.</inlineHelpText>
    <required>false</required>
</CustomField>"#;
        fs::write(obj_dir.join("fields").join("Tip__c.field-meta.xml"), field_xml).unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert_eq!(meta.fields[0].help_text, "Enter a helpful tip here.");
    }

    #[test]
    fn field_required_defaults_to_false() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Defaults__c");
        let xml = make_object_xml("Defaults", "");
        let obj_path = obj_dir.join("Defaults__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        let field_xml = r#"<?xml version="1.0"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>NoReq</label>
    <type>Text</type>
</CustomField>"#;
        fs::write(obj_dir.join("fields").join("NoReq__c.field-meta.xml"), field_xml).unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert!(!meta.fields[0].required);
    }

    #[test]
    fn nested_label_in_name_field_not_used_as_object_label() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("Nested__c.object-meta.xml");
        // The <label>Name</label> inside <nameField> should NOT be picked up as the object label
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Proper Label</label>
    <nameField>
        <label>Name</label>
        <type>Text</type>
    </nameField>
</CustomObject>"#;
        fs::write(&path, xml).unwrap();
        let meta = parse_object(&path, xml).unwrap();
        assert_eq!(meta.label, "Proper Label");
    }

    #[test]
    fn non_field_files_in_fields_dir_ignored() {
        let tmp = TempDir::new().unwrap();
        let obj_dir = setup_object_dir(&tmp, "Mixed__c");
        let xml = make_object_xml("Mixed", "");
        let obj_path = obj_dir.join("Mixed__c.object-meta.xml");
        fs::write(&obj_path, &xml).unwrap();

        // This file doesn't end with .field-meta.xml — should be ignored
        fs::write(obj_dir.join("fields").join("README.md"), "docs").unwrap();
        fs::write(
            obj_dir.join("fields").join("Real__c.field-meta.xml"),
            make_field_xml("Real", "Text"),
        ).unwrap();

        let meta = parse_object(&obj_path, &xml).unwrap();
        assert_eq!(meta.fields.len(), 1);
        assert_eq!(meta.fields[0].api_name, "Real__c");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib object_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/object_parser.rs
git commit -m "test: add edge case and negative tests for object parser"
```

---

### Task 7: LWC Parser Edge Cases

**Files:**
- Modify: `src/lwc_parser.rs` (tests section starting at line 168)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_js_source_gives_no_api_props() {
        let tmp = TempDir::new().unwrap();
        let meta = setup_component(&tmp, "emptyComp", "", "");
        let result = parse_lwc(&meta, "").unwrap();
        assert!(result.api_props.is_empty());
        assert!(result.slots.is_empty());
        assert!(result.referenced_components.is_empty());
    }

    #[test]
    fn api_prop_with_equals_initializer() {
        let tmp = TempDir::new().unwrap();
        let js = "@api recordId = '001xxx';";
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        assert!(result.api_props.iter().any(|p| p.name == "recordId" && !p.is_method));
    }

    #[test]
    fn mixed_api_props_and_methods() {
        let tmp = TempDir::new().unwrap();
        let js = r#"
            @api label;
            @api variant = 'brand';
            @api focus() { }
            @api reset() { }
        "#;
        let meta = setup_component(&tmp, "myComp", js, "");
        let result = parse_lwc(&meta, js).unwrap();
        let props: Vec<&str> = result.api_props.iter().filter(|p| !p.is_method).map(|p| p.name.as_str()).collect();
        let methods: Vec<&str> = result.api_props.iter().filter(|p| p.is_method).map(|p| p.name.as_str()).collect();
        assert!(props.contains(&"label"));
        assert!(props.contains(&"variant"));
        assert!(methods.contains(&"focus"));
        assert!(methods.contains(&"reset"));
    }

    #[test]
    fn self_closing_slot_tag() {
        let tmp = TempDir::new().unwrap();
        let html = "<template><slot/></template>";
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert!(result.slots.contains(&"default".to_string()));
    }

    #[test]
    fn multiple_different_c_component_refs() {
        let tmp = TempDir::new().unwrap();
        let html = r#"<template>
            <c-my-button></c-my-button>
            <c-my-input></c-my-input>
            <c-data-table></c-data-table>
        </template>"#;
        let meta = setup_component(&tmp, "myComp", "", html);
        let result = parse_lwc(&meta, "").unwrap();
        assert_eq!(result.referenced_components.len(), 3);
        assert!(result.referenced_components.contains(&"myButton".to_string()));
        assert!(result.referenced_components.contains(&"myInput".to_string()));
        assert!(result.referenced_components.contains(&"dataTable".to_string()));
    }

    #[test]
    fn kebab_to_camel_single_word() {
        assert_eq!(kebab_to_camel("button"), "button");
    }

    #[test]
    fn kebab_to_camel_multiple_hyphens() {
        assert_eq!(kebab_to_camel("my-very-long-name"), "myVeryLongName");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib lwc_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/lwc_parser.rs
git commit -m "test: add edge case and negative tests for LWC parser"
```

---

### Task 8: FlexiPage Parser Edge Cases

**Files:**
- Modify: `src/flexipage_parser.rs` (tests section starting at line 92)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_xml_returns_defaults() {
        let src = r#"<?xml version="1.0"?><FlexiPage></FlexiPage>"#;
        let meta = parse_flexipage("Empty_Page", src).unwrap();
        assert_eq!(meta.api_name, "Empty_Page");
        assert!(meta.label.is_empty());
        assert!(meta.page_type.is_empty());
        assert!(meta.sobject.is_empty());
        assert!(meta.component_names.is_empty());
    }

    #[test]
    fn app_page_type() {
        let src = r#"<?xml version="1.0"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>My App Page</masterLabel>
    <type>AppPage</type>
</FlexiPage>"#;
        let meta = parse_flexipage("My_App_Page", src).unwrap();
        assert_eq!(meta.page_type, "AppPage");
        assert!(meta.sobject.is_empty());
    }

    #[test]
    fn home_page_type() {
        let src = r#"<?xml version="1.0"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Home</masterLabel>
    <type>HomePage</type>
</FlexiPage>"#;
        let meta = parse_flexipage("Home", src).unwrap();
        assert_eq!(meta.page_type, "HomePage");
    }

    #[test]
    fn component_without_c_prefix_kept_as_is() {
        let src = r#"<?xml version="1.0"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Page</masterLabel>
    <type>RecordPage</type>
    <flexiPageRegions>
        <componentInstances>
            <componentInstance>
                <componentName>force:detailPanel</componentName>
            </componentInstance>
        </componentInstances>
    </flexiPageRegions>
</FlexiPage>"#;
        let meta = parse_flexipage("Page", src).unwrap();
        assert!(meta.component_names.contains(&"force:detailPanel".to_string()));
    }

    #[test]
    fn duplicate_component_names_deduplicated() {
        let src = r#"<?xml version="1.0"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Page</masterLabel>
    <type>RecordPage</type>
    <flexiPageRegions>
        <componentInstances>
            <componentInstance><componentName>c__myComp</componentName></componentInstance>
            <componentInstance><componentName>c__myComp</componentName></componentInstance>
        </componentInstances>
    </flexiPageRegions>
</FlexiPage>"#;
        let meta = parse_flexipage("Page", src).unwrap();
        assert_eq!(meta.component_names.iter().filter(|n| n.as_str() == "myComp").count(), 1);
    }

    #[test]
    fn flow_action_names_extracted() {
        let src = r#"<?xml version="1.0"?>
<FlexiPage xmlns="http://soap.sforce.com/2006/04/metadata">
    <masterLabel>Page</masterLabel>
    <type>RecordPage</type>
    <flexiPageRegions>
        <componentInstances>
            <componentInstance>
                <componentName>flowruntime:interview</componentName>
                <actionName>My_Flow</actionName>
            </componentInstance>
        </componentInstances>
    </flexiPageRegions>
</FlexiPage>"#;
        let meta = parse_flexipage("Page", src).unwrap();
        assert!(meta.flow_names.contains(&"My_Flow".to_string()));
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib flexipage_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/flexipage_parser.rs
git commit -m "test: add edge case and negative tests for FlexiPage parser"
```

---

### Task 9: Aura Parser Edge Cases

**Files:**
- Modify: `src/aura_parser.rs` (tests section starting at line 143)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_component_tag_returns_defaults() {
        let tmp = TempDir::new().unwrap();
        let cmp = "<aura:component></aura:component>";
        let path = setup_component(&tmp, "emptyComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.component_name, "emptyComp");
        assert!(result.attributes.is_empty());
        assert!(result.events_handled.is_empty());
        assert!(result.extends.is_none());
    }

    #[test]
    fn attribute_with_all_fields() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="mode" type="String" default="view" description="Display mode: view or edit"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes[0].name, "mode");
        assert_eq!(result.attributes[0].attr_type, "String");
        assert_eq!(result.attributes[0].default, "view");
        assert_eq!(result.attributes[0].description, "Display mode: view or edit");
    }

    #[test]
    fn attribute_without_optional_fields() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="items" type="List"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes[0].name, "items");
        assert_eq!(result.attributes[0].attr_type, "List");
        assert!(result.attributes[0].default.is_empty());
        assert!(result.attributes[0].description.is_empty());
    }

    #[test]
    fn attribute_with_empty_name_skipped() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:attribute name="" type="String"/>
    <aura:attribute name="valid" type="String"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.attributes.len(), 1);
        assert_eq!(result.attributes[0].name, "valid");
    }

    #[test]
    fn duplicate_events_deduplicated() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component>
    <aura:handler event="c:myEvent" action="{!c.handle1}"/>
    <aura:handler event="c:myEvent" action="{!c.handle2}"/>
</aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.events_handled.iter().filter(|e| e.as_str() == "c:myEvent").count(), 1);
    }

    #[test]
    fn extends_with_namespace() {
        let tmp = TempDir::new().unwrap();
        let cmp = r#"<aura:component extends="lightning:baseComponent"></aura:component>"#;
        let path = setup_component(&tmp, "myComp", cmp);
        let result = parse_aura(&path, cmp).unwrap();
        assert_eq!(result.extends.as_deref(), Some("lightning:baseComponent"));
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib aura_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/aura_parser.rs
git commit -m "test: add edge case and negative tests for Aura parser"
```

---

### Task 10: Custom Metadata Parser Edge Cases

**Files:**
- Modify: `src/custom_metadata_parser.rs` (tests section starting at line 116)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn filename_without_dot_gives_empty_record_name() {
        let path = PathBuf::from("SinglePart.md-meta.xml");
        let src = r#"<?xml version="1.0"?><CustomMetadata><label>Test</label></CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.type_name, "SinglePart");
        assert!(rec.record_name.is_empty());
    }

    #[test]
    fn empty_xml_returns_defaults() {
        let path = PathBuf::from("Type__mdt.Record.md-meta.xml");
        let src = r#"<?xml version="1.0"?><CustomMetadata></CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.type_name, "Type__mdt");
        assert_eq!(rec.record_name, "Record");
        assert!(rec.label.is_empty());
        assert!(rec.values.is_empty());
    }

    #[test]
    fn value_with_empty_field_skipped() {
        let path = PathBuf::from("T__mdt.R.md-meta.xml");
        let src = r#"<?xml version="1.0"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Test</label>
    <values>
        <field></field>
        <value>should be skipped</value>
    </values>
    <values>
        <field>Valid__c</field>
        <value>kept</value>
    </values>
</CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.values.len(), 1);
        assert_eq!(rec.values[0].0, "Valid__c");
    }

    #[test]
    fn multiple_values_preserved_in_order() {
        let path = PathBuf::from("Config__mdt.Main.md-meta.xml");
        let src = r#"<?xml version="1.0"?>
<CustomMetadata xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Main Config</label>
    <values><field>First__c</field><value>1</value></values>
    <values><field>Second__c</field><value>2</value></values>
    <values><field>Third__c</field><value>3</value></values>
</CustomMetadata>"#;
        let rec = parse_custom_metadata_record(&path, src).unwrap();
        assert_eq!(rec.values.len(), 3);
        assert_eq!(rec.values[0].0, "First__c");
        assert_eq!(rec.values[1].0, "Second__c");
        assert_eq!(rec.values[2].0, "Third__c");
    }

    #[test]
    fn protected_field_does_not_affect_parsing() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        // <protected>false</protected> should not be treated as a value
        assert!(!rec.values.iter().any(|(f, _)| f == "protected"));
    }

    #[test]
    fn value_with_xsi_type_attribute_still_reads_text() {
        let rec = parse_custom_metadata_record(&sample_path(), SAMPLE_XML).unwrap();
        let timeout = rec.values.iter().find(|(f, _)| f == "Timeout__c").unwrap();
        assert_eq!(timeout.1, "30");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib custom_metadata_parser::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/custom_metadata_parser.rs
git commit -m "test: add edge case and negative tests for custom metadata parser"
```

---

### Task 11: Scanner Edge Cases

**Files:**
- Modify: `src/scanner.rs` (tests section starting at line 256)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_directory_returns_empty_vec() {
        let tmp = TempDir::new().unwrap();
        let files = ApexScanner.scan(tmp.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn scanner_skips_sfdx_directory() {
        let tmp = TempDir::new().unwrap();
        let sfdx_dir = tmp.path().join(".sfdx");
        fs::create_dir_all(&sfdx_dir).unwrap();
        write_file(&sfdx_dir, "Hidden.cls", "public class Hidden {}");
        write_file(tmp.path(), "Visible.cls", "public class Visible {}");

        let files = ApexScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Visible.cls");
    }

    #[test]
    fn deeply_nested_files_found() {
        let tmp = TempDir::new().unwrap();
        let deep = tmp.path().join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep).unwrap();
        write_file(&deep, "Deep.cls", "public class Deep {}");

        let files = ApexScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Deep.cls");
    }

    #[test]
    fn trigger_scanner_excludes_cls_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "MyClass.cls", "public class MyClass {}");
        write_file(tmp.path(), "MyTrigger.trigger", "trigger MyTrigger on Account (before insert) {}");

        let files = TriggerScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "MyTrigger.trigger");
    }

    #[test]
    fn flow_scanner_excludes_other_meta_xml() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "My.flow-meta.xml", "<Flow/>");
        write_file(tmp.path(), "My.object-meta.xml", "<CustomObject/>");
        write_file(tmp.path(), "My.validationRule-meta.xml", "<ValidationRule/>");

        let files = FlowScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "My.flow-meta.xml");
    }

    #[test]
    fn validation_rule_scanner_only_finds_vr_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Rule.validationRule-meta.xml", "<ValidationRule/>");
        write_file(tmp.path(), "Flow.flow-meta.xml", "<Flow/>");

        let files = ValidationRuleScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Rule.validationRule-meta.xml");
    }

    #[test]
    fn flexipage_scanner_finds_flexipage_files() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "Page.flexipage-meta.xml", "<FlexiPage/>");
        write_file(tmp.path(), "Other.flow-meta.xml", "<Flow/>");

        let files = FlexiPageScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Page.flexipage-meta.xml");
    }

    #[test]
    fn custom_metadata_scanner_requires_custom_metadata_ancestor() {
        let tmp = TempDir::new().unwrap();
        // File under customMetadata/ should be found
        let cm_dir = tmp.path().join("customMetadata");
        fs::create_dir_all(&cm_dir).unwrap();
        write_file(&cm_dir, "Type__mdt.Record.md-meta.xml", "<CustomMetadata/>");
        // File NOT under customMetadata/ should be ignored
        write_file(tmp.path(), "Other.md-meta.xml", "<CustomMetadata/>");

        let files = CustomMetadataScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "Type__mdt.Record.md-meta.xml");
    }

    #[test]
    fn aura_scanner_requires_aura_ancestor() {
        let tmp = TempDir::new().unwrap();
        let aura_dir = tmp.path().join("aura").join("myComp");
        fs::create_dir_all(&aura_dir).unwrap();
        write_file(&aura_dir, "myComp.cmp", "<aura:component/>");
        // File NOT under aura/ should be ignored
        let other = tmp.path().join("other");
        fs::create_dir_all(&other).unwrap();
        write_file(&other, "other.cmp", "<aura:component/>");

        let files = AuraScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "myComp.cmp");
    }

    #[test]
    fn aura_scanner_reads_sibling_js_as_source() {
        let tmp = TempDir::new().unwrap();
        let comp_dir = tmp.path().join("aura").join("myComp");
        fs::create_dir_all(&comp_dir).unwrap();
        write_file(&comp_dir, "myComp.cmp", "<aura:component/>");
        write_file(&comp_dir, "myComp.js", "({ handleClick: function() {} })");

        let files = AuraScanner.scan(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        // raw_source should come from the .js file, not .cmp
        assert!(files[0].raw_source.contains("handleClick"));
    }

    #[test]
    fn raw_source_preserves_file_content() {
        let tmp = TempDir::new().unwrap();
        let content = "public class Preserved {\n    // exact content\n}";
        write_file(tmp.path(), "Preserved.cls", content);

        let files = ApexScanner.scan(tmp.path()).unwrap();
        assert_eq!(files[0].raw_source, content);
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib scanner::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/scanner.rs
git commit -m "test: add edge case and negative tests for all scanners"
```

---

### Task 12: Cache Edge Cases

**Files:**
- Modify: `src/cache.rs` (tests section starting at line 210)

- [ ] **Step 1: Write edge case tests**

```rust
    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn load_from_nonexistent_dir_returns_empty_cache() {
        let cache = Cache::load(std::path::Path::new("/nonexistent/path"));
        // Should not panic, should return empty default cache
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
        // Simulate a cache file from before trigger support was added
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
        cache.update_trigger("AccountTrigger.trigger".to_string(), "abc123".to_string(), "model-1", doc);
        cache.save(tmp.path()).unwrap();

        let loaded = Cache::load(tmp.path());
        let entry = loaded.get_trigger_if_fresh("AccountTrigger.trigger", "abc123", "model-1").unwrap();
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
        assert!(loaded.get_flow_if_fresh("My_Flow", "hash1", "model-1").is_some());
        assert!(loaded.get_flow_if_fresh("My_Flow", "wrong", "model-1").is_none());
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
        assert!(loaded.get_validation_rule_if_fresh("Rule1", "h1", "m1").is_some());
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
        assert!(loaded.get_object_if_fresh("Invoice__c", "h2", "m2").is_some());
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
        assert_eq!(h.len(), 64); // SHA-256 produces 64 hex chars
        // Consistent across calls
        assert_eq!(h, hash_source(""));
    }

    #[test]
    fn hash_source_unicode_content() {
        let h = hash_source("public class Über {}");
        assert_eq!(h.len(), 64);
        assert_ne!(h, hash_source("public class Uber {}"));
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
        assert_eq!(cache.get_if_fresh("Foo.cls", "hash2", "model").unwrap().documentation.summary, "Version 2");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib cache::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/cache.rs
git commit -m "test: add cache round-trip and edge case tests for all metadata types"
```

---

### Task 13: Provider Tests

**Files:**
- Modify: `src/providers.rs` (add tests section at end of file)

- [ ] **Step 1: Write provider tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_providers_have_default_models() {
        for provider in Provider::all() {
            assert!(!provider.default_model().is_empty(), "{:?} missing default model", provider);
        }
    }

    #[test]
    fn all_providers_have_display_names() {
        for provider in Provider::all() {
            assert!(!provider.display_name().is_empty());
        }
    }

    #[test]
    fn all_providers_have_keychain_keys() {
        for provider in Provider::all() {
            assert!(!provider.keychain_key().is_empty());
        }
    }

    #[test]
    fn all_providers_have_cli_names() {
        for provider in Provider::all() {
            assert!(!provider.cli_name().is_empty());
        }
    }

    #[test]
    fn gemini_has_no_base_url() {
        assert!(Provider::Gemini.base_url().is_none());
    }

    #[test]
    fn openai_compat_providers_have_base_urls() {
        assert!(Provider::Groq.base_url().is_some());
        assert!(Provider::OpenAi.base_url().is_some());
        assert!(Provider::Ollama.base_url().is_some());
    }

    #[test]
    fn ollama_does_not_require_api_key() {
        assert!(!Provider::Ollama.requires_api_key());
    }

    #[test]
    fn non_ollama_providers_require_api_key() {
        assert!(Provider::Gemini.requires_api_key());
        assert!(Provider::Groq.requires_api_key());
        assert!(Provider::OpenAi.requires_api_key());
    }

    #[test]
    fn ollama_has_no_env_var() {
        assert!(Provider::Ollama.env_var().is_none());
    }

    #[test]
    fn non_ollama_providers_have_env_vars() {
        assert!(Provider::Gemini.env_var().is_some());
        assert!(Provider::Groq.env_var().is_some());
        assert!(Provider::OpenAi.env_var().is_some());
    }

    #[test]
    fn display_trait_uses_display_name() {
        assert_eq!(format!("{}", Provider::Gemini), "Google Gemini");
        assert_eq!(format!("{}", Provider::Ollama), "Ollama (local)");
    }

    #[test]
    fn all_returns_four_providers() {
        assert_eq!(Provider::all().len(), 4);
    }

    #[test]
    fn provider_equality() {
        assert_eq!(Provider::Gemini, Provider::Gemini);
        assert_ne!(Provider::Gemini, Provider::Groq);
    }
}
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib providers::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/providers.rs
git commit -m "test: add comprehensive provider tests"
```

---

### Task 14: Renderer Edge Cases

**Files:**
- Modify: `src/renderer.rs` (tests section starting at line 1616)

- [ ] **Step 1: Write renderer edge case tests**

Add to the existing `#[cfg(test)] mod tests` block:

```rust
    // -----------------------------------------------------------------------
    // Edge cases & additional coverage
    // -----------------------------------------------------------------------

    #[test]
    fn class_page_with_no_methods_skips_methods_section() {
        let mut ctx = sample_context();
        ctx.metadata.methods.clear();
        ctx.documentation.methods.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Methods"));
    }

    #[test]
    fn class_page_with_no_properties_skips_properties_section() {
        let mut ctx = sample_context();
        ctx.metadata.properties.clear();
        ctx.documentation.properties.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Properties"));
    }

    #[test]
    fn class_page_with_no_usage_examples_skips_section() {
        let mut ctx = sample_context();
        ctx.documentation.usage_examples.clear();
        let page = render_class_page(&ctx);
        assert!(!page.contains("## Usage Examples"));
    }

    #[test]
    fn class_page_interface_badge() {
        let mut ctx = sample_context();
        ctx.metadata.is_interface = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("interface"), "interface badge missing");
    }

    #[test]
    fn class_page_abstract_badge() {
        let mut ctx = sample_context();
        ctx.metadata.is_abstract = true;
        let page = render_class_page(&ctx);
        assert!(page.contains("abstract"));
    }

    #[test]
    fn class_page_implements_badge() {
        let ctx = sample_context();
        let page = render_class_page(&ctx);
        assert!(page.contains("Queueable"));
    }

    #[test]
    fn index_with_empty_bundle() {
        let bundle = DocumentationBundle {
            classes: &[],
            triggers: &[],
            flows: &[],
            validation_rules: &[],
            objects: &[],
            lwc: &[],
            flexipages: &[],
            custom_metadata: &[],
            aura: &[],
        };
        let index = render_index(&bundle);
        assert!(index.contains("# Apex Documentation Index"));
    }

    #[test]
    fn write_output_html_creates_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = sample_context();
        let bundle = DocumentationBundle {
            classes: &[ctx],
            triggers: &[],
            flows: &[],
            validation_rules: &[],
            objects: &[],
            lwc: &[],
            flexipages: &[],
            custom_metadata: &[],
            aura: &[],
        };
        write_output(tmp.path(), &crate::cli::OutputFormat::Html, &bundle).unwrap();
        assert!(tmp.path().join("index.html").exists());
    }

    #[test]
    fn sanitize_filename_removes_special_chars() {
        assert_eq!(sanitize_filename("Hello World"), "Hello_World");
        assert_eq!(sanitize_filename("test/path"), "test_path");
        assert_eq!(sanitize_filename("normal"), "normal");
    }

    #[test]
    fn sanitize_filename_preserves_underscores_and_hyphens() {
        assert_eq!(sanitize_filename("my-file_name"), "my-file_name");
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib renderer::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/renderer.rs
git commit -m "test: add renderer edge case tests for empty states and badges"
```

---

### Task 15: Retry Edge Cases

**Files:**
- Modify: `src/retry.rs` (tests section starting at line 79)

- [ ] **Step 1: Write additional retry tests**

```rust
    // -----------------------------------------------------------------------
    // Additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn should_retry_covers_expected_codes() {
        assert!(should_retry(429));
        assert!(should_retry(500));
        assert!(should_retry(502));
        assert!(should_retry(503));
        assert!(should_retry(504));
    }

    #[test]
    fn should_retry_rejects_success_and_client_errors() {
        assert!(!should_retry(200));
        assert!(!should_retry(201));
        assert!(!should_retry(400));
        assert!(!should_retry(401));
        assert!(!should_retry(403));
        assert!(!should_retry(404));
    }

    #[test]
    fn retry_delay_empty_body_uses_backoff() {
        let d = retry_delay_secs(None, "", 0);
        assert!((2..=5).contains(&d), "empty body delay {d} out of range");
    }

    #[test]
    fn retry_delay_header_value_of_one() {
        assert_eq!(retry_delay_secs(Some(1), "", 0), 1);
    }
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib retry::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/retry.rs
git commit -m "test: add retry should_retry and edge case tests"
```

---

### Task 16: Apex Common Tests

**Files:**
- Modify: `src/apex_common.rs` (add tests section at end of file)

- [ ] **Step 1: Write apex_common tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_ref_regex_matches_pascal_case() {
        let text = "AccountService handler = new AccountService();";
        let matches: Vec<String> = re_type_ref()
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect();
        assert!(matches.contains(&"AccountService".to_string()));
    }

    #[test]
    fn type_ref_regex_does_not_match_lowercase() {
        let text = "string name = 'hello';";
        let matches: Vec<String> = re_type_ref()
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect();
        // Only PascalCase (starting uppercase) should match
        assert!(matches.is_empty() || !matches.iter().any(|m| m == "string"));
    }

    #[test]
    fn builtins_list_contains_common_types() {
        assert!(APEX_BUILTINS.contains(&"String"));
        assert!(APEX_BUILTINS.contains(&"Integer"));
        assert!(APEX_BUILTINS.contains(&"Boolean"));
        assert!(APEX_BUILTINS.contains(&"List"));
        assert!(APEX_BUILTINS.contains(&"Map"));
        assert!(APEX_BUILTINS.contains(&"Set"));
        assert!(APEX_BUILTINS.contains(&"SObject"));
        assert!(APEX_BUILTINS.contains(&"Database"));
    }

    #[test]
    fn builtins_list_does_not_contain_custom_types() {
        assert!(!APEX_BUILTINS.contains(&"AccountService"));
        assert!(!APEX_BUILTINS.contains(&"MyCustomType"));
    }
}
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --lib apex_common::tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add src/apex_common.rs
git commit -m "test: add apex_common regex and builtins tests"
```

---

### Task 17: Integration Tests — All Metadata Types with Fixtures

**Files:**
- Modify: `tests/integration.rs`

This task adds integration tests that use the new fixture files to test the full scan→parse pipeline for each metadata type, plus cross-linking and mixed-type rendering.

- [ ] **Step 1: Add fixture path helpers and stub functions for remaining types**

Add after the existing helper functions at the top of `tests/integration.rs`:

```rust
use sfdoc::aura_parser;
use sfdoc::custom_metadata_parser;
use sfdoc::flexipage_parser;
use sfdoc::object_parser;
use sfdoc::validation_rule_parser;
use sfdoc::scanner::{
    AuraScanner, CustomMetadataScanner, FlexiPageScanner, ObjectScanner, ValidationRuleScanner,
};
use sfdoc::renderer::{
    AuraRenderContext, CustomMetadataRenderContext, FlexiPageRenderContext,
};
use sfdoc::types::{
    AuraAttributeDocumentation, AuraDocumentation, CustomMetadataRecord,
    FlexiPageDocumentation, LwcPropDocumentation,
};
use sfdoc::cli::OutputFormat;

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
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validation-rules"))
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
```

- [ ] **Step 2: Add fixture-based scanner + parser tests**

```rust
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
    // Check field details
    let status = meta.fields.iter().find(|f| f.api_name == "Status__c").unwrap();
    assert_eq!(status.field_type, "Picklist");
    assert!(status.required);
    let account = meta.fields.iter().find(|f| f.api_name == "Account__c").unwrap();
    assert_eq!(account.field_type, "Lookup");
    assert_eq!(account.reference_to, "Account");
    assert!(!account.help_text.is_empty());
}

#[test]
fn validation_rule_scanner_finds_fixture() {
    let files = ValidationRuleScanner.scan(validation_rule_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Require_Email.validationRule-meta.xml");
}

#[test]
fn validation_rule_fixture_parses_correctly() {
    let files = ValidationRuleScanner.scan(validation_rule_fixtures_dir()).unwrap();
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
    // Check @api properties
    let prop_names: Vec<&str> = meta.api_props.iter().filter(|p| !p.is_method).map(|p| p.name.as_str()).collect();
    assert!(prop_names.contains(&"label"), "missing @api label: {:?}", prop_names);
    assert!(prop_names.contains(&"variant"), "missing @api variant: {:?}", prop_names);
    assert!(prop_names.contains(&"disabled"), "missing @api disabled: {:?}", prop_names);
    // Check @api methods
    let method_names: Vec<&str> = meta.api_props.iter().filter(|p| p.is_method).map(|p| p.name.as_str()).collect();
    assert!(method_names.contains(&"focus"), "missing @api focus(): {:?}", method_names);
    // Check slots
    assert!(meta.slots.contains(&"icon".to_string()), "missing named slot 'icon': {:?}", meta.slots);
    assert!(meta.slots.contains(&"default".to_string()), "missing default slot: {:?}", meta.slots);
    // Check component references
    assert!(meta.referenced_components.contains(&"tooltip".to_string()), "missing c-tooltip ref: {:?}", meta.referenced_components);
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
    let meta = sfdoc::flexipage_parser::parse_flexipage(api_name, &file.raw_source).unwrap();
    assert_eq!(meta.label, "Account Record Page");
    assert_eq!(meta.page_type, "RecordPage");
    assert_eq!(meta.sobject, "Account");
    assert!(meta.component_names.contains(&"accountDetails".to_string()));
    assert!(meta.component_names.contains(&"relatedContacts".to_string()));
    assert!(meta.component_names.contains(&"force:detailPanel".to_string()));
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
    assert!(meta.attributes.iter().any(|a| a.name == "title" && a.default == "Details"));
    assert!(meta.events_handled.contains(&"onSave".to_string()));
}

#[test]
fn custom_metadata_scanner_finds_fixture() {
    let files = CustomMetadataScanner.scan(custom_metadata_fixtures_dir()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "Integration_Settings__mdt.Default.md-meta.xml");
}

#[test]
fn custom_metadata_fixture_parses_correctly() {
    let files = CustomMetadataScanner.scan(custom_metadata_fixtures_dir()).unwrap();
    let file = &files[0];
    let rec = custom_metadata_parser::parse_custom_metadata_record(&file.path, &file.raw_source).unwrap();
    assert_eq!(rec.type_name, "Integration_Settings__mdt");
    assert_eq!(rec.record_name, "Default");
    assert_eq!(rec.label, "Default Settings");
    assert_eq!(rec.values.len(), 3);
    assert!(rec.values.iter().any(|(f, v)| f == "Endpoint__c" && v == "https://api.example.com"));
    assert!(rec.values.iter().any(|(f, v)| f == "Timeout__c" && v == "30"));
    assert!(rec.values.iter().any(|(f, v)| f == "Enabled__c" && v == "true"));
}
```

- [ ] **Step 3: Add cross-linking and mixed-type rendering tests**

```rust
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

    let trigger_ctx = TriggerRenderContext {
        metadata: trigger_parser::parse_apex_trigger(
            "trigger AccountTrigger on Account (before insert) { }",
        ).unwrap(),
        documentation: stub_trigger_doc("AccountTrigger", "Account"),
        all_names: all_names.clone(),
        folder: "triggers".to_string(),
    };

    let flow_ctx = FlowRenderContext {
        metadata: sfdoc::types::FlowMetadata {
            api_name: "My_Flow".to_string(),
            label: "My Flow".to_string(),
            ..Default::default()
        },
        documentation: stub_flow_doc("My_Flow"),
        all_names: all_names.clone(),
        folder: "flows".to_string(),
    };

    let lwc_ctx = LwcRenderContext {
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
    assert!(index.contains("My_Flow") || index.contains("My Flow"), "index missing flow");
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
    renderer::write_output(tmp.path(), &OutputFormat::Markdown, &bundle).unwrap();
    assert!(tmp.path().join("classes/Svc.md").exists());
    assert!(tmp.path().join("index.md").exists());
}
```

- [ ] **Step 4: Run all integration tests**

Run: `cargo test --test integration`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for all metadata types with fixture files"
```

---

### Task 18: Final Full Test Run and Verification

- [ ] **Step 1: Run the complete test suite**

Run: `cargo test`
Expected: All tests pass (existing + new).

- [ ] **Step 2: Count total tests**

Run: `cargo test 2>&1 | tail -5`
Expected: Significantly more tests than the original ~134. New total should be approximately 250+.

- [ ] **Step 3: Run with verbose output to verify test names**

Run: `cargo test -- --list 2>&1 | wc -l`
Expected: ~250+ test names listed.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "test: complete comprehensive test suite for all sfdoc metadata types"
```
