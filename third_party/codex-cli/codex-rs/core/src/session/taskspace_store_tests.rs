use super::taskspace_store::hydrate_action_map_store;
use crate::action_map::ActionMapRuntimeState;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::ResumedHistory;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionMeta;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::TaskSpaceMapRelation;
use codex_state::TaskSpaceMapWriteOutcome;
use uuid::Uuid;

fn child_source(parent_thread_id: ThreadId) -> SessionSource {
    SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth: 1,
        agent_path: Some(AgentPath::root().join("worker").expect("agent path")),
        agent_nickname: None,
        agent_role: None,
    })
}

#[tokio::test]
async fn resume_fork_and_child_hydrate_the_same_canonical_map() {
    let home =
        std::env::temp_dir().join(format!("codex-taskspace-hydration-test-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let owner = ThreadId::new();
    let mut runtime = ActionMapRuntimeState::default();
    let (activation, _) = runtime.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    let map_id = activation.active_map_id.expect("mechanical Map identity");
    let created = state_db
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            snapshot: runtime.snapshot(),
            commit_id: "create-hydration-map".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create canonical Map");
    assert!(matches!(created, TaskSpaceMapWriteOutcome::Applied(_)));

    let resumed = hydrate_action_map_store(
        Some(&state_db),
        owner,
        &InitialHistory::Resumed(ResumedHistory {
            conversation_id: owner,
            history: Vec::new(),
            rollout_path: None,
        }),
        &SessionSource::Exec,
        true,
    )
    .await
    .expect("hydrate resumed Map")
    .expect("resumed Map handle");
    assert_eq!(resumed.handle.map_id, map_id);
    assert_eq!(resumed.runtime.snapshot(), runtime.snapshot());

    let child = ThreadId::new();
    let child_hydrated = hydrate_action_map_store(
        Some(&state_db),
        child,
        &InitialHistory::New,
        &child_source(owner),
        true,
    )
    .await
    .expect("hydrate child Map")
    .expect("child Map handle");
    assert_eq!(child_hydrated.handle.map_id, map_id);
    assert_eq!(child_hydrated.handle.store_revision, 1);
    let (child_record, child_binding) = state_db
        .load_taskspace_map_for_thread(child)
        .await
        .expect("load child binding")
        .expect("child binding");
    assert_eq!(child_record.map_id, map_id);
    assert_eq!(child_binding.relation, TaskSpaceMapRelation::Child);
    assert_eq!(child_binding.parent_thread_id, Some(owner));

    let fork = ThreadId::new();
    let fork_hydrated = hydrate_action_map_store(
        Some(&state_db),
        fork,
        &InitialHistory::Forked(vec![RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: owner,
                ..Default::default()
            },
            git: None,
        })]),
        &SessionSource::Exec,
        true,
    )
    .await
    .expect("hydrate fork Map")
    .expect("fork Map handle");
    assert_eq!(fork_hydrated.handle.map_id, map_id);
    let (_, fork_binding) = state_db
        .load_taskspace_map_for_thread(fork)
        .await
        .expect("load fork binding")
        .expect("fork binding");
    assert_eq!(fork_binding.relation, TaskSpaceMapRelation::Fork);
    assert_eq!(fork_binding.parent_thread_id, Some(owner));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn persisted_taskspace_resume_requires_store() {
    let thread_id = ThreadId::new();
    let result = hydrate_action_map_store(
        None,
        thread_id,
        &InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        &SessionSource::Exec,
        true,
    )
    .await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("missing Store must fail"),
    };
    assert!(
        error
            .to_string()
            .contains("require a canonical Map binding")
    );
}

#[tokio::test]
async fn taskspace_child_requires_store() {
    let parent = ThreadId::new();
    let result = hydrate_action_map_store(
        None,
        ThreadId::new(),
        &InitialHistory::New,
        &child_source(parent),
        true,
    )
    .await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("TaskSpace child without Store must fail"),
    };
    assert!(
        error
            .to_string()
            .contains("require a canonical Map binding")
    );
}

#[tokio::test]
async fn taskspace_child_requires_parent_binding() {
    let home = std::env::temp_dir().join(format!(
        "codex-taskspace-missing-parent-test-{}",
        Uuid::new_v4()
    ));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let child = ThreadId::new();
    let result = hydrate_action_map_store(
        Some(&state_db),
        child,
        &InitialHistory::New,
        &child_source(ThreadId::new()),
        true,
    )
    .await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("TaskSpace child without parent binding must fail"),
    };
    assert!(error.to_string().contains("no canonical binding"));
    assert!(
        state_db
            .load_taskspace_map_for_thread(child)
            .await
            .expect("load child binding")
            .is_none()
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn standard_child_does_not_inherit_taskspace_binding() {
    let home = std::env::temp_dir().join(format!(
        "codex-standard-child-binding-test-{}",
        Uuid::new_v4()
    ));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let owner = ThreadId::new();
    let mut runtime = ActionMapRuntimeState::default();
    let (activation, _) = runtime.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    state_db
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: activation.active_map_id.expect("mechanical Map identity"),
            owner_thread_id: owner,
            snapshot: runtime.snapshot(),
            commit_id: "create-standard-parent-map".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create canonical Map");
    let child = ThreadId::new();

    let hydrated = hydrate_action_map_store(
        Some(&state_db),
        child,
        &InitialHistory::New,
        &child_source(owner),
        false,
    )
    .await
    .expect("Standard child hydration must not fail");

    assert!(hydrated.is_none());
    assert!(
        state_db
            .load_taskspace_map_for_thread(child)
            .await
            .expect("load Standard child binding")
            .is_none()
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}
