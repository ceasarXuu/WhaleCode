use super::*;
use codex_protocol::models::FunctionCallOutputPayload;

fn bootstrap_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "taskspace_control".into(),
        namespace: None,
        arguments: serde_json::json!({
            "action": "initialize_map",
            "root": {"node_id": "root", "goal": "solve"},
            "initial_work_node": {"node_id": "node-1", "goal": "inspect"},
            "additional_work_nodes": [],
            "finish_identity": {"id": "finish"},
            "edges": [{"from": "root", "to": "node-1"}, {"from": "node-1", "to": "finish"}],
            "required_next_call": "ordinary_tool"
        })
        .to_string(),
        call_id: call_id.into(),
    }
}

fn committed_control_output(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.into(),
        output: FunctionCallOutputPayload::from_text(
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultV2",
                "action": "initialize_map",
                "status": "committed",
                "success": true,
                "state_commit": true,
                "partial_commit": false,
                "canonical_revision": 1,
                "submitted_expected_revision": null,
                "committed_revision": 1,
                "delta": {
                    "map_id": "map-1",
                    "committed_revision": 1,
                    "graph_event_refs": [],
                    "node_detail_event_refs": []
                },
                "steps": [{
                    "kind": "map_initialized",
                    "map_id": "map-1",
                    "revision": 1
                }],
                "read": null,
                "error": null
            })
            .to_string(),
        ),
    }
}

fn nested_exec_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "exec_command".into(),
        namespace: None,
        arguments: serde_json::json!({"cmd": "pwd"}).to_string(),
        call_id: call_id.into(),
    }
}

fn nested_output(call_id: &str, success: Option<bool>) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.into(),
        output: FunctionCallOutputPayload {
            body: codex_protocol::models::FunctionCallOutputBody::Text("raw output".into()),
            success,
        },
    }
}

#[test]
fn nested_call_and_output_are_independent_events_linked_to_outer_control() {
    let mut store = TaskSpaceEventStore::new();
    let outer_id = "outer-control";
    store
        .record_item(
            &ResponseItem::FunctionCall {
                id: None,
                name: "taskspace_control".into(),
                namespace: None,
                arguments: r#"{"action":"initialize_map"}"#.into(),
                call_id: outer_id.into(),
            },
            None,
            None,
            1,
        )
        .expect("outer call");
    let nested_call = store
        .record_item(
            &ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".into(),
                namespace: None,
                arguments: r#"{"cmd":"pwd"}"#.into(),
                call_id: "outer-control:nested:0".into(),
            },
            Some("node-1"),
            Some(outer_id.into()),
            2,
        )
        .expect("nested call");
    let nested_output = store
        .record_item(
            &ResponseItem::FunctionCallOutput {
                call_id: "outer-control:nested:0".into(),
                output: FunctionCallOutputPayload::from_text("raw output".into()),
            },
            Some("node-1"),
            Some(outer_id.into()),
            3,
        )
        .expect("nested output");

    assert_eq!(nested_call.parent_call_id.as_deref(), Some(outer_id));
    assert_eq!(nested_output.parent_call_id.as_deref(), Some(outer_id));
    assert_eq!(
        nested_call.owner,
        TaskSpaceEventOwner::Node("node-1".into())
    );
    assert_eq!(nested_output.owner, nested_call.owner);
    assert_eq!(
        store.linearize()[1],
        nested_call.to_response_item().unwrap()
    );
    assert_eq!(
        store.linearize()[2],
        nested_output.to_response_item().unwrap()
    );
}

#[test]
fn successful_bootstrap_keeps_identity_bearing_outer_pair_and_nested_pair_visible() {
    let mut store = TaskSpaceEventStore::new();
    let outer = bootstrap_call("outer-control");
    let outer_output = committed_control_output("outer-control");
    let nested_call = nested_exec_call("outer-control:nested:0");
    let nested_output = nested_output("outer-control:nested:0", Some(true));

    store.record_item(&outer, None, None, 1).unwrap();
    store.record_item(&outer_output, None, None, 2).unwrap();
    store
        .record_item(
            &nested_call,
            Some("node-1"),
            Some("outer-control".into()),
            3,
        )
        .unwrap();
    store
        .record_item(
            &nested_output,
            Some("node-1"),
            Some("outer-control".into()),
            4,
        )
        .unwrap();

    assert_eq!(store.events().len(), 4);
    assert_eq!(
        store.linearize(),
        vec![outer, outer_output, nested_call, nested_output]
    );
}

#[test]
fn bootstrap_outer_pair_stays_visible_when_nested_pair_is_incomplete_or_failed() {
    for (include_nested_output, nested_success, expected_len) in
        [(false, Some(true), 3), (true, Some(false), 4)]
    {
        let mut store = TaskSpaceEventStore::new();
        let outer = bootstrap_call("outer-control");
        let outer_output = committed_control_output("outer-control");
        let nested_call = nested_exec_call("outer-control:nested:0");
        store.record_item(&outer, None, None, 1).unwrap();
        store.record_item(&outer_output, None, None, 2).unwrap();
        store
            .record_item(
                &nested_call,
                Some("node-1"),
                Some("outer-control".into()),
                3,
            )
            .unwrap();
        if include_nested_output {
            store
                .record_item(
                    &nested_output("outer-control:nested:0", nested_success),
                    Some("node-1"),
                    Some("outer-control".into()),
                    4,
                )
                .unwrap();
        }

        let visible = store.linearize();
        assert_eq!(visible.len(), expected_len);
        assert_eq!(visible[0], outer);
        assert_eq!(visible[1], outer_output);
    }
}

#[test]
fn bootstrap_outer_pair_stays_visible_when_nested_orphan_count_is_nonzero() {
    let mut store = TaskSpaceEventStore::new();
    let outer = bootstrap_call("outer-control");
    let outer_output = committed_control_output("outer-control");
    store.record_item(&outer, None, None, 1).unwrap();
    store.record_item(&outer_output, None, None, 2).unwrap();
    store
        .record_item(
            &nested_exec_call("outer-control:nested:0"),
            Some("node-1"),
            Some("outer-control".into()),
            3,
        )
        .unwrap();
    store
        .record_item(
            &nested_output("outer-control:nested:0", Some(true)),
            Some("node-1"),
            Some("outer-control".into()),
            4,
        )
        .unwrap();
    store
        .record_item(
            &nested_exec_call("outer-control:nested:orphan"),
            Some("node-1"),
            Some("outer-control".into()),
            5,
        )
        .unwrap();

    let visible = store.linearize();
    assert_eq!(visible[0], outer);
    assert_eq!(visible[1], outer_output);
}
