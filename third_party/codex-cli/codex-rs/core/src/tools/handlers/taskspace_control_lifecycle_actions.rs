use crate::function_tool::FunctionCallError;
use crate::session::FinishActionMapError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::handlers::taskspace_control_output::action_failure_error;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::rejected_control_result;

use super::ControlExecution;

pub(super) async fn finish_map(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    expected_revision: u64,
    finish_node_id: String,
    exact_summary: String,
) -> Result<ControlExecution, FunctionCallError> {
    let outcome = session
        .finish_action_map(
            turn,
            expected_revision,
            finish_node_id.clone(),
            exact_summary,
            call_id.to_string(),
        )
        .await;
    match outcome {
        Ok(outcome) => {
            tracing::info!(
                target: "codex_core::taskspace",
                event_name = "taskspace.finish_map_committed",
                call_id,
                map_id = outcome.map_id,
                revision = outcome.revision,
                finish_node_id,
                summary_bytes = outcome.final_summary.len(),
                "committed Agent-declared TaskSpace terminal"
            );
            Ok(terminal_execution(outcome))
        }
        Err(FinishActionMapError::Rejected(error)) => {
            tracing::warn!(
                target: "codex_core::taskspace",
                event_name = "taskspace.finish_map_rejected",
                call_id,
                expected_revision,
                finish_node_id,
                state_commit = false,
                "rejected Agent-declared TaskSpace terminal"
            );
            Ok((rejected_control_result(&error), false, None))
        }
        Err(FinishActionMapError::Persistence(error)) => {
            let canonical_revision = session
                .action_map_control_state(None)
                .await
                .map(|state| state.revision);
            Err(action_failure_error(
                "finish_map",
                Some(expected_revision),
                canonical_revision,
                "resource",
                "TASKSPACE_RESOURCE_FAILURE",
                "resource_failed",
                error,
            ))
        }
    }
}

fn terminal_execution(outcome: crate::action_map::ActionMapTerminalOutcome) -> ControlExecution {
    (
        format_state_batch(
            vec![serde_json::json!({
                "kind": "finish_map",
                "finish_node_id": outcome.terminal_node_id,
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
