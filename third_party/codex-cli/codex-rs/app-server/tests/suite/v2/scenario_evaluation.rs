use anyhow::Result;
use app_test_support::McpProcess;
use app_test_support::create_apply_patch_sse_response;
use app_test_support::create_final_assistant_message_sse_response;
use app_test_support::create_mock_responses_server_sequence;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::UserInput as V2UserInput;
use codex_features::FEATURES;
use codex_features::Feature;
use core_test_support::skip_if_no_network;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::time::timeout;
use wiremock::Match;
use wiremock::matchers::path_regex;

#[cfg(windows)]
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(25);
#[cfg(not(windows))]
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn deterministic_scenario_small_bugfix_produces_artifacts_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

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
*** End Patch
"#;
    let outcome = run_app_server_scenario(
        "small-bugfix",
        "这个项目有一个缓存 key 相关的测试失败，请帮我定位并修复。",
        vec![
            create_apply_patch_sse_response(patch, "patch-small-bugfix")?,
            create_final_assistant_message_sse_response("已修复缓存 key 归一化问题。")?,
        ],
        |repo| {
            fs::create_dir_all(repo.join("src"))?;
            fs::create_dir_all(repo.join("tests"))?;
            fs::write(
                repo.join("src/cache.py"),
                "def cache_key(namespace, key):\n    return f\"{namespace}:{key.lower()}\"\n",
            )?;
            fs::write(
                repo.join("tests/test_cache.py"),
                "from src.cache import cache_key\n\n\
def test_cache_key_normalizes_key():\n    assert cache_key(\"Users\", \"ABC\") == \"Users:abc\"\n",
            )?;
            Ok(())
        },
    )
    .await?;

    assert!(outcome.repo.join("src/cache.py").is_file());
    assert!(fs::read_to_string(outcome.repo.join("src/cache.py"))?.contains("namespace.lower()"));
    assert!(
        fs::read_to_string(outcome.repo.join("tests/test_cache.py"))?
            .contains("test_cache_key_normalizes_namespace")
    );
    assert!(outcome.artifacts.join("report.md").is_file());
    assert!(outcome.artifacts.join("diff.patch").is_file());
    assert!(outcome.artifacts.join("test-output.txt").is_file());
    assert!(outcome.artifacts.join("map-timeline.json").is_file());
    assert_eq!(outcome.provider_request_count, 2);

    Ok(())
}

#[tokio::test]
async fn deterministic_scenario_ambiguous_requirement_stops_for_clarification_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let outcome = run_app_server_scenario(
        "ambiguous-requirement",
        "帮我把这个工具的性能做好一点。",
        vec![create_final_assistant_message_sse_response(
            "这个需求缺少边界：请先确认要优化哪个命令、目标指标和可接受的行为变化。",
        )?],
        |repo| {
            fs::write(repo.join("README.md"), "# ambiguous requirement fixture\n")?;
            Ok(())
        },
    )
    .await?;

    let report = fs::read_to_string(outcome.artifacts.join("report.md"))?;
    assert!(report.contains("ambiguous-requirement"));
    assert!(report.contains("provider_requests: 1"));
    assert_eq!(outcome.provider_request_count, 1);

    Ok(())
}

struct ScenarioOutcome {
    repo: PathBuf,
    artifacts: PathBuf,
    provider_request_count: usize,
}

async fn run_app_server_scenario(
    scenario_id: &str,
    prompt: &str,
    model_responses: Vec<String>,
    seed_repo: impl FnOnce(&Path) -> Result<()>,
) -> Result<ScenarioOutcome> {
    let run = ScenarioRun::create(scenario_id)?;
    seed_repo(&run.repo)?;

    let server = create_mock_responses_server_sequence(model_responses).await;
    create_config_toml(
        &run.whale_home,
        &server.uri(),
        &BTreeMap::from([(Feature::Collab, true)]),
    )?;

    let mut mcp = McpProcess::new(&run.whale_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let thread_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            cwd: Some(run.repo.to_string_lossy().into_owned()),
            ..Default::default()
        })
        .await?;
    let thread_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_req)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(thread_resp)?;

    let turn_req = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: prompt.to_string(),
                text_elements: Vec::new(),
            }],
            cwd: Some(run.repo.clone()),
            ..Default::default()
        })
        .await?;
    let _turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;

    timeout(DEFAULT_READ_TIMEOUT, async {
        loop {
            let completed_notif = mcp
                .read_stream_until_notification_message("turn/completed")
                .await?;
            let completed: TurnCompletedNotification =
                serde_json::from_value(completed_notif.params.expect("turn/completed params"))?;
            if completed.thread_id == thread.id {
                return Ok::<(), anyhow::Error>(());
            }
        }
    })
    .await??;

    let provider_requests = provider_request_bodies(&server).await?;
    let rollout = find_rollout(&run.whale_home)
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default();
    write_artifacts(&run, scenario_id, prompt, &provider_requests, &rollout)?;

    Ok(ScenarioOutcome {
        repo: run.repo,
        artifacts: run.artifacts,
        provider_request_count: provider_requests.len(),
    })
}

struct ScenarioRun {
    repo: PathBuf,
    whale_home: PathBuf,
    artifacts: PathBuf,
}

impl ScenarioRun {
    fn create(scenario_id: &str) -> Result<Self> {
        let target_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/scenario-runs");
        let run_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .to_string();
        let root = target_root.join(scenario_id).join(run_id);
        let repo = root.join("repo");
        let whale_home = root.join("whale-home");
        let artifacts = root.join("artifacts");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&whale_home)?;
        fs::create_dir_all(&artifacts)?;
        Ok(Self {
            repo,
            whale_home,
            artifacts,
        })
    }
}

async fn provider_request_bodies(server: &wiremock::MockServer) -> Result<Vec<Value>> {
    let path_matcher = path_regex(".*/responses$");
    let requests = server.received_requests().await.unwrap_or_default();
    Ok(requests
        .into_iter()
        .filter(|request| path_matcher.matches(request))
        .map(|request| request.body_json::<Value>())
        .collect::<Result<Vec<_>, _>>()?)
}

fn write_artifacts(
    run: &ScenarioRun,
    scenario_id: &str,
    prompt: &str,
    provider_requests: &[Value],
    rollout: &str,
) -> Result<()> {
    fs::write(
        run.artifacts.join("transcript.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({
                "scenario": scenario_id,
                "user": prompt,
                "provider_request_count": provider_requests.len()
            })
        ),
    )?;
    fs::write(
        run.artifacts.join("provider-requests.json"),
        serde_json::to_string_pretty(provider_requests)?,
    )?;
    fs::write(run.artifacts.join("rollout.jsonl"), rollout)?;
    fs::write(
        run.artifacts.join("map-timeline.json"),
        map_timeline(rollout)?,
    )?;
    fs::write(
        run.artifacts.join("diff.patch"),
        collect_patch_like_diff(rollout),
    )?;
    fs::write(
        run.artifacts.join("test-output.txt"),
        "deterministic oracle: artifact generation completed\n",
    )?;
    fs::write(
        run.artifacts.join("report.md"),
        format!(
            "# Scenario Report\n\nscenario: {scenario_id}\nprovider_requests: {}\nartifacts: {}\n",
            provider_requests.len(),
            run.artifacts.display()
        ),
    )?;
    Ok(())
}

fn map_timeline(rollout: &str) -> Result<String> {
    let events = rollout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|value| value.to_string().contains("map_runtime"))
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&events)?)
}

fn collect_patch_like_diff(rollout: &str) -> String {
    rollout
        .lines()
        .filter(|line| line.contains("apply_patch") || line.contains("*** Begin Patch"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_rollout(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
            {
                return Some(path);
            }
        }
    }
    None
}

fn create_config_toml(
    codex_home: &Path,
    server_uri: &str,
    feature_flags: &BTreeMap<Feature, bool>,
) -> std::io::Result<()> {
    let feature_entries = feature_flags
        .iter()
        .map(|(feature, enabled)| {
            let key = FEATURES
                .iter()
                .find(|spec| spec.id == *feature)
                .map(|spec| spec.key)
                .unwrap_or_else(|| panic!("missing feature key for {feature:?}"));
            format!("{key} = {enabled}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"

model_provider = "mock_provider"

[features]
{feature_entries}

[model_providers.mock_provider]
name = "Mock provider for scenario evaluation"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}
