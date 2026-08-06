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
