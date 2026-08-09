use std::collections::BTreeMap;
use std::sync::Arc;

use codex_tools::AdditionalProperties;
use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;
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
                description: "Run a shell command.".into(),
                strict: false,
                parameters: JsonSchema::object(
                    BTreeMap::from([("cmd".into(), JsonSchema::string(None))]),
                    Some(vec!["cmd".into()]),
                    Some(AdditionalProperties::Boolean(false)),
                ),
                output_schema: None,
                defer_loading: None,
            }),
            ToolSpec::Function(ResponsesApiTool {
                name: "read_file".into(),
                description: "Read a file.".into(),
                strict: false,
                parameters: JsonSchema::object(
                    BTreeMap::from([("path".into(), JsonSchema::string(None))]),
                    Some(vec!["path".into()]),
                    Some(AdditionalProperties::Boolean(false)),
                ),
                output_schema: None,
                defer_loading: None,
            }),
            ToolSpec::Freeform(FreeformTool {
                name: "apply_patch".into(),
                description: "Apply one patch.".into(),
                format: FreeformToolFormat {
                    r#type: "grammar".into(),
                    syntax: "lark".into(),
                    definition: "start: /.+/".into(),
                },
            }),
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
                "blocked",
                "blocked work",
                NodeState::Blocked,
                "",
                vec!["root".into()],
            ),
        ],
        map_node(
            "finish",
            "close",
            NodeState::Waiting,
            "",
            vec!["work".into(), "blocked".into()],
        ),
    )
}

fn envelope(arguments: serde_json::Value, map: Option<&TaskSpaceMap>) -> TaskSpaceExecEnvelope {
    TaskSpaceExecRequestContext::capture("map-1", map, catalog())
        .unwrap()
        .decode_outer_call("outer-1", &arguments.to_string())
        .unwrap()
}

fn read_call(node_id: &str) -> serde_json::Value {
    json!({"tool": "read_file", "node_id": node_id, "arguments": {"path": "src/lib.rs"}})
}

fn update_completed(node_id: &str) -> serde_json::Value {
    json!({
        "tool": "update_map",
        "arguments": {
            "add_work_nodes": [],
            "node_patches": [{"node_id": node_id, "state": "completed"}]
        }
    })
}

fn initialize_call() -> serde_json::Value {
    json!({
        "tool": "initialize_map",
        "arguments": {
            "root": {"node_id": "root", "goal": "deliver", "content": "", "parents": []},
            "work_nodes": [{
                "node_id": "work",
                "goal": "implement",
                "content": "",
                "parents": ["root"]
            }],
            "finish": {"node_id": "finish", "goal": "close", "content": "", "parents": ["work"]}
        }
    })
}

#[test]
fn valid_work_update_and_finish_builds_one_side_effect_free_candidate() {
    let mut current = open_map();
    current
        .work_nodes
        .iter_mut()
        .find(|node| node.node_id == "blocked")
        .unwrap()
        .state = NodeState::Completed;
    let before = current.clone();
    let envelope = envelope(
        json!({
            "calls": [
                read_call("work"),
                update_completed("work"),
                {"tool": "finish_map", "arguments": {"content": "Delivered."}}
            ],
            "hosted_bindings": []
        }),
        Some(&current),
    );

    let prepared = preflight_taskspace_exec(&envelope, Some(&current), &[]).unwrap();

    assert_eq!(current, before, "preflight mutated its input Map");
    assert_eq!(prepared.client_calls.len(), 1);
    assert_eq!(
        prepared.client_calls[0].identity.transport_id(),
        "outer-1/taskspace/call/0"
    );
    let candidate = prepared.candidate_map.unwrap();
    assert_eq!(candidate.revision, 3);
    assert_eq!(candidate.root.state, NodeState::Completed);
    assert_eq!(candidate.finish.state, NodeState::Completed);
}

#[test]
fn initialize_and_work_are_admitted_in_one_plan() {
    let envelope = envelope(
        json!({
            "calls": [initialize_call(), read_call("work")],
            "hosted_bindings": []
        }),
        None,
    );

    let prepared = preflight_taskspace_exec(&envelope, None, &[]).unwrap();
    assert_eq!(prepared.candidate_map.unwrap().map_id, "map-1");
    assert_eq!(prepared.client_calls.len(), 1);
}

#[test]
fn rendered_first_turn_example_passes_the_real_preflight_contract() {
    let example = canonical_first_turn_example();
    let envelope = envelope(example, None);

    let prepared = preflight_taskspace_exec(&envelope, None, &[]).unwrap();
    assert_eq!(prepared.client_calls.len(), 1);
    assert_eq!(prepared.client_calls[0].call.public_name, "exec_command");
    assert_eq!(prepared.client_calls[0].call.node_id, "inspect");
    assert_eq!(prepared.candidate_map.unwrap().map_id, "map-1");
}

#[test]
fn read_only_plan_returns_the_complete_agent_visible_map() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "calls": [{"tool": "read_map", "arguments": {}}],
            "hosted_bindings": []
        }),
        Some(&current),
    );

    let prepared = preflight_taskspace_exec(&envelope, Some(&current), &[]).unwrap();
    assert_eq!(prepared.candidate_map, Some(current.clone()));
    assert_eq!(prepared.read_maps.len(), 1);
    let (index, view) = &prepared.read_maps[0];
    assert_eq!(*index, 0);
    assert_eq!(view.map_id, current.map_id);
    let work = view
        .nodes
        .iter()
        .find(|node| node.node_id == "work")
        .unwrap();
    assert_eq!(work.parents, vec!["root"]);
    assert_eq!(work.children, vec!["finish"]);
}

#[test]
fn stale_request_and_invalid_map_boundaries_are_rejected() {
    let requested = open_map();
    let mut current = requested.clone();
    current.revision += 1;
    let stale = envelope(
        json!({"calls": [read_call("work")], "hosted_bindings": []}),
        Some(&requested),
    );
    assert!(matches!(
        preflight_taskspace_exec(&stale, Some(&current), &[]),
        Err(TaskSpaceExecPreflightError::RequestContext(
            TaskSpaceExecEnvelopeError::MapRevisionChanged { .. }
        ))
    ));

    let invalid = envelope(
        json!({
            "calls": [read_call("work"), {"tool": "read_map", "arguments": {}}],
            "hosted_bindings": []
        }),
        Some(&requested),
    );
    assert_eq!(
        preflight_taskspace_exec(&invalid, Some(&requested), &[]),
        Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index: 1,
            operation: "read_map"
        })
    );

    let read_then_work = envelope(
        json!({
            "calls": [{"tool": "read_map", "arguments": {}}, read_call("work")],
            "hosted_bindings": []
        }),
        Some(&requested),
    );
    assert_eq!(
        preflight_taskspace_exec(&read_then_work, Some(&requested), &[]),
        Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index: 0,
            operation: "read_map"
        })
    );
}

#[test]
fn lifecycle_transitions_require_work_and_finish_must_be_last() {
    let init_only = envelope(
        json!({"calls": [initialize_call()], "hosted_bindings": []}),
        None,
    );
    assert_eq!(
        preflight_taskspace_exec(&init_only, None, &[]),
        Err(TaskSpaceExecPreflightError::LifecycleRequiresWork {
            index: 0,
            operation: "initialize_map"
        })
    );

    let initialize_then_finish = envelope(
        json!({
            "calls": [
                initialize_call(),
                {"tool": "finish_map", "arguments": {"content": "Done."}}
            ],
            "hosted_bindings": []
        }),
        None,
    );
    assert_eq!(
        preflight_taskspace_exec(&initialize_then_finish, None, &[]),
        Err(TaskSpaceExecPreflightError::LifecycleRequiresWork {
            index: 0,
            operation: "initialize_map"
        })
    );

    let mut current = open_map();
    for node in &mut current.work_nodes {
        node.state = NodeState::Completed;
    }
    let finish_then_work = envelope(
        json!({
            "calls": [
                {"tool": "finish_map", "arguments": {"content": "Done."}},
                read_call("root")
            ],
            "hosted_bindings": []
        }),
        Some(&current),
    );
    assert_eq!(
        preflight_taskspace_exec(&finish_then_work, Some(&current), &[]),
        Err(TaskSpaceExecPreflightError::InvalidMapBoundary {
            index: 0,
            operation: "finish_map"
        })
    );
}

#[test]
fn reopen_requires_and_accepts_real_followup_work() {
    let mut finished = open_map();
    for node in &mut finished.work_nodes {
        node.state = NodeState::Completed;
    }
    finished.root.state = NodeState::Completed;
    finished.finish.state = NodeState::Completed;
    finished.finish.content = "Earlier delivery.".into();
    let envelope = envelope(
        json!({
            "calls": [
                {"tool": "reopen_map", "arguments": {}},
                {
                    "tool": "update_map",
                    "arguments": {
                        "add_work_nodes": [{
                            "node_id": "followup",
                            "goal": "handle user follow-up",
                            "content": "",
                            "parents": ["work", "blocked"]
                        }],
                        "node_patches": [{"node_id": "finish", "parents": ["followup"]}]
                    }
                },
                read_call("followup")
            ],
            "hosted_bindings": []
        }),
        Some(&finished),
    );

    let prepared = preflight_taskspace_exec(&envelope, Some(&finished), &[]).unwrap();
    assert_eq!(
        prepared.candidate_map.unwrap().root.state,
        NodeState::InFlight
    );
    assert_eq!(prepared.client_calls.len(), 1);
}

#[test]
fn noop_update_and_canonical_dag_errors_are_rejected_without_mutation() {
    let current = open_map();
    let before = current.clone();
    let noop = envelope(
        json!({
            "calls": [{"tool": "update_map", "arguments": {"add_work_nodes": [], "node_patches": []}}],
            "hosted_bindings": []
        }),
        Some(&current),
    );
    assert_eq!(
        preflight_taskspace_exec(&noop, Some(&current), &[]),
        Err(TaskSpaceExecPreflightError::NoEffectMapUpdate { index: 0 })
    );
    assert_eq!(current, before);

    let mut invalid_init = initialize_call();
    invalid_init["arguments"]["finish"]["parents"] = json!(["missing"]);
    let invalid = envelope(
        json!({"calls": [invalid_init, read_call("work")], "hosted_bindings": []}),
        None,
    );
    assert!(matches!(
        preflight_taskspace_exec(&invalid, None, &[]),
        Err(TaskSpaceExecPreflightError::MapOperationRejected { index: 0, .. })
    ));
}

#[test]
fn client_node_state_and_function_schema_are_checked_before_dispatch() {
    let current = open_map();
    for (call, expected) in [
        (
            read_call("missing"),
            TaskSpaceExecPreflightError::ClientNodeMissing {
                index: 0,
                node_id: "missing".into(),
            },
        ),
        (
            read_call("root"),
            TaskSpaceExecPreflightError::ClientNodeNotWork {
                index: 0,
                node_id: "root".into(),
                role: crate::action_map::rooted_dag::NodeRole::TaskRoot,
            },
        ),
    ] {
        let envelope = envelope(
            json!({"calls": [call], "hosted_bindings": []}),
            Some(&current),
        );
        assert_eq!(
            preflight_taskspace_exec(&envelope, Some(&current), &[]),
            Err(expected)
        );
    }

    let blocked = envelope(
        json!({"calls": [read_call("blocked")], "hosted_bindings": []}),
        Some(&current),
    );
    assert!(preflight_taskspace_exec(&blocked, Some(&current), &[]).is_ok());

    let mut waiting_map = current.clone();
    waiting_map.work_nodes.push(map_node(
        "waiting",
        "dependent work",
        NodeState::Waiting,
        "",
        vec!["work".into()],
    ));
    waiting_map.finish.parents = vec!["waiting".into(), "blocked".into()];
    let waiting = envelope(
        json!({"calls": [read_call("waiting")], "hosted_bindings": []}),
        Some(&waiting_map),
    );
    assert_eq!(
        preflight_taskspace_exec(&waiting, Some(&waiting_map), &[]),
        Err(TaskSpaceExecPreflightError::ClientNodeNotExecutable {
            index: 0,
            node_id: "waiting".into(),
            state: NodeState::Waiting,
        })
    );

    for arguments in [json!({}), json!({"path": "a", "extra": true})] {
        let envelope = envelope(
            json!({
                "calls": [{"tool": "read_file", "node_id": "work", "arguments": arguments}],
                "hosted_bindings": []
            }),
            Some(&current),
        );
        assert!(matches!(
            preflight_taskspace_exec(&envelope, Some(&current), &[]),
            Err(TaskSpaceExecPreflightError::ClientArgumentsInvalid { index: 0, .. })
        ));
    }
}

#[test]
fn one_response_cannot_prepare_multiple_apply_patch_calls() {
    let current = open_map();
    let envelope = envelope(
        json!({
            "calls": [
                {"tool": "apply_patch", "node_id": "work", "input": "patch-a"},
                {"tool": "apply_patch", "node_id": "work", "input": "patch-b"}
            ],
            "hosted_bindings": []
        }),
        Some(&current),
    );
    assert_eq!(
        preflight_taskspace_exec(&envelope, Some(&current), &[]),
        Err(TaskSpaceExecPreflightError::PatchLimitExceeded {
            indices: vec![0, 1]
        })
    );
}
