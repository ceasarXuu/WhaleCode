use super::events::EventBatch;
use super::events::MapEvent;
use super::events::ReplayError;
use super::events::replay_batches;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::NodeStatus;
use super::transactions::GraphMutation;
use super::transactions::InitializeMap;
use super::transactions::Rejection;
use super::transactions::finish_map;
use super::transactions::initialize;
use super::transactions::mutate_graph;
use super::transactions::transition_node;
use super::transitions::NodeTransition;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

fn chain_input(work_count: usize) -> InitializeMap {
    let mut work_nodes = BTreeMap::new();
    for index in 1..=work_count {
        work_nodes.insert(format!("work-{index:02}"), format!("work {index}"));
    }
    let mut ids = vec!["root".to_string()];
    ids.extend((1..=work_count).map(|index| format!("work-{index:02}")));
    ids.push("finish".to_string());
    let edges = ids
        .windows(2)
        .map(|pair| MapEdge::new(&pair[0], &pair[1]))
        .collect();
    InitializeMap {
        map_id: ("map-test").to_string(),
        root_node_id: ("root").to_string(),
        root_goal: "solve task".into(),
        source_refs: vec!["source-b".into(), "source-a".into(), "source-b".into()],
        finish_node_id: ("finish").to_string(),
        work_nodes,
        edges,
    }
}

fn violation_codes(rejection: &Rejection) -> Vec<ViolationCode> {
    rejection
        .violations
        .iter()
        .map(|violation| violation.code)
        .collect()
}

fn complete_work(map: &mut super::model::TaskSpaceMap, node_id: &str) {
    let bound = transition_node(
        map,
        map.revision,
        (node_id).to_string(),
        NodeTransition::Bind,
    )
    .unwrap();
    *map = bound.map;
    let completed = transition_node(
        map,
        map.revision,
        (node_id).to_string(),
        NodeTransition::Complete,
    )
    .unwrap();
    *map = completed.map;
}

#[test]
fn initialization_derives_only_the_first_frontier_and_preserves_sources() {
    let commit = initialize(chain_input(3)).unwrap();

    assert_eq!(commit.map.revision, 1);
    assert_eq!(
        commit.map.node(&("root").to_string()).unwrap().goal,
        "solve task"
    );
    assert_eq!(
        commit.map.node(&("work-01").to_string()).unwrap().goal,
        "work 1"
    );
    assert_eq!(commit.map.node(&("finish").to_string()).unwrap().goal, "");
    assert_eq!(
        commit.map.node(&("work-01").to_string()).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        commit.map.node(&("work-02").to_string()).unwrap().status,
        NodeStatus::Pending
    );
    assert_eq!(
        commit.map.node(&("finish").to_string()).unwrap().status,
        NodeStatus::Pending
    );
    assert_eq!(
        commit.map.node(&("root").to_string()).unwrap().source_refs,
        vec!["source-b", "source-a", "source-b"]
    );
    assert_eq!(commit.events.records.len(), 2);
    assert_eq!(validate(&commit.map), vec![]);
}

#[test]
fn fork_join_waits_for_every_predecessor() {
    let input = InitializeMap {
        map_id: ("fork-join").to_string(),
        root_node_id: ("root").to_string(),
        root_goal: "solve".into(),
        source_refs: vec!["source".into()],
        finish_node_id: ("finish").to_string(),
        work_nodes: BTreeMap::from([
            (("left").to_string(), "left".into()),
            (("right").to_string(), "right".into()),
            (("join").to_string(), "join".into()),
        ]),
        edges: vec![
            MapEdge::new("root", "left"),
            MapEdge::new("root", "right"),
            MapEdge::new("left", "join"),
            MapEdge::new("right", "join"),
            MapEdge::new("join", "finish"),
        ],
    };
    let mut map = initialize(input).unwrap().map;
    assert_eq!(
        map.node(&("left").to_string()).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        map.node(&("right").to_string()).unwrap().status,
        NodeStatus::Ready
    );

    complete_work(&mut map, "left");
    assert_eq!(
        map.node(&("join").to_string()).unwrap().status,
        NodeStatus::Pending
    );
    complete_work(&mut map, "right");
    assert_eq!(
        map.node(&("join").to_string()).unwrap().status,
        NodeStatus::Ready
    );
}

#[test]
fn graph_mutation_rewires_atomically() {
    let original = initialize(chain_input(1)).unwrap().map;
    let mutation = GraphMutation {
        expected_revision: original.revision,
        add_nodes: BTreeMap::from([(("work-02").to_string(), "second".into())]),
        add_edges: vec![
            MapEdge::new("work-01", "work-02"),
            MapEdge::new("work-02", "finish"),
        ],
        remove_edges: vec![MapEdge::new("work-01", "finish")],
    };

    let committed = mutate_graph(&original, mutation).unwrap();

    assert_eq!(original.revision, 1);
    assert_eq!(committed.map.revision, 2);
    assert_eq!(committed.map.nodes.len(), 4);
    assert_eq!(committed.map.edges.len(), 3);
    assert_eq!(
        committed.map.node(&("work-02").to_string()).unwrap().goal,
        "second"
    );
    assert_eq!(validate(&committed.map), vec![]);
}

#[test]
fn rejected_mutation_keeps_hash_and_revision_unchanged() {
    let original = initialize(chain_input(1)).unwrap().map;
    let before_hash = original.state_sha256().unwrap();
    let mutation = GraphMutation {
        expected_revision: original.revision,
        add_nodes: BTreeMap::from([(("orphan-sink").to_string(), "orphan".into())]),
        add_edges: vec![MapEdge::new("root", "orphan-sink")],
        remove_edges: vec![],
    };

    let rejection = mutate_graph(&original, mutation).unwrap_err();

    assert_eq!(rejection.state_commit, false);
    assert_eq!(rejection.current_revision, original.revision);
    assert!(violation_codes(&rejection).contains(&ViolationCode::NonFinishZeroOutdegree));
    assert_eq!(original.revision, 1);
    assert_eq!(original.state_sha256().unwrap(), before_hash);
}

#[test]
fn invalid_transition_and_stale_revision_are_mechanical_rejections() {
    let original = initialize(chain_input(1)).unwrap().map;
    let before_hash = original.state_sha256().unwrap();

    let invalid = transition_node(
        &original,
        original.revision,
        ("work-01").to_string(),
        NodeTransition::Complete,
    )
    .unwrap_err();
    let stale = transition_node(
        &original,
        original.revision + 1,
        ("work-01").to_string(),
        NodeTransition::Bind,
    )
    .unwrap_err();

    assert_eq!(
        violation_codes(&invalid),
        vec![ViolationCode::TransitionInvalid]
    );
    assert_eq!(violation_codes(&stale), vec![ViolationCode::StaleRevision]);
    assert_eq!(original.state_sha256().unwrap(), before_hash);
}

#[test]
fn block_and_unblock_are_agent_requested_transitions() {
    let mut map = initialize(chain_input(1)).unwrap().map;
    map = transition_node(
        &map,
        map.revision,
        ("work-01").to_string(),
        NodeTransition::Bind,
    )
    .unwrap()
    .map;
    map = transition_node(
        &map,
        map.revision,
        ("work-01").to_string(),
        NodeTransition::Block,
    )
    .unwrap()
    .map;
    assert_eq!(
        map.node(&("work-01").to_string()).unwrap().status,
        NodeStatus::Blocked
    );
    assert_eq!(map.current_binding, None);

    map = transition_node(
        &map,
        map.revision,
        ("work-01").to_string(),
        NodeTransition::Unblock,
    )
    .unwrap()
    .map;
    assert_eq!(
        map.node(&("work-01").to_string()).unwrap().status,
        NodeStatus::Ready
    );
}

#[test]
fn released_lease_replays_to_an_unbound_ready_node() {
    let initialized = initialize(chain_input(1)).unwrap();
    let bound = transition_node(
        &initialized.map,
        initialized.map.revision,
        "work-01".to_string(),
        NodeTransition::Bind,
    )
    .unwrap();
    let released = transition_node(
        &bound.map,
        bound.map.revision,
        "work-01".to_string(),
        NodeTransition::ReleaseLease,
    )
    .unwrap();

    assert_eq!(released.map.current_binding, None);
    assert_eq!(
        released.map.node(&"work-01".to_string()).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        released.events.records[0].event,
        MapEvent::NodeLeaseReleased {
            node_id: "work-01".to_string()
        }
    );
    assert_eq!(
        replay_batches(&[initialized.events, bound.events, released.events]).unwrap(),
        released.map
    );
}

#[test]
fn finish_remains_manual_and_empty_summary_does_not_commit() {
    let mut map = initialize(chain_input(1)).unwrap().map;
    complete_work(&mut map, "work-01");
    assert_eq!(
        map.node(&("finish").to_string()).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(map.is_complete(), false);
    let before_hash = map.state_sha256().unwrap();

    let wrong_terminal = finish_map(
        &map,
        map.revision,
        "root".into(),
        "exact agent summary".into(),
    )
    .unwrap_err();
    assert_eq!(
        violation_codes(&wrong_terminal),
        vec![ViolationCode::TransitionInvalid]
    );
    assert_eq!(map.state_sha256().unwrap(), before_hash);

    let rejection = finish_map(&map, map.revision, "finish".into(), "  ".into()).unwrap_err();
    assert_eq!(
        violation_codes(&rejection),
        vec![ViolationCode::FinalSummaryEmpty]
    );
    assert_eq!(map.state_sha256().unwrap(), before_hash);

    let committed = finish_map(
        &map,
        map.revision,
        "finish".into(),
        "exact agent summary".into(),
    )
    .unwrap();
    assert_eq!(committed.map.is_complete(), true);
    assert!(committed.map.terminal_summary_ref.is_some());
    assert_eq!(
        committed.events.records[0].event,
        MapEvent::TerminalCommitted {
            final_summary: "exact agent summary".into()
        }
    );
}

#[test]
fn twenty_work_cycles_replay_to_identical_state_and_hash() {
    let initialized = initialize(chain_input(20)).unwrap();
    let mut map = initialized.map;
    let mut journal = vec![initialized.events];
    for index in 1..=20 {
        let id = format!("work-{index:02}");
        let bound = transition_node(&map, map.revision, id.clone(), NodeTransition::Bind).unwrap();
        map = bound.map;
        journal.push(bound.events);
        let completed = transition_node(&map, map.revision, id, NodeTransition::Complete).unwrap();
        map = completed.map;
        journal.push(completed.events);
    }
    let terminal = finish_map(
        &map,
        map.revision,
        "finish".into(),
        "summary preserved exactly".into(),
    )
    .unwrap();
    map = terminal.map;
    journal.push(terminal.events);

    let encoded = serde_json::to_vec(&journal).unwrap();
    let restored_journal: Vec<EventBatch> = serde_json::from_slice(&encoded).unwrap();
    let replayed = replay_batches(&restored_journal).unwrap();

    assert_eq!(replayed, map);
    assert_eq!(
        replayed.state_sha256().unwrap(),
        map.state_sha256().unwrap()
    );
    assert_eq!(replayed.revision, 42);
}

#[test]
fn corrupted_event_identity_is_rejected() {
    let initialized = initialize(chain_input(1)).unwrap();
    let mut corrupted = initialized.events;
    corrupted.records[0].event_id = "wrong-event".into();

    assert_eq!(
        replay_batches(&[corrupted]),
        Err(ReplayError::EventIdMismatch)
    );
}

#[test]
fn empty_event_batch_cannot_advance_revision() {
    let initialized = initialize(chain_input(1)).unwrap();
    let empty = EventBatch {
        map_id: initialized.map.id.clone(),
        revision: initialized.map.revision + 1,
        records: vec![],
    };

    assert_eq!(
        super::events::apply_batch(Some(&initialized.map), &empty),
        Err(ReplayError::EmptyBatch)
    );
}
