use super::normalize::colocate_call_outputs;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;

fn call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: call_id.to_string(),
    }
}

fn output(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.to_string(),
        output: FunctionCallOutputPayload::from_text(format!("output {call_id}")),
    }
}

fn layout(items: &[ResponseItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            ResponseItem::FunctionCall { call_id, .. } => format!("call:{call_id}"),
            ResponseItem::FunctionCallOutput { call_id, .. } => format!("output:{call_id}"),
            _ => "other".to_string(),
        })
        .collect()
}

#[test]
fn colocate_call_outputs_keeps_parallel_call_block_before_outputs() {
    let mut items = vec![
        call("call_1"),
        call("call_2"),
        output("call_1"),
        output("call_2"),
    ];

    colocate_call_outputs(&mut items);

    assert_eq!(
        layout(&items),
        vec![
            "call:call_1",
            "call:call_2",
            "output:call_1",
            "output:call_2"
        ]
    );
}

#[test]
fn colocate_call_outputs_preserves_sequential_tool_rounds() {
    let mut items = vec![
        call("call_1"),
        output("call_1"),
        call("call_2"),
        output("call_2"),
    ];

    colocate_call_outputs(&mut items);

    assert_eq!(
        layout(&items),
        vec![
            "call:call_1",
            "output:call_1",
            "call:call_2",
            "output:call_2"
        ]
    );
}

#[test]
fn colocate_call_outputs_pulls_early_output_after_matching_call() {
    let mut items = vec![output("call_1"), call("call_1")];

    colocate_call_outputs(&mut items);

    assert_eq!(layout(&items), vec!["call:call_1", "output:call_1"]);
}
