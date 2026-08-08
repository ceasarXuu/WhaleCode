use super::taskspace_store::canonical_map_for_store;
use super::taskspace_store::hydrate_action_map_store;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::ProjectionEnvelope;
use crate::action_map::rooted_dag;
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
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use codex_protocol::taskspace::TaskSpaceNodeState;
use codex_state::CommitTaskSpaceMapRequest;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::TaskSpaceMapRelation;
use codex_state::TaskSpaceMapWriteOutcome;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
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

fn map_node(
    node_id: &str,
    goal: &str,
    state: TaskSpaceNodeState,
    parents: &[&str],
) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: goal.into(),
        state,
        content: String::new(),
        parents: parents.iter().map(|id| (*id).to_string()).collect(),
        actions: Vec::new(),
    }
}

fn multi_parent_map(map_id: &str) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id: map_id.into(),
        root: map_node("root", "deliver", TaskSpaceNodeState::InFlight, &[]),
        work_nodes: vec![
            map_node("inspect", "inspect", TaskSpaceNodeState::Ready, &["root"]),
            map_node("research", "research", TaskSpaceNodeState::Ready, &["root"]),
            map_node(
                "implement",
                "implement",
                TaskSpaceNodeState::Waiting,
                &["inspect", "research"],
            ),
        ],
        finish: map_node(
            "finish",
            "finish",
            TaskSpaceNodeState::Waiting,
            &["implement"],
        ),
        revision: 1,
    }
}

fn completed_multi_parent_map(map_id: &str, reopened: bool) -> TaskSpaceCanonicalMap {
    let mut map = multi_parent_map(map_id);
    for node in &mut map.work_nodes {
        node.state = TaskSpaceNodeState::Completed;
    }
    if reopened {
        map.finish.state = TaskSpaceNodeState::Ready;
        map.revision = 3;
    } else {
        map.root.state = TaskSpaceNodeState::Completed;
        map.finish.state = TaskSpaceNodeState::Completed;
        map.finish.content = "final summary".into();
        map.revision = 2;
    }
    map
}

#[tokio::test]
#[traced_test]
async fn factual_action_settlement_rebases_on_latest_store_head_without_losing_other_changes() {
    let home = std::env::temp_dir().join(format!("codex-taskspace-rebase-test-{}", Uuid::new_v4()));
    let state_db = codex_state::StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("initialize state runtime");
    let (mut session, _) = super::tests::make_session_and_context().await;
    session.services.state_db = Some(Arc::clone(&state_db));

    let (activation, _) = session
        .set_persisted_action_map_mode(MapRuntimeMode::Experiment)
        .await
        .expect("activate persisted TaskSpace Map");
    let map_id = activation.active_map_id.expect("mechanical Map identity");
    let mut initial = multi_parent_map(&map_id);
    initial.work_nodes[0].actions.push(TaskSpaceNodeAction {
        action_id: "client-1".into(),
        tool_name: "inspect".into(),
        outcome: TaskSpaceActionOutcome::Pending,
    });
    let install_map_id = map_id.clone();
    session
        .mutate_canonical_action_map("test_install_action", move |runtime, owner| {
            runtime
                .restore_store_map(&install_map_id, owner, Some(initial.clone()))
                .expect("install test Map");
            ((), Vec::new())
        })
        .await
        .expect("persist initial action");

    let current = state_db
        .load_taskspace_map(&map_id)
        .await
        .expect("load current Map")
        .expect("current Map record");
    let mut concurrent_map = current.canonical_map.clone().expect("canonical Map");
    concurrent_map.work_nodes[1].content = "concurrent research evidence".into();
    concurrent_map.revision += 1;
    let concurrent = state_db
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: current.store_revision,
            canonical_map: Some(concurrent_map),
            commit_id: "concurrent-map-update".into(),
            operation: "test_concurrent_update".into(),
            actor_thread_id: session.conversation_id,
            binding: None,
        })
        .await
        .expect("commit concurrent Map change");
    assert!(matches!(concurrent, TaskSpaceMapWriteOutcome::Applied(_)));

    let applications = Arc::new(AtomicUsize::new(0));
    let application_counter = Arc::clone(&applications);
    let settle_map_id = map_id.clone();
    session
        .mutate_canonical_action_map_rebased(
            "settle_taskspace_exec_action",
            "outer-1/taskspace/call/0",
            move |runtime, owner| {
                application_counter.fetch_add(1, Ordering::SeqCst);
                let current = runtime
                    .canonical_map_for_store()
                    .expect("settlement requires canonical Map");
                let commit = rooted_dag::settle_action(
                    &current,
                    "client-1",
                    "inspect",
                    rooted_dag::ActionOutcome::Succeeded,
                )
                .expect("settle pending action");
                runtime
                    .restore_store_map(&settle_map_id, owner, Some(commit.map))
                    .expect("restore settled Map");
                ((), Vec::new())
            },
        )
        .await
        .expect("rebase factual action settlement");

    assert_eq!(applications.load(Ordering::SeqCst), 2);
    let final_record = state_db
        .load_taskspace_map(&map_id)
        .await
        .expect("load final Map")
        .expect("final Map record");
    let final_map = final_record.canonical_map.expect("final canonical Map");
    let research = final_map
        .work_nodes
        .iter()
        .find(|node| node.node_id == "research")
        .expect("research node");
    assert_eq!(research.content, "concurrent research evidence");
    let inspect = final_map
        .work_nodes
        .iter()
        .find(|node| node.node_id == "inspect")
        .expect("inspect node");
    assert_eq!(
        inspect.actions[0].outcome,
        TaskSpaceActionOutcome::Succeeded
    );
    logs_assert(|lines: &[&str]| {
        lines
            .iter()
            .find(|line| {
                line.contains("taskspace.map_store_rebase_retry")
                    && line.contains("correlation_id=\"outer-1/taskspace/call/0\"")
                    && line.contains("rebase_attempt=1")
            })
            .map(|_| Ok(()))
            .unwrap_or_else(|| Err("expected factual settlement rebase event".to_string()))
    });
    let _ = tokio::fs::remove_dir_all(home).await;
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
    invalid.root.parents.push("finish".into());
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
