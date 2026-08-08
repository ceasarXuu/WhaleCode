use super::StateRuntime;
use crate::CreateTaskSpaceMapRequest;
use crate::SettleTaskSpaceActionRequest;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use codex_protocol::taskspace::TaskSpaceNodeState;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

fn map(map_id: &str) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.to_string(),
        map_id: map_id.to_string(),
        root: node("root", Vec::new(), Vec::new()),
        work_nodes: vec![node(
            "work",
            vec!["root".to_string()],
            vec![TaskSpaceNodeAction {
                action_id: "call-1".to_string(),
                tool_name: "inspect".to_string(),
                outcome: TaskSpaceActionOutcome::Pending,
            }],
        )],
        finish: node("finish", vec!["work".to_string()], Vec::new()),
        revision: 1,
    }
}

fn node(
    node_id: &str,
    parents: Vec<String>,
    actions: Vec<TaskSpaceNodeAction>,
) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.to_string(),
        goal: node_id.to_string(),
        state: if node_id == "work" {
            TaskSpaceNodeState::Ready
        } else if node_id == "root" {
            TaskSpaceNodeState::InFlight
        } else {
            TaskSpaceNodeState::Waiting
        },
        content: String::new(),
        parents,
        actions,
    }
}

async fn runtime() -> (std::path::PathBuf, Arc<StateRuntime>) {
    let home = std::env::temp_dir().join(format!("taskspace-settlement-{}", Uuid::new_v4()));
    let runtime = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    (home, runtime)
}

async fn initialized() -> (std::path::PathBuf, Arc<StateRuntime>, ThreadId, String) {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(map(&map_id)),
            commit_id: "create".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");
    (home, runtime, owner, map_id)
}

fn request(
    owner: ThreadId,
    map_id: &str,
    outcome: TaskSpaceActionOutcome,
) -> SettleTaskSpaceActionRequest {
    SettleTaskSpaceActionRequest {
        map_id: map_id.to_string(),
        commit_id: "settle-call-1".to_string(),
        mutation_id: "settle-call-1".to_string(),
        action_id: "call-1".to_string(),
        node_ids: vec!["work".to_string()],
        tool_name: "inspect".to_string(),
        outcome,
        operation: "taskspace_exec_settle".to_string(),
        actor_thread_id: owner,
    }
}

#[tokio::test]
async fn outcome_only_settlement_is_idempotent() {
    let (home, runtime, owner, map_id) = initialized().await;
    let request = request(owner, &map_id, TaskSpaceActionOutcome::Succeeded);
    let first = runtime
        .settle_taskspace_action_outcome(request.clone())
        .await
        .expect("settle action");
    assert!(matches!(first, TaskSpaceMapWriteOutcome::Applied(_)));
    let replay = runtime
        .settle_taskspace_action_outcome(request)
        .await
        .expect("replay action settlement");
    assert!(matches!(
        replay,
        TaskSpaceMapWriteOutcome::IdempotentReplay(_)
    ));
    let record = runtime.load_taskspace_map(&map_id).await.unwrap().unwrap();
    let map = record.canonical_map.unwrap();
    assert_eq!(map.revision, 2);
    assert_eq!(
        map.work_nodes[0].actions[0].outcome,
        TaskSpaceActionOutcome::Succeeded
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn outcome_only_settlement_rejects_wrong_attribution_without_mutating_map() {
    let (home, runtime, owner, map_id) = initialized().await;
    let mut request = request(owner, &map_id, TaskSpaceActionOutcome::Succeeded);
    request.node_ids = vec!["other".to_string()];
    let error = runtime
        .settle_taskspace_action_outcome(request)
        .await
        .expect_err("wrong node attribution must fail");
    assert!(error.to_string().contains("node attribution mismatch"));
    let record = runtime.load_taskspace_map(&map_id).await.unwrap().unwrap();
    assert_eq!(record.store_revision, 1);
    assert_eq!(
        record.canonical_map.unwrap().work_nodes[0].actions[0].outcome,
        TaskSpaceActionOutcome::Pending
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn outcome_only_settlement_waits_past_configured_busy_timeout() {
    let (home, runtime, owner, map_id) = initialized().await;
    let blocker = runtime.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let settling = {
        let runtime = Arc::clone(&runtime);
        let request = request(owner, &map_id, TaskSpaceActionOutcome::Succeeded);
        tokio::spawn(async move { runtime.settle_taskspace_action_outcome(request).await })
    };
    tokio::time::sleep(Duration::from_millis(5_200)).await;
    blocker.rollback().await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(3), settling)
        .await
        .expect("settlement should continue after the first busy timeout")
        .expect("settlement task should join")
        .expect("settlement should succeed");
    assert!(matches!(outcome, TaskSpaceMapWriteOutcome::Applied(_)));
    let _ = tokio::fs::remove_dir_all(home).await;
}
