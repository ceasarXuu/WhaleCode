use std::collections::BTreeMap;
use std::sync::Arc;

use codex_tools::AdditionalProperties;
use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::*;
use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;
use crate::action_map::rooted_dag::map_node;
use crate::action_map::rooted_dag::new_map;

fn catalog() -> Arc<TaskSpaceExecCatalog> {
    Arc::new(
        TaskSpaceExecCatalog::build(&[
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
            ToolSpec::Freeform(FreeformTool {
                name: "apply_patch".into(),
                description: "Apply one patch.".into(),
                defer_loading: None,
                format: FreeformToolFormat {
                    r#type: "grammar".into(),
                    syntax: "lark".into(),
                    definition: "start: /.+/".into(),
                },
            }),
            ToolSpec::WebSearch {
                external_web_access: Some(true),
                indexed_web_access: None,
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
                "inspect",
                "inspect",
                NodeState::Ready,
                "",
                vec!["root".into()],
            ),
            map_node(
                "implement",
                "implement",
                NodeState::Waiting,
                "",
                vec!["inspect".into()],
            ),
        ],
        map_node(
            "finish",
            "close",
            NodeState::Waiting,
            "",
            vec!["implement".into()],
        ),
    )
}

fn complete_map() -> TaskSpaceMap {
    let mut map = open_map();
    map.work_nodes
        .iter_mut()
        .for_each(|node| node.state = NodeState::Completed);
    map.root.state = NodeState::Completed;
    map.finish.state = NodeState::Completed;
    map.finish.content = "done".into();
    map
}

fn envelope(value: Value, map: Option<&TaskSpaceMap>) -> TaskSpaceExecEnvelope {
    TaskSpaceExecRequestContext::capture("map-1", map, catalog())
        .unwrap()
        .decode_outer_call("outer", &value.to_string())
        .unwrap()
}

fn read_tool(node_id: &str) -> Value {
    json!({"tool": "read_file", "node_id": node_id, "input": {"path": "src/lib.rs"}})
}

fn complete(node_id: &str) -> Value {
    json!({
        "add_work_nodes": [],
        "node_patches": [{"node_id": node_id, "state": "completed"}]
    })
}

#[test]
fn l1_initialize_and_work_starts_the_agent_selected_ready_node() {
    let prepared =
        preflight_taskspace_exec(&envelope(canonical_first_turn_example(), None), None).unwrap();
    assert_eq!(prepared.client_calls.len(), 1);
    assert_eq!(prepared.client_calls[0].identity.index, 0);
    let map = prepared.candidate_map.unwrap();
    assert_eq!(
        rooted_dag::node(&map, "inspect").unwrap().state,
        NodeState::InFlight
    );
    assert_eq!(
        rooted_dag::node(&map, "implement").unwrap().state,
        NodeState::Waiting
    );
}

#[test]
fn canonical_examples_form_one_executable_lifecycle() {
    let initialized =
        preflight_taskspace_exec(&envelope(canonical_first_turn_example(), None), None)
            .unwrap()
            .candidate_map
            .unwrap();
    assert_eq!(
        rooted_dag::node(&initialized, "inspect").unwrap().state,
        NodeState::InFlight
    );
    assert_eq!(
        rooted_dag::node(&initialized, "implement").unwrap().state,
        NodeState::Waiting
    );

    let handed_off = preflight_taskspace_exec(
        &envelope(canonical_handoff_example(), Some(&initialized)),
        Some(&initialized),
    )
    .unwrap()
    .candidate_map
    .unwrap();
    assert_eq!(
        rooted_dag::node(&handed_off, "inspect").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(
        rooted_dag::node(&handed_off, "implement").unwrap().state,
        NodeState::InFlight
    );

    let finished = preflight_taskspace_exec(
        &envelope(canonical_finish_example(), Some(&handed_off)),
        Some(&handed_off),
    )
    .unwrap()
    .candidate_map
    .unwrap();
    assert_eq!(finished.root.state, NodeState::Completed);
    assert_eq!(
        rooted_dag::node(&finished, "implement").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(finished.finish.state, NodeState::Completed);
}

#[test]
fn l2_work_starts_ready_but_preserves_inflight_nodes() {
    let current = open_map();
    let prepared = preflight_taskspace_exec(
        &envelope(
            json!({"type": "work", "tools": [read_tool("inspect")]}),
            Some(&current),
        ),
        Some(&current),
    )
    .unwrap();
    assert_eq!(
        rooted_dag::node(prepared.candidate_map.as_ref().unwrap(), "inspect")
            .unwrap()
            .state,
        NodeState::InFlight
    );
}

#[test]
fn l3_pure_update_is_valid_without_forced_followup_work() {
    let current = open_map();
    let prepared = preflight_taskspace_exec(
        &envelope(
            json!({"type": "update_map", "update_map": complete("inspect")}),
            Some(&current),
        ),
        Some(&current),
    )
    .unwrap();
    assert!(prepared.client_calls.is_empty());
    assert_eq!(
        rooted_dag::node(prepared.candidate_map.as_ref().unwrap(), "implement")
            .unwrap()
            .state,
        NodeState::Ready
    );
}

#[test]
fn l4_parent_completion_unlocks_and_starts_direct_child_work() {
    let current = open_map();
    let prepared = preflight_taskspace_exec(
        &envelope(canonical_handoff_example(), Some(&current)),
        Some(&current),
    )
    .unwrap();
    assert_eq!(
        rooted_dag::node(prepared.candidate_map.as_ref().unwrap(), "implement")
            .unwrap()
            .state,
        NodeState::InFlight
    );
}

#[test]
fn l5_update_and_finish_closes_only_a_ready_finish() {
    let mut current = open_map();
    current
        .work_nodes
        .iter_mut()
        .find(|node| node.node_id == "inspect")
        .unwrap()
        .state = NodeState::Completed;
    current
        .work_nodes
        .iter_mut()
        .find(|node| node.node_id == "implement")
        .unwrap()
        .state = NodeState::InFlight;
    let prepared = preflight_taskspace_exec(
        &envelope(canonical_finish_example(), Some(&current)),
        Some(&current),
    )
    .unwrap();
    let map = prepared.candidate_map.unwrap();
    assert_eq!(map.root.state, NodeState::Completed);
    assert_eq!(map.finish.state, NodeState::Completed);
}

#[test]
fn l5_update_and_finish_applies_dependency_patches_in_declared_order() {
    let mut current = open_map();
    current
        .work_nodes
        .iter_mut()
        .find(|node| node.node_id == "inspect")
        .unwrap()
        .state = NodeState::InFlight;
    let before = current.clone();
    let ordered = json!({
        "type": "update_and_finish",
        "update_map": {
            "add_work_nodes": [],
            "node_patches": [
                {"node_id": "inspect", "state": "completed"},
                {"node_id": "implement", "state": "completed"}
            ]
        },
        "finish_map": {"content": "done"}
    });
    let prepared =
        preflight_taskspace_exec(&envelope(ordered, Some(&current)), Some(&current)).unwrap();
    let map = prepared.candidate_map.unwrap();
    assert_eq!(map.root.state, NodeState::Completed);
    assert_eq!(
        rooted_dag::node(&map, "inspect").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(
        rooted_dag::node(&map, "implement").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(map.finish.state, NodeState::Completed);
    assert_eq!(current, before);

    let reversed = json!({
        "type": "update_and_finish",
        "update_map": {
            "add_work_nodes": [],
            "node_patches": [
                {"node_id": "implement", "state": "completed"},
                {"node_id": "inspect", "state": "completed"}
            ]
        },
        "finish_map": {"content": "done"}
    });
    assert!(preflight_taskspace_exec(&envelope(reversed, Some(&current)), Some(&current)).is_err());
    assert_eq!(current, before);
}

#[test]
fn l6_read_returns_the_complete_agent_visible_map() {
    let current = open_map();
    let prepared = preflight_taskspace_exec(
        &envelope(canonical_read_example(), Some(&current)),
        Some(&current),
    )
    .unwrap();
    assert_eq!(prepared.read_maps.len(), 1);
    assert_eq!(prepared.read_maps[0].1.nodes.len(), 4);
    assert_eq!(prepared.candidate_map, Some(current));
}

#[test]
fn l7_reopen_update_and_work_keeps_recovery_agent_authored() {
    let current = complete_map();
    let value = json!({
        "type": "reopen_update_and_work",
        "reopen_map": {},
        "update_map": {
            "add_work_nodes": [{
                "node_id": "repair", "goal": "repair", "content": "", "parents": ["root"]
            }],
            "node_patches": [{"node_id": "finish", "parents": ["implement", "repair"]}]
        },
        "tools": [read_tool("repair")]
    });
    let prepared =
        preflight_taskspace_exec(&envelope(value, Some(&current)), Some(&current)).unwrap();
    assert_eq!(
        rooted_dag::node(prepared.candidate_map.as_ref().unwrap(), "repair")
            .unwrap()
            .state,
        NodeState::InFlight
    );
}

#[test]
fn l8_finish_closes_a_map_already_ready_to_finish() {
    let mut current = open_map();
    current
        .work_nodes
        .iter_mut()
        .for_each(|node| node.state = NodeState::Completed);
    current.finish.state = NodeState::Ready;
    let prepared = preflight_taskspace_exec(
        &envelope(
            json!({"type": "finish_map", "finish_map": {"content": "done"}}),
            Some(&current),
        ),
        Some(&current),
    )
    .unwrap();
    assert_eq!(
        prepared.candidate_map.unwrap().finish.state,
        NodeState::Completed
    );
}

#[test]
fn dynamic_node_arguments_patch_and_revision_errors_fail_before_dispatch() {
    let current = open_map();
    for value in [
        json!({"type": "work", "tools": [read_tool("missing")]}),
        json!({"type": "work", "tools": [read_tool("implement")]}),
        json!({"type": "work", "tools": [
            {"tool": "apply_patch", "node_id": "inspect", "input": "one"},
            {"tool": "apply_patch", "node_id": "inspect", "input": "two"}
        ]}),
    ] {
        let envelope = envelope(value, Some(&current));
        assert!(preflight_taskspace_exec(&envelope, Some(&current)).is_err());
    }

    assert!(
        TaskSpaceExecRequestContext::capture("map-1", Some(&current), catalog())
            .unwrap()
            .decode_outer_call(
                "outer",
                &json!({"type": "work", "tools": [{
                    "tool": "read_file", "node_id": "inspect", "input": {"path": 1}
                }]})
                .to_string(),
            )
            .is_err()
    );

    let request = open_map();
    let mut changed = request.clone();
    changed.revision += 1;
    let stale = envelope(
        json!({"type": "work", "tools": [read_tool("inspect")]}),
        Some(&request),
    );
    assert!(matches!(
        preflight_taskspace_exec(&stale, Some(&changed)),
        Err(TaskSpaceExecPreflightError::RequestContext(_))
    ));
}

#[test]
fn noop_update_and_invalid_finish_are_rejected_without_mutation() {
    let current = open_map();
    let before = current.clone();
    let noop = envelope(
        json!({"type": "update_map", "update_map": {"add_work_nodes": [], "node_patches": []}}),
        Some(&current),
    );
    assert!(matches!(
        preflight_taskspace_exec(&noop, Some(&current)),
        Err(TaskSpaceExecPreflightError::NoEffectMapUpdate { .. })
    ));
    let finish = envelope(
        json!({"type": "finish_map", "finish_map": {"content": "too soon"}}),
        Some(&current),
    );
    assert!(preflight_taskspace_exec(&finish, Some(&current)).is_err());
    assert_eq!(current, before);
}
