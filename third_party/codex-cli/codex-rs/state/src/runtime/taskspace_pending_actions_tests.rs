use super::StateRuntime;
use crate::EnqueueTaskSpacePendingProviderActionRequest;
use crate::TaskSpacePendingActionWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
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
        provider_item_id: "web-search-1".to_string(),
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
