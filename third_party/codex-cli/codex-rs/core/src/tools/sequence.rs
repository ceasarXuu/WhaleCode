use codex_protocol::error::Result;
use codex_protocol::models::ResponseInputItem;
use futures::future::join_all;
use sha2::Digest;
use sha2::Sha256;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
use crate::action_map::ACTION_MAP_RESPONSE_STATE_COMMIT_FAILED_CODE;
use crate::action_map::ActionMapResponsePrepareError;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::failure_provenance::provider_response_failure_provenance;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::provider_tool_declaration::ProviderToolDeclaration;
use crate::tools::provider_tool_declaration::provider_response_failure_fact;
use crate::tools::router::ToolCall;
use crate::tools::sequence_preflight::ToolSequencePlan;
use crate::tools::sequence_preflight::validate_tool_sequence;

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
    ApplyPatch,
}

impl BarrierKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::TaskSpaceControl => "taskspace_control",
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
    let failure_provenance =
        provider_response_failure_provenance(descriptors.iter().filter_map(|descriptor| {
            descriptor
                .get("call_id")
                .and_then(serde_json::Value::as_str)
        }));
    let response_payload = serde_json::json!({
        "canonical_revision": canonical_revision,
        "declared_tool_count": declarations.len(),
        "build_failure_count": build_failure_count,
        "rejected_native_count": rejected_native_count,
        "executed_tool_call_count": 0,
        "declarations": descriptors,
    });
    let failure_payload = serde_json::json!({
        "schema_version": "ProviderToolResponsePreflightV2",
        "status": "protocol_failed",
        "success": false,
        "state_commit": false,
        "failure_provenance": failure_provenance.clone(),
        "error": {
            "class": "protocol",
            "code": PROVIDER_TOOL_DECLARATION_INVALID_CODE,
            "message": "the provider response contains an invalid tool declaration; no client tool calls were executed",
        },
        "response": response_payload,
    })
    .to_string();
    let (mut pairing_outputs, supplemental_outputs): (Vec<_>, Vec<_>) = declarations
        .iter()
        .flat_map(|declaration| declaration.rejection_responses(&failure_payload))
        .partition(|response| !matches!(response, ResponseInputItem::Message { .. }));
    pairing_outputs.extend(supplemental_outputs);
    pairing_outputs.push(provider_response_failure_fact(
        PROVIDER_TOOL_DECLARATION_INVALID_CODE,
        response_payload,
        failure_provenance,
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
    if taskspace_active {
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace_response_preflight_started",
            declared_tool_count = calls.len(),
            call_ids = tool_sequence_call_ids(&calls),
            sequence_sha256 = tool_sequence_sha256(&calls),
            "started TaskSpace response preflight"
        );
    }
    let (manifest, plan) = match validate_tool_sequence(&calls, taskspace_active) {
        Ok((manifest, plan)) => (manifest, plan),
        Err(failure) => {
            let canonical_revision = runtime.taskspace_canonical_revision().await;
            tracing::warn!(
                target: "codex_core::taskspace",
                event_name = "taskspace_response_preflight_rejected",
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
    if let ToolSequencePlan::TaskSpaceExecute {
        control_index,
        args,
        declared_calls,
    } = plan
    {
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace_action_manifest_matched",
            control_call_id = calls[control_index].call_id,
            declared_action_count = declared_calls.len(),
            request_patch_count = manifest.request_patch_count,
            sequence_sha256 = tool_sequence_sha256(&calls),
            "matched Agent-declared TaskSpace actions to native tool calls"
        );
        let prepared = match runtime
            .prepare_taskspace_response(&calls[control_index].call_id, args, declared_calls)
            .await
        {
            Ok(prepared) => prepared,
            Err(error) => {
                let canonical_revision = runtime.taskspace_canonical_revision().await;
                let violation_codes = error.violation_codes();
                let violation_facts_json = error.violation_facts_json();
                tracing::warn!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace_response_preflight_rejected",
                    reason_code = error.reason_code(),
                    error_class = error.class(),
                    violation_codes = ?violation_codes,
                    violation_facts_json = violation_facts_json.as_deref().unwrap_or(""),
                    rejection_revision = ?error.current_revision(),
                    control_call_id = calls[control_index].call_id,
                    canonical_revision = ?canonical_revision,
                    zero_dispatch = true,
                    state_commit = false,
                    error = ?error,
                    "taskspace_response_preflight_rejected"
                );
                return Ok(taskspace_prepare_failure_outcome(
                    &calls,
                    canonical_revision,
                    &error,
                ));
            }
        };
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace_action_reservation_committed",
            control_call_id = calls[control_index].call_id,
            map_id = prepared.map_id,
            revision_before = prepared.revision_before,
            revision_after = prepared.revision_after,
            reservation_count = prepared.prepared_calls.len(),
            "committed TaskSpace action reservations"
        );
        if prepared.action == "reopen_map" {
            tracing::info!(
                target: "codex_core::taskspace",
                event_name = "taskspace_map_reopened",
                control_call_id = calls[control_index].call_id,
                map_id = prepared.map_id,
                revision_before = prepared.revision_before,
                revision_after = prepared.revision_after,
                reservation_count = prepared.prepared_calls.len(),
                "reopened canonical TaskSpace Map from Agent-declared user-feedback work"
            );
        }
        let control_call_id = calls[control_index].call_id.clone();
        let sibling_calls = calls
            .iter()
            .enumerate()
            .filter_map(|(index, call)| (index != control_index).then_some(call.clone()))
            .collect::<Vec<_>>();
        return execute_prepared_taskspace_siblings(
            runtime,
            sibling_calls,
            prepared,
            control_call_id,
            cancellation_token,
        )
        .await;
    }
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
                    tool_name = call.provider_tool_name_display(),
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
                    tool_name = call.provider_tool_name_display(),
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
                    tool_name = call.provider_tool_name_display(),
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

fn taskspace_prepare_failure_outcome(
    calls: &[ToolCall],
    canonical_revision: Option<u64>,
    error: &ActionMapResponsePrepareError,
) -> ToolSequenceOutcome {
    let failure_provenance =
        provider_response_failure_provenance(calls.iter().map(|call| call.call_id.as_str()));
    let payload = error.model_visible_failure(canonical_revision, failure_provenance);
    let (mut pairing, supplemental): (Vec<_>, Vec<_>) = calls
        .iter()
        .flat_map(|call| {
            ToolCallRuntime::provider_response_rejection_responses(call, payload.clone())
        })
        .partition(|response| !matches!(response, ResponseInputItem::Message { .. }));
    pairing.extend(supplemental);
    let factual_payload = serde_json::from_str(&payload)
        .expect("TaskSpace response failure is produced by serde_json");
    pairing.push(ToolCallRuntime::factual_message(factual_payload));
    ToolSequenceOutcome {
        outputs: pairing,
        terminal_completion: None,
    }
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
    let range = segment_range(segment);
    &calls[range]
}

fn segment_range(segment: &SequenceSegment) -> std::ops::Range<usize> {
    match *segment {
        SequenceSegment::Parallel { start, end } => start..end,
        SequenceSegment::Barrier { index, .. } => index..index + 1,
    }
}

fn barrier_kind(call: &ToolCall) -> Option<BarrierKind> {
    if call.provider_tool_name.namespace.is_some() {
        return None;
    }
    match call.provider_tool_name.name.as_str() {
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
        hasher.update(call.provider_tool_name_display().as_bytes());
        hasher.update([0xff]);
    }
    format!("{:x}", hasher.finalize())
}

async fn execute_prepared_taskspace_siblings(
    runtime: ToolCallRuntime,
    calls: Vec<ToolCall>,
    prepared: crate::action_map::ActionMapPreparedResponse,
    control_call_id: String,
    cancellation_token: CancellationToken,
) -> Result<ToolSequenceOutcome> {
    let prepared_calls = &prepared.prepared_calls;
    debug_assert_eq!(calls.len(), prepared_calls.len());
    debug_assert!(
        calls
            .iter()
            .zip(prepared_calls)
            .enumerate()
            .all(|(index, (call, prepared))| {
                prepared.call_index == index
                    && prepared.call_id == call.call_id
                    && prepared.tool_name == call.provider_tool_name_display()
            })
    );
    let bound_calls = prepared_calls;
    let segments = sequence_segments(&calls);
    let mut outputs = Vec::with_capacity(calls.len() + 1);
    let mut supplemental_outputs = Vec::new();
    let mut prior_failure: Option<String> = None;
    let mut terminal_completion: Option<TaskSpaceTerminalCompletion> = None;
    let prepared_action_count = bound_calls.len();
    let mut dispatched_action_count = 0usize;
    let mut skipped_action_count = 0usize;
    let mut closure_attempt_count = 0usize;

    for segment in segments {
        if let Some(prior_call_id) = prior_failure.as_deref() {
            for index in segment_range(&segment) {
                let call = &calls[index];
                let prepared = &bound_calls[index];
                append_pairing_and_supplemental(
                    &mut outputs,
                    &mut supplemental_outputs,
                    ToolCallRuntime::skipped_responses(call, prior_call_id),
                );
                skipped_action_count += 1;
                closure_attempt_count += 1;
                match runtime.record_taskspace_skipped_tool_result(prepared).await {
                    Ok(()) => {
                        tracing::info!(
                            target: "codex_core::taskspace",
                            event_name = "taskspace_prepared_tool_skipped_and_released",
                            call_id = prepared.call_id,
                            call_index = prepared.call_index,
                            tool_name = prepared.tool_name,
                            map_id = prepared.map_id,
                            node_id = prepared.node_id,
                            reservation_id = prepared.reservation_id,
                            prior_call_id,
                            tool_success = false,
                            state_commit = true,
                            "released skipped Agent-declared native tool action"
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: "codex_core::taskspace",
                            event_name = "taskspace_prepared_tool_skip_release_failed",
                            call_id = prepared.call_id,
                            call_index = prepared.call_index,
                            tool_name = prepared.tool_name,
                            map_id = prepared.map_id,
                            node_id = prepared.node_id,
                            reservation_id = prepared.reservation_id,
                            prior_call_id,
                            state_commit = false,
                            error = %error,
                            "failed to release skipped Agent-declared native tool action"
                        );
                        supplemental_outputs.push(
                            ToolCallRuntime::taskspace_bound_result_commit_failure_response(
                                prepared, error,
                            ),
                        );
                    }
                }
            }
            continue;
        }

        let segment_action_count = segment_range(&segment).len();
        let segment_executions = match segment {
            SequenceSegment::Parallel { start, end } => {
                let futures =
                    calls[start..end]
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(offset, call)| {
                            runtime
                                .clone()
                                .handle_taskspace_bound_tool_call_for_sequence(
                                    call,
                                    bound_calls[start + offset].clone(),
                                    cancellation_token.child_token(),
                                )
                        });
                join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            }
            SequenceSegment::Barrier { index, .. } => {
                let call = calls[index].clone();
                let prepared = bound_calls[index].clone();
                vec![
                    runtime
                        .clone()
                        .handle_taskspace_bound_tool_call_for_sequence(
                            call,
                            prepared,
                            cancellation_token.child_token(),
                        )
                        .await?,
                ]
            }
        };
        dispatched_action_count += segment_action_count;
        closure_attempt_count += segment_action_count;

        for execution in segment_executions {
            if prior_failure.is_none()
                && let Some(call_id) = execution_failure_call_id(&execution)
            {
                prior_failure = Some(call_id.to_string());
            }
            if let Some(carrier) = execution.taskspace_terminal_carrier.as_ref() {
                terminal_completion = Some(TaskSpaceTerminalCompletion {
                    call_id: response_input_call_id(&execution.response).to_string(),
                    carrier: carrier.clone(),
                });
            }
            outputs.push(execution.response);
            supplemental_outputs.extend(execution.supplemental_responses);
        }
    }

    let all_prepared_actions_accounted =
        dispatched_action_count + skipped_action_count == prepared_action_count;
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace_response_action_closure_audited",
        prepared_action_count,
        dispatched_action_count,
        skipped_action_count,
        closure_attempt_count,
        all_prepared_actions_accounted,
        failed = prior_failure.is_some(),
        "audited TaskSpace prepared action closure attempts"
    );
    outputs.extend(supplemental_outputs);
    let settlement = match runtime.taskspace_response_settlement(&prepared).await {
        Ok(settlement) => settlement,
        Err(error) => crate::action_map::ActionMapResponseSettlement::unavailable(&prepared, error),
    };
    emit_taskspace_response_finalized(&prepared, &settlement, &control_call_id);
    let control_output = ResponseInputItem::FunctionCallOutput {
        call_id: control_call_id,
        output: codex_protocol::models::FunctionCallOutputPayload {
            body: codex_protocol::models::FunctionCallOutputBody::Text(
                settlement.finalized_model_visible_result(&prepared),
            ),
            success: Some(settlement.complete()),
        },
    };
    outputs.insert(0, control_output);
    Ok(ToolSequenceOutcome {
        outputs,
        terminal_completion,
    })
}

fn emit_taskspace_response_finalized(
    prepared: &crate::action_map::ActionMapPreparedResponse,
    settlement: &crate::action_map::ActionMapResponseSettlement,
    control_call_id: &str,
) {
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace_response_finalized",
        control_call_id,
        map_id = prepared.map_id,
        prepare_revision = prepared.revision_after,
        canonical_revision = settlement.canonical_revision.unwrap_or_default(),
        canonical_revision_available = settlement.canonical_revision.is_some(),
        prepared_action_count = settlement.prepared_action_count,
        attributed_result_count = settlement.attributed_result_count,
        outstanding_reservation_count = settlement.outstanding_reservation_count,
        status = settlement.settlement_status(),
        reason_code = settlement.reason_code().unwrap_or("none"),
        settlement_complete = settlement.complete(),
        "finalized the canonical TaskSpace control result"
    );
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

#[cfg(test)]
#[path = "sequence_taskspace_tests.rs"]
mod taskspace_tests;

#[cfg(test)]
#[path = "sequence_taskspace_log_tests.rs"]
mod taskspace_log_tests;

#[cfg(test)]
#[path = "sequence_taskspace_rejection_tests.rs"]
mod taskspace_rejection_tests;

#[cfg(test)]
#[path = "sequence_identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
#[path = "sequence_ownership_tests.rs"]
mod ownership_tests;
