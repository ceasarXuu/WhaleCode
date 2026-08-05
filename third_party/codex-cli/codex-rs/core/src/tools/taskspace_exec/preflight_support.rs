use serde_json::Value;

use super::plan::TaskspaceExecCall;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TaskspaceExecPreflightError {
    pub(crate) reason_code: &'static str,
    pub(crate) item_index: Option<usize>,
    pub(crate) item_id: Option<String>,
    pub(crate) message: String,
}

pub(super) fn validate_map_call(
    call: &TaskspaceExecCall,
    index: usize,
    action: &str,
) -> Result<(), TaskspaceExecPreflightError> {
    if call.node_id.is_some() {
        return Err(call_error(
            "map_call_node_binding_forbidden",
            index,
            call,
            "Map-level taskspace_control calls must not declare node_id",
        ));
    }
    let expected_revision_required = matches!(action, "execute" | "reopen_map" | "finish_map");
    if expected_revision_required
        && call
            .input
            .get("expected_revision")
            .and_then(Value::as_u64)
            .is_none()
    {
        return Err(call_error(
            "map_revision_missing",
            index,
            call,
            format!("taskspace_control `{action}` requires an unsigned expected_revision"),
        ));
    }
    Ok(())
}

pub(super) fn map_action<'a>(
    call: &'a TaskspaceExecCall,
    index: usize,
) -> Result<&'a str, TaskspaceExecPreflightError> {
    let action = call.input.get("action").and_then(Value::as_str);
    match action {
        Some(
            action @ ("initialize_and_execute"
            | "execute"
            | "reopen_map"
            | "read_map"
            | "read_output_ref"
            | "finish_map"),
        ) => Ok(action),
        _ => Err(call_error(
            "map_action_invalid",
            index,
            call,
            "taskspace_control input requires a recognized action",
        )),
    }
}

pub(super) fn call_error(
    reason_code: &'static str,
    index: usize,
    call: &TaskspaceExecCall,
    message: impl Into<String>,
) -> TaskspaceExecPreflightError {
    TaskspaceExecPreflightError {
        reason_code,
        item_index: Some(index),
        item_id: (!call.item_id.is_empty()).then(|| call.item_id.clone()),
        message: message.into(),
    }
}

pub(super) fn plan_error(
    reason_code: &'static str,
    message: impl Into<String>,
) -> TaskspaceExecPreflightError {
    TaskspaceExecPreflightError {
        reason_code,
        item_index: None,
        item_id: None,
        message: message.into(),
    }
}
