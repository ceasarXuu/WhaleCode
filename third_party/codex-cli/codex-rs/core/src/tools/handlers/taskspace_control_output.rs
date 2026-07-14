use serde_json::Value as JsonValue;

use crate::action_map::ActionMapControlState;
#[cfg(test)]
use crate::action_map::ActionMapExpandedDetailRef;
use crate::action_map::ActionMapFinishNodeOutcome;
use crate::action_map::ActionMapInitializeOutcome;
use crate::action_map::ActionMapNodeDetailExpansionOutcome;
use crate::action_map::TaskSpaceHardGateClass;
use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StateCommit {
    None,
    Partial,
    Full,
}

impl StateCommit {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Partial => "partial",
            Self::Full => "full",
        }
    }
}

pub(super) fn format_initialize_step(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "kind": "map_initialized",
        "task_id": outcome.task_id,
        "map_id": outcome.map_id,
        "created_node_ids": outcome.node_ids,
        "current_node_id": outcome.current_node_id,
    })
}

pub(super) fn format_node_detail_expansion_step(
    index: usize,
    outcome: &ActionMapNodeDetailExpansionOutcome,
) -> JsonValue {
    let restored_details = outcome
        .restored_details
        .iter()
        .map(|detail| {
            serde_json::json!({
                "event_id": detail.event_id,
                "event_kind": detail.event_kind,
                "source": detail.source,
                "detail_tier": detail.detail_tier,
                "evidence_class": detail.evidence_class,
                "action_class": detail.action_class,
                "tool_success": detail.tool_success,
                "content_sha256": detail.content_sha256,
                "raw_ref": detail.raw_ref,
                "artifact_refs": detail.artifact_refs,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "kind": "node_detail_expanded",
        "index": index,
        "node_id": outcome.node_id,
        "expansion_event_id": outcome.expansion_event_id,
        "detail_ref": outcome.detail_ref,
        "restored_detail_count": restored_details.len(),
        "restored_details": restored_details,
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

pub(super) fn format_state_batch(
    steps: Vec<JsonValue>,
    success: bool,
    state_commit: StateCommit,
    map_state: Option<&ActionMapControlState>,
) -> String {
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": if success { "committed" } else { "state_machine_failed" },
        "success": success,
        "state_commit": state_commit.as_str(),
        "map_state": map_state.map(format_map_state),
        "steps": steps,
    })
    .to_string()
}

pub(super) fn state_commit_for_steps(steps: &[JsonValue], success: bool) -> StateCommit {
    if success {
        return StateCommit::Full;
    }
    if steps.iter().any(|step| {
        step.get("kind").and_then(JsonValue::as_str) == Some("state_transition")
            && step.get("success") != Some(&JsonValue::Bool(false))
    }) {
        StateCommit::Partial
    } else {
        StateCommit::None
    }
}

pub(super) fn control_state_observation(message: &str) -> Option<(String, usize, usize, bool)> {
    let value = serde_json::from_str::<JsonValue>(message).ok()?;
    let state_commit = value.get("state_commit")?.as_str()?;
    let map_state = value.get("map_state")?.as_object()?;
    Some((
        state_commit.to_string(),
        map_state.get("open_node_ids")?.as_array()?.len(),
        map_state.get("blocked_node_ids")?.as_array()?.len(),
        !map_state.get("current_node_id")?.is_null(),
    ))
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

pub(super) fn state_machine_error(message: String) -> FunctionCallError {
    let reason = hard_state_reason(&message)
        .unwrap_or("transition_rejected")
        .to_string();
    gate_error(message, TaskSpaceHardGateClass::StateMachine, &reason)
}

pub(super) fn protocol_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Protocol, reason)
}

pub(super) fn resource_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Resource, reason)
}

fn gate_error(message: String, class: TaskSpaceHardGateClass, reason: &str) -> FunctionCallError {
    let result = serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": format!("{}_failed", class.as_str()),
        "success": false,
        "error": {
            "class": class.as_str(),
            "code": reason,
            "message": message,
        },
    });
    FunctionCallError::RespondToModel(result.to_string())
}

fn format_map_state(state: &ActionMapControlState) -> JsonValue {
    serde_json::json!({
        "task_id": state.task_id,
        "task_status": state.task_status,
        "map_id": state.map_id,
        "map_status": state.map_status,
        "current_node_id": state.current_node_id,
        "pending_node_ids": state.pending_node_ids,
        "open_node_ids": state.open_node_ids,
        "blocked_node_ids": state.blocked_node_ids,
        "completed_node_count": state.completed_node_count,
        "total_node_count": state.total_node_count,
    })
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
        Some("node_created") => has_text(step, "node_id"),
        Some("node_bound") => has_text(step, "current_node_id"),
        Some("node_blocked") => has_text(step, "node_id") && has_text(step, "result_id"),
        Some("node_detail_expanded") => {
            has_text(step, "node_id")
                && has_text(step, "expansion_event_id")
                && has_text(step, "detail_ref")
                && step
                    .get("restored_details")
                    .and_then(JsonValue::as_array)
                    .is_some_and(|details| !details.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_step_returns_hidden_event_refs_without_duplicate_hash_field() {
        let outcome = ActionMapNodeDetailExpansionOutcome {
            node_id: "node-1".into(),
            expansion_event_id: "node-event-expand".into(),
            detail_ref: "taskspace-detail://sha256/abc".into(),
            restored_details: vec![ActionMapExpandedDetailRef {
                event_id: "node-event-read".into(),
                event_kind: "tool_result".into(),
                source: "main_tool".into(),
                detail_tier: "D3".into(),
                evidence_class: "P1".into(),
                action_class: Some("read".into()),
                tool_success: Some(true),
                content_sha256: Some("def".into()),
                raw_ref: Some("output-ref-1".into()),
                artifact_refs: vec!["src/lib.rs".into()],
            }],
        };

        let step = format_node_detail_expansion_step(0, &outcome);

        assert_eq!(step["restored_detail_count"], 1);
        assert_eq!(step["restored_details"][0]["event_id"], "node-event-read");
        assert_eq!(step["restored_details"][0]["raw_ref"], "output-ref-1");
        assert!(step.get("detail_sha256").is_none());
        assert!(step_has_required_identity(&step));
    }
}
