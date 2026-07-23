use std::sync::Arc;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseInputItem;
use serde_json::json;

use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control::execute_taskspace_transition;
use crate::tools::handlers::taskspace_transition_args::TaskSpaceActionArgs;
use crate::tools::handlers::taskspace_transition_args::parse_taskspace_action_args;
use crate::tools::router::ToolCall;

pub(crate) enum CarrierAction {
    None,
    ContinueValidated,
    Rejected(String),
    TransitionCommitted(String),
}

pub(crate) async fn prepare_carried_action(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    call: &ToolCall,
) -> CarrierAction {
    if call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control" {
        return match call.taskspace_action.as_deref() {
            None => CarrierAction::None,
            Some(_) => reject_action(
                call,
                "protocol",
                action_failure(
                    None,
                    "protocol",
                    "TASKSPACE_ACTION_FORBIDDEN",
                    "taskspace_control cannot carry taskspace_action",
                    None,
                    None,
                    None,
                    None,
                ),
            ),
        };
    }
    if !session.taskspace_active().await {
        if call.taskspace_action.is_none() {
            return CarrierAction::None;
        }
        tracing::warn!(
            target: "codex_core::taskspace",
            call_id = call.call_id,
            tool_name = call.tool_name.display(),
            stage = "mode",
            "taskspace.carrier_action_rejected"
        );
        return CarrierAction::Rejected(action_failure(
            None,
            "protocol",
            "TASKSPACE_ACTION_MODE_MISMATCH",
            "taskspace_action is only available in TaskSpace sessions",
            None,
            None,
            None,
            None,
        ));
    }
    let Some(raw_action) = call.taskspace_action.as_deref() else {
        return if requires_taskspace_action(call) {
            reject_action(
                call,
                "required",
                action_failure(
                    None,
                    "protocol",
                    "TASKSPACE_ACTION_REQUIRED",
                    "TaskSpace ordinary Tool calls must explicitly declare taskspace_action",
                    None,
                    None,
                    None,
                    None,
                ),
            )
        } else {
            CarrierAction::None
        };
    };
    let args = match parse_taskspace_action_args(raw_action) {
        Ok(args) => args,
        Err(error) => {
            return reject_action(
                call,
                "arguments",
                action_failure(
                    None,
                    "protocol",
                    "TASKSPACE_ACTION_INVALID",
                    &error.to_string(),
                    None,
                    None,
                    None,
                    None,
                ),
            );
        }
    };
    if let TaskSpaceActionArgs::ContinueCurrent {
        expected_revision,
        current_node_id,
    } = &args
    {
        return validate_continue_current(session, call, *expected_revision, current_node_id).await;
    }
    let action = args.action_name();
    match execute_taskspace_transition(session, turn, &call.call_id, args).await {
        Ok(execution) if execution.success => {
            tracing::info!(
                target: "codex_core::taskspace",
                call_id = call.call_id,
                tool_name = call.tool_name.display(),
                action,
                "taskspace.carrier_transition_committed"
            );
            CarrierAction::TransitionCommitted(execution.message)
        }
        Ok(execution) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id = call.call_id,
                tool_name = call.tool_name.display(),
                action,
                stage = "state_machine",
                "taskspace.carrier_transition_rejected"
            );
            CarrierAction::Rejected(execution.message)
        }
        Err(error) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id = call.call_id,
                tool_name = call.tool_name.display(),
                action,
                stage = "execution",
                "taskspace.carrier_transition_rejected"
            );
            CarrierAction::Rejected(error.to_string())
        }
    }
}

fn requires_taskspace_action(call: &ToolCall) -> bool {
    matches!(
        call.payload,
        ToolPayload::Function { .. } | ToolPayload::ToolSearch { .. } | ToolPayload::Mcp { .. }
    )
}

async fn validate_continue_current(
    session: &Session,
    call: &ToolCall,
    expected_revision: u64,
    current_node_id: &str,
) -> CarrierAction {
    let state = session.action_map_control_state(None).await;
    if let Some(failure) = continue_current_failure(
        expected_revision,
        current_node_id,
        state
            .as_ref()
            .map(|state| (state.revision, state.current_node_id.as_deref())),
    ) {
        return reject_action(call, "state_machine", failure);
    }
    let state = state.expect("validated continuation requires canonical state");
    tracing::info!(
        target: "codex_core::taskspace",
        call_id = call.call_id,
        tool_name = call.tool_name.display(),
        revision = state.revision,
        current_node_id,
        "taskspace.carrier_continue_validated"
    );
    CarrierAction::ContinueValidated
}

fn continue_current_failure(
    expected_revision: u64,
    current_node_id: &str,
    canonical: Option<(u64, Option<&str>)>,
) -> Option<String> {
    let Some((canonical_revision, canonical_current_node_id)) = canonical else {
        return Some(action_failure(
            Some("continue_current"),
            "state_machine",
            "TASKSPACE_NO_ACTIVE_MAP",
            "continue_current requires an active TaskSpace Map",
            Some(expected_revision),
            None,
            Some(current_node_id),
            None,
        ));
    };
    if canonical_revision != expected_revision {
        return Some(action_failure(
            Some("continue_current"),
            "state_machine",
            "TASKSPACE_REVISION_MISMATCH",
            "continue_current expected_revision does not match the canonical Map revision",
            Some(expected_revision),
            Some(canonical_revision),
            Some(current_node_id),
            canonical_current_node_id,
        ));
    }
    if canonical_current_node_id != Some(current_node_id) {
        return Some(action_failure(
            Some("continue_current"),
            "state_machine",
            "TASKSPACE_BINDING_MISMATCH",
            "continue_current current_node_id does not match the canonical active binding",
            Some(expected_revision),
            Some(canonical_revision),
            Some(current_node_id),
            canonical_current_node_id,
        ));
    }
    None
}

fn reject_action(call: &ToolCall, stage: &'static str, message: String) -> CarrierAction {
    tracing::warn!(
        target: "codex_core::taskspace",
        call_id = call.call_id,
        tool_name = call.tool_name.display(),
        stage,
        "taskspace.carrier_action_rejected"
    );
    CarrierAction::Rejected(message)
}

fn action_failure(
    action: Option<&str>,
    failure_class: &str,
    code: &str,
    message: &str,
    submitted_expected_revision: Option<u64>,
    canonical_revision: Option<u64>,
    submitted_current_node_id: Option<&str>,
    canonical_current_node_id: Option<&str>,
) -> String {
    let status = if failure_class == "protocol" {
        "protocol_failed"
    } else {
        "state_machine_failed"
    };
    json!({
        "schema_version": "TaskSpaceActionValidationResultV1",
        "action": action,
        "status": status,
        "success": false,
        "state_commit": false,
        "submitted_expected_revision": submitted_expected_revision,
        "canonical_revision": canonical_revision,
        "submitted_current_node_id": submitted_current_node_id,
        "canonical_current_node_id": canonical_current_node_id,
        "error": {
            "class": failure_class,
            "code": code,
            "message": message,
        }
    })
    .to_string()
}

pub(crate) fn wrap_carrier_response(
    response: &mut ResponseInputItem,
    action: &CarrierAction,
    tool_dispatched: bool,
) {
    let message = match action {
        CarrierAction::None | CarrierAction::ContinueValidated => return,
        CarrierAction::Rejected(message) | CarrierAction::TransitionCommitted(message) => message,
    };
    let action_value = serde_json::from_str::<serde_json::Value>(message)
        .unwrap_or_else(|_| serde_json::Value::String(message.clone()));
    let header = serde_json::json!({
        "schema_version": "TaskSpaceCarrierResultV2",
        "action_result": action_value,
        "tool_dispatched": tool_dispatched,
    })
    .to_string();

    if !tool_dispatched {
        replace_function_output(response, header);
        return;
    }

    match response {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => {
            prepend_to_function_output(&mut output.body, header);
        }
        ResponseInputItem::McpToolCallOutput { output, .. } => {
            output.content.insert(
                0,
                serde_json::json!({
                    "type": "text",
                    "text": header,
                }),
            );
        }
        ResponseInputItem::ToolSearchOutput { .. } | ResponseInputItem::Message { .. } => {}
    }
}

fn replace_function_output(response: &mut ResponseInputItem, body: String) {
    match response {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => {
            output.body = FunctionCallOutputBody::Text(body);
        }
        ResponseInputItem::McpToolCallOutput { output, .. } => {
            output.content = vec![serde_json::json!({
                "type": "text",
                "text": body,
            })];
        }
        ResponseInputItem::ToolSearchOutput { .. } | ResponseInputItem::Message { .. } => {}
    }
}

fn prepend_to_function_output(body: &mut FunctionCallOutputBody, header: String) {
    match body {
        FunctionCallOutputBody::Text(text) => {
            *text = format!("{header}\n{text}");
        }
        FunctionCallOutputBody::ContentItems(items) => {
            items.insert(0, FunctionCallOutputContentItem::InputText { text: header });
        }
    }
}

#[cfg(test)]
#[path = "taskspace_carrier_tests.rs"]
mod tests;
