use super::*;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::Violation;

fn prepared_response() -> ActionMapPreparedResponse {
    ActionMapPreparedResponse {
        map_id: "map-1".to_string(),
        revision_before: 3,
        revision_after: 4,
        action: "execute",
        prepared_calls: vec![ActionMapPreparedCall {
            map_id: "map-1".to_string(),
            revision: 4,
            call_id: "call-1".to_string(),
            call_index: 0,
            node_id: "work-1".to_string(),
            tool_name: "exec_command".to_string(),
            reservation_id: "reservation-1".to_string(),
        }],
    }
}

#[test]
fn finalized_result_exposes_one_continuation_revision() {
    let prepared = prepared_response();
    let receipt = ActionMapResponseFinalReceipt {
        map_id: prepared.map_id.clone(),
        control_call_id: "control".to_string(),
        reservation_revision_after: prepared.revision_after,
        canonical_revision: Some(5),
        prepared_action_count: 1,
        attributed_result_count: 1,
        outstanding_reservation_count: 0,
        error: None,
    };

    let value: serde_json::Value =
        serde_json::from_str(&receipt.finalized_model_visible_result(&prepared)).unwrap();

    assert_eq!(value["schema_version"], "TaskSpaceResponseResultV2");
    assert_eq!(value["status"], "settled");
    assert_eq!(value["success"], true);
    assert_eq!(value["state_commit"], true);
    assert_eq!(value["action"], "execute");
    assert_eq!(value["canonical_revision"], 5);
    assert_eq!(value["reserved_actions"][0]["call_id"], "call-1");
    assert_eq!(value["settlement"]["prepared_action_count"], 1);
    assert!(value.get("revision_before").is_none());
    assert!(value.get("revision_after").is_none());
    assert!(value.get("reservation_revision_after").is_none());
    assert!(value.get("control_call_id").is_none());
    assert!(value.get("receipt_only").is_none());
    assert!(value.get("error").is_none());
}

#[test]
fn finalized_result_reports_incomplete_attribution_without_hiding_commit() {
    let prepared = prepared_response();
    let receipt = ActionMapResponseFinalReceipt {
        map_id: prepared.map_id.clone(),
        control_call_id: "control".to_string(),
        reservation_revision_after: prepared.revision_after,
        canonical_revision: Some(5),
        prepared_action_count: 1,
        attributed_result_count: 0,
        outstanding_reservation_count: 1,
        error: None,
    };

    let value: serde_json::Value =
        serde_json::from_str(&receipt.finalized_model_visible_result(&prepared)).unwrap();

    assert_eq!(value["status"], "settlement_incomplete");
    assert_eq!(value["success"], false);
    assert_eq!(value["state_commit"], true);
    assert_eq!(value["canonical_revision"], 5);
    assert_eq!(
        value["error"]["code"],
        "taskspace_response_attribution_incomplete"
    );
}

#[test]
fn finalized_result_reports_unavailable_store_without_guessing_revision() {
    let prepared = prepared_response();
    let receipt = ActionMapResponseFinalReceipt::unavailable(
        &prepared,
        "control",
        "store unavailable".to_string(),
    );

    let value: serde_json::Value =
        serde_json::from_str(&receipt.finalized_model_visible_result(&prepared)).unwrap();

    assert_eq!(value["status"], "settlement_incomplete");
    assert_eq!(value["success"], false);
    assert_eq!(value["state_commit"], true);
    assert!(value["canonical_revision"].is_null());
    assert_eq!(
        value["error"]["code"],
        "taskspace_response_final_state_unavailable"
    );
    assert_eq!(value["error"]["detail"], "store unavailable");
}

#[test]
fn state_rejection_exposes_node_facts_without_nested_json() {
    let error = ActionMapResponsePrepareError::state(Rejection {
        state_commit: false,
        current_revision: 7,
        violations: vec![Violation::node_state(
            "verify",
            Some(NodeState::Waiting),
            vec![NodeState::Ready, NodeState::InFlight],
            vec!["inspect".into(), "patch".into()],
        )],
    });

    let value: serde_json::Value = serde_json::from_str(&error.model_visible_failure(
        Some(7),
        serde_json::json!({
            "scope": "provider_response",
            "copy_group_id": "provider_response:control",
            "zero_dispatch": true,
        }),
    ))
    .unwrap();

    assert_eq!(value["schema_version"], "TaskSpaceResponseCommitFailureV3");
    assert_eq!(value["current_revision"], 7);
    assert_eq!(value["rejected_candidate_committed"], false);
    assert_eq!(
        value["error"]["code"],
        ACTION_MAP_RESPONSE_STATE_COMMIT_FAILED_CODE
    );
    assert_eq!(
        value["error"]["violations"][0]["code"],
        "node_state_invalid"
    );
    assert_eq!(value["error"]["violations"][0]["node_id"], "verify");
    assert_eq!(
        value["error"]["violations"][0]["rejected_candidate_at_violation"]["state"],
        "waiting"
    );
    assert_eq!(
        value["error"]["violations"][0]["rejected_candidate_at_violation"]["allowed_states"],
        serde_json::json!(["ready", "in_flight"])
    );
    assert_eq!(
        value["error"]["violations"][0]["rejected_candidate_at_violation"]["unsatisfied_predecessor_ids"],
        serde_json::json!(["inspect", "patch"])
    );
    assert_eq!(
        value["error"]["violations"][0]["rejected_candidate_at_violation"]["committed"],
        false
    );
    assert!(
        value["error"]["violations"][0]
            .get("evaluated_state_at_violation")
            .is_none()
    );
    assert_eq!(
        value["failure_provenance"]["copy_group_id"],
        "provider_response:control"
    );
    assert!(value["error"].get("detail").is_none());
}

#[test]
fn state_rejection_serializes_absent_canonical_node_explicitly() {
    let mut violation = Violation::node_state(
        "missing",
        Some(NodeState::Completed),
        vec![NodeState::Ready],
        Vec::new(),
    );
    violation.canonical_node_present_before_transaction = Some(false);
    let error = ActionMapResponsePrepareError::state(Rejection {
        state_commit: false,
        current_revision: 7,
        violations: vec![violation],
    });

    let value: serde_json::Value = serde_json::from_str(&error.model_visible_failure(
        Some(7),
        serde_json::json!({
            "scope": "provider_response",
            "copy_group_id": "provider_response:control",
            "zero_dispatch": true,
            "affected_call_ids": ["control"],
        }),
    ))
    .unwrap();

    let canonical = &value["error"]["violations"][0]["canonical_before_transaction"];
    assert_eq!(canonical["node_present"], false);
    assert!(canonical["state"].is_null());
    assert_eq!(
        value["error"]["violations"][0]["rejected_candidate_at_violation"]["state"],
        "completed"
    );
}
