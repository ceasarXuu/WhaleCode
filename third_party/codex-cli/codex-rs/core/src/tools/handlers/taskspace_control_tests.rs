use super::*;
use crate::action_map::ActionMapControlDelta;
use crate::action_map::ActionMapControlState;
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

fn control_state(
    current_node_id: Option<&str>,
    running_work_node_ids: &[&str],
) -> ActionMapControlState {
    ActionMapControlState {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        revision: 3,
        root_node_id: "root".into(),
        finish_node_id: "finish".into(),
        complete: false,
        current_node_id: current_node_id.map(str::to_string),
        pending_work_node_ids: Vec::new(),
        ready_work_node_ids: Vec::new(),
        running_work_node_ids: running_work_node_ids
            .iter()
            .map(|node_id| (*node_id).to_string())
            .collect(),
        blocked_work_node_ids: Vec::new(),
        finish_ready: false,
        completed_work_node_count: 0,
        total_node_count: 3,
    }
}

#[test]
fn mutation_continuation_requires_the_current_running_binding() {
    assert!(!control_state_has_active_binding(None));
    assert!(!control_state_has_active_binding(Some(&control_state(
        None,
        &["work"]
    ))));
    assert!(!control_state_has_active_binding(Some(&control_state(
        Some("work"),
        &["other"]
    ))));
    assert!(control_state_has_active_binding(Some(&control_state(
        Some("work"),
        &["work"]
    ))));
}

#[test]
fn initialize_output_exposes_rooted_map_identity() {
    let outcome = ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec!["root".into(), "work".into(), "finish".into()],
        current_node_id: "work".into(),
        delta: control_delta(),
    };
    let delta = control_delta();
    let intermediate = format_state_batch(
        vec![
            format_initialize_step(&outcome),
            format_initialize_binding_step(&outcome),
        ],
        true,
        true,
        &[&delta],
    );
    let output = normalize_control_result(intermediate, "initialize_map", None, Some(3), true);
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
    assert_eq!(value["steps"][1]["kind"], "node_bound");
    assert_eq!(value["steps"][1]["node_id"], "work");
    assert!(value["steps"][0].get("created_node_ids").is_none());
    assert_eq!(state_identity_coverage(&output), Some((2, true)));
    assert_v2_envelope(&value);
}

#[test]
fn rejected_output_cannot_report_partial_commit() {
    let intermediate = format_state_batch(
        vec![serde_json::json!({
            "kind": "state_rejection",
            "error": {"violations": [{"code": "node_unreachable"}]},
        })],
        false,
        false,
        &[],
    );
    let output = normalize_control_result(intermediate, "mutate_graph", Some(3), Some(3), false);
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(value["status"], "state_machine_failed");
    assert_eq!(value["state_commit"], false);
    assert_eq!(value["partial_commit"], false);
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
    let output = normalize_control_result(
        format_state_batch(Vec::new(), true, true, &[&delta]),
        "mutate_graph",
        Some(1),
        Some(2),
        true,
    );
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
        output.len() <= 900,
        "typed control feedback grew beyond the compact V2 budget: {} bytes",
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

    let output = normalize_control_result(
        rejected_control_result(&error),
        "bind_node",
        Some(0),
        Some(0),
        false,
    );
    let value: JsonValue = serde_json::from_str(&output).expect("one typed JSON result");
    assert_eq!(value["canonical_revision"], 0);
    assert_eq!(value["partial_commit"], false);
    assert_eq!(
        value["error"]["actual"]["violations"][0]["subjects"][0],
        "work"
    );
    assert_eq!(value["error"]["code"], "TASKSPACE_LIFECYCLE_INVARIANT");
    assert!(value["committed_revision"].is_null());
    assert!(value["delta"].is_null());
    assert_eq!(value["steps"], serde_json::json!([]));
    assert!(value["error"].is_object());
    assert_v2_envelope(&value);
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
            "kind": "node_bound",
            "map_id": "map-1",
            "node_id": "work",
            "revision": 4,
            "status": "running",
        }),
        serde_json::json!({
            "kind": "close_finish_with_no_active_work",
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
    for action in [
        "create_node",
        "finish_nodes",
        "finish_then_end",
        "finish_end",
    ] {
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

fn assert_v2_envelope(value: &JsonValue) {
    for field in [
        "schema_version",
        "action",
        "status",
        "success",
        "state_commit",
        "partial_commit",
        "canonical_revision",
        "submitted_expected_revision",
        "committed_revision",
        "delta",
        "steps",
        "read",
        "error",
    ] {
        assert!(value.get(field).is_some(), "V2 result omits {field}");
    }
    assert_eq!(value["partial_commit"], false);
}
