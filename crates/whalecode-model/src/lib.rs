use std::env;

use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

mod bootstrap;
mod secrets;
mod tool_calls;

pub use bootstrap::*;
pub use secrets::*;
pub use tool_calls::*;

pub const DEEPSEEK_DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-v4-flash";
pub const DEEPSEEK_PRO_MODEL: &str = "deepseek-v4-pro";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub model: String,
    pub context_window_tokens: u64,
    pub max_output_tokens: u64,
    pub supports_thinking: bool,
    pub supports_tool_calls: bool,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelStreamEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: String,
        arguments_delta: String,
    },
    Usage(ModelTokenUsage),
    Finished,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingMode {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub thinking: ThinkingMode,
    pub reasoning_effort: ReasoningEffort,
}

impl DeepSeekConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| DEEPSEEK_DEFAULT_BASE_URL.to_owned()),
            api_key: resolve_deepseek_api_key(),
            model: env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEEPSEEK_DEFAULT_MODEL.to_owned()),
            thinking: ThinkingMode::Enabled,
            reasoning_effort: ReasoningEffort::Medium,
        }
    }

    pub fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatMessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<DeepSeekToolCall>>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatMessageRole::User,
            content: content.into(),
            reasoning_content: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: DeepSeekFunctionCall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub thinking: ThinkingControl,
    pub reasoning_effort: String,
    pub stream_options: StreamOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingControl {
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

impl DeepSeekChatRequest {
    pub fn streaming(config: &DeepSeekConfig, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: config.model.clone(),
            messages,
            stream: true,
            thinking: ThinkingControl {
                kind: config.thinking.as_api_str().to_owned(),
            },
            reasoning_effort: config.reasoning_effort.as_api_str().to_owned(),
            stream_options: StreamOptions {
                include_usage: true,
            },
            tools: None,
            tool_choice: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<Value>) -> Self {
        self.tool_choice = Some(json!("auto"));
        self.tools = Some(tools);
        self
    }
}

#[derive(Debug, Clone)]
pub struct DeepSeekClient {
    config: DeepSeekConfig,
    client: reqwest::Client,
}

impl DeepSeekClient {
    pub fn new(config: DeepSeekConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn stream_chat(
        &self,
        request: &DeepSeekChatRequest,
    ) -> Result<Vec<ModelStreamEvent>, ModelError> {
        self.stream_chat_with_observer(request, |_| {}).await
    }

    pub async fn stream_chat_with_observer<F>(
        &self,
        request: &DeepSeekChatRequest,
        mut observer: F,
    ) -> Result<Vec<ModelStreamEvent>, ModelError>
    where
        F: FnMut(&ModelStreamEvent),
    {
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or(ModelError::MissingApiKey)?;
        let response = self
            .client
            .post(self.config.endpoint())
            .bearer_auth(api_key)
            .json(request)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ModelError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut events = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = std::str::from_utf8(&chunk).map_err(|source| ModelError::Utf8 {
                message: source.to_string(),
            })?;
            buffer.push_str(text);
            drain_complete_sse_frames(&mut buffer, &mut events, &mut observer)?;
        }
        if !buffer.trim().is_empty() {
            push_observed_events(&mut events, parse_sse_stream(&buffer)?, &mut observer);
        }
        Ok(events)
    }
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error(
        "DeepSeek API key is required for live DeepSeek calls; set DEEPSEEK_API_KEY or run /apikey inside whale"
    )]
    MissingApiKey,
    #[error("model http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("model returned http {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("malformed SSE JSON payload: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stream chunk is not valid UTF-8: {message}")]
    Utf8 { message: String },
}

pub fn parse_sse_stream(input: &str) -> Result<Vec<ModelStreamEvent>, ModelError> {
    parse_sse_stream_with_observer(input, |_| {})
}

pub fn parse_sse_stream_with_observer<F>(
    input: &str,
    mut observer: F,
) -> Result<Vec<ModelStreamEvent>, ModelError>
where
    F: FnMut(&ModelStreamEvent),
{
    let mut events = Vec::new();
    for frame in sse_data_frames(input) {
        if frame == "[DONE]" {
            push_observed_events(&mut events, vec![ModelStreamEvent::Finished], &mut observer);
            continue;
        }
        push_observed_events(
            &mut events,
            events_from_chunk(&serde_json::from_str(&frame)?),
            &mut observer,
        );
    }
    Ok(events)
}

pub fn response_from_stream_events(events: Vec<ModelStreamEvent>) -> ModelResponse {
    let final_text = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::TextDelta(content) => Some(content.as_str()),
            _ => None,
        })
        .collect::<String>();
    ModelResponse { events, final_text }
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamChunk {
    #[serde(default)]
    choices: Vec<DeepSeekStreamChoice>,
    usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamChoice {
    delta: DeepSeekDelta,
}

#[derive(Debug, Deserialize)]
struct DeepSeekDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<DeepSeekToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    prompt_cache_hit_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    function: Option<DeepSeekFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

fn events_from_chunk(chunk: &DeepSeekStreamChunk) -> Vec<ModelStreamEvent> {
    let mut events = Vec::new();
    if let Some(usage) = &chunk.usage {
        events.push(ModelStreamEvent::Usage(ModelTokenUsage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cached_input_tokens: usage.prompt_cache_hit_tokens.unwrap_or(0),
        }));
    }
    for choice in &chunk.choices {
        if let Some(reasoning) = choice.delta.reasoning_content.as_deref() {
            if !reasoning.is_empty() {
                events.push(ModelStreamEvent::ReasoningDelta(reasoning.to_owned()));
            }
        }
        if let Some(content) = choice.delta.content.as_deref() {
            if !content.is_empty() {
                events.push(ModelStreamEvent::TextDelta(content.to_owned()));
            }
        }
        for tool_call in choice.delta.tool_calls.as_deref().unwrap_or_default() {
            let Some(function) = &tool_call.function else {
                continue;
            };
            let _kind = tool_call.kind.as_deref().unwrap_or("function");
            let name = function.name.clone().unwrap_or_default();
            let arguments_delta = function.arguments.clone().unwrap_or_default();
            if !name.is_empty() || !arguments_delta.is_empty() {
                events.push(ModelStreamEvent::ToolCallDelta {
                    index: tool_call.index.unwrap_or(0),
                    id: tool_call.id.clone(),
                    name,
                    arguments_delta,
                });
            }
        }
    }
    events
}

fn sse_data_frames(input: &str) -> Vec<String> {
    let normalized = input.replace("\r\n", "\n");
    normalized
        .split("\n\n")
        .filter_map(|frame| {
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(|line| line.trim_start())
                .collect::<Vec<_>>()
                .join("\n");
            (!data.is_empty()).then_some(data)
        })
        .collect()
}

fn drain_complete_sse_frames(
    buffer: &mut String,
    events: &mut Vec<ModelStreamEvent>,
    observer: &mut impl FnMut(&ModelStreamEvent),
) -> Result<(), ModelError> {
    while let Some(index) = find_frame_boundary(buffer) {
        let frame = buffer[..index].to_owned();
        let drain_to = if buffer[index..].starts_with("\r\n\r\n") {
            index + 4
        } else {
            index + 2
        };
        buffer.drain(..drain_to);
        push_observed_events(events, parse_sse_stream(&frame)?, observer);
    }
    Ok(())
}

fn push_observed_events(
    events: &mut Vec<ModelStreamEvent>,
    new_events: Vec<ModelStreamEvent>,
    observer: &mut impl FnMut(&ModelStreamEvent),
) {
    for event in new_events {
        observer(&event);
        events.push(event);
    }
}

fn find_frame_boundary(buffer: &str) -> Option<usize> {
    match (buffer.find("\n\n"), buffer.find("\r\n\r\n")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

impl ThinkingMode {
    fn as_api_str(&self) -> &'static str {
        match self {
            ThinkingMode::Enabled => "enabled",
            ThinkingMode::Disabled => "disabled",
        }
    }
}

impl ReasoningEffort {
    fn as_api_str(&self) -> &'static str {
        match self {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}
