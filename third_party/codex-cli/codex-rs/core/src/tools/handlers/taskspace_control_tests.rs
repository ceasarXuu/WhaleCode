use super::*;
use crate::action_map::ActionMapControlDelta;
use crate::action_map::ActionMapInitializeOutcome;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;
use crate::tools::handlers::taskspace_control_output::*;
use codex_protocol::protocol::MapRuntimeGraphRevisionCommittedEvent;

fn control_delta() -> ActionMapControlDelta {
    ActionMapControlDelta {
        map_id: "map-1".into(),
        committed_revision: 3,
        graph_revision_batches: Vec::new(),
        node_detail_events: Vec::new(),
    }
}

#[test]
fn initialize_output_exposes_rooted_map_identity() {
    let step = format_initialize_step(&ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec!["root".into(), "work".into(), "finish".into()],
        current_node_id: "work".into(),
        delta: control_delta(),
    });
    let delta = control_delta();
    let output = format_state_batch(vec![step], true, true, &[&delta]);
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(
        value["schema_version"],
        TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION
    );
    assert_eq!(value["state_commit"], true);
    assert!(value.get("map_state").is_none());
    assert_eq!(value["committed_revision"], 3);
    assert_eq!(value["delta"]["map_id"], "map-1");
    assert_eq!(value["steps"][0]["revision"], 3);
    assert!(value["steps"][0].get("created_node_ids").is_none());
    assert_eq!(state_identity_coverage(&output), Some((1, true)));
}

#[test]
fn rejected_output_cannot_report_partial_commit() {
    let output = format_state_batch(
        vec![serde_json::json!({
            "kind": "state_rejection",
            "error": {"violations": [{"code": "node_unreachable"}]},
        })],
        false,
        false,
        &[],
    );
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(value["status"], "state_machine_failed");
    assert_eq!(value["state_commit"], false);
    assert_eq!(value["partial_commit"], 0);
    assert_eq!(
        control_commit_observation(&output),
        Some((false, None, 0, 0))
    );
}

#[test]
fn committed_delta_references_canonical_events_without_copying_event_payloads() {
    let delta = ActionMapControlDelta {
        map_id: "018f4b68-4f8d-7f1f-9d11-4f5d915efe61".into(),
        committed_revision: 2,
        graph_revision_batches: vec![MapRuntimeGraphRevisionCommittedEvent {
            map_id: "018f4b68-4f8d-7f1f-9d11-4f5d915efe61".into(),
            revision: 2,
            operation: "initialize_map".into(),
            event_ids: vec!["018f4b68-4f8d-7f1f-9d11-4f5d915efe62".into()],
            events: vec![serde_json::json!({
                "type": "map_initialized",
                "map": {"root_node_id": "must-not-be-copied"},
            })],
        }],
        node_detail_events: Vec::new(),
    };
    let output = format_state_batch(Vec::new(), true, true, &[&delta]);
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(
        value["delta"]["graph_event_refs"][0]["event_id"],
        "018f4b68-4f8d-7f1f-9d11-4f5d915efe62"
    );
    assert_eq!(
        value["delta"]["graph_event_refs"][0]["event_type"],
        "map_initialized"
    );
    assert!(value["delta"].get("graph_revision_batches").is_none());
    assert!(value["delta"].get("node_detail_events").is_none());
    assert!(!output.contains("must-not-be-copied"));
    assert_eq!(
        control_commit_observation(&output),
        Some((true, Some(2), 1, 0))
    );
    assert!(
        output.len() <= 712,
        "compact initialization feedback must stay at least 30% below the 1,018-byte E6 fixture, got {} bytes",
        output.len()
    );
}

#[test]
fn rooted_rejection_is_exposed_once_without_string_wrapping() {
    let error = serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": "state_machine_failed",
        "success": false,
        "state_commit": false,
        "current_revision": 0,
        "violations": [{"code": "transition_invalid", "subjects": ["work"]}],
    })
    .to_string();

    let output = rejected_control_result(&error);
    let value: JsonValue = serde_json::from_str(&output).expect("one typed JSON result");
    assert_eq!(value["current_revision"], 0);
    assert_eq!(value["partial_commit"], 0);
    assert_eq!(value["violations"][0]["subjects"][0], "work");
    assert!(value.get("map_state").is_none());
    assert!(value["committed_revision"].is_null());
    assert!(value["delta"].is_null());
    assert!(value.get("steps").is_none());
    assert!(value.get("error").is_none());
}

#[test]
fn graph_and_terminal_steps_have_required_identity() {
    for step in [
        serde_json::json!({
            "kind": "graph_mutation",
            "map_id": "map-1",
            "revision": 4,
        }),
        serde_json::json!({
            "kind": "node_transition",
            "map_id": "map-1",
            "node_id": "work",
            "revision": 4,
            "status": "completed",
        }),
        serde_json::json!({
            "kind": "terminal_transition",
            "map_id": "map-1",
            "revision": 5,
            "finish_closed": true,
            "root_closed": true,
        }),
    ] {
        assert!(step_has_required_identity(&step));
    }
}

#[test]
fn legacy_actions_are_rejected_by_parser() {
    for action in ["create_node", "finish_nodes", "finish_then_end"] {
        let args = format!(r#"{{"action":"{action}"}}"#);
        assert!(parse_taskspace_control_args(&args).is_err());
    }
}

#[test]
fn terminal_output_preserves_committed_map_revision_and_summary() {
    let carrier = TaskSpaceTerminalCarrier {
        map_id: "map-1".into(),
        revision: 5,
        summary: "Agent-authored final summary".into(),
    };
    let output = TaskSpaceControlOutput {
        message: "committed".into(),
        success: true,
        terminal_carrier: Some(carrier.clone()),
    };

    assert_eq!(output.taskspace_terminal_carrier(), Some(&carrier));
}
