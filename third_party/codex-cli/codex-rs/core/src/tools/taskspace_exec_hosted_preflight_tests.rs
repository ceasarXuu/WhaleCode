use std::sync::Arc;

use codex_tools::ToolSpec;
use serde_json::json;

use super::*;
use crate::action_map::rooted_dag::ActionOutcome;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;
use crate::action_map::rooted_dag::map_node;
use crate::action_map::rooted_dag::new_map;

fn catalog() -> Arc<TaskSpaceExecCatalog> {
    Arc::new(
        TaskSpaceExecCatalog::build(&[
            ToolSpec::WebSearch {
                external_web_access: Some(true),
                filters: None,
                user_location: None,
                search_context_size: None,
                search_content_types: None,
            },
            ToolSpec::ImageGeneration {
                output_format: "png".into(),
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
                "support",
                "supporting work",
                NodeState::Ready,
                "",
                vec!["root".into()],
            ),
        ],
        map_node(
            "finish",
            "close",
            NodeState::Waiting,
            "",
            vec!["work".into(), "support".into()],
        ),
    )
}

fn envelope(arguments: serde_json::Value, map: Option<&TaskSpaceMap>) -> TaskSpaceExecEnvelope {
    TaskSpaceExecRequestContext::capture("map-1", map, catalog())
        .unwrap()
        .decode_outer_call("outer-hosted", &arguments.to_string())
        .unwrap()
}

fn initialize_input() -> serde_json::Value {
    json!({
        "root": {"node_id": "root", "goal": "deliver", "content": "", "parents": []},
        "work_nodes": [{
            "node_id": "work", "goal": "implement", "content": "", "parents": ["root"]
        }],
        "finish": {"node_id": "finish", "goal": "close", "content": "", "parents": ["work"]}
    })
}

fn hosted(tool: &str, outcome: ActionOutcome) -> HostedToolFact {
    HostedToolFact {
        tool: tool.into(),
        outcome,
    }
}

#[test]
fn logical_hosted_tools_bind_by_capability_not_internal_output_order() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "type": "work",
            "tools": [
                {"tool": "web_search", "node_ids": ["work", "support"]},
                {"tool": "image_generation", "node_ids": ["work"]}
            ]
        }),
        Some(&current),
    );
    let facts = [
        hosted("image_generation", ActionOutcome::Succeeded),
        hosted("web_search", ActionOutcome::Succeeded),
    ];

    let prepared = preflight_taskspace_exec(&envelope, Some(&current), &facts).unwrap();
    assert_eq!(prepared.provider_actions[0].tool_index, 0);
    assert_eq!(prepared.provider_actions[0].tool, "web_search");
    assert_eq!(
        prepared.provider_actions[0].node_ids,
        vec!["work", "support"]
    );
    assert_eq!(
        prepared.provider_actions[0].identity.transport_id(),
        "outer-hosted/taskspace/call/0"
    );
    assert_eq!(prepared.provider_actions[1].tool_index, 1);
    assert_eq!(prepared.provider_actions[1].tool, "image_generation");
}

#[test]
fn initialization_can_record_one_logical_hosted_tool_on_new_nodes() {
    let envelope = envelope(
        json!({
            "type": "initialize_and_work",
            "initialize_map": initialize_input(),
            "tools": [{"tool": "web_search", "node_ids": ["work"]}]
        }),
        None,
    );
    let facts = [hosted("web_search", ActionOutcome::Succeeded)];

    let prepared = preflight_taskspace_exec(&envelope, None, &facts).unwrap();
    assert_eq!(prepared.candidate_map.unwrap().map_id, "map-1");
    assert_eq!(prepared.provider_actions[0].node_ids, vec!["work"]);
}

#[test]
fn read_map_schema_cannot_share_a_response_with_provider_work() {
    let current = open_map();
    let context = TaskSpaceExecRequestContext::capture("map-1", Some(&current), catalog()).unwrap();
    assert!(
        context
            .decode_outer_call(
                "outer",
                &json!({
                    "type": "read_map",
                    "read_map": {},
                    "tools": [{"tool": "web_search", "node_ids": ["work"]}]
                })
                .to_string()
            )
            .is_err()
    );
}

#[test]
fn hosted_tool_set_and_node_mismatches_are_rejected() {
    let current = open_map();
    let omitted_binding = envelope(
        json!({
            "type": "update_map",
            "update_map": {
                "add_work_nodes": [],
                "node_patches": [{"node_id": "work", "content": "noted"}]
            }
        }),
        Some(&current),
    );
    let search = [hosted("web_search", ActionOutcome::Succeeded)];
    assert_eq!(
        preflight_taskspace_exec(&omitted_binding, Some(&current), &search),
        Err(TaskSpaceExecPreflightError::HostedToolSetMismatch {
            actual: vec!["web_search".into()],
            declared: vec![],
        })
    );

    let one_binding = envelope(
        json!({
            "type": "work",
            "tools": [{"tool": "web_search", "node_ids": ["work"]}]
        }),
        Some(&current),
    );
    assert_eq!(
        preflight_taskspace_exec(&one_binding, Some(&current), &[]),
        Err(TaskSpaceExecPreflightError::HostedToolSetMismatch {
            actual: vec![],
            declared: vec!["web_search".into()],
        })
    );
    let wrong_tool = [hosted("image_generation", ActionOutcome::Succeeded)];
    assert!(matches!(
        preflight_taskspace_exec(&one_binding, Some(&current), &wrong_tool),
        Err(TaskSpaceExecPreflightError::HostedToolSetMismatch { .. })
    ));

    for (nodes, reason) in [
        (json!(["work", "work"]), "empty_or_duplicate_node"),
        (json!(["missing"]), "unknown_node"),
        (json!(["root"]), "boundary_node"),
    ] {
        let binding = envelope(
            json!({
                "type": "work",
                "tools": [{"tool": "web_search", "node_ids": nodes}]
            }),
            Some(&current),
        );
        assert!(matches!(
            preflight_taskspace_exec(&binding, Some(&current), &search),
            Err(TaskSpaceExecPreflightError::HostedNodeInvalid {reason: actual, ..}) if actual == reason
        ));
    }
}

#[test]
fn duplicate_logical_hosted_tools_are_rejected() {
    let current = open_map();
    let duplicate_declaration = envelope(
        json!({
            "type": "work",
            "tools": [
                {"tool": "web_search", "node_ids": ["work"]},
                {"tool": "web_search", "node_ids": ["support"]}
            ]
        }),
        Some(&current),
    );
    let search = [hosted("web_search", ActionOutcome::Succeeded)];
    assert!(matches!(
        preflight_taskspace_exec(&duplicate_declaration, Some(&current), &search),
        Err(TaskSpaceExecPreflightError::HostedToolDuplicate { .. })
    ));

    let one_binding = envelope(
        json!({
            "type": "work",
            "tools": [{"tool": "web_search", "node_ids": ["work"]}]
        }),
        Some(&current),
    );
    let duplicate_facts = [
        hosted("web_search", ActionOutcome::Succeeded),
        hosted("web_search", ActionOutcome::Failed),
    ];
    assert!(matches!(
        preflight_taskspace_exec(&one_binding, Some(&current), &duplicate_facts),
        Err(TaskSpaceExecPreflightError::HostedFactDuplicate { .. })
    ));
}
