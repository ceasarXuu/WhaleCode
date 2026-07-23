use codex_protocol::models::ResponseInputItem;

use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;

pub(crate) const REQUEST_MULTIPLE_PATCHES_CODE: &str =
    "request_multiple_apply_patch_calls_not_allowed";
pub(crate) const BOUNDARY_ACTION_REQUIRES_FOLLOW_UP_CODE: &str =
    "taskspace_boundary_action_requires_follow_up";

#[derive(Debug)]
pub(crate) struct ToolSequencePreflightFailure {
    pub(crate) reason_code: &'static str,
    pub(crate) message: String,
    pub(crate) request_patch_count: Option<usize>,
    pub(crate) declared_tool_count: Option<usize>,
}

impl ToolSequencePreflightFailure {
    pub(crate) fn outputs(
        &self,
        calls: &[ToolCall],
        _canonical_revision: Option<u64>,
    ) -> Vec<ResponseInputItem> {
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
    let manifest = ToolSequenceManifest::from_calls(calls);
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
    for (index, entry) in manifest.entries.iter().enumerate() {
        let Some(action) = entry.taskspace_control_action.as_deref() else {
            continue;
        };
        if !matches!(
            action,
            "initialize_map" | "bind_node" | "complete_then_continue"
        ) {
            continue;
        }
        let has_immediate_follow_up = manifest
            .entries
            .get(index + 1)
            .is_some_and(|next| !next.is_taskspace_control);
        if !has_immediate_follow_up {
            return Err(ToolSequencePreflightFailure {
                reason_code: BOUNDARY_ACTION_REQUIRES_FOLLOW_UP_CODE,
                message: format!(
                    "taskspace_control action `{action}` must be immediately followed by a real action Tool in the same provider response; no calls were executed"
                ),
                request_patch_count: Some(manifest.request_patch_count),
                declared_tool_count: Some(manifest.entries.len()),
            });
        }
    }
    Ok(manifest)
}
