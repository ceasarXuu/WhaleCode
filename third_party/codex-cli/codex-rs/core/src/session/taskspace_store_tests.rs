use super::taskspace_store::canonical_map_for_store;
use super::taskspace_store::hydrate_action_map_store;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::MapEdge;
use crate::action_map::ProjectionEnvelope;
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
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceCompletionRecord;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceTerminalRecord;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::TaskSpaceMapRelation;
use codex_state::TaskSpaceMapWriteOutcome;
use tracing_test::traced_test;
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

fn map_node(node_id: &str, goal: &str) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: goal.into(),
        source_refs: Vec::new(),
    }
}

fn multi_parent_map(map_id: &str) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id: map_id.into(),
        root: map_node("root", "deliver"),
        work_nodes: vec![
            map_node("inspect", "inspect"),
            map_node("research", "research"),
            map_node("implement", "implement"),
        ],
        finish: map_node("finish", "finish"),
        edges: vec![
            MapEdge {
                from: "root".into(),
                to: "inspect".into(),
            },
            MapEdge {
                from: "root".into(),
                to: "research".into(),
            },
            MapEdge {
                from: "inspect".into(),
                to: "implement".into(),
            },
            MapEdge {
                from: "research".into(),
                to: "implement".into(),
            },
            MapEdge {
                from: "implement".into(),
                to: "finish".into(),
            },
        ],
        completion_records: Default::default(),
        block_records: Default::default(),
        action_records: Default::default(),
        result_refs: Default::default(),
        evidence_refs: Default::default(),
        terminal_record: None,
        terminal_history: Vec::new(),
        revision: 1,
    }
}

fn completed_multi_parent_map(map_id: &str, reopened: bool) -> TaskSpaceCanonicalMap {
    let mut map = multi_parent_map(map_id);
    for node_id in ["inspect", "research", "implement"] {
        map.completion_records.insert(
            node_id.into(),
            TaskSpaceCompletionRecord {
                action_id: format!("complete-{node_id}"),
                result_ref_ids: Vec::new(),
                evidence_ref_ids: Vec::new(),
            },
        );
    }
    let terminal = TaskSpaceTerminalRecord {
        action_id: "finish-map".into(),
        summary_ref: "summary://final".into(),
    };
    if reopened {
        map.terminal_history.push(terminal);
        map.revision = 3;
    } else {
        map.terminal_record = Some(terminal);
        map.revision = 2;
    }
    map
}

fn forked_history(parent_thread_id: ThreadId) -> InitialHistory {
    InitialHistory::Forked(vec![RolloutItem::SessionMeta(SessionMetaLine {
        meta: SessionMeta {
            id: parent_thread_id,
            ..Default::default()
        },
        git: None,
    })])
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
            canonical_map: canonical_map_for_store(&runtime),
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
    assert_eq!(
        canonical_map_for_store(&resumed.runtime),
        canonical_map_for_store(&runtime)
    );

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
        &forked_history(owner),
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
    let owner_projection = runtime
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .expect("owner projection");
    let resumed_projection = resumed
        .runtime
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .expect("resumed projection");
    let child_projection = child_hydrated
        .runtime
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .expect("child projection");
    let fork_projection = fork_hydrated
        .runtime
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .expect("fork projection");
    assert_eq!(resumed_projection, owner_projection);
    assert_eq!(child_projection, owner_projection);
    assert_eq!(fork_projection, owner_projection);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
#[traced_test]
async fn invalid_parent_map_is_rejected_before_child_and_fork_binding() {
    let home = std::env::temp_dir().join(format!(
        "codex-taskspace-invalid-parent-test-{}",
        Uuid::new_v4()
    ));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let mut invalid = multi_parent_map(&map_id);
    invalid.edges.push(MapEdge {
        from: "finish".into(),
        to: "root".into(),
    });
    state_db
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id,
            owner_thread_id: owner,
            canonical_map: Some(invalid),
            commit_id: "create-invalid-parent-map".to_string(),
            operation: "test_invalid_parent".to_string(),
        })
        .await
        .expect("persist storage-consistent invalid Map");

    let resume_result = hydrate_action_map_store(
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
    .await;
    let resume_error = resume_result
        .err()
        .expect("resume must reject invalid canonical Map");
    assert!(resume_error.to_string().contains("cycle_detected"));

    for (actor, history, source) in [
        (ThreadId::new(), InitialHistory::New, child_source(owner)),
        (ThreadId::new(), forked_history(owner), SessionSource::Exec),
    ] {
        let result =
            hydrate_action_map_store(Some(&state_db), actor, &history, &source, true).await;
        let error = result
            .err()
            .expect("child/fork must reject invalid parent Map");
        assert!(error.to_string().contains("cycle_detected"));
        assert!(
            state_db
                .load_taskspace_map_for_thread(actor)
                .await
                .expect("query rejected binding")
                .is_none(),
            "failed hydrate must not leave a child/fork binding"
        );
    }
    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("taskspace.map_store_hydrate_rejected")
                    && line.contains("reason_code=\"canonical_map_invalid\"")
                    && line.contains("store_revision=1")
                    && line.contains("map_revision=1")
                    && !line.contains("deliver")
            })
            .map(|_| Ok(()))
            .unwrap_or_else(|| {
                Err("expected content-free canonical Map hydrate rejection event".to_string())
            })
    });
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn persisted_multi_parent_map_hydrates_without_graph_rewrite() {
    let home = std::env::temp_dir().join(format!(
        "codex-taskspace-multi-parent-test-{}",
        Uuid::new_v4()
    ));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let canonical_map = multi_parent_map(&map_id);
    state_db
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(canonical_map.clone()),
            commit_id: "create-multi-parent-map".to_string(),
            operation: "test_multi_parent".to_string(),
        })
        .await
        .expect("persist multi-parent Map");

    let hydrated = hydrate_action_map_store(
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
    .expect("hydrate multi-parent Map")
    .expect("hydrated Map handle");

    assert_eq!(hydrated.handle.map_id, map_id);
    assert_eq!(
        canonical_map_for_store(&hydrated.runtime),
        Some(canonical_map)
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn persisted_closed_and_reopened_maps_hydrate_without_lifecycle_rewrite() {
    let home =
        std::env::temp_dir().join(format!("codex-taskspace-lifecycle-test-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");

    for reopened in [false, true] {
        let owner = ThreadId::new();
        let map_id = format!("map-{owner}");
        let canonical_map = completed_multi_parent_map(&map_id, reopened);
        state_db
            .create_taskspace_map(CreateTaskSpaceMapRequest {
                map_id: map_id.clone(),
                owner_thread_id: owner,
                canonical_map: Some(canonical_map.clone()),
                commit_id: format!("create-lifecycle-map-{reopened}"),
                operation: "test_lifecycle_hydrate".to_string(),
            })
            .await
            .expect("persist lifecycle Map");

        let hydrated = hydrate_action_map_store(
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
        .expect("hydrate lifecycle Map")
        .expect("hydrated Map handle");

        assert_eq!(
            canonical_map_for_store(&hydrated.runtime),
            Some(canonical_map)
        );
    }
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
async fn standard_child_does_not_inherit_parent_map_store_identity() {
    let home = std::env::temp_dir().join(format!(
        "codex-standard-child-map-store-test-{}",
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
            canonical_map: canonical_map_for_store(&runtime),
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
