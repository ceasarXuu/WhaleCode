use super::StateRuntime;
use crate::BindTaskSpaceMapRequest;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use crate::TaskSpaceMapRelation;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotSentinelSummary;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;
use codex_protocol::protocol::MapRuntimeMode;
use std::sync::Arc;
use uuid::Uuid;

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

fn initialized_snapshot(map_id: &str, revision: u64) -> ActionMapSnapshot {
    let mut snapshot = blank_snapshot();
    snapshot.bootstrap_required = false;
    snapshot.map = Some(ActionMapSnapshotMap {
        id: map_id.to_string(),
        task_id: Some("task-1".to_string()),
        owner_session_id: Some(ThreadId::new()),
        root_node_id: "root".to_string(),
        finish_node_id: "finish".to_string(),
        revision,
        current_node_id: Some("work".to_string()),
        terminal_summary_ref: None,
        complete: false,
        ready_work_node_count: 0,
        running_work_node_count: 1,
        completed_work_node_count: 0,
        finish_ready: false,
        nodes: Vec::new(),
        edges: Vec::new(),
        leases: Vec::new(),
        results: Vec::new(),
        node_events: Vec::new(),
    });
    snapshot
}

fn temp_home() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("codex-taskspace-map-test-{}", Uuid::new_v4()))
}

async fn runtime() -> (std::path::PathBuf, Arc<StateRuntime>) {
    let home = temp_home();
    let runtime = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    (home, runtime)
}

#[tokio::test]
async fn create_load_bind_and_commit_taskspace_map() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let fork = ThreadId::new();
    let map_id = format!("map-{owner}");
    let created = runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            snapshot: blank_snapshot(),
            commit_id: "create-1".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");
    let TaskSpaceMapWriteOutcome::Applied(created) = created else {
        panic!("map creation should apply");
    };
    assert_eq!(created.store_revision, 1);
    assert_eq!(created.graph_revision, 0);

    runtime
        .bind_thread_to_taskspace_map(BindTaskSpaceMapRequest {
            thread_id: fork,
            map_id: map_id.clone(),
            relation: TaskSpaceMapRelation::Fork,
            parent_thread_id: Some(owner),
            node_id: None,
            lease_id: None,
        })
        .await
        .expect("bind fork");
    let (loaded, binding) = runtime
        .load_taskspace_map_for_thread(fork)
        .await
        .expect("load fork map")
        .expect("fork binding exists");
    assert_eq!(loaded.map_id, map_id);
    assert_eq!(binding.relation, TaskSpaceMapRelation::Fork);

    let committed = runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: loaded.map_id.clone(),
            expected_store_revision: loaded.store_revision,
            snapshot: initialized_snapshot(&loaded.map_id, 3),
            commit_id: "commit-1".to_string(),
            operation: "initialize_map".to_string(),
            actor_thread_id: fork,
        })
        .await
        .expect("commit map");
    let TaskSpaceMapWriteOutcome::Applied(committed) = committed else {
        panic!("map commit should apply");
    };
    assert_eq!(committed.store_revision, 2);
    assert_eq!(committed.graph_revision, 3);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn taskspace_map_commit_is_idempotent_and_rejects_key_reuse() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let create = CreateTaskSpaceMapRequest {
        map_id: map_id.clone(),
        owner_thread_id: owner,
        snapshot: blank_snapshot(),
        commit_id: "create-idempotent".to_string(),
        operation: "activate_taskspace".to_string(),
    };
    runtime
        .create_taskspace_map(create.clone())
        .await
        .expect("create");
    assert!(matches!(
        runtime.create_taskspace_map(create).await.expect("replay"),
        TaskSpaceMapWriteOutcome::IdempotentReplay(_)
    ));

    let commit = CommitTaskSpaceMapRequest {
        map_id: map_id.clone(),
        expected_store_revision: 1,
        snapshot: initialized_snapshot(&map_id, 1),
        commit_id: "commit-idempotent".to_string(),
        operation: "initialize_map".to_string(),
        actor_thread_id: owner,
    };
    runtime
        .compare_and_swap_taskspace_map(commit.clone())
        .await
        .expect("commit");
    assert!(matches!(
        runtime
            .compare_and_swap_taskspace_map(commit.clone())
            .await
            .expect("commit replay"),
        TaskSpaceMapWriteOutcome::IdempotentReplay(_)
    ));
    let mut reused = commit;
    reused.operation = "different_operation".to_string();
    let error = runtime
        .compare_and_swap_taskspace_map(reused)
        .await
        .expect_err("changed input must fail");
    assert!(error.to_string().contains("reused with different input"));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn concurrent_taskspace_map_writers_have_one_winner() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            snapshot: blank_snapshot(),
            commit_id: "create-concurrent".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create");
    let request = |commit_id: &str, revision: u64| CommitTaskSpaceMapRequest {
        map_id: map_id.clone(),
        expected_store_revision: 1,
        snapshot: initialized_snapshot(&map_id, revision),
        commit_id: commit_id.to_string(),
        operation: "concurrent_test".to_string(),
        actor_thread_id: owner,
    };
    let (left, right) = tokio::join!(
        runtime.compare_and_swap_taskspace_map(request("left", 1)),
        runtime.compare_and_swap_taskspace_map(request("right", 2))
    );
    let outcomes = [left.expect("left result"), right.expect("right result")];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TaskSpaceMapWriteOutcome::Applied(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TaskSpaceMapWriteOutcome::Conflict { .. }))
            .count(),
        1
    );
    let loaded = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load")
        .expect("map");
    assert_eq!(loaded.store_revision, 2);
    let _ = tokio::fs::remove_dir_all(home).await;
}
