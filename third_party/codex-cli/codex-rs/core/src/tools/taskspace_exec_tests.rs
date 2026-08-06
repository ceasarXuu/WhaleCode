use codex_tools::ToolSpecCapabilityInput;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;
use crate::action_map::rooted_dag::NodeState;

fn initialize_value() -> serde_json::Value {
    json!({
        "tool": "initialize_map",
        "arguments": {
            "root": {
                "node_id": "root",
                "goal": "Deliver the task",
                "content": "",
                "parents": []
            },
            "work_nodes": [{
                "node_id": "work",
                "goal": "Implement",
                "state": "ready",
                "content": "",
                "parents": ["root"]
            }],
            "finish": {
                "node_id": "finish",
                "goal": "Close the task",
                "content": "",
                "parents": ["work"]
            }
        }
    })
}

#[test]
fn initialize_contract_omits_runtime_owned_fields_and_builds_canonical_map() {
    let operation: MapOperation = serde_json::from_value(initialize_value()).unwrap();
    let effect = apply_map_operation(None, "map-session", operation).unwrap();
    let MapOperationEffect::Candidate(map) = effect else {
        panic!("initialize must produce a candidate")
    };

    assert_eq!(map.map_id, "map-session");
    assert_eq!(map.revision, 1);
    assert_eq!(map.root.state, NodeState::InFlight);
    assert_eq!(map.finish.state, NodeState::Waiting);
    assert!(map.root.actions.is_empty());
    assert!(map.work_nodes[0].actions.is_empty());

    let wire = initialize_value().to_string();
    for runtime_owned in ["map_id", "revision", "actions", "children"] {
        assert!(
            !wire.contains(runtime_owned),
            "{runtime_owned} leaked into wire"
        );
    }
}

#[test]
fn update_changes_parents_and_state_through_canonical_transaction() {
    let initial: MapOperation = serde_json::from_value(initialize_value()).unwrap();
    let MapOperationEffect::Candidate(map) =
        apply_map_operation(None, "map-session", initial).unwrap()
    else {
        panic!("initialize must produce a candidate")
    };
    let update: MapOperation = serde_json::from_value(json!({
        "tool": "update_map",
        "arguments": {
            "add_work_nodes": [{
                "node_id": "verify",
                "goal": "Verify",
                "state": "waiting",
                "content": "",
                "parents": ["work"]
            }],
            "node_patches": [
                {"node_id": "work", "state": "completed"},
                {"node_id": "finish", "parents": ["verify"]}
            ]
        }
    }))
    .unwrap();

    let MapOperationEffect::Candidate(updated) =
        apply_map_operation(Some(&map), "map-session", update).unwrap()
    else {
        panic!("update must produce a candidate")
    };

    assert_eq!(updated.revision, 2);
    assert_eq!(
        updated
            .work_nodes
            .iter()
            .find(|node| node.node_id == "work")
            .unwrap()
            .state,
        NodeState::Completed
    );
    assert_eq!(
        updated
            .work_nodes
            .iter()
            .find(|node| node.node_id == "verify")
            .unwrap()
            .state,
        NodeState::Ready
    );
    assert_eq!(updated.finish.parents, vec!["verify"]);
}

#[test]
fn read_reopen_and_finish_use_the_same_canonical_map() {
    let initial: MapOperation = serde_json::from_value(initialize_value()).unwrap();
    let MapOperationEffect::Candidate(mut map) =
        apply_map_operation(None, "map-session", initial).unwrap()
    else {
        panic!("initialize must produce a candidate")
    };
    map.work_nodes[0].state = NodeState::Completed;

    let finish: MapOperation = serde_json::from_value(json!({
        "tool": "finish_map",
        "arguments": {"content": "Delivered and verified."}
    }))
    .unwrap();
    let MapOperationEffect::Candidate(finished) =
        apply_map_operation(Some(&map), "map-session", finish).unwrap()
    else {
        panic!("finish must produce a candidate")
    };
    let MapOperationEffect::Read(read) = apply_map_operation(
        Some(&finished),
        "map-session",
        serde_json::from_value(json!({"tool": "read_map", "arguments": {}})).unwrap(),
    )
    .unwrap() else {
        panic!("read must not produce a write candidate")
    };
    assert_eq!(read, finished);

    let MapOperationEffect::Candidate(reopened) = apply_map_operation(
        Some(&finished),
        "map-session",
        serde_json::from_value(json!({"tool": "reopen_map", "arguments": {}})).unwrap(),
    )
    .unwrap() else {
        panic!("reopen must produce a candidate")
    };
    assert_eq!(reopened.root.state, NodeState::InFlight);
    assert_eq!(reopened.finish.state, NodeState::Ready);
}

#[test]
fn strict_decoder_rejects_unknown_and_runtime_owned_fields() {
    for (path, value) in [
        ("revision", json!(3)),
        ("map_id", json!("agent-map")),
        ("actions", json!([])),
        ("children", json!([])),
    ] {
        let mut input = initialize_value();
        input["arguments"]["root"][path] = value;
        let error = serde_json::from_value::<MapOperation>(input).unwrap_err();
        assert!(
            error.to_string().contains("unknown field"),
            "{path}: {error}"
        );
    }
}

#[test]
fn map_capabilities_are_deterministic_and_strict() {
    let first = map_operation_capabilities();
    let second = map_operation_capabilities();
    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|spec| spec.public_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "initialize_map",
            "update_map",
            "read_map",
            "reopen_map",
            "finish_map"
        ]
    );
    for capability in first {
        let ToolSpecCapabilityInput::Function(schema) = capability.input else {
            panic!("Map operation must be a structured function")
        };
        assert_eq!(
            schema.additional_properties,
            Some(codex_tools::AdditionalProperties::Boolean(false))
        );
    }
}

#[test]
fn operation_errors_do_not_mutate_or_invent_a_map() {
    let read = serde_json::from_value(json!({"tool": "read_map", "arguments": {}})).unwrap();
    assert_eq!(
        apply_map_operation(None, "map-session", read),
        Err(MapOperationApplyError::MapNotInitialized {
            operation: "read_map"
        })
    );

    let initial: MapOperation = serde_json::from_value(initialize_value()).unwrap();
    let MapOperationEffect::Candidate(map) =
        apply_map_operation(None, "map-session", initial.clone()).unwrap()
    else {
        panic!("initialize must produce a candidate")
    };
    assert_eq!(
        apply_map_operation(Some(&map), "map-session", initial),
        Err(MapOperationApplyError::MapAlreadyInitialized {
            map_id: "map-session".into()
        })
    );
}
