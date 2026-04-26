use crate::auth::SharedAuthProvider;
use crate::common::ResponseStream;
use crate::common::ResponsesApiRequest;
use crate::endpoint::session::EndpointSession;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::provider::WireApi;
use crate::requests::Compression;
use crate::requests::attach_item_ids;
use crate::requests::headers::build_conversation_headers;
use crate::requests::headers::insert_header;
use crate::requests::headers::subagent_header;
use crate::sse::spawn_chat_completions_stream;
use crate::sse::spawn_response_stream;
use crate::telemetry::SseTelemetry;
use codex_client::HttpTransport;
use codex_client::RequestCompression;
use codex_client::RequestTelemetry;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use http::HeaderMap;
use http::HeaderValue;
use http::Method;
use serde_json::Value;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::instrument;

pub struct ResponsesClient<T: HttpTransport> {
    session: EndpointSession<T>,
    sse_telemetry: Option<Arc<dyn SseTelemetry>>,
}

#[derive(Default)]
pub struct ResponsesOptions {
    pub conversation_id: Option<String>,
    pub session_source: Option<SessionSource>,
    pub extra_headers: HeaderMap,
    pub compression: Compression,
    pub turn_state: Option<Arc<OnceLock<String>>>,
}

impl<T: HttpTransport> ResponsesClient<T> {
    pub fn new(transport: T, provider: Provider, auth: SharedAuthProvider) -> Self {
        Self {
            session: EndpointSession::new(transport, provider, auth),
            sse_telemetry: None,
        }
    }

    pub fn with_telemetry(
        self,
        request: Option<Arc<dyn RequestTelemetry>>,
        sse: Option<Arc<dyn SseTelemetry>>,
    ) -> Self {
        Self {
            session: self.session.with_request_telemetry(request),
            sse_telemetry: sse,
        }
    }

    #[instrument(
        name = "responses.stream_request",
        level = "info",
        skip_all,
        fields(
            transport = "responses_http",
            http.method = "POST",
            api.path = "responses"
        )
    )]
    pub async fn stream_request(
        &self,
        request: ResponsesApiRequest,
        options: ResponsesOptions,
    ) -> Result<ResponseStream, ApiError> {
        if self.session.provider().wire_api == WireApi::ChatCompletions {
            return self.stream_chat_completions_request(request, options).await;
        }

        let ResponsesOptions {
            conversation_id,
            session_source,
            extra_headers,
            compression,
            turn_state,
        } = options;

        let mut body = serde_json::to_value(&request)
            .map_err(|e| ApiError::Stream(format!("failed to encode responses request: {e}")))?;
        if request.store && self.session.provider().is_azure_responses_endpoint() {
            attach_item_ids(&mut body, &request.input);
        }

        let mut headers = extra_headers;
        if let Some(ref conv_id) = conversation_id {
            insert_header(&mut headers, "x-client-request-id", conv_id);
        }
        headers.extend(build_conversation_headers(conversation_id));
        if let Some(subagent) = subagent_header(&session_source) {
            insert_header(&mut headers, "x-openai-subagent", &subagent);
        }

        self.stream(body, headers, compression, turn_state).await
    }

    async fn stream_chat_completions_request(
        &self,
        request: ResponsesApiRequest,
        options: ResponsesOptions,
    ) -> Result<ResponseStream, ApiError> {
        let ResponsesOptions {
            conversation_id,
            session_source,
            extra_headers,
            turn_state,
            ..
        } = options;

        let mut headers = extra_headers;
        if let Some(ref conv_id) = conversation_id {
            insert_header(&mut headers, "x-client-request-id", conv_id);
        }
        headers.extend(build_conversation_headers(conversation_id));
        if let Some(subagent) = subagent_header(&session_source) {
            insert_header(&mut headers, "x-openai-subagent", &subagent);
        }

        let body = build_chat_completions_body(request);
        let stream_response = self
            .session
            .stream_with(
                Method::POST,
                "chat/completions",
                headers,
                Some(body),
                |req| {
                    req.headers.insert(
                        http::header::ACCEPT,
                        HeaderValue::from_static("text/event-stream"),
                    );
                },
            )
            .await?;

        Ok(spawn_chat_completions_stream(
            stream_response,
            self.session.provider().stream_idle_timeout,
            self.sse_telemetry.clone(),
            turn_state,
        ))
    }

    fn path() -> &'static str {
        "responses"
    }

    #[instrument(
        name = "responses.stream",
        level = "info",
        skip_all,
        fields(
            transport = "responses_http",
            http.method = "POST",
            api.path = "responses",
            turn.has_state = turn_state.is_some()
        )
    )]
    pub async fn stream(
        &self,
        body: Value,
        extra_headers: HeaderMap,
        compression: Compression,
        turn_state: Option<Arc<OnceLock<String>>>,
    ) -> Result<ResponseStream, ApiError> {
        let request_compression = match compression {
            Compression::None => RequestCompression::None,
            Compression::Zstd => RequestCompression::Zstd,
        };

        let stream_response = self
            .session
            .stream_with(
                Method::POST,
                Self::path(),
                extra_headers,
                Some(body),
                |req| {
                    req.headers.insert(
                        http::header::ACCEPT,
                        HeaderValue::from_static("text/event-stream"),
                    );
                    req.compression = request_compression;
                },
            )
            .await?;

        Ok(spawn_response_stream(
            stream_response,
            self.session.provider().stream_idle_timeout,
            self.sse_telemetry.clone(),
            turn_state,
        ))
    }
}

fn build_chat_completions_body(request: ResponsesApiRequest) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), Value::String(request.model));
    body.insert("stream".to_string(), Value::Bool(true));
    body.insert(
        "stream_options".to_string(),
        serde_json::json!({ "include_usage": true }),
    );

    let mut messages = Vec::new();
    if !request.instructions.trim().is_empty() {
        messages.push(serde_json::json!({
            "role": "system",
            "content": request.instructions,
        }));
    }
    messages.extend(
        request
            .input
            .iter()
            .flat_map(chat_messages_from_response_item),
    );
    body.insert("messages".to_string(), Value::Array(messages));

    let tools = chat_tools_from_responses_tools(&request.tools);
    if !tools.is_empty() {
        body.insert("tools".to_string(), Value::Array(tools));
        if request.tool_choice == "none" || request.tool_choice == "auto" {
            body.insert(
                "tool_choice".to_string(),
                Value::String(request.tool_choice),
            );
        }
    }

    Value::Object(body)
}

fn chat_messages_from_response_item(item: &ResponseItem) -> Vec<Value> {
    match item {
        ResponseItem::Message { role, content, .. } => {
            let role = if role == "developer" { "system" } else { role };
            let text = content_items_to_text(content);
            if text.trim().is_empty() {
                Vec::new()
            } else {
                vec![serde_json::json!({ "role": role, "content": text })]
            }
        }
        ResponseItem::FunctionCall {
            name,
            arguments,
            call_id,
            ..
        }
        | ResponseItem::CustomToolCall {
            name,
            input: arguments,
            call_id,
            ..
        } => vec![serde_json::json!({
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": [{
                "id": call_id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": arguments,
                }
            }]
        })],
        ResponseItem::FunctionCallOutput { call_id, output }
        | ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => vec![serde_json::json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": function_output_to_text(&output.body),
        })],
        ResponseItem::ToolSearchOutput {
            call_id: Some(call_id),
            tools,
            ..
        } => vec![serde_json::json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": serde_json::to_string(tools).unwrap_or_default(),
        })],
        _ => Vec::new(),
    }
}

fn content_items_to_text(content: &[ContentItem]) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            ContentItem::InputImage { .. } => Some("[Image omitted: DeepSeek is text-only]"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn function_output_to_text(body: &FunctionCallOutputBody) -> String {
    body.to_text().unwrap_or_default()
}

fn chat_tools_from_responses_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|tool| {
            let object = tool.as_object()?;
            if object.get("type").and_then(Value::as_str) != Some("function") {
                return None;
            }
            let name = object.get("name")?.as_str()?;
            let mut function = serde_json::Map::new();
            function.insert("name".to_string(), Value::String(name.to_string()));
            if let Some(description) = object.get("description").and_then(Value::as_str) {
                function.insert(
                    "description".to_string(),
                    Value::String(description.to_string()),
                );
            }
            if let Some(parameters) = object.get("parameters") {
                function.insert("parameters".to_string(), parameters.clone());
            }
            Some(serde_json::json!({
                "type": "function",
                "function": Value::Object(function),
            }))
        })
        .collect()
}
