use codex_protocol::models::ResponseInputItem;

use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;
use crate::tools::sequence_manifest::is_boundary_action;
use crate::tools::taskspace_binding::ACTIVE_BINDING;
use crate::tools::taskspace_binding::AFTER_BOUNDARY_BINDING;

pub(crate) const REQUEST_MULTIPLE_PATCHES_CODE: &str =
    "request_multiple_apply_patch_calls_not_allowed";
pub(crate) const TASKSPACE_BINDING_REQUIRED_CODE: &str = "taskspace_binding_required";
pub(crate) const TASKSPACE_BINDING_INVALID_CODE: &str = "taskspace_binding_invalid";
pub(crate) const TASKSPACE_BINDING_MODE_MISMATCH_CODE: &str = "taskspace_binding_mode_mismatch";
pub(crate) const TASKSPACE_CONTROL_BINDING_FORBIDDEN_CODE: &str =
    "taskspace_control_binding_forbidden";
pub(crate) const TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE: &str =
    "taskspace_control_arguments_invalid";
pub(crate) const TASKSPACE_TOOL_SHAPE_UNSUPPORTED_CODE: &str = "taskspace_tool_shape_unsupported";
pub(crate) const TASKSPACE_BOUNDARY_REQUIRES_ACTION_CODE: &str =
    "taskspace_boundary_requires_after_boundary_action";
pub(crate) const TASKSPACE_AFTER_BOUNDARY_REQUIRES_CONTROL_CODE: &str =
    "taskspace_after_boundary_requires_boundary_control";

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
        canonical_revision: Option<u64>,
    ) -> Vec<ResponseInputItem> {
        let actual_sequence = ToolSequenceManifest::from_calls(calls)
            .entries
            .into_iter()
            .map(|entry| {
                serde_json::json!({
                    "tool": entry.tool_name,
                    "control_action": entry.taskspace_control_action,
                    "taskspace_binding": entry.taskspace_binding,
                    "payload_kind": entry.payload_kind,
                })
            })
            .collect::<Vec<_>>();
        let expected_sequence = match self.reason_code {
            TASKSPACE_BOUNDARY_REQUIRES_ACTION_CODE => serde_json::json!({
                "immediately_after_boundary": {
                    "tool_kind": "ordinary_tool",
                    "taskspace_binding": AFTER_BOUNDARY_BINDING,
                }
            }),
            TASKSPACE_AFTER_BOUNDARY_REQUIRES_CONTROL_CODE => serde_json::json!({
                "immediately_before_after_boundary": {
                    "tool": "taskspace_control",
                    "action": [
                        "initialize_map",
                        "bind_node",
                        "complete_then_continue",
                    ],
                }
            }),
            _ => serde_json::Value::Null,
        };
        let payload = serde_json::json!({
            "schema_version": "ToolSequencePreflightResultV1",
            "status": "protocol_failed",
            "success": false,
            "state_commit": false,
            "canonical_revision": canonical_revision,
            "error": {
                "class": "protocol",
                "code": self.reason_code,
                "message": self.message,
            },
            "request": {
                "tool_call_count": calls.len(),
                "patch_call_count": self.request_patch_count,
                "executed_tool_call_count": 0,
                "actual_sequence": actual_sequence,
                "expected_sequence": expected_sequence,
            },
        })
        .to_string();
        let responses = calls
            .iter()
            .flat_map(|call| ToolCallRuntime::invalid_call_responses(call, payload.clone()))
            .collect::<Vec<_>>();
        let (mut pairing_outputs, supplemental_outputs): (Vec<_>, Vec<_>) = responses
            .into_iter()
            .partition(|response| !matches!(response, ResponseInputItem::Message { .. }));
        pairing_outputs.extend(supplemental_outputs);
        pairing_outputs
    }
}

pub(crate) fn validate_tool_sequence(
    calls: &[ToolCall],
    taskspace_active: bool,
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

    for (call, entry) in calls.iter().zip(&manifest.entries) {
        if !taskspace_active {
            if entry.taskspace_binding.is_some() {
                return Err(failure(
                    TASKSPACE_BINDING_MODE_MISMATCH_CODE,
                    "taskspace_binding is not available in Standard mode",
                    &manifest,
                ));
            }
            continue;
        }
        if entry.unsupported_taskspace_payload {
            return Err(failure(
                TASKSPACE_TOOL_SHAPE_UNSUPPORTED_CODE,
                format!(
                    "TaskSpace cannot sequence provider payload kind `{}`; no tool calls were executed",
                    entry.payload_kind
                ),
                &manifest,
            ));
        }
        if entry.is_taskspace_control {
            if entry.taskspace_binding.is_some() {
                return Err(failure(
                    TASKSPACE_CONTROL_BINDING_FORBIDDEN_CODE,
                    "taskspace_control cannot carry taskspace_binding",
                    &manifest,
                ));
            }
            let ToolPayload::Function { arguments } = &call.payload else {
                return Err(failure(
                    TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE,
                    "taskspace_control must use function arguments",
                    &manifest,
                ));
            };
            if let Err(error) = parse_taskspace_control_args(arguments) {
                return Err(failure(
                    TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE,
                    control_argument_error_message(error),
                    &manifest,
                ));
            }
            continue;
        }
        if entry.requires_taskspace_binding && entry.taskspace_binding.is_none() {
            return Err(failure(
                TASKSPACE_BINDING_REQUIRED_CODE,
                "a TaskSpace ordinary Tool is missing required taskspace_binding",
                &manifest,
            ));
        }
        if let Some(binding) = entry.taskspace_binding.as_deref()
            && binding != ACTIVE_BINDING
            && binding != AFTER_BOUNDARY_BINDING
        {
            return Err(failure(
                TASKSPACE_BINDING_INVALID_CODE,
                "taskspace_binding must be active or after_boundary",
                &manifest,
            ));
        }
    }

    if taskspace_active {
        for (index, entry) in manifest.entries.iter().enumerate() {
            if is_boundary_action(entry.taskspace_control_action.as_deref()) {
                let paired = manifest.entries.get(index + 1).is_some_and(|next| {
                    !next.is_taskspace_control
                        && next.taskspace_binding.as_deref() == Some(AFTER_BOUNDARY_BINDING)
                });
                if !paired {
                    return Err(failure(
                        TASKSPACE_BOUNDARY_REQUIRES_ACTION_CODE,
                        "a boundary taskspace_control must be immediately followed by an ordinary Tool with taskspace_binding after_boundary",
                        &manifest,
                    ));
                }
            }
            if entry.taskspace_binding.as_deref() == Some(AFTER_BOUNDARY_BINDING) {
                let paired = index.checked_sub(1).and_then(|previous| {
                    manifest
                        .entries
                        .get(previous)
                        .map(|entry| is_boundary_action(entry.taskspace_control_action.as_deref()))
                }) == Some(true);
                if !paired {
                    return Err(failure(
                        TASKSPACE_AFTER_BOUNDARY_REQUIRES_CONTROL_CODE,
                        "taskspace_binding after_boundary must immediately follow a boundary taskspace_control",
                        &manifest,
                    ));
                }
            }
        }
    }
    Ok(manifest)
}

fn control_argument_error_message(error: FunctionCallError) -> String {
    match error {
        FunctionCallError::RespondToModel(message) => message,
        FunctionCallError::MissingLocalShellCallId => {
            "taskspace_control arguments are missing a call identity".to_string()
        }
        FunctionCallError::Fatal(_) => {
            "taskspace_control arguments failed canonical parsing".to_string()
        }
    }
}

fn failure(
    reason_code: &'static str,
    message: impl Into<String>,
    manifest: &ToolSequenceManifest,
) -> ToolSequencePreflightFailure {
    ToolSequencePreflightFailure {
        reason_code,
        message: message.into(),
        request_patch_count: Some(manifest.request_patch_count),
        declared_tool_count: Some(manifest.entries.len()),
    }
}
