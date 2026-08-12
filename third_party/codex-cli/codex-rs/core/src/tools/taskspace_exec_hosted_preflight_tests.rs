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

#[test]
fn hosted_facts_are_sorted_by_provider_index_and_bind_to_multiple_nodes() {
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
        HostedOutputFact {
            output_index: 9,
            provider_id: "image-1".into(),
            tool: "image_generation".into(),
            outcome: ActionOutcome::Succeeded,
        },
        HostedOutputFact {
            output_index: 2,
            provider_id: "search-1".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
    ];

    let prepared = preflight_taskspace_exec(&envelope, Some(&current), &facts).unwrap();
    assert_eq!(prepared.hosted_bindings[0].output_index, 2);
    assert_eq!(
        prepared.hosted_bindings[0].node_ids,
        vec!["work", "support"]
    );
    assert_eq!(prepared.hosted_bindings[1].output_index, 9);
}

#[test]
fn initialization_can_bind_already_completed_hosted_work_to_new_nodes() {
    let envelope = envelope(
        json!({
            "type": "initialize_and_work",
            "initialize_map": initialize_input(),
            "tools": [{"tool": "web_search", "node_ids": ["work"]}]
        }),
        None,
    );
    let facts = [HostedOutputFact {
        output_index: 4,
        provider_id: "search-new-map".into(),
        tool: "web_search".into(),
        outcome: ActionOutcome::Succeeded,
    }];

    let prepared = preflight_taskspace_exec(&envelope, None, &facts).unwrap();
    assert_eq!(prepared.candidate_map.unwrap().map_id, "map-1");
    assert_eq!(prepared.hosted_bindings[0].node_ids, vec!["work"]);
}

#[test]
fn read_map_schema_cannot_share_a_response_with_hosted_work() {
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
fn hosted_count_tool_and_node_mismatches_are_rejected() {
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
    let search = [HostedOutputFact {
        output_index: 1,
        provider_id: "search-1".into(),
        tool: "web_search".into(),
        outcome: ActionOutcome::Succeeded,
    }];
    assert_eq!(
        preflight_taskspace_exec(&omitted_binding, Some(&current), &search),
        Err(TaskSpaceExecPreflightError::HostedCountMismatch {
            actual: 1,
            declared: 0
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
        Err(TaskSpaceExecPreflightError::HostedCountMismatch {
            actual: 0,
            declared: 1
        })
    );
    let wrong_tool = [HostedOutputFact {
        output_index: 1,
        provider_id: "image-1".into(),
        tool: "image_generation".into(),
        outcome: ActionOutcome::Succeeded,
    }];
    assert!(matches!(
        preflight_taskspace_exec(&one_binding, Some(&current), &wrong_tool),
        Err(TaskSpaceExecPreflightError::HostedToolMismatch { .. })
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
fn duplicate_hosted_provider_facts_are_rejected() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "type": "work",
            "tools": [
                {"tool": "web_search", "node_ids": ["work"]},
                {"tool": "web_search", "node_ids": ["work"]}
            ]
        }),
        Some(&current),
    );
    let duplicate_index = [
        HostedOutputFact {
            output_index: 1,
            provider_id: "search-1".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
        HostedOutputFact {
            output_index: 1,
            provider_id: "search-2".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
    ];
    assert!(matches!(
        preflight_taskspace_exec(&envelope, Some(&current), &duplicate_index),
        Err(TaskSpaceExecPreflightError::HostedFactInvalid {
            reason: "duplicate_output_index",
            ..
        })
    ));

    let duplicate_provider_id = [
        HostedOutputFact {
            output_index: 1,
            provider_id: "search-1".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
        HostedOutputFact {
            output_index: 2,
            provider_id: "search-1".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Failed,
        },
    ];
    assert!(matches!(
        preflight_taskspace_exec(&envelope, Some(&current), &duplicate_provider_id),
        Err(TaskSpaceExecPreflightError::HostedFactInvalid {
            reason: "missing_or_duplicate_provider_id",
            ..
        })
    ));

    let missing_provider_id = [
        HostedOutputFact {
            output_index: 1,
            provider_id: String::new(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
        HostedOutputFact {
            output_index: 2,
            provider_id: "search-2".into(),
            tool: "web_search".into(),
            outcome: ActionOutcome::Succeeded,
        },
    ];
    assert!(matches!(
        preflight_taskspace_exec(&envelope, Some(&current), &missing_provider_id),
        Err(TaskSpaceExecPreflightError::HostedFactInvalid {
            reason: "missing_or_duplicate_provider_id",
            ..
        })
    ));
}
