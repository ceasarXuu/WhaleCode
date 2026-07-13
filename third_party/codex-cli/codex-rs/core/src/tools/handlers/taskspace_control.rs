use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::handlers::taskspace_control_lifecycle::execute_bind_node;
use crate::tools::handlers::taskspace_control_lifecycle::execute_block_node;
use crate::tools::handlers::taskspace_control_lifecycle::execute_create_node;
use crate::tools::handlers::taskspace_control_lifecycle::execute_nonterminal_finishes;
use crate::tools::handlers::taskspace_control_lifecycle::execute_terminal_finish_chain;
use crate::tools::handlers::taskspace_control_lifecycle::parse_node_kind;
use crate::tools::handlers::taskspace_control_output::StateCommit;
use crate::tools::handlers::taskspace_control_output::control_state_observation;
use crate::tools::handlers::taskspace_control_output::format_failed_state_step;
use crate::tools::handlers::taskspace_control_output::format_initialize_step;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::hard_state_reason;
use crate::tools::handlers::taskspace_control_output::protocol_error;
use crate::tools::handlers::taskspace_control_output::resource_error;
use crate::tools::handlers::taskspace_control_output::state_commit_for_steps;
use crate::tools::handlers::taskspace_control_output::state_identity_coverage;
use crate::tools::handlers::taskspace_control_output::state_machine_error;
use crate::tools::output_reference::OUTPUT_SLICE_MAX_BYTES;
use crate::tools::output_reference::OutputSliceMode;
use crate::tools::output_reference::OutputSliceRequest;
use crate::tools::output_reference::read_output_artifact_slice;
use crate::tools::output_reference::read_output_bytes_slice;
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
            call_id,
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
                initial_nodes,
                current_node_id,
                continuation: _,
            } => {
                let source_event_ids = session
                    .taskspace_initialization_source_event_ids(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                let nodes = initial_nodes
                    .into_iter()
                    .map(|node| {
                        let title = node.node_id.clone();
                        Ok(ActionMapInitializeNodeInput {
                            id: node.node_id,
                            kind: parse_node_kind("initial_nodes.kind", &node.kind)?,
                            title,
                            context_summary: node.goal,
                            dependency_node_ids: node.dependency_node_ids,
                        })
                    })
                    .collect::<Result<Vec<_>, FunctionCallError>>()?;
                let outcome = session
                    .initialize_action_map_for_main(
                        &turn,
                        ActionMapInitializeInput {
                            task_title: "TaskSpace task".into(),
                            source_event_ids,
                            nodes,
                            current_node_id,
                        },
                    )
                    .await
                    .map_err(state_machine_error)?;
                let map_state = session
                    .action_map_control_state(Some(&outcome.map_id))
                    .await;
                (
                    format_state_batch(
                        vec![format_initialize_step(&outcome)],
                        true,
                        StateCommit::Full,
                        map_state.as_ref(),
                    ),
                    true,
                    None,
                )
            }
            TaskSpaceControlArgs::FinishNodes { finishes } => {
                let conclusion_event_id = session
                    .taskspace_event_id_for_call(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                let (steps, success) =
                    execute_nonterminal_finishes(&session, &turn, finishes, &conclusion_event_id)
                        .await;
                let state_commit = state_commit_for_steps(&steps, success);
                let map_state = session.action_map_control_state(None).await;
                (
                    format_state_batch(steps, success, state_commit, map_state.as_ref()),
                    success,
                    None,
                )
            }
            TaskSpaceControlArgs::FinishThenEnd {
                finish_node_ids,
                final_candidate,
            } => {
                let conclusion_event_id = session
                    .taskspace_event_id_for_call(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                let declared_step_count = finish_node_ids.len();
                tracing::info!(
                    target: "codex_core::taskspace",
                    call_id,
                    declared_step_count,
                    "taskspace.terminal_finish_chain_declared"
                );
                let map_id_hint = session
                    .action_map_control_state(None)
                    .await
                    .map(|state| state.map_id);
                match execute_terminal_finish_chain(
                    &session,
                    &turn,
                    finish_node_ids,
                    &final_candidate,
                    &conclusion_event_id,
                )
                .await
                {
                    Ok(steps) => {
                        tracing::info!(
                            target: "codex_core::taskspace",
                            call_id,
                            committed_step_count = steps.len(),
                            "taskspace.terminal_finish_chain_committed"
                        );
                        let map_state = session
                            .action_map_control_state(map_id_hint.as_deref())
                            .await;
                        (
                            format_state_batch(steps, true, StateCommit::Full, map_state.as_ref()),
                            true,
                            Some(final_candidate),
                        )
                    }
                    Err(error) => {
                        let error_text = error.to_string();
                        let reason_code =
                            hard_state_reason(&error_text).unwrap_or("transition_rejected");
                        tracing::warn!(
                            target: "codex_core::taskspace",
                            call_id,
                            declared_step_count,
                            reason_code,
                            "taskspace.terminal_finish_chain_rejected"
                        );
                        let map_state = session
                            .action_map_control_state(map_id_hint.as_deref())
                            .await;
                        (
                            format_state_batch(
                                vec![format_failed_state_step(0, &error)],
                                false,
                                StateCommit::None,
                                map_state.as_ref(),
                            ),
                            false,
                            None,
                        )
                    }
                }
            }
            TaskSpaceControlArgs::CreateNode {
                kind,
                goal,
                dependency_node_ids,
                bind_current,
            } => {
                let (message, success) = execute_create_node(
                    &session,
                    &turn,
                    kind,
                    goal,
                    dependency_node_ids,
                    bind_current,
                )
                .await?;
                (message, success, None)
            }
            TaskSpaceControlArgs::BindNode { node_id } => {
                let (message, success) = execute_bind_node(&session, &turn, node_id).await;
                (message, success, None)
            }
            TaskSpaceControlArgs::BlockNode { node_id } => {
                let (message, success) =
                    execute_block_node(&session, &turn, &call_id, node_id).await?;
                (message, success, None)
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
                let slice = if let Some(bytes) = session
                    .action_map_projection_archive_bytes(&output_ref)
                    .await
                {
                    read_output_bytes_slice(&output_ref, &bytes, request)
                } else {
                    let rollout_path = session.current_rollout_path().await.map_err(|error| {
                        resource_error(error.to_string(), "output_reference_store_unavailable")
                    })?;
                    read_output_artifact_slice(rollout_path.as_deref(), &output_ref, request).await
                }
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
        if let Some((step_count, identity_complete)) = state_identity_coverage(&message) {
            if success {
                tracing::info!(
                    target: "codex_core::taskspace",
                    call_id,
                    step_count,
                    identity_complete,
                    "taskspace.control_state_committed"
                );
            } else {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    call_id,
                    step_count,
                    "taskspace.control_state_rejected"
                );
            }
        }
        if let Some((state_commit, open_node_count, blocked_node_count, has_current_node)) =
            control_state_observation(&message)
        {
            tracing::info!(
                target: "codex_core::taskspace",
                call_id,
                state_commit,
                open_node_count,
                blocked_node_count,
                has_current_node,
                "taskspace.control_map_state_exposed"
            );
        }
        Ok(TaskSpaceControlOutput {
            message,
            success,
            terminal_agent_message,
        })
    }
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

#[cfg(test)]
#[path = "taskspace_control_tests.rs"]
mod tests;
