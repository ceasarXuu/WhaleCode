use codex_api::ResponsesApiRequest;
use codex_model_provider_info::WireApi;
use codex_protocol::models::BASE_INSTRUCTIONS_DEFAULT;
use codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_TASKSPACE;
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

#[path = "provider_wire_sections.rs"]
mod provider_wire_sections;

use provider_wire_sections::ProviderWireSectionCost;

const TRACE_PATH_ENV: &str = "WHALE_PROVIDER_WIRE_TRACE_PATH";
const WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION: &str = "whalecode-standard-v0.0.6";
const WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256: &str =
    "84affc85717284d5a201ad6123a4c63a5ed68f57558f84f3b8b94ce8b7996cad";
const WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION: &str = "whalecode-taskspace-0.147";
const WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256: &str =
    "f1a963f8476d98dee15cba3118e962981c8f0b7231b28a5884c32fc4be234363";

#[derive(Debug)]
pub(crate) struct ProviderWireTrace {
    path: Option<PathBuf>,
    state: Mutex<ProviderWireTraceState>,
}

#[derive(Debug, Default)]
struct ProviderWireTraceState {
    epoch_id: String,
    next_logical_request_index: usize,
    next_request_index: usize,
    previous: Option<WireRequestShape>,
    active_request_identity: Option<ProviderWireRequestIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderWireRequestIdentity {
    pub(crate) request_id: String,
    pub(crate) logical_request_id: String,
    pub(crate) attempt_seq: usize,
    transport: String,
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
    logical_request_id: &'a str,
    attempt_seq: usize,
    transport: &'a str,
    epoch_id: &'a str,
    request_index: usize,
    provider_wire_api: String,
    pre_wire_payload_sha256: String,
    provider_payload_sha256: String,
    provider_payload_bytes: usize,
    section_cost: &'a ProviderWireSectionCost,
    messages_hash: String,
    tools_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    taskspace_capability_identity: Option<&'a str>,
    cache_shape_hash: String,
    tools_count: usize,
    tool_choice_kind: &'a str,
    tool_choice_name: Option<&'a str>,
    message_count: usize,
    message_shapes: &'a [WireMessageShape],
    base_instructions_identity: BaseInstructionsWireIdentity,
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

#[derive(Debug, Serialize, PartialEq, Eq)]
struct BaseInstructionsWireIdentity {
    count: usize,
    message_index: Option<usize>,
    wire_role: Option<String>,
    message_bytes: Option<usize>,
    estimated_tokens: Option<usize>,
    profile: Option<&'static str>,
    version: Option<&'static str>,
    sha256: Option<&'static str>,
    matches_current_contract: bool,
    unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct WireTerminalEvent<'a> {
    schema_version: &'static str,
    event_name: &'static str,
    request_id: &'a str,
    logical_request_id: &'a str,
    attempt_seq: usize,
    transport: &'a str,
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

    pub(crate) fn begin_logical_request(&self, epoch_id: &str) -> String {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        reset_epoch_if_needed(&mut state, epoch_id);
        state.next_logical_request_index += 1;
        format!(
            "provider-wire:{epoch_id}:logical-{}",
            state.next_logical_request_index
        )
    }

    pub(crate) fn record_request(
        &self,
        epoch_id: &str,
        logical_request_id: &str,
        attempt_seq: usize,
        transport: &str,
        provider_wire_api: WireApi,
        request: &ResponsesApiRequest,
        wire_override: Option<Value>,
        taskspace_capability_identity: Option<&str>,
    ) -> Value {
        let pre_wire = serde_json::to_value(request).unwrap_or(Value::Null);
        let wire = wire_override.unwrap_or_else(|| pre_wire.clone());
        let Some(path) = self.path.as_ref() else {
            return wire;
        };

        let messages_field = "input";
        let messages = wire
            .get(messages_field)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let message_shapes = message_shapes(&messages);
        let tools = wire.get("tools").unwrap_or(&Value::Null);
        let tools_hash = json_hash(tools);
        let tool_choice_kind = request.tool_choice.as_str();
        let tool_choice_name = None;
        let cache_shape_hash = cache_shape_hash(&tools_hash, tool_choice_kind, tool_choice_name);
        let tools_count = tools.as_array().map(Vec::len).unwrap_or(0);
        let messages_hash = json_hash(wire.get(messages_field).unwrap_or(&Value::Null));
        let provider_payload_bytes = json_bytes(&wire).len();
        let provider_payload_sha256 = json_hash(&wire);
        let pre_wire_payload_sha256 = json_hash(&pre_wire);
        let section_cost = ProviderWireSectionCost::measure(&wire, messages_field);
        debug_assert_eq!(
            section_cost.section_bytes_total, provider_payload_bytes,
            "provider wire section bytes must reconcile with payload bytes"
        );

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        reset_epoch_if_needed(&mut state, epoch_id);
        state.next_request_index += 1;
        let request_id = format!("{logical_request_id}:attempt-{attempt_seq}");
        let request_identity = ProviderWireRequestIdentity {
            request_id: request_id.clone(),
            logical_request_id: logical_request_id.to_string(),
            attempt_seq,
            transport: transport.to_string(),
        };
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
        let base_instructions_identity = base_instructions_identity(&wire);
        let event = WireShapeEvent {
            schema_version: "provider-chat-wire-trace-v11",
            event_name,
            request_id: &request_identity.request_id,
            logical_request_id: &request_identity.logical_request_id,
            attempt_seq: request_identity.attempt_seq,
            transport,
            epoch_id,
            request_index: state.next_request_index,
            provider_wire_api: format!("{provider_wire_api:?}"),
            pre_wire_payload_sha256,
            provider_payload_sha256,
            provider_payload_bytes,
            section_cost: &section_cost,
            messages_hash,
            tools_hash: tools_hash.clone(),
            taskspace_capability_identity,
            cache_shape_hash,
            tools_count,
            tool_choice_kind,
            tool_choice_name,
            message_count: message_shapes.len(),
            message_shapes: &message_shapes,
            base_instructions_identity,
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
            request_id,
            tools_hash,
            tool_choice_kind: tool_choice_kind.to_string(),
            tool_choice_name: tool_choice_name.map(str::to_string),
            messages: message_shapes,
        });
        state.active_request_identity = Some(request_identity);
        wire
    }

    pub(crate) fn record_terminal(
        &self,
        status: &str,
        usage: Option<&TokenUsage>,
    ) -> Option<ProviderWireRequestIdentity> {
        let Some(path) = self.path.as_ref() else {
            return None;
        };
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(request_identity) = state.active_request_identity.take() else {
            warn!(event.name = "provider.chat_wire_shape_missing", status);
            return None;
        };
        let event = WireTerminalEvent {
            schema_version: "provider-chat-wire-trace-v11",
            event_name: "provider.chat_wire_request_terminal",
            request_id: &request_identity.request_id,
            logical_request_id: &request_identity.logical_request_id,
            attempt_seq: request_identity.attempt_seq,
            transport: &request_identity.transport,
            status,
            input_tokens: usage.map(|value| value.input_tokens),
            cached_input_tokens: usage.map(|value| value.cached_input_tokens),
            output_tokens: usage.map(|value| value.output_tokens),
            reasoning_output_tokens: usage.map(|value| value.reasoning_output_tokens),
            total_tokens: usage.map(|value| value.total_tokens),
        };
        append_json(path, &event);
        Some(request_identity)
    }
}

fn reset_epoch_if_needed(state: &mut ProviderWireTraceState, epoch_id: &str) {
    if state.epoch_id == epoch_id {
        return;
    }
    state.epoch_id = epoch_id.to_string();
    state.next_logical_request_index = 0;
    state.next_request_index = 0;
    state.previous = None;
    state.active_request_identity = None;
}

fn base_instructions_identity(wire: &Value) -> BaseInstructionsWireIdentity {
    let mut matches = Vec::new();
    if let Some(instructions) = wire.get("instructions").and_then(Value::as_str)
        && let Some((profile, version, sha256)) = known_base_instructions(instructions)
    {
        matches.push((
            None,
            "instructions",
            profile,
            version,
            sha256,
            json_bytes(&Value::String(instructions.to_string())).len(),
        ));
    }
    if matches.is_empty() {
        return unavailable_base_instructions_identity(0, "base_instructions_unrecognized");
    }
    if matches.len() != 1 {
        return unavailable_base_instructions_identity(
            matches.len(),
            "base_instructions_count_invalid",
        );
    }
    let (message_index, wire_role, profile, version, sha256, message_bytes) = matches[0];
    BaseInstructionsWireIdentity {
        count: 1,
        message_index,
        wire_role: Some(wire_role.to_string()),
        message_bytes: Some(message_bytes),
        estimated_tokens: Some(message_bytes.div_ceil(4)),
        profile: Some(profile),
        version: Some(version),
        sha256: Some(sha256),
        matches_current_contract: true,
        unavailable_reason: None,
    }
}

fn known_base_instructions(text: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if text == BASE_INSTRUCTIONS_DEFAULT {
        Some((
            "standard",
            WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION,
            WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256,
        ))
    } else if text == BASE_INSTRUCTIONS_WHALECODE_TASKSPACE {
        Some((
            "taskspace",
            WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION,
            WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256,
        ))
    } else {
        None
    }
}

fn unavailable_base_instructions_identity(
    count: usize,
    reason: &'static str,
) -> BaseInstructionsWireIdentity {
    BaseInstructionsWireIdentity {
        count,
        message_index: None,
        wire_role: None,
        message_bytes: None,
        estimated_tokens: None,
        profile: None,
        version: None,
        sha256: None,
        matches_current_contract: false,
        unavailable_reason: Some(reason),
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
    fn base_instruction_hashes_match_embedded_prompts() {
        let digest = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
        assert_eq!(
            digest(BASE_INSTRUCTIONS_DEFAULT),
            WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256
        );
        assert_eq!(
            digest(BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256
        );
    }

    fn trace_request() -> ResponsesApiRequest {
        ResponsesApiRequest {
            model: "test-model".to_string(),
            instructions: String::new(),
            input: Vec::new(),
            tools: None,
            tool_choice: "auto".into(),
            parallel_tool_calls: true,
            reasoning: None,
            store: false,
            stream: true,
            stream_options: None,
            include: Vec::new(),
            service_tier: None,
            prompt_cache_key: None,
            text: None,
            client_metadata: None,
        }
    }

    #[test]
    fn wire_trace_preserves_logical_attempt_terminal_and_transport_identity() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("provider-wire.jsonl");
        let trace = ProviderWireTrace {
            path: Some(path.clone()),
            state: Mutex::new(ProviderWireTraceState::default()),
        };
        let request = trace_request();
        let logical = trace.begin_logical_request("epoch-1");

        trace.record_request(
            "epoch-1",
            &logical,
            1,
            "responses_http",
            WireApi::Responses,
            &request,
            None,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        let first = trace
            .record_terminal("retry_unauthorized", None)
            .expect("first terminal identity");
        trace.record_request(
            "epoch-1",
            &logical,
            2,
            "responses_http",
            WireApi::Responses,
            &request,
            None,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        let usage = TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 8,
            cache_write_input_tokens: 0,
            output_tokens: 2,
            reasoning_output_tokens: 0,
            total_tokens: 12,
            codex_rollout_budget_units: None,
        };
        let second = trace
            .record_terminal("response_completed", Some(&usage))
            .expect("second terminal identity");

        let websocket_logical = trace.begin_logical_request("epoch-1");
        trace.record_request(
            "epoch-1",
            &websocket_logical,
            1,
            "responses_websocket",
            WireApi::Responses,
            &request,
            Some(serde_json::json!({"input": [], "tools": []})),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        let websocket = trace
            .record_terminal("cancelled", None)
            .expect("websocket terminal identity");

        assert_eq!(first.logical_request_id, logical);
        assert_eq!(first.attempt_seq, 1);
        assert_eq!(second.logical_request_id, logical);
        assert_eq!(second.attempt_seq, 2);
        assert_eq!(websocket.logical_request_id, websocket_logical);
        assert_eq!(websocket.attempt_seq, 1);

        let rows = std::fs::read_to_string(path)
            .expect("wire trace")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("trace row"))
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|row| {
            row.get("schema_version").and_then(Value::as_str)
                == Some("provider-chat-wire-trace-v11")
        }));
        assert_eq!(
            rows[0].get("request_id").and_then(Value::as_str),
            Some(first.request_id.as_str())
        );
        assert_eq!(
            rows[2].get("request_id").and_then(Value::as_str),
            Some(second.request_id.as_str())
        );
        assert_eq!(
            rows[4].get("transport").and_then(Value::as_str),
            Some("responses_websocket")
        );
        for index in [0, 2, 4] {
            assert_eq!(
                rows[index]
                    .get("taskspace_capability_identity")
                    .and_then(Value::as_str),
                Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            );
        }
        assert_eq!(
            rows[5].get("status").and_then(Value::as_str),
            Some("cancelled")
        );
    }

    #[test]
    fn standard_base_identity_tracks_version_hash_and_position() {
        let identity = base_instructions_identity(&serde_json::json!({
            "instructions": BASE_INSTRUCTIONS_DEFAULT,
            "input": [message("user", "task")],
        }));
        assert_eq!(identity.count, 1);
        assert_eq!(identity.message_index, None);
        assert_eq!(identity.wire_role.as_deref(), Some("instructions"));
        assert_eq!(identity.profile, Some("standard"));
        assert_eq!(
            identity.version,
            Some(WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION)
        );
        assert_eq!(
            identity.sha256,
            Some(WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256)
        );
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn taskspace_base_identity_tracks_version_hash_and_position() {
        let identity = base_instructions_identity(&serde_json::json!({
            "instructions": BASE_INSTRUCTIONS_WHALECODE_TASKSPACE,
            "input": [message("user", "task")],
        }));
        assert_eq!(identity.count, 1);
        assert_eq!(identity.message_index, None);
        assert_eq!(identity.wire_role.as_deref(), Some("instructions"));
        assert_eq!(identity.profile, Some("taskspace"));
        assert_eq!(
            identity.version,
            Some(WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION)
        );
        assert_eq!(
            identity.sha256,
            Some(WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256)
        );
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn unknown_or_user_quoted_base_is_not_counted() {
        let identity = base_instructions_identity(&serde_json::json!({
            "instructions": "unknown",
            "input": [message("user", BASE_INSTRUCTIONS_DEFAULT)],
        }));
        assert_eq!(identity.count, 0);
        assert_eq!(
            identity.unavailable_reason,
            Some("base_instructions_unrecognized")
        );
    }

    #[test]
    fn responses_base_identity_uses_top_level_instructions() {
        let messages = vec![message("user", "task")];
        let wire = serde_json::json!({
            "instructions": BASE_INSTRUCTIONS_WHALECODE_TASKSPACE,
            "input": messages,
        });

        let identity = base_instructions_identity(&wire);

        assert_eq!(identity.count, 1);
        assert_eq!(identity.message_index, None);
        assert_eq!(identity.wire_role.as_deref(), Some("instructions"));
        assert_eq!(identity.profile, Some("taskspace"));
        assert_eq!(
            identity.version,
            Some(WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION)
        );
        assert!(identity.matches_current_contract);
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
            tool_choice_name: Some("exec_command".to_string()),
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
