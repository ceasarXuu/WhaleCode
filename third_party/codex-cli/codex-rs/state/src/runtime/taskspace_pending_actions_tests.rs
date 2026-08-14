use super::StateRuntime;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use crate::EnqueueTaskSpacePendingProviderActionRequest;
use crate::TaskSpacePendingActionWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use codex_protocol::taskspace::TaskSpaceNodeState;
use std::sync::Arc;
use uuid::Uuid;

async fn runtime() -> (std::path::PathBuf, Arc<StateRuntime>) {
    let home = std::env::temp_dir().join(format!("codex-pending-provider-test-{}", Uuid::new_v4()));
    let runtime = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    (home, runtime)
}

fn request(thread_id: ThreadId) -> EnqueueTaskSpacePendingProviderActionRequest {
    EnqueueTaskSpacePendingProviderActionRequest {
        action_id: "provider-action-1".to_string(),
        origin_thread_id: thread_id,
        map_id: None,
        provider_response_id: "response-1".to_string(),
        provider_action_key: "response-1/web_search".to_string(),
        tool_name: "web_search".to_string(),
        outcome: TaskSpaceActionOutcome::Succeeded,
    }
}

#[tokio::test]
async fn pending_provider_action_survives_reload_and_replays_idempotently() {
    let (home, runtime) = runtime().await;
    let thread_id = ThreadId::new();
    assert_eq!(
        runtime
            .enqueue_taskspace_pending_provider_action(request(thread_id))
            .await
            .expect("insert"),
        TaskSpacePendingActionWriteOutcome::Inserted
    );
    assert_eq!(
        runtime
            .enqueue_taskspace_pending_provider_action(request(thread_id))
            .await
            .expect("replay"),
        TaskSpacePendingActionWriteOutcome::IdempotentReplay
    );
    drop(runtime);

    let runtime = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("reopen state runtime");
    let pending = runtime
        .load_taskspace_pending_provider_actions(thread_id, None)
        .await
        .expect("load pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].action_id, "provider-action-1");
    assert_eq!(pending[0].tool_name, "web_search");
    assert_eq!(pending[0].outcome, TaskSpaceActionOutcome::Succeeded);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn pending_provider_action_rejects_identity_reuse_with_different_facts() {
    let (home, runtime) = runtime().await;
    let thread_id = ThreadId::new();
    runtime
        .enqueue_taskspace_pending_provider_action(request(thread_id))
        .await
        .expect("insert");
    let mut changed = request(thread_id);
    changed.tool_name = "image_generation".to_string();
    let error = runtime
        .enqueue_taskspace_pending_provider_action(changed)
        .await
        .expect_err("identity reuse must fail");
    assert!(error.to_string().contains("reused with different facts"));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn map_commit_and_pending_attribution_are_one_transaction() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: None,
            commit_id: "create-map".into(),
            operation: "activate_taskspace".into(),
        })
        .await
        .expect("create Map");
    let mut pending = request(owner);
    pending.map_id = Some(map_id.clone());
    runtime
        .enqueue_taskspace_pending_provider_action(pending)
        .await
        .expect("enqueue pending action");

    let map = map_with_provider_action(&map_id);
    let rejected = runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: Some(map.clone()),
            commit_id: "wrong-attribution".into(),
            operation: "taskspace_exec_prepare".into(),
            actor_thread_id: owner,
            binding: None,
            consumed_pending_action_ids: vec!["unknown-action".into()],
        })
        .await
        .expect_err("mismatched pending set must reject");
    assert!(
        rejected
            .to_string()
            .contains("changed before attribution commit")
    );
    assert!(
        runtime
            .load_taskspace_map(&map_id)
            .await
            .unwrap()
            .unwrap()
            .canonical_map
            .is_none()
    );
    assert_eq!(
        runtime
            .load_taskspace_pending_provider_actions(owner, Some(&map_id))
            .await
            .unwrap()
            .len(),
        1
    );

    runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: Some(map.clone()),
            commit_id: "attribute-provider-action".into(),
            operation: "taskspace_exec_prepare".into(),
            actor_thread_id: owner,
            binding: None,
            consumed_pending_action_ids: vec!["provider-action-1".into()],
        })
        .await
        .expect("commit attribution");
    assert!(
        runtime
            .load_taskspace_pending_provider_actions(owner, Some(&map_id))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        runtime
            .load_taskspace_map(&map_id)
            .await
            .unwrap()
            .unwrap()
            .canonical_map,
        Some(map)
    );

    let _ = tokio::fs::remove_dir_all(home).await;
}

fn map_with_provider_action(map_id: &str) -> TaskSpaceCanonicalMap {
    let node = |node_id: &str, state, parents: Vec<String>, actions| TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: node_id.into(),
        state,
        content: String::new(),
        parents,
        actions,
    };
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id: map_id.into(),
        root: node("root", TaskSpaceNodeState::InFlight, Vec::new(), Vec::new()),
        work_nodes: vec![node(
            "work",
            TaskSpaceNodeState::Ready,
            vec!["root".into()],
            vec![TaskSpaceNodeAction {
                action_id: "provider-action-1".into(),
                tool_name: "web_search".into(),
                outcome: TaskSpaceActionOutcome::Succeeded,
            }],
        )],
        finish: node(
            "finish",
            TaskSpaceNodeState::Waiting,
            vec!["work".into()],
            Vec::new(),
        ),
        revision: 1,
    }
}
