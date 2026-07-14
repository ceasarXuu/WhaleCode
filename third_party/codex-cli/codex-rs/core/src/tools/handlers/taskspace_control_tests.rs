use super::*;
use crate::action_map::ActionMapControlState;
use crate::action_map::ActionMapFinishNodeOutcome;
use crate::action_map::ActionMapInitializeOutcome;
use crate::tools::handlers::taskspace_control_output::format_terminal_chain_steps;

fn control_state() -> ActionMapControlState {
    ActionMapControlState {
        task_id: "task-1".into(),
        task_status: "active".into(),
        map_id: "map-1".into(),
        map_status: "active".into(),
        current_node_id: Some("implement".into()),
        pending_node_ids: Vec::new(),
        open_node_ids: vec!["implement".into(), "verify".into()],
        blocked_node_ids: Vec::new(),
        completed_node_count: 1,
        total_node_count: 3,
    }
}

#[test]
fn parses_agent_authored_map() {
    let args = parse_taskspace_control_args(
        &serde_json::json!({
            "action": "initialize_then_actions",
            "initial_nodes": [{
                "node_id": "inspect",
                "kind": "inspect_code_context",
                "goal": "Read relevant code"
            }],
            "current_node_id": "inspect",
            "continuation": {
                "kind": "actions",
                "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "pwd"}}]
            }
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
fn initialize_output_preserves_all_committed_identities() {
    let output = format_initialize_step(&ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec!["inspect".into(), "implement".into()],
        current_node_id: "inspect".into(),
    });
    let value: JsonValue = output;

    assert_eq!(value["kind"], "map_initialized");
    assert_eq!(value["task_id"], "task-1");
    assert_eq!(value["map_id"], "map-1");
    assert_eq!(value["current_node_id"], "inspect");
    assert_eq!(
        value["created_node_ids"],
        serde_json::json!(["inspect", "implement"])
    );
}

#[test]
fn successful_state_batch_preserves_committed_transition_identities() {
    let output = format_state_batch(
        vec![serde_json::json!({
            "kind": "state_transition",
            "index": 0,
            "finished_node_id": "inspect",
            "result_id": "result-1",
            "next": {"kind": "existing", "node_id": "implement"},
            "current_node_id": "implement",
        })],
        true,
        StateCommit::Full,
        Some(&control_state()),
    );
    let value: JsonValue = serde_json::from_str(&output).expect("success batch json");

    assert_eq!(value["status"], "committed");
    assert_eq!(value["schema_version"], "TaskSpaceControlResultV3");
    assert!(value.get("action").is_none());
    assert_eq!(value["success"], true);
    assert_eq!(value["state_commit"], "full");
    assert_eq!(
        value["map_state"]["open_node_ids"],
        serde_json::json!(["implement", "verify"])
    );
    assert_eq!(value["steps"][0]["finished_node_id"], "inspect");
    assert_eq!(value["steps"][0]["result_id"], "result-1");
    assert_eq!(value["steps"][0]["next"]["node_id"], "implement");
    assert_eq!(value["steps"][0]["current_node_id"], "implement");
    assert_eq!(state_identity_coverage(&output), Some((1, true)));
}

#[test]
fn terminal_chain_output_preserves_every_committed_identity() {
    let steps = format_terminal_chain_steps(vec![
        (
            "implement".into(),
            ActionMapFinishNodeOutcome {
                result_id: "result-1".into(),
                next_node_id: Some("verify".into()),
            },
        ),
        (
            "verify".into(),
            ActionMapFinishNodeOutcome {
                result_id: "result-2".into(),
                next_node_id: None,
            },
        ),
    ])
    .expect("format terminal chain");
    let mut state = control_state();
    state.task_status = "completed".into();
    state.map_status = "completed".into();
    state.current_node_id = None;
    state.open_node_ids.clear();
    state.completed_node_count = 3;
    let output = format_state_batch(steps, true, StateCommit::Full, Some(&state));
    let value: JsonValue = serde_json::from_str(&output).expect("terminal chain json");

    assert_eq!(value["steps"][0]["finished_node_id"], "implement");
    assert_eq!(value["steps"][0]["next"]["node_id"], "verify");
    assert_eq!(value["steps"][0]["current_node_id"], "verify");
    assert_eq!(value["steps"][1]["finished_node_id"], "verify");
    assert_eq!(value["steps"][1]["current_node_id"], JsonValue::Null);
    assert_eq!(state_identity_coverage(&output), Some((2, true)));
}

#[test]
fn failed_state_batch_preserves_protocol_and_raw_error() {
    let error = FunctionCallError::RespondToModel("exact transition error".into());
    let output = format_state_batch(
        vec![format_failed_state_step(0, &error)],
        false,
        StateCommit::None,
        Some(&control_state()),
    );
    let value: JsonValue = serde_json::from_str(&output).expect("failure batch json");

    assert_eq!(value["schema_version"], "TaskSpaceControlResultV3");
    assert_eq!(value["status"], "state_machine_failed");
    assert_eq!(value["success"], false);
    assert_eq!(value["state_commit"], "none");
    assert_eq!(value["map_state"]["current_node_id"], "implement");
    assert_eq!(
        value["steps"][0]["error"]["message"],
        "exact transition error"
    );
}

#[test]
fn partial_state_batch_reports_committed_prefix() {
    let steps = vec![
        serde_json::json!({
            "kind": "state_transition",
            "finished_node_id": "inspect",
            "result_id": "result-1",
            "next": {"kind": "existing", "node_id": "implement"},
            "current_node_id": "implement",
        }),
        serde_json::json!({
            "kind": "state_transition",
            "success": false,
            "error": {"code": "rejected"},
        }),
    ];
    assert_eq!(state_commit_for_steps(&steps, false), StateCommit::Partial);
    let output = format_state_batch(steps, false, StateCommit::Partial, Some(&control_state()));
    assert_eq!(
        control_state_observation(&output),
        Some(("partial".to_string(), 2, 0, true))
    );
}

#[test]
fn parses_agent_authored_terminal_candidate() {
    let args = parse_taskspace_control_args(
        &serde_json::json!({
            "action": "finish_then_end",
            "finish_node_ids": ["final"],
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
    let error = parse_taskspace_control_args(r#"{"action":"finish_nodes","finishes":[{}]}"#)
        .expect_err("missing next binding");
    assert!(error.to_string().contains("missing field `next`"));
}

#[test]
fn terminal_finish_chain_rejects_empty_and_duplicate_node_ids() {
    let empty = parse_taskspace_control_args(
        r#"{"action":"finish_then_end","finish_node_ids":[],"final_candidate":"answer"}"#,
    )
    .expect_err("empty terminal chain");
    assert!(empty.to_string().contains("must contain at least one item"));

    let duplicate = parse_taskspace_control_args(
        r#"{"action":"finish_then_end","finish_node_ids":["verify","verify"],"final_candidate":"answer"}"#,
    )
    .expect_err("duplicate terminal chain");
    assert!(duplicate.to_string().contains("unique finish_node_ids"));
}

#[test]
fn hard_state_reason_is_mechanical() {
    assert_eq!(
        hard_state_reason("blocked. hard_state: node_tool_calls_in_flight. rejected"),
        Some("node_tool_calls_in_flight")
    );
}

#[test]
fn gate_error_has_one_typed_representation() {
    let error =
        state_machine_error("blocked. hard_state: node_tool_calls_in_flight. rejected".to_string());
    let value: JsonValue = serde_json::from_str(&error.to_string()).expect("typed error json");

    assert_eq!(value["schema_version"], "TaskSpaceControlResultV3");
    assert_eq!(value["status"], "state_machine_failed");
    assert_eq!(value["error"]["code"], "node_tool_calls_in_flight");
    assert_eq!(
        value["error"]["message"],
        "blocked. hard_state: node_tool_calls_in_flight. rejected"
    );
    assert!(!error.to_string().contains("TaskSpaceGateRecoveryV1"));
}
