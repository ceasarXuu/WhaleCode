use codex_protocol::error::Result;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use futures::future::join_all;
use tokio_util::sync::CancellationToken;

use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNestedAction;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::parallel::ToolCallExecution;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;

pub(crate) struct TerminalAgentMessage {
    pub(crate) call_id: String,
    pub(crate) message: String,
}

pub(crate) struct ToolSequenceOutcome {
    pub(crate) outputs: Vec<ResponseInputItem>,
    pub(crate) terminal_agent_message: Option<TerminalAgentMessage>,
}

#[derive(Debug, PartialEq, Eq)]
enum SequenceSegment {
    Parallel { start: usize, end: usize },
    Barrier { index: usize },
}

pub(crate) async fn execute_response_tool_sequence(
    runtime: ToolCallRuntime,
    calls: Vec<ToolCall>,
    cancellation_token: CancellationToken,
) -> Result<ToolSequenceOutcome> {
    if calls.is_empty() {
        return Ok(ToolSequenceOutcome {
            outputs: Vec::new(),
            terminal_agent_message: None,
        });
    }

    match ToolSequenceManifest::from_calls(&calls) {
        Ok(manifest) => tracing::info!(
            target: "codex_core::taskspace",
            declared_tool_count = manifest.entries.len(),
            request_patch_count = manifest.request_patch_count,
            "tool.request_manifest_built"
        ),
        Err(error) => tracing::warn!(
            target: "codex_core::taskspace",
            error,
            "tool.request_manifest_invalid"
        ),
    }

    let segments = sequence_segments(&calls);
    tracing::info!(
        target: "codex_core::taskspace",
        call_count = calls.len(),
        segment_count = segments.len(),
        "tool_response_sequence_started"
    );

    let mut outputs = Vec::with_capacity(calls.len());
    let mut prior_failure: Option<String> = None;
    let mut terminal_agent_message: Option<TerminalAgentMessage> = None;
    for (segment_index, segment) in segments.into_iter().enumerate() {
        if let Some(terminal) = terminal_agent_message.as_ref() {
            for call in calls_for_segment(&calls, &segment) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    terminal_call_id = terminal.call_id,
                    "tool_response_sequence_call_skipped"
                );
                outputs.push(ToolCallRuntime::terminal_completion_skipped_response(
                    call,
                    &terminal.call_id,
                ));
            }
            continue;
        }
        if let Some(prior_call_id) = prior_failure.as_deref() {
            for call in calls_for_segment(&calls, &segment) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    prior_call_id,
                    "tool_response_sequence_call_skipped"
                );
                outputs.push(ToolCallRuntime::skipped_response(call, prior_call_id));
            }
            continue;
        }
        let barrier_call_id = match &segment {
            SequenceSegment::Barrier { index } => Some(calls[*index].call_id.clone()),
            SequenceSegment::Parallel { .. } => None,
        };
        let segment_executions = match segment {
            SequenceSegment::Parallel { start, end } => {
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_count = end - start,
                    "tool_response_parallel_segment_started"
                );
                let futures = calls[start..end].iter().cloned().map(|call| {
                    runtime
                        .clone()
                        .handle_tool_call_for_sequence(call, cancellation_token.child_token())
                });
                join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            }
            SequenceSegment::Barrier { index } => {
                let call = calls[index].clone();
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    "tool.barrier_started"
                );
                vec![
                    execute_taskspace_barrier(
                        runtime.clone(),
                        call,
                        cancellation_token.child_token(),
                    )
                    .await?,
                ]
            }
        };

        for execution in &segment_executions {
            let output = &execution.response;
            if !response_input_succeeded(output) && prior_failure.is_none() {
                prior_failure = Some(response_input_call_id(output).to_string());
            }
            if let Some(message) = execution.terminal_agent_message.as_ref() {
                let call_id = response_input_call_id(output).to_string();
                terminal_agent_message = Some(TerminalAgentMessage {
                    call_id: call_id.clone(),
                    message: message.clone(),
                });
                tracing::info!(
                    target: "codex_core::taskspace",
                    call_id,
                    candidate_bytes = message.len(),
                    "taskspace_agent_final_staged"
                );
            }
        }
        if let Some(call_id) = barrier_call_id {
            if prior_failure.as_deref() == Some(call_id.as_str()) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id,
                    failure_class = "tool_output_unsuccessful",
                    "tool.barrier_failed"
                );
            } else {
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id,
                    "tool.barrier_completed"
                );
            }
        }
        tracing::info!(
            target: "codex_core::taskspace",
            segment_index,
            failed = prior_failure.is_some(),
            "tool_response_sequence_segment_completed"
        );
        outputs.extend(
            segment_executions
                .into_iter()
                .map(|execution| execution.response),
        );
    }

    tracing::info!(
        target: "codex_core::taskspace",
        call_count = calls.len(),
        failed = prior_failure.is_some(),
        "tool_response_sequence_completed"
    );
    Ok(ToolSequenceOutcome {
        outputs,
        terminal_agent_message,
    })
}

async fn execute_taskspace_barrier(
    runtime: ToolCallRuntime,
    call: ToolCall,
    cancellation_token: CancellationToken,
) -> Result<ToolCallExecution> {
    let nested_actions = taskspace_nested_actions(&call);
    if nested_actions.is_empty() {
        return runtime
            .handle_tool_call_for_sequence(call, cancellation_token)
            .await;
    }

    let mut nested_calls = Vec::with_capacity(nested_actions.len());
    for (index, action) in nested_actions.iter().enumerate() {
        if !runtime.nested_action_is_visible(action) {
            let message = format!(
                "taskspace_control nested tool `{}` is not visible in the current request",
                action.tool_name()
            );
            tracing::warn!(
                target: "codex_core::taskspace",
                outer_call_id = call.call_id,
                nested_index = index,
                tool_name = action.tool_name(),
                reason = "nested_tool_not_visible",
                "taskspace.control_batch_preflight_failed"
            );
            return Ok(ToolCallExecution {
                response: ToolCallRuntime::invalid_call_response(&call, message),
                terminal_agent_message: None,
            });
        }
        let call_id = format!("{}:nested:{index}", call.call_id);
        let nested_call = match runtime.build_nested_tool_call(action, call_id).await {
            Ok(Some(call)) => call,
            Ok(None) => {
                let message = format!(
                    "taskspace_control nested tool `{}` did not produce a callable payload",
                    action.tool_name()
                );
                return Ok(ToolCallExecution {
                    response: ToolCallRuntime::invalid_call_response(&call, message),
                    terminal_agent_message: None,
                });
            }
            Err(error) => {
                return Ok(ToolCallExecution {
                    response: ToolCallRuntime::invalid_call_response(&call, error.to_string()),
                    terminal_agent_message: None,
                });
            }
        };
        nested_calls.push(nested_call);
    }

    tracing::info!(
        target: "codex_core::taskspace",
        outer_call_id = call.call_id,
        nested_count = nested_calls.len(),
        "taskspace.control_batch_validated"
    );
    let state_execution = runtime
        .clone()
        .handle_tool_call_for_sequence(call.clone(), cancellation_token.child_token())
        .await?;
    if !response_input_succeeded(&state_execution.response)
        || state_execution.terminal_agent_message.is_some()
    {
        return Ok(state_execution);
    }

    let mut nested_outputs = Vec::with_capacity(nested_calls.len());
    let mut failed_call_id = None;
    for (nested_call, call_item) in nested_calls {
        let tool_name = nested_call.tool_name.display();
        let call_event_ref = runtime
            .record_taskspace_child_item(&call_item, &call.call_id)
            .await?;
        let output = if let Some(prior_call_id) = failed_call_id.as_deref() {
            ToolCallRuntime::skipped_response(&nested_call, prior_call_id)
        } else {
            let execution = runtime
                .clone()
                .handle_tool_call_for_sequence(
                    nested_call.clone(),
                    cancellation_token.child_token(),
                )
                .await?;
            if !response_input_succeeded(&execution.response) {
                failed_call_id = Some(nested_call.call_id.clone());
            }
            execution.response
        };
        let output_item = ResponseItem::from(output.clone());
        let output_event_ref = runtime
            .record_taskspace_child_item(&output_item, &call.call_id)
            .await?;
        nested_outputs.push((
            tool_name,
            nested_call.call_id,
            response_input_succeeded(&output),
            call_event_ref,
            output_event_ref,
        ));
    }

    let success = failed_call_id.is_none();
    tracing::info!(
        target: "codex_core::taskspace",
        outer_call_id = call.call_id,
        nested_count = nested_outputs.len(),
        success,
        "taskspace.control_batch_completed"
    );
    Ok(ToolCallExecution {
        response: aggregate_taskspace_batch_response(
            &call.call_id,
            state_execution.response,
            nested_outputs,
            success,
        ),
        terminal_agent_message: None,
    })
}

fn taskspace_nested_actions(call: &ToolCall) -> Vec<TaskSpaceNestedAction> {
    let Some(arguments) = taskspace_control_arguments(call) else {
        return Vec::new();
    };
    parse_taskspace_control_args(&arguments.to_string())
        .map(|args| args.nested_actions())
        .unwrap_or_default()
}

fn aggregate_taskspace_batch_response(
    outer_call_id: &str,
    state_response: ResponseInputItem,
    nested_outputs: Vec<(String, String, bool, String, String)>,
    success: bool,
) -> ResponseInputItem {
    let mut batch = state_response_text(&state_response)
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultV1",
                "status": "state_committed",
                "success": true,
                "steps": [{
                    "kind": "state_transition",
                    "response": state_response,
                }],
            })
        });
    let steps = batch
        .as_object_mut()
        .and_then(|object| object.get_mut("steps"))
        .and_then(serde_json::Value::as_array_mut);
    if let Some(steps) = steps {
        for (tool_name, call_id, success, call_event_ref, output_event_ref) in nested_outputs {
            steps.push(serde_json::json!({
                "kind": "ordinary_tool",
                "tool_name": tool_name,
                "call_id": call_id,
                "success": success,
                "call_event_ref": call_event_ref,
                "output_event_ref": output_event_ref,
            }));
        }
    }
    if let Some(object) = batch.as_object_mut() {
        object.insert("success".into(), serde_json::json!(success));
        object.insert(
            "status".into(),
            serde_json::json!(if success { "completed" } else { "partial" }),
        );
    }
    let mut output = FunctionCallOutputPayload::from_text(batch.to_string());
    output.success = Some(success);
    ResponseInputItem::FunctionCallOutput {
        call_id: outer_call_id.to_string(),
        output,
    }
}

fn state_response_text(response: &ResponseInputItem) -> Option<&str> {
    let ResponseInputItem::FunctionCallOutput { output, .. } = response else {
        return None;
    };
    match &output.body {
        FunctionCallOutputBody::Text(text) => Some(text),
        FunctionCallOutputBody::ContentItems(_) => None,
    }
}

fn sequence_segments(calls: &[ToolCall]) -> Vec<SequenceSegment> {
    let mut segments = Vec::new();
    let mut ordinary_start = 0;
    for (index, call) in calls.iter().enumerate() {
        if !is_taskspace_control(call) {
            continue;
        }
        if ordinary_start < index {
            segments.push(SequenceSegment::Parallel {
                start: ordinary_start,
                end: index,
            });
        }
        segments.push(SequenceSegment::Barrier { index });
        ordinary_start = index + 1;
    }
    if ordinary_start < calls.len() {
        segments.push(SequenceSegment::Parallel {
            start: ordinary_start,
            end: calls.len(),
        });
    }
    segments
}

fn calls_for_segment<'a>(calls: &'a [ToolCall], segment: &SequenceSegment) -> &'a [ToolCall] {
    match *segment {
        SequenceSegment::Parallel { start, end } => &calls[start..end],
        SequenceSegment::Barrier { index } => &calls[index..=index],
    }
}

fn is_taskspace_control(call: &ToolCall) -> bool {
    call.tool_name.namespace.is_none() && call.tool_name.name == "taskspace_control"
}

fn taskspace_control_arguments(call: &ToolCall) -> Option<serde_json::Value> {
    if !is_taskspace_control(call) {
        return None;
    }
    let ToolPayload::Function { arguments } = &call.payload else {
        return None;
    };
    serde_json::from_str(arguments).ok()
}

fn response_input_call_id(output: &ResponseInputItem) -> &str {
    match output {
        ResponseInputItem::FunctionCallOutput { call_id, .. }
        | ResponseInputItem::McpToolCallOutput { call_id, .. }
        | ResponseInputItem::CustomToolCallOutput { call_id, .. }
        | ResponseInputItem::ToolSearchOutput { call_id, .. } => call_id,
        ResponseInputItem::Message { .. } => "unknown",
    }
}

fn response_input_succeeded(output: &ResponseInputItem) -> bool {
    match output {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => output.success != Some(false),
        ResponseInputItem::McpToolCallOutput { output, .. } => output.success(),
        ResponseInputItem::ToolSearchOutput { status, .. } => status == "completed",
        ResponseInputItem::Message { .. } => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tools::ToolName;

    fn function_call(name: &str, call_id: &str) -> ToolCall {
        function_call_with_arguments(name, call_id, "{}")
    }

    fn function_call_with_arguments(name: &str, call_id: &str, arguments: &str) -> ToolCall {
        ToolCall {
            tool_name: ToolName::plain(name),
            call_id: call_id.to_string(),
            payload: ToolPayload::Function {
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn preserves_provider_order_around_state_barriers() {
        let calls = vec![
            function_call("read_file", "read-1"),
            function_call("read_file", "read-2"),
            function_call("taskspace_control", "finish"),
            function_call("apply_patch", "edit"),
            function_call("taskspace_control", "finish-2"),
            function_call("exec_command", "test"),
        ];

        assert_eq!(
            sequence_segments(&calls),
            vec![
                SequenceSegment::Parallel { start: 0, end: 2 },
                SequenceSegment::Barrier { index: 2 },
                SequenceSegment::Parallel { start: 3, end: 4 },
                SequenceSegment::Barrier { index: 4 },
                SequenceSegment::Parallel { start: 5, end: 6 },
            ]
        );
    }

    #[test]
    fn leaves_ordinary_only_response_as_one_parallel_segment() {
        let calls = vec![
            function_call("read_file", "read-1"),
            function_call("exec_command", "read-2"),
        ];
        assert_eq!(
            sequence_segments(&calls),
            vec![SequenceSegment::Parallel { start: 0, end: 2 }]
        );
    }

    #[test]
    fn preserves_adjacent_finish_barriers_before_follow_up_action() {
        let calls = vec![
            function_call("taskspace_control", "finish-1"),
            function_call("taskspace_control", "finish-2"),
            function_call("exec_command", "test"),
        ];
        assert_eq!(
            sequence_segments(&calls),
            vec![
                SequenceSegment::Barrier { index: 0 },
                SequenceSegment::Barrier { index: 1 },
                SequenceSegment::Parallel { start: 2, end: 3 },
            ]
        );
    }

    #[test]
    fn skipped_output_preserves_call_id_and_failure_status() {
        let call = function_call("apply_patch", "edit-call");
        let output = ToolCallRuntime::skipped_response(&call, "finish-call");
        assert_eq!(response_input_call_id(&output), "edit-call");
        assert!(!response_input_succeeded(&output));
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("expected function output");
        };
        assert!(
            output
                .body
                .to_text()
                .is_some_and(|text| text.contains("skipped_due_to_prior_failure"))
        );
    }

    #[test]
    fn extracts_bootstrap_nested_actions() {
        let call = function_call_with_arguments(
            "taskspace_control",
            "outer",
            r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Read"}],"current_node_id":"node-1","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
        );

        let actions = taskspace_nested_actions(&call);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].tool_name(), "exec_command");
    }

    #[test]
    fn aggregate_references_canonical_nested_events_without_copying_output() {
        let state = ResponseInputItem::FunctionCallOutput {
            call_id: "outer".into(),
            output: FunctionCallOutputPayload::from_text(
                serde_json::json!({
                    "schema_version": "TaskSpaceControlResultV1",
                    "success": true,
                    "steps": [{"kind": "finish", "success": true}],
                })
                .to_string(),
            ),
        };
        let aggregated = aggregate_taskspace_batch_response(
            "outer",
            state,
            vec![(
                "apply_patch".into(),
                "outer:nested:0".into(),
                true,
                "task-event-7".into(),
                "task-event-8".into(),
            )],
            true,
        );
        let ResponseInputItem::FunctionCallOutput { call_id, output } = aggregated else {
            panic!("expected outer function output");
        };
        assert_eq!(call_id, "outer");
        assert_eq!(output.success, Some(true));
        let value: serde_json::Value =
            serde_json::from_str(&output.body.to_text().expect("text")).expect("batch json");
        assert_eq!(value["steps"].as_array().expect("steps").len(), 2);
        assert_eq!(value["steps"][1]["call_event_ref"], "task-event-7");
        assert_eq!(value["steps"][1]["output_event_ref"], "task-event-8");
        assert!(value["steps"][1].get("response").is_none());
    }
}
