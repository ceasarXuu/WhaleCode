use serde_json::Value as JsonValue;

use crate::action_map::ActionMapFinishNodeOutcome;
use crate::action_map::ActionMapInitializeOutcome;
use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control::protocol_error;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

pub(super) fn format_initialize_step(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "kind": "map_initialized",
        "task_id": outcome.task_id,
        "map_id": outcome.map_id,
        "created_node_ids": outcome.node_ids,
        "current_node_id": outcome.current_node_id,
    })
}

pub(super) fn format_failed_state_step(index: usize, error: &FunctionCallError) -> JsonValue {
    let typed_result = serde_json::from_str::<JsonValue>(&error.to_string())
        .unwrap_or_else(|_| serde_json::json!({"message": error.to_string()}));
    let typed_error = typed_result.get("error").cloned().unwrap_or(typed_result);
    serde_json::json!({
        "kind": "state_transition",
        "index": index,
        "success": false,
        "error": typed_error,
    })
}

pub(super) fn format_state_batch(steps: Vec<JsonValue>, success: bool) -> String {
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": if success { "committed" } else { "state_machine_failed" },
        "success": success,
        "steps": steps,
    })
    .to_string()
}

pub(super) fn format_terminal_chain_steps(
    outcomes: Vec<(String, ActionMapFinishNodeOutcome)>,
) -> Result<Vec<JsonValue>, FunctionCallError> {
    let step_count = outcomes.len();
    outcomes
        .into_iter()
        .enumerate()
        .map(|(index, (finished_node_id, outcome))| {
            if index + 1 == step_count {
                Ok(serde_json::json!({
                    "kind": "terminal_transition",
                    "index": index,
                    "finished_node_id": finished_node_id,
                    "result_id": outcome.result_id,
                    "map_status": "completed",
                    "task_status": "completed",
                    "current_node_id": JsonValue::Null,
                }))
            } else {
                let next_node_id = outcome.next_node_id.ok_or_else(|| {
                    protocol_error(
                        "TaskSpace committed a terminal chain step without a next node identity"
                            .into(),
                        "missing_committed_identity",
                    )
                })?;
                Ok(serde_json::json!({
                    "kind": "state_transition",
                    "index": index,
                    "finished_node_id": finished_node_id,
                    "result_id": outcome.result_id,
                    "next": {"kind": "existing", "node_id": next_node_id},
                    "current_node_id": next_node_id,
                }))
            }
        })
        .collect()
}

pub(super) fn state_identity_coverage(message: &str) -> Option<(usize, bool)> {
    let value = serde_json::from_str::<JsonValue>(message).ok()?;
    if value.get("schema_version")?.as_str()? != TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION {
        return None;
    }
    let steps = value.get("steps")?.as_array()?;
    Some((steps.len(), steps.iter().all(step_has_required_identity)))
}

pub(super) fn hard_state_reason(message: &str) -> Option<&str> {
    message
        .split_once("hard_state:")?
        .1
        .trim_start()
        .split(|character: char| character.is_whitespace() || ".,;".contains(character))
        .next()
        .filter(|reason| !reason.is_empty())
}

fn step_has_required_identity(step: &JsonValue) -> bool {
    match step.get("kind").and_then(JsonValue::as_str) {
        Some("map_initialized") => {
            has_text(step, "task_id")
                && has_text(step, "map_id")
                && step
                    .get("created_node_ids")
                    .and_then(JsonValue::as_array)
                    .is_some_and(|ids| !ids.is_empty())
                && has_text(step, "current_node_id")
        }
        Some("state_transition") if step.get("success") == Some(&JsonValue::Bool(false)) => true,
        Some("state_transition") => {
            has_text(step, "finished_node_id")
                && has_text(step, "result_id")
                && has_text(step.get("next").unwrap_or(&JsonValue::Null), "node_id")
                && has_text(step, "current_node_id")
        }
        Some("terminal_transition") => {
            has_text(step, "finished_node_id")
                && has_text(step, "result_id")
                && step.get("current_node_id") == Some(&JsonValue::Null)
        }
        Some("ordinary_tool") => has_text(step, "call_id") && has_text(step, "output_event_ref"),
        _ => false,
    }
}

fn has_text(value: &JsonValue, field: &str) -> bool {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .is_some_and(|text| !text.is_empty())
}
