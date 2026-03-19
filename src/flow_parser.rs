use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::types::{FlowActionCall, FlowMetadata, FlowRecordOperation, FlowVariable};

pub fn parse_flow(api_name: &str, source: &str) -> Result<FlowMetadata> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut meta = FlowMetadata {
        api_name: api_name.to_string(),
        ..Default::default()
    };

    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    let mut current_var: Option<FlowVariable> = None;
    let mut current_record_op: Option<FlowRecordOperation> = None;
    let mut current_action: Option<FlowActionCall> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                match name.as_str() {
                    "variables" => current_var = Some(FlowVariable::default()),
                    "recordLookups" => {
                        current_record_op = Some(FlowRecordOperation {
                            operation: "lookup".to_string(),
                            object: String::new(),
                        })
                    }
                    "recordCreates" => {
                        current_record_op = Some(FlowRecordOperation {
                            operation: "create".to_string(),
                            object: String::new(),
                        })
                    }
                    "recordUpdates" => {
                        current_record_op = Some(FlowRecordOperation {
                            operation: "update".to_string(),
                            object: String::new(),
                        })
                    }
                    "recordDeletes" => {
                        current_record_op = Some(FlowRecordOperation {
                            operation: "delete".to_string(),
                            object: String::new(),
                        })
                    }
                    "actionCalls" => {
                        current_action = Some(FlowActionCall {
                            name: String::new(),
                            action_type: String::new(),
                        })
                    }
                    "decisions" => meta.decisions += 1,
                    "loops" => meta.loops += 1,
                    "screens" => meta.screens += 1,
                    _ => {}
                }
                stack.push(name);
            }
            Event::End(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                match name.as_str() {
                    "variables" => {
                        if let Some(v) = current_var.take() {
                            if !v.name.is_empty() {
                                meta.variables.push(v);
                            }
                        }
                    }
                    "recordLookups" | "recordCreates" | "recordUpdates" | "recordDeletes" => {
                        if let Some(op) = current_record_op.take() {
                            if !op.object.is_empty() {
                                meta.record_operations.push(op);
                            }
                        }
                    }
                    "actionCalls" => {
                        if let Some(action) = current_action.take() {
                            if !action.name.is_empty() {
                                meta.action_calls.push(action);
                            }
                        }
                    }
                    _ => {}
                }
                stack.pop();
            }
            Event::Text(e) => {
                let text = match e.unescape() {
                    Ok(t) => t.trim().to_string(),
                    Err(_) => continue,
                };
                if text.is_empty() {
                    continue;
                }

                let parent = stack.last().map(|s| s.as_str()).unwrap_or("");
                let grandparent = stack
                    .len()
                    .checked_sub(2)
                    .and_then(|i| stack.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                match (grandparent, parent) {
                    // Top-level flow metadata
                    (_, "label") if grandparent == "Flow" || grandparent.is_empty() => {
                        if meta.label.is_empty() {
                            meta.label = text;
                        }
                    }
                    (_, "processType") => {
                        if meta.process_type.is_empty() {
                            meta.process_type = text;
                        }
                    }
                    (_, "description") if grandparent == "Flow" || grandparent.is_empty() => {
                        if meta.description.is_empty() {
                            meta.description = text;
                        }
                    }
                    // Variable fields
                    ("variables", "name") => {
                        if let Some(v) = &mut current_var {
                            v.name = text;
                        }
                    }
                    ("variables", "dataType") => {
                        if let Some(v) = &mut current_var {
                            v.data_type = text;
                        }
                    }
                    ("variables", "isInput") => {
                        if let Some(v) = &mut current_var {
                            v.is_input = text == "true";
                        }
                    }
                    ("variables", "isOutput") => {
                        if let Some(v) = &mut current_var {
                            v.is_output = text == "true";
                        }
                    }
                    // Record operation object
                    (
                        "recordLookups" | "recordCreates" | "recordUpdates" | "recordDeletes",
                        "object",
                    ) => {
                        if let Some(op) = &mut current_record_op {
                            op.object = text;
                        }
                    }
                    // Action call fields
                    ("actionCalls", "actionName") => {
                        if let Some(a) = &mut current_action {
                            a.name = text;
                        }
                    }
                    ("actionCalls", "actionType") => {
                        if let Some(a) = &mut current_action {
                            a.action_type = text;
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    // Fallback: use api_name as label if none found
    if meta.label.is_empty() {
        meta.label = meta.api_name.replace('_', " ");
    }

    Ok(meta)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_FLOW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>59.0</apiVersion>
    <description>Processes new accounts automatically.</description>
    <label>Account Onboarding Flow</label>
    <processType>AutoLaunchedFlow</processType>
    <variables>
        <name>varAccountId</name>
        <dataType>String</dataType>
        <isInput>true</isInput>
        <isOutput>false</isOutput>
    </variables>
    <variables>
        <name>varResult</name>
        <dataType>Boolean</dataType>
        <isInput>false</isInput>
        <isOutput>true</isOutput>
    </variables>
    <decisions>
        <name>Check_Active</name>
        <rules>
            <name>Is_Active</name>
        </rules>
    </decisions>
    <recordLookups>
        <name>Get_Account</name>
        <object>Account</object>
    </recordLookups>
    <recordUpdates>
        <name>Update_Account</name>
        <object>Account</object>
    </recordUpdates>
    <actionCalls>
        <name>Send_Email</name>
        <actionName>emailSimple</actionName>
        <actionType>emailAlert</actionType>
    </actionCalls>
</Flow>"#;

    #[test]
    fn parses_label() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.label, "Account Onboarding Flow");
    }

    #[test]
    fn parses_process_type() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.process_type, "AutoLaunchedFlow");
    }

    #[test]
    fn parses_description() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert!(meta.description.contains("Processes new accounts"));
    }

    #[test]
    fn parses_variables() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.variables.len(), 2);
        let input_var = meta
            .variables
            .iter()
            .find(|v| v.name == "varAccountId")
            .unwrap();
        assert!(input_var.is_input);
        assert!(!input_var.is_output);
        let output_var = meta
            .variables
            .iter()
            .find(|v| v.name == "varResult")
            .unwrap();
        assert!(!output_var.is_input);
        assert!(output_var.is_output);
    }

    #[test]
    fn counts_decisions() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.decisions, 1);
    }

    #[test]
    fn parses_record_operations() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.record_operations.len(), 2);
        assert!(meta
            .record_operations
            .iter()
            .any(|op| op.operation == "lookup" && op.object == "Account"));
        assert!(meta
            .record_operations
            .iter()
            .any(|op| op.operation == "update" && op.object == "Account"));
    }

    #[test]
    fn parses_action_calls() {
        let meta = parse_flow("Account_Onboarding_Flow", SAMPLE_FLOW).unwrap();
        assert_eq!(meta.action_calls.len(), 1);
        assert_eq!(meta.action_calls[0].name, "emailSimple");
        assert_eq!(meta.action_calls[0].action_type, "emailAlert");
    }

    #[test]
    fn api_name_fallback_label() {
        let src = r#"<?xml version="1.0"?><Flow><processType>Flow</processType></Flow>"#;
        let meta = parse_flow("My_Test_Flow", src).unwrap();
        assert_eq!(meta.label, "My Test Flow");
    }

    // -----------------------------------------------------------------------
    // Edge cases & negative tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_xml_returns_defaults() {
        let src = r#"<?xml version="1.0"?><Flow></Flow>"#;
        let meta = parse_flow("Empty_Flow", src).unwrap();
        assert_eq!(meta.api_name, "Empty_Flow");
        assert_eq!(meta.label, "Empty Flow");
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
    fn action_call_with_empty_action_name_is_skipped() {
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
}
