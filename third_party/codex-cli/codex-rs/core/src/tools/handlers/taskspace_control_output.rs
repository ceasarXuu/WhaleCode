use serde_json::Value as JsonValue;

use crate::action_map::ActionMapControlDelta;
#[cfg(test)]
use crate::action_map::ActionMapExpandedDetailRef;
use crate::action_map::ActionMapInitializeOutcome;
use crate::action_map::ActionMapNodeDetailExpansionOutcome;
use crate::action_map::TaskSpaceHardGateClass;
use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control_args::TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION;

pub(super) fn format_initialize_step(outcome: &ActionMapInitializeOutcome) -> JsonValue {
    serde_json::json!({
        "kind": "map_initialized",
        "map_id": outcome.map_id,
        "revision": outcome.delta.committed_revision,
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

pub(super) fn format_state_batch(
    steps: Vec<JsonValue>,
    success: bool,
    state_commit: bool,
    deltas: &[&ActionMapControlDelta],
) -> String {
    let delta = format_committed_delta(deltas);
    let committed_revision = delta
        .as_ref()
        .and_then(|value| value.get("committed_revision"))
        .cloned()
        .unwrap_or(JsonValue::Null);
    serde_json::json!({
        "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
        "status": if success { "committed" } else { "state_machine_failed" },
        "success": success,
        "state_commit": state_commit,
        "partial_commit": 0,
        "committed_revision": committed_revision,
        "delta": delta,
        "steps": steps,
    })
    .to_string()
}

pub(super) fn rejected_control_result(error: &str) -> String {
    let Ok(mut exact_error) = serde_json::from_str::<JsonValue>(error) else {
        return format_state_batch(
            vec![serde_json::json!({
                "kind": "state_rejection",
                "error": {"message": error},
            })],
            false,
            false,
            &[],
        );
    };
    let is_rooted_rejection = exact_error
        .get("schema_version")
        .and_then(JsonValue::as_str)
        == Some(TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION)
        && exact_error.get("status").and_then(JsonValue::as_str) == Some("state_machine_failed")
        && exact_error.get("state_commit").and_then(JsonValue::as_bool) == Some(false);
    if !is_rooted_rejection {
        return format_state_batch(
            vec![serde_json::json!({
                "kind": "state_rejection",
                "error": exact_error,
            })],
            false,
            false,
            &[],
        );
    }
    exact_error["partial_commit"] = JsonValue::from(0);
    exact_error["committed_revision"] = JsonValue::Null;
    exact_error["delta"] = JsonValue::Null;
    exact_error.to_string()
}

pub(super) fn control_commit_observation(
    message: &str,
) -> Option<(bool, Option<u64>, usize, usize)> {
    let value = serde_json::from_str::<JsonValue>(message).ok()?;
    let state_commit = value.get("state_commit")?.as_bool()?;
    let committed_revision = value.get("committed_revision").and_then(JsonValue::as_u64);
    let delta = value.get("delta").and_then(JsonValue::as_object);
    Some((
        state_commit,
        committed_revision,
        delta
            .and_then(|delta| delta.get("graph_event_refs"))
            .and_then(JsonValue::as_array)
            .map_or(0, Vec::len),
        delta
            .and_then(|delta| delta.get("node_detail_event_refs"))
            .and_then(JsonValue::as_array)
            .map_or(0, Vec::len),
    ))
}

fn format_committed_delta(deltas: &[&ActionMapControlDelta]) -> Option<JsonValue> {
    let first = deltas.first()?;
    let graph_event_refs = deltas
        .iter()
        .flat_map(|delta| delta.graph_revision_batches.iter())
        .flat_map(|batch| {
            batch
                .event_ids
                .iter()
                .zip(batch.events.iter())
                .map(|(event_id, event)| {
                    serde_json::json!({
                        "revision": batch.revision,
                        "event_id": event_id,
                        "event_type": event.get("type").and_then(JsonValue::as_str),
                    })
                })
        })
        .collect::<Vec<_>>();
    let node_detail_event_refs = deltas
        .iter()
        .flat_map(|delta| delta.node_detail_events.iter())
        .map(|event| {
            serde_json::json!({
                "node_id": &event.node_id,
                "expansion_event_id": &event.expansion_event_id,
            })
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        "map_id": &first.map_id,
        "committed_revision": deltas
            .iter()
            .map(|delta| delta.committed_revision)
            .max()
            .unwrap_or(first.committed_revision),
        "graph_event_refs": graph_event_refs,
        "node_detail_event_refs": node_detail_event_refs,
    }))
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
        "state_commit": false,
        "partial_commit": 0,
        "committed_revision": JsonValue::Null,
        "delta": JsonValue::Null,
        "error": {
            "class": class.as_str(),
            "code": reason,
            "message": message,
        },
    });
    FunctionCallError::RespondToModel(result.to_string())
}

pub(super) fn step_has_required_identity(step: &JsonValue) -> bool {
    match step.get("kind").and_then(JsonValue::as_str) {
        Some("map_initialized") => has_text(step, "map_id") && step.get("revision").is_some(),
        Some("graph_mutation") => has_text(step, "map_id") && step.get("revision").is_some(),
        Some("node_transition") => {
            has_text(step, "map_id")
                && has_text(step, "node_id")
                && has_text(step, "status")
                && step.get("revision").is_some()
        }
        Some("complete_then_continue") => {
            has_text(step, "map_id")
                && has_text(step, "current_node_id")
                && has_text(step, "next_node_id")
                && step.get("revision").is_some()
        }
        Some("terminal_transition" | "complete_then_end") => {
            has_text(step, "map_id")
                && step.get("revision").is_some()
                && step.get("finish_closed") == Some(&JsonValue::Bool(true))
                && step.get("root_closed") == Some(&JsonValue::Bool(true))
        }
        Some("node_detail_expanded") => {
            has_text(step, "node_id")
                && has_text(step, "expansion_event_id")
                && has_text(step, "detail_ref")
                && step
                    .get("restored_details")
                    .and_then(JsonValue::as_array)
                    .is_some_and(|details| !details.is_empty())
        }
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
            delta: ActionMapControlDelta {
                map_id: "map-1".into(),
                committed_revision: 3,
                graph_revision_batches: Vec::new(),
                node_detail_events: Vec::new(),
            },
        };

        let step = format_node_detail_expansion_step(0, &outcome);

        assert_eq!(step["restored_detail_count"], 1);
        assert_eq!(step["restored_details"][0]["event_id"], "node-event-read");
        assert_eq!(step["restored_details"][0]["raw_ref"], "output-ref-1");
        assert!(step.get("detail_sha256").is_none());
        assert!(step_has_required_identity(&step));
    }

    #[test]
    fn atomic_completion_steps_have_complete_identity() {
        let handoff = serde_json::json!({
            "kind": "complete_then_continue",
            "map_id": "map-1",
            "current_node_id": "inspect",
            "next_node_id": "implement",
            "revision": 3,
        });
        let terminal = serde_json::json!({
            "kind": "complete_then_end",
            "map_id": "map-1",
            "revision": 4,
            "finish_closed": true,
            "root_closed": true,
        });

        assert!(step_has_required_identity(&handoff));
        assert!(step_has_required_identity(&terminal));
    }
}
