use std::sync::Arc;
use std::time::Duration;

use codex_api::AuthProvider;
use codex_api::Provider;
use codex_api::ReqwestTransport;
use codex_api::ResponseEvent;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesClient;
use codex_api::ResponsesOptions;
use codex_api::RetryConfig;
use codex_api::ToolChoice;
use codex_api::WireApi;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeMode;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::create_image_generation_tool;
use codex_tools::create_tools_json_for_responses_api;
use futures::StreamExt;
use http::HeaderMap;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use super::execute_response_tool_sequence;
use crate::function_tool::FunctionCallError;
use crate::session::tests::make_session_and_context;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;

#[derive(Clone, Default)]
struct DummyAuth;

impl AuthProvider for DummyAuth {
    fn add_auth_headers(&self, _headers: &mut HeaderMap) {}
}

struct HostedImageHandler {
    base_url: String,
}

impl HostedImageHandler {
    fn provider(&self) -> Provider {
        Provider {
            name: "mvt-hosted".to_string(),
            base_url: self.base_url.clone(),
            wire_api: WireApi::Responses,
            query_params: None,
            headers: HeaderMap::new(),
            retry: RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(1),
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
            stream_idle_timeout: Duration::from_secs(2),
        }
    }
}

impl ToolHandler for HostedImageHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolPayload::Function { arguments } = invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "hosted image adapter requires function arguments".to_string(),
            ));
        };
        let arguments: serde_json::Value = serde_json::from_str(&arguments).map_err(|error| {
            FunctionCallError::RespondToModel(format!(
                "hosted image adapter arguments are invalid: {error}"
            ))
        })?;
        let prompt = arguments
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "hosted image adapter requires prompt".to_string(),
                )
            })?;
        let request = ResponsesApiRequest {
            model: "mvt-provider-model".to_string(),
            instructions: String::new(),
            input: vec![ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: prompt.to_string(),
                }],
                end_turn: None,
                phase: None,
            }],
            tools: create_tools_json_for_responses_api(&[create_image_generation_tool("png")])
                .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?,
            tool_choice: ToolChoice::Required,
            parallel_tool_calls: false,
            reasoning: None,
            store: false,
            stream: true,
            include: Vec::new(),
            service_tier: None,
            prompt_cache_key: None,
            text: None,
            client_metadata: None,
        };
        let client = ResponsesClient::new(
            ReqwestTransport::new(reqwest::Client::new()),
            self.provider(),
            Arc::new(DummyAuth),
        );
        let mut stream = client
            .stream_request(request, ResponsesOptions::default())
            .await
            .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
        let mut results = Vec::new();
        while let Some(event) = stream.next().await {
            match event.map_err(|error| FunctionCallError::RespondToModel(error.to_string()))? {
                ResponseEvent::OutputItemDone(ResponseItem::ImageGenerationCall {
                    id,
                    status,
                    revised_prompt,
                    result: image,
                }) => {
                    results.push(serde_json::json!({
                        "id": id,
                        "status": status,
                        "revised_prompt": revised_prompt,
                        "result": image
                    }));
                }
                ResponseEvent::Completed { .. } => break,
                _ => {}
            }
        }
        if results.len() != 1 || results[0]["status"] != "completed" {
            return Err(FunctionCallError::RespondToModel(format!(
                "hosted image response requires one completed image_generation_call; got {}",
                results.len()
            )));
        }
        let result = results.pop().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "hosted image response did not contain image_generation_call".to_string(),
            )
        })?;
        Ok(FunctionToolOutput::from_text(
            result.to_string(),
            Some(true),
        ))
    }
}

fn function_call(name: &str, call_id: &str, arguments: impl Into<String>) -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain(name),
        dispatch_tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.into(),
        },
    }
}

fn hosted_router(base_url: String) -> ToolRouter {
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec_with_parallel_support(
        ToolSpec::Function(ResponsesApiTool {
            name: "mvt_hosted_image".to_string(),
            description: String::new(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::default(),
            output_schema: None,
        }),
        true,
    );
    builder.register_handler(
        "mvt_hosted_image",
        Arc::new(HostedImageHandler { base_url }),
    );
    ToolRouter::from_builder_for_test(builder)
}

fn hosted_sse() -> String {
    [
        serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "image_generation_call",
                "id": "ig-mvt-4",
                "status": "completed",
                "revised_prompt": "A precise blue square",
                "result": "Zm9v"
            }
        }),
        serde_json::json!({
            "type": "response.completed",
            "response": {
                "id": "resp-mvt-4",
                "usage": {
                    "input_tokens": 0,
                    "input_tokens_details": null,
                    "output_tokens": 0,
                    "output_tokens_details": null,
                    "total_tokens": 0
                }
            }
        }),
    ]
    .into_iter()
    .map(|event| {
        format!(
            "event: {}\ndata: {event}\n\n",
            event["type"].as_str().unwrap()
        )
    })
    .collect()
}

#[tokio::test]
async fn hosted_work_uses_one_constrained_responses_request_after_map_preflight() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(hosted_sse()),
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
    let calls = vec![
        function_call(
            "taskspace_control",
            "map-prelude",
            serde_json::json!({
                "action": "initialize_and_execute",
                "root": {"node_id": "root", "goal": "Generate an image"},
                "work_nodes": [{"node_id": "image", "goal": "Generate the image"}],
                "finish": {"node_id": "finish", "goal": "Finish"},
                "edges": [
                    {"from": "root", "to": "image"},
                    {"from": "image", "to": "finish"}
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
    ];

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("hosted Work sequence");

    let requests = server
        .received_requests()
        .await
        .expect("captured hosted request");
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).expect("hosted request JSON");
    assert_eq!(
        body["tools"],
        serde_json::json!([{
            "type": "image_generation",
            "output_format": "png"
        }])
    );
    assert_eq!(body["tool_choice"], "required");
    assert_eq!(body["parallel_tool_calls"], false);
    assert_eq!(body["model"], "mvt-provider-model");
    assert_eq!(body["input"].as_array().map(Vec::len), Some(1));
    assert!(body.get("previous_response_id").is_none());
    assert!(body.get("instructions").is_none());
    let hosted_output = outcome
        .outputs
        .iter()
        .find_map(|output| match output {
            ResponseInputItem::FunctionCallOutput { call_id, output }
                if call_id == "hosted-image" =>
            {
                output.text_content()
            }
            _ => None,
        })
        .expect("hosted result pairing");
    let hosted_output: serde_json::Value =
        serde_json::from_str(hosted_output).expect("hosted result JSON");
    assert_eq!(hosted_output["id"], "ig-mvt-4");
    assert_eq!(hosted_output["status"], "completed");
    assert_eq!(hosted_output["result"], "Zm9v");

    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("snapshot")
        .map
        .expect("initialized map");
    assert_eq!(map.results.len(), 1);
    assert_eq!(map.results[0].node_id, "image");
    assert!(!map.results[0].is_error);
    assert!(map.reservations.is_empty());
}

#[tokio::test]
async fn invalid_map_execute_rejects_before_the_hosted_request() {
    let server = MockServer::start().await;
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
    let calls = vec![
        function_call(
            "taskspace_control",
            "invalid-control",
            serde_json::json!({
                "action": "execute",
                "expected_revision": 0,
                "actions": [{"node_id": "missing", "tool": "mvt_hosted_image"}]
            })
            .to_string(),
        ),
        function_call(
            "mvt_hosted_image",
            "hosted-must-not-run",
            serde_json::json!({"prompt": "Must not execute"}).to_string(),
        ),
    ];

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("preflight rejection response");

    assert!(matches!(
        outcome.outputs.last(),
        Some(ResponseInputItem::Message { .. })
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("captured requests")
            .is_empty()
    );
    assert!(
        session
            .canonical_action_map_snapshot()
            .await
            .expect("snapshot")
            .map
            .is_none()
    );
}
