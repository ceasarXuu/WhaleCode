use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;

use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;

#[derive(Clone, Debug)]
pub(crate) enum ProviderToolDeclaration {
    Ready(ToolCall),
    BuildFailed(BuildFailedToolDeclaration),
    UnpairedBuildFailed(UnpairedToolDeclaration),
    RejectedNative(RejectedNativeToolDeclaration),
}

#[derive(Clone, Debug)]
pub(crate) struct BuildFailedToolDeclaration {
    call_id: String,
    tool_name: String,
    payload_kind: &'static str,
    pairing: ToolCallPairing,
    error: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RejectedNativeToolDeclaration {
    call_id: Option<String>,
    tool_name: &'static str,
    payload_kind: &'static str,
    error: String,
}

#[derive(Clone, Debug)]
pub(crate) struct UnpairedToolDeclaration {
    call_id: Option<String>,
    tool_name: &'static str,
    payload_kind: &'static str,
    error: String,
}

#[derive(Clone, Copy, Debug)]
enum ToolCallPairing {
    Function,
    Custom,
    ToolSearch,
}

impl ProviderToolDeclaration {
    pub(crate) fn ready(call: ToolCall) -> Self {
        Self::Ready(call)
    }

    pub(crate) fn build_failed(item: &ResponseItem, error: impl Into<String>) -> Self {
        let error = error.into();
        let failed = match item {
            ResponseItem::FunctionCall { call_id, name, .. } => Some(BuildFailedToolDeclaration {
                call_id: call_id.clone(),
                tool_name: name.clone(),
                payload_kind: "function",
                pairing: ToolCallPairing::Function,
                error: error.clone(),
            }),
            ResponseItem::CustomToolCall { call_id, name, .. } => {
                Some(BuildFailedToolDeclaration {
                    call_id: call_id.clone(),
                    tool_name: name.clone(),
                    payload_kind: "custom",
                    pairing: ToolCallPairing::Custom,
                    error: error.clone(),
                })
            }
            ResponseItem::LocalShellCall { call_id, id, .. } => call_id
                .as_ref()
                .or(id.as_ref())
                .map(|call_id| BuildFailedToolDeclaration {
                    call_id: call_id.clone(),
                    tool_name: "local_shell".to_string(),
                    payload_kind: "local_shell",
                    pairing: ToolCallPairing::Function,
                    error: error.clone(),
                }),
            ResponseItem::ToolSearchCall {
                call_id: Some(call_id),
                ..
            } => Some(BuildFailedToolDeclaration {
                call_id: call_id.clone(),
                tool_name: "tool_search".to_string(),
                payload_kind: "tool_search",
                pairing: ToolCallPairing::ToolSearch,
                error: error.clone(),
            }),
            _ => None,
        };
        failed.map_or_else(
            || {
                let (tool_name, payload_kind) = unpaired_tool_identity(item);
                Self::UnpairedBuildFailed(UnpairedToolDeclaration {
                    call_id: response_item_id(item),
                    tool_name,
                    payload_kind,
                    error,
                })
            },
            Self::BuildFailed,
        )
    }

    pub(crate) fn rejected_taskspace_native(item: &ResponseItem) -> Option<Self> {
        let rejected = match item {
            ResponseItem::WebSearchCall { id, .. } => RejectedNativeToolDeclaration {
                call_id: id.clone(),
                tool_name: "web_search",
                payload_kind: "web_search",
                error:
                    "provider emitted web_search although that native tool is hidden in TaskSpace"
                        .to_string(),
            },
            ResponseItem::ImageGenerationCall { id, .. } => RejectedNativeToolDeclaration {
                call_id: Some(id.clone()),
                tool_name: "image_generation",
                payload_kind: "image_generation",
                error: "provider emitted image_generation although that native tool is hidden in TaskSpace"
                    .to_string(),
            },
            _ => return None,
        };
        Some(Self::RejectedNative(rejected))
    }

    pub(crate) fn is_invalid(&self) -> bool {
        !matches!(self, Self::Ready(_))
    }

    pub(crate) fn identity_key(&self) -> String {
        match self {
            Self::Ready(call) => format!("ready:{}", call.call_id),
            Self::BuildFailed(failed) => format!("build_failed:{}", failed.call_id),
            Self::UnpairedBuildFailed(failed) => format!(
                "unpaired_build_failed:{}:{}",
                failed.tool_name,
                failed.call_id.as_deref().unwrap_or("<missing>")
            ),
            Self::RejectedNative(rejected) => format!(
                "rejected_native:{}:{}",
                rejected.tool_name,
                rejected.call_id.as_deref().unwrap_or("<missing>")
            ),
        }
    }

    pub(crate) fn deduplicates_stream_events(&self) -> bool {
        matches!(self, Self::RejectedNative(_))
    }

    pub(crate) fn descriptor(&self) -> serde_json::Value {
        match self {
            Self::Ready(call) => serde_json::json!({
                "status": "ready",
                "call_id": call.call_id,
                "tool": call.provider_tool_name_display(),
                "payload_kind": payload_kind(&call.payload),
            }),
            Self::BuildFailed(failed) => serde_json::json!({
                "status": "build_failed",
                "call_id": failed.call_id,
                "tool": failed.tool_name,
                "payload_kind": failed.payload_kind,
                "error": failed.error,
            }),
            Self::UnpairedBuildFailed(failed) => serde_json::json!({
                "status": "build_failed_unpaired",
                "call_id": failed.call_id,
                "tool": failed.tool_name,
                "payload_kind": failed.payload_kind,
                "error": failed.error,
            }),
            Self::RejectedNative(rejected) => serde_json::json!({
                "status": "rejected_native",
                "call_id": rejected.call_id,
                "tool": rejected.tool_name,
                "payload_kind": rejected.payload_kind,
                "error": rejected.error,
            }),
        }
    }

    pub(crate) fn rejection_responses(
        &self,
        response_failure_payload: &str,
    ) -> Vec<ResponseInputItem> {
        match self {
            Self::Ready(call) => {
                ToolCallRuntime::invalid_call_responses(call, response_failure_payload)
            }
            Self::BuildFailed(failed) => {
                vec![failed.pairing_response(response_failure_payload)]
            }
            Self::UnpairedBuildFailed(_) | Self::RejectedNative(_) => Vec::new(),
        }
    }
}

impl BuildFailedToolDeclaration {
    fn pairing_response(&self, message: &str) -> ResponseInputItem {
        let output = || FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(message.to_string()),
            success: Some(false),
        };
        match self.pairing {
            ToolCallPairing::Function => ResponseInputItem::FunctionCallOutput {
                call_id: self.call_id.clone(),
                output: output(),
            },
            ToolCallPairing::Custom => ResponseInputItem::CustomToolCallOutput {
                call_id: self.call_id.clone(),
                name: None,
                output: output(),
            },
            ToolCallPairing::ToolSearch => ResponseInputItem::ToolSearchOutput {
                call_id: self.call_id.clone(),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: Vec::new(),
            },
        }
    }
}

pub(crate) fn provider_response_failure_fact(
    reason_code: &'static str,
    payload: serde_json::Value,
    failure_provenance: serde_json::Value,
) -> ResponseInputItem {
    ResponseInputItem::Message {
        role: "developer".to_string(),
        content: vec![ContentItem::InputText {
            text: serde_json::json!({
                "schema_version": "ProviderToolResponsePreflightV2",
                "status": "protocol_failed",
                "success": false,
                "state_commit": false,
                "failure_provenance": failure_provenance,
                "error": {
                    "class": "protocol",
                    "code": reason_code,
                },
                "response": payload,
            })
            .to_string(),
        }],
    }
}

fn payload_kind(payload: &ToolPayload) -> &'static str {
    match payload {
        ToolPayload::Function { .. } => "function",
        ToolPayload::ToolSearch { .. } => "tool_search",
        ToolPayload::Custom { .. } => "custom",
        ToolPayload::LocalShell { .. } => "local_shell",
        ToolPayload::Mcp { .. } => "mcp",
    }
}

fn unpaired_tool_identity(item: &ResponseItem) -> (&'static str, &'static str) {
    match item {
        ResponseItem::LocalShellCall { .. } => ("local_shell", "local_shell"),
        ResponseItem::ToolSearchCall { .. } => ("tool_search", "tool_search"),
        ResponseItem::WebSearchCall { .. } => ("web_search", "web_search"),
        ResponseItem::ImageGenerationCall { .. } => ("image_generation", "image_generation"),
        _ => ("unidentifiable_provider_tool", "unknown"),
    }
}

fn response_item_id(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::LocalShellCall { call_id, id, .. } => {
            call_id.as_ref().or(id.as_ref()).cloned()
        }
        ResponseItem::ToolSearchCall { call_id, id, .. } => {
            call_id.as_ref().or(id.as_ref()).cloned()
        }
        ResponseItem::WebSearchCall { id, .. } => id.clone(),
        ResponseItem::ImageGenerationCall { id, .. } => Some(id.clone()),
        ResponseItem::FunctionCall { call_id, .. }
        | ResponseItem::CustomToolCall { call_id, .. } => Some(call_id.clone()),
        _ => None,
    }
}
