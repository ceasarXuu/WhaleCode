use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use std::path::PathBuf;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const CHAT_COMPLETION_STREAM: &str = concat!(
    "data: {\"id\":\"chatcmpl-cache-contract\",\"choices\":[{\"index\":0,",
    "\"delta\":{\"content\":\"turn complete\"},\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-cache-contract\",\"choices\":[{\"index\":0,",
    "\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":1,",
    "\"total_tokens\":11,\"prompt_tokens_details\":{\"cached_tokens\":0}}}\n\n",
    "data: [DONE]\n\n"
);

async fn submit_turn(
    test: &core_test_support::test_codex::TestCodex,
    text: &str,
) -> anyhow::Result<()> {
    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: text.to_string(),
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
    Ok(())
}

fn replace_tag_value(text: &str, tag: &str, replacement: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let Some(start) = text.find(&open) else {
        return text.to_string();
    };
    let value_start = start + open.len();
    let Some(relative_end) = text[value_start..].find(&close) else {
        return text.to_string();
    };
    let value_end = value_start + relative_end;
    format!(
        "{}{}{}",
        &text[..value_start],
        replacement,
        &text[value_end..]
    )
}

fn stabilize_fixture_inputs(value: &mut Value, path_prefixes: &[(&str, &str)]) {
    match value {
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| stabilize_fixture_inputs(value, path_prefixes)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| stabilize_fixture_inputs(value, path_prefixes)),
        Value::String(text) => {
            if text.contains("<environment_context>") {
                *text = replace_tag_value(text, "current_date", "FIXED_CURRENT_DATE");
                *text = replace_tag_value(text, "timezone", "FIXED_TIMEZONE");
            }
            for (prefix, replacement) in path_prefixes {
                *text = text.replace(prefix, replacement);
            }
        }
        _ => {}
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standard_request_pair_preserves_the_complete_prefix() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(CHAT_COMPLETION_STREAM, "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(|config| {
            config.model_provider.wire_api = WireApi::ChatCompletions;
            config.model = Some("deepseek-v4-flash".to_string());
            config.cwd =
                AbsolutePathBuf::try_from(PathBuf::from("/tmp")).expect("fixed cache contract cwd");
        })
        .build(&server)
        .await?;
    submit_turn(&test, "cache contract turn one").await?;
    submit_turn(&test, "cache contract turn two").await?;

    let requests = server
        .received_requests()
        .await
        .expect("two final-wire requests");
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
    let first_messages = first.structured_body["messages"]
        .as_array()
        .expect("first messages");
    let second_messages = second.structured_body["messages"]
        .as_array()
        .expect("second messages");
    assert_eq!(&second_messages[..first_messages.len()], first_messages);
    assert_eq!(
        first.structured_body["tools"],
        second.structured_body["tools"]
    );
    assert_eq!(
        first.structured_body["tool_choice"],
        second.structured_body["tool_choice"]
    );
    let mut inserted_message = first.clone();
    inserted_message.structured_body["messages"]
        .as_array_mut()
        .expect("mutable messages")
        .insert(
            1,
            serde_json::json!({"role": "developer", "content": "inserted mutation"}),
        );
    assert_ne!(inserted_message, first);

    let mut snapshot = serde_json::json!({
        "provider_identity": {
            "provider_id": "deepseek",
            "wire_api": "chat_completions",
            "endpoint_path": "/v1/chat/completions"
        },
        "request_1": first.structured_body,
        "request_2": second.structured_body,
    });
    let codex_home = test.codex_home_path().to_string_lossy();
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("Codex source root")
        .to_string_lossy()
        .into_owned();
    stabilize_fixture_inputs(
        &mut snapshot,
        &[
            (codex_home.as_ref(), "<CODEX_HOME>"),
            (&source_root, "<CODEX_SOURCE_ROOT>"),
        ],
    );
    insta::assert_snapshot!(
        "standard_two_request_final_wire",
        serde_json::to_string_pretty(&snapshot)?
    );
    Ok(())
}
