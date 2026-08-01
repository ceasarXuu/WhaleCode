use std::collections::HashSet;

use codex_protocol::models::ResponseInputItem;

use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolPayload;
use crate::tools::failure_provenance::provider_response_failure_provenance;
use crate::tools::handlers::taskspace_control_args::TaskSpaceActionArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::handlers::taskspace_control_args::parse_taskspace_control_args;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::sequence_manifest::ToolSequenceManifest;
use crate::tools::sequence_manifest::is_taskspace_control;

pub(crate) const REQUEST_MULTIPLE_PATCHES_CODE: &str =
    "request_multiple_apply_patch_calls_not_allowed";
pub(crate) const TASKSPACE_CONTROL_REQUIRED_CODE: &str = "taskspace_control_required";
pub(crate) const TASKSPACE_CONTROL_MULTIPLE_CODE: &str = "taskspace_control_multiple";
pub(crate) const TASKSPACE_CONTROL_MUST_BE_FIRST_CODE: &str = "taskspace_control_must_be_first";
pub(crate) const TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE: &str =
    "taskspace_control_arguments_invalid";
pub(crate) const TASKSPACE_ACTION_COUNT_MISMATCH_CODE: &str = "taskspace_action_count_mismatch";
pub(crate) const TASKSPACE_ACTION_TOOL_MISMATCH_CODE: &str = "taskspace_action_tool_mismatch";
pub(crate) const TASKSPACE_DUPLICATE_CALL_ID_CODE: &str = "taskspace_duplicate_call_id";
pub(crate) const TASKSPACE_EMPTY_CALL_ID_CODE: &str = "taskspace_empty_call_id";
pub(crate) const TASKSPACE_CONTROL_ONLY_ACTION_HAS_SIBLINGS_CODE: &str =
    "taskspace_control_only_action_has_siblings";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskSpaceDeclaredCall {
    pub(crate) call_id: String,
    pub(crate) call_index: usize,
    pub(crate) node_id: String,
    pub(crate) tool_name: String,
}

#[derive(Clone, Debug)]
pub(crate) enum ToolSequencePlan {
    Standard,
    TaskSpaceControlOnly,
    TaskSpaceExecute {
        control_index: usize,
        args: TaskSpaceControlArgs,
        declared_calls: Vec<TaskSpaceDeclaredCall>,
    },
}

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
                    "payload_kind": entry.payload_kind,
                })
            })
            .collect::<Vec<_>>();
        let failure_provenance =
            provider_response_failure_provenance(calls.iter().map(|call| call.call_id.as_str()));
        let payload = serde_json::json!({
            "schema_version": "ToolSequencePreflightResultV3",
            "status": "protocol_failed",
            "success": false,
            "state_commit": false,
            "canonical_revision": canonical_revision,
            "failure_provenance": failure_provenance,
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
            },
        });
        let payload_text = payload.to_string();
        let (mut pairing, supplemental): (Vec<_>, Vec<_>) = calls
            .iter()
            .flat_map(|call| {
                ToolCallRuntime::provider_response_rejection_responses(call, payload_text.clone())
            })
            .partition(|response| !matches!(response, ResponseInputItem::Message { .. }));
        pairing.extend(supplemental);
        pairing.push(ToolCallRuntime::factual_message(payload));
        pairing
    }
}

pub(crate) fn validate_tool_sequence(
    calls: &[ToolCall],
    taskspace_active: bool,
) -> Result<(ToolSequenceManifest, ToolSequencePlan), ToolSequencePreflightFailure> {
    let manifest = ToolSequenceManifest::from_calls(calls);
    if manifest.request_patch_count > 1 {
        return Err(failure(
            REQUEST_MULTIPLE_PATCHES_CODE,
            format!(
                "provider response declares {} apply_patch calls; maximum is 1",
                manifest.request_patch_count
            ),
            &manifest,
        ));
    }
    if !taskspace_active {
        return Ok((manifest, ToolSequencePlan::Standard));
    }
    validate_taskspace_call_ids(calls, &manifest)?;

    let control_indices = calls
        .iter()
        .enumerate()
        .filter_map(|(index, call)| is_taskspace_control(call).then_some(index))
        .collect::<Vec<_>>();
    let control_index = match control_indices.as_slice() {
        [] => {
            return Err(failure(
                TASKSPACE_CONTROL_REQUIRED_CODE,
                "TaskSpace response requires one taskspace_control manifest",
                &manifest,
            ));
        }
        [index] => *index,
        _ => {
            return Err(failure(
                TASKSPACE_CONTROL_MULTIPLE_CODE,
                "TaskSpace response contains multiple taskspace_control manifests",
                &manifest,
            ));
        }
    };
    if control_index != 0 {
        return Err(failure(
            TASKSPACE_CONTROL_MUST_BE_FIRST_CODE,
            "taskspace_control must be the first Tool call in a TaskSpace response",
            &manifest,
        ));
    }
    let ToolPayload::Function { arguments } = &calls[control_index].payload else {
        return Err(failure(
            TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE,
            "taskspace_control must use function arguments",
            &manifest,
        ));
    };
    let args = parse_taskspace_control_args(arguments).map_err(|error| {
        failure(
            TASKSPACE_CONTROL_ARGUMENTS_INVALID_CODE,
            control_argument_error_message(error),
            &manifest,
        )
    })?;
    let ordinary_calls = &calls[control_index + 1..];
    match &args {
        TaskSpaceControlArgs::InitializeAndExecute { actions, .. }
        | TaskSpaceControlArgs::Execute { actions, .. }
        | TaskSpaceControlArgs::ReopenMap { actions, .. } => {
            let declared_calls = match_actions(actions, ordinary_calls, &manifest)?;
            Ok((
                manifest,
                ToolSequencePlan::TaskSpaceExecute {
                    control_index,
                    args,
                    declared_calls,
                },
            ))
        }
        TaskSpaceControlArgs::ReadMap
        | TaskSpaceControlArgs::ReadOutputRef { .. }
        | TaskSpaceControlArgs::FinishMap { .. } => {
            if !ordinary_calls.is_empty() {
                return Err(failure(
                    TASKSPACE_CONTROL_ONLY_ACTION_HAS_SIBLINGS_CODE,
                    format!(
                        "{} must not include sibling ordinary Tool calls",
                        args.action_name()
                    ),
                    &manifest,
                ));
            }
            Ok((manifest, ToolSequencePlan::TaskSpaceControlOnly))
        }
    }
}

fn validate_taskspace_call_ids(
    calls: &[ToolCall],
    manifest: &ToolSequenceManifest,
) -> Result<(), ToolSequencePreflightFailure> {
    let mut seen = HashSet::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        if call.call_id.trim().is_empty() {
            return Err(failure(
                TASKSPACE_EMPTY_CALL_ID_CODE,
                format!(
                    "TaskSpace response Tool call at index {index} requires a non-empty call_id"
                ),
                manifest,
            ));
        }
        if !seen.insert(call.call_id.as_str()) {
            return Err(failure(
                TASKSPACE_DUPLICATE_CALL_ID_CODE,
                format!(
                    "TaskSpace response call_id `{}` is duplicated at index {index}",
                    call.call_id
                ),
                manifest,
            ));
        }
    }
    Ok(())
}

fn match_actions(
    actions: &[TaskSpaceActionArgs],
    calls: &[ToolCall],
    manifest: &ToolSequenceManifest,
) -> Result<Vec<TaskSpaceDeclaredCall>, ToolSequencePreflightFailure> {
    if actions.len() != calls.len() {
        return Err(failure(
            TASKSPACE_ACTION_COUNT_MISMATCH_CODE,
            format!(
                "TaskSpace manifest declares {} actions for {} sibling Tool calls",
                actions.len(),
                calls.len()
            ),
            manifest,
        ));
    }
    actions
        .iter()
        .zip(calls)
        .enumerate()
        .map(|(offset, (action, call))| {
            let actual = call.provider_tool_name_display();
            if action.tool != actual {
                return Err(failure(
                    TASKSPACE_ACTION_TOOL_MISMATCH_CODE,
                    format!(
                        "TaskSpace actions[{offset}].tool is `{}`, sibling Tool is `{actual}`",
                        action.tool
                    ),
                    manifest,
                ));
            }
            Ok(TaskSpaceDeclaredCall {
                call_id: call.call_id.clone(),
                call_index: offset,
                node_id: action.node_id.clone(),
                tool_name: actual,
            })
        })
        .collect()
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
