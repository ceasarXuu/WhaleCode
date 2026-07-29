use codex_api::ResponsesApiRequest;
use codex_api::WireApi;
use codex_api::build_chat_completions_body;
use codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_STANDARD;
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

use crate::context::TASKSPACE_CONTRACT_MANIFEST_ID;
use crate::context::TASKSPACE_CONTRACT_MANIFEST_SHA256;
use crate::context::TASKSPACE_CONTRACT_MANIFEST_VERSION;
use crate::context::TASKSPACE_CORE_PROTOCOL;
use crate::context::TASKSPACE_CORE_PROTOCOL_SHA256;
use crate::context::TASKSPACE_CORE_PROTOCOL_VERSION;
use crate::context::WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256;
use crate::context::WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION;
use crate::context::WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256;
use crate::context::WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION;
use crate::context::taskspace_contract_manifest_matches;
use crate::context::taskspace_core_protocol_matches;

#[path = "provider_wire_sections.rs"]
mod provider_wire_sections;

use provider_wire_sections::ProviderWireSectionCost;

const TRACE_PATH_ENV: &str = "WHALE_PROVIDER_WIRE_TRACE_PATH";
const TASKSPACE_FINAL_RECEIPT_SCHEMA: &str = "TaskSpaceResponseFinalReceiptV1";

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
    active_request_identity: Option<ProviderWireRequestIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderWireRequestIdentity {
    pub(crate) request_id: String,
    pub(crate) logical_request_id: String,
    pub(crate) attempt_seq: usize,
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
    epoch_id: &'a str,
    request_index: usize,
    provider_wire_api: String,
    pre_wire_payload_sha256: String,
    provider_payload_sha256: String,
    provider_payload_bytes: usize,
    section_cost: &'a ProviderWireSectionCost,
    messages_hash: String,
    tools_hash: String,
    cache_shape_hash: String,
    tools_count: usize,
    tool_choice_kind: &'a str,
    tool_choice_name: Option<&'a str>,
    message_count: usize,
    message_shapes: &'a [WireMessageShape],
    base_instructions_identity: BaseInstructionsWireIdentity,
    taskspace_wire_contract_identity: TaskspaceWireContractIdentity,
    taskspace_final_receipt_identity: TaskspaceFinalReceiptIdentity,
    taskspace_contract_manifest_identity: TaskspaceContractManifestWireIdentity,
    taskspace_core_protocol_identity: TaskspaceCoreProtocolWireIdentity,
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

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TaskspaceContractManifestWireIdentity {
    count: usize,
    contract_id: Option<&'static str>,
    version: Option<&'static str>,
    sha256: Option<&'static str>,
    matches_current_contract: bool,
    unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TaskspaceCoreProtocolWireIdentity {
    count: usize,
    message_index: Option<usize>,
    wire_role: Option<String>,
    section_order: Option<usize>,
    bytes: Option<usize>,
    estimated_tokens: Option<usize>,
    version: Option<&'static str>,
    sha256: Option<&'static str>,
    matches_current_contract: bool,
    unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TaskspaceWireContractIdentity {
    system_message_count: usize,
    expected_system_message_count: Option<usize>,
    map_handle_count: usize,
    map_handle_message_index: Option<usize>,
    map_handle_wire_role: Option<String>,
    map_handle_is_request_tail: bool,
    matches_current_contract: bool,
    unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TaskspaceFinalReceiptIdentity {
    count: usize,
    receipts: Vec<TaskspaceFinalReceiptMessageIdentity>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TaskspaceFinalReceiptMessageIdentity {
    message_index: usize,
    wire_role: String,
    control_call_id_sha256: Option<String>,
    reservation_revision_after: Option<u64>,
    canonical_revision: Option<u64>,
    revision_delta: Option<i64>,
    complete: bool,
}

#[derive(Debug, Serialize)]
struct WireTerminalEvent<'a> {
    schema_version: &'static str,
    event_name: &'static str,
    request_id: &'a str,
    logical_request_id: &'a str,
    attempt_seq: usize,
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
        let section_cost = ProviderWireSectionCost::measure(&wire, messages_field);
        debug_assert_eq!(
            section_cost.section_bytes_total, provider_payload_bytes,
            "provider wire section bytes must reconcile with payload bytes"
        );

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.epoch_id != epoch_id {
            state.epoch_id = epoch_id.to_string();
            state.next_request_index = 0;
            state.previous = None;
            state.active_request_identity = None;
        }
        state.next_request_index += 1;
        let request_id = format!("provider-wire:{epoch_id}:{}", state.next_request_index);
        let logical_request_id = format!(
            "provider-wire:{epoch_id}:logical-{}",
            state.next_request_index
        );
        let request_identity = ProviderWireRequestIdentity {
            request_id: request_id.clone(),
            logical_request_id,
            attempt_seq: 1,
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
        let base_instructions_identity = base_instructions_identity(&messages);
        let taskspace_core_protocol_identity =
            taskspace_core_protocol_identity(&messages, &base_instructions_identity);
        let taskspace_wire_contract_identity =
            taskspace_wire_contract_identity(&messages, &base_instructions_identity);
        let taskspace_final_receipt_identity = taskspace_final_receipt_identity(&messages);
        let taskspace_contract_manifest_identity = taskspace_contract_manifest_identity(
            &base_instructions_identity,
            &taskspace_core_protocol_identity,
            &taskspace_wire_contract_identity,
        );
        let event = WireShapeEvent {
            schema_version: "provider-chat-wire-trace-v9",
            event_name,
            request_id: &request_identity.request_id,
            logical_request_id: &request_identity.logical_request_id,
            attempt_seq: request_identity.attempt_seq,
            epoch_id,
            request_index: state.next_request_index,
            provider_wire_api: format!("{provider_wire_api:?}"),
            pre_wire_payload_sha256,
            provider_payload_sha256,
            provider_payload_bytes,
            section_cost: &section_cost,
            messages_hash,
            tools_hash: tools_hash.clone(),
            cache_shape_hash,
            tools_count,
            tool_choice_kind,
            tool_choice_name,
            message_count: message_shapes.len(),
            message_shapes: &message_shapes,
            base_instructions_identity,
            taskspace_wire_contract_identity,
            taskspace_final_receipt_identity,
            taskspace_contract_manifest_identity,
            taskspace_core_protocol_identity,
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
            schema_version: "provider-chat-wire-trace-v9",
            event_name: "provider.chat_wire_request_terminal",
            request_id: &request_identity.request_id,
            logical_request_id: &request_identity.logical_request_id,
            attempt_seq: request_identity.attempt_seq,
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

fn base_instructions_identity(messages: &[Value]) -> BaseInstructionsWireIdentity {
    let mut matches = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let Some(role @ ("developer" | "system")) = message.get("role").and_then(Value::as_str)
        else {
            continue;
        };
        let mut strings = Vec::new();
        collect_strings(message.get("content").unwrap_or(&Value::Null), &mut strings);
        for text in strings {
            let identity = if text == BASE_INSTRUCTIONS_WHALECODE_STANDARD {
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
            };
            if let Some((profile, version, sha256)) = identity {
                matches.push((
                    index,
                    role,
                    profile,
                    version,
                    sha256,
                    json_bytes(message).len(),
                ));
            }
        }
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
        message_index: Some(message_index),
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

fn taskspace_contract_manifest_identity(
    base_identity: &BaseInstructionsWireIdentity,
    core_protocol_identity: &TaskspaceCoreProtocolWireIdentity,
    wire_contract_identity: &TaskspaceWireContractIdentity,
) -> TaskspaceContractManifestWireIdentity {
    if base_identity.profile == Some("taskspace") && base_identity.matches_current_contract {
        let matches_current_contract = taskspace_contract_manifest_matches()
            && core_protocol_identity.matches_current_contract
            && wire_contract_identity.matches_current_contract;
        return TaskspaceContractManifestWireIdentity {
            count: 1,
            contract_id: Some(TASKSPACE_CONTRACT_MANIFEST_ID),
            version: Some(TASKSPACE_CONTRACT_MANIFEST_VERSION),
            sha256: Some(TASKSPACE_CONTRACT_MANIFEST_SHA256),
            matches_current_contract,
            unavailable_reason: (!matches_current_contract).then_some(
                if !core_protocol_identity.matches_current_contract {
                    "taskspace_core_protocol_invalid"
                } else {
                    "taskspace_wire_shape_invalid"
                },
            ),
        };
    }

    TaskspaceContractManifestWireIdentity {
        count: 0,
        contract_id: None,
        version: None,
        sha256: None,
        matches_current_contract: false,
        unavailable_reason: Some(if base_identity.profile == Some("standard") {
            "taskspace_profile_not_active"
        } else {
            "taskspace_base_identity_unavailable"
        }),
    }
}

fn taskspace_wire_contract_identity(
    messages: &[Value],
    base_identity: &BaseInstructionsWireIdentity,
) -> TaskspaceWireContractIdentity {
    let system_message_count = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("system"))
        .count();
    let mut handles = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        let Some(role) = message.get("role").and_then(Value::as_str) else {
            continue;
        };
        let mut strings = Vec::new();
        collect_strings(message.get("content").unwrap_or(&Value::Null), &mut strings);
        for text in strings {
            for _ in 0..text.matches("TaskSpaceMapHandleR7V1:").count() {
                handles.push((message_index, role));
            }
        }
    }

    if base_identity.profile != Some("taskspace") || !base_identity.matches_current_contract {
        return TaskspaceWireContractIdentity {
            system_message_count,
            expected_system_message_count: None,
            map_handle_count: handles.len(),
            map_handle_message_index: None,
            map_handle_wire_role: None,
            map_handle_is_request_tail: false,
            matches_current_contract: false,
            unavailable_reason: Some(if base_identity.profile == Some("standard") {
                "taskspace_profile_not_active"
            } else {
                "taskspace_base_identity_unavailable"
            }),
        };
    }

    let single_handle = match handles.as_slice() {
        [(message_index, role)] => Some((*message_index, *role)),
        _ => None,
    };
    let map_handle_message_index = single_handle.map(|(message_index, _)| message_index);
    let map_handle_wire_role = single_handle.map(|(_, role)| role.to_string());
    let map_handle_is_request_tail =
        single_handle.is_some_and(|(message_index, _)| message_index + 1 == messages.len());
    let handle_shape_valid = handles.is_empty()
        || single_handle.is_some_and(|(_, role)| role == "user" && map_handle_is_request_tail);
    let matches_current_contract = system_message_count == 2 && handle_shape_valid;
    TaskspaceWireContractIdentity {
        system_message_count,
        expected_system_message_count: Some(2),
        map_handle_count: handles.len(),
        map_handle_message_index,
        map_handle_wire_role,
        map_handle_is_request_tail,
        matches_current_contract,
        unavailable_reason: (!matches_current_contract).then_some(if system_message_count != 2 {
            "taskspace_system_message_count_invalid"
        } else {
            "taskspace_map_handle_position_invalid"
        }),
    }
}

fn taskspace_final_receipt_identity(messages: &[Value]) -> TaskspaceFinalReceiptIdentity {
    let receipts = messages
        .iter()
        .enumerate()
        .filter_map(|(message_index, message)| {
            let wire_role = message.get("role").and_then(Value::as_str)?;
            let payload = exact_schema_payload(
                message.get("content").unwrap_or(&Value::Null),
                TASKSPACE_FINAL_RECEIPT_SCHEMA,
            )?;
            let reservation_revision_after = payload
                .get("reservation_revision_after")
                .and_then(Value::as_u64);
            let canonical_revision = payload.get("canonical_revision").and_then(Value::as_u64);
            Some(TaskspaceFinalReceiptMessageIdentity {
                message_index,
                wire_role: wire_role.to_string(),
                control_call_id_sha256: payload
                    .get("control_call_id")
                    .and_then(Value::as_str)
                    .map(|value| json_hash(&Value::String(value.to_string()))),
                reservation_revision_after,
                canonical_revision,
                revision_delta: reservation_revision_after.zip(canonical_revision).and_then(
                    |(before, after)| {
                        Some(i64::try_from(after).ok()? - i64::try_from(before).ok()?)
                    },
                ),
                complete: payload.get("status").and_then(Value::as_str) == Some("complete")
                    && payload.get("success").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect::<Vec<_>>();
    TaskspaceFinalReceiptIdentity {
        count: receipts.len(),
        receipts,
    }
}

fn exact_schema_payload(content: &Value, schema: &str) -> Option<Value> {
    match content {
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .filter(|value| value.get("schema_version").and_then(Value::as_str) == Some(schema)),
        Value::Array(values) => values
            .iter()
            .find_map(|value| exact_schema_payload(value, schema)),
        Value::Object(object) => {
            if object.get("schema_version").and_then(Value::as_str) == Some(schema) {
                return Some(content.clone());
            }
            object
                .get("text")
                .or_else(|| object.get("content"))
                .and_then(|value| exact_schema_payload(value, schema))
        }
        _ => None,
    }
}

fn taskspace_core_protocol_identity(
    messages: &[Value],
    base_identity: &BaseInstructionsWireIdentity,
) -> TaskspaceCoreProtocolWireIdentity {
    let mut matches = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        let Some(role @ ("developer" | "system")) = message.get("role").and_then(Value::as_str)
        else {
            continue;
        };
        let mut strings = Vec::new();
        collect_strings(message.get("content").unwrap_or(&Value::Null), &mut strings);
        for text in strings {
            for _ in 0..text.matches(TASKSPACE_CORE_PROTOCOL).count() {
                matches.push((
                    message_index,
                    role,
                    text.starts_with(TASKSPACE_CORE_PROTOCOL),
                ));
            }
        }
    }

    if matches.is_empty() && base_identity.profile == Some("standard") {
        return unavailable_taskspace_core_protocol_identity(0, "taskspace_profile_not_active");
    }
    if matches.len() != 1 {
        return unavailable_taskspace_core_protocol_identity(
            matches.len(),
            if matches.is_empty() {
                "taskspace_core_protocol_missing"
            } else {
                "taskspace_core_protocol_count_invalid"
            },
        );
    }
    if base_identity.profile != Some("taskspace") || !base_identity.matches_current_contract {
        return unavailable_taskspace_core_protocol_identity(
            1,
            "taskspace_base_identity_unavailable",
        );
    }

    let (message_index, wire_role, is_first_section) = matches[0];
    let expected_message_index = base_identity.message_index.map(|index| index + 1);
    if Some(message_index) != expected_message_index || !is_first_section {
        return unavailable_taskspace_core_protocol_identity(
            1,
            "taskspace_core_protocol_position_invalid",
        );
    }

    TaskspaceCoreProtocolWireIdentity {
        count: 1,
        message_index: Some(message_index),
        wire_role: Some(wire_role.to_string()),
        section_order: Some(0),
        bytes: Some(TASKSPACE_CORE_PROTOCOL.len()),
        estimated_tokens: Some(TASKSPACE_CORE_PROTOCOL.len().div_ceil(4)),
        version: Some(TASKSPACE_CORE_PROTOCOL_VERSION),
        sha256: Some(TASKSPACE_CORE_PROTOCOL_SHA256),
        matches_current_contract: taskspace_core_protocol_matches(),
        unavailable_reason: None,
    }
}

fn unavailable_taskspace_core_protocol_identity(
    count: usize,
    reason: &'static str,
) -> TaskspaceCoreProtocolWireIdentity {
    TaskspaceCoreProtocolWireIdentity {
        count,
        message_index: None,
        wire_role: None,
        section_order: None,
        bytes: None,
        estimated_tokens: None,
        version: None,
        sha256: None,
        matches_current_contract: false,
        unavailable_reason: Some(reason),
    }
}

fn collect_strings<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => strings.push(text),
        Value::Array(values) => {
            for value in values {
                collect_strings(value, strings);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_strings(value, strings);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
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
    fn standard_base_identity_tracks_version_hash_and_position() {
        let messages = vec![
            message("developer", BASE_INSTRUCTIONS_WHALECODE_STANDARD),
            message("user", "task"),
        ];

        let identity = base_instructions_identity(&messages);
        assert_eq!(identity.count, 1);
        assert_eq!(identity.message_index, Some(0));
        assert_eq!(identity.wire_role.as_deref(), Some("developer"));
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
    fn taskspace_base_identity_accepts_chat_completions_system_role() {
        let messages = vec![message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE)];

        let identity = base_instructions_identity(&messages);
        assert_eq!(identity.count, 1);
        assert_eq!(identity.wire_role.as_deref(), Some("system"));
        assert_eq!(identity.profile, Some("taskspace"));
        assert!(identity.matches_current_contract);
    }

    #[test]
    fn taskspace_base_selects_one_matching_contract_manifest() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("system", TASKSPACE_CORE_PROTOCOL),
            message(
                "user",
                "TaskSpaceMapHandleR7V1:\nTaskSpaceMapHandleR7V1 end.",
            ),
        ];
        let base_identity = base_instructions_identity(&messages);
        let core_identity = taskspace_core_protocol_identity(&messages, &base_identity);
        let wire_identity = taskspace_wire_contract_identity(&messages, &base_identity);

        let identity =
            taskspace_contract_manifest_identity(&base_identity, &core_identity, &wire_identity);

        assert_eq!(identity.count, 1);
        assert_eq!(identity.contract_id, Some(TASKSPACE_CONTRACT_MANIFEST_ID));
        assert_eq!(identity.version, Some(TASKSPACE_CONTRACT_MANIFEST_VERSION));
        assert_eq!(identity.sha256, Some(TASKSPACE_CONTRACT_MANIFEST_SHA256));
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn standard_base_does_not_select_a_taskspace_contract_manifest() {
        let messages = vec![message("system", BASE_INSTRUCTIONS_WHALECODE_STANDARD)];
        let base_identity = base_instructions_identity(&messages);
        let core_identity = taskspace_core_protocol_identity(&messages, &base_identity);
        let wire_identity = taskspace_wire_contract_identity(&messages, &base_identity);

        let identity =
            taskspace_contract_manifest_identity(&base_identity, &core_identity, &wire_identity);

        assert_eq!(identity.count, 0);
        assert_eq!(identity.contract_id, None);
        assert!(!identity.matches_current_contract);
        assert_eq!(
            identity.unavailable_reason,
            Some("taskspace_profile_not_active")
        );
    }

    #[test]
    fn taskspace_wire_contract_accepts_two_system_messages_and_user_tail_handle() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("system", TASKSPACE_CORE_PROTOCOL),
            message("user", "task"),
            message(
                "user",
                "TaskSpaceMapHandleR7V1:\nTaskSpaceMapHandleR7V1 end.",
            ),
        ];
        let base_identity = base_instructions_identity(&messages);

        let identity = taskspace_wire_contract_identity(&messages, &base_identity);

        assert_eq!(identity.system_message_count, 2);
        assert_eq!(identity.expected_system_message_count, Some(2));
        assert_eq!(identity.map_handle_count, 1);
        assert_eq!(identity.map_handle_message_index, Some(3));
        assert_eq!(identity.map_handle_wire_role.as_deref(), Some("user"));
        assert!(identity.map_handle_is_request_tail);
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn taskspace_wire_contract_rejects_static_system_map_handle() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("system", TASKSPACE_CORE_PROTOCOL),
            message(
                "system",
                "TaskSpaceMapHandleR7V1:\nTaskSpaceMapHandleR7V1 end.",
            ),
            message("user", "task"),
        ];
        let base_identity = base_instructions_identity(&messages);
        let core_identity = taskspace_core_protocol_identity(&messages, &base_identity);
        let wire_identity = taskspace_wire_contract_identity(&messages, &base_identity);

        assert_eq!(wire_identity.system_message_count, 3);
        assert!(!wire_identity.matches_current_contract);
        assert_eq!(
            wire_identity.unavailable_reason,
            Some("taskspace_system_message_count_invalid")
        );
        let manifest_identity =
            taskspace_contract_manifest_identity(&base_identity, &core_identity, &wire_identity);
        assert!(!manifest_identity.matches_current_contract);
        assert_eq!(
            manifest_identity.unavailable_reason,
            Some("taskspace_wire_shape_invalid")
        );
    }

    #[test]
    fn taskspace_wire_contract_accepts_no_handle_for_projection_policies() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("system", TASKSPACE_CORE_PROTOCOL),
            message("user", "task"),
        ];
        let base_identity = base_instructions_identity(&messages);

        let identity = taskspace_wire_contract_identity(&messages, &base_identity);

        assert_eq!(identity.system_message_count, 2);
        assert_eq!(identity.map_handle_count, 0);
        assert_eq!(identity.map_handle_message_index, None);
        assert_eq!(identity.map_handle_wire_role, None);
        assert!(!identity.map_handle_is_request_tail);
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn final_receipt_identity_preserves_exact_wire_role_and_revision_facts() {
        let receipt = serde_json::json!({
            "schema_version": TASKSPACE_FINAL_RECEIPT_SCHEMA,
            "status": "complete",
            "success": true,
            "control_call_id": "control-1",
            "reservation_revision_after": 4,
            "canonical_revision": 6,
        })
        .to_string();
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("user", "task"),
            message("system", &receipt),
        ];

        let identity = taskspace_final_receipt_identity(&messages);

        assert_eq!(identity.count, 1);
        assert_eq!(identity.receipts[0].message_index, 2);
        assert_eq!(identity.receipts[0].wire_role, "system");
        assert_eq!(identity.receipts[0].reservation_revision_after, Some(4));
        assert_eq!(identity.receipts[0].canonical_revision, Some(6));
        assert_eq!(identity.receipts[0].revision_delta, Some(2));
        assert!(identity.receipts[0].control_call_id_sha256.is_some());
        assert!(identity.receipts[0].complete);
    }

    #[test]
    fn final_receipt_identity_ignores_user_text_that_only_mentions_schema() {
        let messages = vec![message(
            "user",
            "please explain TaskSpaceResponseFinalReceiptV1 without emitting one",
        )];

        let identity = taskspace_final_receipt_identity(&messages);

        assert_eq!(identity.count, 0);
        assert!(identity.receipts.is_empty());
    }

    #[test]
    fn taskspace_core_protocol_is_the_second_system_message_first_section() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message(
                "system",
                &format!("{TASKSPACE_CORE_PROTOCOL}\n<permissions>stable</permissions>"),
            ),
            message("user", "task"),
        ];
        let base_identity = base_instructions_identity(&messages);

        let identity = taskspace_core_protocol_identity(&messages, &base_identity);

        assert_eq!(identity.count, 1);
        assert_eq!(identity.message_index, Some(1));
        assert_eq!(identity.wire_role.as_deref(), Some("system"));
        assert_eq!(identity.section_order, Some(0));
        assert_eq!(identity.version, Some(TASKSPACE_CORE_PROTOCOL_VERSION));
        assert_eq!(identity.sha256, Some(TASKSPACE_CORE_PROTOCOL_SHA256));
        assert!(identity.matches_current_contract);
        assert_eq!(identity.unavailable_reason, None);
    }

    #[test]
    fn duplicate_taskspace_core_protocol_is_invalid() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            message("system", TASKSPACE_CORE_PROTOCOL),
            message("system", TASKSPACE_CORE_PROTOCOL),
        ];
        let base_identity = base_instructions_identity(&messages);

        let identity = taskspace_core_protocol_identity(&messages, &base_identity);

        assert_eq!(identity.count, 2);
        assert!(!identity.matches_current_contract);
        assert_eq!(
            identity.unavailable_reason,
            Some("taskspace_core_protocol_count_invalid")
        );
    }

    #[test]
    fn standard_wire_has_no_taskspace_core_protocol() {
        let messages = vec![
            message("system", BASE_INSTRUCTIONS_WHALECODE_STANDARD),
            message("system", "permissions"),
        ];
        let base_identity = base_instructions_identity(&messages);

        let identity = taskspace_core_protocol_identity(&messages, &base_identity);

        assert_eq!(identity.count, 0);
        assert_eq!(
            identity.unavailable_reason,
            Some("taskspace_profile_not_active")
        );
    }

    #[test]
    fn unknown_or_user_quoted_base_is_not_counted() {
        let messages = vec![message("user", BASE_INSTRUCTIONS_WHALECODE_STANDARD)];

        let identity = base_instructions_identity(&messages);
        assert_eq!(identity.count, 0);
        assert_eq!(
            identity.unavailable_reason,
            Some("base_instructions_unrecognized")
        );
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
