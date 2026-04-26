use serde_json::json;
use whalecode_model::{
    parse_sse_stream, parse_sse_stream_with_observer, response_from_stream_events, ChatMessage,
    DeepSeekChatRequest, DeepSeekConfig, ModelStreamEvent, ReasoningEffort, ThinkingMode,
    DEEPSEEK_DEFAULT_BASE_URL, DEEPSEEK_DEFAULT_MODEL,
};

#[test]
fn builds_streaming_request_with_thinking_enabled() {
    let config = DeepSeekConfig {
        base_url: DEEPSEEK_DEFAULT_BASE_URL.to_owned(),
        api_key: None,
        model: DEEPSEEK_DEFAULT_MODEL.to_owned(),
        thinking: ThinkingMode::Enabled,
        reasoning_effort: ReasoningEffort::High,
    };

    let request =
        DeepSeekChatRequest::streaming(&config, vec![ChatMessage::user("inspect this repository")]);
    let value = serde_json::to_value(request).expect("serialize request");

    assert_eq!(value["model"], "deepseek-v4-flash");
    assert_eq!(value["stream"], true);
    assert_eq!(value["thinking"]["type"], "enabled");
    assert_eq!(value["reasoning_effort"], "high");
    assert_eq!(value["messages"][0]["role"], "user");
    assert_eq!(value["messages"][0]["content"], "inspect this repository");
}

#[test]
fn builds_tool_request_with_auto_choice() {
    let config = DeepSeekConfig::from_env();
    let request = DeepSeekChatRequest::streaming(&config, vec![ChatMessage::user("task")])
        .with_tools(vec![json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a UTF-8 file",
                "parameters": {"type": "object"}
            }
        })]);
    let value = serde_json::to_value(request).expect("serialize request");

    assert_eq!(value["tool_choice"], "auto");
    assert_eq!(value["tools"][0]["function"]["name"], "read_file");
}

#[test]
fn parses_text_reasoning_and_done_sse_events() {
    let input = r#"data: {"choices":[{"delta":{"reasoning_content":"thinking"},"finish_reason":null,"index":0}]}

data: {"choices":[{"delta":{"content":"Hello"},"finish_reason":null,"index":0}]}

data: {"choices":[{"delta":{"content":" world"},"finish_reason":null,"index":0}]}

data: [DONE]

"#;

    let events = parse_sse_stream(input).expect("parse sse");

    assert_eq!(
        events,
        vec![
            ModelStreamEvent::ReasoningDelta("thinking".to_owned()),
            ModelStreamEvent::TextDelta("Hello".to_owned()),
            ModelStreamEvent::TextDelta(" world".to_owned()),
            ModelStreamEvent::Finished,
        ]
    );
    let response = response_from_stream_events(events);
    assert_eq!(response.final_text, "Hello world");
}

#[test]
fn observes_sse_events_as_they_are_parsed() {
    let input = r#"data: {"choices":[{"delta":{"content":"Hello"},"finish_reason":null,"index":0}]}

data: {"choices":[{"delta":{"content":" stream"},"finish_reason":null,"index":0}]}

data: [DONE]

"#;
    let mut observed = Vec::new();

    let events = parse_sse_stream_with_observer(input, |event| observed.push(event.clone()))
        .expect("parse sse");

    assert_eq!(events, observed);
    assert_eq!(
        observed,
        vec![
            ModelStreamEvent::TextDelta("Hello".to_owned()),
            ModelStreamEvent::TextDelta(" stream".to_owned()),
            ModelStreamEvent::Finished,
        ]
    );
}

#[test]
fn parses_tool_call_deltas() {
    let input = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"read_file","arguments":"{\"path\""}}]},"finish_reason":null,"index":0}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":":\"README.md\"}"}}]},"finish_reason":null,"index":0}]}

data: [DONE]

"#;

    let events = parse_sse_stream(input).expect("parse tool calls");

    assert_eq!(
        events,
        vec![
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
                arguments_delta: ":\"README.md\"}".to_owned(),
            },
            ModelStreamEvent::Finished,
        ]
    );
}

#[test]
fn reports_malformed_sse_json() {
    let err = parse_sse_stream("data: {not-json}\n\n").expect_err("malformed json");

    assert!(err.to_string().contains("malformed SSE JSON"));
}
