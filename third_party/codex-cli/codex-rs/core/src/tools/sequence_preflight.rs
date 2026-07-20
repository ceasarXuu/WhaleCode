use codex_protocol::models::ResponseInputItem;

use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceRequiredNextCall;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::handlers::taskspace_control_output::TaskSpaceFailureResult;
use crate::tools::handlers::taskspace_control_output::format_failure_result;
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
    pub(crate) failed_call_id: Option<String>,
    pub(crate) required_next_call: Option<TaskSpaceRequiredNextCall>,
    pub(crate) observed_next_tool: Option<String>,
}

impl ToolSequencePreflightFailure {
    pub(crate) fn outputs(
        &self,
        calls: &[ToolCall],
        canonical_revision: Option<u64>,
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
            .map(|call| {
                let control_payload = self
                    .failed_call_id
                    .as_deref()
                    .filter(|failed_call_id| *failed_call_id == call.call_id)
                    .and_then(|_| taskspace_arguments(call))
                    .and_then(|arguments| parse_taskspace_control_args(arguments).ok())
                    .map(|arguments| {
                        let action = arguments.action_name();
                        let message = format!(
                            "{action} requires a following top-level {} call in the same response",
                            self.required_next_call
                                .map_or("non-control", TaskSpaceRequiredNextCall::as_str)
                        );
                        format_failure_result(TaskSpaceFailureResult {
                            action: Some(action),
                            status: "protocol_failed",
                            class: "protocol",
                            code: "TASKSPACE_REQUIRED_SIBLING_MISSING",
                            message: &message,
                            canonical_revision,
                            submitted_expected_revision: arguments
                                .submitted_expected_revision(),
                            actual: serde_json::json!({
                                "observed_next_tool": self.observed_next_tool,
                                "executed_tool_call_count": 0,
                            }),
                            expected: serde_json::json!({
                                "required_next_call": self.required_next_call.map(TaskSpaceRequiredNextCall::as_str),
                            }),
                        })
                    });
                ToolCallRuntime::invalid_call_response(
                    call,
                    control_payload.unwrap_or_else(|| payload.clone()),
                )
            })
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
            None,
            None,
            None,
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
                    "taskspace_control call {} declares required_next_call={} but no immediately following top-level tool call exists; no tool calls were executed",
                    entry.call_id,
                    requirement.as_str()
                ),
                &manifest,
                Some(entry.call_id.clone()),
                Some(requirement),
                None,
            ));
        };
        match requirement {
            TaskSpaceRequiredNextCall::OrdinaryTool
                if next.is_taskspace_control || next.is_apply_patch =>
            {
                return Err(failure(
                    TASKSPACE_REQUIRED_ORDINARY_TOOL_INVALID_CODE,
                    format!(
                        "taskspace_control call {} declares required_next_call=ordinary_tool but the immediately following call is {}; no tool calls were executed",
                        entry.call_id, next.tool_name
                    ),
                    &manifest,
                    Some(entry.call_id.clone()),
                    Some(requirement),
                    Some(next.tool_name.clone()),
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
                    Some(entry.call_id.clone()),
                    Some(requirement),
                    Some(next.tool_name.clone()),
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
                    Some(entry.call_id.clone()),
                    Some(requirement),
                    Some(next.tool_name.clone()),
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
    failed_call_id: Option<String>,
    required_next_call: Option<TaskSpaceRequiredNextCall>,
    observed_next_tool: Option<String>,
) -> ToolSequencePreflightFailure {
    ToolSequencePreflightFailure {
        reason_code,
        message,
        request_patch_count: Some(manifest.request_patch_count),
        declared_tool_count: Some(manifest.entries.len()),
        failed_call_id,
        required_next_call,
        observed_next_tool,
    }
}

fn taskspace_arguments(call: &ToolCall) -> Option<&str> {
    if call.tool_name.namespace.is_some() || call.tool_name.name != "taskspace_control" {
        return None;
    }
    let ToolPayload::Function { arguments } = &call.payload else {
        return None;
    };
    Some(arguments)
}
