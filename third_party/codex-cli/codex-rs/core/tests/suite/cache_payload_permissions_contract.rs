use super::cache_payload_contract::completed_response_stream;
use super::cache_payload_contract::configure_deepseek_responses;
use super::cache_payload_contract::provider_identity;
use super::cache_payload_contract::stabilize_fixture_inputs;
use super::cache_payload_contract::submit_turn;
use super::cache_payload_contract::value_contains_text;
use codex_core::config::Constrained;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use codex_utils_absolute_path::AbsolutePathBuf;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::cache_payload::render_cache_snapshot;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use serde_json::Value;
use std::path::PathBuf;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

async fn capture_permission_request_pair(restricted: bool) -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(
                    completed_response_stream("resp-permissions-contract"),
                    "text/event-stream",
                ),
        )
        .expect(2)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(move |config| {
            configure_deepseek_responses(config);
            config.cwd = AbsolutePathBuf::try_from(PathBuf::from("/tmp"))
                .expect("fixed permissions contract cwd");
            if restricted {
                config.permissions.approval_policy =
                    Constrained::allow_any(AskForApproval::OnRequest);
                config.permissions.sandbox_policy =
                    Constrained::allow_any(SandboxPolicy::new_read_only_policy());
            }
        })
        .build(&server)
        .await?;
    submit_turn(&test, "permissions contract turn one").await?;
    submit_turn(&test, "permissions contract turn two").await?;

    let requests = server
        .received_requests()
        .await
        .expect("permissions final-wire requests");
    assert_eq!(requests.len(), 2);
    let first = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    let second = FinalWireEvidence::from_raw_body(&requests[1].body)?;
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
    Ok(snapshot)
}

fn replace_permissions_section(text: &str) -> String {
    const START: &str = "<permissions instructions>";
    const END: &str = "</permissions instructions>";
    let Some(start) = text.find(START) else {
        return text.to_string();
    };
    let Some(relative_end) = text[start..].find(END) else {
        return text.to_string();
    };
    let end = start + relative_end + END.len();
    format!(
        "{}<PERMISSIONS_INSTRUCTIONS>{}",
        &text[..start],
        &text[end..]
    )
}

fn normalize_permissions(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(normalize_permissions),
        Value::Object(values) => values.values_mut().for_each(normalize_permissions),
        Value::String(text) => *text = replace_permissions_section(text),
        _ => {}
    }
}

fn permissions_section_count(request: &Value) -> usize {
    request["input"]
        .as_array()
        .expect("request input")
        .iter()
        .filter(|item| value_contains_text(item, "<permissions instructions>"))
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permission_context_is_the_only_wire_difference() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let default = capture_permission_request_pair(false).await?;
    let restricted = capture_permission_request_pair(true).await?;

    for request_key in ["request_1", "request_2"] {
        assert_eq!(permissions_section_count(&default[request_key]), 1);
        assert_eq!(permissions_section_count(&restricted[request_key]), 1);
    }
    let mut normalized_default = default;
    let mut normalized_restricted = restricted.clone();
    normalize_permissions(&mut normalized_default);
    normalize_permissions(&mut normalized_restricted);
    assert_eq!(normalized_restricted, normalized_default);

    insta::assert_snapshot!(
        "restricted_permissions_two_request_final_wire",
        render_cache_snapshot("restricted_permissions_two_request_final_wire", &restricted)?
    );
    Ok(())
}
