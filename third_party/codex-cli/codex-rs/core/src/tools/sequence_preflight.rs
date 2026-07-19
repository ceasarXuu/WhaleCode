use codex_protocol::models::ResponseInputItem;

use crate::tools::handlers::taskspace_control_args::TaskSpaceRequiredNextCall;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;

pub(crate) const REQUEST_MULTIPLE_PATCHES_CODE: &str =
    "request_multiple_apply_patch_calls_not_allowed";
pub(crate) const TASKSPACE_REQUIRED_NEXT_CALL_MISSING_CODE: &str =
    "taskspace_required_next_call_missing";
pub(crate) const TASKSPACE_REQUIRED_ORDINARY_TOOL_INVALID_CODE: &str =
    "taskspace_required_ordinary_tool_not_immediate";
pub(crate) const TASKSPACE_REQUIRED_PATCH_INVALID_CODE: &str =
    "taskspace_required_apply_patch_not_immediate";
pub(crate) const TASKSPACE_REQUIRED_PATCH_ARGUMENTS_INVALID_CODE: &str =
    "taskspace_required_apply_patch_arguments_invalid";

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
    let manifest = ToolSequenceManifest::from_calls(calls);
    if manifest.request_patch_count > 1 {
        return Err(failure(
            REQUEST_MULTIPLE_PATCHES_CODE,
            format!(
                "provider response declares {} apply_patch calls; maximum is 1; no tool calls were executed",
                manifest.request_patch_count
            ),
            &manifest,
        ));
    }
    for (index, entry) in manifest.entries.iter().enumerate() {
        let Some(requirement) = entry.required_next_call else {
            continue;
        };
        let Some(next) = manifest.entries.get(index + 1) else {
            return Err(failure(
                TASKSPACE_REQUIRED_NEXT_CALL_MISSING_CODE,
                format!(
                    "taskspace_control call {} declares required_next_call={} but no immediately following top-level tool call exists; the declaration does not execute or schedule that call; no tool calls were executed",
                    entry.call_id,
                    requirement.as_str()
                ),
                &manifest,
            ));
        };
        match requirement {
            TaskSpaceRequiredNextCall::OrdinaryTool
                if next.is_taskspace_control || next.is_apply_patch =>
            {
                return Err(failure(
                    TASKSPACE_REQUIRED_ORDINARY_TOOL_INVALID_CODE,
                    format!(
                        "taskspace_control call {} declares required_next_call=ordinary_tool but the immediately following call is {}; use required_next_call=apply_patch for apply_patch; no tool calls were executed",
                        entry.call_id, next.tool_name
                    ),
                    &manifest,
                ));
            }
            TaskSpaceRequiredNextCall::ApplyPatch if !next.is_apply_patch => {
                return Err(failure(
                    TASKSPACE_REQUIRED_PATCH_INVALID_CODE,
                    format!(
                        "taskspace_control call {} declares required_next_call=apply_patch but the immediately following call is {}; no tool calls were executed",
                        entry.call_id, next.tool_name
                    ),
                    &manifest,
                ));
            }
            TaskSpaceRequiredNextCall::ApplyPatch if !next.apply_patch_arguments_valid => {
                return Err(failure(
                    TASKSPACE_REQUIRED_PATCH_ARGUMENTS_INVALID_CODE,
                    format!(
                        "taskspace_control call {} declares required_next_call=apply_patch but the immediately following apply_patch arguments are invalid; no tool calls were executed",
                        entry.call_id
                    ),
                    &manifest,
                ));
            }
            TaskSpaceRequiredNextCall::OrdinaryTool | TaskSpaceRequiredNextCall::ApplyPatch => {}
        }
    }
    Ok(manifest)
}

fn failure(
    reason_code: &'static str,
    message: String,
    manifest: &ToolSequenceManifest,
) -> ToolSequencePreflightFailure {
    ToolSequencePreflightFailure {
        reason_code,
        message,
        request_patch_count: Some(manifest.request_patch_count),
        declared_tool_count: Some(manifest.entries.len()),
    }
}
