use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapEdge;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_state::BindTaskSpaceMapRequest;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::StateRuntime;
use codex_state::TaskSpaceMapRelation;
use tempfile::TempDir;

fn whale_command(codex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("whale")?);
    cmd.env("WHALE_HOME", codex_home);
    Ok(cmd)
}

fn utf8_path(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn canonical_map(map_id: &str) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.to_string(),
        map_id: map_id.to_string(),
        root: TaskSpaceMapNode {
            node_id: "root".to_string(),
            goal: "deliver the task".to_string(),
            source_refs: vec!["user-turn-1".to_string()],
        },
        work_nodes: vec![TaskSpaceMapNode {
            node_id: "work".to_string(),
            goal: "implement the change".to_string(),
            source_refs: Vec::new(),
        }],
        finish: TaskSpaceMapNode {
            node_id: "finish".to_string(),
            goal: "verify and summarize".to_string(),
            source_refs: Vec::new(),
        },
        edges: vec![
            TaskSpaceMapEdge {
                from: "root".to_string(),
                to: "work".to_string(),
            },
            TaskSpaceMapEdge {
                from: "work".to_string(),
                to: "finish".to_string(),
            },
        ],
        completion_records: BTreeMap::new(),
        block_records: BTreeMap::new(),
        action_reservations: BTreeMap::new(),
        result_refs: BTreeMap::new(),
        evidence_refs: BTreeMap::new(),
        terminal_record: None,
        terminal_history: Vec::new(),
        revision: 1,
    }
}

async fn seed_taskspace_map(codex_home: &Path) -> Result<(ThreadId, String)> {
    let runtime = StateRuntime::init(codex_home.to_path_buf(), "test-provider".to_string()).await?;
    let thread_id = ThreadId::new();
    let map_id = format!("map-{thread_id}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: thread_id,
            canonical_map: Some(canonical_map(&map_id)),
            commit_id: "create-debug-export-fixture".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await?;
    runtime
        .bind_thread_to_taskspace_map(BindTaskSpaceMapRequest {
            thread_id,
            map_id: map_id.clone(),
            relation: TaskSpaceMapRelation::Owner,
            parent_thread_id: None,
        })
        .await?;
    drop(runtime);
    Ok((thread_id, map_id))
}

#[tokio::test]
async fn debug_taskspace_map_exports_the_store_record_and_binding() -> Result<()> {
    let codex_home = TempDir::new()?;
    let (thread_id, map_id) = seed_taskspace_map(codex_home.path()).await?;

    let output = codex_home.path().join("exports").join("taskspace-map.json");
    let mut cmd = whale_command(codex_home.path())?;
    cmd.args([
        "debug",
        "taskspace-map",
        "--thread-id",
        &thread_id.to_string(),
        "--output",
        utf8_path(&output)?,
    ])
    .assert()
    .success();

    let envelope: serde_json::Value = serde_json::from_slice(&std::fs::read(output)?)?;
    assert_eq!(envelope["schema_version"], "TaskSpaceMapExportR7V2");
    assert_eq!(envelope["status"], "ok");
    assert_eq!(envelope["map"]["map_id"], map_id);
    assert_eq!(envelope["map"]["owner_thread_id"], thread_id.to_string());
    assert_eq!(envelope["map"]["store_revision"], 1);
    assert_eq!(envelope["map"]["map_revision"], 1);
    assert_eq!(envelope["map"]["terminal"], false);
    assert_eq!(envelope["binding"]["thread_id"], thread_id.to_string());
    assert_eq!(envelope["binding"]["relation"], "owner");
    assert_eq!(
        envelope["map"]["canonical_map"]["schema_version"],
        TASKSPACE_CANONICAL_SCHEMA_VERSION
    );
    Ok(())
}

#[tokio::test]
async fn debug_taskspace_map_rejects_a_thread_without_a_binding() -> Result<()> {
    let codex_home = TempDir::new()?;
    let runtime =
        StateRuntime::init(codex_home.path().to_path_buf(), "test-provider".to_string()).await?;
    drop(runtime);
    let output = codex_home.path().join("taskspace-map.json");
    let mut cmd = whale_command(codex_home.path())?;
    cmd.args([
        "debug",
        "taskspace-map",
        "--thread-id",
        &ThreadId::new().to_string(),
        "--output",
        utf8_path(&output)?,
    ])
    .assert()
    .failure();
    assert!(!output.exists());
    Ok(())
}

#[test]
fn debug_taskspace_map_rejects_an_invalid_thread_id() -> Result<()> {
    let codex_home = TempDir::new()?;
    let output = codex_home.path().join("taskspace-map.json");
    let mut cmd = whale_command(codex_home.path())?;
    cmd.args([
        "debug",
        "taskspace-map",
        "--thread-id",
        "not-a-thread-id",
        "--output",
        utf8_path(&output)?,
    ])
    .assert()
    .failure();
    assert!(!output.exists());
    Ok(())
}

#[tokio::test]
async fn powershell_observability_exports_from_the_real_map_store() -> Result<()> {
    let codex_home = TempDir::new()?;
    let (thread_id, map_id) = seed_taskspace_map(codex_home.path()).await?;
    let fixture = TempDir::new()?;
    let rollout = fixture.path().join("rollout.jsonl");
    let jsonl = fixture.path().join("whale-exec.jsonl");
    let output = fixture.path().join("observability");
    std::fs::write(
        &rollout,
        "{\"timestamp\":\"2026-07-24T00:00:00Z\",\"payload\":{\"type\":\"fixture\"}}\n",
    )?;
    std::fs::write(&jsonl, "")?;

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()?;
    let export_script = repo_root.join("scripts/export-action-map-observability.ps1");
    let whale = codex_utils_cargo_bin::cargo_bin("whale")?;
    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };
    let mut cmd = assert_cmd::Command::new(shell);
    cmd.env("WHALE_HOME", codex_home.path())
        .args([
            "-NoProfile",
            "-File",
            utf8_path(&export_script)?,
            "-RolloutPath",
            utf8_path(&rollout)?,
            "-JsonlPath",
            utf8_path(&jsonl)?,
            "-OutputDir",
            utf8_path(&output)?,
            "-WhalePath",
            utf8_path(&whale)?,
            "-ThreadId",
            &thread_id.to_string(),
        ])
        .assert()
        .success();

    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(
        output.join("action-map-observability.json"),
    )?)?;
    assert_eq!(report["source"]["mapStore"]["availability"], "measured");
    assert_eq!(report["source"]["mapStore"]["map_id"], map_id);
    assert_eq!(
        report["source"]["mapStore"]["binding_thread_id"],
        thread_id.to_string()
    );
    Ok(())
}

#[tokio::test]
async fn powershell_observability_bounds_large_rollouts_with_the_real_map_store() -> Result<()> {
    let codex_home = TempDir::new()?;
    let (thread_id, _) = seed_taskspace_map(codex_home.path()).await?;
    let fixture = TempDir::new()?;
    let rollout = fixture.path().join("rollout.jsonl");
    let jsonl = fixture.path().join("whale-exec.jsonl");
    let output = fixture.path().join("observability");
    let marker = "bounded-rollout-marker-".repeat(4_000);
    let mut lines = String::new();
    for index in 0..20 {
        lines.push_str(
            &serde_json::json!({
                "timestamp": "2026-07-24T00:00:00Z",
                "payload": {
                    "type": "fixture_event",
                    "index": index,
                    "body": marker,
                }
            })
            .to_string(),
        );
        lines.push('\n');
    }
    std::fs::write(&rollout, lines)?;
    std::fs::write(&jsonl, "")?;

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()?;
    let export_script = repo_root.join("scripts/export-action-map-observability.ps1");
    let whale = codex_utils_cargo_bin::cargo_bin("whale")?;
    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };
    let mut cmd = assert_cmd::Command::new(shell);
    cmd.env("WHALE_HOME", codex_home.path())
        .env("TASKSPACE_OBSERVABILITY_ROLLOUT_MAX_BYTES", "1048576")
        .env("TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES", "65536")
        .args([
            "-NoProfile",
            "-File",
            utf8_path(&export_script)?,
            "-RolloutPath",
            utf8_path(&rollout)?,
            "-JsonlPath",
            utf8_path(&jsonl)?,
            "-OutputDir",
            utf8_path(&output)?,
            "-WhalePath",
            utf8_path(&whale)?,
            "-ThreadId",
            &thread_id.to_string(),
        ])
        .assert()
        .success();

    let report_path = output.join("action-map-observability.json");
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&report_path)?)?;
    assert_eq!(
        report["source"]["exportPolicy"]["rollout_export_mode"],
        "summary_only_large_rollout"
    );
    assert!(
        report["source"]["rolloutReadStats"]["largeLineSkippedCount"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(std::fs::metadata(report_path)?.len() < 1_048_576);
    Ok(())
}
