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
                finish: ActionMapInitializeNodeInput {
                    id: "finish".into(),
                    goal: "Finish".into(),
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
    assert_eq!(rejection["partial_commit"], 0);
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
