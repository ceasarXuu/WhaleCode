use anyhow::Context;
use anyhow::Result;
use codex_features::Feature;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::Op;
use core_test_support::responses;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::time::sleep;
use wiremock::Request;

const USER_PROMPT: &str = "请用一个子 agent 调查缓存模块边界，然后继续推进。";
const CHILD_PROMPT: &str = "调查缓存模块边界";
const SPAWN_CALL_ID: &str = "spawn-map-node";

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

    write_artifacts(&timeline, &rollout_path)?;

    Ok(())
}

async fn wait_for_rollout_fragment(path: &std::path::Path, fragment: &str) -> Result<String> {
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

fn write_artifacts(timeline: &[Value], rollout_path: &std::path::Path) -> Result<()> {
    let target_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/scenario-runs");
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();
    let artifacts = target_root
        .join("map-runtime-node-bound-subagent")
        .join(run_id)
        .join("artifacts");
    fs::create_dir_all(&artifacts)?;
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
