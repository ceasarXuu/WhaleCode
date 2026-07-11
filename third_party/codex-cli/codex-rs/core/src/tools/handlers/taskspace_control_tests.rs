use super::*;

#[test]
fn parses_agent_authored_map() {
    let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
        "action": "initialize_map",
        "task_title": "Patch bug",
        "task_objective": "Fix and verify",
        "initial_nodes": [{
            "node_key": "inspect",
            "kind": "inspect_code_context",
            "title": "Inspect",
            "context_summary": "Read relevant code"
        }],
        "current_node_key": "inspect"
    }))
    .expect("parse initialize_map");
    assert!(matches!(args, TaskSpaceControlArgs::InitializeMap { .. }));
}

#[test]
fn initialize_output_has_directional_node_mapping() {
    let output = format_initialize_map_output(&ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec![
            ("inspect".into(), "node-1".into()),
            ("implement".into(), "node-2".into()),
        ],
        current_node_id: "node-1".into(),
    });
    let value: JsonValue = serde_json::from_str(&output).expect("structured output");

    assert_eq!(value["schema_version"], "TaskSpaceInitializeMapResultV1");
    assert_eq!(value["current_node_key"], "inspect");
    assert_eq!(value["current_node_id"], "node-1");
    assert_eq!(value["node_id_by_key"]["inspect"], "node-1");
    assert_eq!(value["node_id_by_key"]["implement"], "node-2");
}

#[test]
fn parses_agent_authored_terminal_candidate() {
    let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
        "action": "finish_node",
        "result_summary": "Validation passed.",
        "final_candidate": "Exact final answer."
    }))
    .expect("parse terminal finish");
    assert!(matches!(
        args,
        TaskSpaceControlArgs::FinishNode {
            final_candidate: Some(candidate),
            ..
        } if candidate == "Exact final answer."
    ));
}

#[test]
fn rejects_removed_semantic_action_at_parse_boundary() {
    let error = serde_json::from_value::<TaskSpaceControlArgs>(serde_json::json!({
        "action": "record_fact",
        "claim_id": "fact-1",
        "statement": "legacy"
    }))
    .expect_err("removed action");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn finish_without_next_node_has_no_draft() {
    assert_eq!(
        build_next_node_draft(None, None, None, Vec::new()).expect("draft"),
        None
    );
}

#[test]
fn hard_state_reason_is_mechanical() {
    assert_eq!(
        hard_state_reason("blocked. hard_state: node_tool_calls_in_flight. rejected"),
        Some("node_tool_calls_in_flight")
    );
}
