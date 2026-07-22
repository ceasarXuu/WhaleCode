use super::*;

fn action_names(spec: ToolSpec) -> Vec<String> {
    let ToolSpec::Function(tool) = spec else {
        panic!("taskspace_control must be a function tool");
    };
    tool.parameters
        .any_of
        .expect("control variants")
        .into_iter()
        .map(|variant| {
            variant.properties.expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                .as_str()
                .expect("action string")
                .to_string()
        })
        .collect()
}

#[test]
fn standalone_control_excludes_action_carrying_transitions() {
    let actions = action_names(create_taskspace_control_tool());

    assert!(!actions.contains(&"initialize_map".to_string()));
    assert!(!actions.contains(&"bind_node".to_string()));
    assert!(!actions.contains(&"complete_then_continue".to_string()));
    for action in [
        "mutate_graph",
        "block_node",
        "unblock_node",
        "rework_node",
        "complete_last_running_work_then_end",
        "close_finish_with_no_active_work",
        "expand_nodes",
        "read_map",
        "read_output_ref",
    ] {
        assert!(actions.contains(&action.to_string()), "missing {action}");
    }
    assert!(!actions.contains(&"finish_end".to_string()));
    assert!(!actions.contains(&"complete_then_end".to_string()));
    assert!(!actions.contains(&"complete_active_work_then_end".to_string()));
    assert!(!actions.contains(&"close_ready_finish".to_string()));
}

#[test]
fn last_running_work_closure_requires_exact_state_declarations() {
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    let variants = tool.parameters.any_of.expect("control variants");
    let closure = variants
        .iter()
        .find(|variant| {
            variant.properties.as_ref().expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                == json!("complete_last_running_work_then_end")
        })
        .expect("last Running Work closure");

    assert_eq!(
        closure.required.as_ref().expect("required"),
        &[
            "action",
            "expected_revision",
            "current_node_id",
            "other_incomplete_work_status",
            "finish_status",
            "final_summary",
        ]
    );
    let properties = closure.properties.as_ref().expect("properties");
    assert_eq!(
        properties["other_incomplete_work_status"]
            .enum_values
            .as_ref()
            .expect("other incomplete Work enum"),
        &[json!("none")]
    );
    assert_eq!(
        properties["finish_status"]
            .enum_values
            .as_ref()
            .expect("Finish enum"),
        &[json!("pending")]
    );
}

#[test]
fn ready_finish_closure_requires_exact_state_declarations() {
    let ToolSpec::Function(tool) = create_taskspace_control_tool() else {
        panic!("taskspace_control must be a function tool");
    };
    let variants = tool.parameters.any_of.expect("control variants");
    let closure = variants
        .iter()
        .find(|variant| {
            variant.properties.as_ref().expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                == json!("close_finish_with_no_active_work")
        })
        .expect("Ready Finish closure");

    assert_eq!(
        closure.required.as_ref().expect("required"),
        &[
            "action",
            "expected_revision",
            "active_work_status",
            "finish_status",
            "final_summary",
        ]
    );
    let properties = closure.properties.as_ref().expect("properties");
    assert_eq!(
        properties["active_work_status"]
            .enum_values
            .as_ref()
            .expect("active Work enum"),
        &[json!("none")]
    );
    assert_eq!(
        properties["finish_status"]
            .enum_values
            .as_ref()
            .expect("Finish enum"),
        &[json!("ready")]
    );
}

#[test]
fn action_schema_requires_explicit_continuation_or_lifecycle_change() {
    let actions = taskspace_action_schema()
        .any_of
        .expect("transition variants")
        .into_iter()
        .map(|variant| {
            variant.properties.expect("properties")["action"]
                .enum_values
                .as_ref()
                .expect("action enum")[0]
                .as_str()
                .expect("action string")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actions,
        [
            "continue_current",
            "initialize_map",
            "bind_node",
            "complete_then_continue"
        ]
    );
}

#[test]
fn action_schema_has_no_sibling_declaration() {
    let serialized = serde_json::to_string(&taskspace_action_schema()).expect("serialize");
    assert!(!serialized.contains("required_next_call"));
    assert!(!serialized.contains("sibling"));
}

#[test]
fn initialization_keeps_explicit_rooted_graph_contract() {
    let schema = taskspace_action_schema();
    let initialize = &schema.any_of.expect("variants")[1];
    assert_eq!(
        initialize.required.as_ref().expect("required"),
        &[
            "action",
            "root",
            "initial_work_node",
            "finish_identity",
            "additional_work_nodes",
            "edges",
        ]
    );
    let properties = initialize.properties.as_ref().expect("properties");
    assert!(properties.contains_key("root"));
    assert!(properties.contains_key("initial_work_node"));
    assert!(properties.contains_key("finish_identity"));
    assert!(properties.contains_key("edges"));
}
