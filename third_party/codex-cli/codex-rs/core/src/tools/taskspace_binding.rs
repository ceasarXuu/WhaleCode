use std::sync::Arc;

use serde_json::json;

use crate::session::session::Session;
use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;

pub(crate) const ACTIVE_BINDING: &str = "active";
pub(crate) const AFTER_BOUNDARY_BINDING: &str = "after_boundary";

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

    match call.taskspace_binding.as_deref() {
        Some(ACTIVE_BINDING | AFTER_BOUNDARY_BINDING) => Ok(()),
        Some(binding) => Err(binding_failure(
            "TASKSPACE_BINDING_INVALID",
            "taskspace_binding must be active or after_boundary",
            Some(binding),
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
