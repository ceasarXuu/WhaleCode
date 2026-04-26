use whalecode_model::{collect_model_output, CollectedToolCall, ModelStreamEvent};

#[test]
fn aggregates_split_tool_call_deltas_by_index() {
    let output = collect_model_output(&[
        ModelStreamEvent::ReasoningDelta("think ".to_owned()),
        ModelStreamEvent::TextDelta("I will read".to_owned()),
        ModelStreamEvent::ToolCallDelta {
            index: 0,
            id: Some("call_1".to_owned()),
            name: "read_file".to_owned(),
            arguments_delta: "{\"path\"".to_owned(),
        },
        ModelStreamEvent::ToolCallDelta {
            index: 0,
            id: None,
            name: String::new(),
            arguments_delta: ":\"src/lib.rs\"}".to_owned(),
        },
        ModelStreamEvent::Finished,
    ]);

    assert_eq!(output.text, "I will read");
    assert_eq!(output.reasoning, "think ");
    assert!(output.finished);
    assert_eq!(
        output.tool_calls,
        vec![CollectedToolCall {
            index: 0,
            id: "call_1".to_owned(),
            name: "read_file".to_owned(),
            arguments: "{\"path\":\"src/lib.rs\"}".to_owned(),
        }]
    );
}

#[test]
fn uses_stable_fallback_id_when_stream_omits_id() {
    let output = collect_model_output(&[ModelStreamEvent::ToolCallDelta {
        index: 3,
        id: None,
        name: "list_files".to_owned(),
        arguments_delta: "{}".to_owned(),
    }]);

    assert_eq!(output.tool_calls[0].id, "tool-3");
}
