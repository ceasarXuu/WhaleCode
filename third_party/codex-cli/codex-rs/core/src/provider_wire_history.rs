use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use super::json_bytes;
use super::json_hash;

#[derive(Debug, Serialize)]
pub(super) struct ProviderWireHistoryCost {
    kind: HistoryKind,
    pub(super) count: usize,
    pub(super) bytes: usize,
    estimated_tokens: usize,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum HistoryKind {
    UserMessage,
    AssistantMessage,
    ClientToolCall,
    ClientToolOutput,
    TaskspaceExecCall,
    TaskspaceExecOutput,
    ProviderHostedItem,
    ReasoningItem,
    CompactionItem,
    OtherHistory,
}

impl HistoryKind {
    pub(super) const ALL: [Self; 10] = [
        Self::UserMessage,
        Self::AssistantMessage,
        Self::ClientToolCall,
        Self::ClientToolOutput,
        Self::TaskspaceExecCall,
        Self::TaskspaceExecOutput,
        Self::ProviderHostedItem,
        Self::ReasoningItem,
        Self::CompactionItem,
        Self::OtherHistory,
    ];

    pub(super) const fn index(self) -> usize {
        match self {
            Self::UserMessage => 0,
            Self::AssistantMessage => 1,
            Self::ClientToolCall => 2,
            Self::ClientToolOutput => 3,
            Self::TaskspaceExecCall => 4,
            Self::TaskspaceExecOutput => 5,
            Self::ProviderHostedItem => 6,
            Self::ReasoningItem => 7,
            Self::CompactionItem => 8,
            Self::OtherHistory => 9,
        }
    }
}

pub(super) fn measure(messages: &[Value]) -> Vec<ProviderWireHistoryCost> {
    let call_names = response_call_names(messages);
    let mut counts = [0usize; 10];
    let mut bytes = [0usize; 10];
    let mut values: [Vec<Value>; 10] = std::array::from_fn(|_| Vec::new());
    for message in messages {
        let kind = classify(message, &call_names);
        counts[kind.index()] += 1;
        bytes[kind.index()] += json_bytes(message).len();
        values[kind.index()].push(message.clone());
    }

    HistoryKind::ALL
        .into_iter()
        .zip(counts)
        .zip(bytes)
        .zip(values)
        .map(|(((kind, count), bytes), values)| ProviderWireHistoryCost {
            kind,
            count,
            bytes,
            estimated_tokens: bytes.div_ceil(4),
            sha256: json_hash(&Value::Array(values)),
        })
        .collect()
}

fn response_call_names(messages: &[Value]) -> HashMap<&str, &str> {
    messages
        .iter()
        .filter_map(|message| {
            let item_type = message.get("type").and_then(Value::as_str)?;
            if !matches!(
                item_type,
                "function_call" | "custom_tool_call" | "tool_search_call"
            ) {
                return None;
            }
            Some((
                message.get("call_id")?.as_str()?,
                message.get("name").and_then(Value::as_str).unwrap_or(""),
            ))
        })
        .collect()
}

fn classify(message: &Value, call_names: &HashMap<&str, &str>) -> HistoryKind {
    match message.get("type").and_then(Value::as_str) {
        Some("message") => classify_message_role(message),
        Some("function_call") => match message.get("name").and_then(Value::as_str) {
            Some("taskspace_exec") => HistoryKind::TaskspaceExecCall,
            _ => HistoryKind::ClientToolCall,
        },
        Some("function_call_output") => {
            let name = message
                .get("call_id")
                .and_then(Value::as_str)
                .and_then(|call_id| call_names.get(call_id).copied());
            if name == Some("taskspace_exec") {
                HistoryKind::TaskspaceExecOutput
            } else {
                HistoryKind::ClientToolOutput
            }
        }
        Some("local_shell_call" | "custom_tool_call" | "tool_search_call") => {
            HistoryKind::ClientToolCall
        }
        Some("mcp_tool_call_output" | "custom_tool_call_output" | "tool_search_output") => {
            HistoryKind::ClientToolOutput
        }
        Some("web_search_call" | "image_generation_call") => HistoryKind::ProviderHostedItem,
        Some("reasoning") => HistoryKind::ReasoningItem,
        Some("compaction" | "compaction_summary") => HistoryKind::CompactionItem,
        Some(_) | None => classify_message_role(message),
    }
}

fn classify_message_role(message: &Value) -> HistoryKind {
    match message.get("role").and_then(Value::as_str) {
        Some("user") => HistoryKind::UserMessage,
        Some("assistant") => HistoryKind::AssistantMessage,
        _ => HistoryKind::OtherHistory,
    }
}
