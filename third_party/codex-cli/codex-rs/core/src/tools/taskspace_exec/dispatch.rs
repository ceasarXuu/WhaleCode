use codex_protocol::error::CodexErr;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use tokio_util::sync::CancellationToken;

use crate::function_tool::FunctionCallError;
use crate::session::TaskSpaceActionSettlementFact;
use crate::session::session::Session;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;

use super::ClientCallInput;
use super::PreparedClientCall;
use super::TaskSpaceExecInternalCallId;
use super::catalog::TaskSpaceClientTransport;

#[derive(Clone, Debug)]
pub(crate) struct NativeClientCall {
    pub(crate) identity: TaskSpaceExecInternalCallId,
    pub(crate) node_id: String,
    pub(crate) display_name: String,
    pub(super) call: ToolCall,
}

#[derive(Debug)]
pub(crate) struct DispatchedClientCall {
    pub(crate) identity: TaskSpaceExecInternalCallId,
    pub(crate) node_id: String,
    pub(crate) display_name: String,
    pub(crate) response: Result<ResponseInputItem, CodexErr>,
    pub(crate) cancelled: bool,
    pub(crate) execution_failed: bool,
    pub(crate) settlement_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecDispatchPrepareError {
    InvalidArguments {
        identity: TaskSpaceExecInternalCallId,
        tool: String,
        reason: String,
    },
    NativeCallRejected {
        identity: TaskSpaceExecInternalCallId,
        tool: String,
        reason: String,
    },
    NativeCallMissing {
        identity: TaskSpaceExecInternalCallId,
        tool: String,
    },
}

pub(crate) async fn prepare_client_calls(
    session: &Session,
    calls: &[PreparedClientCall],
) -> Result<Vec<NativeClientCall>, TaskSpaceExecDispatchPrepareError> {
    let mut prepared = Vec::with_capacity(calls.len());
    for item in calls {
        let response_item = native_response_item(item)?;
        let call = ToolRouter::build_tool_call(session, response_item)
            .await
            .map_err(|error| native_call_rejected(item, error))?
            .ok_or_else(|| TaskSpaceExecDispatchPrepareError::NativeCallMissing {
                identity: item.identity.clone(),
                tool: item.call.display_name.clone(),
            })?;
        tracing::debug!(
            event = "taskspace_exec_client_prepared",
            outer_call_id = item.identity.outer_call_id,
            call_index = item.identity.index,
            node_id = item.call.node_id,
            tool = item.call.display_name,
        );
        prepared.push(NativeClientCall {
            identity: item.identity.clone(),
            node_id: item.call.node_id.clone(),
            display_name: item.call.display_name.clone(),
            call,
        });
    }
    Ok(prepared)
}

pub(crate) fn dispatch_client_calls(
    runtime: ToolCallRuntime,
    session: std::sync::Arc<Session>,
    map_id: String,
    calls: Vec<NativeClientCall>,
    cancellation_token: CancellationToken,
) -> FuturesUnordered<BoxFuture<'static, DispatchedClientCall>> {
    calls
        .into_iter()
        .map(|item| {
            let runtime = runtime.clone();
            let producer_session = std::sync::Arc::clone(&session);
            let map_id = map_id.clone();
            let cancellation_token = cancellation_token.clone();
            let identity = item.identity.clone();
            let node_id = item.node_id.clone();
            let display_name = item.display_name.clone();
            let producer = async move {
                tracing::debug!(
                    event = "taskspace_exec_client_dispatch_started",
                    outer_call_id = item.identity.outer_call_id,
                    call_index = item.identity.index,
                    node_id = item.node_id,
                    tool = item.display_name,
                );
                let handled = runtime
                    .handle_tool_call_with_status(item.call, cancellation_token)
                    .await;
                let mut result = DispatchedClientCall {
                    identity: item.identity,
                    node_id: item.node_id,
                    display_name: item.display_name,
                    response: handled.response,
                    cancelled: handled.cancelled,
                    execution_failed: handled.execution_failed,
                    settlement_error: None,
                };
                let outcome = dispatched_outcome(&result);
                result.settlement_error = producer_session
                    .enqueue_taskspace_action_settlement(TaskSpaceActionSettlementFact {
                        map_id,
                        outer_call_id: result.identity.outer_call_id.clone(),
                        action_id: result.identity.transport_id(),
                        node_ids: vec![result.node_id.clone()],
                        tool_name: result.display_name.clone(),
                        outcome,
                    })
                    .err();
                tracing::debug!(
                    event = "taskspace_exec_client_dispatch_finished",
                    outer_call_id = result.identity.outer_call_id,
                    call_index = result.identity.index,
                    node_id = result.node_id,
                    tool = result.display_name,
                    fatal = result.response.is_err(),
                    cancelled = result.cancelled,
                    execution_failed = result.execution_failed,
                    settlement_queued = result.settlement_error.is_none(),
                );
                result
            };
            let handle = session.spawn_taskspace_action_producer(producer);
            Box::pin(async move {
                match handle {
                    Ok(handle) => handle.await.unwrap_or_else(|error| DispatchedClientCall {
                        identity,
                        node_id,
                        display_name,
                        response: Err(CodexErr::Fatal(format!(
                            "TaskSpace Action producer failed: {error}"
                        ))),
                        cancelled: false,
                        execution_failed: true,
                        settlement_error: Some(
                            "TaskSpace Action producer stopped before settlement.".to_string(),
                        ),
                    }),
                    Err(error) => DispatchedClientCall {
                        identity,
                        node_id,
                        display_name,
                        response: Err(CodexErr::Fatal(error.clone())),
                        cancelled: false,
                        execution_failed: true,
                        settlement_error: Some(error),
                    },
                }
            }) as BoxFuture<'static, DispatchedClientCall>
        })
        .collect()
}

pub(crate) fn dispatched_outcome(
    result: &DispatchedClientCall,
) -> codex_protocol::taskspace::TaskSpaceActionOutcome {
    use codex_protocol::taskspace::TaskSpaceActionOutcome as Outcome;

    if result.cancelled {
        return Outcome::Cancelled;
    }
    if result.execution_failed {
        return Outcome::Failed;
    }
    match &result.response {
        Err(_) => Outcome::Failed,
        Ok(ResponseInputItem::FunctionCallOutput { output, .. })
        | Ok(ResponseInputItem::CustomToolCallOutput { output, .. })
            if output.success == Some(false) =>
        {
            Outcome::Failed
        }
        Ok(_) => Outcome::Succeeded,
    }
}

fn native_response_item(
    prepared: &PreparedClientCall,
) -> Result<ResponseItem, TaskSpaceExecDispatchPrepareError> {
    let call_id = prepared.identity.transport_id();
    let tool_name = &prepared.call.tool_name;
    match (&prepared.call.transport, &prepared.call.input) {
        (TaskSpaceClientTransport::Function, ClientCallInput::Function(arguments)) => {
            let arguments = serialize_arguments(prepared, arguments)?;
            Ok(ResponseItem::FunctionCall {
                id: None,
                name: tool_name.name.clone(),
                namespace: tool_name.namespace.clone(),
                arguments,
                call_id,
            })
        }
        (TaskSpaceClientTransport::Freeform, ClientCallInput::Freeform(input)) => {
            Ok(ResponseItem::CustomToolCall {
                id: None,
                status: None,
                call_id,
                name: tool_name.name.clone(),
                input: input.clone(),
            })
        }
        (TaskSpaceClientTransport::ToolSearch, ClientCallInput::Function(arguments)) => {
            Ok(ResponseItem::ToolSearchCall {
                id: None,
                call_id: Some(call_id),
                status: None,
                execution: "client".to_string(),
                arguments: arguments.clone(),
            })
        }
        _ => Err(TaskSpaceExecDispatchPrepareError::InvalidArguments {
            identity: prepared.identity.clone(),
            tool: prepared.call.display_name.clone(),
            reason: "client transport and input kind do not match".to_string(),
        }),
    }
}

fn serialize_arguments(
    prepared: &PreparedClientCall,
    arguments: &serde_json::Value,
) -> Result<String, TaskSpaceExecDispatchPrepareError> {
    serde_json::to_string(arguments).map_err(|error| {
        TaskSpaceExecDispatchPrepareError::InvalidArguments {
            identity: prepared.identity.clone(),
            tool: prepared.call.display_name.clone(),
            reason: error.to_string(),
        }
    })
}

fn native_call_rejected(
    prepared: &PreparedClientCall,
    error: FunctionCallError,
) -> TaskSpaceExecDispatchPrepareError {
    TaskSpaceExecDispatchPrepareError::NativeCallRejected {
        identity: prepared.identity.clone(),
        tool: prepared.call.display_name.clone(),
        reason: error.to_string(),
    }
}
