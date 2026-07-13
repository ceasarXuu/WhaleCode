use serde_json::Value as JsonValue;

use crate::action_map::ActionMapNextNodeDraft;
use crate::action_map::NodeKind;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNextArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceNonterminalFinishArgs;
use crate::tools::handlers::taskspace_control_output::StateCommit;
use crate::tools::handlers::taskspace_control_output::format_failed_state_step;
use crate::tools::handlers::taskspace_control_output::format_state_batch;
use crate::tools::handlers::taskspace_control_output::format_terminal_chain_steps;
use crate::tools::handlers::taskspace_control_output::protocol_error;
use crate::tools::handlers::taskspace_control_output::state_machine_error;

pub(super) fn parse_node_kind(field: &str, value: &str) -> Result<NodeKind, FunctionCallError> {
    NodeKind::from_str(value).ok_or_else(|| {
        protocol_error(
            format!("taskspace_control {field} has invalid node kind `{value}`"),
            "invalid_argument_value",
        )
    })
}

pub(super) async fn execute_nonterminal_finishes(
    session: &Session,
    turn: &TurnContext,
    finishes: Vec<TaskSpaceNonterminalFinishArgs>,
    conclusion_event_id: &str,
) -> (Vec<JsonValue>, bool) {
    let mut steps = Vec::with_capacity(finishes.len());
    for finish in finishes {
        let index = steps.len();
        match execute_nonterminal_finish(session, turn, finish, conclusion_event_id, index).await {
            Ok(step) => steps.push(step),
            Err(error) => {
                steps.push(format_failed_state_step(steps.len(), &error));
                return (steps, false);
            }
        }
    }
    (steps, true)
}

pub(super) async fn execute_terminal_finish_chain(
    session: &Session,
    turn: &TurnContext,
    node_ids: Vec<String>,
    final_candidate: &str,
    conclusion_event_id: &str,
) -> Result<Vec<JsonValue>, FunctionCallError> {
    let outcomes = session
        .finish_action_map_node_chain_with_terminal_candidate(
            turn,
            &node_ids,
            conclusion_event_id.to_string(),
            final_candidate,
        )
        .await
        .map_err(state_machine_error)?;
    format_terminal_chain_steps(outcomes)
}

pub(super) async fn execute_create_node(
    session: &Session,
    turn: &TurnContext,
    kind: String,
    goal: String,
    dependency_node_ids: Vec<String>,
    bind_current: bool,
) -> Result<(String, bool), FunctionCallError> {
    let kind = parse_node_kind("kind", &kind)?;
    let result = session
        .create_action_map_node_for_main_with_kind(
            turn,
            kind,
            goal.clone(),
            goal,
            dependency_node_ids,
            bind_current,
        )
        .await;
    let map_state = session.action_map_control_state(None).await;
    Ok(match result {
        Ok(node_id) => (
            format_state_batch(
                vec![serde_json::json!({
                    "kind": "node_created",
                    "node_id": node_id,
                    "bound_current": bind_current,
                })],
                true,
                StateCommit::Full,
                map_state.as_ref(),
            ),
            true,
        ),
        Err(message) => rejected_state_result(message, map_state.as_ref()),
    })
}

pub(super) async fn execute_bind_node(
    session: &Session,
    turn: &TurnContext,
    node_id: String,
) -> (String, bool) {
    let result = session.bind_action_map_main_node(turn, &node_id).await;
    let map_state = session.action_map_control_state(None).await;
    match result {
        Ok(()) => (
            format_state_batch(
                vec![serde_json::json!({
                    "kind": "node_bound",
                    "current_node_id": node_id,
                })],
                true,
                StateCommit::Full,
                map_state.as_ref(),
            ),
            true,
        ),
        Err(message) => rejected_state_result(message, map_state.as_ref()),
    }
}

pub(super) async fn execute_block_node(
    session: &Session,
    turn: &TurnContext,
    call_id: &str,
    node_id: String,
) -> Result<(String, bool), FunctionCallError> {
    let conclusion_event_id = session
        .taskspace_event_id_for_call(call_id)
        .await
        .map_err(state_machine_error)?;
    let result = session
        .block_action_map_main_node(turn, &node_id, conclusion_event_id)
        .await;
    let map_state = session.action_map_control_state(None).await;
    Ok(match result {
        Ok(result_id) => (
            format_state_batch(
                vec![serde_json::json!({
                    "kind": "node_blocked",
                    "node_id": node_id,
                    "result_id": result_id,
                })],
                true,
                StateCommit::Full,
                map_state.as_ref(),
            ),
            true,
        ),
        Err(message) => rejected_state_result(message, map_state.as_ref()),
    })
}

async fn execute_nonterminal_finish(
    session: &Session,
    turn: &TurnContext,
    finish: TaskSpaceNonterminalFinishArgs,
    conclusion_event_id: &str,
    index: usize,
) -> Result<JsonValue, FunctionCallError> {
    let (requested_next_node_id, draft, next_kind) = match finish.next {
        TaskSpaceNextArgs::Existing { node_id } => (Some(node_id), None, "existing"),
        TaskSpaceNextArgs::Create {
            node_kind,
            goal,
            dependency_node_ids,
        } => (
            None,
            Some(build_next_node_draft(node_kind, goal, dependency_node_ids)?),
            "created",
        ),
    };
    let (finished_node_id, outcome) = session
        .finish_action_map_current_or_named_node_with_next(
            turn,
            finish.node_id.as_deref(),
            conclusion_event_id.to_string(),
            requested_next_node_id,
            draft,
        )
        .await
        .map_err(state_machine_error)?;
    let next_node_id = outcome.next_node_id.ok_or_else(|| {
        protocol_error(
            "TaskSpace committed a nonterminal finish without a next node identity".into(),
            "missing_committed_identity",
        )
    })?;
    Ok(serde_json::json!({
        "kind": "state_transition",
        "index": index,
        "finished_node_id": finished_node_id,
        "result_id": outcome.result_id,
        "next": {"kind": next_kind, "node_id": next_node_id},
        "current_node_id": next_node_id,
    }))
}

fn build_next_node_draft(
    kind: String,
    goal: String,
    dependency_node_ids: Vec<String>,
) -> Result<ActionMapNextNodeDraft, FunctionCallError> {
    let kind = parse_node_kind("next.node_kind", &kind)?;
    if goal.trim().is_empty() {
        return Err(protocol_error(
            "finish next-node creation requires a non-empty goal".into(),
            "missing_argument",
        ));
    }
    Ok(ActionMapNextNodeDraft {
        kind,
        title: goal.clone(),
        context_summary: goal,
        dependency_node_ids,
    })
}

fn rejected_state_result(
    message: String,
    map_state: Option<&crate::action_map::ActionMapControlState>,
) -> (String, bool) {
    let error = state_machine_error(message);
    (
        format_state_batch(
            vec![format_failed_state_step(0, &error)],
            false,
            StateCommit::None,
            map_state,
        ),
        false,
    )
}
