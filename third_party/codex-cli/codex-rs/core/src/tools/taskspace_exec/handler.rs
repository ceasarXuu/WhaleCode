use std::sync::Arc;

use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::ActionBinding;
use crate::action_map::rooted_dag::ActionOutcome;
use crate::action_map::rooted_dag::TaskSpaceMap;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::router::ToolRouter;
use futures::StreamExt;

use super::TaskSpaceExecCatalog;
use super::TaskSpaceExecEnvelopeError;
use super::TaskSpaceExecPlanDecodeError;
use super::TaskSpaceExecRequestContext;
use super::TaskSpaceExecResponseScope;
use super::dispatch::dispatched_outcome;
use super::dispatch_client_calls;
use super::preflight::TaskSpaceExecPreflightError;
use super::preflight_taskspace_exec;
use super::prepare_client_calls;
use super::result::ClientResult;
use super::result::MapReadResult;
use super::result::TaskSpaceExecResult;

pub(crate) struct TaskSpaceExecHandler {
    catalog: Arc<TaskSpaceExecCatalog>,
    client_router: Arc<ToolRouter>,
    response_scope: Arc<TaskSpaceExecResponseScope>,
}

impl TaskSpaceExecHandler {
    pub(crate) fn new(
        catalog: Arc<TaskSpaceExecCatalog>,
        client_router: Arc<ToolRouter>,
        response_scope: Arc<TaskSpaceExecResponseScope>,
    ) -> Self {
        Self {
            catalog,
            client_router,
            response_scope,
        }
    }
}

impl ToolHandler for TaskSpaceExecHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn is_mutating(&self, _invocation: &ToolInvocation) -> bool {
        true
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolPayload::Function { arguments } = &invocation.payload else {
            return Err(FunctionCallError::Fatal(
                "taskspace_exec received a non-function payload".to_string(),
            ));
        };
        let claim = self
            .response_scope
            .claim_response(&invocation.call_id)
            .map_err(|error| {
                taskspace_rejection("response_claim_rejected", Some(&invocation.call_id), error)
            })?;
        if claim.request.capability_identity.as_ref() != self.catalog.capability_identity() {
            return Err(taskspace_rejection(
                "capability_identity_mismatch",
                Some(&invocation.call_id),
                "TaskSpace request capability identity does not match the dispatch catalog",
            ));
        }
        let response = &claim.response;
        let (map_id, current_map) =
            read_current_map(invocation.session.as_ref(), &invocation.call_id).await?;
        let request = TaskSpaceExecRequestContext::from_request_snapshot(
            claim.request.map_id,
            claim.request.revision,
            Arc::clone(&self.catalog),
        )
        .map_err(|error| {
            taskspace_rejection(
                "request_context_rejected",
                Some(&invocation.call_id),
                format!("request context: {error:?}"),
            )
        })?;
        let envelope = request
            .decode_outer_call(invocation.call_id.clone(), arguments)
            .map_err(|error| {
                taskspace_rejection(
                    "envelope_rejected",
                    Some(&invocation.call_id),
                    render_envelope_rejection(&error),
                )
            })?;
        let prepared =
            preflight_taskspace_exec(&envelope, current_map.as_ref()).map_err(|error| {
                taskspace_rejection(
                    "preflight_rejected",
                    Some(&invocation.call_id),
                    render_preflight_rejection(&error),
                )
            })?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.preflight_accepted",
            provider_request_id = response.provider_request_id.as_deref().unwrap_or(""),
            provider_logical_request_id = response.provider_logical_request_id.as_deref().unwrap_or(""),
            provider_attempt_seq = ?response.provider_attempt_seq,
            provider_response_id = %response.provider_response_id,
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            request_revision = ?envelope.request().request_revision(),
            capability_identity = self.catalog.capability_identity(),
            client_call_count = prepared.client_calls.len(),
            read_count = prepared.read_maps.len(),
        );
        let native_calls =
            prepare_client_calls(invocation.session.as_ref(), &prepared.client_calls)
                .await
                .map_err(|error| {
                    taskspace_rejection(
                        "client_preparation_rejected",
                        Some(&invocation.call_id),
                        format!("client preparation: {error:?}"),
                    )
                })?;

        let action_bindings = client_action_bindings(&prepared.client_calls);
        let candidate_map = attach_preflight_actions(prepared.candidate_map, &action_bindings)
            .map_err(|error| {
                taskspace_rejection(
                    "action_attribution_rejected",
                    Some(&invocation.call_id),
                    error,
                )
            })?;
        let candidate_revision = candidate_map.as_ref().map(|map| map.revision);
        persist_candidate(
            invocation.session.as_ref(),
            envelope.request(),
            &map_id,
            current_map.as_ref(),
            candidate_map.as_ref(),
            &invocation.call_id,
        )
        .await?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.candidate_persisted",
            provider_request_id = response.provider_request_id.as_deref().unwrap_or(""),
            provider_logical_request_id = response.provider_logical_request_id.as_deref().unwrap_or(""),
            provider_attempt_seq = ?response.provider_attempt_seq,
            provider_response_id = %response.provider_response_id,
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            candidate_revision = ?candidate_revision,
            capability_identity = self.catalog.capability_identity(),
            action_count = action_bindings.len(),
        );
        let client_runtime = ToolCallRuntime::new(
            Arc::clone(&self.client_router),
            Arc::clone(&invocation.session),
            Arc::clone(&invocation.turn),
            Arc::clone(&invocation.tracker),
        );
        let mut dispatched = dispatch_client_calls(
            client_runtime,
            Arc::clone(&invocation.session),
            map_id.clone(),
            native_calls,
            invocation.cancellation_token.clone(),
        );
        let mut client_results = Vec::new();
        while let Some(result) = dispatched.next().await {
            let outcome = dispatched_outcome(&result);
            let settlement_error = result.settlement_error.clone();
            if let Some(error) = settlement_error.as_ref() {
                tracing::error!(
                    target: "codex_core::taskspace_exec",
                    event_name = "taskspace.exec.action_settlement_failed",
                    provider_request_id = response.provider_request_id.as_deref().unwrap_or(""),
                    provider_response_id = %response.provider_response_id,
                    outer_call_id = %invocation.call_id,
                    action_id = %result.identity.transport_id(),
                    node_id = %result.node_id,
                    tool = %result.display_name,
                    outcome = outcome_name(outcome),
                    capability_identity = self.catalog.capability_identity(),
                    error = %error,
                );
            }
            client_results.push(ClientResult {
                call_index: result.identity.index,
                action_id: result.identity.transport_id(),
                node_id: result.node_id,
                tool: result.display_name,
                outcome: outcome_name(outcome),
                result: result.result,
                error: result.error,
                settlement_error,
            });
        }
        client_results.sort_by_key(|result| result.call_index);
        let reads = prepared
            .read_maps
            .into_iter()
            .map(|(call_index, map)| MapReadResult { call_index, map })
            .collect::<Vec<_>>();
        let all_succeeded = client_results
            .iter()
            .all(|result| result.outcome == "succeeded" && result.settlement_error.is_none());
        let client_result_count = client_results.len();
        let output = TaskSpaceExecResult::new(
            invocation.call_id.clone(),
            map_id.clone(),
            candidate_revision,
            reads,
            client_results,
        );
        let text = serde_json::to_string(&output).map_err(|error| {
            FunctionCallError::Fatal(format!(
                "taskspace_exec feedback serialization failed: {error}"
            ))
        })?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.completed",
            provider_request_id = response.provider_request_id.as_deref().unwrap_or(""),
            provider_logical_request_id = response.provider_logical_request_id.as_deref().unwrap_or(""),
            provider_attempt_seq = ?response.provider_attempt_seq,
            provider_response_id = %response.provider_response_id,
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            map_revision = ?candidate_revision,
            capability_identity = self.catalog.capability_identity(),
            client_result_count,
            success = all_succeeded,
        );
        Ok(FunctionToolOutput::from_text(text, Some(all_succeeded)))
    }
}

pub(super) async fn read_current_map(
    session: &Session,
    outer_call_id: &str,
) -> Result<(String, Option<TaskSpaceMap>), FunctionCallError> {
    session
        .read_canonical_action_map("taskspace_exec_read", |runtime, _| {
            (
                runtime.active_map_id().map(str::to_string),
                runtime.canonical_map_for_store(),
            )
        })
        .await
        .map_err(|error| taskspace_fatal("map_read_failed", Some(outer_call_id), error))
        .and_then(|(map_id, map)| {
            map_id.map(|map_id| (map_id, map)).ok_or_else(|| {
                taskspace_fatal(
                    "map_identity_missing",
                    Some(outer_call_id),
                    "TaskSpace Map identity is unavailable",
                )
            })
        })
}

fn client_action_bindings(calls: &[super::PreparedClientCall]) -> Vec<ActionBinding> {
    calls
        .iter()
        .map(|call| ActionBinding {
            action_id: call.identity.transport_id(),
            tool_name: call.call.display_name.clone(),
            outcome: ActionOutcome::Pending,
            node_ids: vec![call.call.node_id.clone()],
        })
        .collect()
}

fn attach_preflight_actions(
    candidate: Option<TaskSpaceMap>,
    bindings: &[ActionBinding],
) -> Result<Option<TaskSpaceMap>, String> {
    if bindings.is_empty() {
        return Ok(candidate);
    }
    let candidate = candidate.ok_or_else(|| "actions require an initialized Map".to_string())?;
    rooted_dag::attach_actions(&candidate, bindings)
        .map(|commit| Some(commit.map))
        .map_err(|error| format!("action attribution: {error:?}"))
}

async fn persist_candidate(
    session: &Session,
    request: &TaskSpaceExecRequestContext,
    map_id: &str,
    before: Option<&TaskSpaceMap>,
    candidate: Option<&TaskSpaceMap>,
    outer_call_id: &str,
) -> Result<(), FunctionCallError> {
    if before == candidate {
        return Ok(());
    }
    let request = request.clone();
    let map_id = map_id.to_string();
    let candidate = candidate.cloned();
    let (result, _) = session
        .mutate_canonical_action_map("taskspace_exec_prepare", move |runtime, owner| {
            let current = runtime.canonical_map_for_store();
            let result = request
                .validate_current_map(current.as_ref())
                .map_err(|error| format!("stale request context: {error:?}"))
                .and_then(|()| runtime.restore_store_map(&map_id, owner, candidate.clone()));
            (result, Vec::new())
        })
        .await
        .map_err(|error| taskspace_fatal("map_persist_failed", Some(outer_call_id), error))?;
    result.map_err(|error| taskspace_rejection("map_persist_rejected", Some(outer_call_id), error))
}

fn outcome_name(outcome: ActionOutcome) -> &'static str {
    match outcome {
        ActionOutcome::Pending => "pending",
        ActionOutcome::Succeeded => "succeeded",
        ActionOutcome::Failed => "failed",
        ActionOutcome::Cancelled => "cancelled",
    }
}

fn taskspace_rejection(
    reason_code: &'static str,
    outer_call_id: Option<&str>,
    message: impl Into<String>,
) -> FunctionCallError {
    let message = message.into();
    tracing::warn!(
        target: "codex_core::taskspace_exec",
        event_name = "taskspace.exec.rejected",
        reason_code,
        outer_call_id = outer_call_id.unwrap_or(""),
        reason = %message,
    );
    FunctionCallError::RespondToModel(format!("taskspace_exec rejected: {message}"))
}

fn render_envelope_rejection(error: &TaskSpaceExecEnvelopeError) -> String {
    const NOTHING_EXECUTED: &str = "No Map or Tool actions were executed.";
    match error {
        TaskSpaceExecEnvelopeError::PlanDecode(TaskSpaceExecPlanDecodeError::InvalidJson(
            detail,
        )) => format!("invalid JSON syntax: {detail}. {NOTHING_EXECUTED}"),
        TaskSpaceExecEnvelopeError::PlanDecode(
            TaskSpaceExecPlanDecodeError::UnexpectedArgumentsField,
        ) => format!(
            "invalid top-level contract: unexpected field `arguments`. Submit exactly one declared TaskSpace sequence directly; do not wrap it in an `arguments` field. {NOTHING_EXECUTED}"
        ),
        TaskSpaceExecEnvelopeError::PlanDecode(TaskSpaceExecPlanDecodeError::InvalidEnvelope(
            detail,
        )) => format!("invalid top-level contract: {detail}. {NOTHING_EXECUTED}"),
        _ => format!("invalid envelope: {error:?}. {NOTHING_EXECUTED}"),
    }
}

pub(super) fn render_preflight_rejection(error: &TaskSpaceExecPreflightError) -> String {
    match error {
        TaskSpaceExecPreflightError::ClientNodeNotExecutable {
            index,
            node_id,
            state,
            incomplete_parent_ids,
        } => format!(
            "Tool action {index} targeted work node `{node_id}` in state `{}`; incomplete direct parent nodes: {incomplete_parent_ids:?}. Only the sequence's preceding Map operation can unlock work; Tool outcomes do not change node state. No Map or Tool actions were executed.",
            node_state_label(*state)
        ),
        _ => format!("preflight: {error:?}. No Map or Tool actions were executed."),
    }
}

fn node_state_label(state: rooted_dag::NodeState) -> &'static str {
    match state {
        rooted_dag::NodeState::Waiting => "waiting",
        rooted_dag::NodeState::Ready => "ready",
        rooted_dag::NodeState::InFlight => "in_flight",
        rooted_dag::NodeState::Completed => "completed",
    }
}

fn taskspace_fatal(
    reason_code: &'static str,
    outer_call_id: Option<&str>,
    message: impl Into<String>,
) -> FunctionCallError {
    let message = message.into();
    tracing::error!(
        target: "codex_core::taskspace_exec",
        event_name = "taskspace.exec.fatal",
        reason_code,
        outer_call_id = outer_call_id.unwrap_or(""),
        reason = %message,
    );
    FunctionCallError::Fatal(message)
}
