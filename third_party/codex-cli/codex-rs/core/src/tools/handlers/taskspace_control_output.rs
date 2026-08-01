use serde_json::Value as JsonValue;

use crate::action_map::ActionMapControlDelta;
use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

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
    let code = violations
        .as_array()
        .and_then(|violations| violations.first())
        .and_then(|violation| violation.get("code"))
        .and_then(JsonValue::as_str)
        .unwrap_or("taskspace_state_rejected");
    let condition = parsed
        .pointer("/error/message")
        .cloned()
        .unwrap_or_else(|| parsed.clone());
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
            "message": "canonical TaskSpace state rejected the submitted action",
            "actual": {
                "canonical_revision": canonical_revision,
                "violations": violations,
                "condition": condition,
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
            batch.facts.iter().enumerate().map(|(index, fact)| {
                serde_json::json!({
                    "revision": batch.revision,
                    "event_id": format!("{}:{}:{index}", batch.map_id, batch.revision),
                    "event_type": serde_json::to_value(fact)
                        .ok()
                        .and_then(|value| value.get("fact").cloned()),
                })
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
        "node_detail_event_refs": [],
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
    FunctionCallError::RespondToModel(format_failure_result(TaskSpaceFailureResult {
        action: None,
        status: "protocol_failed",
        class: "protocol",
        code: "TASKSPACE_PROTOCOL_FAILURE",
        message: &message,
        canonical_revision: None,
        submitted_expected_revision: None,
        actual: serde_json::json!({"condition": reason}),
        expected: JsonValue::Null,
    }))
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

pub(super) fn step_has_required_identity(step: &JsonValue) -> bool {
    match step.get("kind").and_then(JsonValue::as_str) {
        Some("finish_map") => {
            has_text(step, "map_id")
                && has_text(step, "finish_node_id")
                && step.get("revision").is_some()
                && step.get("finish_closed") == Some(&JsonValue::Bool(true))
                && step.get("root_closed") == Some(&JsonValue::Bool(true))
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
    fn finish_step_has_complete_identity() {
        let terminal = serde_json::json!({
            "kind": "finish_map",
            "finish_node_id": "finish",
            "map_id": "map-1",
            "revision": 4,
            "finish_closed": true,
            "root_closed": true,
        });

        assert!(step_has_required_identity(&terminal));
    }

    #[test]
    fn rejected_control_result_preserves_structured_runtime_error() {
        let source = serde_json::json!({
            "error": {
                "class": "protocol",
                "code": "revision_conflict",
                "message": "expected revision 3, actual revision 4",
                "actual": {"revision": 4},
                "expected": {"revision": 3},
            }
        });

        let result = rejected_control_result(&source.to_string());

        assert_eq!(
            serde_json::from_str::<JsonValue>(&result).expect("structured rejection"),
            source
        );
    }

    #[test]
    fn rejected_control_result_wraps_unstructured_runtime_error_without_reinterpreting_it() {
        let result = rejected_control_result("storage unavailable");
        let value = serde_json::from_str::<JsonValue>(&result).expect("fallback rejection");

        assert_eq!(
            value,
            serde_json::json!({"error": {"message": "storage unavailable"}})
        );
    }
}
