use codex_api::ResponsesApiRequest;
use codex_api::WireApi;
use codex_api::build_chat_completions_body;
use codex_protocol::protocol::TokenUsage;
use serde::Serialize;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;
use tracing::warn;

const TRACE_PATH_ENV: &str = "WHALE_PROVIDER_WIRE_TRACE_PATH";

#[derive(Debug)]
pub(crate) struct ProviderWireTrace {
    path: Option<PathBuf>,
    state: Mutex<ProviderWireTraceState>,
}

#[derive(Debug, Default)]
struct ProviderWireTraceState {
    epoch_id: String,
    next_request_index: usize,
    previous: Option<WireRequestShape>,
    active_request_id: Option<String>,
}

#[derive(Debug, Clone)]
struct WireRequestShape {
    request_id: String,
    tools_hash: String,
    tool_choice_kind: String,
    tool_choice_name: Option<String>,
    messages: Vec<WireMessageShape>,
}

#[derive(Debug, Clone, Serialize)]
struct WireMessageShape {
    index: usize,
    role: String,
    bytes: usize,
    message_sha256: String,
    content_sha256: String,
}

#[derive(Debug, Serialize)]
struct WireShapeEvent<'a> {
    schema_version: &'static str,
    event_name: &'static str,
    request_id: &'a str,
    epoch_id: &'a str,
    request_index: usize,
    provider_wire_api: String,
    pre_wire_payload_sha256: String,
    provider_payload_sha256: String,
    provider_payload_bytes: usize,
    messages_hash: String,
    tools_hash: String,
    cache_shape_hash: String,
    tools_count: usize,
    tool_choice_kind: &'a str,
    tool_choice_name: Option<&'a str>,
    message_count: usize,
    message_shapes: &'a [WireMessageShape],
    previous_request_id: Option<&'a str>,
    lcp_message_count: usize,
    lcp_message_bytes: usize,
    message_prefix_preserved: Option<bool>,
    tool_choice_preserved: Option<bool>,
    tool_choice_changed: Option<bool>,
    prefix_preserved: Option<bool>,
    first_diff_index: Option<usize>,
    first_diff_path: Option<String>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct WireTerminalEvent<'a> {
    schema_version: &'static str,
    event_name: &'static str,
    request_id: &'a str,
    status: &'a str,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

impl ProviderWireTrace {
    pub(crate) fn from_env() -> Self {
        let path = std::env::var_os(TRACE_PATH_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self {
            path,
            state: Mutex::new(ProviderWireTraceState::default()),
        }
    }

    pub(crate) fn record_request(
        &self,
        epoch_id: &str,
        provider_wire_api: WireApi,
        request: &ResponsesApiRequest,
    ) -> Value {
        let pre_wire = serde_json::to_value(request).unwrap_or(Value::Null);
        let wire = match provider_wire_api {
            WireApi::ChatCompletions => build_chat_completions_body(request),
            WireApi::Responses => pre_wire.clone(),
        };
        let Some(path) = self.path.as_ref() else {
            return wire;
        };

        let messages_field = match provider_wire_api {
            WireApi::ChatCompletions => "messages",
            WireApi::Responses => "input",
        };
        let messages = wire
            .get(messages_field)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let message_shapes = message_shapes(&messages);
        let tools = wire.get("tools").unwrap_or(&Value::Null);
        let tools_hash = json_hash(tools);
        let tool_choice_kind = request.tool_choice.kind();
        let tool_choice_name = request.tool_choice.function_name();
        let cache_shape_hash = cache_shape_hash(&tools_hash, tool_choice_kind, tool_choice_name);
        let tools_count = tools.as_array().map(Vec::len).unwrap_or(0);
        let messages_hash = json_hash(wire.get(messages_field).unwrap_or(&Value::Null));
        let provider_payload_bytes = json_bytes(&wire).len();
        let provider_payload_sha256 = json_hash(&wire);
        let pre_wire_payload_sha256 = json_hash(&pre_wire);

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.epoch_id != epoch_id {
            state.epoch_id = epoch_id.to_string();
            state.next_request_index = 0;
            state.previous = None;
            state.active_request_id = None;
        }
        state.next_request_index += 1;
        let request_id = format!("provider-wire:{epoch_id}:{}", state.next_request_index);
        let comparison = state.previous.as_ref().map(|previous| {
            compare_shapes(
                previous,
                &tools_hash,
                tool_choice_kind,
                tool_choice_name,
                &message_shapes,
            )
        });
        let previous_request_id = state
            .previous
            .as_ref()
            .map(|previous| previous.request_id.as_str());
        let event_name = match comparison.as_ref().map(|value| value.prefix_preserved) {
            Some(true) => "provider.chat_wire_prefix_preserved",
            Some(false) => "provider.chat_wire_prefix_broken",
            None => "provider.chat_wire_shape_recorded",
        };
        let event = WireShapeEvent {
            schema_version: "provider-chat-wire-trace-v2",
            event_name,
            request_id: &request_id,
            epoch_id,
            request_index: state.next_request_index,
            provider_wire_api: format!("{provider_wire_api:?}"),
            pre_wire_payload_sha256,
            provider_payload_sha256,
            provider_payload_bytes,
            messages_hash,
            tools_hash: tools_hash.clone(),
            cache_shape_hash: cache_shape_hash.clone(),
            tools_count,
            tool_choice_kind,
            tool_choice_name,
            message_count: message_shapes.len(),
            message_shapes: &message_shapes,
            previous_request_id,
            lcp_message_count: comparison
                .as_ref()
                .map(|value| value.lcp_message_count)
                .unwrap_or(0),
            lcp_message_bytes: comparison
                .as_ref()
                .map(|value| value.lcp_message_bytes)
                .unwrap_or(0),
            message_prefix_preserved: comparison
                .as_ref()
                .map(|value| value.message_prefix_preserved),
            tool_choice_preserved: comparison.as_ref().map(|value| value.tool_choice_preserved),
            tool_choice_changed: comparison
                .as_ref()
                .map(|value| !value.tool_choice_preserved),
            prefix_preserved: comparison.as_ref().map(|value| value.prefix_preserved),
            first_diff_index: comparison.as_ref().and_then(|value| value.first_diff_index),
            first_diff_path: comparison.and_then(|value| value.first_diff_path),
            status: "payload_captured",
        };
        append_json(path, &event);
        info!(
            event.name = event_name,
            request_id = %request_id,
            epoch_id,
            message_count = message_shapes.len(),
            "provider Chat wire shape recorded"
        );
        state.previous = Some(WireRequestShape {
            request_id: request_id.clone(),
            tools_hash,
            tool_choice_kind: tool_choice_kind.to_string(),
            tool_choice_name: tool_choice_name.map(str::to_string),
            messages: message_shapes,
        });
        state.active_request_id = Some(request_id);
        wire
    }

    pub(crate) fn record_terminal(&self, status: &str, usage: Option<&TokenUsage>) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(request_id) = state.active_request_id.take() else {
            warn!(event.name = "provider.chat_wire_shape_missing", status);
            return;
        };
        let event = WireTerminalEvent {
            schema_version: "provider-chat-wire-trace-v2",
            event_name: "provider.chat_wire_request_terminal",
            request_id: &request_id,
            status,
            input_tokens: usage.map(|value| value.input_tokens),
            cached_input_tokens: usage.map(|value| value.cached_input_tokens),
            output_tokens: usage.map(|value| value.output_tokens),
            reasoning_output_tokens: usage.map(|value| value.reasoning_output_tokens),
            total_tokens: usage.map(|value| value.total_tokens),
        };
        append_json(path, &event);
    }
}

#[derive(Debug)]
struct ShapeComparison {
    lcp_message_count: usize,
    lcp_message_bytes: usize,
    message_prefix_preserved: bool,
    tool_choice_preserved: bool,
    prefix_preserved: bool,
    first_diff_index: Option<usize>,
    first_diff_path: Option<String>,
}

fn compare_shapes(
    previous: &WireRequestShape,
    tools_hash: &str,
    tool_choice_kind: &str,
    tool_choice_name: Option<&str>,
    current: &[WireMessageShape],
) -> ShapeComparison {
    let lcp_message_count = previous
        .messages
        .iter()
        .zip(current.iter())
        .take_while(|(left, right)| left.message_sha256 == right.message_sha256)
        .count();
    let lcp_message_bytes = previous
        .messages
        .iter()
        .take(lcp_message_count)
        .map(|message| message.bytes)
        .sum();
    let message_prefix_preserved =
        lcp_message_count == previous.messages.len() && current.len() >= previous.messages.len();
    let tool_choice_preserved = previous.tool_choice_kind == tool_choice_kind
        && previous.tool_choice_name.as_deref() == tool_choice_name;
    let prefix_preserved =
        previous.tools_hash == tools_hash && tool_choice_preserved && message_prefix_preserved;
    let first_diff_path = if previous.tools_hash != tools_hash {
        Some("tools".to_string())
    } else if !tool_choice_preserved {
        Some("tool_choice".to_string())
    } else if message_prefix_preserved {
        None
    } else {
        Some(format!("messages[{lcp_message_count}].message"))
    };
    let first_diff_index = first_diff_path
        .as_deref()
        .is_some_and(|path| path.starts_with("messages["))
        .then_some(lcp_message_count);
    ShapeComparison {
        lcp_message_count,
        lcp_message_bytes,
        message_prefix_preserved,
        tool_choice_preserved,
        prefix_preserved,
        first_diff_index,
        first_diff_path,
    }
}

fn cache_shape_hash(
    tools_hash: &str,
    tool_choice_kind: &str,
    tool_choice_name: Option<&str>,
) -> String {
    json_hash(&serde_json::json!({
        "tools_hash": tools_hash,
        "tool_choice_kind": tool_choice_kind,
        "tool_choice_name": tool_choice_name,
    }))
}

fn message_shapes(messages: &[Value]) -> Vec<WireMessageShape> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| WireMessageShape {
            index,
            role: message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            bytes: json_bytes(message).len(),
            message_sha256: json_hash(message),
            content_sha256: json_hash(message.get("content").unwrap_or(&Value::Null)),
        })
        .collect()
}

fn json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

fn json_hash(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(json_bytes(value));
    format!("{:x}", hasher.finalize())
}

fn append_json(path: &PathBuf, value: &impl Serialize) {
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, value)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        file.write_all(b"\n")
    })();
    if let Err(error) = result {
        warn!(
            event.name = "provider.chat_wire_trace_write_failed",
            path = %path.display(),
            %error
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, content: &str) -> Value {
        serde_json::json!({"role": role, "content": content})
    }

    #[test]
    fn comparison_detects_append_only_history() {
        let previous_values = vec![message("system", "stable"), message("user", "task")];
        let previous = WireRequestShape {
            request_id: "request-1".to_string(),
            tools_hash: json_hash(&serde_json::json!([])),
            tool_choice_kind: "auto".to_string(),
            tool_choice_name: None,
            messages: message_shapes(&previous_values),
        };
        let current_values = vec![
            message("system", "stable"),
            message("user", "task"),
            message("assistant", "next"),
        ];
        let comparison = compare_shapes(
            &previous,
            &previous.tools_hash,
            "auto",
            None,
            &message_shapes(&current_values),
        );
        assert!(comparison.prefix_preserved);
        assert!(comparison.message_prefix_preserved);
        assert!(comparison.tool_choice_preserved);
        assert_eq!(comparison.lcp_message_count, 2);
        assert_eq!(comparison.first_diff_path, None);
    }

    #[test]
    fn comparison_locates_replaced_history_message() {
        let previous_values = vec![message("system", "stable"), message("developer", "P1")];
        let previous = WireRequestShape {
            request_id: "request-1".to_string(),
            tools_hash: json_hash(&serde_json::json!([])),
            tool_choice_kind: "auto".to_string(),
            tool_choice_name: None,
            messages: message_shapes(&previous_values),
        };
        let current_values = vec![
            message("system", "stable"),
            message("developer", "P2"),
            message("assistant", "next"),
        ];
        let comparison = compare_shapes(
            &previous,
            &previous.tools_hash,
            "auto",
            None,
            &message_shapes(&current_values),
        );
        assert!(!comparison.prefix_preserved);
        assert_eq!(comparison.first_diff_index, Some(1));
        assert_eq!(
            comparison.first_diff_path.as_deref(),
            Some("messages[1].message")
        );
    }

    #[test]
    fn comparison_marks_named_to_auto_as_cache_shape_change() {
        let values = vec![message("system", "stable"), message("user", "task")];
        let previous = WireRequestShape {
            request_id: "request-1".to_string(),
            tools_hash: json_hash(&serde_json::json!([])),
            tool_choice_kind: "named_function".to_string(),
            tool_choice_name: Some("taskspace_control".to_string()),
            messages: message_shapes(&values),
        };
        let mut current = values;
        current.push(message("assistant", "next"));

        let comparison = compare_shapes(
            &previous,
            &previous.tools_hash,
            "auto",
            None,
            &message_shapes(&current),
        );

        assert!(comparison.message_prefix_preserved);
        assert!(!comparison.tool_choice_preserved);
        assert!(!comparison.prefix_preserved);
        assert_eq!(comparison.first_diff_index, None);
        assert_eq!(comparison.first_diff_path.as_deref(), Some("tool_choice"));
    }
}
