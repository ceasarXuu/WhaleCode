use std::path::Path;

use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotSentinelSummary;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;
use codex_protocol::protocol::MapRuntimeMode;
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

fn blank_snapshot() -> ActionMapSnapshot {
    ActionMapSnapshot {
        schema_version: "action-map-snapshot-v2".to_string(),
        mode: MapRuntimeMode::Experiment,
        routing_required: false,
        bootstrap_required: true,
        reborn_requested: false,
        map: None,
        maintenance_barriers: Vec::new(),
        trace_summary: ActionMapSnapshotTraceSummary::default(),
        trace_events: Vec::new(),
        sentinel_summary: ActionMapSnapshotSentinelSummary::default(),
        sentinel_warnings: Vec::new(),
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
            snapshot: blank_snapshot(),
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
            node_id: None,
            lease_id: None,
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
        output.to_str().expect("UTF-8 output path"),
    ])
    .assert()
    .success();

    let envelope: serde_json::Value = serde_json::from_slice(&std::fs::read(output)?)?;
    assert_eq!(envelope["schema_version"], "TaskSpaceMapExportR7V1");
    assert_eq!(envelope["status"], "ok");
    assert_eq!(envelope["map"]["map_id"], map_id);
    assert_eq!(envelope["map"]["owner_thread_id"], thread_id.to_string());
    assert_eq!(envelope["map"]["store_revision"], 1);
    assert_eq!(envelope["binding"]["thread_id"], thread_id.to_string());
    assert_eq!(envelope["binding"]["relation"], "owner");
    assert_eq!(
        envelope["map"]["snapshot"]["schemaVersion"],
        "action-map-snapshot-v2"
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
        output.to_str().expect("UTF-8 output path"),
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
        output.to_str().expect("UTF-8 output path"),
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
            export_script.to_str().expect("UTF-8 script path"),
            "-RolloutPath",
            rollout.to_str().expect("UTF-8 rollout path"),
            "-JsonlPath",
            jsonl.to_str().expect("UTF-8 JSONL path"),
            "-OutputDir",
            output.to_str().expect("UTF-8 output path"),
            "-WhalePath",
            whale.to_str().expect("UTF-8 whale path"),
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
