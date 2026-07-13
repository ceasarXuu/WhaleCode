use anyhow::Context;
use anyhow::Result;
use codex_features::Feature;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::Op;
use core_test_support::responses;
use core_test_support::responses::ev_apply_patch_custom_tool_call;
use core_test_support::responses::ev_apply_patch_function_call;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::ev_shell_command_call;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::TestCodexHarness;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::time::sleep;
use wiremock::Request;

const USER_PROMPT: &str = "请用一个子 agent 调查缓存模块边界，然后继续推进。";
const CHILD_PROMPT: &str = "调查缓存模块边界";
const CREATE_NODE_CALL_ID: &str = "create-map-node";
const SPAWN_CALL_ID: &str = "create-map-node:nested:0";

const REALISTIC_USER_PROMPT: &str =
    "这个沙盒项目有一个缓存 key 相关的回归失败。请先让子 agent 调查边界，再修复代码并验证。";
const REALISTIC_CHILD_PROMPT: &str = "调查缓存 key 失败边界，阅读 src/cache.py 和测试文件。";
const REALISTIC_CREATE_NODE_CALL_ID: &str = "create-cache-scope-node";
const REALISTIC_SPAWN_CALL_ID: &str = "create-cache-scope-node:nested:0";
const REALISTIC_CHILD_READ_CALL_ID: &str = "child-read-cache-files";
const REALISTIC_WAIT_CALL_ID: &str = "parent-wait-cache-scope-agent";
const REALISTIC_IMPLEMENT_NODE_CALL_ID: &str = "create-cache-fix-node";
const REALISTIC_PATCH_CALL_ID: &str = "apply-cache-fix";
const REALISTIC_TEST_CALL_ID: &str = "parent-run-cache-validation";
const REALISTIC_FINISH_NODE_CALL_ID: &str = "parent-finish-cache-fix-node";

const ORDERED_SEQUENCE_PROMPT: &str = "按固定三个节点修复 value 文件并验证。";
const INIT_BARRIER_CALL_ID: &str = "ordered-init";
const READ_AFTER_INIT_CALL_ID: &str = "ordered-init:nested:0";
const FINISH_INSPECT_CALL_ID: &str = "ordered-finish-inspect";
const EDIT_AFTER_FINISH_CALL_ID: &str = "ordered-edit-after-finish";
const FINISH_IMPLEMENT_CALL_ID: &str = "ordered-finish-implement";
const TEST_AFTER_FINISH_CALL_ID: &str = "ordered-test-after-finish";
const TRAILING_FINISH_PROMPT: &str = "验证非终态 finish 在同一调用中继续执行。";
const CHAINED_FINISH_PROMPT: &str = "验证同一响应连续完成两个节点。";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_candidate_finishes_turn_without_extra_provider_request() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::new().await?;
    harness
        .write_file("README.md", "Terminal fixture.\n")
        .await?;
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [{
            "node_id": "inspect",
            "kind": "inspect_code_context",
            "goal": "Read README."
        }],
        "current_node_id": "inspect",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "cat README.md"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "验证终态候选"),
        sse(vec![
            ev_response_created("terminal-response-1"),
            ev_function_call("terminal-init", "taskspace_control", &initialize),
            ev_completed("terminal-response-1"),
        ]),
    )
    .await;

    let exact_final = "Agent final line one.\nAgent final line two.";
    let finish = serde_json::to_string(&json!({
        "action": "finish_then_end",
        "finish_node_ids": ["inspect"],
        "final_candidate": exact_final
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "terminal-init:nested:0"),
        sse(vec![
            ev_response_created("terminal-response-2"),
            ev_function_call("terminal-finish", "taskspace_control", &finish),
            ev_completed("terminal-response-2"),
        ]),
    )
    .await;

    enable_taskspace(&harness).await?;
    harness.submit("验证终态候选").await?;

    let requests = harness.request_bodies().await;
    assert_eq!(requests.len(), 2, "terminal finish must not resample");
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert_eq!(snapshot.maps[0].nodes.len(), 1);
    assert_eq!(snapshot.maps[0].nodes[0].status, "completed");
    let rollout_path = harness
        .test()
        .codex
        .rollout_path()
        .context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, "Agent final line one.").await?;
    assert!(rollout.contains("\"phase\":\"final_answer\""));
    assert!(rollout.contains("Agent final line one.\\nAgent final line two."));
    assert!(rollout.contains("terminal_transition"));
    assert!(rollout.contains("finished_node_id"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plain_final_with_open_map_finishes_turn_without_resampling() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::new().await?;
    harness
        .write_file("README.md", "Open map fixture.\n")
        .await?;
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [{
            "node_id": "inspect",
            "kind": "inspect_code_context",
            "goal": "Read README but leave the node open."
        }],
        "current_node_id": "inspect",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "cat README.md"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "验证开放Map的普通最终回答"),
        sse(vec![
            ev_response_created("open-final-response-1"),
            ev_function_call("open-final-init", "taskspace_control", &initialize),
            ev_completed("open-final-response-1"),
        ]),
    )
    .await;

    let plain_final = "The requested inspection is complete.";
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "open-final-init:nested:0"),
        sse(vec![
            ev_response_created("open-final-response-2"),
            ev_assistant_message("open-final-message", plain_final),
            ev_completed("open-final-response-2"),
        ]),
    )
    .await;

    enable_taskspace(&harness).await?;
    harness.submit("验证开放Map的普通最终回答").await?;

    let requests = harness.request_bodies().await;
    assert_eq!(
        requests.len(),
        2,
        "plain final must end the turn without a recovery request"
    );
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert_eq!(snapshot.maps[0].nodes[0].status, "running");
    assert_eq!(snapshot.maps[0].status, "active");
    assert_eq!(snapshot.tasks[0].status, "active");

    let rollout_path = harness
        .test()
        .codex
        .rollout_path()
        .context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, plain_final).await?;
    assert!(!rollout.contains("TaskSpaceFinalAnswerRejectedV1"));
    assert!(!rollout.contains("final_rejected"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nonterminal_finish_executes_sibling_action_after_barrier() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::new().await?;
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [
            {
                "node_id": "inspect",
                "kind": "inspect_code_context",
                "goal": "Exercise cadence observation."
            },
            {
                "node_id": "complete",
                "kind": "final_synthesis",
                "goal": "Finish after the standalone transition.",
                "dependency_node_ids": ["inspect"]
            }
        ],
        "current_node_id": "inspect",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "pwd"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, TRAILING_FINISH_PROMPT),
        sse(vec![
            ev_response_created("cadence-response-1"),
            ev_function_call("cadence-init", "taskspace_control", &initialize),
            ev_completed("cadence-response-1"),
        ]),
    )
    .await;

    let nonterminal_finish = serde_json::to_string(&json!({
        "action": "finish_nodes",
        "finishes": [{
            "node_id": "inspect",
            "next": {"kind": "existing", "node_id": "complete"},
        }]
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "cadence-init"),
        sse(vec![
            ev_response_created("cadence-response-2"),
            ev_function_call(
                "cadence-trailing-finish",
                "taskspace_control",
                &nonterminal_finish,
            ),
            ev_shell_command_call("cadence-after-finish", "pwd"),
            ev_completed("cadence-response-2"),
        ]),
    )
    .await;

    let terminal_finish = serde_json::to_string(&json!({
        "action": "finish_then_end",
        "finish_node_ids": ["complete"],
        "final_candidate": "Cadence ownership verified."
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "cadence-after-finish"),
        sse(vec![
            ev_response_created("cadence-response-3"),
            ev_function_call(
                "cadence-terminal-finish",
                "taskspace_control",
                &terminal_finish,
            ),
            ev_completed("cadence-response-3"),
        ]),
    )
    .await;

    enable_taskspace(&harness).await?;
    harness.submit(TRAILING_FINISH_PROMPT).await?;

    let requests = harness.request_bodies().await;
    assert_eq!(requests.len(), 3);
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert_eq!(snapshot.maps[0].nodes.len(), 2);
    assert_eq!(snapshot.maps[0].nodes[0].status, "completed");
    assert_eq!(snapshot.maps[0].nodes[1].status, "completed");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adjacent_finish_calls_claim_successive_ready_targets() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::new().await?;
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [
            {
                "node_id": "first",
                "kind": "inspect_code_context",
                "goal": "Record the first result."
            },
            {
                "node_id": "second",
                "kind": "final_synthesis",
                "goal": "Record the second result.",
                "dependency_node_ids": ["first"]
            }
        ],
        "current_node_id": "first",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "pwd"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, CHAINED_FINISH_PROMPT),
        sse(vec![
            ev_response_created("chained-response-1"),
            ev_function_call("chained-init", "taskspace_control", &initialize),
            ev_completed("chained-response-1"),
        ]),
    )
    .await;

    let finish_second = serde_json::to_string(&json!({
        "action": "finish_then_end",
        "finish_node_ids": ["first", "second"],
        "final_candidate": "Both nodes finished in one response."
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "chained-init"),
        sse(vec![
            ev_response_created("chained-response-2"),
            ev_function_call("chained-finish-second", "taskspace_control", &finish_second),
            ev_completed("chained-response-2"),
        ]),
    )
    .await;

    enable_taskspace(&harness).await?;
    harness.submit(CHAINED_FINISH_PROMPT).await?;

    let requests = harness.request_bodies().await;
    assert_eq!(requests.len(), 2, "adjacent finishes must not resample");
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert_eq!(snapshot.maps[0].nodes.len(), 2);
    assert!(
        snapshot.maps[0]
            .nodes
            .iter()
            .all(|node| node.status == "completed")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_sequence_executes_dependent_tools_after_latest_state_barrier() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
    });
    let harness = TestCodexHarness::with_builder(builder).await?;
    harness
        .write_file("README.md", "Replace old with new and validate it.\n")
        .await?;
    harness.write_file("src/value.txt", "old\n").await?;

    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [
            {
                "node_id": "inspect",
                "kind": "inspect_code_context",
                "goal": "Read the fixture instructions."
            },
            {
                "node_id": "implement",
                "kind": "implement_solution",
                "goal": "Apply the required edit.",
                "dependency_node_ids": ["inspect"]
            },
            {
                "node_id": "validate",
                "kind": "smoke_test",
                "goal": "Run the fixture validation.",
                "dependency_node_ids": ["implement"]
            }
        ],
        "current_node_id": "inspect",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "cat README.md"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, ORDERED_SEQUENCE_PROMPT),
        sse(vec![
            ev_response_created("ordered-response-1"),
            ev_function_call(INIT_BARRIER_CALL_ID, "taskspace_control", &initialize),
            ev_completed("ordered-response-1"),
        ]),
    )
    .await;

    let patch = "*** Begin Patch\n*** Update File: src/value.txt\n@@\n-old\n+new\n*** End Patch";
    let finish_inspect = serde_json::to_string(&json!({
        "action": "finish_nodes",
        "finishes": [{
            "node_id": "inspect",
            "next": {"kind": "existing", "node_id": "implement"}
        }]
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, READ_AFTER_INIT_CALL_ID),
        sse(vec![
            ev_response_created("ordered-response-2"),
            ev_function_call(FINISH_INSPECT_CALL_ID, "taskspace_control", &finish_inspect),
            ev_apply_patch_function_call(EDIT_AFTER_FINISH_CALL_ID, patch),
            ev_completed("ordered-response-2"),
        ]),
    )
    .await;

    let finish_implement = serde_json::to_string(&json!({
        "action": "finish_nodes",
        "finishes": [{
            "node_id": "implement",
            "next": {"kind": "existing", "node_id": "validate"}
        }]
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, EDIT_AFTER_FINISH_CALL_ID),
        sse(vec![
            ev_response_created("ordered-response-3"),
            ev_function_call(
                FINISH_IMPLEMENT_CALL_ID,
                "taskspace_control",
                &finish_implement,
            ),
            ev_shell_command_call(
                TEST_AFTER_FINISH_CALL_ID,
                "grep -q '^new$' src/value.txt && echo validation-passed",
            ),
            ev_completed("ordered-response-3"),
        ]),
    )
    .await;

    let finish_validate = serde_json::to_string(&json!({
        "action": "finish_then_end",
        "finish_node_ids": ["validate"],
        "final_candidate": "Fixture fixed and validated."
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, TEST_AFTER_FINISH_CALL_ID),
        sse(vec![
            ev_response_created("ordered-response-4"),
            ev_function_call(
                "ordered-finish-validate",
                "taskspace_control",
                &finish_validate,
            ),
            ev_completed("ordered-response-4"),
        ]),
    )
    .await;
    enable_taskspace(&harness).await?;
    harness.submit(ORDERED_SEQUENCE_PROMPT).await?;

    let requests = harness.request_bodies().await;
    assert_eq!(
        harness.read_file_text("src/value.txt").await?,
        "new\n",
        "sibling patch did not run after the state barrier; requests: {requests:#?}"
    );
    assert!(requests.iter().any(|request| {
        let text = request.to_string();
        text.contains(TEST_AFTER_FINISH_CALL_ID) && text.contains("validation-passed")
    }));
    assert!(
        requests
            .iter()
            .skip(1)
            .any(|request| request.to_string().contains("created_node_ids")),
        "initialization identities did not enter the next provider request"
    );
    assert!(
        requests
            .iter()
            .skip(2)
            .any(|request| request.to_string().contains("finished_node_id")),
        "finish identities did not enter a later provider request"
    );
    for call_id in [
        INIT_BARRIER_CALL_ID,
        READ_AFTER_INIT_CALL_ID,
        FINISH_INSPECT_CALL_ID,
        EDIT_AFTER_FINISH_CALL_ID,
        FINISH_IMPLEMENT_CALL_ID,
        TEST_AFTER_FINISH_CALL_ID,
    ] {
        assert!(
            requests
                .iter()
                .any(|request| request.to_string().contains(call_id)),
            "missing ordered call/output {call_id}"
        );
    }
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert_eq!(snapshot.maps[0].nodes.len(), 3);
    assert!(
        snapshot.maps[0]
            .nodes
            .iter()
            .all(|node| node.status == "completed")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_multi_patch_preflight_prevents_control_and_file_side_effects() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
    });
    let harness = TestCodexHarness::with_builder(builder).await?;
    let nested_patch = "*** Begin Patch\n*** Add File: nested.txt\n+nested\n*** End Patch";
    let top_patch = "*** Begin Patch\n*** Add File: top.txt\n+top\n*** End Patch";
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [{
            "node_id": "implement",
            "kind": "implement_solution",
            "goal": "Apply one patch."
        }],
        "current_node_id": "implement",
        "continuation": {
            "kind": "patch_then_actions",
            "patch": {"tool_name": "apply_patch", "input": nested_patch}
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "验证多patch预检"),
        sse(vec![
            ev_response_created("multi-patch-map-response"),
            ev_function_call("multi-patch-bootstrap", "taskspace_control", &initialize),
            ev_apply_patch_custom_tool_call("multi-patch-top", top_patch),
            ev_completed("multi-patch-map-response"),
        ]),
    )
    .await;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "request_multiple_apply_patch_calls_not_allowed"),
        sse(vec![
            ev_assistant_message("multi-patch-observed", "preflight observed"),
            ev_completed("multi-patch-observed-response"),
        ]),
    )
    .await;

    enable_taskspace(&harness).await?;
    harness.submit("验证多patch预检").await?;

    assert!(!harness.path("nested.txt").exists());
    assert!(!harness.path("top.txt").exists());
    let snapshot = harness.test().codex.action_map_snapshot().await;
    assert!(snapshot.maps.iter().all(|map| {
        map.nodes.is_empty()
            && map.edges.is_empty()
            && map.results.is_empty()
            && map.leases.is_empty()
    }));
    let requests = harness.request_bodies().await;
    assert_eq!(requests.len(), 2);
    let feedback = requests[1].to_string();
    assert!(feedback.contains("multi-patch-bootstrap"));
    assert!(feedback.contains("multi-patch-top"));
    assert!(feedback.contains("executed_tool_call_count"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_state_barrier_skips_dependent_tail_without_side_effect() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::new().await?;
    harness
        .write_file("README.md", "Read before finish.\n")
        .await?;
    let initialize = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [{
            "node_id": "inspect",
            "kind": "inspect_code_context",
            "goal": "Read README before finishing."
        }],
        "current_node_id": "inspect",
        "continuation": {
            "kind": "actions",
            "actions": [{"tool_name": "exec_command", "arguments": {"cmd": "cat README.md"}}]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "验证失败屏障"),
        sse(vec![
            ev_response_created("failure-response-1"),
            ev_function_call("failure-init", "taskspace_control", &initialize),
            ev_completed("failure-response-1"),
        ]),
    )
    .await;

    let invalid_bind = serde_json::to_string(&json!({
        "action": "bind_node",
        "node_id": "missing-node"
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "failure-init"),
        sse(vec![
            ev_response_created("failure-response-2"),
            ev_function_call("failure-barrier", "taskspace_control", &invalid_bind),
            ev_shell_command_call("failure-tail", "printf touched > should-not-exist"),
            ev_completed("failure-response-2"),
        ]),
    )
    .await;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "skipped_due_to_prior_failure"),
        sse(vec![
            ev_response_created("failure-response-3"),
            ev_shell_command_call("failure-read", "cat README.md"),
            ev_completed("failure-response-3"),
        ]),
    )
    .await;
    let finish = serde_json::to_string(&json!({
        "action": "finish_then_end",
        "finish_node_ids": ["inspect"],
        "final_candidate": "Failure path verified."
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, "failure-read"),
        sse(vec![
            ev_response_created("failure-response-4"),
            ev_function_call("failure-finish", "taskspace_control", &finish),
            ev_completed("failure-response-4"),
        ]),
    )
    .await;
    enable_taskspace(&harness).await?;
    harness.submit("验证失败屏障").await?;

    assert!(!harness.path("should-not-exist").exists());
    let requests = harness.request_bodies().await;
    assert!(requests.iter().any(|request| {
        let text = request.to_string();
        text.contains("failure-tail") && text.contains("skipped_due_to_prior_failure")
    }));
    Ok(())
}

async fn enable_taskspace(harness: &TestCodexHarness) -> Result<()> {
    harness
        .test()
        .codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await?;
    wait_for_event(&harness.test().codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn map_runtime_conversation_records_node_bound_subagent_events() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let create_node_args = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [
            {
                "node_id": "coordinate",
                "kind": "inspect_code_context",
                "goal": "等待并整合缓存模块边界调查结果。"
            },
            {
                "node_id": "investigate",
                "kind": "inspect_code_context",
                "goal": "供子 agent 调查缓存模块边界。"
            }
        ],
        "current_node_id": "coordinate",
        "continuation": {
            "kind": "actions",
            "actions": [{
                "tool_name": "spawn_agent",
                "arguments": {"message": CHILD_PROMPT, "task_name": "scope"}
            }]
        }
    }))?;

    responses::mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, USER_PROMPT),
        sse(vec![
            ev_response_created("resp-parent-1"),
            ev_function_call(CREATE_NODE_CALL_ID, "taskspace_control", &create_node_args),
            ev_completed("resp-parent-1"),
        ]),
    )
    .await;
    responses::mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, CHILD_PROMPT) && !body_contains(req, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-child-1"),
            ev_assistant_message(
                "msg-child-1",
                "边界调查完成：缓存 key 由 namespace 和 key 组成。",
            ),
            ev_completed("resp-child-1"),
        ]),
    )
    .await;
    responses::mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-3"),
            ev_assistant_message("msg-parent-3", "已收到子 agent 完成通知。"),
            ev_completed("resp-parent-3"),
        ]),
    )
    .await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should enable multi-agent v2");
    });
    let test = builder.build(&server).await?;
    test.codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
    let initial_snapshot = test.codex.action_map_snapshot().await;
    assert!(!initial_snapshot.routing_required);
    assert!(!initial_snapshot.bootstrap_required);
    assert_eq!(initial_snapshot.tasks.len(), 1);
    assert_eq!(initial_snapshot.maps.len(), 1);
    assert!(initial_snapshot.maps[0].nodes.is_empty());

    test.submit_turn(USER_PROMPT).await?;
    let rollout_path = test.codex.rollout_path().context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, "lease_released").await?;
    let timeline = map_runtime_timeline(&rollout)?;
    assert_event_order(&timeline, "task_created", "map_created");
    assert_event_present(&timeline, "snapshot_updated");
    assert_event_present(&timeline, "map_created");
    assert_event_present(&timeline, "node_status_changed");
    assert_event_present(&timeline, "lease_created");
    assert_event_present(&timeline, "lease_attached");
    assert_event_present(&timeline, "node_result_recorded");
    assert_event_present(&timeline, "lease_released");

    let lease_created_count = count_event(&timeline, "lease_created");
    let lease_released_count = count_event(&timeline, "lease_released");
    assert_eq!(lease_created_count, 2);
    assert_eq!(lease_released_count, 1);

    write_basic_artifacts(&timeline, &rollout_path, &initial_snapshot)?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn realistic_user_bugfix_runs_agent_actions_with_action_map() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should enable multi-agent v2");
    });
    let harness = TestCodexHarness::with_builder(builder).await?;
    seed_cache_fixture(&harness).await?;
    mount_realistic_user_bugfix_responses(&harness).await?;

    harness
        .test()
        .codex
        .submit(Op::SetMapRuntimeMode {
            mode: MapRuntimeMode::Experiment,
        })
        .await?;
    wait_for_event(&harness.test().codex, |event| {
        matches!(event, EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(_)))
    })
    .await;
    let initial_snapshot = harness.test().codex.action_map_snapshot().await;
    assert!(!initial_snapshot.routing_required);
    assert!(!initial_snapshot.bootstrap_required);
    assert_eq!(initial_snapshot.tasks.len(), 1);
    assert_eq!(initial_snapshot.maps.len(), 1);
    assert!(initial_snapshot.maps[0].nodes.is_empty());

    harness.submit(REALISTIC_USER_PROMPT).await?;

    let request_bodies = harness.request_bodies().await;
    assert!(
        request_bodies.iter().any(|request| {
            let text = request.to_string();
            text.contains("created") && text.contains("finished_node_id")
        }),
        "created-node identities did not enter a later request: {request_bodies:#?}"
    );
    assert!(
        request_bodies
            .iter()
            .any(|body| body.to_string().contains(REALISTIC_PATCH_CALL_ID)),
        "parent agent should request the patch tool after waiting for the subagent: {request_bodies:#?}"
    );
    let patch_output = request_bodies
        .iter()
        .map(Value::to_string)
        .find(|body| body.contains(REALISTIC_PATCH_CALL_ID) && body.contains("execution_outcome"))
        .context("direct apply_patch output should be present in the next provider request")?;
    assert!(
        patch_output.contains("exited") && patch_output.contains("shell_exit_code"),
        "patch tool should succeed before file assertions: {patch_output}"
    );
    assert!(
        harness
            .read_file_text("src/cache.py")
            .await?
            .contains("namespace.lower()"),
        "agent patch should normalize namespace"
    );
    assert!(
        harness
            .read_file_text("tests/test_cache.py")
            .await?
            .contains("test_cache_key_normalizes_namespace"),
        "agent patch should add a regression test"
    );

    let validation_output = harness.function_call_stdout(REALISTIC_TEST_CALL_ID).await;
    assert!(
        validation_output.contains("cache validation passed"),
        "parent agent should run validation after patch: {validation_output}"
    );

    assert!(
        request_bodies.iter().any(|body| {
            let text = body.to_string();
            text.contains("TaskSpace node assignment") && text.contains("Node: investigate")
        }),
        "child model request should include the TaskSpace node assignment"
    );
    let rollout_path = harness
        .test()
        .codex
        .rollout_path()
        .context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, "node_result_recorded").await?;
    let timeline = map_runtime_timeline(&rollout)?;
    assert_event_order(&timeline, "task_created", "map_created");
    for event_type in [
        "mode_changed",
        "snapshot_updated",
        "task_created",
        "map_created",
        "node_status_changed",
        "lease_created",
        "lease_attached",
        "node_result_recorded",
        "lease_released",
    ] {
        assert_event_present(&timeline, event_type);
    }
    assert_eq!(count_event(&timeline, "lease_created"), 3);
    assert_eq!(count_event(&timeline, "lease_released"), 3);
    assert_eq!(count_lease_released_by_holder(&timeline, "subagent"), 1);
    assert_eq!(count_lease_released_by_holder(&timeline, "main"), 2);
    assert!(
        timeline.iter().any(|event| {
            let text = event.to_string();
            text.contains("node_result_recorded")
                && text.contains("investigate")
                && text.contains("main_tool_call")
        }),
        "subagent tool evidence should remain attached to its initialized investigation node"
    );

    write_realistic_artifacts(
        &timeline,
        &rollout_path,
        harness.cwd(),
        &request_bodies,
        &validation_output,
        &initial_snapshot,
    )?;

    Ok(())
}

async fn seed_cache_fixture(harness: &TestCodexHarness) -> Result<()> {
    harness
        .write_file(
            "src/cache.py",
            "def cache_key(namespace, key):\n    return f\"{namespace}:{key.lower()}\"\n",
        )
        .await?;
    harness
        .write_file(
            "tests/test_cache.py",
            "from src.cache import cache_key\n\n\
def test_cache_key_normalizes_key():\n    assert cache_key(\"Users\", \"ABC\") == \"Users:abc\"\n",
        )
        .await?;
    harness
        .write_file(
            "README.md",
            "# Cache fixture\n\nThe cache key should normalize namespace and key consistently.\n",
        )
        .await?;
    Ok(())
}

async fn mount_realistic_user_bugfix_responses(harness: &TestCodexHarness) -> Result<()> {
    let create_node_args = serde_json::to_string(&json!({
        "action": "initialize_then_actions",
        "initial_nodes": [
            {
                "node_id": "coordinate",
                "kind": "inspect_code_context",
                "goal": "等待并整合边界调查结果。"
            },
            {
                "node_id": "investigate",
                "kind": "inspect_code_context",
                "goal": "供子 agent 阅读缓存代码和测试。"
            }
        ],
        "current_node_id": "coordinate",
        "continuation": {
            "kind": "actions",
            "actions": [{
                "tool_name": "spawn_agent",
                "arguments": {"message": REALISTIC_CHILD_PROMPT, "task_name": "scope"}
            }]
        }
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_USER_PROMPT),
        sse(vec![
            ev_response_created("resp-parent-create-node"),
            ev_function_call(
                REALISTIC_CREATE_NODE_CALL_ID,
                "taskspace_control",
                &create_node_args,
            ),
            ev_completed("resp-parent-create-node"),
        ]),
    )
    .await;

    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| {
            body_contains(req, REALISTIC_CHILD_PROMPT)
                && body_contains(req, "TaskSpace node assignment")
        },
        sse(vec![
            ev_response_created("resp-child-read"),
            ev_shell_command_call(
                REALISTIC_CHILD_READ_CALL_ID,
                "python -c \"from pathlib import Path; print(Path('src/cache.py').read_text()); print(Path('tests/test_cache.py').read_text())\"",
            ),
            ev_completed("resp-child-read"),
        ]),
    )
    .await;

    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_CHILD_READ_CALL_ID),
        sse(vec![
            ev_response_created("resp-child-final"),
            ev_assistant_message(
                "msg-child-final",
                "边界调查完成：cache_key 当前只归一化 key，没有归一化 namespace；应补充 namespace 回归测试。",
            ),
            ev_completed("resp-child-final"),
        ]),
    )
    .await;

    let wait_args = serde_json::to_string(&json!({ "timeout_ms": 10_000 }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-wait"),
            ev_function_call(REALISTIC_WAIT_CALL_ID, "wait_agent", &wait_args),
            ev_completed("resp-parent-wait"),
        ]),
    )
    .await;

    let patch = r#"*** Begin Patch
*** Update File: src/cache.py
@@
-    return f"{namespace}:{key.lower()}"
+    return f"{namespace.lower()}:{key.lower()}"
*** Update File: tests/test_cache.py
@@
 def test_cache_key_normalizes_key():
     assert cache_key("Users", "ABC") == "Users:abc"
+
+def test_cache_key_normalizes_namespace():
+    assert cache_key("Users", "ABC") == "users:abc"
*** End Patch"#;
    let implementation_node_args = serde_json::to_string(&json!({
        "action": "finish_nodes",
        "finishes": [{
            "next": {
                "kind": "create",
                "node_kind": "implement_solution",
                "goal": "Implement the cache key namespace normalization fix after the boundary investigation finished."
            }
        }]
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_WAIT_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-create-implementation-node"),
            ev_function_call(
                REALISTIC_IMPLEMENT_NODE_CALL_ID,
                "taskspace_control",
                &implementation_node_args,
            ),
            ev_apply_patch_function_call(REALISTIC_PATCH_CALL_ID, patch),
            ev_completed("resp-parent-create-implementation-node"),
        ]),
    )
    .await;

    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_PATCH_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-test"),
            ev_shell_command_call(
                REALISTIC_TEST_CALL_ID,
                "python -c \"from src.cache import cache_key; assert cache_key('Users', 'ABC') == 'users:abc'; print('cache validation passed')\"",
            ),
            ev_completed("resp-parent-test"),
        ]),
    )
    .await;

    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_TEST_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-finish-implementation-node"),
            ev_function_call(
                REALISTIC_FINISH_NODE_CALL_ID,
                "taskspace_control",
                &serde_json::to_string(&json!({
                    "action": "finish_then_end",
                    "finish_node_ids": ["node-1"],
                    "final_candidate": "已修复缓存 key namespace 归一化问题，并运行验证通过。",
                }))?,
            ),
            ev_completed("resp-parent-finish-implementation-node"),
        ]),
    )
    .await;

    Ok(())
}

async fn wait_for_rollout_fragment(path: &Path, fragment: &str) -> Result<String> {
    for _ in 0..50 {
        let text = fs::read_to_string(path).unwrap_or_default();
        if text.contains(fragment) {
            return Ok(text);
        }
        sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("rollout did not contain `{fragment}` at {}", path.display())
}

fn body_contains(req: &Request, text: &str) -> bool {
    String::from_utf8(req.body.clone())
        .ok()
        .is_some_and(|body| body.contains(text))
}

fn map_runtime_timeline(rollout: &str) -> Result<Vec<Value>> {
    let mut events = Vec::new();
    for line in rollout.lines() {
        let value: Value = serde_json::from_str(line)?;
        if is_map_runtime_event(&value) {
            events.push(value);
        }
    }
    Ok(events)
}

fn is_map_runtime_event(value: &Value) -> bool {
    let text = value.to_string();
    [
        "mode_changed",
        "task_created",
        "task_status_changed",
        "task_routed",
        "map_created",
        "map_status_changed",
        "node_status_changed",
        "lease_created",
        "lease_attached",
        "lease_released",
        "node_result_recorded",
        "timeout_summary_requested",
        "maintenance_barrier_raised",
        "maintenance_barrier_cleared",
        "snapshot_updated",
    ]
    .iter()
    .any(|event_type| text.contains(event_type))
}

fn assert_event_present(timeline: &[Value], event_type: &str) {
    assert!(
        count_event(timeline, event_type) > 0,
        "expected map runtime event `{event_type}` in {timeline:#?}"
    );
}

fn assert_event_order(timeline: &[Value], before: &str, after: &str) {
    let before_index = timeline
        .iter()
        .position(|event| event.to_string().contains(before))
        .unwrap_or_else(|| panic!("expected event `{before}` in {timeline:#?}"));
    let after_index = timeline
        .iter()
        .position(|event| event.to_string().contains(after))
        .unwrap_or_else(|| panic!("expected event `{after}` in {timeline:#?}"));
    assert!(
        before_index < after_index,
        "expected `{before}` before `{after}` in {timeline:#?}"
    );
}

fn count_event(timeline: &[Value], event_type: &str) -> usize {
    timeline
        .iter()
        .filter(|event| event.to_string().contains(event_type))
        .count()
}

fn count_lease_released_by_holder(timeline: &[Value], holder: &str) -> usize {
    timeline
        .iter()
        .filter(|event| {
            event.to_string().contains("lease_released")
                && event
                    .pointer("/payload/holder")
                    .and_then(Value::as_str)
                    .is_some_and(|event_holder| event_holder == holder)
        })
        .count()
}

fn scenario_artifacts_dir(scenario_id: &str) -> Result<PathBuf> {
    let target_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/scenario-runs");
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();
    let artifacts = target_root.join(scenario_id).join(run_id).join("artifacts");
    fs::create_dir_all(&artifacts)?;
    Ok(artifacts)
}

fn write_basic_artifacts(
    timeline: &[Value],
    rollout_path: &Path,
    initial_snapshot: &ActionMapSnapshot,
) -> Result<()> {
    let artifacts = scenario_artifacts_dir("map-runtime-node-bound-subagent")?;
    fs::write(
        artifacts.join("map-timeline.json"),
        serde_json::to_string_pretty(timeline)?,
    )?;
    fs::write(
        artifacts.join("report.md"),
        format!(
            "# Scenario Report\n\nscenario: map-runtime-node-bound-subagent\nrollout: {}\nevents: {}\ninitial_routing_required: {}\ninitial_bootstrap_required: {}\ninitial_task_count: {}\ninitial_map_count: {}\nsnapshot_updated_events: {}\n",
            rollout_path.display(),
            timeline.len(),
            initial_snapshot.routing_required,
            initial_snapshot.bootstrap_required,
            initial_snapshot.tasks.len(),
            initial_snapshot.maps.len(),
            count_event(timeline, "snapshot_updated")
        ),
    )?;
    Ok(())
}

fn write_realistic_artifacts(
    timeline: &[Value],
    rollout_path: &Path,
    workspace: &Path,
    provider_requests: &[Value],
    validation_output: &str,
    initial_snapshot: &ActionMapSnapshot,
) -> Result<()> {
    let artifacts = scenario_artifacts_dir("action-map-realistic-user-bugfix")?;
    fs::write(
        artifacts.join("map-timeline.json"),
        serde_json::to_string_pretty(timeline)?,
    )?;
    fs::write(
        artifacts.join("provider-requests.json"),
        serde_json::to_string_pretty(provider_requests)?,
    )?;
    fs::write(artifacts.join("test-output.txt"), validation_output)?;
    fs::write(
        artifacts.join("transcript.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({
                "scenario": "action-map-realistic-user-bugfix",
                "user": REALISTIC_USER_PROMPT,
                "provider_request_count": provider_requests.len(),
                "workspace": workspace.display().to_string()
            })
        ),
    )?;
    fs::write(
        artifacts.join("report.md"),
        format!(
            "# Scenario Report\n\n\
scenario: action-map-realistic-user-bugfix\n\
user_prompt: {REALISTIC_USER_PROMPT}\n\
workspace: {}\n\
rollout: {}\n\
provider_requests: {}\n\
map_events: {}\n\
initial_routing_required: {}\n\
initial_bootstrap_required: {}\n\
initial_task_count: {}\n\
initial_map_count: {}\n\
snapshot_updated_events: {}\n\
validation: {}\n",
            workspace.display(),
            rollout_path.display(),
            provider_requests.len(),
            timeline.len(),
            initial_snapshot.routing_required,
            initial_snapshot.bootstrap_required,
            initial_snapshot.tasks.len(),
            initial_snapshot.maps.len(),
            count_event(timeline, "snapshot_updated"),
            validation_output.trim()
        ),
    )?;
    Ok(())
}
