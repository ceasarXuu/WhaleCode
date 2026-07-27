use super::StateRuntime;
use crate::BindTaskSpaceMapRequest;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use crate::TaskSpaceMapRelation;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapEdge;
use codex_protocol::taskspace::TaskSpaceMapNode;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

fn blank_map() -> Option<TaskSpaceCanonicalMap> {
    None
}

fn initialized_map(map_id: &str, revision: u64) -> Option<TaskSpaceCanonicalMap> {
    Some(TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.to_string(),
        map_id: map_id.to_string(),
        root: TaskSpaceMapNode {
            node_id: "root".to_string(),
            goal: "deliver".to_string(),
            source_refs: Vec::new(),
        },
        work_nodes: vec![TaskSpaceMapNode {
            node_id: "work".to_string(),
            goal: "implement".to_string(),
            source_refs: Vec::new(),
        }],
        finish: TaskSpaceMapNode {
            node_id: "finish".to_string(),
            goal: "finish".to_string(),
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
        revision,
    })
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
            canonical_map: blank_map(),
            commit_id: "create-1".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");
    let TaskSpaceMapWriteOutcome::Applied(created) = created else {
        panic!("map creation should apply");
    };
    assert_eq!(created.store_revision, 1);
    assert_eq!(created.map_revision, 0);

    runtime
        .bind_thread_to_taskspace_map(BindTaskSpaceMapRequest {
            thread_id: fork,
            map_id: map_id.clone(),
            relation: TaskSpaceMapRelation::Fork,
            parent_thread_id: Some(owner),
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
            canonical_map: initialized_map(&loaded.map_id, 3),
            commit_id: "commit-1".to_string(),
            operation: "initialize_map".to_string(),
            actor_thread_id: fork,
            binding: None,
        })
        .await
        .expect("commit map");
    let TaskSpaceMapWriteOutcome::Applied(committed) = committed else {
        panic!("map commit should apply");
    };
    assert_eq!(committed.store_revision, 2);
    assert_eq!(committed.map_revision, 3);
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
        canonical_map: blank_map(),
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
        canonical_map: initialized_map(&map_id, 1),
        commit_id: "commit-idempotent".to_string(),
        operation: "initialize_map".to_string(),
        actor_thread_id: owner,
        binding: None,
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
    runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 2,
            canonical_map: initialized_map(&map_id, 2),
            commit_id: "commit-after-replay".to_string(),
            operation: "advance_map".to_string(),
            actor_thread_id: owner,
            binding: None,
        })
        .await
        .expect("advance after original commit");
    assert!(matches!(
        runtime
            .compare_and_swap_taskspace_map(commit.clone())
            .await
            .expect("late commit replay"),
        TaskSpaceMapWriteOutcome::IdempotentReplay(record) if record.store_revision == 3
    ));
    let mut reused = commit.clone();
    reused.operation = "different_operation".to_string();
    let error = runtime
        .compare_and_swap_taskspace_map(reused)
        .await
        .expect_err("changed input must fail");
    assert!(error.to_string().contains("reused with different input"));

    let mut reused = commit;
    reused.binding = Some(BindTaskSpaceMapRequest {
        thread_id: ThreadId::new(),
        map_id,
        relation: TaskSpaceMapRelation::Child,
        parent_thread_id: Some(owner),
    });
    let error = runtime
        .compare_and_swap_taskspace_map(reused)
        .await
        .expect_err("changed binding must fail");
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
            canonical_map: blank_map(),
            commit_id: "create-concurrent".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create");
    let request = |commit_id: &str, revision: u64| CommitTaskSpaceMapRequest {
        map_id: map_id.clone(),
        expected_store_revision: 1,
        canonical_map: initialized_map(&map_id, revision),
        commit_id: commit_id.to_string(),
        operation: "concurrent_test".to_string(),
        actor_thread_id: owner,
        binding: None,
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

#[tokio::test]
async fn unchanged_canonical_commit_still_applies_atomic_child_binding() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let child = ThreadId::new();
    let map_id = format!("map-{owner}");
    let canonical_map = blank_map();
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: canonical_map.clone(),
            commit_id: "create-binding".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create");
    let committed = runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map,
            commit_id: "bind-child".to_string(),
            operation: "attach_child_binding".to_string(),
            actor_thread_id: owner,
            binding: Some(BindTaskSpaceMapRequest {
                thread_id: child,
                map_id: map_id.clone(),
                relation: TaskSpaceMapRelation::Child,
                parent_thread_id: Some(owner),
            }),
        })
        .await
        .expect("commit child binding");
    assert!(matches!(
        committed,
        TaskSpaceMapWriteOutcome::Applied(ref record) if record.store_revision == 2
    ));
    let (_, binding) = runtime
        .load_taskspace_map_for_thread(child)
        .await
        .expect("load child")
        .expect("child binding");
    assert_eq!(binding.parent_thread_id, Some(owner));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn failed_child_binding_rolls_back_map_and_binding_together() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let child = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: blank_map(),
            commit_id: "create-rollback".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");

    let error = runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: initialized_map(&map_id, 2),
            commit_id: "commit-invalid-binding".to_string(),
            operation: "attach_child_binding".to_string(),
            actor_thread_id: owner,
            binding: Some(BindTaskSpaceMapRequest {
                thread_id: child,
                map_id: "different-map".to_string(),
                relation: TaskSpaceMapRelation::Child,
                parent_thread_id: Some(owner),
            }),
        })
        .await
        .expect_err("cross-map binding must abort the transaction");
    assert!(error.to_string().contains("targets a different map"));

    let record = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load original map")
        .expect("original map exists");
    assert_eq!(record.store_revision, 1);
    assert!(record.canonical_map.is_none());
    assert!(
        runtime
            .load_taskspace_map_for_thread(child)
            .await
            .expect("load child binding")
            .is_none()
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn corrupted_canonical_hash_is_rejected_on_load() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: blank_map(),
            commit_id: "create-corruption".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");
    sqlx::query("UPDATE taskspace_maps SET canonical_sha256 = 'corrupt' WHERE map_id = ?")
        .bind(&map_id)
        .execute(runtime.pool.as_ref())
        .await
        .expect("inject hash corruption");

    let error = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect_err("corrupt canonical map must not load");
    assert!(error.to_string().contains("canonical hash mismatch"));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn taskspace_map_survives_state_runtime_restart() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: blank_map(),
            commit_id: "create-restart".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");
    drop(runtime);

    let reopened = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("reopen state runtime");
    let record = reopened
        .load_taskspace_map(&map_id)
        .await
        .expect("load persisted map")
        .expect("persisted map exists");
    assert_eq!(record.owner_thread_id, owner);
    assert_eq!(record.store_revision, 1);
    let _ = tokio::fs::remove_dir_all(home).await;
}
