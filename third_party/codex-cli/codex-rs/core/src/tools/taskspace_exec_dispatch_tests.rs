use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::ResponseInputItem;
use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use futures::StreamExt;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::catalog::TaskSpaceClientTransport;
use super::*;
use crate::function_tool::FunctionCallError;
use crate::session::tests::make_session_and_context;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;

fn function_spec(name: &str) -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: name.to_string(),
        description: String::new(),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([
                ("delay_ms".to_string(), JsonSchema::integer(None)),
                ("fail".to_string(), JsonSchema::boolean(None)),
            ]),
            None,
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
        defer_loading: None,
    })
}

fn prepared_function(
    index: usize,
    tool_name: ToolName,
    arguments: serde_json::Value,
) -> PreparedClientCall {
    PreparedClientCall {
        identity: TaskSpaceExecInternalCallId {
            outer_call_id: "outer".to_string(),
            index,
        },
        call: ClientCall {
            public_name: tool_name.display(),
            tool_name,
            node_id: "work".to_string(),
            input: ClientCallInput::Function(arguments),
            transport: TaskSpaceClientTransport::Function,
        },
    }
}

#[tokio::test]
async fn preparation_reuses_native_alias_namespace_and_tool_search_parsing() {
    let (session, _) = make_session_and_context().await;
    let calls = vec![
        prepared_function(
            0,
            ToolName::plain("exec_command"),
            json!({"cmd": "pwd", "workdir": "."}),
        ),
        prepared_function(
            1,
            ToolName::namespaced("mcp__calendar", "list_events"),
            json!({"limit": 2}),
        ),
        PreparedClientCall {
            identity: TaskSpaceExecInternalCallId {
                outer_call_id: "outer".to_string(),
                index: 2,
            },
            call: ClientCall {
                public_name: "apply_patch".to_string(),
                tool_name: ToolName::plain("apply_patch"),
                node_id: "work".to_string(),
                input: ClientCallInput::Freeform("*** Begin Patch\n*** End Patch".to_string()),
                transport: TaskSpaceClientTransport::Freeform,
            },
        },
        PreparedClientCall {
            identity: TaskSpaceExecInternalCallId {
                outer_call_id: "outer".to_string(),
                index: 3,
            },
            call: ClientCall {
                public_name: "tool_search".to_string(),
                tool_name: ToolName::plain("tool_search"),
                node_id: "work".to_string(),
                input: ClientCallInput::Function(json!({"query": "calendar"})),
                transport: TaskSpaceClientTransport::ToolSearch,
            },
        },
    ];

    let native = prepare_client_calls(&session, &calls).await.unwrap();
    assert_eq!(native.len(), 4);
    assert_eq!(
        native[0].call.provider_tool_name,
        ToolName::plain("exec_command")
    );
    assert_eq!(
        native[0].call.dispatch_tool_name,
        ToolName::plain("shell_command")
    );
    assert_eq!(
        native[1].call.dispatch_tool_name,
        ToolName::namespaced("mcp__calendar", "list_events")
    );
    assert!(matches!(native[2].call.payload, ToolPayload::Custom { .. }));
    assert!(matches!(
        native[3].call.payload,
        ToolPayload::ToolSearch { .. }
    ));
    assert_eq!(native[3].call.call_id, "outer/taskspace/call/3");
}

#[derive(Default)]
struct TimingHandler {
    active: AtomicUsize,
    max_active: AtomicUsize,
    calls: Mutex<Vec<String>>,
}

impl ToolHandler for TimingHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolPayload::Function { arguments } = invocation.payload else {
            return Err(FunctionCallError::RespondToModel(
                "wrong payload".to_string(),
            ));
        };
        let arguments: serde_json::Value = serde_json::from_str(&arguments).unwrap();
        let delay = arguments["delay_ms"].as_u64().unwrap_or_default();
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(delay)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        self.calls.lock().await.push(invocation.call_id);
        if arguments["fail"].as_bool().unwrap_or(false) {
            return Err(FunctionCallError::RespondToModel(
                "expected failure".to_string(),
            ));
        }
        Ok(FunctionToolOutput::from_text("ok".to_string(), Some(true)))
    }
}

async fn runtime(
    supports_parallel: bool,
) -> (
    Arc<crate::session::session::Session>,
    ToolCallRuntime,
    Arc<TimingHandler>,
) {
    let handler = Arc::new(TimingHandler::default());
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec_with_parallel_support(function_spec("inspect"), supports_parallel);
    builder.register_handler("inspect", Arc::clone(&handler));
    let router = Arc::new(ToolRouter::from_builder_for_test(builder));
    let (session, turn) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let tracker: SharedTurnDiffTracker = Arc::new(Mutex::new(TurnDiffTracker::new()));
    let runtime = ToolCallRuntime::new(router, Arc::clone(&session), turn, tracker);
    (session, runtime, handler)
}

#[tokio::test]
async fn dispatch_preserves_native_parallel_policy_and_completion_order() {
    let (session, runtime, handler) = runtime(true).await;
    let calls = vec![
        prepared_function(0, ToolName::plain("inspect"), json!({"delay_ms": 80})),
        prepared_function(1, ToolName::plain("inspect"), json!({"delay_ms": 5})),
    ];
    let native = prepare_client_calls(&session, &calls).await.unwrap();
    let mut results = dispatch_client_calls(runtime, native, CancellationToken::new());
    let first = results.next().await.unwrap();
    let second = results.next().await.unwrap();

    assert_eq!(first.identity.index, 1);
    assert_eq!(second.identity.index, 0);
    assert!(handler.max_active.load(Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn dispatch_preserves_native_serial_policy_and_failure_payload() {
    let (session, runtime, handler) = runtime(false).await;
    let calls = vec![
        prepared_function(
            0,
            ToolName::plain("inspect"),
            json!({"delay_ms": 10, "fail": true}),
        ),
        prepared_function(1, ToolName::plain("inspect"), json!({"delay_ms": 10})),
    ];
    let native = prepare_client_calls(&session, &calls).await.unwrap();
    let results = dispatch_client_calls(runtime, native, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;

    assert_eq!(handler.max_active.load(Ordering::SeqCst), 1);
    let failed = results
        .iter()
        .find(|result| result.identity.index == 0)
        .unwrap();
    let ResponseInputItem::FunctionCallOutput { output, .. } = failed.response.as_ref().unwrap()
    else {
        panic!("expected native function output");
    };
    assert_eq!(output.success, Some(false));
    assert!(
        matches!(output.body, FunctionCallOutputBody::Text(ref text) if text == "expected failure")
    );
}

#[tokio::test]
async fn tool_search_pairing_completion_preserves_execution_failure_status() {
    let handler = Arc::new(TimingHandler::default());
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec(ToolSpec::ToolSearch {
        execution: "client".into(),
        description: "Search deferred tools.".into(),
        parameters: JsonSchema::object(
            BTreeMap::from([("query".into(), JsonSchema::string(None))]),
            Some(vec!["query".into()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
    });
    builder.register_handler("tool_search", handler);
    let router = Arc::new(ToolRouter::from_builder_for_test(builder));
    let (session, turn) = make_session_and_context().await;
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        router,
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    let calls = vec![PreparedClientCall {
        identity: TaskSpaceExecInternalCallId {
            outer_call_id: "outer".into(),
            index: 0,
        },
        call: ClientCall {
            public_name: "tool_search".into(),
            tool_name: ToolName::plain("tool_search"),
            node_id: "work".into(),
            input: ClientCallInput::Function(json!({"query": "calendar"})),
            transport: TaskSpaceClientTransport::ToolSearch,
        },
    }];

    let native = prepare_client_calls(session.as_ref(), &calls)
        .await
        .unwrap();
    let result = dispatch_client_calls(runtime, native, CancellationToken::new())
        .next()
        .await
        .unwrap();

    assert!(result.execution_failed);
    assert!(!result.cancelled);
    let ResponseInputItem::ToolSearchOutput { status, tools, .. } = result.response.unwrap() else {
        panic!("ToolSearch failure must preserve its native pairing output")
    };
    assert_eq!(status, "completed");
    assert!(tools.is_empty());
}
