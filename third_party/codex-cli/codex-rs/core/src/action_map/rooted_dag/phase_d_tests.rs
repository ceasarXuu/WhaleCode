use super::invariants::ViolationCode;
use super::model::ActionReservation;
use super::model::CompletionRecord;
use super::model::MapEdge;
use super::model::NodeState;
use super::model::TaskSpaceMap;
use super::model::map_node;
use super::model::state_sha256;
use super::transactions::ExecuteTransaction;
use super::transactions::GraphMutation;
use super::transactions::InitializeMap;
use super::transactions::NodeMutation;
use super::transactions::ReservationInput;
use super::transactions::ReservationRelease;
use super::transactions::execute;
use super::transactions::initialize;
use super::transactions::release_reservation;
use super::transitions::derive_node_state;
use pretty_assertions::assert_eq;

fn edge(from: &str, to: &str) -> MapEdge {
    MapEdge {
        from: from.into(),
        to: to.into(),
    }
}

fn reservation(id: &str, node_id: &str, index: u32) -> ReservationInput {
    ReservationInput {
        reservation_id: format!("reservation-{id}"),
        reservation: ActionReservation {
            action_id: format!("action-{id}"),
            node_id: node_id.into(),
            tool_name: "exec_command".into(),
            response_call_index: index,
        },
    }
}

fn fork_join(reservations: Vec<ReservationInput>) -> TaskSpaceMap {
    initialize(InitializeMap {
        map_id: "phase-b1x".into(),
        root: map_node("root", "solve", vec!["task-event".into()]),
        work_nodes: vec![
            map_node("left", "left", vec![]),
            map_node("right", "right", vec![]),
            map_node("join", "join", vec![]),
        ],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![
            edge("root", "left"),
            edge("root", "right"),
            edge("left", "join"),
            edge("right", "join"),
            edge("join", "finish"),
        ],
        reservations,
    })
    .unwrap()
    .map
}

fn release(map: TaskSpaceMap, reservation_id: &str) -> TaskSpaceMap {
    release_reservation(
        &map,
        ReservationRelease {
            expected_revision: map.revision,
            reservation_id: reservation_id.into(),
            result_refs: vec![],
            evidence_refs: vec![],
        },
    )
    .unwrap()
    .map
}

fn completion(action_id: &str) -> CompletionRecord {
    CompletionRecord {
        action_id: action_id.into(),
        result_ref_ids: vec![],
        evidence_ref_ids: vec![],
    }
}

#[test]
fn initialization_exposes_multiple_ready_and_inflight_nodes() {
    let map = fork_join(vec![reservation("left-a", "left", 0)]);

    assert_eq!(derive_node_state(&map, "left"), Some(NodeState::InFlight));
    assert_eq!(derive_node_state(&map, "right"), Some(NodeState::Ready));
    assert_eq!(derive_node_state(&map, "join"), Some(NodeState::Waiting));
    assert_eq!(map.action_reservations.len(), 1);
}

#[test]
fn multiple_actions_can_reserve_the_same_ready_node() {
    let map = fork_join(vec![
        reservation("left-a", "left", 0),
        reservation("left-b", "left", 1),
        reservation("right", "right", 2),
    ]);

    assert_eq!(derive_node_state(&map, "left"), Some(NodeState::InFlight));
    assert_eq!(derive_node_state(&map, "right"), Some(NodeState::InFlight));
    assert_eq!(
        map.action_reservations
            .values()
            .filter(|reservation| reservation.node_id == "left")
            .count(),
        2
    );
}

#[test]
fn waiting_node_rejection_preserves_multi_parent_state_facts() {
    let rejection = initialize(InitializeMap {
        map_id: "phase-b1x".into(),
        root: map_node("root", "solve", vec!["task-event".into()]),
        work_nodes: vec![
            map_node("left", "left", vec![]),
            map_node("right", "right", vec![]),
            map_node("join", "join", vec![]),
        ],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![
            edge("root", "left"),
            edge("root", "right"),
            edge("left", "join"),
            edge("right", "join"),
            edge("join", "finish"),
        ],
        reservations: vec![reservation("join", "join", 0)],
    })
    .unwrap_err();

    let violation = &rejection.violations[0];
    assert_eq!(violation.code, ViolationCode::NodeStateInvalid);
    assert_eq!(violation.node_id.as_deref(), Some("join"));
    assert_eq!(
        violation.evaluated_state_at_violation,
        Some(NodeState::Waiting)
    );
    assert_eq!(
        violation.allowed_states_at_violation,
        vec![NodeState::Ready, NodeState::InFlight]
    );
    assert_eq!(
        violation.evaluated_unsatisfied_predecessor_ids_at_violation,
        vec!["left".to_string(), "right".to_string()]
    );
}

#[test]
fn completed_and_blocked_node_rejections_preserve_actual_state() {
    let map = release(
        fork_join(vec![reservation("left-a", "left", 0)]),
        "reservation-left-a",
    );
    let completed = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "left".into(),
                record: completion("complete-left"),
            }],
            reservations: vec![reservation("right-a", "right", 0)],
        },
    )
    .unwrap()
    .map;
    let completed_rejection = execute(
        &completed,
        ExecuteTransaction {
            expected_revision: completed.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![],
            reservations: vec![reservation("left-b", "left", 0)],
        },
    )
    .unwrap_err();
    assert_eq!(
        completed_rejection.violations[0].evaluated_state_at_violation,
        Some(NodeState::Completed)
    );

    let map = release(
        fork_join(vec![reservation("left-a", "left", 0)]),
        "reservation-left-a",
    );
    let blocked = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Block {
                node_id: "right".into(),
                record: super::model::BlockRecord {
                    action_id: "block-right".into(),
                    reason_ref: "blocked".into(),
                },
            }],
            reservations: vec![reservation("left-b", "left", 0)],
        },
    )
    .unwrap()
    .map;
    let blocked_rejection = execute(
        &blocked,
        ExecuteTransaction {
            expected_revision: blocked.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![],
            reservations: vec![reservation("right-b", "right", 0)],
        },
    )
    .unwrap_err();
    assert_eq!(
        blocked_rejection.violations[0].evaluated_state_at_violation,
        Some(NodeState::Blocked)
    );
}

#[test]
fn rejected_transaction_distinguishes_canonical_and_evaluated_node_state() {
    let current = release(
        fork_join(vec![reservation("left-a", "left", 0)]),
        "reservation-left-a",
    );
    let rejection = execute(
        &current,
        ExecuteTransaction {
            expected_revision: current.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "left".into(),
                record: completion("complete-left"),
            }],
            reservations: vec![reservation("left-b", "left", 0)],
        },
    )
    .unwrap_err();

    let violation = &rejection.violations[0];
    assert_eq!(
        violation.canonical_state_before_transaction,
        Some(NodeState::Ready)
    );
    assert_eq!(
        violation.evaluated_state_at_violation,
        Some(NodeState::Completed)
    );
    assert!(
        violation
            .canonical_unsatisfied_predecessor_ids_before_transaction
            .is_empty()
    );
    assert!(
        violation
            .evaluated_unsatisfied_predecessor_ids_at_violation
            .is_empty()
    );
}

#[test]
fn duplicate_reservation_identity_remains_reservation_invalid() {
    let duplicate = reservation("left-a", "left", 0);
    let rejection = initialize(InitializeMap {
        map_id: "phase-b1x".into(),
        root: map_node("root", "solve", vec!["task-event".into()]),
        work_nodes: vec![map_node("left", "left", vec![])],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![edge("root", "left"), edge("left", "finish")],
        reservations: vec![duplicate.clone(), duplicate],
    })
    .unwrap_err();

    assert_eq!(
        rejection.violations[0].code,
        ViolationCode::ReservationInvalid
    );
    assert_eq!(rejection.violations[0].node_id, None);
}

#[test]
fn completion_and_next_actions_commit_in_one_revision() {
    let map = release(
        fork_join(vec![reservation("left-a", "left", 0)]),
        "reservation-left-a",
    );
    let before_revision = map.revision;
    let committed = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "left".into(),
                record: completion("control-complete-left"),
            }],
            reservations: vec![reservation("right-a", "right", 0)],
        },
    )
    .unwrap();

    assert_eq!(committed.map.revision, before_revision + 1);
    assert_eq!(
        derive_node_state(&committed.map, "left"),
        Some(NodeState::Completed)
    );
    assert_eq!(
        derive_node_state(&committed.map, "right"),
        Some(NodeState::InFlight)
    );
    assert_eq!(committed.events.facts.len(), 2);
}

#[test]
fn graph_mutation_and_new_node_reservation_are_atomic() {
    let map = fork_join(vec![reservation("left-a", "left", 0)]);
    let committed = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation {
                add_work_nodes: vec![map_node("side", "side evidence", vec![])],
                add_edges: vec![edge("root", "side"), edge("side", "finish")],
                remove_edges: vec![],
            },
            node_mutations: vec![],
            reservations: vec![reservation("side-a", "side", 1)],
        },
    )
    .unwrap();

    assert_eq!(
        derive_node_state(&committed.map, "side"),
        Some(NodeState::InFlight)
    );
    assert_eq!(committed.map.revision, map.revision + 1);
}

#[test]
fn invalid_mixed_transaction_has_zero_partial_commit() {
    let map = fork_join(vec![reservation("left-a", "left", 0)]);
    let before_hash = state_sha256(&map).unwrap();
    let rejection = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation {
                add_work_nodes: vec![map_node("orphan", "orphan", vec![])],
                add_edges: vec![edge("missing", "orphan")],
                remove_edges: vec![],
            },
            node_mutations: vec![],
            reservations: vec![reservation("orphan-a", "orphan", 1)],
        },
    )
    .unwrap_err();

    assert_eq!(rejection.state_commit, false);
    assert_eq!(state_sha256(&map).unwrap(), before_hash);
    assert!(map.work_nodes.iter().all(|node| node.node_id != "orphan"));
}

#[test]
fn stale_cas_rejects_without_changing_facts() {
    let map = fork_join(vec![reservation("left-a", "left", 0)]);
    let before_hash = state_sha256(&map).unwrap();
    let rejection = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision + 1,
            graph: GraphMutation::default(),
            node_mutations: vec![],
            reservations: vec![reservation("right-a", "right", 1)],
        },
    )
    .unwrap_err();

    assert_eq!(rejection.violations[0].code, ViolationCode::StaleRevision);
    assert_eq!(rejection.current_revision, map.revision);
    assert_eq!(state_sha256(&map).unwrap(), before_hash);
}

#[test]
fn standalone_nonterminal_mutation_is_rejected() {
    let map = fork_join(vec![reservation("left-a", "left", 0)]);
    let rejection = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Block {
                node_id: "right".into(),
                record: super::model::BlockRecord {
                    action_id: "control-block".into(),
                    reason_ref: "reason-ref".into(),
                },
            }],
            reservations: vec![],
        },
    )
    .unwrap_err();

    assert_eq!(
        rejection.violations[0].code,
        ViolationCode::ReservationInvalid
    );
    assert!(!map.block_records.contains_key("right"));
}

#[test]
fn block_unblock_and_completion_are_explicit_facts() {
    let map = release(
        fork_join(vec![reservation("left-a", "left", 0)]),
        "reservation-left-a",
    );
    let blocked = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Block {
                node_id: "right".into(),
                record: super::model::BlockRecord {
                    action_id: "control-block-right".into(),
                    reason_ref: "block-reason-ref".into(),
                },
            }],
            reservations: vec![reservation("left-b", "left", 0)],
        },
    )
    .unwrap()
    .map;
    assert_eq!(
        derive_node_state(&blocked, "right"),
        Some(NodeState::Blocked)
    );

    let unblocked = execute(
        &blocked,
        ExecuteTransaction {
            expected_revision: blocked.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Unblock {
                node_id: "right".into(),
            }],
            reservations: vec![reservation("left-c", "left", 0)],
        },
    )
    .unwrap()
    .map;
    assert_eq!(
        derive_node_state(&unblocked, "right"),
        Some(NodeState::Ready)
    );

    let left_ready = release(unblocked, "reservation-left-b");
    let left_ready = release(left_ready, "reservation-left-c");
    let completed = execute(
        &left_ready,
        ExecuteTransaction {
            expected_revision: left_ready.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "left".into(),
                record: completion("control-complete-left"),
            }],
            reservations: vec![reservation("right-b", "right", 0)],
        },
    )
    .unwrap()
    .map;
    assert!(completed.completion_records.contains_key("left"));
    assert_eq!(
        derive_node_state(&completed, "left"),
        Some(NodeState::Completed)
    );
}
