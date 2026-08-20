use std::path::Path;

use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeState;
use codex_state::BindTaskSpaceMapRequest;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::StateRuntime;
use codex_state::TaskSpaceMapRelation;
use codex_utils_absolute_path::test_support::PathExt;
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
            state: TaskSpaceNodeState::InFlight,
            content: "The user requested this task.".to_string(),
            parents: Vec::new(),
            actions: Vec::new(),
        },
        work_nodes: vec![TaskSpaceMapNode {
            node_id: "work".to_string(),
            goal: "implement the change".to_string(),
            state: TaskSpaceNodeState::Ready,
            content: String::new(),
            parents: vec!["root".to_string()],
            actions: Vec::new(),
        }],
        finish: TaskSpaceMapNode {
            node_id: "finish".to_string(),
            goal: "verify and summarize".to_string(),
            state: TaskSpaceNodeState::Waiting,
            content: String::new(),
            parents: vec!["work".to_string()],
            actions: Vec::new(),
        },
        revision: 1,
    }
}

async fn seed_taskspace_map(codex_home: &Path) -> Result<(ThreadId, String)> {
    let sqlite = codex_state::SqliteConfig::new_for_testing(codex_home.abs());
    let runtime = StateRuntime::init(sqlite, "test-provider".to_string()).await?;
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
    whale_command(codex_home.path())?
        .args([
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
    assert_eq!(envelope["schema_version"], "TaskSpaceMapExportR8V1");
    assert_eq!(envelope["status"], "ok");
    assert_eq!(envelope["map"]["map_id"], map_id);
    assert_eq!(envelope["map"]["owner_thread_id"], thread_id.to_string());
    assert_eq!(envelope["map"]["store_revision"], 1);
    assert!(envelope["map"].get("map_revision").is_none());
    assert!(envelope["map"].get("terminal").is_none());
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
    let sqlite = codex_state::SqliteConfig::new_for_testing(codex_home.path().abs());
    let runtime = StateRuntime::init(sqlite, "test-provider".to_string()).await?;
    drop(runtime);
    let output = codex_home.path().join("taskspace-map.json");
    whale_command(codex_home.path())?
        .args([
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
    whale_command(codex_home.path())?
        .args([
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
