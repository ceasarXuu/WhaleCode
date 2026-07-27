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
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

#[path = "taskspace_control_lifecycle_actions.rs"]
mod lifecycle_actions;
#[path = "taskspace_control_read_actions.rs"]
mod read_actions;

pub struct TaskSpaceControlHandler;

pub struct TaskSpaceControlOutput {
    message: String,
    success: bool,
    terminal_carrier: Option<TaskSpaceTerminalCarrier>,
}

pub(super) type ControlExecution = (String, bool, Option<TaskSpaceTerminalCarrier>);

impl TaskSpaceControlOutput {
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn success(&self) -> bool {
        self.success
    }
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
        execute_parsed_action(&session, &turn, &call_id, args).await
    }
}

#[deprecated(
    note = "A2-B1X removes ordinary Tool initialization carriers; response preflight owns initialize_and_execute"
)]
pub(crate) async fn execute_taskspace_initialization_binding(
    _session: &Session,
    _turn: &TurnContext,
    _call_id: &str,
    _arguments: &str,
) -> Result<TaskSpaceControlOutput, FunctionCallError> {
    Err(protocol_error(
        "ordinary Tool initialization carriers were removed by A2-B1X".into(),
        "taskspace_initialization_carrier_removed",
    ))
}

async fn execute_parsed_action(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    args: TaskSpaceControlArgs,
) -> Result<TaskSpaceControlOutput, FunctionCallError> {
    let action = args.action_name();
    let submitted_expected_revision = args.submitted_expected_revision();

    let execution = execute_action(session, turn, call_id, args).await?;
    finalize_control_output(
        session,
        call_id,
        action,
        submitted_expected_revision,
        execution,
    )
    .await
}

async fn finalize_control_output(
    session: &Session,
    call_id: &str,
    action: &str,
    submitted_expected_revision: Option<u64>,
    execution: ControlExecution,
) -> Result<TaskSpaceControlOutput, FunctionCallError> {
    let (message, success, terminal_carrier) = execution;
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
    Ok(TaskSpaceControlOutput {
        message,
        success,
        terminal_carrier,
    })
}

async fn execute_action(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    args: TaskSpaceControlArgs,
) -> Result<ControlExecution, FunctionCallError> {
    match args {
        // TODO(A2-B1X-response-preflight): these manifests must be consumed before
        // ToolHandler dispatch. Do not add a nested dispatcher or old Runtime fallback here.
        TaskSpaceControlArgs::InitializeAndExecute { .. }
        | TaskSpaceControlArgs::Execute { .. } => Err(protocol_error(
            "TaskSpace action manifests require complete-response preflight".into(),
            "taskspace_action_manifest_requires_response_preflight",
        )),
        TaskSpaceControlArgs::FinishMap {
            expected_revision,
            finish_node_id,
            exact_summary,
        } => {
            lifecycle_actions::finish_map(
                session,
                turn,
                call_id,
                expected_revision,
                finish_node_id,
                exact_summary,
            )
            .await
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
                session, turn, output_ref, mode, start_line, end_line, pattern, max_bytes,
            )
            .await
        }
        TaskSpaceControlArgs::ReadMap => read_actions::read_map(session, turn, call_id).await,
    }
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
