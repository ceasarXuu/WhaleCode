use std::collections::BTreeMap;
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_state::TaskSpacePendingProviderAction;
use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use serde_json::json;

use super::*;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;
use crate::action_map::rooted_dag::map_node;
use crate::action_map::rooted_dag::new_map;

fn catalog() -> Arc<TaskSpaceExecCatalog> {
    Arc::new(
        TaskSpaceExecCatalog::build(&[
            ToolSpec::Function(ResponsesApiTool {
                name: "exec_command".into(),
                description: "Run a command.".into(),
                strict: false,
                parameters: JsonSchema::object(
                    BTreeMap::from([("cmd".into(), JsonSchema::string(None))]),
                    Some(vec!["cmd".into()]),
                    Some(AdditionalProperties::Boolean(false)),
                ),
                output_schema: None,
                defer_loading: None,
            }),
            ToolSpec::WebSearch {
                external_web_access: Some(true),
                filters: None,
                user_location: None,
                search_context_size: None,
                search_content_types: None,
            },
        ])
        .unwrap(),
    )
}

fn open_map() -> TaskSpaceMap {
    new_map(
        "map-1".into(),
        map_node("root", "deliver", NodeState::InFlight, "", vec![]),
        vec![
            map_node(
                "work",
                "implement",
                NodeState::Ready,
                "",
                vec!["root".into()],
            ),
            map_node(
                "done",
                "completed evidence",
                NodeState::Completed,
                "",
                vec!["root".into()],
            ),
        ],
        map_node(
            "finish",
            "close",
            NodeState::Waiting,
            "",
            vec!["work".into(), "done".into()],
        ),
    )
}

fn pending(action_id: &str) -> TaskSpacePendingProviderAction {
    TaskSpacePendingProviderAction {
        action_id: action_id.into(),
        origin_thread_id: ThreadId::new(),
        map_id: Some("map-1".into()),
        provider_response_id: "response-1".into(),
        provider_action_key: "response-1/web_search".into(),
        tool_name: "web_search".into(),
        outcome: TaskSpaceActionOutcome::Succeeded,
        created_at_ms: 1,
    }
}

fn envelope(arguments: serde_json::Value, map: Option<&TaskSpaceMap>) -> TaskSpaceExecEnvelope {
    TaskSpaceExecRequestContext::capture("map-1", map, catalog())
        .unwrap()
        .decode_outer_call("outer-attribution", &arguments.to_string())
        .unwrap()
}

#[test]
fn pending_actions_are_assigned_by_stable_id_to_agent_selected_nodes() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "type": "attribute_actions",
            "assign_pending_actions": [{
                "action_id": "provider-action-1",
                "node_ids": ["work", "done"]
            }]
        }),
        Some(&current),
    );

    let prepared = preflight_taskspace_exec(
        &envelope,
        Some(&current),
        &[pending("provider-action-1")],
        false,
    )
    .unwrap();
    assert_eq!(prepared.pending_attributions.len(), 1);
    assert_eq!(
        prepared.pending_attributions[0].action_id,
        "provider-action-1"
    );
    assert_eq!(
        prepared.pending_attributions[0].node_ids,
        vec!["work", "done"]
    );
    assert_eq!(prepared.candidate_map, Some(current));
}

#[test]
fn mutating_sequences_require_the_exact_pending_set() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "type": "work",
            "tools": [{"tool": "exec_command", "node_id": "work", "input": {"cmd": "pwd"}}]
        }),
        Some(&current),
    );
    assert!(matches!(
        preflight_taskspace_exec(
            &envelope,
            Some(&current),
            &[pending("provider-action-1")],
            false,
        ),
        Err(TaskSpaceExecPreflightError::PendingAttributionSetMismatch { .. })
    ));
}

#[test]
fn read_map_may_leave_pending_actions_for_the_next_sequence() {
    let current = open_map();
    let envelope = envelope(json!({"type": "read_map", "read_map": {}}), Some(&current));
    let prepared = preflight_taskspace_exec(
        &envelope,
        Some(&current),
        &[pending("provider-action-1")],
        false,
    )
    .unwrap();
    assert!(prepared.pending_attributions.is_empty());
    assert_eq!(prepared.read_maps.len(), 1);
}

#[test]
fn unknown_duplicate_and_boundary_attributions_are_rejected() {
    let current = open_map();
    for (entries, expected_reason) in [
        (
            json!([{"action_id": "missing", "node_ids": ["work"]}]),
            "set",
        ),
        (
            json!([
                {"action_id": "provider-action-1", "node_ids": ["work"]},
                {"action_id": "provider-action-1", "node_ids": ["done"]}
            ]),
            "duplicate",
        ),
        (
            json!([{"action_id": "provider-action-1", "node_ids": ["root"]}]),
            "node",
        ),
    ] {
        let envelope = envelope(
            json!({"type": "attribute_actions", "assign_pending_actions": entries}),
            Some(&current),
        );
        let error = preflight_taskspace_exec(
            &envelope,
            Some(&current),
            &[pending("provider-action-1")],
            false,
        )
        .unwrap_err();
        match expected_reason {
            "set" => assert!(matches!(
                error,
                TaskSpaceExecPreflightError::PendingAttributionSetMismatch { .. }
            )),
            "duplicate" => assert!(matches!(
                error,
                TaskSpaceExecPreflightError::PendingAttributionDuplicate { .. }
            )),
            "node" => assert!(matches!(
                error,
                TaskSpaceExecPreflightError::PendingAttributionNodeInvalid { .. }
            )),
            _ => unreachable!(),
        }
    }
}
