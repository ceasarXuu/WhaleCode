use super::*;

#[test]
fn parses_agent_authored_map() {
    let args = parse_taskspace_control_args(
        &serde_json::json!({
            "action": "initialize_then_actions",
            "task_title": "Patch bug",
            "task_objective": "Fix and verify",
            "initial_nodes": [{
                "node_id": "inspect",
                "kind": "inspect_code_context",
                "title": "Inspect",
                "context_summary": "Read relevant code"
            }],
            "current_node_id": "inspect",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "pwd"}}]
        })
        .to_string(),
    )
    .expect("parse initialize_then_actions");
    assert!(matches!(
        args,
        TaskSpaceControlArgs::InitializeThenActions { .. }
    ));
}

#[test]
fn initialize_output_preserves_agent_node_ids() {
    let output = format_initialize_map_output(&ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec!["inspect".into(), "implement".into()],
        current_node_id: "inspect".into(),
    });
    let value: JsonValue = output;

    assert_eq!(value["schema_version"], "TaskSpaceInitializeMapResultV1");
    assert_eq!(value["current_node_id"], "inspect");
    assert_eq!(
        value["node_ids"],
        serde_json::json!(["inspect", "implement"])
    );
}

#[test]
fn parses_agent_authored_terminal_candidate() {
    let args = parse_taskspace_control_args(
        &serde_json::json!({
            "action": "finish_then_end",
            "terminal_finish": {"result_summary": "Validation passed."},
            "final_candidate": "Exact final answer."
        })
        .to_string(),
    )
    .expect("parse terminal finish");
    assert!(matches!(
        args,
        TaskSpaceControlArgs::FinishThenEnd {
            final_candidate: candidate,
            ..
        } if candidate == "Exact final answer."
    ));
}

#[test]
fn rejects_removed_semantic_action_at_parse_boundary() {
    let error = parse_taskspace_control_args(
        &serde_json::json!({
            "action": "record_fact",
            "claim_id": "fact-1",
            "statement": "legacy"
        })
        .to_string(),
    )
    .expect_err("removed action");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn finish_without_next_action_is_rejected() {
    let error = parse_taskspace_control_args(
        r#"{"action":"finish_then_actions","finishes":[{"result_summary":"done"}],"actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}"#,
    )
    .expect_err("missing next binding");
    assert!(error.to_string().contains("requires exactly one"));
}

#[test]
fn hard_state_reason_is_mechanical() {
    assert_eq!(
        hard_state_reason("blocked. hard_state: node_tool_calls_in_flight. rejected"),
        Some("node_tool_calls_in_flight")
    );
}
