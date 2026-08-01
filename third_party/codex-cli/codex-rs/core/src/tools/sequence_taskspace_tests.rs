use std::collections::HashSet;
use std::sync::Arc;

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

fn initialize_actions(actions: &[(&str, &str)]) -> ToolCall {
    let work_nodes = actions
        .iter()
        .map(|(node_id, _)| {
            serde_json::json!({
                "node_id": node_id,
                "goal": format!("Run the {node_id} action"),
            })
        })
        .collect::<Vec<_>>();
    let mut edges = actions
        .iter()
        .map(|(node_id, _)| serde_json::json!({"from": "root", "to": node_id}))
        .collect::<Vec<_>>();
    edges.extend(
        actions
            .iter()
            .map(|(node_id, _)| serde_json::json!({"from": node_id, "to": "finish"})),
    );
    let declared_actions = actions
        .iter()
        .map(|(node_id, tool)| serde_json::json!({"node_id": node_id, "tool": tool}))
        .collect::<Vec<_>>();
    function_call(
        "taskspace_control",
        "control",
        serde_json::json!({
            "action": "initialize_and_execute",
            "root": {"node_id": "root", "goal": "Repair the defect"},
            "work_nodes": work_nodes,
            "finish": {"node_id": "finish", "goal": "Close the task"},
            "edges": edges,
            "actions": declared_actions,
        })
        .to_string(),
    )
}

fn initialize_two_actions() -> ToolCall {
    initialize_actions(&[("edit", "apply_patch"), ("verify", "exec_command")])
}

fn execute_actions(expected_revision: u64, actions: &[(&str, &str)]) -> ToolCall {
    function_call(
        "taskspace_control",
        "control-next",
        serde_json::json!({
            "action": "execute",
            "expected_revision": expected_revision,
            "actions": actions
                .iter()
                .map(|(node_id, tool)| serde_json::json!({
                    "node_id": node_id,
                    "tool": tool,
                }))
                .collect::<Vec<_>>(),
        })
        .to_string(),
    )
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

fn function_call_output(
    outputs: &[codex_protocol::models::ResponseInputItem],
    expected_call_id: &str,
) -> serde_json::Value {
    let output = outputs
        .iter()
        .find_map(|item| match item {
            codex_protocol::models::ResponseInputItem::FunctionCallOutput { call_id, output }
                if call_id == expected_call_id =>
            {
                Some(output)
            }
            _ => None,
        })
        .expect("function call output");
    let codex_protocol::models::FunctionCallOutputBody::Text(text) = &output.body else {
        panic!("TaskSpace control output must be JSON text");
    };
    serde_json::from_str(text).expect("function call output JSON")
}

#[tokio::test]
async fn response_exposes_one_final_control_revision_authority() {
    let (session, mut turn) = make_session_and_context().await;
    turn.tools_config
        .experimental_supported_tools
        .push("test_sync_tool".to_string());
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );

    let outcome = execute_response_tool_sequence(
        runtime,
        vec![
            initialize_actions(&[("work", "test_sync_tool")]),
            function_call("test_sync_tool", "initial-work", "{}"),
        ],
        CancellationToken::new(),
    )
    .await
    .expect("initialize response");

    let result = function_call_output(&outcome.outputs, "control");
    let final_revision = result["canonical_revision"]
        .as_u64()
        .expect("final canonical revision");
    let map_revision = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map")
        .revision;

    assert_eq!(result["schema_version"], "TaskSpaceResponseResultV2");
    assert_eq!(result["status"], "settled");
    assert_eq!(final_revision, map_revision);
    assert!(result.get("revision_after").is_none());
    assert!(
        outcome.outputs.iter().all(|output| !matches!(
            output,
            codex_protocol::models::ResponseInputItem::Message { .. }
        )),
        "the response must not expose a second developer-message authority"
    );
}

#[tokio::test]
async fn response_final_control_revision_is_accepted_by_the_next_execute() {
    let (session, mut turn) = make_session_and_context().await;
    turn.tools_config
        .experimental_supported_tools
        .push("test_sync_tool".to_string());
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );

    let initialized = execute_response_tool_sequence(
        runtime.clone(),
        vec![
            initialize_actions(&[("work", "test_sync_tool")]),
            function_call("test_sync_tool", "initial-work", "{}"),
        ],
        CancellationToken::new(),
    )
    .await
    .expect("initialize response");
    let initial_result = function_call_output(&initialized.outputs, "control");
    let final_revision = initial_result["canonical_revision"]
        .as_u64()
        .expect("canonical revision");
    assert_eq!(initial_result["status"], "settled");

    let continued = execute_response_tool_sequence(
        runtime,
        vec![
            execute_actions(final_revision, &[("work", "test_sync_tool")]),
            function_call("test_sync_tool", "continued-work", "{}"),
        ],
        CancellationToken::new(),
    )
    .await
    .expect("next execute response");
    let continued_result = function_call_output(&continued.outputs, "control-next");
    assert_eq!(continued_result["status"], "settled");
    assert!(
        continued_result["canonical_revision"]
            .as_u64()
            .expect("continued revision")
            > final_revision
    );
}

#[tokio::test]
async fn prior_failure_releases_every_prepared_taskspace_reservation() {
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
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

    assert_eq!(outcome.outputs.len(), 4);
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
    let result = function_call_output(&outcome.outputs, "control");
    assert_eq!(result["schema_version"], "TaskSpaceResponseResultV2");
    assert_eq!(result["status"], "settled");
    assert_eq!(result["canonical_revision"], map.revision);
    assert_eq!(result["settlement"]["prepared_action_count"], 2);
    assert_eq!(result["settlement"]["attributed_result_count"], 2);
    assert_eq!(result["settlement"]["outstanding_reservation_count"], 0);
}

#[tokio::test]
async fn cancelled_parallel_actions_release_every_prepared_reservation() {
    let (session, mut turn) = make_session_and_context().await;
    turn.tools_config
        .experimental_supported_tools
        .push("test_sync_tool".to_string());
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    let calls = vec![
        initialize_actions(&[("left", "test_sync_tool"), ("right", "test_sync_tool")]),
        function_call(
            "test_sync_tool",
            "left",
            serde_json::json!({"sleep_before_ms": 10_000}).to_string(),
        ),
        function_call(
            "test_sync_tool",
            "right",
            serde_json::json!({"sleep_before_ms": 10_000}).to_string(),
        ),
    ];
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let outcome = execute_response_tool_sequence(runtime, calls, cancellation)
        .await
        .expect("cancelled response sequence");

    assert_eq!(outcome.outputs.len(), 3);
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map");
    assert!(map.reservations.is_empty());
    assert_eq!(map.results.len(), 2);
    assert!(map.results.iter().all(|result| result.is_error));
    let result = function_call_output(&outcome.outputs, "control");
    assert_eq!(result["status"], "settled");
    assert_eq!(result["canonical_revision"], map.revision);
}

#[tokio::test]
async fn parallel_tool_timeouts_release_every_prepared_reservation() {
    let (session, mut turn) = make_session_and_context().await;
    turn.tools_config
        .experimental_supported_tools
        .push("test_sync_tool".to_string());
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    let timeout_args = serde_json::json!({
        "barrier": {
            "id": "taskspace-parallel-timeout",
            "participants": 3,
            "timeout_ms": 10
        }
    })
    .to_string();
    let calls = vec![
        initialize_actions(&[("left", "test_sync_tool"), ("right", "test_sync_tool")]),
        function_call("test_sync_tool", "left", timeout_args.clone()),
        function_call("test_sync_tool", "right", timeout_args),
    ];

    let outcome = execute_response_tool_sequence(runtime, calls, CancellationToken::new())
        .await
        .expect("timed out response sequence");

    assert_eq!(outcome.outputs.len(), 3);
    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map");
    assert!(map.reservations.is_empty());
    assert_eq!(map.results.len(), 2);
    assert!(map.results.iter().all(|result| result.is_error));
    let result = function_call_output(&outcome.outputs, "control");
    assert_eq!(result["status"], "settled");
    assert_eq!(result["canonical_revision"], map.revision);
}

#[tokio::test]
async fn fatal_dispatch_error_releases_the_prepared_reservation() {
    let (session, turn) = make_session_and_context().await;
    session
        .set_action_map_mode_for_test(MapRuntimeMode::Experiment)
        .await;
    let router = router_for_turn(&turn);
    let session = Arc::new(session);
    let runtime = ToolCallRuntime::new(
        Arc::new(router),
        Arc::clone(&session),
        Arc::new(turn),
        Arc::new(Mutex::new(TurnDiffTracker::new())),
    );
    let malformed_exec = ToolCall {
        provider_tool_name: ToolName::plain("exec_command"),
        dispatch_tool_name: ToolName::plain("exec_command"),
        call_id: "fatal".to_string(),
        payload: ToolPayload::ToolSearch {
            arguments: SearchToolCallParams {
                query: "force incompatible handler payload".to_string(),
                limit: None,
            },
        },
    };
    let calls = vec![
        initialize_actions(&[("work", "exec_command")]),
        malformed_exec,
    ];

    let error = match execute_response_tool_sequence(runtime, calls, CancellationToken::new()).await
    {
        Ok(_) => panic!("incompatible native payload must remain fatal"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("incompatible payload"));

    let map = session
        .canonical_action_map_snapshot()
        .await
        .expect("canonical snapshot")
        .map
        .expect("initialized map");
    assert!(
        map.reservations.is_empty(),
        "fatal dispatch must not strand a prepared reservation: {:?}",
        map.reservations
    );
    assert!(
        map.results
            .iter()
            .any(|result| result.id == "tool-result://call/fatal" && result.is_error)
    );
}
