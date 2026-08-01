use std::collections::HashSet;
use std::sync::Arc;

use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::SearchToolCallParams;
use codex_protocol::protocol::MapRuntimeMode;
use codex_tools::ToolName;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::execute_response_tool_sequence;
use crate::session::tests::make_session_and_context;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use crate::tools::router::ToolRouterParams;
use crate::turn_diff_tracker::TurnDiffTracker;

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

fn tool_search_call(call_id: &str, query: &str) -> ToolCall {
    ToolCall {
        provider_tool_name: ToolName::plain("tool_search"),
        dispatch_tool_name: ToolName::plain("tool_search"),
        call_id: call_id.to_string(),
        payload: ToolPayload::ToolSearch {
            arguments: SearchToolCallParams {
                query: query.to_string(),
                limit: None,
            },
        },
    }
}

fn router_for_turn(turn: &crate::session::turn_context::TurnContext) -> ToolRouter {
    ToolRouter::from_config(
        &turn.tools_config,
        ToolRouterParams {
            deferred_mcp_tools: None,
            mcp_tools: None,
            unavailable_called_tools: Vec::new(),
            parallel_mcp_server_names: HashSet::new(),
            discoverable_tools: None,
            dynamic_tools: turn.dynamic_tools.as_slice(),
        },
    )
}

fn factual_message(outputs: &[ResponseInputItem]) -> serde_json::Value {
    let ResponseInputItem::Message { content, .. } =
        outputs.last().expect("response-level factual message")
    else {
        panic!("response-level factual message must be last");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            ContentItem::InputText { text } => Some(text),
            _ => None,
        })
        .expect("factual JSON");
    serde_json::from_str(text).expect("factual message JSON")
}

#[tokio::test]
async fn production_entry_rejects_complete_then_tool_search_before_dispatch() {
    let (session, mut turn) = make_session_and_context().await;
    turn.tools_config
        .experimental_supported_tools
        .push("test_sync_tool".to_string());
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router_for_turn(&turn)),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );

    execute_response_tool_sequence(
        runtime.clone(),
        vec![
            function_call(
                "taskspace_control",
                "initialize-control",
                serde_json::json!({
                    "action": "initialize_and_execute",
                    "root": {"node_id": "root", "goal": "Repair"},
                    "work_nodes": [{"node_id": "verify", "goal": "Verify"}],
                    "finish": {"node_id": "finish", "goal": "Close"},
                    "edges": [
                        {"from": "root", "to": "verify"},
                        {"from": "verify", "to": "finish"}
                    ],
                    "actions": [{"node_id": "verify", "tool": "test_sync_tool"}],
                })
                .to_string(),
            ),
            function_call("test_sync_tool", "initial-verify", "{}"),
        ],
        CancellationToken::new(),
    )
    .await
    .expect("initialize a ready work node");
    let before = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map");
    let before_revision = before.revision;
    let before_results = before.results.clone();
    assert_eq!(
        before
            .nodes
            .iter()
            .find(|node| node.id == "verify")
            .unwrap()
            .state,
        "ready"
    );

    let control = function_call(
        "taskspace_control",
        "reject-control",
        serde_json::json!({
            "action": "execute",
            "expected_revision": before_revision,
            "mutations": [{"action": "complete_node", "node_id": "verify"}],
            "actions": [{"node_id": "verify", "tool": "tool_search"}],
        })
        .to_string(),
    );
    let mut poisoned_search = tool_search_call("reject-search", "read_file");
    poisoned_search.dispatch_tool_name = ToolName::plain("exec_command");
    let outcome = execute_response_tool_sequence(
        runtime,
        vec![control, poisoned_search],
        CancellationToken::new(),
    )
    .await
    .expect("state rejection must happen before the poisoned dispatch path");

    assert_eq!(outcome.outputs.len(), 3);
    assert!(matches!(
        &outcome.outputs[1],
        ResponseInputItem::ToolSearchOutput {
            call_id,
            status,
            tools,
            ..
        } if call_id == "reject-search" && status == "completed" && tools.is_empty()
    ));
    let failure = factual_message(&outcome.outputs);
    assert_eq!(failure["state_commit"], false);
    assert_eq!(failure["executed_tool_call_count"], 0);
    assert_eq!(
        failure["failure_provenance"]["affected_call_ids"],
        serde_json::json!(["reject-control", "reject-search"])
    );
    let violation = &failure["error"]["violations"][0];
    assert_eq!(violation["canonical_before_transaction"]["state"], "ready");
    assert_eq!(
        violation["rejected_candidate_at_violation"]["state"],
        "completed"
    );

    let after = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot after rejection")
        .map
        .expect("map remains initialized");
    assert_eq!(after.revision, before_revision);
    assert_eq!(after.results, before_results);
    assert!(after.reservations.is_empty());
    assert_eq!(
        after
            .nodes
            .iter()
            .find(|node| node.id == "verify")
            .unwrap()
            .state,
        "ready"
    );
}
