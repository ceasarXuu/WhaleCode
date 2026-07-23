use std::sync::Arc;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseInputItem;

use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::taskspace_control::execute_taskspace_initialization_binding;
use crate::tools::router::ToolCall;
use crate::tools::taskspace_binding::is_initialization_binding;

pub(crate) enum InitializationAction {
    None,
    Committed(String),
    Rejected(String),
}

pub(crate) async fn prepare_initialization(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    call: &ToolCall,
) -> Result<InitializationAction, FunctionCallError> {
    if !is_initialization_binding(call.taskspace_binding.as_deref()) {
        return Ok(InitializationAction::None);
    }
    let arguments = call
        .taskspace_binding
        .as_deref()
        .expect("initialization binding requires arguments");
    let output = Box::pin(execute_taskspace_initialization_binding(
        session,
        turn,
        &call.call_id,
        arguments,
    ))
    .await?;
    if output.success() {
        tracing::info!(
            target: "codex_core::taskspace",
            call_id = call.call_id,
            tool_name = call.tool_name.display(),
            "taskspace.initialization_carrier_committed"
        );
        Ok(InitializationAction::Committed(
            output.message().to_string(),
        ))
    } else {
        tracing::warn!(
            target: "codex_core::taskspace",
            call_id = call.call_id,
            tool_name = call.tool_name.display(),
            "taskspace.initialization_carrier_rejected"
        );
        Ok(InitializationAction::Rejected(output.message().to_string()))
    }
}

pub(crate) fn wrap_initialization_response(
    response: &mut ResponseInputItem,
    action: &InitializationAction,
    tool_dispatched: bool,
) {
    let initialization_result = match action {
        InitializationAction::None => return,
        InitializationAction::Committed(message) | InitializationAction::Rejected(message) => {
            serde_json::from_str::<serde_json::Value>(message)
                .unwrap_or_else(|_| serde_json::Value::String(message.clone()))
        }
    };
    let header = serde_json::json!({
        "schema_version": "TaskSpaceInitializationCarrierResultV1",
        "initialization_result": initialization_result,
        "tool_dispatched": tool_dispatched,
    })
    .to_string();

    if !tool_dispatched && matches!(action, InitializationAction::Rejected(_)) {
        replace_output(response, header);
        return;
    }

    match response {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => {
            prepend_output(&mut output.body, header);
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

fn replace_output(response: &mut ResponseInputItem, body: String) {
    match response {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => {
            output.body = FunctionCallOutputBody::Text(body);
            output.success = Some(false);
        }
        ResponseInputItem::McpToolCallOutput { output, .. } => {
            output.content = vec![serde_json::json!({
                "type": "text",
                "text": body,
            })];
            output.is_error = Some(true);
        }
        ResponseInputItem::ToolSearchOutput { .. } | ResponseInputItem::Message { .. } => {}
    }
}

fn prepend_output(body: &mut FunctionCallOutputBody, header: String) {
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
mod tests {
    use codex_protocol::models::FunctionCallOutputPayload;

    use super::*;

    #[test]
    fn committed_initialization_preserves_exact_tool_output() {
        let original = "exact tool output";
        let mut response = ResponseInputItem::FunctionCallOutput {
            call_id: "call".into(),
            output: FunctionCallOutputPayload::from_text(original.into()),
        };

        wrap_initialization_response(
            &mut response,
            &InitializationAction::Committed(
                r#"{"action":"initialize_map","state_commit":true,"committed_revision":2}"#.into(),
            ),
            true,
        );

        let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
            panic!("function output");
        };
        let text = output.body.to_text().expect("text");
        assert!(text.contains("TaskSpaceInitializationCarrierResultV1"));
        assert!(text.contains("\"tool_dispatched\":true"));
        assert!(text.ends_with(original));
    }

    #[test]
    fn rejected_initialization_replaces_non_executed_tool_output_once() {
        let mut response = ResponseInputItem::FunctionCallOutput {
            call_id: "call".into(),
            output: FunctionCallOutputPayload::from_text("placeholder".into()),
        };

        wrap_initialization_response(
            &mut response,
            &InitializationAction::Rejected("invalid graph".into()),
            false,
        );

        let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
            panic!("function output");
        };
        let text = output.body.to_text().expect("text");
        assert!(text.contains("\"tool_dispatched\":false"));
        assert_eq!(text.matches("invalid graph").count(), 1);
        assert!(!text.contains("placeholder"));
        assert_eq!(output.success, Some(false));
    }
}
