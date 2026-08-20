use super::*;

fn node(id: &str, state: NodeState, parents: &[&str]) -> MapNode {
    map_node(
        id,
        format!("goal-{id}"),
        state,
        "",
        parents.iter().map(|id| (*id).to_string()).collect(),
    )
}

fn fork_join_map() -> TaskSpaceMap {
    new_map(
        "map-1".into(),
        node("root", NodeState::InFlight, &[]),
        vec![
            node("left", NodeState::Ready, &["root"]),
            node("right", NodeState::Ready, &["root"]),
            node("join", NodeState::Waiting, &["left", "right"]),
        ],
        node("finish", NodeState::Waiting, &["join"]),
    )
}

#[test]
fn parents_are_canonical_and_children_are_derived() {
    let map = fork_join_map();
    assert!(validate(&map).is_empty());
    let views = derive_node_views(&map);
    let root = views.iter().find(|node| node.node_id == "root").unwrap();
    let join = views.iter().find(|node| node.node_id == "join").unwrap();
    assert_eq!(root.children, vec!["left", "right"]);
    assert_eq!(join.parents, vec!["left", "right"]);
    assert_eq!(join.children, vec!["finish"]);
}

#[test]
fn cycle_and_missing_parent_are_rejected() {
    let mut cycle = fork_join_map();
    cycle.work_nodes[0].parents = vec!["join".into()];
    assert!(
        validate(&cycle)
            .iter()
            .any(|v| v.code == ViolationCode::CycleDetected)
    );

    let mut missing = fork_join_map();
    missing.work_nodes[0].parents = vec!["missing".into()];
    assert!(
        validate(&missing)
            .iter()
            .any(|v| v.code == ViolationCode::ParentEndpointMissing)
    );
}

#[test]
fn finish_and_reopen_are_explicit_transactions() {
    let mut map = fork_join_map();
    for node in &mut map.work_nodes {
        node.state = NodeState::Completed;
    }
    normalize_readiness(&mut map);
    let finished = finish_map(
        &map,
        FinishMap {
            request_revision: 1,
            content: "Delivered and verified.".into(),
        },
    )
    .unwrap()
    .map;
    assert!(is_complete(&finished));
    assert_eq!(finished.finish.content, "Delivered and verified.");

    let reopened = reopen_map(
        &finished,
        ReopenMap {
            request_revision: finished.revision,
        },
    )
    .unwrap()
    .map;
    assert!(!is_complete(&reopened));
    assert_eq!(reopened.root.state, NodeState::InFlight);
    assert_eq!(reopened.finish.state, NodeState::Ready);
}

#[test]
fn action_outcome_does_not_change_node_state() {
    let map = fork_join_map();
    let committed = execute(
        &map,
        ExecuteTransaction {
            request_revision: map.revision,
            add_work_nodes: vec![],
            patches: vec![NodePatch {
                node_id: "left".into(),
                append_actions: vec![NodeAction {
                    action_id: "call-1".into(),
                    tool_name: "exec_command".into(),
                    outcome: ActionOutcome::Failed,
                }],
                ..Default::default()
            }],
        },
    )
    .unwrap()
    .map;
    assert_eq!(
        super::node(&committed, "left").unwrap().state,
        NodeState::Ready
    );
}

#[test]
fn duplicate_parent_and_local_action_are_rejected() {
    let mut map = fork_join_map();
    map.work_nodes[0].parents = vec!["root".into(), "root".into()];
    let action = NodeAction {
        action_id: "call-1".into(),
        tool_name: "exec_command".into(),
        outcome: ActionOutcome::Succeeded,
    };
    map.work_nodes[0].actions = vec![action.clone(), action];

    let violations = validate(&map);
    assert!(
        violations
            .iter()
            .any(|violation| violation.code == ViolationCode::ParentDuplicate)
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.code == ViolationCode::ActionInvalid)
    );
}

#[test]
fn one_action_can_belong_to_multiple_nodes_without_conflict() {
    let mut map = fork_join_map();
    let action = NodeAction {
        action_id: "hosted-call-1".into(),
        tool_name: "web_search".into(),
        outcome: ActionOutcome::Succeeded,
    };
    map.work_nodes[0].actions.push(action.clone());
    map.work_nodes[1].actions.push(action);

    assert!(validate(&map).is_empty());
}

#[test]
fn map_requires_work_and_boundary_nodes_cannot_own_actions() {
    let mut without_work = fork_join_map();
    without_work.work_nodes.clear();
    without_work.finish.parents = vec!["root".into()];
    assert!(
        validate(&without_work)
            .iter()
            .any(|violation| violation.code == ViolationCode::WorkNodeRequired)
    );

    let mut boundary_action = fork_join_map();
    boundary_action.root.actions.push(NodeAction {
        action_id: "root-call".into(),
        tool_name: "read_file".into(),
        outcome: ActionOutcome::Succeeded,
    });
    assert!(validate(&boundary_action).iter().any(|violation| {
        violation.code == ViolationCode::ActionInvalid
            && violation.subjects == vec!["root".to_string()]
    }));
}

#[test]
fn canonical_transactions_derive_new_work_state_from_the_complete_graph() {
    let initialized = initialize(InitializeMap {
        map_id: "map-derived".into(),
        root: node("root", NodeState::Completed, &[]),
        work_nodes: vec![
            node("tail", NodeState::Completed, &["head"]),
            node("head", NodeState::Waiting, &["root"]),
        ],
        finish: node("finish", NodeState::Completed, &["tail"]),
    })
    .unwrap()
    .map;
    assert_eq!(
        super::node(&initialized, "head").unwrap().state,
        NodeState::Ready
    );
    assert_eq!(
        super::node(&initialized, "tail").unwrap().state,
        NodeState::Waiting
    );

    let rejected = execute(
        &initialized,
        ExecuteTransaction {
            request_revision: initialized.revision,
            add_work_nodes: vec![node("new", NodeState::Completed, &["head"])],
            patches: vec![NodePatch {
                node_id: "new".into(),
                state: Some(NodeState::Completed),
                ..Default::default()
            }],
        },
    )
    .unwrap_err();
    assert_eq!(
        rejected.violations[0].code,
        ViolationCode::TransitionInvalid
    );
}

#[test]
fn ordered_patches_can_complete_a_newly_unlocked_descendant_atomically() {
    let current = new_map(
        "map-ordered".into(),
        node("root", NodeState::InFlight, &[]),
        vec![
            node("fix", NodeState::InFlight, &["root"]),
            node("verify", NodeState::Waiting, &["fix"]),
        ],
        node("finish", NodeState::Waiting, &["verify"]),
    );
    let original = current.clone();
    let committed = execute(
        &current,
        ExecuteTransaction {
            request_revision: current.revision,
            add_work_nodes: vec![],
            patches: vec![
                NodePatch {
                    node_id: "fix".into(),
                    state: Some(NodeState::Completed),
                    ..Default::default()
                },
                NodePatch {
                    node_id: "verify".into(),
                    state: Some(NodeState::Completed),
                    ..Default::default()
                },
            ],
        },
    )
    .unwrap()
    .map;

    assert_eq!(
        super::node(&committed, "fix").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(
        super::node(&committed, "verify").unwrap().state,
        NodeState::Completed
    );
    assert_eq!(committed.finish.state, NodeState::Ready);
    assert_eq!(current, original);

    let rejected = execute(
        &current,
        ExecuteTransaction {
            request_revision: current.revision,
            add_work_nodes: vec![],
            patches: vec![
                NodePatch {
                    node_id: "verify".into(),
                    state: Some(NodeState::Completed),
                    ..Default::default()
                },
                NodePatch {
                    node_id: "fix".into(),
                    state: Some(NodeState::Completed),
                    ..Default::default()
                },
            ],
        },
    )
    .unwrap_err();
    assert_eq!(
        rejected.violations[0].code,
        ViolationCode::TransitionInvalid
    );
    assert_eq!(current, original);
}
