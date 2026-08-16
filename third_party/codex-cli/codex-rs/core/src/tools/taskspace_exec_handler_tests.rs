use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_protocol::models::ResponseItem;
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
use tracing_test::traced_test;

use super::*;
use crate::action_map::rooted_dag::NodeState;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::tests::make_session_and_context;
use crate::session::turn_context::TurnContext;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;

#[test]
fn waiting_preflight_feedback_names_parents_and_mechanical_batch_boundary() {
    let feedback = super::handler::render_preflight_rejection(
        &TaskSpaceExecPreflightError::ClientNodeNotExecutable {
            index: 2,
            node_id: "implement".into(),
            state: NodeState::Waiting,
            incomplete_parent_ids: vec!["inspect".into(), "design".into()],
        },
    );

    assert_eq!(
        feedback,
        "Tool action 2 targeted work node `implement` in state `waiting`; incomplete direct parent nodes: [\"inspect\", \"design\"]. Only the sequence's preceding Map operation can unlock work; Tool outcomes do not change node state. No Map or Tool actions were executed."
    );
}

#[test]
fn missing_response_work_feedback_names_both_legal_sources() {
    let feedback = super::handler::render_preflight_rejection(
        &TaskSpaceExecPreflightError::ResponseWorkMissing {
            sequence_type: "initialize_and_work".into(),
        },
    );
    assert!(feedback.contains("native Provider Tool"));
    assert!(feedback.contains("taskspace_exec.tools"));
}
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

fn initialize_input() -> Value {
    json!({
        "root": {"node_id": "root", "goal": "deliver", "content": "", "parents": []},
        "work_nodes": [{
            "node_id": "work",
            "goal": "implement",
            "content": "",
            "parents": ["root"]
        }],
        "finish": {"node_id": "finish", "goal": "close", "content": "", "parents": ["work"]}
    })
}

fn inspect_action(input: Value) -> Value {
    json!({"tool": "inspect", "node_id": "work", "input": input})
}

fn inspect_action_for(node_id: &str, input: Value) -> Value {
    json!({"tool": "inspect", "node_id": node_id, "input": input})
}

fn initialize_work(tools: Vec<Value>) -> Value {
    json!({
        "type": "initialize_and_work",
        "initialize_map": initialize_input(),
        "tools": tools
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
                action.action_id.ends_with("/call/0") && action.outcome == "succeeded"
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
    catalog: Arc<TaskSpaceExecCatalog>,
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
    let response_scope = Arc::new(TaskSpaceExecResponseScope::new(
        catalog.capability_identity_arc(),
        catalog.hosted_tool_identities(),
    ));
    let handler = TaskSpaceExecHandler::new(
        Arc::clone(&catalog),
        client_router,
        Arc::clone(&response_scope),
    );
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    Harness {
        session: Arc::new(session),
        turn: Arc::new(turn),
        handler,
        catalog,
        response_scope,
        client_handler,
    }
}

fn assert_feedback_matches_declared_result(harness: &Harness, feedback: &Value) {
    let schema = harness
        .catalog
        .declaration()
        .output_schema
        .clone()
        .expect("TaskSpace Exec declares its result contract");
    let schema: JsonSchema = serde_json::from_value(schema).unwrap();
    super::schema_validation::validate_json_schema(feedback, &schema).unwrap();
}

fn finalize_scope(harness: &Harness) {
    harness
        .response_scope
        .record_completed_item(&ResponseItem::FunctionCall {
            id: None,
            name: TASKSPACE_EXEC_TOOL_NAME.into(),
            namespace: None,
            arguments: "{}".into(),
            call_id: "outer".into(),
        });
    harness
        .response_scope
        .finalize(
            true,
            Some(TaskSpaceExecResponseIdentity {
                provider_response_id: "response-1".into(),
                provider_request_id: Some("request-1".into()),
                provider_logical_request_id: Some("logical-1".into()),
                provider_attempt_seq: Some(1),
            }),
        )
        .unwrap();
}

async fn begin_scope(harness: &Harness) {
    let (map_id, map) = super::handler::read_current_map(harness.session.as_ref(), "outer")
        .await
        .unwrap();
    harness
        .response_scope
        .begin_request(map_id, map.as_ref().map(|map| map.revision))
        .unwrap();
}

fn invocation(harness: &Harness, arguments: Value) -> ToolInvocation {
    invocation_raw_with_token(harness, arguments.to_string(), CancellationToken::new())
}

fn invocation_with_token(
    harness: &Harness,
    arguments: Value,
    cancellation_token: CancellationToken,
) -> ToolInvocation {
    invocation_raw_with_token(harness, arguments.to_string(), cancellation_token)
}

fn invocation_raw(harness: &Harness, arguments: impl Into<String>) -> ToolInvocation {
    invocation_raw_with_token(harness, arguments.into(), CancellationToken::new())
}

fn invocation_raw_with_token(
    harness: &Harness,
    arguments: String,
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
        payload: ToolPayload::Function { arguments },
    }
}

#[tokio::test]
async fn malformed_json_feedback_preserves_only_the_syntax_error() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let malformed = r#"{"type":"initialize_and_work","initialize_map":{},"tools":[{"tool":"inspect","node_id":"work","input":{}}]"#;

    let result = harness
        .handler
        .handle(invocation_raw(&harness, malformed))
        .await;
    let error = match result {
        Ok(_) => panic!("malformed JSON must be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("invalid JSON syntax:"));
    assert!(!message.contains("top-level input must directly contain"));
    assert!(!message.contains("do not wrap it in an `arguments` field"));
    assert!(message.contains("No Map or Tool actions were executed"));
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
async fn wrapped_arguments_feedback_distinguishes_contract_from_json_syntax() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let wrapped =
        json!({"arguments": initialize_work(vec![inspect_action(json!({}))]).to_string()});

    let result = harness.handler.handle(invocation(&harness, wrapped)).await;
    let error = match result {
        Ok(_) => panic!("wrapped arguments must be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("invalid top-level contract:"));
    assert!(message.contains("unexpected field `arguments`"));
    assert!(message.contains("do not wrap it in an `arguments` field"));
    assert!(!message.contains("invalid JSON syntax:"));
    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn other_top_level_contract_errors_do_not_inject_wrapper_guidance() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let unknown = json!({
        "type": "initialize_and_work",
        "initialize_map": initialize_input(),
        "tools": [inspect_action(json!({}))],
        "unexpected": true
    });

    let result = harness.handler.handle(invocation(&harness, unknown)).await;
    let error = match result {
        Ok(_) => panic!("unknown top-level field must be rejected"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("invalid top-level contract:"));
    assert!(message.contains("unknown field `unexpected`"));
    assert!(!message.contains("do not wrap it in an `arguments` field"));
    assert!(!message.contains("invalid JSON syntax:"));
    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn provider_work_allows_initialization_without_placeholder_client_work() {
    let harness = harness(true).await;
    begin_scope(&harness).await;
    harness
        .response_scope
        .record_completed_item(&ResponseItem::WebSearchCall {
            id: Some("provider-work".into()),
            status: Some("completed".into()),
            action: None,
        });
    finalize_scope(&harness);

    harness
        .handler
        .handle(invocation(
            &harness,
            json!({
                "type": "initialize_and_work",
                "initialize_map": initialize_input()
            }),
        ))
        .await
        .expect("current-response Provider work must satisfy the work sequence");

    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
    let map = harness
        .session
        .canonical_action_map_snapshot()
        .await
        .unwrap()
        .map
        .expect("Map initialized by Provider-first response");
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "work")
            .unwrap()
            .state,
        "ready"
    );
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
            initialize_work(vec![inspect_action(json!({"delay_ms": 60}))]),
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
    assert_eq!(work.state, "in_flight");
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
                    arguments: initialize_work(vec![inspect_action(json!({"delay_ms": 60}))])
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
            initialize_work(vec![
                inspect_action(json!({"delay_ms": 5})),
                inspect_action(json!({"delay_ms": 60, "fail": true})),
            ]),
        ))
        .await
        .unwrap();

    assert_eq!(output.success, Some(false));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_feedback_matches_declared_result(&harness, &feedback);
    assert_eq!(feedback["kind"], "taskspace_exec_result");
    assert_eq!(feedback["outer_call_id"], "outer");
    assert_eq!(
        feedback["client_results"][0]["action_id"],
        "outer/taskspace/call/0"
    );
    assert_eq!(
        feedback["client_results"][0]["result"],
        json!({"type": "function", "output": "native-result"})
    );
    assert_eq!(feedback["client_results"][1]["outcome"], "failed");
    assert_eq!(feedback["client_results"][1]["error"], "expected failure");
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
    assert_eq!(work.state, "in_flight");
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
            initialize_work(vec![
                inspect_action(json!({"delay_ms": 5})),
                inspect_action(json!({"fatal": true})),
            ]),
        ))
        .await
        .expect("internal fatal must remain an outer Tool result");

    assert_eq!(output.success, Some(false));
    let feedback: Value = serde_json::from_str(&output.into_text()).unwrap();
    assert_eq!(feedback["client_results"].as_array().unwrap().len(), 2);
    assert_eq!(feedback["client_results"][0]["outcome"], "succeeded");
    assert_eq!(
        feedback["client_results"][0]["result"],
        json!({"type": "function", "output": "native-result"})
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
#[traced_test]
async fn failed_preflight_has_no_map_or_client_tool_side_effect() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    finalize_scope(&harness);
    let result = harness
        .handler
        .handle(invocation(
            &harness,
            initialize_work(vec![inspect_action_for("missing", json!({}))]),
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
    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("taskspace.exec.rejected")
                    && line.contains("reason_code=\"preflight_rejected\"")
                    && line.contains("outer_call_id=\"outer\"")
            })
            .map(|_| Ok(()))
            .unwrap_or_else(|| Err("expected stable preflight rejection event".to_string()))
    });
}

#[tokio::test]
async fn response_uses_request_time_revision_and_rejects_a_stale_plan() {
    let harness = harness(false).await;
    begin_scope(&harness).await;
    let (map_id, current) = super::handler::read_current_map(harness.session.as_ref(), "outer")
        .await
        .unwrap();
    assert!(current.is_none());
    let operation: MapOperation = serde_json::from_value(json!({
        "tool": "initialize_map",
        "arguments": initialize_input()
    }))
    .unwrap();
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
            initialize_work(vec![inspect_action(json!({}))]),
        ))
        .await;

    let error = match result {
        Ok(_) => panic!("stale response must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("MapRevisionChanged"));
    assert_eq!(harness.client_handler.calls.load(Ordering::SeqCst), 0);
    let (_, current) = super::handler::read_current_map(harness.session.as_ref(), "outer")
        .await
        .unwrap();
    assert_eq!(current.unwrap().revision, 1);
}

#[tokio::test]
async fn production_router_exposes_only_exec_and_hosted_and_blocks_client_bypass() {
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec(inspect_spec());
    builder.push_spec(hosted_spec());
    builder.register_handler("inspect", Arc::new(LedgerAwareHandler::default()));
    let router = ToolRouter::from_builder_for_test(builder)
        .into_taskspace(&[])
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
