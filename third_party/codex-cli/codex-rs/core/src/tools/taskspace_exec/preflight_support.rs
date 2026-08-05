use std::collections::BTreeSet;

use serde_json::Value;

use super::TASKSPACE_EXEC_PLAN_VERSION;
use super::catalog::TaskspaceExecCatalog;
use super::plan::TaskspaceExecCall;
use super::plan::TaskspaceExecHostedBinding;
use super::plan::TaskspaceExecPlan;

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

pub(super) fn validate_hosted_bindings(
    bindings: &[TaskspaceExecHostedBinding],
) -> Result<(), TaskspaceExecPreflightError> {
    for (index, binding) in bindings.iter().enumerate() {
        if binding.tool.trim().is_empty() {
            return Err(plan_error(
                "hosted_binding_tool_empty",
                format!("TaskSpace Exec hosted_bindings[{index}].tool must be non-empty"),
            ));
        }
        if binding.node_ids.is_empty() {
            return Err(plan_error(
                "hosted_binding_node_ids_empty",
                format!("TaskSpace Exec hosted_bindings[{index}].node_ids must be non-empty"),
            ));
        }
        let mut node_ids = BTreeSet::new();
        for (node_index, node_id) in binding.node_ids.iter().enumerate() {
            let node_id = node_id.trim();
            if node_id.is_empty() {
                return Err(plan_error(
                    "hosted_binding_node_id_empty",
                    format!(
                        "TaskSpace Exec hosted_bindings[{index}].node_ids[{node_index}] must be non-empty"
                    ),
                ));
            }
            if !node_ids.insert(node_id) {
                return Err(plan_error(
                    "hosted_binding_node_id_duplicate",
                    format!("TaskSpace Exec hosted_bindings[{index}].node_ids repeats `{node_id}`"),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_plan_identity(
    plan: &TaskspaceExecPlan,
    catalog: &TaskspaceExecCatalog,
) -> Result<(), TaskspaceExecPreflightError> {
    if plan.version != TASKSPACE_EXEC_PLAN_VERSION {
        return Err(plan_error(
            "plan_version_mismatch",
            format!(
                "TaskSpace Exec plan version `{}` does not match `{TASKSPACE_EXEC_PLAN_VERSION}`",
                plan.version
            ),
        ));
    }
    if plan.capability_id != catalog.identity {
        return Err(plan_error(
            "capability_identity_mismatch",
            "TaskSpace Exec plan capability identity does not match the admitted catalog",
        ));
    }
    Ok(())
}

pub(super) fn validate_work_binding(
    call: &TaskspaceExecCall,
    index: usize,
) -> Result<(), TaskspaceExecPreflightError> {
    if call
        .node_id
        .as_deref()
        .is_none_or(|node_id| node_id.trim().is_empty())
    {
        return Err(call_error(
            "work_node_binding_missing",
            index,
            call,
            "Every non-map client call requires an Agent-declared node_id",
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
