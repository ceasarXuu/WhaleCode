use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapEdgeInput;
use crate::action_map::ActionMapGraphMutationInput;
use crate::action_map::ActionMapInitializeFinishInput;
use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::NodeTransition;
use crate::function_tool::FunctionCallError;
use crate::session::FinishActionMapError;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceFinishNodeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphEdgeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphNodeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNodeTransition;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::handlers::taskspace_control_output::control_commit_observation;
use crate::tools::handlers::taskspace_control_output::format_initialize_step;
use crate::tools::handlers::taskspace_control_output::format_node_detail_expansion_step;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::protocol_error;
use crate::tools::handlers::taskspace_control_output::rejected_control_result;
use crate::tools::handlers::taskspace_control_output::resource_error;
use crate::tools::handlers::taskspace_control_output::state_identity_coverage;
use crate::tools::handlers::taskspace_control_output::state_machine_error;
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
    terminal_carrier: Option<TaskSpaceTerminalCarrier>,
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

    fn taskspace_terminal_carrier(&self) -> Option<&TaskSpaceTerminalCarrier> {
        self.terminal_carrier.as_ref()
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
        let args = match parse_taskspace_control_args(&arguments) {
            Ok(args) => args,
            Err(error) => {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    call_id,
                    "taskspace.control_arguments_rejected"
                );
                return Err(error);
            }
        };

        let (message, success, terminal_carrier) = match args {
            TaskSpaceControlArgs::InitializeMap {
                root,
                initial_work_node,
                finish,
                additional_work_nodes,
                edges,
                continuation: _,
            } => {
                let source_event_ids = session
                    .taskspace_initialization_source_event_ids(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                let outcome = session
                    .initialize_action_map_for_main(
                        &turn,
                        ActionMapInitializeInput {
                            root: map_node_input(root),
                            current_work_node: map_node_input(initial_work_node),
                            finish: map_finish_input(finish),
                            work_nodes: additional_work_nodes
                                .into_iter()
                                .map(map_node_input)
                                .collect(),
                            edges: edges.into_iter().map(map_edge_input).collect(),
                            source_event_ids,
                        },
                    )
                    .await;
                match outcome {
                    Ok(outcome) => (
                        format_state_batch(
                            vec![format_initialize_step(&outcome)],
                            true,
                            true,
                            &[&outcome.delta],
                        ),
                        true,
                        None,
                    ),
                    Err(error) => {
                        tracing::warn!(
                            target: "codex_core::taskspace",
                            call_id,
                            "taskspace.map_initialization_rejected"
                        );
                        (rejected_control_result(&error), false, None)
                    }
                }
            }
            TaskSpaceControlArgs::MutateGraph {
                expected_revision,
                add_nodes,
                add_edges,
                remove_edges,
            } => match session
                .mutate_action_map_graph(
                    &turn,
                    ActionMapGraphMutationInput {
                        expected_revision,
                        add_nodes: add_nodes.into_iter().map(map_node_input).collect(),
                        add_edges: add_edges.into_iter().map(map_edge_input).collect(),
                        remove_edges: remove_edges.into_iter().map(map_edge_input).collect(),
                    },
                )
                .await
            {
                Ok(outcome) => (
                    format_state_batch(
                        vec![serde_json::json!({
                            "kind": "graph_mutation",
                            "map_id": outcome.map_id,
                            "revision": outcome.revision,
                        })],
                        true,
                        true,
                        &[&outcome.delta],
                    ),
                    true,
                    None,
                ),
                Err(error) => (rejected_control_result(&error), false, None),
            },
            TaskSpaceControlArgs::TransitionNode {
                expected_revision,
                node_id,
                transition,
            } => {
                let source_event_ref = session
                    .taskspace_event_id_for_call(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                match session
                    .transition_action_map_node(
                        &turn,
                        expected_revision,
                        node_id,
                        map_transition(transition),
                        source_event_ref,
                    )
                    .await
                {
                    Ok(outcome) => (
                        format_state_batch(
                            vec![serde_json::json!({
                                "kind": "node_transition",
                                "map_id": outcome.map_id,
                                "node_id": outcome.node_id,
                                "revision": outcome.revision,
                                "status": outcome.status,
                            })],
                            true,
                            true,
                            &[&outcome.delta],
                        ),
                        true,
                        None,
                    ),
                    Err(error) => (rejected_control_result(&error), false, None),
                }
            }
            TaskSpaceControlArgs::FinishEnd {
                expected_revision,
                final_summary,
            } => {
                match session
                    .finish_action_map(&turn, expected_revision, final_summary.clone())
                    .await
                {
                    Ok(outcome) => (
                        format_state_batch(
                            vec![serde_json::json!({
                                "kind": "terminal_transition",
                                "map_id": outcome.map_id,
                                "revision": outcome.revision,
                                "finish_closed": true,
                                "root_closed": true,
                            })],
                            true,
                            true,
                            &[&outcome.delta],
                        ),
                        true,
                        Some(TaskSpaceTerminalCarrier {
                            map_id: outcome.map_id,
                            revision: outcome.revision,
                            summary: outcome.final_summary,
                        }),
                    ),
                    Err(FinishActionMapError::Rejected(error)) => {
                        (rejected_control_result(&error), false, None)
                    }
                    Err(FinishActionMapError::Persistence(error)) => {
                        return Err(resource_error(error, "terminal_persistence_failed"));
                    }
                    Err(FinishActionMapError::Internal(error)) => {
                        return Err(protocol_error(error, "terminal_transaction_invalid"));
                    }
                }
            }
            TaskSpaceControlArgs::ExpandNodes { node_ids } => {
                let source_event_id = session
                    .taskspace_event_id_for_call(&call_id)
                    .await
                    .map_err(state_machine_error)?;
                let requested_node_count = node_ids.len();
                match session
                    .expand_action_map_node_details(
                        &turn,
                        node_ids,
                        call_id.clone(),
                        source_event_id,
                    )
                    .await
                {
                    Ok(outcomes) => {
                        let restored_detail_count = outcomes
                            .iter()
                            .map(|outcome| outcome.restored_details.len())
                            .sum::<usize>();
                        tracing::info!(
                            target: "codex_core::taskspace",
                            call_id,
                            requested_node_count,
                            committed_node_count = outcomes.len(),
                            restored_detail_count,
                            "taskspace.node_details_expanded"
                        );
                        let steps = outcomes
                            .iter()
                            .enumerate()
                            .map(|(index, outcome)| {
                                format_node_detail_expansion_step(index, outcome)
                            })
                            .collect();
                        let deltas = outcomes
                            .iter()
                            .map(|outcome| &outcome.delta)
                            .collect::<Vec<_>>();
                        (format_state_batch(steps, true, true, &deltas), true, None)
                    }
                    Err(error_message) => {
                        tracing::warn!(
                            target: "codex_core::taskspace",
                            call_id,
                            requested_node_count,
                            "taskspace.node_details_expansion_rejected"
                        );
                        (rejected_control_result(&error_message), false, None)
                    }
                }
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
        if let Some((
            state_commit,
            committed_revision,
            graph_event_ref_count,
            node_detail_event_ref_count,
        )) = control_commit_observation(&message)
        {
            tracing::info!(
                target: "codex_core::taskspace",
                call_id,
                state_commit,
                committed_revision,
                graph_event_ref_count,
                node_detail_event_ref_count,
                "taskspace.control_delta_exposed"
            );
        }
        Ok(TaskSpaceControlOutput {
            message,
            success,
            terminal_carrier,
        })
    }
}

fn map_node_input(node: TaskSpaceGraphNodeArgs) -> ActionMapInitializeNodeInput {
    ActionMapInitializeNodeInput {
        id: node.node_id,
        goal: node.goal,
    }
}
fn map_finish_input(node: TaskSpaceFinishNodeArgs) -> ActionMapInitializeFinishInput {
    ActionMapInitializeFinishInput { id: node.node_id }
}
fn map_edge_input(edge: TaskSpaceGraphEdgeArgs) -> ActionMapEdgeInput {
    ActionMapEdgeInput {
        from: edge.from,
        to: edge.to,
    }
}
fn map_transition(transition: TaskSpaceNodeTransition) -> NodeTransition {
    match transition {
        TaskSpaceNodeTransition::Bind => NodeTransition::Bind,
        TaskSpaceNodeTransition::Complete => NodeTransition::Complete,
        TaskSpaceNodeTransition::Block => NodeTransition::Block,
        TaskSpaceNodeTransition::Unblock => NodeTransition::Unblock,
        TaskSpaceNodeTransition::Rework => NodeTransition::Rework,
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
