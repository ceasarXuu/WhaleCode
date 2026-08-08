use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_protocol::models::ResponseItem;
use codex_protocol::models::WebSearchAction;
use codex_protocol::protocol::MapRuntimeMode;
use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::Value;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::tests::make_session_and_context;
use crate::session::turn_context::TurnContext;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;

fn inspect_spec() -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: "inspect".into(),
        description: "Test client Tool.".into(),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([
                ("delay_ms".into(), JsonSchema::integer(None)),
                ("fail".into(), JsonSchema::boolean(None)),
                ("fatal".into(), JsonSchema::boolean(None)),
            ]),
            None,
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
        defer_loading: None,
    })
}

fn hosted_spec() -> ToolSpec {
    ToolSpec::WebSearch {
        external_web_access: Some(true),
        filters: None,
        user_location: None,
        search_context_size: None,
        search_content_types: None,
    }
}

fn initialize_call() -> Value {
    json!({
        "tool": "initialize_map",
        "arguments": {
            "root": {"node_id": "root", "goal": "deliver", "content": "", "parents": []},
            "work_nodes": [{
                "node_id": "work",
                "goal": "implement",
                "content": "",
                "parents": ["root"]
            }],
            "finish": {"node_id": "finish", "goal": "close", "content": "", "parents": ["work"]}
        }
    })
}

#[derive(Default)]
struct LedgerAwareHandler {
    calls: AtomicUsize,
    saw_pending_before_work: AtomicBool,
    slow_saw_fast_settled: AtomicBool,
}

impl ToolHandler for LedgerAwareHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let arguments: Value = match &invocation.payload {
            ToolPayload::Function { arguments } => serde_json::from_str(arguments).unwrap(),
            _ => panic!("expected function payload"),
        };
        let snapshot = invocation
            .session
            .canonical_action_map_snapshot()
            .await
            .unwrap();
        let map = snapshot
            .map
            .expect("candidate Map persisted before dispatch");
        let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
        if work
            .actions
            .iter()
            .any(|action| action.action_id == invocation.call_id && action.outcome == "pending")
        {
            self.saw_pending_before_work.store(true, Ordering::SeqCst);
        }

        let delay = arguments["delay_ms"].as_u64().unwrap_or_default();
        tokio::time::sleep(Duration::from_millis(delay)).await;
        if delay >= 50 {
            let snapshot = invocation
                .session
                .canonical_action_map_snapshot()
                .await
                .unwrap();
            let work = snapshot
                .map
                .unwrap()
                .nodes
                .into_iter()
                .find(|node| node.id == "work")
                .unwrap();
            if work.actions.iter().any(|action| {
                action.action_id.ends_with("/call/1") && action.outcome == "succeeded"
            }) {
                self.slow_saw_fast_settled.store(true, Ordering::SeqCst);
            }
        }
        if arguments["fail"].as_bool().unwrap_or(false) {
            return Err(FunctionCallError::RespondToModel("expected failure".into()));
        }
        if arguments["fatal"].as_bool().unwrap_or(false) {
            return Err(FunctionCallError::Fatal("expected fatal".into()));
        }
        Ok(FunctionToolOutput::from_text(
            "native-result".into(),
            Some(true),
        ))
    }
}

struct Harness {
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    handler: TaskSpaceExecHandler,
    response_scope: Arc<TaskSpaceExecResponseScope>,
    client_handler: Arc<LedgerAwareHandler>,
}

async fn harness(include_hosted: bool) -> Harness {
    let client_handler = Arc::new(LedgerAwareHandler::default());
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec_with_parallel_support(inspect_spec(), true);
    if include_hosted {
        builder.push_spec_with_parallel_support(hosted_spec(), true);
    }
    builder.register_handler("inspect", Arc::clone(&client_handler));
    let client_router = Arc::new(ToolRouter::from_builder_for_test(builder));
    let catalog = Arc::new(TaskSpaceExecCatalog::build(&client_router.specs()).unwrap());
    let response_scope = Arc::new(TaskSpaceExecResponseScope::default());
    let handler = TaskSpaceExecHandler::new(catalog, client_router, Arc::clone(&response_scope));
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    Harness {
        session: Arc::new(session),
        turn: Arc::new(turn),
        handler,
        response_scope,
        client_handler,
    }
}

fn finalize_scope(harness: &Harness) {
    harness.response_scope.record_completed_item(
        Some(99),
        &ResponseItem::FunctionCall {
            id: None,
            name: TASKSPACE_EXEC_TOOL_NAME.into(),
            namespace: None,
            arguments: "{}".into(),
            call_id: "outer".into(),
        },
    );
    harness.response_scope.finalize(true).unwrap();
}

async fn begin_scope(harness: &Harness) {
    let (map_id, map) = super::handler::read_current_map(harness.session.as_ref())
        .await
        .unwrap();
    harness
        .response_scope
        .begin_request(map_id, map.as_ref().map(|map| map.revision))
        .unwrap();
}

fn invocation(harness: &Harness, arguments: Value) -> ToolInvocation {
    invocation_with_token(harness, arguments, CancellationToken::new())
}

fn invocation_with_token(
    harness: &Harness,
    arguments: Value,
    cancellation_token: CancellationToken,
) -> ToolInvocation {
    ToolInvocation {
        session: Arc::clone(&harness.session),
        turn: Arc::clone(&harness.turn),
        cancellation_token,
        tracker: Arc::new(Mutex::new(TurnDiffTracker::new())),
        call_id: "outer".into(),
        tool_name: ToolName::plain(TASKSPACE_EXEC_TOOL_NAME),
        source: ToolCallSource::Direct,
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
    }
}

#[tokio::test]
async fn cancelled_native_call_is_settled_without_changing_node_state() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let cancellation_token = CancellationToken::new();
    cancellation_token.cancel();
    let output = harness
        .handler
        .handle(invocation_with_token(
            &harness,
            json!({
                "calls": [
                    initialize_call(),
                    {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": 60}}
                ],
                "hosted_bindings": []
            }),
            cancellation_token,
        ))
        .await
        .unwrap();

    assert_eq!(output.success, Some(false));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_eq!(feedback["client_results"][0]["outcome"], "cancelled");
    harness
        .session
        .await_taskspace_action_settlements()
        .await
        .unwrap();
    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .unwrap();
    let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
    assert_eq!(work.state, "ready");
    assert_eq!(work.actions[0].outcome, "cancelled");
}

#[tokio::test]
async fn interrupted_outer_exec_does_not_cancel_registered_action_producer() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let session = Arc::clone(&harness.session);
    let handler = harness.handler;
    let turn = Arc::clone(&harness.turn);
    let client_handler = Arc::clone(&harness.client_handler);
    let task = tokio::spawn(async move {
        handler
            .handle(ToolInvocation {
                session,
                turn,
                cancellation_token: CancellationToken::new(),
                tracker: Arc::new(Mutex::new(TurnDiffTracker::new())),
                call_id: "outer".into(),
                tool_name: ToolName::plain(TASKSPACE_EXEC_TOOL_NAME),
                source: ToolCallSource::Direct,
                payload: ToolPayload::Function {
                    arguments: json!({
                        "calls": [
                            initialize_call(),
                            {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": 60}}
                        ],
                        "hosted_bindings": []
                    })
                    .to_string(),
                },
            })
            .await
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        while !client_handler
            .saw_pending_before_work
            .load(Ordering::SeqCst)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    task.abort();
    let join_error = match task.await {
        Ok(_) => panic!("interrupted Exec must not complete"),
        Err(error) => error,
    };
    assert!(join_error.is_cancelled());

    harness.session.finish_taskspace_action_producers().await;
    harness
        .session
        .await_taskspace_action_settlements()
        .await
        .expect("registered producer must publish its settlement");
    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .unwrap();
    let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
    assert_eq!(work.actions[0].outcome, "succeeded");
    assert_eq!(client_handler.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handler_persists_pending_then_settles_each_native_result_without_node_transition() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let output = harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "calls": [
                    initialize_call(),
                    {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": 5}},
                    {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": 60, "fail": true}}
                ],
                "hosted_bindings": []
            }),
        ))
        .await
        .unwrap();

    assert_eq!(output.success, Some(false));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_eq!(feedback["kind"], "taskspace_exec_result");
    assert_eq!(feedback["outer_call_id"], "outer");
    assert_eq!(
        feedback["client_results"][0]["action_id"],
        "outer/taskspace/call/1"
    );
    assert_eq!(
        feedback["client_results"][0]["response"]["call_id"],
        "outer/taskspace/call/1"
    );
    assert_eq!(feedback["client_results"][1]["outcome"], "failed");
    assert!(
        harness
            .client_handler
            .saw_pending_before_work
            .load(Ordering::SeqCst)
    );
    assert!(
        harness
            .client_handler
            .slow_saw_fast_settled
            .load(Ordering::SeqCst)
    );
    harness
        .session
        .await_taskspace_action_settlements()
        .await
        .unwrap();

    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .unwrap();
    let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
    assert_eq!(work.state, "ready");
    assert_eq!(work.actions[0].outcome, "succeeded");
    assert_eq!(work.actions[1].outcome, "failed");
}

#[tokio::test]
async fn internal_fatal_is_returned_once_with_successful_sibling_feedback() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let output = harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "calls": [
                    initialize_call(),
                    {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": 5}},
                    {"tool": "inspect", "node_id": "work", "arguments": {"fatal": true}}
                ],
                "hosted_bindings": []
            }),
        ))
        .await
        .expect("internal fatal must remain an outer Tool result");

    assert_eq!(output.success, Some(false));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_eq!(feedback["client_results"].as_array().unwrap().len(), 2);
    assert_eq!(feedback["client_results"][0]["outcome"], "succeeded");
    assert_eq!(
        feedback["client_results"][0]["response"]["call_id"],
        "outer/taskspace/call/1"
    );
    assert_eq!(feedback["client_results"][1]["outcome"], "failed");
    assert_eq!(
        feedback["client_results"][1]["error"],
        "Fatal error: expected fatal"
    );
    harness
        .session
        .await_taskspace_action_settlements()
        .await
        .unwrap();

    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .unwrap();
    let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
    assert_eq!(work.actions[0].outcome, "succeeded");
    assert_eq!(work.actions[1].outcome, "failed");
}

#[tokio::test]
async fn failed_preflight_has_no_map_or_client_tool_side_effect() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let result = harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "calls": [
                    initialize_call(),
                    {"tool": "inspect", "node_id": "work", "arguments": {"delay_ms": "invalid"}}
                ],
                "hosted_bindings": []
            }),
        ))
        .await;
    let error = match result {
        Ok(_) => panic!("invalid batch must be rejected"),
        Err(error) => error,
    };

    assert!(matches!(error, FunctionCallError::RespondToModel(_)));
    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
    assert!(
        harness
            .session
            .canonical_action_map_snapshot()
            .await
            .unwrap()
            .map
            .is_none()
    );
}

#[tokio::test]
async fn response_uses_request_time_revision_and_rejects_a_stale_plan() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    let (map_id, current) = super::handler::read_current_map(harness.session.as_ref())
        .await
        .unwrap();
    assert!(current.is_none());
    let operation: MapOperation = serde_json::from_value(initialize_call()).unwrap();
    let MapOperationEffect::Candidate(candidate) =
        apply_map_operation(None, &map_id, operation).unwrap()
    else {
        panic!("initialize must produce a candidate")
    };
    let map_id_for_commit = map_id.clone();
    let (restored, _) = harness
        .session
        .mutate_canonical_action_map("test_concurrent_initialize", move |runtime, owner| {
            let restored =
                runtime.restore_store_map(&map_id_for_commit, owner, Some(candidate.clone()));
            (restored, Vec::new())
        })
        .await
        .unwrap();
    restored.unwrap();
    finalize_scope(&harness);

    let result = harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "calls": [
                    initialize_call(),
                    {"tool": "inspect", "node_id": "work", "arguments": {}}
                ],
                "hosted_bindings": []
            }),
        ))
        .await;

    let error = match result {
        Ok(_) => panic!("stale response must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("MapRevisionChanged"));
    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
    let (_, current) = super::handler::read_current_map(harness.session.as_ref())
        .await
        .unwrap();
    assert_eq!(current.unwrap().revision, 1);
}

#[tokio::test]
async fn hosted_result_is_bound_by_provider_identity_without_changing_node_state() {
    let harness = harness(true).await;
    begin_scope(&harness).await;
    harness.response_scope.record_completed_item(
        Some(2),
        &ResponseItem::WebSearchCall {
            id: Some("provider-search-1".into()),
            status: Some("completed".into()),
            action: Some(WebSearchAction::Search {
                query: Some("evidence".into()),
                queries: None,
            }),
        },
    );
    finalize_scope(&harness);
    let output = harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "calls": [initialize_call()],
                "hosted_bindings": [{"tool": "web_search", "node_ids": ["work"]}]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(output.success, Some(true));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_eq!(
        feedback["hosted_results"][0]["provider_id"],
        "provider-search-1"
    );
    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .unwrap();
    let work = map.nodes.iter().find(|node| node.id == "work").unwrap();
    assert_eq!(work.state, "ready");
    assert_eq!(work.actions[0].action_id, "provider-search-1");
    assert_eq!(work.actions[0].outcome, "succeeded");
}

#[tokio::test]
async fn production_router_exposes_only_exec_and_hosted_and_blocks_client_bypass() {
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec(inspect_spec());
    builder.push_spec(hosted_spec());
    builder.register_handler("inspect", Arc::new(LedgerAwareHandler::default()));
    let router = ToolRouter::from_builder_for_test(builder)
        .into_taskspace()
        .unwrap();
    let visible = router
        .model_visible_specs()
        .into_iter()
        .map(|spec| spec.name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(visible, vec!["taskspace_exec", "web_search"]);

    let (session, turn) = make_session_and_context().await;
    let result = router
        .dispatch_tool_call_with_code_mode_result(
            Arc::new(session),
            Arc::new(turn),
            CancellationToken::new(),
            Arc::new(Mutex::new(TurnDiffTracker::new())),
            ToolCall {
                provider_tool_name: ToolName::plain("inspect"),
                dispatch_tool_name: ToolName::plain("inspect"),
                call_id: "bypass".into(),
                payload: ToolPayload::Function {
                    arguments: "{}".into(),
                },
            },
            ToolCallSource::Direct,
        )
        .await;
    assert!(result.is_err());
}
