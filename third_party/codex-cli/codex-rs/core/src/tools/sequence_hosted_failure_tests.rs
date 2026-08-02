use std::sync::Arc;

use codex_protocol::models::ResponseInputItem;
use codex_protocol::protocol::MapRuntimeMode;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use super::execute_response_tool_sequence;
use super::hosted_wire_tests::function_call;
use super::hosted_wire_tests::hosted_router;
use crate::session::tests::make_session_and_context;
use crate::tools::parallel::ToolCallRuntime;
use crate::turn_diff_tracker::TurnDiffTracker;

fn failed_sse() -> String {
    let event = serde_json::json!({
        "type": "response.failed",
        "response": {
            "id": "resp-failed",
            "status": "failed",
            "error": {
                "code": "invalid_prompt",
                "message": "The hosted request was rejected"
            }
        }
    });
    format!("event: response.failed\ndata: {event}\n\n")
}

fn incomplete_sse() -> String {
    let event = serde_json::json!({
        "type": "response.created",
        "response": {"id": "resp-incomplete"}
    });
    format!("event: response.created\ndata: {event}\n\n")
}

fn hosted_chain_calls() -> Vec<crate::tools::router::ToolCall> {
    vec![
        function_call(
            "taskspace_control",
            "map-prelude",
            serde_json::json!({
                "action": "initialize_and_execute",
                "root": {"node_id": "root", "goal": "Generate an image"},
                "work_nodes": [
                    {"node_id": "image", "goal": "Generate the image"},
                    {"node_id": "after", "goal": "Use the image result"}
                ],
                "finish": {"node_id": "finish", "goal": "Finish"},
                "edges": [
                    {"from": "root", "to": "image"},
                    {"from": "image", "to": "after"},
                    {"from": "after", "to": "finish"}
                ],
                "actions": [{"node_id": "image", "tool": "mvt_hosted_image"}]
            })
            .to_string(),
        ),
        function_call(
            "mvt_hosted_image",
            "hosted-image",
            serde_json::json!({"prompt": "A blue square"}).to_string(),
        ),
    ]
}

fn hosted_status(outputs: &[ResponseInputItem]) -> (&str, serde_json::Value) {
    let output = outputs
        .iter()
        .find_map(|output| match output {
            ResponseInputItem::FunctionCallOutput { call_id, output }
                if call_id == "hosted-image" =>
            {
                Some(output)
            }
            _ => None,
        })
        .expect("hosted output pairing");
    let text = output.text_content().expect("hosted output text");
    (
        output
            .success
            .map_or("missing", |success| if success { "true" } else { "false" }),
        serde_json::from_str(text).expect("hosted outcome JSON"),
    )
}

#[tokio::test]
async fn hosted_failure_and_unknown_outcome_are_distinct_and_never_retried() {
    for (label, response_body, expected_status) in [
        ("provider-rejected", failed_sse(), "failed"),
        ("stream-interrupted", incomplete_sse(), "outcome_unknown"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response_body),
            )
            .expect(1)
            .mount(&server)
            .await;
        let (session, turn) = make_session_and_context().await;
        session
            .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
            .await;
        let session = Arc::new(session);
        let runtime = ToolCallRuntime::new(
            Arc::new(hosted_router(server.uri())),
            Arc::clone(&session),
            Arc::new(turn),
            Arc::new(Mutex::new(TurnDiffTracker::new())),
        );

        let outcome =
            execute_response_tool_sequence(runtime, hosted_chain_calls(), CancellationToken::new())
                .await
                .expect("hosted failure sequence");

        assert_eq!(
            server
                .received_requests()
                .await
                .expect("captured requests")
                .len(),
            1,
            "{label} must not retry"
        );
        let (success, hosted) = hosted_status(&outcome.outputs);
        assert_eq!(success, "false", "{label}");
        assert_eq!(hosted["status"], expected_status, "{label}");
        let map = session
            .canonical_action_map_snapshot()
            .await
            .expect("snapshot")
            .map
            .expect("initialized map");
        assert_eq!(map.results.len(), 1, "{label}");
        assert!(map.results[0].is_error, "{label}");
        assert!(map.reservations.is_empty(), "{label}");
        assert_eq!(
            map.nodes
                .iter()
                .find(|node| node.id == "image")
                .unwrap()
                .state,
            "ready",
            "{label}"
        );
        assert_eq!(
            map.nodes
                .iter()
                .find(|node| node.id == "after")
                .unwrap()
                .state,
            "waiting",
            "{label}"
        );
    }
}
