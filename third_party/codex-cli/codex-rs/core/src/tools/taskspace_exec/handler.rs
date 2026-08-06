use std::sync::Arc;

use codex_protocol::models::ResponseInputItem;
use futures::StreamExt;
use serde::Serialize;

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

use super::DispatchedClientCall;
use super::PreparedHostedBinding;
use super::TaskSpaceExecCatalog;
use super::TaskSpaceExecRequestContext;
use super::TaskSpaceExecResponseScope;
use super::dispatch_client_calls;
use super::preflight_taskspace_exec;
use super::prepare_client_calls;

pub(crate) struct TaskSpaceExecHandler {
    catalog: Arc<TaskSpaceExecCatalog>,
    client_router: Arc<ToolRouter>,
    response_scope: Arc<TaskSpaceExecResponseScope>,
}

#[derive(Serialize)]
struct ClientResult {
    call_index: usize,
    node_id: String,
    tool: String,
    outcome: &'static str,
    response: ResponseInputItem,
}

#[derive(Serialize)]
struct HostedResult {
    output_index: usize,
    provider_id: String,
    tool: String,
    outcome: &'static str,
    node_ids: Vec<String>,
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
        let hosted_facts = self
            .response_scope
            .claim_hosted_facts(&invocation.call_id)
            .map_err(taskspace_rejection)?;
        let (map_id, current_map) = read_current_map(invocation.session.as_ref()).await?;
        let request = TaskSpaceExecRequestContext::capture(
            map_id.clone(),
            current_map.as_ref(),
            Arc::clone(&self.catalog),
        )
        .map_err(|error| taskspace_rejection(format!("request context: {error:?}")))?;
        let envelope = request
            .decode_outer_call(invocation.call_id.clone(), arguments)
            .map_err(|error| taskspace_rejection(format!("invalid envelope: {error:?}")))?;
        let prepared = preflight_taskspace_exec(&envelope, current_map.as_ref(), &hosted_facts)
            .map_err(|error| taskspace_rejection(format!("preflight: {error:?}")))?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.preflight_accepted",
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            request_revision = ?envelope.request().request_revision(),
            client_call_count = prepared.client_calls.len(),
            hosted_binding_count = prepared.hosted_bindings.len(),
            read_count = prepared.read_maps.len(),
        );
        let native_calls =
            prepare_client_calls(invocation.session.as_ref(), &prepared.client_calls)
                .await
                .map_err(|error| taskspace_rejection(format!("client preparation: {error:?}")))?;

        let action_bindings = client_action_bindings(&prepared.client_calls)
            .into_iter()
            .chain(hosted_action_bindings(&prepared.hosted_bindings))
            .collect::<Vec<_>>();
        let candidate_map = attach_preflight_actions(prepared.candidate_map, &action_bindings)?;
        let candidate_revision = candidate_map.as_ref().map(|map| map.revision);
        persist_candidate(
            invocation.session.as_ref(),
            envelope.request(),
            &map_id,
            current_map.as_ref(),
            candidate_map.as_ref(),
        )
        .await?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.candidate_persisted",
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            candidate_revision = ?candidate_revision,
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
            native_calls,
            invocation.cancellation_token.clone(),
        );
        let mut client_results = Vec::new();
        let mut fatal_error = None;
        while let Some(result) = dispatched.next().await {
            let outcome = dispatched_outcome(&result);
            settle_client_action(invocation.session.as_ref(), &result, outcome).await?;
            match result.response {
                Ok(response) => client_results.push(ClientResult {
                    call_index: result.identity.index,
                    node_id: result.node_id,
                    tool: result.public_name,
                    outcome: outcome_name(outcome),
                    response,
                }),
                Err(error) => {
                    fatal_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = fatal_error {
            return Err(FunctionCallError::Fatal(error.to_string()));
        }
        client_results.sort_by_key(|result| result.call_index);
        let hosted_results = prepared
            .hosted_bindings
            .into_iter()
            .map(hosted_result)
            .collect::<Vec<_>>();
        let (_, settled_map) = read_current_map(invocation.session.as_ref()).await?;
        let reads = prepared
            .read_maps
            .into_iter()
            .map(|(call_index, map)| {
                serde_json::json!({
                    "call_index": call_index,
                    "map": map,
                })
            })
            .collect::<Vec<_>>();
        let all_succeeded = client_results
            .iter()
            .all(|result| result.outcome == "succeeded")
            && hosted_results
                .iter()
                .all(|result| result.outcome == "succeeded");
        let client_result_count = client_results.len();
        let hosted_result_count = hosted_results.len();
        let output = serde_json::json!({
            "status": "completed",
            "map": settled_map.as_ref().map(|map| serde_json::json!({
                "map_id": map.map_id,
                "revision": map.revision,
            })),
            "reads": reads,
            "client_results": client_results,
            "hosted_results": hosted_results,
        });
        let text = serde_json::to_string(&output).map_err(|error| {
            FunctionCallError::Fatal(format!(
                "taskspace_exec feedback serialization failed: {error}"
            ))
        })?;
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.completed",
            outer_call_id = %invocation.call_id,
            map_id = %map_id,
            client_result_count,
            hosted_result_count,
            success = all_succeeded,
        );
        Ok(FunctionToolOutput::from_text(text, Some(all_succeeded)))
    }
}

async fn read_current_map(
    session: &Session,
) -> Result<(String, Option<TaskSpaceMap>), FunctionCallError> {
    session
        .read_canonical_action_map("taskspace_exec_read", |runtime, _| {
            (
                runtime.active_map_id().map(str::to_string),
                runtime.canonical_map_for_store(),
            )
        })
        .await
        .map_err(taskspace_fatal)
        .and_then(|(map_id, map)| {
            map_id
                .map(|map_id| (map_id, map))
                .ok_or_else(|| taskspace_fatal("TaskSpace Map identity is unavailable"))
        })
}

fn client_action_bindings(calls: &[super::PreparedClientCall]) -> Vec<ActionBinding> {
    calls
        .iter()
        .map(|call| ActionBinding {
            action_id: call.identity.transport_id(),
            tool_name: call.call.public_name.clone(),
            outcome: ActionOutcome::Pending,
            node_ids: vec![call.call.node_id.clone()],
        })
        .collect()
}

fn hosted_action_bindings(
    bindings: &[PreparedHostedBinding],
) -> impl Iterator<Item = ActionBinding> + '_ {
    bindings.iter().map(|binding| ActionBinding {
        action_id: binding.provider_id.clone(),
        tool_name: binding.tool.clone(),
        outcome: binding.outcome,
        node_ids: binding.node_ids.clone(),
    })
}

fn attach_preflight_actions(
    candidate: Option<TaskSpaceMap>,
    bindings: &[ActionBinding],
) -> Result<Option<TaskSpaceMap>, FunctionCallError> {
    if bindings.is_empty() {
        return Ok(candidate);
    }
    let candidate =
        candidate.ok_or_else(|| taskspace_rejection("actions require an initialized Map"))?;
    rooted_dag::attach_actions(&candidate, bindings)
        .map(|commit| Some(commit.map))
        .map_err(|error| taskspace_rejection(format!("action attribution: {error:?}")))
}

async fn persist_candidate(
    session: &Session,
    request: &TaskSpaceExecRequestContext,
    map_id: &str,
    before: Option<&TaskSpaceMap>,
    candidate: Option<&TaskSpaceMap>,
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
                .and_then(|()| runtime.restore_store_map(&map_id, owner, candidate));
            (result, Vec::new())
        })
        .await
        .map_err(taskspace_fatal)?;
    result.map_err(taskspace_rejection)
}

async fn settle_client_action(
    session: &Session,
    result: &DispatchedClientCall,
    outcome: ActionOutcome,
) -> Result<(), FunctionCallError> {
    let action_id = result.identity.transport_id();
    let tool_name = result.public_name.clone();
    let log_action_id = action_id.clone();
    let log_tool_name = tool_name.clone();
    let (settled, _) = session
        .mutate_canonical_action_map("taskspace_exec_settle", move |runtime, owner| {
            let current = runtime.canonical_map_for_store();
            let settled = current
                .as_ref()
                .ok_or_else(|| "canonical Map disappeared during Tool settlement".to_string())
                .and_then(|map| {
                    rooted_dag::settle_action(map, &action_id, &tool_name, outcome)
                        .map(|commit| commit.map)
                        .map_err(|error| format!("action settlement rejected: {error:?}"))
                })
                .and_then(|candidate| {
                    let map_id = candidate.map_id.clone();
                    runtime.restore_store_map(&map_id, owner, Some(candidate))
                });
            (settled, Vec::new())
        })
        .await
        .map_err(taskspace_fatal)?;
    settled.map_err(taskspace_fatal)?;
    tracing::info!(
        target: "codex_core::taskspace_exec",
        event_name = "taskspace.exec.action_settled",
        action_id = %log_action_id,
        tool = %log_tool_name,
        outcome = outcome_name(outcome),
    );
    Ok(())
}

fn dispatched_outcome(result: &DispatchedClientCall) -> ActionOutcome {
    if result.cancelled {
        return ActionOutcome::Cancelled;
    }
    match &result.response {
        Err(_) => ActionOutcome::Failed,
        Ok(ResponseInputItem::FunctionCallOutput { output, .. })
        | Ok(ResponseInputItem::CustomToolCallOutput { output, .. })
            if output.success == Some(false) =>
        {
            ActionOutcome::Failed
        }
        Ok(_) => ActionOutcome::Succeeded,
    }
}

fn hosted_result(binding: PreparedHostedBinding) -> HostedResult {
    HostedResult {
        output_index: binding.output_index,
        provider_id: binding.provider_id,
        tool: binding.tool,
        outcome: outcome_name(binding.outcome),
        node_ids: binding.node_ids,
    }
}

fn outcome_name(outcome: ActionOutcome) -> &'static str {
    match outcome {
        ActionOutcome::Pending => "pending",
        ActionOutcome::Succeeded => "succeeded",
        ActionOutcome::Failed => "failed",
        ActionOutcome::Cancelled => "cancelled",
    }
}

fn taskspace_rejection(message: impl Into<String>) -> FunctionCallError {
    let message = message.into();
    tracing::warn!(
        target: "codex_core::taskspace_exec",
        event_name = "taskspace.exec.rejected",
        reason = %message,
    );
    FunctionCallError::RespondToModel(format!("taskspace_exec rejected: {message}"))
}

fn taskspace_fatal(message: impl Into<String>) -> FunctionCallError {
    let message = message.into();
    tracing::error!(
        target: "codex_core::taskspace_exec",
        event_name = "taskspace.exec.fatal",
        reason = %message,
    );
    FunctionCallError::Fatal(message)
}
