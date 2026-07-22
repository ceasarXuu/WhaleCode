use crate::action_map::NodeTransition;
use crate::function_tool::FunctionCallError;
use crate::session::FinishActionMapError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::handlers::taskspace_control_output::action_failure_error;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::rejected_control_result;

use super::ControlExecution;

pub(super) async fn bind_node(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    node_id: String,
) -> Result<ControlExecution, FunctionCallError> {
    execute_node_transition(
        session,
        turn,
        call_id,
        expected_revision,
        node_id,
        NodeTransition::Bind,
        "bind_node",
        "node_bound",
    )
    .await
}

pub(super) async fn block_node(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    node_id: String,
) -> Result<ControlExecution, FunctionCallError> {
    execute_node_transition(
        session,
        turn,
        call_id,
        expected_revision,
        node_id,
        NodeTransition::Block,
        "block_node",
        "node_blocked",
    )
    .await
}

pub(super) async fn unblock_node(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    node_id: String,
) -> Result<ControlExecution, FunctionCallError> {
    execute_node_transition(
        session,
        turn,
        call_id,
        expected_revision,
        node_id,
        NodeTransition::Unblock,
        "unblock_node",
        "node_unblocked",
    )
    .await
}

pub(super) async fn rework_node(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    node_id: String,
) -> Result<ControlExecution, FunctionCallError> {
    execute_node_transition(
        session,
        turn,
        call_id,
        expected_revision,
        node_id,
        NodeTransition::Rework,
        "rework_node",
        "node_reworked",
    )
    .await
}

pub(super) async fn complete_then_continue(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    current_node_id: String,
    next_node_id: String,
) -> Result<ControlExecution, FunctionCallError> {
    let source_event_ref = source_event_ref(
        session,
        call_id,
        "complete_then_continue",
        expected_revision,
    )
    .await?;
    let outcome = session
        .complete_then_bind_action_map_node(
            turn,
            expected_revision,
            current_node_id.clone(),
            next_node_id.clone(),
            source_event_ref,
        )
        .await;
    Ok(match outcome {
        Ok(outcome) => {
            tracing::info!(
                target: "codex_core::taskspace",
                call_id,
                map_id = outcome.map_id,
                revision = outcome.revision,
                current_node_id,
                next_node_id,
                "taskspace.complete_handoff_committed"
            );
            (
                format_state_batch(
                    vec![serde_json::json!({
                        "kind": "complete_then_continue",
                        "map_id": outcome.map_id,
                        "current_node_id": outcome.current_node_id,
                        "next_node_id": outcome.next_node_id,
                        "revision": outcome.revision,
                    })],
                    true,
                    true,
                    &[&outcome.delta],
                ),
                true,
                None,
            )
        }
        Err(error) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id,
                expected_revision,
                current_node_id,
                next_node_id,
                "taskspace.complete_handoff_rejected"
            );
            (rejected_control_result(&error), false, None)
        }
    })
}

pub(super) async fn finish_map(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    terminal_node_id: String,
    final_summary: String,
) -> Result<ControlExecution, FunctionCallError> {
    let source_event_ref =
        source_event_ref(session, call_id, "finish_map", expected_revision).await?;
    let outcome = session
        .finish_action_map(
            turn,
            expected_revision,
            terminal_node_id.clone(),
            final_summary,
            source_event_ref,
        )
        .await;
    match outcome {
        Ok(outcome) => {
            tracing::info!(
                target: "codex_core::taskspace",
                call_id,
                map_id = outcome.map_id,
                revision = outcome.revision,
                terminal_node_id,
                summary_bytes = outcome.final_summary.len(),
                terminal_node_role = outcome.terminal_node_role,
                "taskspace.finish_map_committed"
            );
            Ok(terminal_execution(outcome))
        }
        Err(FinishActionMapError::Rejected(error)) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                call_id,
                expected_revision,
                terminal_node_id,
                "taskspace.finish_map_rejected"
            );
            Ok((rejected_control_result(&error), false, None))
        }
        Err(error) => Err(terminal_failure(session, "finish_map", expected_revision, error).await),
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_node_transition(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    node_id: String,
    transition: NodeTransition,
    action: &'static str,
    step_kind: &'static str,
) -> Result<ControlExecution, FunctionCallError> {
    let source_event_ref = source_event_ref(session, call_id, action, expected_revision).await?;
    let outcome = session
        .transition_action_map_node(
            turn,
            expected_revision,
            node_id,
            transition,
            source_event_ref,
        )
        .await;
    Ok(match outcome {
        Ok(outcome) => (
            format_state_batch(
                vec![serde_json::json!({
                    "kind": step_kind,
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
    })
}

async fn source_event_ref(
    session: &Session,
    call_id: &str,
    action: &'static str,
    expected_revision: u64,
) -> Result<String, FunctionCallError> {
    session
        .taskspace_event_id_for_call(call_id)
        .await
        .map_err(|message| {
            action_failure_error(
                action,
                Some(expected_revision),
                None,
                "state_machine",
                "TASKSPACE_LIFECYCLE_INVARIANT",
                "state_machine_failed",
                message,
            )
        })
}

fn terminal_execution(outcome: crate::action_map::ActionMapTerminalOutcome) -> ControlExecution {
    (
        format_state_batch(
            vec![serde_json::json!({
                "kind": "finish_map",
                "terminal_node_id": outcome.terminal_node_id,
                "terminal_node_role": outcome.terminal_node_role,
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
    )
}

async fn terminal_failure(
    session: &Session,
    action: &'static str,
    expected_revision: u64,
    error: FinishActionMapError,
) -> FunctionCallError {
    let canonical_revision = session
        .action_map_control_state(None)
        .await
        .map(|state| state.revision);
    match error {
        FinishActionMapError::Persistence(error) => action_failure_error(
            action,
            Some(expected_revision),
            canonical_revision,
            "resource",
            "TASKSPACE_RESOURCE_FAILURE",
            "resource_failed",
            error,
        ),
        FinishActionMapError::Internal(error) => action_failure_error(
            action,
            Some(expected_revision),
            canonical_revision,
            "protocol",
            "TASKSPACE_PROTOCOL_FAILURE",
            "protocol_failed",
            error,
        ),
        FinishActionMapError::Rejected(error) => {
            debug_assert!(
                false,
                "rejected finish must be handled before terminal_failure"
            );
            action_failure_error(
                action,
                Some(expected_revision),
                canonical_revision,
                "state_machine",
                "TASKSPACE_LIFECYCLE_INVARIANT",
                "state_machine_failed",
                error,
            )
        }
    }
}
