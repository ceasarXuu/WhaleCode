use super::*;
use serde_json::Value as JsonValue;

#[test]
fn control_accepts_boundary_transitions() {
    for arguments in [
        r#"{"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"work","goal":"Work"}],"root_id":"root","initial_work_id":"work","finish_id":"finish","edges":[]}"#,
        r#"{"action":"bind_node","expected_revision":2,"node_id":"work"}"#,
        r#"{"action":"complete_then_continue","expected_revision":2,"current_node_id":"work","next_node_id":"verify"}"#,
    ] {
        parse_taskspace_control_args(arguments).expect(arguments);
    }
}

#[test]
fn initialize_map_requires_agent_authored_roles_in_the_node_set() {
    for arguments in [
        r#"{"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"work","goal":"Work"}],"root_id":"missing","initial_work_id":"work","finish_id":"finish","edges":[]}"#,
        r#"{"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"work","goal":"Work"}],"root_id":"root","initial_work_id":"missing","finish_id":"finish","edges":[]}"#,
        r#"{"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"root","goal":"Duplicate"},{"id":"work","goal":"Work"}],"root_id":"root","initial_work_id":"work","finish_id":"finish","edges":[]}"#,
        r#"{"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"work","goal":"Work"}],"root_id":"root","initial_work_id":"work","finish_id":"root","edges":[]}"#,
        r#"{"action":"initialize_map","root":{"node_id":"root","goal":"Start"},"initial_work_node":{"node_id":"work","goal":"Work"},"finish_identity":{"id":"finish"},"additional_work_nodes":[],"edges":[]}"#,
    ] {
        assert!(
            parse_taskspace_control_args(arguments).is_err(),
            "{arguments}"
        );
    }
}

#[test]
fn accepts_standalone_graph_and_terminal_actions() {
    parse_taskspace_control_args(
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[],"remove_edges":[]}"#,
    )
    .expect("graph mutation");
    for arguments in [
        r#"{"action":"block_node","expected_revision":2,"node_id":"new"}"#,
        r#"{"action":"unblock_node","expected_revision":3,"node_id":"new"}"#,
        r#"{"action":"rework_node","expected_revision":4,"node_id":"new"}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done"}"#,
        r#"{"action":"finish_map","expected_revision":6,"terminal_node_id":"finish","final_summary":"Done"}"#,
    ] {
        parse_taskspace_control_args(arguments).expect(arguments);
    }
}

#[test]
fn every_control_action_rejects_missing_extra_and_wrong_typed_fields() {
    let fixtures = [
        serde_json::json!({"action":"initialize_map","nodes":[{"id":"root","goal":"Start"},{"id":"work","goal":"Work"}],"root_id":"root","initial_work_id":"work","finish_id":"finish","edges":[]}),
        serde_json::json!({"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[],"remove_edges":[]}),
        serde_json::json!({"action":"bind_node","expected_revision":2,"node_id":"new"}),
        serde_json::json!({"action":"complete_then_continue","expected_revision":2,"current_node_id":"work","next_node_id":"new"}),
        serde_json::json!({"action":"block_node","expected_revision":2,"node_id":"new"}),
        serde_json::json!({"action":"unblock_node","expected_revision":3,"node_id":"new"}),
        serde_json::json!({"action":"rework_node","expected_revision":4,"node_id":"new"}),
        serde_json::json!({"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done"}),
        serde_json::json!({"action":"expand_nodes","node_ids":["new"]}),
        serde_json::json!({"action":"read_map"}),
        serde_json::json!({"action":"read_output_ref","output_ref":"ref-1","mode":"head","max_bytes":64}),
        serde_json::json!({"action":"read_output_ref","output_ref":"ref-1","mode":"tail","max_bytes":64}),
        serde_json::json!({"action":"read_output_ref","output_ref":"ref-1","mode":"line_range","start_line":1,"end_line":2,"max_bytes":64}),
        serde_json::json!({"action":"read_output_ref","output_ref":"ref-1","mode":"grep","pattern":"needle","max_bytes":64}),
    ];

    for fixture in fixtures {
        let object = fixture.as_object().expect("object fixture");
        parse_taskspace_control_args(&fixture.to_string()).expect("valid fixture");

        for required in object.keys() {
            let mut missing = fixture.clone();
            missing
                .as_object_mut()
                .expect("object fixture")
                .remove(required);
            assert!(
                parse_taskspace_control_args(&missing.to_string()).is_err(),
                "accepted missing {required}: {missing}"
            );
        }

        let mut extra = fixture.clone();
        extra
            .as_object_mut()
            .expect("object fixture")
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(
            parse_taskspace_control_args(&extra.to_string()).is_err(),
            "accepted extra field: {extra}"
        );

        let typed_field = object
            .keys()
            .find(|field| field.as_str() != "action")
            .cloned()
            .unwrap_or_else(|| "action".to_string());
        let mut wrong_type = fixture.clone();
        wrong_type
            .as_object_mut()
            .expect("object fixture")
            .insert(typed_field.clone(), serde_json::Value::Null);
        assert!(
            parse_taskspace_control_args(&wrong_type.to_string()).is_err(),
            "accepted null {typed_field}: {wrong_type}"
        );
    }
}

#[test]
fn finish_map_rejects_missing_identity_and_removed_prestate_fields() {
    for arguments in [
        r#"{"action":"finish_map","expected_revision":5,"final_summary":"Done"}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new"}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done","terminal_state":"last_running_work"}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done","incomplete_work_node_ids":["new"]}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done","finish_node_id":"finish"}"#,
        r#"{"action":"finish_map","expected_revision":5,"terminal_node_id":"new","final_summary":"Done","finish_status":"pending"}"#,
    ] {
        assert!(
            parse_taskspace_control_args(arguments).is_err(),
            "{arguments}"
        );
    }
}

#[test]
fn superseded_terminal_action_names_are_rejected() {
    for action in [
        "finish_end",
        "close_ready_finish",
        "complete_then_end",
        "complete_active_work_then_end",
        "complete_last_running_work_then_end",
        "close_finish_with_no_active_work",
    ] {
        let arguments =
            format!(r#"{{"action":"{action}","expected_revision":6,"final_summary":"Done"}}"#);
        assert!(
            parse_taskspace_control_args(&arguments).is_err(),
            "{arguments}"
        );
    }
}

#[test]
fn removed_sibling_metadata_is_rejected() {
    for arguments in [
        r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[],"remove_edges":[],"required_next_call":"ordinary_tool"}"#,
        r#"{"action":"block_node","expected_revision":2,"node_id":"new","required_next_call":"ordinary_tool"}"#,
    ] {
        assert!(
            parse_taskspace_control_args(arguments).is_err(),
            "{arguments}"
        );
    }
}

#[test]
fn mutation_arrays_are_required_and_not_all_empty() {
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[],"add_edges":[],"remove_edges":[]}"#
        )
        .is_err()
    );
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[{"node_id":"new","goal":"New"}],"add_edges":[]}"#
        )
        .is_err()
    );
}

#[test]
fn validates_control_ids_summaries_and_edges() {
    assert!(
        parse_taskspace_control_args(
            r#"{"action":"mutate_graph","expected_revision":1,"add_nodes":[],"add_edges":[{"from":"a","to":"b"},{"from":"a","to":"b"}],"remove_edges":[]}"#
        )
        .is_err()
    );
    for arguments in [
        r#"{"action":"block_node","expected_revision":2,"node_id":""}"#,
        r#"{"action":"finish_map","expected_revision":3,"terminal_node_id":"work","final_summary":""}"#,
        r#"{"action":"finish_map","expected_revision":3,"terminal_node_id":"","final_summary":"Done"}"#,
    ] {
        assert!(
            parse_taskspace_control_args(arguments).is_err(),
            "{arguments}"
        );
    }
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
    let value = invalid_payload(r#"{"action":"unknown"}"#);
    assert_eq!(value["schema_version"], "TaskSpaceControlResultV2");
    assert_eq!(value["status"], "argument_failed");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "TASKSPACE_INVALID_ARGUMENT");
    assert_eq!(value["partial_commit"], false);
}

#[test]
fn rejects_trailing_json() {
    for arguments in [
        r#"{"action":"read_map"}}"#,
        r#"{"action":"read_map"} {"action":"read_map"}"#,
    ] {
        let value = invalid_payload(arguments);
        assert_eq!(value["status"], "argument_failed");
        assert_eq!(value["state_commit"], false);
    }
}

fn invalid_payload(arguments: &str) -> JsonValue {
    let error = parse_taskspace_control_args(arguments).expect_err("arguments should fail");
    let FunctionCallError::RespondToModel(payload) = error else {
        panic!("expected model-facing error");
    };
    serde_json::from_str(&payload).expect("single JSON payload")
}
