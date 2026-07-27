use std::collections::HashSet;
use std::sync::Arc;

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
        tool_name: ToolName::plain(name),
        call_id: call_id.to_string(),
        payload: ToolPayload::Function {
            arguments: arguments.into(),
        },
    }
}

fn initialize_two_actions() -> ToolCall {
    function_call(
        "taskspace_control",
        "control",
        serde_json::json!({
            "action": "initialize_and_execute",
            "root": {"node_id": "root", "goal": "Repair the defect"},
            "work_nodes": [
                {"node_id": "edit", "goal": "Apply the repair"},
                {"node_id": "verify", "goal": "Verify the repair"}
            ],
            "finish": {"node_id": "finish", "goal": "Close the task"},
            "edges": [
                {"from": "root", "to": "edit"},
                {"from": "root", "to": "verify"},
                {"from": "edit", "to": "finish"},
                {"from": "verify", "to": "finish"}
            ],
            "actions": [
                {"node_id": "edit", "tool": "apply_patch"},
                {"node_id": "verify", "tool": "exec_command"}
            ]
        })
        .to_string(),
    )
}

#[tokio::test]
async fn prior_failure_releases_every_prepared_taskspace_reservation() {
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = ToolRouter::from_config(
        &turn.tools_config,
        ToolRouterParams {
            deferred_mcp_tools: None,
            mcp_tools: None,
            unavailable_called_tools: Vec::new(),
            parallel_mcp_server_names: HashSet::new(),
            discoverable_tools: None,
            dynamic_tools: turn.dynamic_tools.as_slice(),
        },
    );
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    let calls = vec![
        initialize_two_actions(),
        function_call("apply_patch", "patch", "{}"),
        function_call(
            "exec_command",
            "verify",
            serde_json::json!({"cmd": "true"}).to_string(),
        ),
    ];

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("response sequence");

    assert_eq!(outcome.outputs.len(), 3);
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map");
    assert!(
        map.reservations.is_empty(),
        "failed and skipped actions must both release their reservations: {:?}",
        map.reservations
    );
    assert_eq!(map.results.len(), 2);
    assert!(
        map.results
            .iter()
            .any(|result| result.id == "tool-result://call/verify" && result.is_error),
        "the skipped action must be attributed as a failed result"
    );
}
