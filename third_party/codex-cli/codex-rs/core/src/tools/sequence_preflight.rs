use codex_protocol::models::ResponseInputItem;

use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;

pub(crate) const REQUEST_MULTIPLE_PATCHES_CODE: &str =
    "request_multiple_apply_patch_calls_not_allowed";
pub(crate) const REQUEST_MANIFEST_INVALID_CODE: &str = "request_tool_manifest_invalid";

#[derive(Debug)]
pub(crate) struct ToolSequencePreflightFailure {
    pub(crate) reason_code: &'static str,
    pub(crate) message: String,
    pub(crate) request_patch_count: Option<usize>,
    pub(crate) declared_tool_count: Option<usize>,
}

impl ToolSequencePreflightFailure {
    pub(crate) fn outputs(&self, calls: &[ToolCall]) -> Vec<ResponseInputItem> {
        let payload = serde_json::json!({
            "schema_version": "ToolSequencePreflightResultV1",
            "status": "protocol_failed",
            "success": false,
            "error": {
                "class": "protocol",
                "code": self.reason_code,
                "message": self.message,
            },
            "request": {
                "tool_call_count": calls.len(),
                "patch_call_count": self.request_patch_count,
                "executed_tool_call_count": 0,
            },
        })
        .to_string();
        calls
            .iter()
            .map(|call| ToolCallRuntime::invalid_call_response(call, payload.clone()))
            .collect()
    }
}

pub(crate) fn validate_tool_sequence(
    calls: &[ToolCall],
) -> Result<ToolSequenceManifest, ToolSequencePreflightFailure> {
    let manifest = ToolSequenceManifest::from_calls(calls).map_err(|message| {
        ToolSequencePreflightFailure {
            reason_code: REQUEST_MANIFEST_INVALID_CODE,
            message,
            request_patch_count: None,
            declared_tool_count: None,
        }
    })?;
    if manifest.request_patch_count > 1 {
        return Err(ToolSequencePreflightFailure {
            reason_code: REQUEST_MULTIPLE_PATCHES_CODE,
            message: format!(
                "provider response declares {} apply_patch calls; maximum is 1; no tool calls were executed",
                manifest.request_patch_count
            ),
            request_patch_count: Some(manifest.request_patch_count),
            declared_tool_count: Some(manifest.entries.len()),
        });
    }
    Ok(manifest)
}
