use crate::common::ResponseEvent;
use crate::common::ResponseStream;
use crate::error::ApiError;
use crate::telemetry::SseTelemetry;
use codex_client::ByteStream;
use codex_client::StreamResponse;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TokenUsage;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::timeout;
use tracing::debug;

pub fn spawn_chat_completions_stream(
    stream_response: StreamResponse,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
    turn_state: Option<Arc<OnceLock<String>>>,
) -> ResponseStream {
    if let Some(turn_state) = turn_state.as_ref()
        && let Some(header_value) = stream_response
            .headers
            .get("x-whale-turn-state")
            .or_else(|| stream_response.headers.get("x-codex-turn-state"))
            .and_then(|v| v.to_str().ok())
    {
        let _ = turn_state.set(header_value.to_string());
    }

    let (tx_event, rx_event) = mpsc::channel::<Result<ResponseEvent, ApiError>>(1600);
    tokio::spawn(process_sse(
        stream_response.bytes,
        tx_event,
        idle_timeout,
        telemetry,
    ));
    ResponseStream { rx_event }
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    id: Option<String>,
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    delta: ChatDelta,
}

#[derive(Debug, Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<ChatToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct ChatToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<ChatFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct ChatFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
    prompt_tokens_details: Option<ChatPromptTokenDetails>,
    completion_tokens_details: Option<ChatCompletionTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct ChatPromptTokenDetails {
    cached_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionTokenDetails {
    reasoning_tokens: Option<i64>,
}

#[derive(Default)]
struct ToolCallState {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

#[derive(Default)]
struct ChatStreamState {
    response_id: Option<String>,
    assistant_started: bool,
    assistant_text: String,
    reasoning_started: bool,
    reasoning_text: String,
    tool_calls: BTreeMap<usize, ToolCallState>,
    usage: Option<TokenUsage>,
}

impl ChatStreamState {
    async fn apply_chunk(
        &mut self,
        chunk: ChatChunk,
        tx_event: &mpsc::Sender<Result<ResponseEvent, ApiError>>,
    ) -> bool {
        if self.response_id.is_none() {
            self.response_id = chunk.id;
        }
        if let Some(usage) = chunk.usage {
            self.usage = Some(usage.into());
        }

        for choice in chunk.choices {
            let delta = choice.delta;
            if let Some(reasoning) = delta.reasoning_content
                && !reasoning.is_empty()
            {
                if !self.reasoning_started {
                    self.reasoning_started = true;
                    if tx_event
                        .send(Ok(ResponseEvent::OutputItemAdded(reasoning_item(
                            String::new(),
                        ))))
                        .await
                        .is_err()
                    {
                        return false;
                    }
                }
                self.reasoning_text.push_str(&reasoning);
                if tx_event
                    .send(Ok(ResponseEvent::ReasoningContentDelta {
                        delta: reasoning,
                        content_index: 0,
                    }))
                    .await
                    .is_err()
                {
                    return false;
                }
            }

            if let Some(content) = delta.content
                && !content.is_empty()
            {
                if !self.assistant_started {
                    self.assistant_started = true;
                    if tx_event
                        .send(Ok(ResponseEvent::OutputItemAdded(assistant_message(
                            String::new(),
                        ))))
                        .await
                        .is_err()
                    {
                        return false;
                    }
                }
                self.assistant_text.push_str(&content);
                if tx_event
                    .send(Ok(ResponseEvent::OutputTextDelta(content)))
                    .await
                    .is_err()
                {
                    return false;
                }
            }

            if let Some(tool_calls) = delta.tool_calls {
                for tool_delta in tool_calls {
                    let state = self.tool_calls.entry(tool_delta.index).or_default();
                    if let Some(id) = tool_delta.id {
                        state.id = Some(id);
                    }
                    if let Some(function) = tool_delta.function {
                        if let Some(name) = function.name {
                            state.name = Some(name);
                        }
                        if let Some(arguments) = function.arguments {
                            state.arguments.push_str(&arguments);
                        }
                    }
                }
            }
        }

        true
    }

    async fn finish(self, tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>) {
        if self.reasoning_started {
            let _ = tx_event
                .send(Ok(ResponseEvent::OutputItemDone(reasoning_item(
                    self.reasoning_text,
                ))))
                .await;
        }

        if self.assistant_started {
            let _ = tx_event
                .send(Ok(ResponseEvent::OutputItemDone(assistant_message(
                    self.assistant_text,
                ))))
                .await;
        }

        for (idx, call) in self.tool_calls {
            let call_id = call
                .id
                .filter(|id| !id.trim().is_empty())
                .unwrap_or_else(|| format!("chatcmpl-tool-call-{idx}"));
            let name = call
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "unknown_tool".to_string());
            let _ = tx_event
                .send(Ok(ResponseEvent::OutputItemDone(
                    ResponseItem::FunctionCall {
                        id: None,
                        name,
                        namespace: None,
                        arguments: call.arguments,
                        call_id,
                    },
                )))
                .await;
        }

        let _ = tx_event
            .send(Ok(ResponseEvent::Completed {
                response_id: self
                    .response_id
                    .unwrap_or_else(|| "chat-completions-response".to_string()),
                token_usage: self.usage,
                end_turn: None,
            }))
            .await;
    }
}

impl From<ChatUsage> for TokenUsage {
    fn from(usage: ChatUsage) -> Self {
        let input_tokens = usage.prompt_tokens.unwrap_or(0);
        let output_tokens = usage.completion_tokens.unwrap_or(0);
        TokenUsage {
            input_tokens,
            cached_input_tokens: usage
                .prompt_tokens_details
                .and_then(|details| details.cached_tokens)
                .unwrap_or(0),
            output_tokens,
            reasoning_output_tokens: usage
                .completion_tokens_details
                .and_then(|details| details.reasoning_tokens)
                .unwrap_or(0),
            total_tokens: usage
                .total_tokens
                .unwrap_or(input_tokens.saturating_add(output_tokens)),
        }
    }
}

fn assistant_message(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText { text }],
        end_turn: None,
        phase: None,
    }
}

fn reasoning_item(text: String) -> ResponseItem {
    ResponseItem::Reasoning {
        id: "chat-completions-reasoning".to_string(),
        summary: Vec::new(),
        content: Some(vec![ReasoningItemContent::ReasoningText { text }]),
        encrypted_content: None,
    }
}

async fn process_sse(
    stream: ByteStream,
    tx_event: mpsc::Sender<Result<ResponseEvent, ApiError>>,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) {
    let mut stream = stream.eventsource();
    let mut state = ChatStreamState::default();

    loop {
        let start = Instant::now();
        let response = timeout(idle_timeout, stream.next()).await;
        if let Some(t) = telemetry.as_ref() {
            t.on_sse_poll(&response, start.elapsed());
        }

        let sse = match response {
            Ok(Some(Ok(sse))) => sse,
            Ok(Some(Err(e))) => {
                let _ = tx_event.send(Err(ApiError::Stream(e.to_string()))).await;
                return;
            }
            Ok(None) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream(
                        "chat completions stream closed before [DONE]".into(),
                    )))
                    .await;
                return;
            }
            Err(_) => {
                let _ = tx_event
                    .send(Err(ApiError::Stream("idle timeout waiting for SSE".into())))
                    .await;
                return;
            }
        };

        let data = sse.data.trim();
        if data == "[DONE]" {
            state.finish(tx_event).await;
            return;
        }

        let chunk: ChatChunk = match serde_json::from_str(data) {
            Ok(chunk) => chunk,
            Err(err) => {
                debug!("failed to parse chat completions SSE chunk: {err}; data={data}");
                continue;
            }
        };

        if !state.apply_chunk(chunk, &tx_event).await {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn chat_usage_maps_cached_and_reasoning_tokens() {
        let usage = TokenUsage::from(ChatUsage {
            prompt_tokens: Some(10),
            completion_tokens: Some(6),
            total_tokens: Some(16),
            prompt_tokens_details: Some(ChatPromptTokenDetails {
                cached_tokens: Some(4),
            }),
            completion_tokens_details: Some(ChatCompletionTokenDetails {
                reasoning_tokens: Some(3),
            }),
        });

        assert_eq!(
            usage,
            TokenUsage {
                input_tokens: 10,
                cached_input_tokens: 4,
                output_tokens: 6,
                reasoning_output_tokens: 3,
                total_tokens: 16,
            }
        );
    }

    #[tokio::test]
    async fn chat_stream_state_emits_text_reasoning_tool_and_completion() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut state = ChatStreamState::default();

        assert!(
            state
                .apply_chunk(
                    ChatChunk {
                        id: Some("chatcmpl-test".to_string()),
                        choices: vec![ChatChoice {
                            delta: ChatDelta {
                                content: Some("hello".to_string()),
                                reasoning_content: Some("think".to_string()),
                                tool_calls: Some(vec![ChatToolCallDelta {
                                    index: 0,
                                    id: Some("call-1".to_string()),
                                    function: Some(ChatFunctionDelta {
                                        name: Some("shell".to_string()),
                                        arguments: Some("{\"cmd\":\"".to_string()),
                                    }),
                                }]),
                            },
                        }],
                        usage: None,
                    },
                    &tx,
                )
                .await
        );
        assert!(
            state
                .apply_chunk(
                    ChatChunk {
                        id: None,
                        choices: vec![ChatChoice {
                            delta: ChatDelta {
                                content: Some(" world".to_string()),
                                reasoning_content: None,
                                tool_calls: Some(vec![ChatToolCallDelta {
                                    index: 0,
                                    id: None,
                                    function: Some(ChatFunctionDelta {
                                        name: None,
                                        arguments: Some("ls\"}".to_string()),
                                    }),
                                }]),
                            },
                        }],
                        usage: Some(ChatUsage {
                            prompt_tokens: Some(1),
                            completion_tokens: Some(2),
                            total_tokens: Some(3),
                            prompt_tokens_details: None,
                            completion_tokens_details: None,
                        }),
                    },
                    &tx,
                )
                .await
        );
        state.finish(tx).await;

        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event.expect("event should be ok"));
        }

        assert!(matches!(events[0], ResponseEvent::OutputItemAdded(_)));
        assert!(matches!(
            &events[1],
            ResponseEvent::ReasoningContentDelta { delta, .. } if delta == "think"
        ));
        assert!(matches!(events[2], ResponseEvent::OutputItemAdded(_)));
        assert!(matches!(&events[3], ResponseEvent::OutputTextDelta(delta) if delta == "hello"));
        assert!(matches!(&events[4], ResponseEvent::OutputTextDelta(delta) if delta == " world"));
        assert!(matches!(events[5], ResponseEvent::OutputItemDone(_)));
        assert!(matches!(events[6], ResponseEvent::OutputItemDone(_)));
        assert!(matches!(
            &events[7],
            ResponseEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            }) if name == "shell" && arguments == "{\"cmd\":\"ls\"}" && call_id == "call-1"
        ));
        assert!(matches!(
            &events[8],
            ResponseEvent::Completed {
                response_id,
                token_usage: Some(TokenUsage { total_tokens: 3, .. }),
                ..
            } if response_id == "chatcmpl-test"
        ));
        assert_eq!(events.len(), 9);
    }
}
