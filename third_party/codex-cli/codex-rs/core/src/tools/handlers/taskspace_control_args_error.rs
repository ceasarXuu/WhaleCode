use serde_json::Value as JsonValue;

use crate::function_tool::FunctionCallError;

use super::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

pub(super) fn invalid_argument_error(message: String) -> FunctionCallError {
    FunctionCallError::RespondToModel(
        serde_json::json!({
            "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
            "action": JsonValue::Null,
            "status": "argument_failed",
            "success": false,
            "state_commit": false,
            "partial_commit": false,
            "canonical_revision": JsonValue::Null,
            "submitted_expected_revision": JsonValue::Null,
            "committed_revision": JsonValue::Null,
            "delta": JsonValue::Null,
            "steps": [],
            "read": JsonValue::Null,
            "error": {
                "class": "argument",
                "code": "TASKSPACE_INVALID_ARGUMENT",
                "message": message,
                "actual": JsonValue::Null,
                "expected": {"contract": "selected action schema"},
            },
        })
        .to_string(),
    )
}

pub(super) fn normalize_invalid_arguments(
    arguments: &str,
    error: FunctionCallError,
) -> FunctionCallError {
    let message = match &error {
        FunctionCallError::RespondToModel(payload) => serde_json::from_str::<JsonValue>(payload)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(JsonValue::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| payload.clone()),
        _ => return error,
    };
    let submitted = serde_json::from_str::<JsonValue>(arguments).ok();
    let action = submitted
        .as_ref()
        .and_then(|value| value.get("action"))
        .and_then(JsonValue::as_str)
        .filter(|action| supported_action(action));
    let submitted_expected_revision = submitted
        .as_ref()
        .and_then(|value| value.get("expected_revision"))
        .and_then(JsonValue::as_u64);
    FunctionCallError::RespondToModel(
        serde_json::json!({
            "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
            "action": action,
            "status": "argument_failed",
            "success": false,
            "state_commit": false,
            "partial_commit": false,
            "canonical_revision": JsonValue::Null,
            "submitted_expected_revision": submitted_expected_revision,
            "committed_revision": JsonValue::Null,
            "delta": JsonValue::Null,
            "steps": [],
            "read": JsonValue::Null,
            "error": {
                "class": "argument",
                "code": "TASKSPACE_INVALID_ARGUMENT",
                "message": message,
                "actual": submitted,
                "expected": {"contract": "selected action schema"},
            },
        })
        .to_string(),
    )
}

pub(crate) fn with_argument_error_canonical_revision(
    error: FunctionCallError,
    canonical_revision: Option<u64>,
) -> FunctionCallError {
    let FunctionCallError::RespondToModel(payload) = error else {
        return error;
    };
    let Ok(mut value) = serde_json::from_str::<JsonValue>(&payload) else {
        return FunctionCallError::RespondToModel(payload);
    };
    if value.get("schema_version").and_then(JsonValue::as_str)
        != Some(TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION)
    {
        return FunctionCallError::RespondToModel(payload);
    }
    let canonical_value = serde_json::to_value(canonical_revision).unwrap_or(JsonValue::Null);
    value["canonical_revision"] = canonical_value.clone();
    if let Some(actual) = value.pointer_mut("/error/actual")
        && let Some(actual) = actual.as_object_mut()
    {
        actual.insert("canonical_revision".into(), canonical_value);
    }
    FunctionCallError::RespondToModel(value.to_string())
}

fn supported_action(action: &str) -> bool {
    matches!(
        action,
        "initialize_map"
            | "mutate_graph"
            | "bind_node"
            | "block_node"
            | "unblock_node"
            | "rework_node"
            | "complete_then_continue"
            | "complete_active_work_then_end"
            | "close_finish_with_no_active_work"
            | "expand_nodes"
            | "read_map"
            | "read_output_ref"
    )
}
