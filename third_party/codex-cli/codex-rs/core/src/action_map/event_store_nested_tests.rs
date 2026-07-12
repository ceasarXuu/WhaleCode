use super::*;
use codex_protocol::models::FunctionCallOutputPayload;

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
                arguments: r#"{"action":"initialize_then_actions"}"#.into(),
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
