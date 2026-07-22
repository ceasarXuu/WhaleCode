use crate::action_map::ActionMapGraphMutationInput;
use crate::action_map::ActionMapInitializeInput;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::taskspace_control_args::TaskSpaceFinishIdentityArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphEdgeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphNodeArgs;
use crate::tools::handlers::taskspace_control_output::action_failure_error;
use crate::tools::handlers::taskspace_control_output::format_initialize_binding_step;
use crate::tools::handlers::taskspace_control_output::format_initialize_step;
use crate::tools::handlers::taskspace_control_output::format_node_detail_expansion_step;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::rejected_control_result;

use super::ControlExecution;
use super::mapping::map_edge_input;
use super::mapping::map_finish_identity_input;
use super::mapping::map_node_input;

pub(super) async fn initialize_map(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    root: TaskSpaceGraphNodeArgs,
    initial_work_node: TaskSpaceGraphNodeArgs,
    finish_identity: TaskSpaceFinishIdentityArgs,
    additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
    edges: Vec<TaskSpaceGraphEdgeArgs>,
) -> Result<ControlExecution, FunctionCallError> {
    let source_event_ids = session
        .taskspace_initialization_source_event_ids(call_id)
        .await
        .map_err(|message| {
            action_failure_error(
                "initialize_map",
                None,
                None,
                "state_machine",
                "TASKSPACE_LIFECYCLE_INVARIANT",
                "state_machine_failed",
                message,
            )
        })?;
    let outcome = session
        .initialize_action_map_for_main(
            turn,
            ActionMapInitializeInput {
                root: map_node_input(root),
                current_work_node: map_node_input(initial_work_node),
                finish: map_finish_identity_input(finish_identity),
                work_nodes: additional_work_nodes
                    .into_iter()
                    .map(map_node_input)
                    .collect(),
                edges: edges.into_iter().map(map_edge_input).collect(),
                source_event_ids,
            },
        )
        .await;
    Ok(match outcome {
        Ok(outcome) => (
            format_state_batch(
                vec![
                    format_initialize_step(&outcome),
                    format_initialize_binding_step(&outcome),
                ],
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
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn mutate_graph(
    session: &Session,
    turn: &TurnContext,
    _call_id: &str,
    expected_revision: u64,
    add_nodes: Vec<TaskSpaceGraphNodeArgs>,
    add_edges: Vec<TaskSpaceGraphEdgeArgs>,
    remove_edges: Vec<TaskSpaceGraphEdgeArgs>,
) -> Result<ControlExecution, FunctionCallError> {
    let outcome = session
        .mutate_action_map_graph(
            turn,
            ActionMapGraphMutationInput {
                expected_revision,
                add_nodes: add_nodes.into_iter().map(map_node_input).collect(),
                add_edges: add_edges.into_iter().map(map_edge_input).collect(),
                remove_edges: remove_edges.into_iter().map(map_edge_input).collect(),
            },
        )
        .await;
    Ok(match outcome {
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
    })
}

pub(super) async fn expand_nodes(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    node_ids: Vec<String>,
) -> Result<ControlExecution, FunctionCallError> {
    let source_event_id =
        session
            .taskspace_event_id_for_call(call_id)
            .await
            .map_err(|message| {
                action_failure_error(
                    "expand_nodes",
                    None,
                    None,
                    "state_machine",
                    "TASKSPACE_LIFECYCLE_INVARIANT",
                    "state_machine_failed",
                    message,
                )
            })?;
    let requested_node_count = node_ids.len();
    let outcome = session
        .expand_action_map_node_details(turn, node_ids, call_id.to_string(), source_event_id)
        .await;
    Ok(match outcome {
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
                .map(format_node_detail_expansion_step)
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
    })
}
