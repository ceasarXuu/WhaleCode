use super::*;

#[test]
fn accepts_complete_root_work_finish_initialization() {
    let args = parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    )
    .expect("valid args");
    assert!(matches!(args, TaskSpaceControlArgs::InitializeMap { .. }));
    assert_eq!(args.nested_actions().len(), 1);
}

#[test]
fn accepts_zero_edge_three_node_initialization_for_runtime_validator() {
    parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    )
    .expect("schema-level zero-edge two-node graph is parseable");
}

#[test]
fn accepts_one_patch_followed_by_non_patch_actions() {
    let args = parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"*** Begin Patch\n*** End Patch"},"actions":[{"tool_name":"exec_command","arguments":{"cmd":"cargo test"}}]}}"#,
    )
    .expect("valid patch continuation");
    let actions = args.nested_actions();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0].tool_name(), "apply_patch");
    assert_eq!(actions[1].tool_name(), "exec_command");
}

#[test]
fn rejects_legacy_actions_old_fields_and_patch_in_ordinary_actions() {
    let legacy = r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","actions":[{"tool_name":"apply_patch","input":"patch"}]}"#;
    assert!(parse_taskspace_control_args(legacy).is_err());
    let old_field = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start","kind":"inspect_code_context"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#;
    assert!(parse_taskspace_control_args(old_field).is_err());
    let misplaced = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"apply_patch","input":"patch"}]}}"#;
    assert!(parse_taskspace_control_args(misplaced).is_err());
    let repeated = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch-1"},"actions":[{"tool_name":"apply_patch","input":"patch-2"}]}}"#;
    assert!(parse_taskspace_control_args(repeated).is_err());
    let old_current_id = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"work_nodes":[{"node_id":"work","goal":"Do work"}],"finish":{"node_id":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"current_node_id":"work","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#;
    assert!(parse_taskspace_control_args(old_current_id).is_err());
}

#[test]
fn rejects_removed_active_actions() {
    assert!(parse_taskspace_control_args(r#"{"action":"create_node","goal":"x"}"#).is_err());
    assert!(parse_taskspace_control_args(r#"{"action":"finish_nodes","finishes":[]}"#).is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"finish_then_end","finish_node_ids":["finish"],"final_candidate":"answer"}"#
    )
    .is_err());
}

#[test]
fn rejects_finish_work_goal_instead_of_ignoring_it() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do and verify work"},"additional_work_nodes":[],"finish":{"node_id":"finish","goal":"Verify and summarize"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("finish"), "{message}");
    assert!(message.contains("unknown field `goal`"), "{message}");
}

#[test]
fn reports_root_path_when_required_goal_is_missing() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("root"), "{message}");
    assert!(message.contains("missing field `goal`"), "{message}");
}

#[test]
fn reports_array_index_when_additional_work_goal_is_missing() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[{"node_id":"later"}],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("additional_work_nodes[0]"), "{message}");
    assert!(message.contains("missing field `goal`"), "{message}");
}

#[test]
fn accepts_r6_active_actions() {
    let mutation = parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[],"remove_edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    )
    .expect("valid graph mutation");
    assert_eq!(mutation.nested_actions().len(), 1);
    let bind = parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"bind","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    )
    .expect("valid transition");
    assert_eq!(bind.nested_actions().len(), 1);
    parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":3,"node_id":"new","transition":"rework"}"#,
    )
    .expect("valid rework transition");
    parse_taskspace_control_args(
        r#"{"action":"finish_end","expected_revision":4,"final_summary":"Done"}"#,
    )
    .expect("valid finish");
}

#[test]
fn continuation_is_required_for_bind_and_forbidden_for_other_transitions() {
    assert!(parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"bind"}"#,
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"complete","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
    )
    .is_err());
}

#[test]
fn mutation_arrays_are_required_and_not_all_empty() {
    assert!(parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[],"add_edges":[],"remove_edges":[]}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[]}"#
    )
    .is_err());
}

#[test]
fn validates_non_empty_and_duplicate_ids_and_edges() {
    assert!(parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"same","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"same"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"root","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[{"node_id":"work","goal":"Do work"}],"finish":{"node_id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[],"add_edges":[{"from":"a","to":"b"},{"from":"a","to":"b"}],"remove_edges":[]}"#
    )
    .is_err());
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"transition_node","expected_revision":2,"node_id":"","transition":"bind","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#
        )
        .is_err()
    );
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"finish_end","expected_revision":3,"final_summary":""}"#
        )
        .is_err()
    );
}

#[test]
fn expand_nodes_requires_non_empty_unique_node_ids() {
    parse_taskspace_control_args(r#"{"action":"expand_nodes","node_ids":["node-1","node-2"]}"#)
        .expect("valid expansion batch");
    assert!(parse_taskspace_control_args(r#"{"action":"expand_nodes","node_ids":[]}"#).is_err());
    assert!(
        parse_taskspace_control_args(r#"{"action":"expand_nodes","node_ids":["node-1","node-1"]}"#)
            .is_err()
    );
}

#[test]
fn invalid_arguments_return_one_typed_json_payload() {
    let arguments = r#"{"action":"unknown"}"#;
    let value = invalid_payload(arguments);
    assert_eq!(value["schema_version"], "TaskSpaceControlResultR6V1");
    assert_eq!(value["status"], "protocol_failed");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["class"], "protocol");
    assert_eq!(value["error"]["code"], "invalid_arguments");
    let message = value["error"]["message"].as_str().expect("message");
    assert!(
        message.starts_with("invalid taskspace_control arguments at action:"),
        "{message}"
    );
}

fn invalid_payload(arguments: &str) -> JsonValue {
    let error = parse_taskspace_control_args(arguments).expect_err("arguments should fail");
    let FunctionCallError::RespondToModel(payload) = error else {
        panic!("expected model-facing error");
    };
    serde_json::from_str(&payload).expect("single JSON payload")
}
