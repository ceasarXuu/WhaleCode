use codex_core::config::Config;
use codex_model_provider_info::DEEPSEEK_PROVIDER_ID;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::cache_payload::render_cache_snapshot;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::sse;
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

pub(super) fn completed_response_stream(response_id: &str) -> String {
    sse(vec![
        ev_response_created(response_id),
        ev_completed(response_id),
    ])
}

pub(super) async fn submit_turn(
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

fn stabilize_projection_canonical_sha256(text: &str) -> String {
    const PREFIX: &str = "- canonical_sha256: ";
    if !text.contains("TaskSpaceMapProjectionR8V1:") {
        return text.to_string();
    }

    text.split_inclusive('\n')
        .map(|line| {
            let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
            let Some(hash) = line_without_newline.strip_prefix(PREFIX) else {
                return line.to_string();
            };
            assert_eq!(hash.len(), 64, "canonical projection hash length");
            assert!(
                hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "canonical projection hash format"
            );
            format!(
                "{PREFIX}<PROJECTION_CANONICAL_SHA256>{}",
                if line.ends_with('\n') { "\n" } else { "" }
            )
        })
        .collect()
}

pub(super) fn stabilize_fixture_inputs(value: &mut Value, path_prefixes: &[(&str, &str)]) {
    match value {
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| stabilize_fixture_inputs(value, path_prefixes)),
        Value::Object(values) => {
            for (key, value) in values {
                match key.as_str() {
                    "prompt_cache_key" => {
                        *value = Value::String("<PROMPT_CACHE_KEY>".to_string());
                    }
                    "x-codex-installation-id" => {
                        *value = Value::String("<INSTALLATION_ID>".to_string());
                    }
                    _ => stabilize_fixture_inputs(value, path_prefixes),
                }
            }
        }
        Value::String(text) => {
            if text.contains("<environment_context>") {
                *text = replace_tag_value(text, "current_date", "FIXED_CURRENT_DATE");
                *text = replace_tag_value(text, "timezone", "FIXED_TIMEZONE");
            }
            *text = stabilize_projection_canonical_sha256(text);
            for (prefix, replacement) in path_prefixes {
                if prefix.is_empty() {
                    continue;
                }
                *text = text.replace(prefix, replacement);
            }
        }
        _ => {}
    }
}

pub(super) fn value_contains_text(value: &Value, marker: &str) -> bool {
    match value {
        Value::Array(values) => values
            .iter()
            .any(|value| value_contains_text(value, marker)),
        Value::Object(values) => values
            .values()
            .any(|value| value_contains_text(value, marker)),
        Value::String(text) => text.contains(marker),
        _ => false,
    }
}

pub(super) fn configure_deepseek_responses(config: &mut Config) {
    let base_url = config.model_provider.base_url.clone();
    let mut provider = ModelProviderInfo::create_deepseek_provider();
    provider.base_url = base_url;
    provider.env_key = None;
    provider.env_key_instructions = None;
    provider.experimental_bearer_token = Some("test-deepseek-key".to_string());
    config.model_provider_id = DEEPSEEK_PROVIDER_ID.to_string();
    config.model_provider = provider;
    config.model = Some("deepseek-v4-flash".to_string());
}

pub(super) fn provider_identity(config: &Config) -> Value {
    assert!(config.model_provider.is_deepseek());
    let endpoint_path = match config.model_provider.wire_api {
        WireApi::Responses => "/v1/responses",
        WireApi::ChatCompletions => "/v1/chat/completions",
    };
    serde_json::json!({
        "provider_id": config.model_provider_id,
        "wire_api": config.model_provider.wire_api.to_string(),
        "endpoint_path": endpoint_path
    })
}

#[test]
fn fixture_stabilization_ignores_empty_prefixes() {
    let mut value = Value::String("unchanged".to_string());
    stabilize_fixture_inputs(&mut value, &[("", "<EMPTY>")]);
    assert_eq!(value, Value::String("unchanged".to_string()));
}

#[test]
fn fixture_stabilization_replaces_only_valid_projection_canonical_hash() {
    let hash = "a".repeat(64);
    let mut value = Value::String(format!(
        "TaskSpaceMapProjectionR8V1:\n- canonical_sha256: {hash}\n- goal: keep"
    ));
    stabilize_fixture_inputs(&mut value, &[]);
    assert_eq!(
        value,
        Value::String(
            "TaskSpaceMapProjectionR8V1:\n- canonical_sha256: <PROJECTION_CANONICAL_SHA256>\n- goal: keep"
                .to_string()
        )
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standard_request_pair_preserves_the_complete_prefix() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-cache-contract"),
                    "text/event-stream",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(|config| {
            configure_deepseek_responses(config);
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
    let first_input = first.structured_body["input"]
        .as_array()
        .expect("first input");
    let second_input = second.structured_body["input"]
        .as_array()
        .expect("second input");
    assert_eq!(&second_input[..first_input.len()], first_input);
    assert_eq!(
        first.structured_body["tools"],
        second.structured_body["tools"]
    );
    assert_eq!(
        first.structured_body["tool_choice"],
        second.structured_body["tool_choice"]
    );
    let mut inserted_message = first.clone();
    inserted_message.structured_body["input"]
        .as_array_mut()
        .expect("mutable input")
        .insert(
            1,
            serde_json::json!({
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": "inserted mutation"}]
            }),
        );
    assert_ne!(inserted_message, first);

    let mut snapshot = serde_json::json!({
        "provider_identity": provider_identity(&test.config),
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
        render_cache_snapshot("standard_two_request_final_wire", &snapshot)?
    );
    Ok(())
}
