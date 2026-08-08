use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::*;
use crate::function_tool::FunctionCallError;
use crate::session::tests::attach_thread_persistence;
use crate::session::tests::make_session_and_context;
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
        description: "Return persisted test evidence.".into(),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::new(),
            None,
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
        defer_loading: None,
    })
}

struct InspectHandler {
    calls: AtomicUsize,
}

impl ToolHandler for InspectHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, _invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(FunctionToolOutput::from_text(
            format!(
                "{}PERSISTED_EXEC_SENTINEL{}",
                "A".repeat(32_000),
                "B".repeat(32_000)
            ),
            Some(true),
        ))
    }
}

#[tokio::test]
async fn persisted_exec_reaches_map_rollout_and_provider_preparation() {
    let home = std::env::temp_dir().join(format!("taskspace-exec-chain-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".into())
        .await
        .expect("initialize state DB");
    let (mut session, turn) = make_session_and_context().await;
    session.services.state_db = Some(Arc::clone(&state_db));
    let rollout_path = attach_thread_persistence(&mut session).await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    session.start_taskspace_action_settlements_for_test();
    let (activation, _) = session
        .set_persisted_action_map_mode(MapRuntimeMode::Experiment)
        .await
        .expect("activate persisted TaskSpace");
    let map_id = activation.active_map_id.expect("active Map identity");

    let native_handler = Arc::new(InspectHandler {
        calls: AtomicUsize::new(0),
    });
    let mut builder = ToolRegistryBuilder::new();
    builder.push_spec_with_parallel_support(inspect_spec(), true);
    builder.register_handler("inspect", Arc::clone(&native_handler));
    let router = Arc::new(
        ToolRouter::from_builder_for_test(builder)
            .into_taskspace()
            .expect("build production TaskSpace router"),
    );
    let response_scope = router.taskspace_response_scope().expect("response scope");
    response_scope.begin_request(map_id.clone(), None).unwrap();
    response_scope.record_completed_item(
        Some(0),
        &ResponseItem::FunctionCall {
            id: None,
            name: TASKSPACE_EXEC_TOOL_NAME.into(),
            namespace: None,
            arguments: "{}".into(),
            call_id: "outer".into(),
        },
    );
    response_scope.finalize(true).unwrap();
    let output = router
        .dispatch_tool_call_with_code_mode_result(
            Arc::clone(&session),
            Arc::clone(&turn),
            CancellationToken::new(),
            Arc::new(Mutex::new(TurnDiffTracker::new())),
            ToolCall {
                provider_tool_name: ToolName::plain(TASKSPACE_EXEC_TOOL_NAME),
                dispatch_tool_name: ToolName::plain(TASKSPACE_EXEC_TOOL_NAME),
                call_id: "outer".into(),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "calls": [
                            {
                                "tool": "initialize_map",
                                "arguments": {
                                    "root": {"node_id": "root", "goal": "deliver", "content": "", "parents": []},
                                    "work_nodes": [{"node_id": "work", "goal": "inspect", "content": "", "parents": ["root"]}],
                                    "finish": {"node_id": "finish", "goal": "close", "content": "", "parents": ["work"]}
                                }
                            },
                            {"tool": "inspect", "node_id": "work", "arguments": {}}
                        ],
                        "hosted_bindings": []
                    })
                    .to_string(),
                },
            },
            ToolCallSource::Direct,
        )
        .await
        .expect("execute persisted TaskSpace batch")
        .into_response();
    assert_eq!(native_handler.calls.load(Ordering::SeqCst), 1);
    session
        .await_taskspace_action_settlements()
        .await
        .expect("settle persisted Tool result");
    let Some(ResponseItem::FunctionCallOutput { output, .. }) =
        crate::stream_events_utils::response_input_to_response_item(&output)
    else {
        panic!("TaskSpace router must return FunctionCallOutput")
    };
    session
        .record_conversation_items(
            turn.as_ref(),
            &[ResponseItem::FunctionCallOutput {
                call_id: "outer".into(),
                output,
            }],
        )
        .await;
    session.flush_rollout().await.expect("flush rollout");

    let stored = state_db
        .load_taskspace_map(&map_id)
        .await
        .expect("load Map")
        .expect("persisted Map")
        .canonical_map
        .expect("canonical Map");
    let work = stored
        .work_nodes
        .iter()
        .find(|node| node.node_id == "work")
        .expect("work node");
    assert_eq!(work.actions.len(), 1);
    assert_eq!(work.actions[0].outcome, TaskSpaceActionOutcome::Succeeded);

    let history = session.clone_history().await;
    let prepared = session
        .prepare_provider_visible_prompt_items(turn.as_ref(), history.raw_items().to_vec())
        .await
        .expect("prepare provider request after settlement");
    assert_eq!(
        prepared.taskspace_request_map.expect("request Map").map_id,
        map_id
    );
    assert!(prepared.projection_identity.is_some());

    let rollout = tokio::fs::read_to_string(rollout_path)
        .await
        .expect("read rollout");
    assert!(rollout.contains("OutputReferenceV1"));
    assert!(rollout.contains("output-ref://sha256/"));
    assert!(!rollout.contains("PERSISTED_EXEC_SENTINEL"));
    let _ = tokio::fs::remove_dir_all(home).await;
}
