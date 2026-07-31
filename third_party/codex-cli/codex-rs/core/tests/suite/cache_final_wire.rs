use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standard_session_reaches_chat_completions_final_wire() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let stream = concat!(
        "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,",
        "\"delta\":{\"content\":\"done\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,",
        "\"delta\":{},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":1,",
        "\"total_tokens\":11,\"prompt_tokens_details\":{\"cached_tokens\":0}}}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(stream, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(|config| {
            config.model_provider.wire_api = WireApi::ChatCompletions;
            config.model = Some("deepseek-v4-flash".to_string());
        })
        .build(&server)
        .await?;
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "inspect final wire".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = server
        .received_requests()
        .await
        .expect("wiremock request capture");
    let body: Value = serde_json::from_slice(&requests[0].body)?;
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages[0]["role"], "system");
    assert!(messages.iter().any(|message| {
        message["role"] == "user"
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("inspect final wire"))
    }));
    assert!(
        body["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty())
    );
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["model"], "deepseek-v4-flash");
    Ok(())
}
