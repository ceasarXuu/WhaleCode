use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde_json::Value as JsonValue;

use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::handlers::taskspace_control_args::with_argument_error_canonical_revision;
use crate::tools::handlers::taskspace_control_output::control_commit_observation;
use crate::tools::handlers::taskspace_control_output::normalize_control_result;
use crate::tools::handlers::taskspace_control_output::protocol_error;
use crate::tools::handlers::taskspace_control_output::state_identity_coverage;
use crate::tools::handlers::taskspace_transition_args::TaskSpaceActionArgs;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

#[path = "taskspace_control_graph_actions.rs"]
mod graph_actions;
#[path = "taskspace_control_lifecycle_actions.rs"]
mod lifecycle_actions;
#[path = "taskspace_control_mapping.rs"]
mod mapping;
#[path = "taskspace_control_read_actions.rs"]
mod read_actions;

#[cfg(test)]
use mapping::control_state_has_active_binding;

pub struct TaskSpaceControlHandler;

pub struct TaskSpaceControlOutput {
    message: String,
    success: bool,
    terminal_carrier: Option<TaskSpaceTerminalCarrier>,
}

pub(super) type ControlExecution = (String, bool, Option<TaskSpaceTerminalCarrier>);

pub(crate) struct TaskSpaceTransitionExecution {
    pub(crate) message: String,
    pub(crate) success: bool,
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
                let canonical_revision = session
                    .action_map_control_state(None)
                    .await
                    .map(|state| state.revision);
                return Err(with_argument_error_canonical_revision(
                    error,
                    canonical_revision,
                ));
            }
        };
        let action = args.action_name();
        let submitted_expected_revision = args.submitted_expected_revision();

        let (message, success, terminal_carrier) =
            execute_action(&session, &turn, &call_id, args).await?;
        let canonical_revision = session
            .action_map_control_state(None)
            .await
            .map(|state| state.revision);
        let message = normalize_control_result(
            message,
            action,
            submitted_expected_revision,
            canonical_revision,
            success,
        );
        log_control_result(&call_id, &message, success);
        Ok(TaskSpaceControlOutput {
            message,
            success,
            terminal_carrier,
        })
    }
}

async fn execute_action(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    args: TaskSpaceControlArgs,
) -> Result<ControlExecution, FunctionCallError> {
    match args {
        TaskSpaceControlArgs::MutateGraph {
            expected_revision,
            add_nodes,
            add_edges,
            remove_edges,
        } => {
            graph_actions::mutate_graph(
                session,
                turn,
                call_id,
                expected_revision,
                add_nodes,
                add_edges,
                remove_edges,
            )
            .await
        }
        TaskSpaceControlArgs::BlockNode {
            expected_revision,
            node_id,
        } => {
            lifecycle_actions::block_node(session, turn, call_id, expected_revision, node_id).await
        }
        TaskSpaceControlArgs::UnblockNode {
            expected_revision,
            node_id,
        } => {
            lifecycle_actions::unblock_node(session, turn, call_id, expected_revision, node_id)
                .await
        }
        TaskSpaceControlArgs::ReworkNode {
            expected_revision,
            node_id,
        } => {
            lifecycle_actions::rework_node(session, turn, call_id, expected_revision, node_id).await
        }
        TaskSpaceControlArgs::CompleteThenEnd {
            expected_revision,
            current_node_id,
            final_summary,
        } => {
            lifecycle_actions::complete_then_end(
                session,
                turn,
                call_id,
                expected_revision,
                current_node_id,
                final_summary,
            )
            .await
        }
        TaskSpaceControlArgs::CloseReadyFinish {
            expected_revision,
            final_summary,
        } => {
            lifecycle_actions::close_ready_finish(session, turn, expected_revision, final_summary)
                .await
        }
        TaskSpaceControlArgs::ExpandNodes { node_ids } => {
            graph_actions::expand_nodes(session, turn, call_id, node_ids).await
        }
        TaskSpaceControlArgs::ReadOutputRef {
            output_ref,
            mode,
            start_line,
            end_line,
            pattern,
            max_bytes,
        } => {
            read_actions::read_output_ref(
                session,
                turn,
                output_ref,
                mode,
                start_line,
                end_line,
                pattern,
                max_bytes.expect("validated read_output_ref max_bytes"),
            )
            .await
        }
        TaskSpaceControlArgs::ReadMap => read_actions::read_map(session, turn, call_id).await,
    }
}

pub(crate) async fn execute_taskspace_transition(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    args: TaskSpaceActionArgs,
) -> Result<TaskSpaceTransitionExecution, FunctionCallError> {
    let action = args.action_name();
    let submitted_expected_revision = args.submitted_expected_revision();
    let (message, success, _) = match args {
        TaskSpaceActionArgs::ContinueCurrent { .. } => {
            return Err(FunctionCallError::RespondToModel(
                "continue_current is a binding assertion, not a lifecycle transition".into(),
            ));
        }
        TaskSpaceActionArgs::InitializeMap {
            root,
            initial_work_node,
            finish_identity,
            additional_work_nodes,
            edges,
        } => {
            graph_actions::initialize_map(
                session,
                turn,
                call_id,
                root,
                initial_work_node,
                finish_identity,
                additional_work_nodes,
                edges,
            )
            .await?
        }
        TaskSpaceActionArgs::BindNode {
            expected_revision,
            node_id,
        } => {
            lifecycle_actions::bind_node(session, turn, call_id, expected_revision, node_id).await?
        }
        TaskSpaceActionArgs::CompleteThenContinue {
            expected_revision,
            current_node_id,
            next_node_id,
        } => {
            lifecycle_actions::complete_then_continue(
                session,
                turn,
                call_id,
                expected_revision,
                current_node_id,
                next_node_id,
            )
            .await?
        }
    };
    let canonical_revision = session
        .action_map_control_state(None)
        .await
        .map(|state| state.revision);
    let message = normalize_control_result(
        message,
        action,
        submitted_expected_revision,
        canonical_revision,
        success,
    );
    log_control_result(call_id, &message, success);
    Ok(TaskSpaceTransitionExecution { message, success })
}

fn log_control_result(call_id: &str, message: &str, success: bool) {
    if let Some((step_count, identity_complete)) = state_identity_coverage(message) {
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
    if let Some((state_commit, revision, graph_refs, detail_refs)) =
        control_commit_observation(message)
    {
        tracing::info!(
            target: "codex_core::taskspace",
            call_id,
            state_commit,
            committed_revision = revision,
            graph_event_ref_count = graph_refs,
            node_detail_event_ref_count = detail_refs,
            "taskspace.control_delta_exposed"
        );
    }
}

#[cfg(test)]
#[path = "taskspace_control_tests.rs"]
mod tests;
