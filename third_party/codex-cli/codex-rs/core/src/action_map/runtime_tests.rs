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
    let (state, _, outcome) = initialized_state(
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
    assert_eq!(outcome.delta.map_id, "map-1");
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
fn finish_end_is_agent_explicit_and_closes_root_and_finish_together() {
    let (mut state, owner, _) = initialized_state(
        &[("work", "Implement the change")],
        &[("root", "work"), ("work", "finish")],
        "work",
    );
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
        .finish_end_for_main(owner, 3, summary.clone())
        .expect("ready finish closes explicitly");
    assert_eq!(outcome.final_summary, summary);
    assert_eq!(outcome.delta.committed_revision, 4);
    assert_eq!(outcome.delta.graph_revision_batches.len(), 1);
    assert!(matches!(
        events.first(),
        Some(MapRuntimeEvent::GraphRevisionCommitted(event))
            if event.operation == "finish_end" && event.revision == 4
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
fn projection_reads_canonical_graph_without_task_or_map_status() {
    let (mut state, _, _) = initialized_state(
        &[("inspect", "Inspect the code")],
        &[("root", "inspect"), ("inspect", "finish")],
        "inspect",
    );
    let projection = state.build_developer_context().expect("projection");

    assert!(projection.contains("TaskSpaceMapEpochSnapshotR6V1"));
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
