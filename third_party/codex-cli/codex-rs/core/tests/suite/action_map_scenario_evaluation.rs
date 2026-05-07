use anyhow::Context;
use anyhow::Result;
use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::Op;
use core_test_support::responses;
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
const SPAWN_CALL_ID: &str = "spawn-map-node";

const REALISTIC_USER_PROMPT: &str =
    "这个沙盒项目有一个缓存 key 相关的回归失败。请先让子 agent 调查边界，再修复代码并验证。";
const REALISTIC_CHILD_PROMPT: &str = "调查缓存 key 失败边界，阅读 src/cache.py 和测试文件。";
const REALISTIC_SPAWN_CALL_ID: &str = "spawn-cache-scope-agent";
const REALISTIC_CHILD_READ_CALL_ID: &str = "child-read-cache-files";
const REALISTIC_WAIT_CALL_ID: &str = "parent-wait-cache-scope-agent";
const REALISTIC_PATCH_CALL_ID: &str = "parent-apply-cache-fix";
const REALISTIC_TEST_CALL_ID: &str = "parent-run-cache-validation";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn map_runtime_conversation_records_node_bound_subagent_events() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let spawn_args = serde_json::to_string(&json!({
        "message": CHILD_PROMPT,
        "task_name": "scope",
    }))?;

    responses::mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, USER_PROMPT),
        sse(vec![
            ev_response_created("resp-parent-1"),
            ev_function_call(SPAWN_CALL_ID, "spawn_agent", &spawn_args),
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
            ev_response_created("resp-parent-2"),
            ev_assistant_message("msg-parent-2", "已收到子 agent 完成通知。"),
            ev_completed("resp-parent-2"),
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

    test.submit_turn(USER_PROMPT).await?;
    let rollout_path = test.codex.rollout_path().context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, "node_result_recorded").await?;
    let timeline = map_runtime_timeline(&rollout)?;
    assert_event_present(&timeline, "map_created");
    assert_event_present(&timeline, "node_status_changed");
    assert_event_present(&timeline, "lease_created");
    assert_event_present(&timeline, "lease_attached");
    assert_event_present(&timeline, "node_result_recorded");
    assert_event_present(&timeline, "lease_released");

    let lease_created_count = count_event(&timeline, "lease_created");
    let lease_released_count = count_event(&timeline, "lease_released");
    assert_eq!(lease_created_count, 1);
    assert_eq!(
        lease_created_count, lease_released_count,
        "node-bound subagent lease should be released when result is recorded"
    );

    write_basic_artifacts(&timeline, &rollout_path)?;

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

    harness.submit(REALISTIC_USER_PROMPT).await?;

    let request_bodies = harness.request_bodies().await;
    assert!(
        request_bodies
            .iter()
            .any(|body| body.to_string().contains(REALISTIC_PATCH_CALL_ID)),
        "parent agent should request the patch tool after waiting for the subagent: {request_bodies:#?}"
    );
    let patch_output = harness.function_call_stdout(REALISTIC_PATCH_CALL_ID).await;
    let patch_output_json: Value = serde_json::from_str(&patch_output)
        .context("apply_patch function output should be JSON")?;
    assert!(
        patch_output_json
            .pointer("/metadata/exit_code")
            .and_then(Value::as_i64)
            == Some(0),
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
            text.contains("Action Map node assignment") && text.contains("Node: define_scope")
        }),
        "child model request should include the Action Map node assignment"
    );

    let rollout_path = harness
        .test()
        .codex
        .rollout_path()
        .context("rollout path")?;
    let rollout = wait_for_rollout_fragment(&rollout_path, "node_result_recorded").await?;
    let timeline = map_runtime_timeline(&rollout)?;
    for event_type in [
        "mode_changed",
        "map_created",
        "node_status_changed",
        "lease_created",
        "lease_attached",
        "node_result_recorded",
        "lease_released",
    ] {
        assert_event_present(&timeline, event_type);
    }
    assert_eq!(count_event(&timeline, "lease_created"), 1);
    assert_eq!(count_event(&timeline, "lease_released"), 1);

    write_realistic_artifacts(
        &timeline,
        &rollout_path,
        harness.cwd(),
        &request_bodies,
        &validation_output,
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
    let spawn_args = serde_json::to_string(&json!({
        "message": REALISTIC_CHILD_PROMPT,
        "task_name": "scope",
    }))?;
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_USER_PROMPT),
        sse(vec![
            ev_response_created("resp-parent-spawn"),
            ev_function_call(REALISTIC_SPAWN_CALL_ID, "spawn_agent", &spawn_args),
            ev_completed("resp-parent-spawn"),
        ]),
    )
    .await;

    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| {
            body_contains(req, REALISTIC_CHILD_PROMPT)
                && body_contains(req, "Action Map node assignment")
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
    responses::mount_sse_once_match(
        harness.server(),
        |req: &Request| body_contains(req, REALISTIC_WAIT_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-patch"),
            ev_apply_patch_function_call(REALISTIC_PATCH_CALL_ID, patch),
            ev_completed("resp-parent-patch"),
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
            ev_response_created("resp-parent-final"),
            ev_assistant_message(
                "msg-parent-final",
                "已修复缓存 key namespace 归一化问题，并运行验证通过。",
            ),
            ev_completed("resp-parent-final"),
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
        "map_created",
        "map_status_changed",
        "node_status_changed",
        "lease_created",
        "lease_attached",
        "lease_released",
        "node_result_recorded",
        "timeout_summary_requested",
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

fn count_event(timeline: &[Value], event_type: &str) -> usize {
    timeline
        .iter()
        .filter(|event| event.to_string().contains(event_type))
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

fn write_basic_artifacts(timeline: &[Value], rollout_path: &Path) -> Result<()> {
    let artifacts = scenario_artifacts_dir("map-runtime-node-bound-subagent")?;
    fs::write(
        artifacts.join("map-timeline.json"),
        serde_json::to_string_pretty(timeline)?,
    )?;
    fs::write(
        artifacts.join("report.md"),
        format!(
            "# Scenario Report\n\nscenario: map-runtime-node-bound-subagent\nrollout: {}\nevents: {}\n",
            rollout_path.display(),
            timeline.len()
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
validation: {}\n",
            workspace.display(),
            rollout_path.display(),
            provider_requests.len(),
            timeline.len(),
            validation_output.trim()
        ),
    )?;
    Ok(())
}
