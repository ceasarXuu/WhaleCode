use super::*;
use serde_json::Value as JsonValue;
use serde_json::json;

fn initialize_fixture() -> JsonValue {
    json!({
        "action": "initialize_and_execute",
        "root": {"node_id": "root", "goal": "Solve the task"},
        "work_nodes": [
            {"node_id": "inspect", "goal": "Inspect the implementation"},
            {"node_id": "research", "goal": "Check the contract"}
        ],
        "finish": {"node_id": "finish", "goal": "Verify and summarize"},
        "edges": [
            {"from": "root", "to": "inspect"},
            {"from": "root", "to": "research"},
            {"from": "inspect", "to": "finish"},
            {"from": "research", "to": "finish"}
        ],
        "actions": [
            {"node_id": "inspect", "tool": "read_file"},
            {"node_id": "research", "tool": "exec_command"}
        ]
    })
}

fn execute_fixture() -> JsonValue {
    json!({
        "action": "execute",
        "expected_revision": 12,
        "mutations": [
            {"action": "complete_node", "node_id": "inspect"}
        ],
        "actions": [
            {"node_id": "research", "tool": "exec_command"}
        ]
    })
}

fn reopen_fixture() -> JsonValue {
    json!({
        "action": "reopen_map",
        "expected_revision": 14,
        "work_nodes": [
            {"node_id": "address-feedback", "goal": "Address user feedback"}
        ],
        "edges": [
            {"from": "root", "to": "address-feedback"},
            {"from": "address-feedback", "to": "finish"}
        ],
        "actions": [
            {"node_id": "address-feedback", "tool": "read_file"}
        ]
    })
}

#[test]
fn parser_accepts_exactly_the_six_top_level_actions() {
    let fixtures = [
        initialize_fixture(),
        execute_fixture(),
        reopen_fixture(),
        json!({"action": "read_map"}),
        json!({
            "action": "read_output_ref",
            "output_ref": "output-1",
            "mode": "head",
            "max_bytes": 64
        }),
        json!({
            "action": "finish_map",
            "expected_revision": 13,
            "finish_node_id": "finish",
            "complete_work_node_ids": ["verify"],
            "exact_summary": "All required work is complete."
        }),
    ];

    for fixture in fixtures {
        parse_taskspace_control_args(&fixture.to_string()).expect("valid B1X action");
    }
}

#[test]
fn initialize_and_execute_parser_matches_the_wire_golden() {
    let parsed = parse_taskspace_control_args(&initialize_fixture().to_string())
        .expect("valid initialization");
    assert_eq!(
        parsed,
        TaskSpaceControlArgs::InitializeAndExecute {
            root: TaskSpaceGraphNodeArgs {
                node_id: "root".into(),
                goal: "Solve the task".into(),
            },
            work_nodes: vec![
                TaskSpaceGraphNodeArgs {
                    node_id: "inspect".into(),
                    goal: "Inspect the implementation".into(),
                },
                TaskSpaceGraphNodeArgs {
                    node_id: "research".into(),
                    goal: "Check the contract".into(),
                },
            ],
            finish: TaskSpaceGraphNodeArgs {
                node_id: "finish".into(),
                goal: "Verify and summarize".into(),
            },
            edges: vec![
                TaskSpaceGraphEdgeArgs {
                    from: "root".into(),
                    to: "inspect".into(),
                },
                TaskSpaceGraphEdgeArgs {
                    from: "root".into(),
                    to: "research".into(),
                },
                TaskSpaceGraphEdgeArgs {
                    from: "inspect".into(),
                    to: "finish".into(),
                },
                TaskSpaceGraphEdgeArgs {
                    from: "research".into(),
                    to: "finish".into(),
                },
            ],
            actions: vec![
                TaskSpaceActionArgs {
                    node_id: "inspect".into(),
                    tool: "read_file".into(),
                },
                TaskSpaceActionArgs {
                    node_id: "research".into(),
                    tool: "exec_command".into(),
                },
            ],
        }
    );
}

#[test]
fn execute_accepts_every_frozen_mutation_variant() {
    let fixture = json!({
        "action": "execute",
        "expected_revision": 7,
        "mutations": [
            {
                "action": "add_work_nodes",
                "work_nodes": [{"node_id": "verify", "goal": "Verify the change"}]
            },
            {
                "action": "add_edges",
                "edges": [{"from": "implement", "to": "verify"}]
            },
            {
                "action": "remove_edges",
                "edges": [{"from": "implement", "to": "finish"}]
            },
            {"action": "complete_node", "node_id": "implement"},
            {"action": "block_node", "node_id": "verify"},
            {"action": "unblock_node", "node_id": "verify"}
        ],
        "actions": [{"node_id": "verify", "tool": "exec_command"}]
    });
    let TaskSpaceControlArgs::Execute {
        expected_revision,
        mutations,
        actions,
    } = parse_taskspace_control_args(&fixture.to_string()).expect("all mutations")
    else {
        panic!("execute expected");
    };

    assert_eq!(expected_revision, 7);
    assert_eq!(mutations.len(), 6);
    assert_eq!(
        actions,
        vec![TaskSpaceActionArgs {
            node_id: "verify".into(),
            tool: "exec_command".into(),
        }]
    );
}

#[test]
fn execute_allows_no_mutation_but_never_allows_no_action() {
    let parsed = parse_taskspace_control_args(
        &json!({
            "action": "execute",
            "expected_revision": 7,
            "actions": [{"node_id": "inspect", "tool": "read_file"}]
        })
        .to_string(),
    )
    .expect("ordinary progress without a mutation");
    let TaskSpaceControlArgs::Execute { mutations, .. } = parsed else {
        panic!("execute expected");
    };
    assert!(mutations.is_empty());

    for fixture in [
        json!({"action":"execute","expected_revision":7,"mutations":[],"actions":[]}),
        json!({
            "action":"initialize_and_execute",
            "root":{"node_id":"root","goal":"Root"},
            "work_nodes":[{"node_id":"work","goal":"Work"}],
            "finish":{"node_id":"finish","goal":"Finish"},
            "edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],
            "actions":[]
        }),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
    }
}

#[test]
fn top_level_contract_rejects_missing_extra_and_wrong_types() {
    let fixtures = [
        initialize_fixture(),
        execute_fixture(),
        reopen_fixture(),
        json!({"action":"read_map"}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"tail","max_bytes":64}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"finish","complete_work_node_ids":["verify"],"exact_summary":"Done"}),
    ];

    for fixture in fixtures {
        for required in fixture.as_object().expect("object").keys() {
            if fixture["action"] == "execute" && required == "mutations" {
                continue;
            }
            let mut missing = fixture.clone();
            missing.as_object_mut().expect("object").remove(required);
            assert!(
                parse_taskspace_control_args(&missing.to_string()).is_err(),
                "accepted missing {required}: {missing}"
            );
        }

        let mut extra = fixture.clone();
        extra
            .as_object_mut()
            .expect("object")
            .insert("unexpected".into(), json!(true));
        assert!(
            parse_taskspace_control_args(&extra.to_string()).is_err(),
            "accepted extra field: {extra}"
        );
    }

    for fixture in [
        json!({"action":"execute","expected_revision":"7","mutations":[],"actions":[{"node_id":"n","tool":"t"}]}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":7,"complete_work_node_ids":["verify"],"exact_summary":"Done"}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"head","max_bytes":"64"}),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
    }
}

#[test]
fn nested_manifest_and_mutation_contracts_are_strict() {
    for fixture in [
        json!({"action":"execute","expected_revision":1,"mutations":[],"actions":[{"node_id":"n","tool":"t","arguments":{}}]}),
        json!({"action":"execute","expected_revision":1,"mutations":[],"actions":[{"node_id":"n"}]}),
        json!({"action":"execute","expected_revision":1,"mutations":[{"action":"complete_node","node_id":"n","reason":"done"}],"actions":[{"node_id":"n","tool":"t"}]}),
        json!({"action":"execute","expected_revision":1,"mutations":[{"action":"add_edges","edges":[]}],"actions":[{"node_id":"n","tool":"t"}]}),
        json!({"action":"execute","expected_revision":1,"mutations":[{"action":"unknown","node_id":"n"}],"actions":[{"node_id":"n","tool":"t"}]}),
        json!({"action":"execute","expected_revision":1,"mutations":[{"action":"complete_node"}],"actions":[{"node_id":"n","tool":"t"}]}),
    ] {
        assert!(
            parse_taskspace_control_args(&fixture.to_string()).is_err(),
            "{fixture}"
        );
    }
}

#[test]
fn initialization_rejects_empty_or_overlapping_role_nodes() {
    for fixture in [
        json!({
            "action":"initialize_and_execute",
            "root":{"node_id":"root","goal":"Root"},
            "work_nodes":[],
            "finish":{"node_id":"finish","goal":"Finish"},
            "edges":[{"from":"root","to":"finish"}],
            "actions":[{"node_id":"root","tool":"read_file"}]
        }),
        json!({
            "action":"initialize_and_execute",
            "root":{"node_id":"root","goal":"Root"},
            "work_nodes":[{"node_id":"root","goal":"Duplicate"}],
            "finish":{"node_id":"finish","goal":"Finish"},
            "edges":[{"from":"root","to":"finish"}],
            "actions":[{"node_id":"root","tool":"read_file"}]
        }),
        json!({
            "action":"initialize_and_execute",
            "root":{"node_id":"root","goal":"Root"},
            "work_nodes":[{"node_id":"work","goal":"Work"}],
            "finish":{"node_id":"work","goal":"Duplicate"},
            "edges":[{"from":"root","to":"work"}],
            "actions":[{"node_id":"work","tool":"read_file"}]
        }),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
    }
}

#[test]
fn read_output_ref_modes_keep_exact_direct_contracts() {
    for fixture in [
        json!({"action":"read_output_ref","output_ref":"ref","mode":"head","max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"tail","max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"line_range","start_line":1,"end_line":3,"max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"grep","pattern":"needle","max_bytes":64}),
    ] {
        parse_taskspace_control_args(&fixture.to_string()).expect("valid read");
    }

    for fixture in [
        json!({"action":"read_output_ref","output_ref":"","mode":"head","max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"head","pattern":"extra","max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"line_range","start_line":3,"end_line":1,"max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"grep","pattern":"","max_bytes":64}),
        json!({"action":"read_output_ref","output_ref":"ref","mode":"grep","pattern":"x","max_bytes":0}),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
    }
}

#[test]
fn old_carriers_and_lifecycle_actions_are_rejected() {
    for action in [
        "initialize_map",
        "mutate_graph",
        "bind_node",
        "complete_then_continue",
        "block_node",
        "unblock_node",
        "rework_node",
        "expand_nodes",
        "active",
        "after_boundary",
    ] {
        let fixture = json!({"action":action});
        assert!(
            parse_taskspace_control_args(&fixture.to_string()).is_err(),
            "{action}"
        );
    }
}

#[test]
fn finish_map_rejects_old_identity_fields_and_empty_values() {
    for fixture in [
        json!({"action":"finish_map","expected_revision":3,"terminal_node_id":"work","final_summary":"Done"}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"","complete_work_node_ids":["verify"],"exact_summary":"Done"}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"finish","complete_work_node_ids":[],"exact_summary":"Done"}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"finish","complete_work_node_ids":["verify","verify"],"exact_summary":"Done"}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"finish","complete_work_node_ids":["verify"],"exact_summary":""}),
        json!({"action":"finish_map","expected_revision":3,"finish_node_id":"finish","complete_work_node_ids":["verify"],"exact_summary":"Done","actions":[]}),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
    }
}

#[test]
fn reopen_map_rejects_empty_structure_and_standalone_shape() {
    for fixture in [
        json!({"action":"reopen_map","expected_revision":14,"work_nodes":[],"edges":[{"from":"root","to":"finish"}],"actions":[{"node_id":"work","tool":"read_file"}]}),
        json!({"action":"reopen_map","expected_revision":14,"work_nodes":[{"node_id":"work","goal":"Work"}],"edges":[],"actions":[{"node_id":"work","tool":"read_file"}]}),
        json!({"action":"reopen_map","expected_revision":14,"work_nodes":[{"node_id":"work","goal":"Work"}],"edges":[{"from":"root","to":"work"}],"actions":[]}),
    ] {
        assert!(parse_taskspace_control_args(&fixture.to_string()).is_err());
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
