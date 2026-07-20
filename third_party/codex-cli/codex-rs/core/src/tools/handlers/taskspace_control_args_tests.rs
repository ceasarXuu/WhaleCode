use super::*;
use serde_json::Value as JsonValue;

#[test]
fn accepts_complete_root_work_finish_initialization() {
    let args = parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"required_next_call":"ordinary_tool"}"#,
    )
    .expect("valid args");
    assert!(matches!(args, TaskSpaceControlArgs::InitializeMap { .. }));
    assert_eq!(
        args.required_next_call(),
        Some(TaskSpaceRequiredNextCall::OrdinaryTool)
    );
}

#[test]
fn accepts_zero_edge_three_node_initialization_for_runtime_validator() {
    parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#,
    )
    .expect("schema-level zero-edge two-node graph is parseable");
}

#[test]
fn accepts_declared_top_level_patch_requirement() {
    let args = parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"apply_patch"}"#,
    )
    .expect("valid patch requirement");
    assert_eq!(
        args.required_next_call(),
        Some(TaskSpaceRequiredNextCall::ApplyPatch)
    );
}

#[test]
fn rejects_legacy_actions_old_fields_and_patch_in_ordinary_actions() {
    let legacy = r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","actions":[{"tool_name":"apply_patch","input":"patch"}]}"#;
    assert!(parse_taskspace_control_args(legacy).is_err());
    let old_field = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start","kind":"inspect_code_context"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#;
    assert!(parse_taskspace_control_args(old_field).is_err());
    let nested_actions = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"continuation":{"kind":"actions","actions":[{"tool_name":"apply_patch","input":"patch"}]}}"#;
    assert!(parse_taskspace_control_args(nested_actions).is_err());
    let nested_patch = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch-1"}}}"#;
    assert!(parse_taskspace_control_args(nested_patch).is_err());
    let legacy_scalar = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"continuation":"next_tool"}"#;
    assert!(parse_taskspace_control_args(legacy_scalar).is_err());
    let old_current_id = r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"work_nodes":[{"node_id":"work","goal":"Do work"}],"finish_identity":{"id":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"current_node_id":"work","required_next_call":"ordinary_tool"}"#;
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
    assert!(parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"bind","required_next_call":"ordinary_tool"}"#
    )
    .is_err());
}

#[test]
fn rejects_finish_identity_work_goal_instead_of_ignoring_it() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do and verify work"},"additional_work_nodes":[],"finish_identity":{"id":"finish","goal":"Verify and summarize"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"required_next_call":"ordinary_tool"}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("finish_identity"), "{message}");
    assert!(message.contains("unknown field `goal`"), "{message}");
}

#[test]
fn rejects_legacy_finish_wire_shapes() {
    for arguments in [
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish":{"node_id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#,
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"node_id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#,
    ] {
        let payload = invalid_payload(arguments);
        let message = payload["error"]["message"].as_str().expect("message");
        assert!(message.contains("finish"), "{message}");
    }
}

#[test]
fn reports_root_path_when_required_goal_is_missing() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("root"), "{message}");
    assert!(message.contains("missing field `goal`"), "{message}");
}

#[test]
fn reports_array_index_when_additional_work_goal_is_missing() {
    let payload = invalid_payload(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[{"node_id":"later"}],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#,
    );
    let message = payload["error"]["message"].as_str().expect("message");
    assert!(message.contains("additional_work_nodes[0]"), "{message}");
    assert!(message.contains("missing field `goal`"), "{message}");
}

#[test]
fn accepts_direct_lifecycle_actions() {
    let mutation = parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[],"remove_edges":[],"required_next_call":"ordinary_tool"}"#,
    )
    .expect("valid graph mutation");
    assert_eq!(
        mutation.required_next_call(),
        Some(TaskSpaceRequiredNextCall::OrdinaryTool)
    );
    let bind = parse_taskspace_control_args(
        r#"{"action":"bind_node","expected_revision":2,"node_id":"new","required_next_call":"ordinary_tool"}"#,
    )
    .expect("valid bind");
    assert_eq!(
        bind.required_next_call(),
        Some(TaskSpaceRequiredNextCall::OrdinaryTool)
    );
    parse_taskspace_control_args(
        r#"{"action":"block_node","expected_revision":3,"node_id":"new"}"#,
    )
    .expect("valid block");
    parse_taskspace_control_args(
        r#"{"action":"unblock_node","expected_revision":4,"node_id":"new"}"#,
    )
    .expect("valid unblock");
    parse_taskspace_control_args(
        r#"{"action":"rework_node","expected_revision":5,"node_id":"new"}"#,
    )
    .expect("valid rework");
    let handoff = parse_taskspace_control_args(
        r#"{"action":"complete_then_continue","expected_revision":6,"current_node_id":"new","next_node_id":"verify","required_next_call":"ordinary_tool"}"#,
    )
    .expect("valid atomic handoff");
    assert_eq!(
        handoff.required_next_call(),
        Some(TaskSpaceRequiredNextCall::OrdinaryTool)
    );
    parse_taskspace_control_args(
        r#"{"action":"complete_then_end","expected_revision":7,"current_node_id":"verify","final_summary":"Done"}"#,
    )
    .expect("valid atomic terminal completion");
    parse_taskspace_control_args(
        r#"{"action":"finish_end","expected_revision":8,"final_summary":"Done"}"#,
    )
    .expect("valid finish");
}

#[test]
fn standalone_complete_is_unrepresentable_and_handoffs_require_next_call() {
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"bind_node","expected_revision":2,"node_id":"new"}"#,
        )
        .is_err()
    );
    assert!(parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"complete","required_next_call":"ordinary_tool"}"#,
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"transition_node","expected_revision":2,"node_id":"new","transition":"complete"}"#,
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"complete_then_continue","expected_revision":2,"current_node_id":"new","next_node_id":"verify"}"#,
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"block_node","expected_revision":2,"node_id":"new","required_next_call":"ordinary_tool"}"#,
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"bind_node","expected_revision":2,"node_id":"new","transition":"bind","required_next_call":"ordinary_tool"}"#,
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
        r#"{"action":"initialize_map","root":{"node_id":"same","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"same"},"edges":[],"required_next_call":"ordinary_tool"}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"root","goal":"Do work"},"additional_work_nodes":[],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Do work"},"additional_work_nodes":[{"node_id":"work","goal":"Do work"}],"finish_identity":{"id":"finish"},"edges":[],"required_next_call":"ordinary_tool"}"#
    )
    .is_err());
    assert!(parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[],"add_edges":[{"from":"a","to":"b"},{"from":"a","to":"b"}],"remove_edges":[]}"#
    )
    .is_err());
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"bind_node","expected_revision":2,"node_id":"","required_next_call":"ordinary_tool"}"#
        )
        .is_err()
    );
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"finish_end","expected_revision":3,"final_summary":""}"#
        )
        .is_err()
    );
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"complete_then_end","expected_revision":3,"current_node_id":"work","final_summary":""}"#
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
fn read_map_accepts_only_the_action_tag() {
    assert!(matches!(
        parse_taskspace_control_args(r#"{"action":"read_map"}"#).expect("valid map read"),
        TaskSpaceControlArgs::ReadMap
    ));
    assert!(
        parse_taskspace_control_args(r#"{"action":"read_map","expected_revision":2}"#).is_err()
    );
}

#[test]
fn read_output_ref_modes_accept_only_their_direct_schema() {
    for arguments in [
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"head","max_bytes":64}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"tail","max_bytes":64}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"line_range","start_line":1,"end_line":3,"max_bytes":64}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"grep","pattern":"needle","max_bytes":64}"#,
    ] {
        parse_taskspace_control_args(arguments).expect("valid direct read mode");
    }

    for arguments in [
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"head"}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"head","pattern":"extra","max_bytes":64}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"line_range","start_line":0,"end_line":3,"max_bytes":64}"#,
        r#"{"action":"read_output_ref","output_ref":"ref-1","mode":"grep","pattern":"needle","max_bytes":0}"#,
    ] {
        assert!(
            parse_taskspace_control_args(arguments).is_err(),
            "{arguments}"
        );
    }
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

#[test]
fn rejects_trailing_json_instead_of_executing_the_first_value() {
    let valid = r#"{"action":"finish_end","expected_revision":3,"final_summary":"Done"}"#;
    for arguments in [
        format!("{valid}}}"),
        format!(r#"{valid} {{"action":"finish_end"}}"#),
    ] {
        let value = invalid_payload(&arguments);
        assert_eq!(value["status"], "protocol_failed");
        assert_eq!(value["success"], false);
        assert_eq!(value["state_commit"], false);
        assert_eq!(value["partial_commit"], 0);
        let message = value["error"]["message"].as_str().expect("message");
        assert!(message.contains("trailing characters"), "{message}");
    }
}

fn invalid_payload(arguments: &str) -> JsonValue {
    let error = parse_taskspace_control_args(arguments).expect_err("arguments should fail");
    let FunctionCallError::RespondToModel(payload) = error else {
        panic!("expected model-facing error");
    };
    serde_json::from_str(&payload).expect("single JSON payload")
}
