use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use tracing::instrument;
use tracing::trace_span;

use crate::action_map::ActionMapPreparedCall;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::AbortedToolOutput;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::TaskSpaceTerminalCarrier;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::taskspace_control_args::TaskSpaceControlArgs;
use crate::tools::registry::AnyToolResult;
use crate::tools::registry::ToolArgumentDiffConsumer;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolCallSource;
use crate::tools::router::ToolRouter;
use crate::tools::sequence_preflight::TaskSpaceDeclaredCall;
use codex_protocol::error::CodexErr;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_tools::ToolSpec;

#[derive(Clone)]
pub(crate) struct ToolCallRuntime {
    router: Arc<ToolRouter>,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tracker: SharedTurnDiffTracker,
    parallel_execution: Arc<RwLock<()>>,
}

pub(crate) struct ToolCallExecution {
    pub(crate) response: ResponseInputItem,
    pub(crate) supplemental_responses: Vec<ResponseInputItem>,
    pub(crate) succeeded: bool,
    pub(crate) taskspace_terminal_carrier: Option<TaskSpaceTerminalCarrier>,
}

impl ToolCallRuntime {
    pub(crate) fn new(
        router: Arc<ToolRouter>,
        session: Arc<Session>,
        turn_context: Arc<TurnContext>,
        tracker: SharedTurnDiffTracker,
    ) -> Self {
        Self {
            router,
            session,
            turn_context,
            tracker,
            parallel_execution: Arc::new(RwLock::new(())),
        }
    }

    pub(crate) fn find_spec(&self, tool_name: &codex_tools::ToolName) -> Option<ToolSpec> {
        self.router.find_spec(tool_name)
    }

    pub(crate) fn create_diff_consumer(
        &self,
        tool_name: &codex_tools::ToolName,
    ) -> Option<Box<dyn ToolArgumentDiffConsumer>> {
        self.router.create_diff_consumer(tool_name)
    }

    pub(crate) async fn taskspace_canonical_revision(&self) -> Option<u64> {
        self.session
            .action_map_control_state(None)
            .await
            .map(|state| state.revision)
    }

    pub(crate) async fn taskspace_active(&self) -> bool {
        self.session.taskspace_active().await
    }

    pub(crate) async fn prepare_taskspace_response(
        &self,
        control_call_id: &str,
        args: TaskSpaceControlArgs,
        declared_calls: Vec<TaskSpaceDeclaredCall>,
    ) -> Result<crate::action_map::ActionMapPreparedResponse, String> {
        self.session
            .prepare_taskspace_response(&self.turn_context, control_call_id, args, declared_calls)
            .await
    }

    pub(crate) fn invalid_call_responses(
        call: &ToolCall,
        message: impl Into<String>,
    ) -> Vec<ResponseInputItem> {
        let error = FunctionCallError::RespondToModel(message.into());
        let mut responses = vec![Self::failure_response_for_error(call, &error)];
        if let Some(supplemental) = Self::supplemental_failure_response(call, &error) {
            responses.push(supplemental);
        }
        responses
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn handle_tool_call_for_sequence(
        self,
        call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ToolCallExecution, CodexErr>> {
        self.handle_native_tool_call_for_sequence(call, cancellation_token)
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn handle_taskspace_bound_tool_call_for_sequence(
        self,
        call: ToolCall,
        prepared_call: ActionMapPreparedCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ToolCallExecution, CodexErr>> {
        async move {
            tracing::info!(
                target: "codex_core::taskspace",
                event_name = "taskspace_native_tool_dispatched",
                call_id = prepared_call.call_id,
                call_index = prepared_call.call_index,
                tool_name = prepared_call.tool_name,
                map_id = prepared_call.map_id,
                revision = prepared_call.revision,
                node_id = prepared_call.node_id,
                reservation_id = prepared_call.reservation_id,
                "dispatched Agent-declared native tool action"
            );
            let mut execution = self
                .clone()
                .handle_native_tool_call_for_sequence(call, cancellation_token)
                .await?;
            match self
                .record_taskspace_bound_tool_result(
                    &prepared_call,
                    execution.succeeded,
                    &execution.response,
                )
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        target: "codex_core::taskspace",
                        event_name = "taskspace_native_tool_result_attributed",
                        call_id = prepared_call.call_id,
                        call_index = prepared_call.call_index,
                        tool_name = prepared_call.tool_name,
                        map_id = prepared_call.map_id,
                        node_id = prepared_call.node_id,
                        reservation_id = prepared_call.reservation_id,
                        tool_success = execution.succeeded,
                        state_commit = true,
                        "attributed native tool result to Agent-declared map node"
                    );
                }
                Err(error) => {
                    Self::apply_failed_bound_result_commit(&mut execution, &prepared_call, error);
                }
            }
            Ok(execution)
        }
        .in_current_span()
    }

    fn handle_native_tool_call_for_sequence(
        self,
        call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ToolCallExecution, CodexErr>> {
        async move {
            let error_call = call.clone();
            let future = self.clone().handle_tool_call_with_source(
                call,
                ToolCallSource::Direct,
                cancellation_token,
            );
            match future.await {
                Ok(response) => {
                    let taskspace_terminal_carrier = response.taskspace_terminal_carrier().cloned();
                    let succeeded = response.result.success_for_logging();
                    let response = response.into_response();
                    Ok(ToolCallExecution {
                        response,
                        supplemental_responses: Vec::new(),
                        succeeded,
                        taskspace_terminal_carrier,
                    })
                }
                Err(FunctionCallError::Fatal(message)) => Err(CodexErr::Fatal(message)),
                Err(other) => {
                    let supplemental_responses =
                        Self::supplemental_failure_response(&error_call, &other)
                            .into_iter()
                            .collect::<Vec<_>>();
                    let response = Self::failure_response(error_call, other);
                    Ok(ToolCallExecution {
                        response,
                        supplemental_responses,
                        succeeded: false,
                        taskspace_terminal_carrier: None,
                    })
                }
            }
        }
        .in_current_span()
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn handle_tool_call_with_source(
        self,
        call: ToolCall,
        source: ToolCallSource,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<AnyToolResult, FunctionCallError>> {
        let supports_parallel = self.router.tool_supports_parallel(&call);
        let router = Arc::clone(&self.router);
        let session = Arc::clone(&self.session);
        let turn = Arc::clone(&self.turn_context);
        let tracker = Arc::clone(&self.tracker);
        let lock = Arc::clone(&self.parallel_execution);
        let invocation_cancellation_token = cancellation_token.clone();
        let started = Instant::now();
        let display_name = call.tool_name.display();

        let dispatch_span = trace_span!(
            "dispatch_tool_call_with_code_mode_result",
            otel.name = display_name.as_str(),
            tool_name = display_name.as_str(),
            call_id = call.call_id.as_str(),
            aborted = false,
        );

        let handle: AbortOnDropHandle<Result<AnyToolResult, FunctionCallError>> =
            AbortOnDropHandle::new(tokio::spawn(async move {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        let secs = started.elapsed().as_secs_f32().max(0.1);
                        dispatch_span.record("aborted", true);
                        let response = Self::aborted_response(&call, secs);
                        Ok(response)
                    },
                    res = async {
                        let _guard = if supports_parallel {
                            Either::Left(lock.read().await)
                        } else {
                            Either::Right(lock.write().await)
                        };

                        let result = router
                            .dispatch_tool_call_with_code_mode_result(
                                Arc::clone(&session),
                                Arc::clone(&turn),
                                invocation_cancellation_token,
                                tracker,
                                call.clone(),
                                source,
                            )
                            .instrument(dispatch_span.clone())
                            .await;
                        result
                    } => res,
                }
            }));

        async move {
            handle.await.map_err(|err| {
                FunctionCallError::Fatal(format!("tool task failed to receive: {err:?}"))
            })?
        }
        .in_current_span()
    }
}

impl ToolCallRuntime {
    fn failure_response(call: ToolCall, err: FunctionCallError) -> ResponseInputItem {
        Self::failure_response_for_error(&call, &err)
    }

    fn failure_response_for_error(call: &ToolCall, err: &FunctionCallError) -> ResponseInputItem {
        let message = function_call_error_model_visible_message(err);
        match &call.payload {
            ToolPayload::ToolSearch { .. } => ResponseInputItem::ToolSearchOutput {
                call_id: call.call_id.clone(),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: Vec::new(),
            },
            ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
                call_id: call.call_id.clone(),
                name: None,
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            },
            _ => ResponseInputItem::FunctionCallOutput {
                call_id: call.call_id.clone(),
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            },
        }
    }

    fn supplemental_failure_response(
        call: &ToolCall,
        err: &FunctionCallError,
    ) -> Option<ResponseInputItem> {
        matches!(call.payload, ToolPayload::ToolSearch { .. }).then(|| {
            let message = function_call_error_model_visible_message(err);
            Self::factual_message(serde_json::json!({
                "schema_version": "ToolSearchFailureV1",
                "status": "failed",
                "success": false,
                "call_id": call.call_id,
                "tool": call.tool_name.display(),
                "error": {
                    "class": "tool",
                    "message": message,
                },
            }))
        })
    }

    pub(crate) fn skipped_responses(
        call: &ToolCall,
        prior_call_id: &str,
    ) -> Vec<ResponseInputItem> {
        Self::skipped_responses_with_status(
            call,
            "skipped_due_to_prior_failure",
            "prior_call_id",
            prior_call_id,
        )
    }

    pub(crate) fn terminal_completion_skipped_responses(
        call: &ToolCall,
        terminal_call_id: &str,
    ) -> Vec<ResponseInputItem> {
        Self::skipped_responses_with_status(
            call,
            "skipped_due_to_terminal_completion",
            "terminal_call_id",
            terminal_call_id,
        )
    }

    fn skipped_responses_with_status(
        call: &ToolCall,
        status: &str,
        cause_field: &str,
        cause_call_id: &str,
    ) -> Vec<ResponseInputItem> {
        let message =
            format!("TaskSpaceToolSkippedV1:\nstatus: {status}\n{cause_field}: {cause_call_id}");
        let response = match &call.payload {
            ToolPayload::ToolSearch { .. } => ResponseInputItem::ToolSearchOutput {
                call_id: call.call_id.clone(),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: Vec::new(),
            },
            ToolPayload::Mcp { .. } => ResponseInputItem::McpToolCallOutput {
                call_id: call.call_id.clone(),
                output: codex_protocol::mcp::CallToolResult::from_error_text(message),
            },
            ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
                call_id: call.call_id.clone(),
                name: None,
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            },
            ToolPayload::Function { .. } | ToolPayload::LocalShell { .. } => {
                ResponseInputItem::FunctionCallOutput {
                    call_id: call.call_id.clone(),
                    output: codex_protocol::models::FunctionCallOutputPayload {
                        body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                        success: Some(false),
                    },
                }
            }
        };
        let mut responses = vec![response];
        if matches!(call.payload, ToolPayload::ToolSearch { .. }) {
            responses.push(Self::factual_message(serde_json::json!({
                "schema_version": "TaskSpaceToolSkippedV1",
                "status": status,
                "success": false,
                "call_id": call.call_id,
                "cause": {
                    "field": cause_field,
                    "call_id": cause_call_id,
                },
            })));
        }
        responses
    }

    fn factual_message(value: serde_json::Value) -> ResponseInputItem {
        ResponseInputItem::Message {
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: value.to_string(),
            }],
        }
    }
}

impl ToolCallRuntime {
    fn aborted_response(call: &ToolCall, secs: f32) -> AnyToolResult {
        AnyToolResult {
            call_id: call.call_id.clone(),
            payload: call.payload.clone(),
            result: Box::new(AbortedToolOutput {
                message: Self::abort_message(call, secs),
            }),
            post_tool_use_payload: None,
        }
    }

    fn abort_message(call: &ToolCall, secs: f32) -> String {
        if call.tool_name.namespace.is_none()
            && matches!(
                call.tool_name.name.as_str(),
                "shell"
                    | "container.exec"
                    | "exec_command"
                    | "local_shell"
                    | "shell_command"
                    | "unified_exec"
            )
        {
            format!("Wall time: {secs:.1} seconds\naborted by user")
        } else {
            format!("aborted by user after {secs:.1}s")
        }
    }

    async fn record_taskspace_bound_tool_result(
        &self,
        prepared: &ActionMapPreparedCall,
        success: bool,
        response: &ResponseInputItem,
    ) -> Result<(), String> {
        self.session
            .record_taskspace_bound_tool_result(
                &self.turn_context,
                prepared,
                success,
                result_ref_id(response),
            )
            .await
    }

    fn apply_failed_bound_result_commit(
        execution: &mut ToolCallExecution,
        prepared: &ActionMapPreparedCall,
        error: String,
    ) {
        tracing::warn!(
            target: "codex_core::taskspace",
            event_name = "taskspace_native_tool_result_attribution_failed",
            call_id = prepared.call_id,
            node_id = prepared.node_id,
            reservation_id = prepared.reservation_id,
            state_commit = false,
            error = %error,
            "taskspace_bound_tool_result_record_failed"
        );
        execution.succeeded = false;
        execution
            .supplemental_responses
            .push(Self::factual_message(serde_json::json!({
                "schema_version": "TaskSpaceBoundResultCommitFailureV1",
                "status": "failed",
                "success": false,
                "state_commit": false,
                "call_id": prepared.call_id,
                "reservation_id": prepared.reservation_id,
                "error": error,
            })));
    }
}

fn result_ref_id(response: &ResponseInputItem) -> String {
    let call_id = match response {
        ResponseInputItem::FunctionCallOutput { call_id, .. }
        | ResponseInputItem::McpToolCallOutput { call_id, .. }
        | ResponseInputItem::CustomToolCallOutput { call_id, .. }
        | ResponseInputItem::ToolSearchOutput { call_id, .. } => call_id,
        ResponseInputItem::Message { .. } => "unknown",
    };
    format!("tool-result://call/{call_id}")
}

fn function_call_error_model_visible_message(err: &FunctionCallError) -> String {
    match err {
        FunctionCallError::RespondToModel(message) => message.clone(),
        FunctionCallError::MissingLocalShellCallId => {
            "Tool call failed because the shell call id was missing.".to_string()
        }
        FunctionCallError::Fatal(_) => "Tool call failed with a fatal runtime error.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::ToolCallExecution;
    use super::ToolCallRuntime;
    use crate::action_map::ActionMapPreparedCall;
    use crate::function_tool::FunctionCallError;
    use crate::tools::context::ToolPayload;
    use crate::tools::context::response_input_model_visible_preview;
    use crate::tools::router::ToolCall;
    use codex_protocol::models::SearchToolCallParams;
    use codex_tools::ToolName;

    fn failure_response_preview(err: FunctionCallError) -> String {
        let call = ToolCall {
            tool_name: ToolName::plain("apply_patch"),
            call_id: "call-test".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let response = ToolCallRuntime::failure_response_for_error(&call, &err);
        response_input_model_visible_preview(&response)
    }

    #[test]
    fn failure_response_preview_records_model_visible_error_text() {
        let preview = failure_response_preview(FunctionCallError::RespondToModel(
            "failed to parse apply_patch: missing field `action`".to_string(),
        ));

        assert!(preview.contains("failed to parse apply_patch"));
        assert!(preview.contains("missing field `action`"));
    }

    #[test]
    fn failure_response_preview_bounds_model_visible_error_text() {
        let long_error = format!("apply_patch failed\n{}", "line\n".repeat(128));
        let preview = failure_response_preview(FunctionCallError::RespondToModel(long_error));

        assert!(preview.contains("apply_patch failed"));
        assert!(preview.contains("telemetry preview truncated"));
    }

    #[test]
    fn failure_response_preview_preserves_local_infra_error() {
        let nul_separated = "Bash/Service/CreateInstance/E_ACCESSDENIED"
            .chars()
            .flat_map(|ch| [ch, '\0'])
            .collect::<String>();
        let preview = failure_response_preview(FunctionCallError::RespondToModel(format!(
            "garbled host output: {nul_separated}"
        )));

        assert!(preview.contains("garbled host output"));
        assert!(preview.contains('\0'));
        assert!(!preview.contains("local_validator_infra_failure"));
    }

    #[test]
    fn tool_search_failure_keeps_pairing_output_and_exact_error_fact() {
        let call = ToolCall {
            tool_name: ToolName::plain("tool_search"),
            call_id: "search-1".to_string(),
            payload: ToolPayload::ToolSearch {
                arguments: SearchToolCallParams {
                    query: String::new(),
                    limit: None,
                },
            },
        };
        let error = FunctionCallError::RespondToModel("query must not be empty".to_string());
        let response = ToolCallRuntime::failure_response_for_error(&call, &error);
        let supplemental =
            ToolCallRuntime::supplemental_failure_response(&call, &error).expect("error fact");

        let codex_protocol::models::ResponseInputItem::ToolSearchOutput { status, tools, .. } =
            response
        else {
            panic!("expected tool_search output");
        };
        assert_eq!(status, "completed");
        assert!(tools.is_empty());
        let codex_protocol::models::ResponseInputItem::Message { content, .. } = supplemental
        else {
            panic!("expected supplemental factual message");
        };
        let text = content
            .into_iter()
            .map(|item| match item {
                codex_protocol::models::ContentItem::InputText { text }
                | codex_protocol::models::ContentItem::OutputText { text } => text,
                codex_protocol::models::ContentItem::InputImage { .. } => String::new(),
            })
            .collect::<String>();
        assert!(text.contains("ToolSearchFailureV1"));
        assert!(text.contains("query must not be empty"));
        assert!(text.contains("\"success\":false"));
    }

    #[test]
    fn bound_result_commit_failure_is_factual_and_marks_execution_failed() {
        let prepared = ActionMapPreparedCall {
            map_id: "map-1".to_string(),
            revision: 8,
            call_id: "call-1".to_string(),
            call_index: 0,
            node_id: "inspect".to_string(),
            tool_name: "read_file".to_string(),
            reservation_id: "reservation-1".to_string(),
        };
        let original_error = "reservation release rejected: revision mismatch";
        let native_response = codex_protocol::models::ResponseInputItem::FunctionCallOutput {
            call_id: prepared.call_id.clone(),
            output: codex_protocol::models::FunctionCallOutputPayload {
                body: codex_protocol::models::FunctionCallOutputBody::Text(
                    "native output".to_string(),
                ),
                success: Some(true),
            },
        };
        let mut execution = ToolCallExecution {
            response: native_response.clone(),
            supplemental_responses: Vec::new(),
            succeeded: true,
            taskspace_terminal_carrier: None,
        };

        ToolCallRuntime::apply_failed_bound_result_commit(
            &mut execution,
            &prepared,
            original_error.to_string(),
        );

        assert!(!execution.succeeded);
        assert_eq!(execution.response, native_response);
        let codex_protocol::models::ResponseInputItem::Message { content, .. } = execution
            .supplemental_responses
            .pop()
            .expect("supplemental failure")
        else {
            panic!("expected factual supplemental message");
        };
        let text = content
            .into_iter()
            .map(|item| match item {
                codex_protocol::models::ContentItem::InputText { text }
                | codex_protocol::models::ContentItem::OutputText { text } => text,
                codex_protocol::models::ContentItem::InputImage { .. } => String::new(),
            })
            .collect::<String>();
        let fact: serde_json::Value = serde_json::from_str(&text).expect("valid fact JSON");
        assert_eq!(fact["state_commit"], false);
        assert_eq!(fact["success"], false);
        assert_eq!(fact["call_id"], prepared.call_id);
        assert_eq!(fact["reservation_id"], prepared.reservation_id);
        assert_eq!(fact["error"], original_error);
    }
}
