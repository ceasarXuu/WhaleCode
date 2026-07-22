use std::sync::Arc;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseInputItem;

use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::taskspace_control::execute_taskspace_transition;
use crate::tools::handlers::taskspace_transition_args::parse_taskspace_transition_args;
use crate::tools::router::ToolCall;

pub(crate) enum CarrierTransition {
    None,
    Rejected(String),
    Committed(String),
}

pub(crate) async fn commit_carried_transition(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    call: &ToolCall,
) -> CarrierTransition {
    let Some(raw_transition) = call.taskspace_transition.as_deref() else {
        return CarrierTransition::None;
    };
    if call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control" {
        tracing::warn!(
            target: "codex_core::taskspace",
            call_id = call.call_id,
            tool_name = call.tool_name.display(),
            stage = "carrier",
            "taskspace.carrier_transition_rejected"
        );
        return CarrierTransition::Rejected(
            "taskspace_control cannot carry taskspace_transition".into(),
        );
    }
    if !turn.tools_config.collab_tools {
        tracing::warn!(
            target: "codex_core::taskspace",
            call_id = call.call_id,
            tool_name = call.tool_name.display(),
            stage = "mode",
            "taskspace.carrier_transition_rejected"
        );
        return CarrierTransition::Rejected(
            "taskspace_transition is only available in TaskSpace sessions".into(),
        );
    }
    let args = match parse_taskspace_transition_args(raw_transition) {
        Ok(args) => args,
        Err(error) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id = call.call_id,
                tool_name = call.tool_name.display(),
                stage = "arguments",
                "taskspace.carrier_transition_rejected"
            );
            return CarrierTransition::Rejected(error.to_string());
        }
    };
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
            CarrierTransition::Committed(execution.message)
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
            CarrierTransition::Rejected(execution.message)
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
            CarrierTransition::Rejected(error.to_string())
        }
    }
}

pub(crate) fn wrap_carrier_response(
    response: &mut ResponseInputItem,
    transition: &CarrierTransition,
    tool_dispatched: bool,
) {
    let message = match transition {
        CarrierTransition::None => return,
        CarrierTransition::Rejected(message) | CarrierTransition::Committed(message) => message,
    };
    let transition_value = serde_json::from_str::<serde_json::Value>(message)
        .unwrap_or_else(|_| serde_json::Value::String(message.clone()));
    let header = serde_json::json!({
        "schema_version": "TaskSpaceCarrierResultV1",
        "transition": transition_value,
        "tool_dispatched": tool_dispatched,
    })
    .to_string();

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
