use super::cache_payload_contract::configure_deepseek_responses;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::cache_payload::FinalWireEvidence;
use core_test_support::responses::sse_completed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

async fn capture_responses_body() -> anyhow::Result<Value> {
    let server = start_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(sse_completed("resp-final-wire"), "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let test = test_codex()
        .with_config(|config| {
            configure_deepseek_responses(config);
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
    let requests = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(requests) = server.received_requests().await
                && !requests.is_empty()
            {
                return requests;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timeout waiting for final-wire request"))?;
    test.codex.submit(Op::Shutdown).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::ShutdownComplete)
    })
    .await;

    let evidence = FinalWireEvidence::from_raw_body(&requests[0].body)?;
    assert_eq!(
        evidence,
        FinalWireEvidence::from_raw_body(&requests[0].body)?
    );
    assert!(evidence.render()?.contains(&evidence.raw_body_sha256));
    Ok(evidence.structured_body)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standard_session_reaches_responses_final_wire() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let body = capture_responses_body().await?;
    assert!(
        body["instructions"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
    let input = body["input"].as_array().expect("input array");
    assert!(
        input.iter().any(|item| {
            item["role"] == "user" && item.to_string().contains("inspect final wire")
        })
    );
    assert!(
        body["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty())
    );
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["model"], "deepseek-v4-flash");
    Ok(())
}
