use std::sync::Arc;

use serde_json::json;

use crate::session::session::Session;
use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;

pub(crate) const ACTIVE_BINDING: &str = "active";
pub(crate) const AFTER_BOUNDARY_BINDING: &str = "after_boundary";
pub(crate) const INITIALIZE_MAP_BINDING: &str = "initialize_map";

pub(crate) fn taskspace_binding_kind(binding: &str) -> Option<&str> {
    if matches!(binding, ACTIVE_BINDING | AFTER_BOUNDARY_BINDING) {
        return Some(binding);
    }
    let value = serde_json::from_str::<serde_json::Value>(binding).ok()?;
    match value.get("action").and_then(serde_json::Value::as_str) {
        Some(INITIALIZE_MAP_BINDING) => Some(INITIALIZE_MAP_BINDING),
        _ => None,
    }
}

pub(crate) fn is_initialization_binding(binding: Option<&str>) -> bool {
    binding.and_then(taskspace_binding_kind) == Some(INITIALIZE_MAP_BINDING)
}

pub(crate) async fn validate_taskspace_binding(
    session: &Arc<Session>,
    call: &ToolCall,
) -> Result<(), String> {
    if is_taskspace_control(call) {
        return match call.taskspace_binding.as_deref() {
            None => Ok(()),
            Some(binding) => Err(binding_failure(
                "TASKSPACE_BINDING_FORBIDDEN",
                "taskspace_control cannot carry taskspace_binding",
                Some(binding),
            )),
        };
    }

    if !session.taskspace_active().await {
        return match call.taskspace_binding.as_deref() {
            None => Ok(()),
            Some(binding) => Err(binding_failure(
                "TASKSPACE_BINDING_MODE_MISMATCH",
                "taskspace_binding is only available in TaskSpace sessions",
                Some(binding),
            )),
        };
    }

    if matches!(
        call.payload,
        ToolPayload::Custom { .. } | ToolPayload::LocalShell { .. }
    ) {
        return Err(binding_failure(
            "TASKSPACE_TOOL_SHAPE_UNSUPPORTED",
            "TaskSpace cannot sequence this provider tool payload shape",
            call.taskspace_binding.as_deref(),
        ));
    }

    if !requires_taskspace_binding(call) {
        return Ok(());
    }

    match call
        .taskspace_binding
        .as_deref()
        .and_then(taskspace_binding_kind)
    {
        Some(ACTIVE_BINDING | AFTER_BOUNDARY_BINDING | INITIALIZE_MAP_BINDING) => Ok(()),
        Some(binding) => Err(binding_failure(
            "TASKSPACE_BINDING_INVALID",
            "taskspace_binding must be active, after_boundary, or a valid initialize_map object",
            Some(binding),
        )),
        None if call.taskspace_binding.is_some() => Err(binding_failure(
            "TASKSPACE_BINDING_INVALID",
            "taskspace_binding must be active, after_boundary, or a valid initialize_map object",
            call.taskspace_binding.as_deref(),
        )),
        None => Err(binding_failure(
            "TASKSPACE_BINDING_REQUIRED",
            "TaskSpace ordinary Tool calls must declare taskspace_binding",
            None,
        )),
    }
}

fn is_taskspace_control(call: &ToolCall) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control"
}

fn requires_taskspace_binding(call: &ToolCall) -> bool {
    !is_taskspace_control(call)
        && matches!(
            call.payload,
            ToolPayload::Function { .. } | ToolPayload::ToolSearch { .. } | ToolPayload::Mcp { .. }
        )
}

fn binding_failure(code: &str, message: &str, submitted_binding: Option<&str>) -> String {
    json!({
        "schema_version": "TaskSpaceBindingValidationResultV1",
        "status": "protocol_failed",
        "success": false,
        "state_commit": false,
        "submitted_binding": submitted_binding,
        "error": {
            "class": "protocol",
            "code": code,
            "message": message,
        }
    })
    .to_string()
}

#[cfg(test)]
#[path = "taskspace_binding_tests.rs"]
mod tests;
