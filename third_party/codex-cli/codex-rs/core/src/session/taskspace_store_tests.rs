use super::taskspace_store::hydrate_action_map_store;
use crate::action_map::ActionMapRuntimeState;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::ResumedHistory;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::TaskSpaceMapRelation;
use codex_state::TaskSpaceMapWriteOutcome;
use uuid::Uuid;

#[tokio::test]
async fn resume_and_child_hydrate_the_same_canonical_map() {
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
        &SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id: owner,
            depth: 1,
            agent_path: Some(AgentPath::root().join("worker").expect("agent path")),
            agent_nickname: None,
            agent_role: None,
        }),
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
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn persisted_taskspace_session_cannot_recover_from_rollout_without_store() {
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
    assert!(error.to_string().contains("cannot recover from rollout"));
}
