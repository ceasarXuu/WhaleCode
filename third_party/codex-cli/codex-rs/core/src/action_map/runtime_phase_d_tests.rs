use super::*;

fn initialized_chain() -> (ActionMapRuntimeState, ThreadId) {
    let owner = ThreadId::new();
    let mut state = ActionMapRuntimeState::default();
    state.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    state
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                root: ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "Solve the task".into(),
                },
                current_work_node: ActionMapInitializeNodeInput {
                    id: "first".into(),
                    goal: "First work".into(),
                },
                finish: ActionMapInitializeFinishInput {
                    id: "finish".into(),
                },
                work_nodes: vec![ActionMapInitializeNodeInput {
                    id: "second".into(),
                    goal: "Second work".into(),
                }],
                edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "first".into(),
                    },
                    ActionMapEdgeInput {
                        from: "first".into(),
                        to: "second".into(),
                    },
                    ActionMapEdgeInput {
                        from: "second".into(),
                        to: "finish".into(),
                    },
                ],
                source_event_ids: vec!["task-event".into()],
            },
        )
        .unwrap();
    (state, owner)
}

fn transition(
    state: &mut ActionMapRuntimeState,
    owner: ThreadId,
    revision: u64,
    node_id: &str,
    action: NodeTransition,
) {
    state
        .transition_node_for_main(
            owner,
            revision,
            node_id.into(),
            action,
            format!("event-{revision}-{node_id}"),
        )
        .unwrap();
}

#[test]
fn rework_preserves_result_history_and_recomputes_the_frontier() {
    let (mut state, owner) = initialized_chain();
    transition(&mut state, owner, 2, "first", NodeTransition::Complete);
    transition(&mut state, owner, 3, "first", NodeTransition::Rework);

    let snapshot = state.snapshot().map.unwrap();
    let first = snapshot
        .nodes
        .iter()
        .find(|node| node.id == "first")
        .unwrap();
    let second = snapshot
        .nodes
        .iter()
        .find(|node| node.id == "second")
        .unwrap();
    assert_eq!(first.status, "ready");
    assert_eq!(first.active_lease, None);
    assert_eq!(first.result_ids.len(), 1);
    assert_eq!(second.status, "pending");
    assert_eq!(snapshot.current_node_id, None);
    assert_eq!(snapshot.leases, []);
}

#[test]
fn runtime_rejects_running_node_dependency_rewrite_atomically() {
    let (mut state, owner) = initialized_chain();
    let before = state.snapshot();

    let error = state
        .mutate_graph_for_main(
            owner,
            ActionMapGraphMutationInput {
                expected_revision: 2,
                add_nodes: vec![ActionMapInitializeNodeInput {
                    id: "late".into(),
                    goal: "Late prerequisite".into(),
                }],
                add_edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "late".into(),
                    },
                    ActionMapEdgeInput {
                        from: "late".into(),
                        to: "first".into(),
                    },
                ],
                remove_edges: Vec::new(),
            },
        )
        .unwrap_err();

    let rejection: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(rejection["state_commit"], false);
    assert_eq!(rejection["partial_commit"], false);
    assert_eq!(
        rejection["violations"][0]["code"],
        "execution_causality_conflict"
    );
    assert_eq!(state.snapshot(), before);
}

#[test]
fn runtime_stale_graph_mutation_does_not_overwrite_the_winner() {
    let (mut state, owner) = initialized_chain();
    transition(&mut state, owner, 2, "first", NodeTransition::Complete);
    state
        .mutate_graph_for_main(
            owner,
            ActionMapGraphMutationInput {
                expected_revision: 3,
                add_nodes: vec![ActionMapInitializeNodeInput {
                    id: "side".into(),
                    goal: "Side work".into(),
                }],
                add_edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "side".into(),
                    },
                    ActionMapEdgeInput {
                        from: "side".into(),
                        to: "finish".into(),
                    },
                ],
                remove_edges: Vec::new(),
            },
        )
        .unwrap();
    let winner = state.snapshot();

    let error = state
        .mutate_graph_for_main(
            owner,
            ActionMapGraphMutationInput {
                expected_revision: 3,
                add_nodes: Vec::new(),
                add_edges: Vec::new(),
                remove_edges: Vec::new(),
            },
        )
        .unwrap_err();

    let rejection: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(rejection["violations"][0]["code"], "stale_revision");
    assert_eq!(state.snapshot(), winner);
}

#[test]
fn runtime_rework_rejects_consumed_results_atomically() {
    let (mut state, owner) = initialized_chain();
    transition(&mut state, owner, 2, "first", NodeTransition::Complete);
    transition(&mut state, owner, 3, "second", NodeTransition::Bind);
    transition(&mut state, owner, 4, "second", NodeTransition::Complete);
    let before = state.snapshot();

    let error = state
        .transition_node_for_main(
            owner,
            5,
            "first".into(),
            NodeTransition::Rework,
            "event-rework".into(),
        )
        .unwrap_err();

    let rejection: serde_json::Value = serde_json::from_str(&error).unwrap();
    assert_eq!(
        rejection["violations"][0]["code"],
        "execution_causality_conflict"
    );
    assert_eq!(state.snapshot(), before);
}

#[test]
fn control_state_exposes_only_work_nodes_as_the_active_frontier() {
    let (mut state, owner) = initialized_chain();
    let projection = state
        .build_developer_context(ProjectionEnvelope::CurrentProjection)
        .unwrap();
    assert!(projection.contains("  active_frontier:\n    - first\n"));
    assert!(!projection.contains("  active_frontier:\n    - root\n"));
    assert!(!projection.contains("  active_frontier:\n    - finish\n"));
    let initial = state.control_state(None).unwrap();
    assert_eq!(initial.pending_work_node_ids, ["second"]);
    assert_eq!(initial.ready_work_node_ids, Vec::<String>::new());
    assert_eq!(initial.running_work_node_ids, ["first"]);
    assert_eq!(initial.blocked_work_node_ids, Vec::<String>::new());
    assert!(!initial.finish_ready);
    assert!(!initial.requires_named_taskspace_control());

    transition(&mut state, owner, 2, "first", NodeTransition::Complete);
    let after_first = state.control_state(None).unwrap();
    assert_eq!(after_first.ready_work_node_ids, ["second"]);
    assert_eq!(after_first.running_work_node_ids, Vec::<String>::new());

    transition(&mut state, owner, 3, "second", NodeTransition::Bind);
    transition(&mut state, owner, 4, "second", NodeTransition::Complete);
    let terminal_frontier = state.control_state(None).unwrap();
    assert_eq!(terminal_frontier.ready_work_node_ids, Vec::<String>::new());
    assert_eq!(
        terminal_frontier.running_work_node_ids,
        Vec::<String>::new()
    );
    assert!(terminal_frontier.finish_ready);
    assert!(terminal_frontier.requires_named_taskspace_control());
}

#[test]
fn finish_cannot_be_selected_as_a_worker_frontier_node() {
    let (mut state, owner) = initialized_chain();
    transition(&mut state, owner, 2, "first", NodeTransition::Complete);
    transition(&mut state, owner, 3, "second", NodeTransition::Bind);
    transition(&mut state, owner, 4, "second", NodeTransition::Complete);
    let map_id = state.active_map_id.clone().unwrap();

    let error = state
        .validate_requested_spawn_node(&map_id, "finish")
        .unwrap_err();

    assert!(error.contains("target_node_role_invalid"));
}

#[test]
fn snapshot_restore_rejects_a_finish_lease() {
    let (state, _) = initialized_chain();
    let mut snapshot = state.snapshot();
    let map = snapshot.map.as_mut().unwrap();
    let lease_id = map.leases[0].id.clone();
    let first = map
        .nodes
        .iter_mut()
        .find(|node| node.id == "first")
        .unwrap();
    first.status = "ready".into();
    first.active_lease = None;
    let finish = map
        .nodes
        .iter_mut()
        .find(|node| node.id == "finish")
        .unwrap();
    finish.active_lease = Some(lease_id);
    map.leases[0].node_id = "finish".into();
    map.current_node_id = Some("finish".into());

    let mut restored = ActionMapRuntimeState::default();
    let error = restored.restore_snapshot(snapshot).unwrap_err();

    assert!(error.contains("does not reference a running work node"));
}
