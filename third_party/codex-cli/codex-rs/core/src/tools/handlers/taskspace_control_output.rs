use serde_json::Value as JsonValue;

use crate::action_map::ActionMapControlDelta;
#[cfg(test)]
use crate::action_map::ActionMapExpandedDetailRef;
use crate::action_map::ActionMapInitializeOutcome;
use crate::action_map::ActionMapNodeDetailExpansionOutcome;
use crate::action_map::TaskSpaceHardGateClass;
use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

pub(super) fn format_initialize_step(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "kind": "map_initialized",
        "map_id": outcome.map_id,
        "revision": outcome.delta.committed_revision,
    })
}

pub(super) fn format_initialize_binding_step(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "kind": "node_bound",
        "map_id": outcome.map_id,
        "node_id": outcome.current_node_id,
        "status": "running",
        "revision": outcome.delta.committed_revision,
    })
}

pub(super) fn format_node_detail_expansion_step(
    outcome: &ActionMapNodeDetailExpansionOutcome,
) -> JsonValue {
    let restored_details = outcome
        .restored_details
        .iter()
        .map(|detail| {
            serde_json::json!({
                "event_id": detail.event_id,
                "event_kind": detail.event_kind,
                "source": detail.source,
                "detail_tier": detail.detail_tier,
                "evidence_class": detail.evidence_class,
                "action_class": detail.action_class,
                "tool_success": detail.tool_success,
                "content_sha256": detail.content_sha256,
                "raw_ref": detail.raw_ref,
                "artifact_refs": detail.artifact_refs,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "kind": "node_detail_expanded",
        "node_id": outcome.node_id,
        "expansion_event_id": outcome.expansion_event_id,
        "detail_ref": outcome.detail_ref,
        "restored_detail_count": restored_details.len(),
        "restored_details": restored_details,
    })
}

pub(super) fn format_state_batch(
    steps: Vec<JsonValue>,
    success: bool,
    state_commit: bool,
    deltas: &[&ActionMapControlDelta],
) -> String {
    let delta = format_committed_delta(deltas);
    let committed_revision = delta
        .as_ref()
        .and_then(|value| value.get("committed_revision"))
        .cloned()
        .unwrap_or(JsonValue::Null);
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": if success { "committed" } else { "state_machine_failed" },
        "success": success,
        "state_commit": state_commit,
        "partial_commit": false,
        "committed_revision": committed_revision,
        "delta": delta,
        "steps": steps,
    })
    .to_string()
}

pub(super) fn normalize_control_result(
    message: String,
    action: &str,
    submitted_expected_revision: Option<u64>,
    canonical_revision: Option<u64>,
    success: bool,
) -> String {
    let parsed = serde_json::from_str::<JsonValue>(&message).unwrap_or_else(|_| {
        serde_json::json!({
            "error": {
                "message": message,
            }
        })
    });
    if parsed.get("schema_version").and_then(JsonValue::as_str)
        == Some(TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION)
        && parsed.get("action").is_some()
    {
        return parsed.to_string();
    }
    if success {
        let committed_revision = parsed.get("committed_revision").and_then(JsonValue::as_u64);
        return serde_json::json!({
            "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
            "action": action,
            "status": "committed",
            "success": true,
            "state_commit": true,
            "partial_commit": false,
            "canonical_revision": canonical_revision.or(committed_revision),
            "submitted_expected_revision": submitted_expected_revision,
            "committed_revision": committed_revision,
            "delta": parsed.get("delta").cloned().unwrap_or(JsonValue::Null),
            "steps": parsed.get("steps").cloned().unwrap_or_else(|| serde_json::json!([])),
            "read": JsonValue::Null,
            "error": JsonValue::Null,
        })
        .to_string();
    }

    let canonical_revision =
        canonical_revision.or_else(|| parsed.get("current_revision").and_then(JsonValue::as_u64));
    let violations = parsed
        .get("violations")
        .cloned()
        .or_else(|| parsed.pointer("/error/violations").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let stale_revision = violations.as_array().is_some_and(|violations| {
        violations.iter().any(|violation| {
            violation.get("code").and_then(JsonValue::as_str) == Some("stale_revision")
        })
    });
    let graph_action = matches!(action, "initialize_map" | "mutate_graph");
    let (code, message) = if stale_revision {
        (
            "TASKSPACE_STALE_REVISION",
            "expected_revision does not match the current canonical revision",
        )
    } else if graph_action {
        (
            "TASKSPACE_GRAPH_INVARIANT",
            "the submitted mutation violates a rooted DAG invariant",
        )
    } else {
        (
            "TASKSPACE_LIFECYCLE_INVARIANT",
            "the submitted transition is not valid from the observed lifecycle state",
        )
    };
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "action": action,
        "status": "state_machine_failed",
        "success": false,
        "state_commit": false,
        "partial_commit": false,
        "canonical_revision": canonical_revision,
        "submitted_expected_revision": submitted_expected_revision,
        "committed_revision": JsonValue::Null,
        "delta": JsonValue::Null,
        "steps": [],
        "read": JsonValue::Null,
        "error": {
            "class": "state_machine",
            "code": code,
            "message": message,
            "actual": {
                "canonical_revision": canonical_revision,
                "violations": violations,
                "condition": parsed.pointer("/error/message").cloned().unwrap_or(JsonValue::Null),
            },
            "expected": {
                "action": action,
                "submitted_expected_revision": submitted_expected_revision,
            },
        },
    })
    .to_string()
}

pub(super) fn format_read_result(
    action: &str,
    canonical_revision: Option<u64>,
    read: JsonValue,
) -> String {
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "action": action,
        "status": "read_ok",
        "success": true,
        "state_commit": false,
        "partial_commit": false,
        "canonical_revision": canonical_revision,
        "submitted_expected_revision": JsonValue::Null,
        "committed_revision": JsonValue::Null,
        "delta": JsonValue::Null,
        "steps": [],
        "read": read,
        "error": JsonValue::Null,
    })
    .to_string()
}

pub(crate) struct TaskSpaceFailureResult<'a> {
    pub(crate) action: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) class: &'a str,
    pub(crate) code: &'a str,
    pub(crate) message: &'a str,
    pub(crate) canonical_revision: Option<u64>,
    pub(crate) submitted_expected_revision: Option<u64>,
    pub(crate) actual: JsonValue,
    pub(crate) expected: JsonValue,
}

pub(crate) fn format_failure_result(result: TaskSpaceFailureResult<'_>) -> String {
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "action": result.action,
        "status": result.status,
        "success": false,
        "state_commit": false,
        "partial_commit": false,
        "canonical_revision": result.canonical_revision,
        "submitted_expected_revision": result.submitted_expected_revision,
        "committed_revision": JsonValue::Null,
        "delta": JsonValue::Null,
        "steps": [],
        "read": JsonValue::Null,
        "error": {
            "class": result.class,
            "code": result.code,
            "message": result.message,
            "actual": result.actual,
            "expected": result.expected,
        },
    })
    .to_string()
}

pub(super) fn rejected_control_result(error: &str) -> String {
    serde_json::from_str::<JsonValue>(error).map_or_else(
        |_| serde_json::json!({"error": {"message": error}}).to_string(),
        |exact_error| exact_error.to_string(),
    )
}

pub(super) fn control_commit_observation(
    message: &str,
) -> Option<(bool, Option<u64>, usize, usize)> {
    let value = serde_json::from_str::<JsonValue>(message).ok()?;
    let state_commit = value.get("state_commit")?.as_bool()?;
    let committed_revision = value.get("committed_revision").and_then(JsonValue::as_u64);
    let delta = value.get("delta").and_then(JsonValue::as_object);
    Some((
        state_commit,
        committed_revision,
        delta
            .and_then(|delta| delta.get("graph_event_refs"))
            .and_then(JsonValue::as_array)
            .map_or(0, Vec::len),
        delta
            .and_then(|delta| delta.get("node_detail_event_refs"))
            .and_then(JsonValue::as_array)
            .map_or(0, Vec::len),
    ))
}

fn format_committed_delta(deltas: &[&ActionMapControlDelta]) -> Option<JsonValue> {
    let first = deltas.first()?;
    let graph_event_refs = deltas
        .iter()
        .flat_map(|delta| delta.graph_revision_batches.iter())
        .flat_map(|batch| {
            batch
                .event_ids
                .iter()
                .zip(batch.events.iter())
                .map(|(event_id, event)| {
                    serde_json::json!({
                        "revision": batch.revision,
                        "event_id": event_id,
                        "event_type": event.get("type").and_then(JsonValue::as_str),
                    })
                })
        })
        .collect::<Vec<_>>();
    let node_detail_event_refs = deltas
        .iter()
        .flat_map(|delta| delta.node_detail_events.iter())
        .map(|event| {
            serde_json::json!({
                "node_id": &event.node_id,
                "expansion_event_id": &event.expansion_event_id,
            })
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        "map_id": &first.map_id,
        "committed_revision": deltas
            .iter()
            .map(|delta| delta.committed_revision)
            .max()
            .unwrap_or(first.committed_revision),
        "graph_event_refs": graph_event_refs,
        "node_detail_event_refs": node_detail_event_refs,
    }))
}

pub(super) fn state_identity_coverage(message: &str) -> Option<(usize, bool)> {
    let value = serde_json::from_str::<JsonValue>(message).ok()?;
    if value.get("schema_version")?.as_str()? != TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION {
        return None;
    }
    if value.get("status")?.as_str()? != "committed" {
        return None;
    }
    let steps = value.get("steps")?.as_array()?;
    Some((steps.len(), steps.iter().all(step_has_required_identity)))
}

pub(super) fn protocol_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Protocol, reason)
}

pub(super) fn action_failure_error(
    action: &str,
    submitted_expected_revision: Option<u64>,
    canonical_revision: Option<u64>,
    class: &str,
    code: &str,
    status: &str,
    message: String,
) -> FunctionCallError {
    FunctionCallError::RespondToModel(format_failure_result(TaskSpaceFailureResult {
        action: Some(action),
        status,
        class,
        code,
        message: &message,
        canonical_revision,
        submitted_expected_revision,
        actual: serde_json::json!({"condition": message.clone()}),
        expected: serde_json::json!({
            "action": action,
            "submitted_expected_revision": submitted_expected_revision,
        }),
    }))
}

fn gate_error(message: String, class: TaskSpaceHardGateClass, reason: &str) -> FunctionCallError {
    let (status, code) = match class {
        TaskSpaceHardGateClass::StateMachine => {
            ("state_machine_failed", "TASKSPACE_LIFECYCLE_INVARIANT")
        }
        TaskSpaceHardGateClass::Protocol => ("protocol_failed", "TASKSPACE_PROTOCOL_FAILURE"),
    };
    FunctionCallError::RespondToModel(format_failure_result(TaskSpaceFailureResult {
        action: None,
        status,
        class: class.as_str(),
        code,
        message: &message,
        canonical_revision: None,
        submitted_expected_revision: None,
        actual: serde_json::json!({"condition": reason}),
        expected: JsonValue::Null,
    }))
}

pub(super) fn step_has_required_identity(step: &JsonValue) -> bool {
    match step.get("kind").and_then(JsonValue::as_str) {
        Some("map_initialized") => has_text(step, "map_id") && step.get("revision").is_some(),
        Some("graph_mutation") => has_text(step, "map_id") && step.get("revision").is_some(),
        Some("node_bound" | "node_blocked" | "node_unblocked" | "node_reworked") => {
            has_text(step, "map_id")
                && has_text(step, "node_id")
                && has_text(step, "status")
                && step.get("revision").is_some()
        }
        Some("complete_then_continue") => {
            has_text(step, "map_id")
                && has_text(step, "current_node_id")
                && has_text(step, "next_node_id")
                && step.get("revision").is_some()
        }
        Some("finish_map") => {
            has_text(step, "map_id")
                && has_text(step, "terminal_node_id")
                && matches!(
                    step.get("terminal_node_role").and_then(JsonValue::as_str),
                    Some("work" | "finish")
                )
                && step.get("revision").is_some()
                && step.get("finish_closed") == Some(&JsonValue::Bool(true))
                && step.get("root_closed") == Some(&JsonValue::Bool(true))
        }
        Some("node_detail_expanded") => {
            has_text(step, "node_id")
                && has_text(step, "expansion_event_id")
                && has_text(step, "detail_ref")
                && step
                    .get("restored_details")
                    .and_then(JsonValue::as_array)
                    .is_some_and(|details| !details.is_empty())
        }
        _ => false,
    }
}

fn has_text(value: &JsonValue, field: &str) -> bool {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .is_some_and(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_step_returns_hidden_event_refs_without_duplicate_hash_field() {
        let outcome = ActionMapNodeDetailExpansionOutcome {
            node_id: "node-1".into(),
            expansion_event_id: "node-event-expand".into(),
            detail_ref: "taskspace-detail://sha256/abc".into(),
            restored_details: vec![ActionMapExpandedDetailRef {
                event_id: "node-event-read".into(),
                event_kind: "tool_result".into(),
                source: "main_tool".into(),
                detail_tier: "D3".into(),
                evidence_class: "P1".into(),
                action_class: Some("read".into()),
                tool_success: Some(true),
                content_sha256: Some("def".into()),
                raw_ref: Some("output-ref-1".into()),
                artifact_refs: vec!["src/lib.rs".into()],
            }],
            delta: ActionMapControlDelta {
                map_id: "map-1".into(),
                committed_revision: 3,
                graph_revision_batches: Vec::new(),
                node_detail_events: Vec::new(),
            },
        };

        let step = format_node_detail_expansion_step(&outcome);

        assert_eq!(step["restored_detail_count"], 1);
        assert_eq!(step["restored_details"][0]["event_id"], "node-event-read");
        assert_eq!(step["restored_details"][0]["raw_ref"], "output-ref-1");
        assert!(step.get("detail_sha256").is_none());
        assert!(step_has_required_identity(&step));
    }

    #[test]
    fn atomic_completion_steps_have_complete_identity() {
        let handoff = serde_json::json!({
            "kind": "complete_then_continue",
            "map_id": "map-1",
            "current_node_id": "inspect",
            "next_node_id": "implement",
            "revision": 3,
        });
        let terminal = serde_json::json!({
            "kind": "finish_map",
            "terminal_node_id": "verify",
            "terminal_node_role": "work",
            "map_id": "map-1",
            "revision": 4,
            "finish_closed": true,
            "root_closed": true,
        });

        assert!(step_has_required_identity(&handoff));
        assert!(step_has_required_identity(&terminal));
    }
}
