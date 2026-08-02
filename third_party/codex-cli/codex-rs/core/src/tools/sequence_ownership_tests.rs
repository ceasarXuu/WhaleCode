use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use codex_protocol::models::ResponseInputItem;
use codex_protocol::protocol::MapRuntimeMode;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use tokio::sync::Barrier;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::execute_response_tool_sequence;
use crate::function_tool::FunctionCallError;
use crate::session::tests::make_session_and_context;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;

#[derive(Clone, Copy)]
enum TestExecutionOwner {
    Client,
    ProviderHosted,
}

impl TestExecutionOwner {
    fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::ProviderHosted => "provider_hosted",
        }
    }
}

struct OwnershipRecordingHandler {
    owner: TestExecutionOwner,
    events: Arc<Mutex<Vec<String>>>,
    ready_frontier: Arc<Barrier>,
}

impl ToolHandler for OwnershipRecordingHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let tool_name = invocation.tool_name.display();
        let owner = self.owner.as_str();
        self.events
            .lock()
            .await
            .push(format!("{owner}:{tool_name}:started"));
        tokio::time::timeout(Duration::from_secs(2), self.ready_frontier.wait())
            .await
            .map_err(|_| {
                FunctionCallError::RespondToModel(format!(
                    "MVT-2 ready-frontier barrier timed out after {owner}:{tool_name} started"
                ))
            })?;
        self.events
            .lock()
            .await
            .push(format!("{owner}:{tool_name}:completed"));
        Ok(FunctionToolOutput::from_text(
            format!("{owner}:{tool_name}"),
            Some(true),
        ))
    }
}

fn function_spec(name: &str) -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: name.to_string(),
        description: String::new(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::default(),
        output_schema: None,
    })
}

fn function_call(name: &str, call_id: &str, arguments: impl Into<String>) -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain(name),
        dispatch_tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.into(),
        },
    }
}

fn initialize_work_batch(dependent: bool) -> ToolCall {
    let edges = if dependent {
        serde_json::json!([
            {"from": "root", "to": "local-a"},
            {"from": "local-a", "to": "hosted-b"},
            {"from": "hosted-b", "to": "local-c"},
            {"from": "local-c", "to": "finish"}
        ])
    } else {
        serde_json::json!([
            {"from": "root", "to": "local-a"},
            {"from": "root", "to": "hosted-b"},
            {"from": "root", "to": "local-c"},
            {"from": "local-a", "to": "finish"},
            {"from": "hosted-b", "to": "finish"},
            {"from": "local-c", "to": "finish"}
        ])
    };
    function_call(
        "taskspace_control",
        "map-prelude",
        serde_json::json!({
            "action": "initialize_and_execute",
            "root": {"node_id": "root", "goal": "Exercise one ready frontier"},
            "work_nodes": [
                {"node_id": "local-a", "goal": "Run local A"},
                {"node_id": "hosted-b", "goal": "Run hosted B"},
                {"node_id": "local-c", "goal": "Run local C"}
            ],
            "finish": {"node_id": "finish", "goal": "Finish"},
            "edges": edges,
            "actions": [
                {"node_id": "local-a", "tool": "mvt_client_a"},
                {"node_id": "hosted-b", "tool": "mvt_hosted_b"},
                {"node_id": "local-c", "tool": "mvt_client_c"}
            ]
        })
        .to_string(),
    )
}

fn work_calls() -> Vec<ToolCall> {
    vec![
        function_call("mvt_client_a", "call-local-a", "{}"),
        function_call("mvt_hosted_b", "call-hosted-b", "{}"),
        function_call("mvt_client_c", "call-local-c", "{}"),
    ]
}

fn recording_router(events: Arc<Mutex<Vec<String>>>, ready_frontier: Arc<Barrier>) -> ToolRouter {
    let client = Arc::new(OwnershipRecordingHandler {
        owner: TestExecutionOwner::Client,
        events: Arc::clone(&events),
        ready_frontier: Arc::clone(&ready_frontier),
    });
    let hosted = Arc::new(OwnershipRecordingHandler {
        owner: TestExecutionOwner::ProviderHosted,
        events,
        ready_frontier,
    });
    let mut builder = ToolRegistryBuilder::new();
    for name in ["mvt_client_a", "mvt_hosted_b", "mvt_client_c"] {
        builder.push_spec_with_parallel_support(function_spec(name), true);
    }
    builder.register_handler("mvt_client_a", Arc::clone(&client));
    builder.register_handler("mvt_hosted_b", hosted);
    builder.register_handler("mvt_client_c", client);
    ToolRouter::from_builder_for_test(builder)
}

async fn test_runtime(
    router: ToolRouter,
) -> (Arc<crate::session::session::Session>, ToolCallRuntime) {
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    (session, runtime)
}

fn output_call_ids(outputs: &[ResponseInputItem]) -> Vec<&str> {
    outputs
        .iter()
        .filter_map(|output| match output {
            ResponseInputItem::FunctionCallOutput { call_id, .. }
            | ResponseInputItem::CustomToolCallOutput { call_id, .. }
            | ResponseInputItem::McpToolCallOutput { call_id, .. }
            | ResponseInputItem::ToolSearchOutput { call_id, .. } => Some(call_id.as_str()),
            ResponseInputItem::Message { .. } => None,
        })
        .collect()
}

#[tokio::test]
async fn ready_frontier_reuses_one_scheduler_for_client_and_hosted_adapters() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let router = recording_router(Arc::clone(&events), Arc::new(Barrier::new(3)));
    let (session, runtime) = test_runtime(router).await;
    let mut calls = vec![initialize_work_batch(false)];
    calls.extend(work_calls());

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("ready frontier execution");

    assert_eq!(
        output_call_ids(&outcome.outputs),
        vec![
            "map-prelude",
            "call-local-a",
            "call-hosted-b",
            "call-local-c"
        ]
    );
    let events = events.lock().await;
    assert_eq!(events.len(), 6);
    let started = events[..3].iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        started,
        BTreeSet::from([
            "client:mvt_client_a:started".to_string(),
            "client:mvt_client_c:started".to_string(),
            "provider_hosted:mvt_hosted_b:started".to_string(),
        ])
    );
    assert!(
        events[3..]
            .iter()
            .all(|event| event.ends_with(":completed"))
    );
    drop(events);

    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("snapshot")
        .map
        .expect("initialized map");
    assert!(map.reservations.is_empty());
    assert_eq!(
        map.results
            .iter()
            .map(|result| result.node_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["hosted-b", "local-a", "local-c"])
    );
    assert!(map.results.iter().all(|result| !result.is_error));
}

#[tokio::test]
async fn dependent_successors_in_one_batch_are_rejected_before_any_adapter_runs() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let router = recording_router(Arc::clone(&events), Arc::new(Barrier::new(3)));
    let (session, runtime) = test_runtime(router).await;
    let mut calls = vec![initialize_work_batch(true)];
    calls.extend(work_calls());

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("dependency rejection response");

    assert!(events.lock().await.is_empty());
    assert!(
        session
            .canonical_action_map_snapshot()
            .await
            .expect("snapshot")
            .map
            .is_none(),
        "rejected initialization must not commit a partial Map"
    );
    assert_eq!(
        output_call_ids(&outcome.outputs),
        vec![
            "map-prelude",
            "call-local-a",
            "call-hosted-b",
            "call-local-c"
        ]
    );
    assert!(matches!(
        outcome.outputs.last(),
        Some(ResponseInputItem::Message { .. })
    ));
}
