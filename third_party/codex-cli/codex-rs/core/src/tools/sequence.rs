use codex_protocol::error::Result;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use futures::future::join_all;
use tokio_util::sync::CancellationToken;

use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;

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
        if let SequenceSegment::Barrier { index } = &segment
            && let Some(reason) = finish_cadence_violation(&calls, *index)
        {
            let call = &calls[*index];
            tracing::warn!(
                target: "codex_core::taskspace",
                segment_index,
                call_id = call.call_id,
                reason,
                "tool.trailing_nonterminal_finish_rejected"
            );
            prior_failure = Some(call.call_id.clone());
            outputs.push(finish_cadence_rejection_response(call, reason));
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
                    runtime
                        .clone()
                        .handle_tool_call_for_sequence(call, cancellation_token.child_token())
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

fn is_nonterminal_finish(call: &ToolCall) -> bool {
    let Some(arguments) = taskspace_control_arguments(call) else {
        return false;
    };
    arguments.get("action").and_then(serde_json::Value::as_str) == Some("finish_node")
        && !arguments
            .get("final_candidate")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|candidate| !candidate.trim().is_empty())
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

fn finish_cadence_violation(calls: &[ToolCall], index: usize) -> Option<&'static str> {
    let call = calls.get(index)?;
    if !is_nonterminal_finish(call) {
        return None;
    }
    let Some(next_call) = calls.get(index + 1) else {
        return Some("nonterminal_finish_requires_follow_up_call");
    };
    let arguments = taskspace_control_arguments(call)?;
    let finish_establishes_binding = arguments
        .get("next_node_id")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|node_id| !node_id.trim().is_empty())
        || arguments
            .get("next_node_kind")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|kind| !kind.trim().is_empty());
    if finish_establishes_binding || call_establishes_binding(next_call) {
        None
    } else {
        Some("nonterminal_finish_requires_next_binding")
    }
}

fn call_establishes_binding(call: &ToolCall) -> bool {
    let Some(arguments) = taskspace_control_arguments(call) else {
        return false;
    };
    match arguments.get("action").and_then(serde_json::Value::as_str) {
        Some("bind_node") => true,
        Some("create_node") => {
            arguments
                .get("bind_current")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        }
        _ => false,
    }
}

fn finish_cadence_rejection_response(call: &ToolCall, reason: &str) -> ResponseInputItem {
    let message = match reason {
        "nonterminal_finish_requires_next_binding" => {
            "Nonterminal finish_node must establish the next binding before subsequent work. Use next_node_id or next_node_* fields, or follow finish_node immediately with bind_node or create_node(bind_current=true)."
        }
        _ => {
            "Nonterminal finish_node cannot be the last call in a response. Follow it with another TaskSpace control or ordinary tool call."
        }
    };
    let metadata = serde_json::json!({
        "schema_version": "TaskSpaceCadenceGateV1",
        "allowed": false,
        "gate_class": "cadence",
        "reason": reason,
        "required_follow_up": "taskspace_control_or_ordinary_tool_call",
        "required_binding": "next_node_id_or_next_node_fields_or_immediate_binding_control",
        "terminal_exception": "non_empty_final_candidate",
    });
    let mut output = FunctionCallOutputPayload::from_text(format!(
        "{message}\nTaskSpaceCadenceGateV1: {metadata}"
    ));
    output.success = Some(false);
    ResponseInputItem::FunctionCallOutput {
        call_id: call.call_id.clone(),
        output,
    }
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
    fn identifies_only_nonterminal_finish_calls() {
        let nonterminal = function_call_with_arguments(
            "taskspace_control",
            "finish",
            r#"{"action":"finish_node","next_node_id":"node-2"}"#,
        );
        let terminal = function_call_with_arguments(
            "taskspace_control",
            "terminal",
            r#"{"action":"finish_node","final_candidate":"done"}"#,
        );
        assert!(is_nonterminal_finish(&nonterminal));
        assert!(!is_nonterminal_finish(&terminal));
        assert!(!is_nonterminal_finish(&function_call(
            "exec_command",
            "ordinary"
        )));
    }

    #[test]
    fn cadence_rejection_is_unsuccessful_and_preserves_call_id() {
        let call = function_call_with_arguments(
            "taskspace_control",
            "finish-call",
            r#"{"action":"finish_node","next_node_id":"node-2"}"#,
        );
        let output =
            finish_cadence_rejection_response(&call, "nonterminal_finish_requires_follow_up_call");
        assert_eq!(response_input_call_id(&output), "finish-call");
        assert!(!response_input_succeeded(&output));
        let ResponseInputItem::FunctionCallOutput { output, .. } = output else {
            panic!("expected function output");
        };
        assert!(
            output
                .body
                .to_text()
                .is_some_and(|text| text.contains("TaskSpaceCadenceGateV1"))
        );
    }

    #[test]
    fn cadence_requires_binding_before_ordinary_follow_up() {
        let finish_without_binding = function_call_with_arguments(
            "taskspace_control",
            "finish",
            r#"{"action":"finish_node"}"#,
        );
        let finish_with_binding = function_call_with_arguments(
            "taskspace_control",
            "finish-next",
            r#"{"action":"finish_node","next_node_id":"node-2"}"#,
        );
        let bind = function_call_with_arguments(
            "taskspace_control",
            "bind",
            r#"{"action":"bind_node","node_id":"node-2"}"#,
        );
        let ordinary = function_call("exec_command", "test");

        assert_eq!(
            finish_cadence_violation(&[finish_without_binding.clone(), ordinary.clone()], 0),
            Some("nonterminal_finish_requires_next_binding")
        );
        assert_eq!(
            finish_cadence_violation(&[finish_with_binding.clone(), ordinary], 0),
            None
        );
        assert_eq!(
            finish_cadence_violation(&[finish_without_binding, bind], 0),
            None
        );
        assert_eq!(
            finish_cadence_violation(&[finish_with_binding], 0),
            Some("nonterminal_finish_requires_follow_up_call")
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
}
