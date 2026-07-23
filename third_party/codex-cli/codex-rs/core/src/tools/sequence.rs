use codex_protocol::error::Result;
use codex_protocol::models::ResponseInputItem;
use futures::future::join_all;
use sha2::Digest;
use sha2::Sha256;
use tokio_util::sync::CancellationToken;

use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::provider_tool_declaration::ProviderToolDeclaration;
use crate::tools::provider_tool_declaration::provider_response_failure_fact;
use crate::tools::router::ToolCall;
use crate::tools::sequence_preflight::validate_tool_sequence;
use crate::tools::taskspace_binding::is_initialization_binding;
use crate::tools::taskspace_binding::taskspace_binding_kind;

const PROVIDER_TOOL_DECLARATION_INVALID_CODE: &str = "provider_tool_declaration_invalid";

pub(crate) struct TaskSpaceTerminalCompletion {
    pub(crate) call_id: String,
    pub(crate) carrier: TaskSpaceTerminalCarrier,
}

pub(crate) struct ToolSequenceOutcome {
    pub(crate) outputs: Vec<ResponseInputItem>,
    pub(crate) terminal_completion: Option<TaskSpaceTerminalCompletion>,
}

#[derive(Debug, PartialEq, Eq)]
enum SequenceSegment {
    Parallel { start: usize, end: usize },
    Barrier { index: usize, kind: BarrierKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarrierKind {
    TaskSpaceControl,
    TaskSpaceInitialization,
    ApplyPatch,
}

impl BarrierKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::TaskSpaceControl => "taskspace_control",
            Self::TaskSpaceInitialization => "taskspace_initialization",
            Self::ApplyPatch => "apply_patch",
        }
    }
}

pub(crate) async fn execute_provider_response_tool_sequence(
    runtime: ToolCallRuntime,
    declarations: Vec<ProviderToolDeclaration>,
    cancellation_token: CancellationToken,
) -> Result<ToolSequenceOutcome> {
    if declarations
        .iter()
        .all(|declaration| !declaration.is_invalid())
    {
        let calls = declarations
            .into_iter()
            .filter_map(|declaration| match declaration {
                ProviderToolDeclaration::Ready(call) => Some(call),
                ProviderToolDeclaration::BuildFailed(_)
                | ProviderToolDeclaration::UnpairedBuildFailed(_)
                | ProviderToolDeclaration::RejectedNative(_) => None,
            })
            .collect();
        return execute_response_tool_sequence(runtime, calls, cancellation_token).await;
    }

    let canonical_revision = runtime.taskspace_canonical_revision().await;
    let build_failure_count = declarations
        .iter()
        .filter(|declaration| {
            matches!(
                declaration,
                ProviderToolDeclaration::BuildFailed(_)
                    | ProviderToolDeclaration::UnpairedBuildFailed(_)
            )
        })
        .count();
    let rejected_native_count = declarations
        .iter()
        .filter(|declaration| matches!(declaration, ProviderToolDeclaration::RejectedNative(_)))
        .count();
    let outcome = invalid_provider_declaration_outcome(&declarations, canonical_revision);
    tracing::warn!(
        target: "codex_core::taskspace",
        reason_code = PROVIDER_TOOL_DECLARATION_INVALID_CODE,
        declared_tool_count = declarations.len(),
        build_failure_count,
        rejected_native_count,
        canonical_revision = ?canonical_revision,
        zero_dispatch = true,
        state_commit = false,
        "tool.response_provider_declaration_rejected"
    );
    Ok(outcome)
}

fn invalid_provider_declaration_outcome(
    declarations: &[ProviderToolDeclaration],
    canonical_revision: Option<u64>,
) -> ToolSequenceOutcome {
    let build_failure_count = declarations
        .iter()
        .filter(|declaration| {
            matches!(
                declaration,
                ProviderToolDeclaration::BuildFailed(_)
                    | ProviderToolDeclaration::UnpairedBuildFailed(_)
            )
        })
        .count();
    let rejected_native_count = declarations
        .iter()
        .filter(|declaration| matches!(declaration, ProviderToolDeclaration::RejectedNative(_)))
        .count();
    let descriptors = declarations
        .iter()
        .map(ProviderToolDeclaration::descriptor)
        .collect::<Vec<_>>();
    let response_payload = serde_json::json!({
        "canonical_revision": canonical_revision,
        "declared_tool_count": declarations.len(),
        "build_failure_count": build_failure_count,
        "rejected_native_count": rejected_native_count,
        "executed_tool_call_count": 0,
        "declarations": descriptors,
    });
    let failure_payload = serde_json::json!({
        "schema_version": "ProviderToolResponsePreflightV1",
        "status": "protocol_failed",
        "success": false,
        "state_commit": false,
        "error": {
            "class": "protocol",
            "code": PROVIDER_TOOL_DECLARATION_INVALID_CODE,
            "message": "the provider response contains an invalid tool declaration; no client tool calls were executed",
        },
        "response": response_payload,
    })
    .to_string();
    let responses = declarations
        .iter()
        .flat_map(|declaration| declaration.rejection_responses(&failure_payload))
        .collect::<Vec<_>>();
    let (mut pairing_outputs, supplemental_outputs): (Vec<_>, Vec<_>) = responses
        .into_iter()
        .partition(|response| !matches!(response, ResponseInputItem::Message { .. }));
    pairing_outputs.extend(supplemental_outputs);
    pairing_outputs.push(provider_response_failure_fact(
        PROVIDER_TOOL_DECLARATION_INVALID_CODE,
        response_payload,
    ));
    ToolSequenceOutcome {
        outputs: pairing_outputs,
        terminal_completion: None,
    }
}

pub(crate) async fn execute_response_tool_sequence(
    runtime: ToolCallRuntime,
    calls: Vec<ToolCall>,
    cancellation_token: CancellationToken,
) -> Result<ToolSequenceOutcome> {
    if calls.is_empty() {
        return Ok(ToolSequenceOutcome {
            outputs: Vec::new(),
            terminal_completion: None,
        });
    }

    let taskspace_active = runtime.taskspace_active().await;
    let manifest = match validate_tool_sequence(&calls, taskspace_active) {
        Ok(manifest) => manifest,
        Err(failure) => {
            let canonical_revision = runtime.taskspace_canonical_revision().await;
            tracing::warn!(
                target: "codex_core::taskspace",
                reason_code = failure.reason_code,
                request_patch_count = failure.request_patch_count,
                declared_tool_count = failure.declared_tool_count,
                call_ids = tool_sequence_call_ids(&calls),
                sequence_sha256 = tool_sequence_sha256(&calls),
                canonical_revision = ?canonical_revision,
                zero_dispatch = true,
                state_commit = false,
                "tool.response_preflight_rejected"
            );
            return Ok(ToolSequenceOutcome {
                outputs: failure.outputs(&calls, canonical_revision),
                terminal_completion: None,
            });
        }
    };
    tracing::info!(
        target: "codex_core::taskspace",
        declared_tool_count = manifest.entries.len(),
        request_patch_count = manifest.request_patch_count,
        call_ids = tool_sequence_call_ids(&calls),
        sequence_sha256 = tool_sequence_sha256(&calls),
        "tool.request_patch_count_validated"
    );
    let segments = sequence_segments(&calls);
    tracing::info!(
        target: "codex_core::taskspace",
        call_count = calls.len(),
        segment_count = segments.len(),
        "tool_response_sequence_started"
    );

    let mut outputs = Vec::with_capacity(calls.len());
    let mut supplemental_outputs = Vec::new();
    let mut prior_failure: Option<String> = None;
    let mut terminal_completion: Option<TaskSpaceTerminalCompletion> = None;
    for (segment_index, segment) in segments.into_iter().enumerate() {
        if let Some(terminal) = terminal_completion.as_ref() {
            for call in calls_for_segment(&calls, &segment) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    terminal_call_id = terminal.call_id,
                    "tool_response_sequence_call_skipped"
                );
                append_pairing_and_supplemental(
                    &mut outputs,
                    &mut supplemental_outputs,
                    ToolCallRuntime::terminal_completion_skipped_responses(call, &terminal.call_id),
                );
            }
            continue;
        }
        if let Some(prior_call_id) = prior_failure.as_deref() {
            for call in calls_for_segment(&calls, &segment) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    prior_call_id,
                    "tool_response_sequence_call_skipped"
                );
                append_pairing_and_supplemental(
                    &mut outputs,
                    &mut supplemental_outputs,
                    ToolCallRuntime::skipped_responses(call, prior_call_id),
                );
            }
            continue;
        }
        let barrier_call_id = match &segment {
            SequenceSegment::Barrier { index, .. } => Some(calls[*index].call_id.clone()),
            SequenceSegment::Parallel { .. } => None,
        };
        let barrier_kind = match &segment {
            SequenceSegment::Barrier { kind, .. } => Some(*kind),
            SequenceSegment::Parallel { .. } => None,
        };
        let segment_executions = match segment {
            SequenceSegment::Parallel { start, end } => {
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_count = end - start,
                    "tool_response_parallel_segment_started"
                );
                let futures = calls[start..end].iter().cloned().map(|call| {
                    runtime
                        .clone()
                        .handle_tool_call_for_sequence(call, cancellation_token.child_token())
                });
                join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            }
            SequenceSegment::Barrier { index, kind } => {
                let call = calls[index].clone();
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id = call.call_id,
                    tool_name = call.tool_name.display(),
                    barrier_kind = kind.as_str(),
                    "tool.barrier_started"
                );
                vec![
                    runtime
                        .clone()
                        .handle_tool_call_for_sequence(call, cancellation_token.child_token())
                        .await?,
                ]
            }
        };

        for execution in &segment_executions {
            let output = &execution.response;
            if prior_failure.is_none()
                && let Some(call_id) = execution_failure_call_id(execution)
            {
                prior_failure = Some(call_id.to_string());
            }
            if let Some(carrier) = execution.taskspace_terminal_carrier.as_ref() {
                let call_id = response_input_call_id(output).to_string();
                terminal_completion = Some(TaskSpaceTerminalCompletion {
                    call_id: call_id.clone(),
                    carrier: carrier.clone(),
                });
                tracing::info!(
                    target: "codex_core::taskspace",
                    call_id,
                    map_id = carrier.map_id,
                    revision = carrier.revision,
                    candidate_bytes = carrier.summary.len(),
                    "taskspace_agent_final_staged"
                );
            }
        }
        if let Some(call_id) = barrier_call_id {
            if prior_failure.as_deref() == Some(call_id.as_str()) {
                tracing::warn!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id,
                    barrier_kind = barrier_kind.map(BarrierKind::as_str),
                    failure_class = "tool_output_unsuccessful",
                    "tool.barrier_failed"
                );
            } else {
                tracing::info!(
                    target: "codex_core::taskspace",
                    segment_index,
                    call_id,
                    barrier_kind = barrier_kind.map(BarrierKind::as_str),
                    "tool.barrier_completed"
                );
            }
        }
        tracing::info!(
            target: "codex_core::taskspace",
            segment_index,
            failed = prior_failure.is_some(),
            "tool_response_sequence_segment_completed"
        );
        for execution in segment_executions {
            outputs.push(execution.response);
            supplemental_outputs.extend(execution.supplemental_responses);
        }
    }

    tracing::info!(
        target: "codex_core::taskspace",
        call_count = calls.len(),
        failed = prior_failure.is_some(),
        "tool_response_sequence_completed"
    );
    outputs.extend(supplemental_outputs);
    Ok(ToolSequenceOutcome {
        outputs,
        terminal_completion,
    })
}

fn append_pairing_and_supplemental(
    pairing_outputs: &mut Vec<ResponseInputItem>,
    supplemental_outputs: &mut Vec<ResponseInputItem>,
    responses: impl IntoIterator<Item = ResponseInputItem>,
) {
    for response in responses {
        if matches!(response, ResponseInputItem::Message { .. }) {
            supplemental_outputs.push(response);
        } else {
            pairing_outputs.push(response);
        }
    }
}

fn sequence_segments(calls: &[ToolCall]) -> Vec<SequenceSegment> {
    let mut segments = Vec::new();
    let mut ordinary_start = 0;
    for (index, call) in calls.iter().enumerate() {
        let Some(kind) = barrier_kind(call) else {
            continue;
        };
        if ordinary_start < index {
            segments.push(SequenceSegment::Parallel {
                start: ordinary_start,
                end: index,
            });
        }
        segments.push(SequenceSegment::Barrier { index, kind });
        ordinary_start = index + 1;
    }
    if ordinary_start < calls.len() {
        segments.push(SequenceSegment::Parallel {
            start: ordinary_start,
            end: calls.len(),
        });
    }
    segments
}

fn calls_for_segment<'a>(calls: &'a [ToolCall], segment: &SequenceSegment) -> &'a [ToolCall] {
    match *segment {
        SequenceSegment::Parallel { start, end } => &calls[start..end],
        SequenceSegment::Barrier { index, .. } => &calls[index..=index],
    }
}

fn barrier_kind(call: &ToolCall) -> Option<BarrierKind> {
    if is_initialization_binding(call.taskspace_binding.as_deref()) {
        return Some(BarrierKind::TaskSpaceInitialization);
    }
    if call.tool_name.namespace.is_some() {
        return None;
    }
    match call.tool_name.name.as_str() {
        "taskspace_control" => Some(BarrierKind::TaskSpaceControl),
        "apply_patch" => Some(BarrierKind::ApplyPatch),
        _ => None,
    }
}

fn response_input_call_id(output: &ResponseInputItem) -> &str {
    match output {
        ResponseInputItem::FunctionCallOutput { call_id, .. }
        | ResponseInputItem::McpToolCallOutput { call_id, .. }
        | ResponseInputItem::CustomToolCallOutput { call_id, .. }
        | ResponseInputItem::ToolSearchOutput { call_id, .. } => call_id,
        ResponseInputItem::Message { .. } => "unknown",
    }
}

fn tool_sequence_call_ids(calls: &[ToolCall]) -> String {
    calls
        .iter()
        .map(|call| call.call_id.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn tool_sequence_sha256(calls: &[ToolCall]) -> String {
    let mut hasher = Sha256::new();
    for call in calls {
        hasher.update(call.call_id.as_bytes());
        hasher.update([0]);
        hasher.update(call.tool_name.display().as_bytes());
        hasher.update([0]);
        hasher.update(
            call.taskspace_binding
                .as_deref()
                .and_then(taskspace_binding_kind)
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update([0xff]);
    }
    format!("{:x}", hasher.finalize())
}

fn execution_failure_call_id(
    execution: &crate::tools::parallel::ToolCallExecution,
) -> Option<&str> {
    (!execution.succeeded).then(|| response_input_call_id(&execution.response))
}

#[cfg(test)]
fn response_input_succeeded(output: &ResponseInputItem) -> bool {
    match output {
        ResponseInputItem::FunctionCallOutput { output, .. }
        | ResponseInputItem::CustomToolCallOutput { output, .. } => output.success != Some(false),
        ResponseInputItem::McpToolCallOutput { output, .. } => output.success(),
        ResponseInputItem::ToolSearchOutput { status, .. } => status == "completed",
        ResponseInputItem::Message { .. } => true,
    }
}

#[cfg(test)]
#[path = "sequence_tests.rs"]
mod tests;
