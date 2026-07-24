use super::*;

fn initialized_state(
    work_nodes: &[(&str, &str)],
    edges: &[(&str, &str)],
    current_node_id: &str,
) -> (ActionMapRuntimeState, ThreadId, ActionMapInitializeOutcome) {
    let (current_node_id, current_node_goal) = work_nodes
        .iter()
        .find(|(id, _)| *id == current_node_id)
        .expect("current work node is declared");
    let owner = ThreadId::new();
    let mut state = ActionMapRuntimeState::default();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    let (outcome, _) = state
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                root: ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "Solve the requested task".into(),
                },
                current_work_node: ActionMapInitializeNodeInput {
                    id: (*current_node_id).into(),
                    goal: (*current_node_goal).into(),
                },
                finish: ActionMapInitializeFinishInput {
                    id: "finish".into(),
                },
                work_nodes: work_nodes
                    .iter()
                    .filter(|(id, _)| id != current_node_id)
                    .map(|(id, goal)| ActionMapInitializeNodeInput {
                        id: (*id).into(),
                        goal: (*goal).into(),
                    })
                    .collect(),
                edges: edges
                    .iter()
                    .map(|(from, to)| ActionMapEdgeInput {
                        from: (*from).into(),
                        to: (*to).into(),
                    })
                    .collect(),
                source_event_ids: vec!["task-event-root".into()],
            },
        )
        .expect("valid rooted map initializes");
    (state, owner, outcome)
}

#[test]
fn initialization_exposes_one_root_one_finish_and_revision_events() {
    let (state, owner, outcome) = initialized_state(
        &[("inspect", "Inspect the code")],
        &[("root", "inspect"), ("inspect", "finish")],
        "inspect",
    );
    let snapshot = state.snapshot();
    let map = snapshot.map.expect("R6 snapshot has one map");

    assert_eq!(map.root_node_id, "root");
    assert_eq!(map.finish_node_id, "finish");
    assert_eq!(map.revision, 2);
    assert_eq!(map.current_node_id.as_deref(), Some("inspect"));
    assert_eq!(
        map.nodes
            .iter()
            .filter(|node| node.role == "task_root")
            .count(),
        1
    );
    assert_eq!(
        map.nodes
            .iter()
            .filter(|node| node.role == "finish")
            .count(),
        1
    );
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.role == "finish")
            .unwrap()
            .goal,
        ""
    );
    assert_eq!(outcome.node_ids, ["root", "inspect", "finish"]);
    assert_eq!(outcome.delta.map_id, format!("map-{owner}"));
    assert_eq!(outcome.delta.committed_revision, 2);
    assert_eq!(outcome.delta.graph_revision_batches.len(), 2);
    assert_eq!(outcome.delta.graph_revision_batches[0].revision, 1);
    assert_eq!(outcome.delta.graph_revision_batches[1].revision, 2);
}

#[test]
fn non_ready_current_work_rejection_is_atomic_and_reports_prestate_revision() {
    let owner = ThreadId::new();
    let mut state = ActionMapRuntimeState::default();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    let before = state.snapshot();
    let error = state
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                root: ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "Solve the task".into(),
                },
                current_work_node: ActionMapInitializeNodeInput {
                    id: "implement".into(),
                    goal: "Implement after inspection".into(),
                },
                finish: ActionMapInitializeFinishInput {
                    id: "finish".into(),
                },
                work_nodes: vec![ActionMapInitializeNodeInput {
                    id: "inspect".into(),
                    goal: "Inspect first".into(),
                }],
                edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "inspect".into(),
                    },
                    ActionMapEdgeInput {
                        from: "inspect".into(),
                        to: "implement".into(),
                    },
                    ActionMapEdgeInput {
                        from: "implement".into(),
                        to: "finish".into(),
                    },
                ],
                source_event_ids: vec!["task-event-root".into()],
            },
        )
        .expect_err("pending work cannot be the initial binding");

    let rejection: serde_json::Value = serde_json::from_str(&error).expect("typed rejection");
    assert_eq!(rejection["state_commit"], false);
    assert_eq!(rejection["current_revision"], 0);
    assert_eq!(rejection["violations"][0]["code"], "transition_invalid");
    assert_eq!(state.snapshot(), before);
}

#[test]
fn invalid_graph_mutation_is_atomic() {
    let (mut state, owner, _) = initialized_state(
        &[("inspect", "Inspect the code")],
        &[("root", "inspect"), ("inspect", "finish")],
        "inspect",
    );
    let before = state.snapshot();
    let error = state
        .mutate_graph_for_main(
            owner,
            ActionMapGraphMutationInput {
                expected_revision: 2,
                add_nodes: vec![ActionMapInitializeNodeInput {
                    id: "orphan".into(),
                    goal: "This node has no path".into(),
                }],
                add_edges: Vec::new(),
                remove_edges: Vec::new(),
            },
        )
        .expect_err("orphan node must be rejected");

    assert!(error.contains("state_commit"));
    assert_eq!(state.snapshot(), before);
}

#[test]
fn complete_then_continue_commits_one_revision_and_rebinds_atomically() {
    let (mut state, owner, _) = initialized_state(
        &[("inspect", "Inspect"), ("implement", "Implement")],
        &[
            ("root", "inspect"),
            ("inspect", "implement"),
            ("implement", "finish"),
        ],
        "inspect",
    );

    let (outcome, events) = state
        .complete_then_bind_for_main(
            owner,
            2,
            "inspect".into(),
            "implement".into(),
            "task-event-handoff".into(),
        )
        .expect("completion and successor binding commit together");

    assert_eq!(outcome.revision, 3);
    assert_eq!(outcome.current_node_id, "inspect");
    assert_eq!(outcome.next_node_id, "implement");
    assert_eq!(outcome.delta.graph_revision_batches.len(), 1);
    assert!(matches!(
        events.as_slice(),
        [
            MapRuntimeEvent::GraphRevisionCommitted(graph),
            MapRuntimeEvent::LeaseReleased(_),
            MapRuntimeEvent::LeaseCreated(_)
        ] if graph.operation == "complete_then_continue" && graph.revision == 3
    ));
    let map = state.snapshot().map.expect("map remains active");
    assert_eq!(map.revision, 3);
    assert_eq!(map.current_node_id.as_deref(), Some("implement"));
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "inspect")
            .unwrap()
            .status,
        "completed"
    );
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "implement")
            .unwrap()
            .status,
        "running"
    );
    assert_eq!(map.results.len(), 1);
    assert_eq!(map.leases.len(), 1);
}

#[test]
fn rejected_complete_handoff_preserves_the_entire_prestate() {
    let (mut state, owner, _) = initialized_state(
        &[("inspect", "Inspect"), ("implement", "Implement")],
        &[
            ("root", "inspect"),
            ("inspect", "implement"),
            ("implement", "finish"),
        ],
        "inspect",
    );
    let before = state.snapshot();

    state
        .complete_then_bind_for_main(
            owner,
            2,
            "inspect".into(),
            "finish".into(),
            "task-event-invalid-handoff".into(),
        )
        .expect_err("Finish cannot be bound as a Work successor");

    assert_eq!(state.snapshot(), before);
}

#[test]
fn finish_map_closes_last_running_work_root_and_finish_in_one_revision() {
    let (mut state, owner, _) = initialized_state(
        &[("work", "Implement and verify")],
        &[("root", "work"), ("work", "finish")],
        "work",
    );
    let summary = "Implemented and verified.".to_string();

    let before = state.snapshot();
    let wrong_terminal = state
        .finish_map_for_main(
            owner,
            2,
            "root".into(),
            summary.clone(),
            "task-event-wrong-terminal".into(),
        )
        .expect_err("Task Root is not a terminal entry node");
    let wrong_terminal: serde_json::Value =
        serde_json::from_str(&wrong_terminal).expect("typed terminal identity rejection");
    assert_eq!(
        wrong_terminal["violations"][0]["code"],
        "transition_invalid"
    );
    assert_eq!(state.snapshot(), before);

    let (outcome, events) = state
        .finish_map_for_main(
            owner,
            2,
            "work".into(),
            summary.clone(),
            "task-event-terminal".into(),
        )
        .expect("final completion and explicit end commit together");

    assert_eq!(outcome.revision, 3);
    assert_eq!(outcome.terminal_node_id, "work");
    assert_eq!(outcome.terminal_node_role, "work");
    assert_eq!(outcome.final_summary, summary);
    assert!(matches!(
        events.first(),
        Some(MapRuntimeEvent::GraphRevisionCommitted(graph))
            if graph.operation == "finish_map" && graph.revision == 3
    ));
    let map = state
        .snapshot()
        .map
        .expect("closed map remains inspectable");
    assert!(map.complete);
    assert_eq!(map.revision, 3);
    assert_eq!(map.current_node_id, None);
    assert_eq!(map.results.len(), 1);
    assert!(map.leases.is_empty());
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "root")
            .unwrap()
            .status,
        "closed"
    );
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "finish")
            .unwrap()
            .status,
        "closed"
    );
}

#[test]
fn rejected_finish_map_reports_live_revision_and_preserves_prestate() {
    let (mut state, owner, _) = initialized_state(
        &[("first", "First branch"), ("second", "Second branch")],
        &[
            ("root", "first"),
            ("root", "second"),
            ("first", "finish"),
            ("second", "finish"),
        ],
        "first",
    );
    let before = state.snapshot();

    let error = state
        .finish_map_for_main(
            owner,
            2,
            "first".into(),
            "Too early".into(),
            "task-event-premature-terminal".into(),
        )
        .expect_err("unfinished parallel work prevents terminal completion");
    let rejection: serde_json::Value = serde_json::from_str(&error).expect("typed rejection");

    assert_eq!(rejection["state_commit"], false);
    assert_eq!(rejection["current_revision"], 2);
    assert_eq!(state.snapshot(), before);
}

#[test]
fn finish_map_accepts_a_ready_finish_without_active_work() {
    let (mut state, owner, _) = initialized_state(
        &[("work", "Implement the change")],
        &[("root", "work"), ("work", "finish")],
        "work",
    );
    let running_snapshot = state.snapshot();
    let rejection = state
        .finish_map_for_main(
            owner,
            2,
            "finish".into(),
            "Too early".into(),
            "task-event-premature-finish".into(),
        )
        .expect_err("finish_map must not close a pending Finish");
    let rejection: serde_json::Value =
        serde_json::from_str(&rejection).expect("typed terminal rejection");
    assert_eq!(rejection["state_commit"], false);
    assert_eq!(rejection["violations"][0]["code"], "finish_not_ready");
    assert_eq!(state.snapshot(), running_snapshot);

    let (transition_outcome, transition_events) = state
        .transition_node_for_main(
            owner,
            2,
            "work".into(),
            NodeTransition::Complete,
            "task-event-complete".into(),
        )
        .expect("work completion commits");
    assert!(matches!(
        transition_events.first(),
        Some(MapRuntimeEvent::GraphRevisionCommitted(event)) if event.revision == 3
    ));
    assert_eq!(transition_outcome.delta.committed_revision, 3);
    assert_eq!(transition_outcome.delta.graph_revision_batches.len(), 1);
    assert_eq!(
        transition_outcome.delta.graph_revision_batches[0].events,
        match &transition_events[0] {
            MapRuntimeEvent::GraphRevisionCommitted(event) => event.events.clone(),
            _ => panic!("expected canonical graph revision event"),
        }
    );
    let before_finish = state.snapshot().map.expect("active map remains visible");
    assert!(!before_finish.complete);
    assert_eq!(
        before_finish
            .nodes
            .iter()
            .find(|n| n.id == "root")
            .unwrap()
            .status,
        "open"
    );

    let summary = "Implemented and verified exactly as requested.".to_string();
    let (outcome, events) = state
        .finish_map_for_main(
            owner,
            3,
            "finish".into(),
            summary.clone(),
            "task-event-ready-finish".into(),
        )
        .expect("ready finish closes explicitly");
    assert_eq!(outcome.terminal_node_id, "finish");
    assert_eq!(outcome.terminal_node_role, "finish");
    assert_eq!(outcome.final_summary, summary);
    assert_eq!(outcome.delta.committed_revision, 4);
    assert_eq!(outcome.delta.graph_revision_batches.len(), 1);
    assert!(matches!(
        events.first(),
        Some(MapRuntimeEvent::GraphRevisionCommitted(event))
            if event.operation == "finish_map" && event.revision == 4
    ));
    let closed = state
        .snapshot()
        .map
        .expect("closed map remains inspectable");
    assert!(closed.complete);
    assert_eq!(
        closed.nodes.iter().find(|n| n.id == "root").unwrap().status,
        "closed"
    );
    assert_eq!(
        closed
            .nodes
            .iter()
            .find(|n| n.id == "finish")
            .unwrap()
            .status,
        "closed"
    );
}

#[test]
fn finish_map_closes_ready_finish_after_last_subagent_result() {
    let (mut state, owner, _) = initialized_state(
        &[
            ("setup", "Prepare the delegated work"),
            ("delegated", "Complete the final verification"),
        ],
        &[
            ("root", "setup"),
            ("setup", "delegated"),
            ("delegated", "finish"),
        ],
        "setup",
    );
    state
        .transition_node_for_main(
            owner,
            2,
            "setup".into(),
            NodeTransition::Complete,
            "task-event-setup".into(),
        )
        .expect("setup completion makes delegated work ready");
    let (assignment, _) = state
        .prepare_spawn_assignment(owner, "delegated", Some("delegated"))
        .expect("ready final work can be delegated");
    let assignment = assignment.expect("TaskSpace creates a subagent assignment");
    let child = ThreadId::new();
    state
        .attach_agent_to_lease(&assignment.lease_id, child, None)
        .expect("subagent attaches to its lease");
    state
        .record_child_result(child, &AgentStatus::Completed(Some("verified".into())))
        .expect("subagent result completes the final Work");

    let ready = state.snapshot().map.expect("open Map remains visible");
    assert!(ready.finish_ready);
    assert_eq!(ready.current_node_id, None);
    assert!(!ready.complete);

    let (outcome, _) = state
        .finish_map_for_main(
            owner,
            ready.revision,
            "finish".into(),
            "Delegated verification completed.".into(),
            "task-event-subagent-finish".into(),
        )
        .expect("Agent explicitly closes the Ready Finish");
    assert_eq!(outcome.terminal_node_role, "finish");
    assert!(state.snapshot().map.expect("closed Map").complete);
}

#[test]
fn store_restore_accepts_a_completed_map_without_an_active_binding() {
    let (mut state, owner, _) = initialized_state(
        &[("work", "Implement and verify")],
        &[("root", "work"), ("work", "finish")],
        "work",
    );
    state
        .finish_map_for_main(
            owner,
            2,
            "work".into(),
            "Verified.".into(),
            "task-event-terminal".into(),
        )
        .expect("finish map");
    let snapshot = state.snapshot();
    let map_id = snapshot.map.as_ref().expect("completed map").id.clone();
    let mut restored = ActionMapRuntimeState::default();

    restored
        .restore_store_snapshot(&map_id, owner, snapshot.clone())
        .expect("completed canonical Store map must restore");

    let restored_snapshot = restored.snapshot();
    let restored_map = restored_snapshot
        .map
        .as_ref()
        .expect("completed map retained");
    assert!(restored_map.complete);
    assert_eq!(restored_map.id, map_id);
    assert_eq!(restored_map.owner_session_id, Some(owner));
    assert_eq!(restored_snapshot, snapshot);
}

#[test]
fn projection_reads_canonical_graph_without_task_or_map_status() {
    let (mut state, _, _) = initialized_state(
        &[("inspect", "Inspect the code")],
        &[("root", "inspect"), ("inspect", "finish")],
        "inspect",
    );
    let projection = state
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .expect("projection");

    assert!(projection.contains("TaskSpaceMapProjectionR7V1"));
    assert!(projection.contains("projection_kind: current_projection"));
    assert!(projection.contains("canonical_sha256:"));
    assert!(projection.contains("root_node_id: root"));
    assert!(projection.contains("finish_node_id: finish"));
    assert!(projection.contains("inspect role=work status=running"));
    assert!(!projection.contains("task_status"));
    assert!(!projection.contains("map_status"));
    assert!(!projection.contains("kind="));
}

#[test]
fn legacy_snapshot_schema_is_fatal_without_guessing_migration() {
    let mut state = ActionMapRuntimeState::default();
    let snapshot = ActionMapSnapshot {
        schema_version: "TaskSpaceSnapshotR5V1".into(),
        mode: MapRuntimeMode::Experiment,
        routing_required: false,
        bootstrap_required: false,
        reborn_requested: false,
        map: None,
        maintenance_barriers: Vec::new(),
        trace_summary: Default::default(),
        trace_events: Vec::new(),
        sentinel_summary: Default::default(),
        sentinel_warnings: Vec::new(),
    };

    let error = state
        .restore_snapshot(snapshot)
        .expect_err("legacy schema must fail");
    assert!(error.contains("legacy_schema_unsupported"));
}
