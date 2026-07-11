use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::ActionMapInitializeOutcome;
use crate::action_map::ActionMapNextNodeDraft;
use crate::action_map::NodeKind;
use crate::action_map::TaskSpaceHardGateClass;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNonterminalFinishArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceTerminalFinishArgs;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::output_reference::OUTPUT_SLICE_MAX_BYTES;
use crate::tools::output_reference::OutputSliceMode;
use crate::tools::output_reference::OutputSliceRequest;
use crate::tools::output_reference::read_output_artifact_slice;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct TaskSpaceControlHandler;

pub struct TaskSpaceControlOutput {
    message: String,
    success: bool,
    terminal_agent_message: Option<String>,
}

impl ToolOutput for TaskSpaceControlOutput {
    fn log_preview(&self) -> String {
        self.message.clone()
    }

    fn success_for_logging(&self) -> bool {
        self.success
    }

    fn to_response_item(&self, call_id: &str, _payload: &ToolPayload) -> ResponseInputItem {
        let mut output = FunctionCallOutputPayload::from_text(self.message.clone());
        output.success = Some(self.success);
        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output,
        }
    }

    fn terminal_agent_message(&self) -> Option<&str> {
        self.terminal_agent_message.as_deref()
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        JsonValue::String(self.message.clone())
    }
}

impl ToolHandler for TaskSpaceControlHandler {
    type Output = TaskSpaceControlOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(protocol_error(
                    "taskspace_control received unsupported payload".into(),
                    "unsupported_payload",
                ));
            }
        };
        let args = parse_taskspace_control_args(&arguments)?;

        let (message, success, terminal_agent_message) = match args {
            TaskSpaceControlArgs::InitializeThenActions {
                task_title,
                task_objective,
                initial_nodes,
                current_node_id,
                actions: _,
            } => {
                let nodes = initial_nodes
                    .into_iter()
                    .map(|node| {
                        Ok(ActionMapInitializeNodeInput {
                            id: node.node_id,
                            kind: parse_node_kind("initial_nodes.kind", &node.kind)?,
                            title: node.title,
                            context_summary: node.context_summary,
                            dependency_node_ids: node.dependency_node_ids,
                        })
                    })
                    .collect::<Result<Vec<_>, FunctionCallError>>()?;
                let outcome = session
                    .initialize_action_map_for_main(
                        &turn,
                        ActionMapInitializeInput {
                            task_title,
                            task_objective,
                            nodes,
                            current_node_id,
                        },
                    )
                    .await
                    .map_err(state_machine_error)?;
                (
                    format_state_batch(
                        "initialize_then_actions",
                        vec![format_initialize_map_output(&outcome)],
                        true,
                    ),
                    true,
                    None,
                )
            }
            TaskSpaceControlArgs::FinishThenActions {
                finishes,
                actions: _,
            } => {
                let (steps, success) =
                    execute_nonterminal_finishes(&session, &turn, finishes).await;
                (
                    format_state_batch("finish_then_actions", steps, success),
                    success,
                    None,
                )
            }
            TaskSpaceControlArgs::FinishThenEnd {
                preceding_finishes,
                terminal_finish,
                final_candidate,
            } => {
                let (mut steps, mut success) =
                    execute_nonterminal_finishes(&session, &turn, preceding_finishes).await;
                let mut terminal_message = None;
                if success {
                    match execute_terminal_finish(
                        &session,
                        &turn,
                        terminal_finish,
                        &final_candidate,
                    )
                    .await
                    {
                        Ok(step) => {
                            steps.push(step);
                            terminal_message = Some(final_candidate);
                        }
                        Err(error) => {
                            steps.push(format_failed_state_step(steps.len(), &error));
                            success = false;
                        }
                    }
                }
                (
                    format_state_batch("finish_then_end", steps, success),
                    success,
                    terminal_message,
                )
            }
            TaskSpaceControlArgs::CreateNode {
                kind,
                title,
                context_summary,
                dependency_node_ids,
                bind_current,
            } => {
                let node_id = session
                    .create_action_map_node_for_main_with_kind(
                        &turn,
                        parse_node_kind("kind", &kind)?,
                        title,
                        context_summary,
                        dependency_node_ids,
                        bind_current,
                    )
                    .await
                    .map_err(state_machine_error)?;
                if bind_current {
                    (
                        format!("TaskSpace node created and bound: {node_id}"),
                        true,
                        None,
                    )
                } else {
                    (format!("TaskSpace node created: {node_id}"), true, None)
                }
            }
            TaskSpaceControlArgs::BindNode { node_id } => {
                session
                    .bind_action_map_main_node(&turn, &node_id)
                    .await
                    .map_err(state_machine_error)?;
                (format!("TaskSpace main node bound: {node_id}"), true, None)
            }
            TaskSpaceControlArgs::BlockNode {
                node_id,
                blocker_summary,
            } => {
                let result_id = session
                    .block_action_map_main_node(&turn, &node_id, blocker_summary)
                    .await
                    .map_err(state_machine_error)?;
                (
                    format!("TaskSpace node blocked: {node_id} result {result_id}"),
                    true,
                    None,
                )
            }
            TaskSpaceControlArgs::ReadOutputRef {
                output_ref,
                mode,
                start_line,
                end_line,
                pattern,
                max_bytes,
            } => {
                let request = OutputSliceRequest {
                    mode: parse_output_slice_mode(&mode, start_line, end_line, pattern)?,
                    max_bytes: max_bytes.unwrap_or(OUTPUT_SLICE_MAX_BYTES),
                };
                let rollout_path = session.current_rollout_path().await.map_err(|error| {
                    resource_error(error.to_string(), "output_reference_store_unavailable")
                })?;
                let slice =
                    read_output_artifact_slice(rollout_path.as_deref(), &output_ref, request)
                        .await
                        .map_err(|error| {
                            resource_error(error.to_string(), "output_reference_read_failed")
                        })?;
                session
                    .record_action_map_output_ref_trace_event(
                        &turn,
                        "output_ref.slice_read",
                        None,
                        output_ref,
                        vec![
                            "output_ref".into(),
                            "slice_read".into(),
                            format!("mode:{mode}"),
                        ],
                    )
                    .await;
                (slice, true, None)
            }
        };
        Ok(TaskSpaceControlOutput {
            message,
            success,
            terminal_agent_message,
        })
    }
}

async fn execute_nonterminal_finishes(
    session: &Session,
    turn: &TurnContext,
    finishes: Vec<TaskSpaceNonterminalFinishArgs>,
) -> (Vec<JsonValue>, bool) {
    let mut steps = Vec::with_capacity(finishes.len());
    for finish in finishes {
        match execute_nonterminal_finish(session, turn, finish).await {
            Ok(step) => steps.push(step),
            Err(error) => {
                steps.push(format_failed_state_step(steps.len(), &error));
                return (steps, false);
            }
        }
    }
    (steps, true)
}

async fn execute_nonterminal_finish(
    session: &Session,
    turn: &TurnContext,
    finish: TaskSpaceNonterminalFinishArgs,
) -> Result<JsonValue, FunctionCallError> {
    let draft = build_next_node_draft(
        finish.next_node_kind,
        finish.next_node_title,
        finish.next_node_context_summary,
        finish.next_dependency_node_ids,
    )?;
    let (node_id, outcome) = session
        .finish_action_map_current_or_named_node_with_next(
            turn,
            finish.node_id.as_deref(),
            finish.result_summary,
            finish.next_node_id,
            draft,
        )
        .await
        .map_err(state_machine_error)?;
    Ok(serde_json::json!({
        "kind": "finish",
        "node_id": node_id,
        "result_id": outcome.result_id,
        "next_node_id": outcome.next_node_id,
        "success": true,
    }))
}

async fn execute_terminal_finish(
    session: &Session,
    turn: &TurnContext,
    finish: TaskSpaceTerminalFinishArgs,
    final_candidate: &str,
) -> Result<JsonValue, FunctionCallError> {
    let (node_id, outcome) = session
        .finish_action_map_node_with_terminal_candidate(
            turn,
            finish.node_id.as_deref(),
            finish.result_summary,
            final_candidate,
        )
        .await
        .map_err(state_machine_error)?;
    Ok(serde_json::json!({
        "kind": "terminal_finish",
        "node_id": node_id,
        "result_id": outcome.result_id,
        "success": true,
    }))
}

fn format_failed_state_step(index: usize, error: &FunctionCallError) -> JsonValue {
    serde_json::json!({
        "kind": "state_transition",
        "index": index,
        "success": false,
        "output": error.to_string(),
    })
}

fn format_state_batch(action: &str, steps: Vec<JsonValue>, success: bool) -> String {
    serde_json::json!({
        "schema_version": "TaskSpaceControlBatchResultV1",
        "action": action,
        "status": if success { "state_committed" } else { "state_failed" },
        "success": success,
        "steps": steps,
    })
    .to_string()
}

fn parse_output_slice_mode(
    mode: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<String>,
) -> Result<OutputSliceMode, FunctionCallError> {
    match mode {
        "head" => Ok(OutputSliceMode::Head),
        "tail" => Ok(OutputSliceMode::Tail),
        "line_range" => Ok(OutputSliceMode::LineRange {
            start_line: start_line.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires start_line".into(),
                    "missing_argument",
                )
            })?,
            end_line: end_line.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires end_line".into(),
                    "missing_argument",
                )
            })?,
        }),
        "grep" => Ok(OutputSliceMode::Grep {
            pattern: pattern.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires pattern".into(),
                    "missing_argument",
                )
            })?,
        }),
        _ => Err(protocol_error(
            "read_output_ref mode must be head, tail, line_range, or grep".into(),
            "invalid_argument_value",
        )),
    }
}

fn parse_node_kind(field: &str, value: &str) -> Result<NodeKind, FunctionCallError> {
    NodeKind::from_str(value).ok_or_else(|| {
        protocol_error(
            format!("taskspace_control {field} has invalid node kind `{value}`"),
            "invalid_argument_value",
        )
    })
}

fn build_next_node_draft(
    kind: Option<String>,
    title: Option<String>,
    context_summary: Option<String>,
    dependency_node_ids: Vec<String>,
) -> Result<Option<ActionMapNextNodeDraft>, FunctionCallError> {
    let has_any = kind
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || title
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || context_summary
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || !dependency_node_ids.is_empty();
    if !has_any {
        return Ok(None);
    }
    let kind = parse_node_kind("next_node_kind", kind.as_deref().unwrap_or_default())?;
    let title = title.unwrap_or_default();
    let context_summary = context_summary.unwrap_or_default();
    if title.trim().is_empty() || context_summary.trim().is_empty() {
        return Err(protocol_error(
            "finish next-node creation requires kind, title, and context".into(),
            "missing_argument",
        ));
    }
    Ok(Some(ActionMapNextNodeDraft {
        kind,
        title,
        context_summary,
        dependency_node_ids,
    }))
}

fn format_initialize_map_output(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "schema_version": "TaskSpaceInitializeMapResultV1",
        "action": "initialize_then_actions",
        "status": "initialized",
        "task_id": outcome.task_id,
        "map_id": outcome.map_id,
        "current_node_id": outcome.current_node_id,
        "node_ids": outcome.node_ids,
    })
}

fn state_machine_error(message: String) -> FunctionCallError {
    let reason = hard_state_reason(&message)
        .unwrap_or("transition_rejected")
        .to_string();
    gate_error(message, TaskSpaceHardGateClass::StateMachine, &reason)
}

fn protocol_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Protocol, reason)
}

fn resource_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Resource, reason)
}

fn gate_error(message: String, class: TaskSpaceHardGateClass, reason: &str) -> FunctionCallError {
    let metadata = serde_json::json!({
        "schema_version": "TaskSpaceGateRecoveryV1",
        "allowed": false,
        "gate_class": class.as_str(),
        "reason": reason,
    });
    FunctionCallError::RespondToModel(format!("{message}\nTaskSpaceGateRecoveryV1: {metadata}"))
}

fn hard_state_reason(message: &str) -> Option<&str> {
    message
        .split_once("hard_state:")?
        .1
        .trim_start()
        .split(|character: char| character.is_whitespace() || ".,;".contains(character))
        .next()
        .filter(|reason| !reason.is_empty())
}

#[cfg(test)]
#[path = "taskspace_control_tests.rs"]
mod tests;
