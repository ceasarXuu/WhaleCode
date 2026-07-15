use super::*;
use crate::action_map::ActionMapControlState;
use crate::action_map::ActionMapInitializeOutcome;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;
use crate::tools::handlers::taskspace_control_output::*;

fn control_state() -> ActionMapControlState {
    ActionMapControlState {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        revision: 3,
        root_node_id: "root".into(),
        finish_node_id: "finish".into(),
        complete: false,
        current_node_id: Some("work".into()),
        pending_work_node_ids: Vec::new(),
        ready_work_node_ids: Vec::new(),
        running_work_node_ids: vec!["work".into()],
        blocked_work_node_ids: Vec::new(),
        finish_ready: false,
        completed_work_node_count: 0,
        total_node_count: 3,
    }
}

#[test]
fn initialize_output_exposes_rooted_map_identity() {
    let step = format_initialize_step(&ActionMapInitializeOutcome {
        task_id: "task-1".into(),
        map_id: "map-1".into(),
        node_ids: vec!["root".into(), "work".into(), "finish".into()],
        current_node_id: "work".into(),
    });
    let output = format_state_batch(vec![step], true, true, Some(&control_state()));
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(
        value["schema_version"],
        TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION
    );
    assert_eq!(value["state_commit"], true);
    assert_eq!(value["map_state"]["root_node_id"], "root");
    assert_eq!(value["map_state"]["finish_node_id"], "finish");
    assert_eq!(value["map_state"]["running_work_node_ids"][0], "work");
    assert_eq!(value["map_state"]["finish_ready"], false);
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
        Some(&control_state()),
    );
    let value: JsonValue = serde_json::from_str(&output).unwrap();

    assert_eq!(value["status"], "state_machine_failed");
    assert_eq!(value["state_commit"], false);
    assert_eq!(value["partial_commit"], 0);
    assert_eq!(
        control_state_observation(&output),
        Some((false, 1, 0, true))
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

    let output = rejected_control_result(&error, None);
    let value: JsonValue = serde_json::from_str(&output).expect("one typed JSON result");
    assert_eq!(value["current_revision"], 0);
    assert_eq!(value["partial_commit"], 0);
    assert_eq!(value["violations"][0]["subjects"][0], "work");
    assert!(value["map_state"].is_null());
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
