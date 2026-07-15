use super::events::MapEvent;
use super::invariants::ViolationCode;
use super::model::MapEdge;
use super::model::NodeStatus;
use super::model::TaskSpaceMap;
use super::transactions::GraphMutation;
use super::transactions::InitializeMap;
use super::transactions::initialize;
use super::transactions::mutate_graph;
use super::transactions::transition_node;
use super::transitions::NodeTransition;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

fn map(work_nodes: &[&str], edges: &[(&str, &str)]) -> TaskSpaceMap {
    initialize(InitializeMap {
        map_id: "phase-d-map".into(),
        root_node_id: "root".into(),
        root_goal: "solve".into(),
        source_refs: vec!["task-event".into()],
        finish_node_id: "finish".into(),
        work_nodes: work_nodes
            .iter()
            .map(|id| ((*id).to_string(), format!("work {id}")))
            .collect(),
        edges: edges
            .iter()
            .map(|(from, to)| MapEdge::new(*from, *to))
            .collect(),
    })
    .unwrap()
    .map
}

fn transition(map: TaskSpaceMap, node_id: &str, action: NodeTransition) -> TaskSpaceMap {
    let revision = map.revision;
    transition_node(&map, revision, node_id.into(), action)
        .unwrap()
        .map
}

fn complete(map: TaskSpaceMap, node_id: &str) -> TaskSpaceMap {
    transition(
        transition(map, node_id, NodeTransition::Bind),
        node_id,
        NodeTransition::Complete,
    )
}

fn late_predecessor_mutation(map: &TaskSpaceMap) -> GraphMutation {
    GraphMutation {
        expected_revision: map.revision,
        add_nodes: BTreeMap::from([("late".into(), "late prerequisite".into())]),
        add_edges: vec![MapEdge::new("root", "late"), MapEdge::new("late", "work")],
        remove_edges: Vec::new(),
    }
}

#[test]
fn ready_node_returns_to_pending_when_a_new_predecessor_is_unmet() {
    let map = map(
        &["left", "target"],
        &[
            ("root", "left"),
            ("root", "target"),
            ("left", "finish"),
            ("target", "finish"),
        ],
    );
    let committed = mutate_graph(
        &map,
        GraphMutation {
            expected_revision: map.revision,
            add_nodes: BTreeMap::new(),
            add_edges: vec![MapEdge::new("left", "target")],
            remove_edges: vec![MapEdge::new("left", "finish")],
        },
    )
    .unwrap();

    assert_eq!(
        committed.map.node(&"target".into()).unwrap().status,
        NodeStatus::Pending
    );
    let completed_left = complete(committed.map, "left");
    assert_eq!(
        completed_left.node(&"target".into()).unwrap().status,
        NodeStatus::Ready
    );
}

#[test]
fn started_work_rejects_incoming_edge_rewrites_without_state_change() {
    for status in [
        NodeStatus::Running,
        NodeStatus::Blocked,
        NodeStatus::Completed,
    ] {
        let mut map = map(&["work"], &[("root", "work"), ("work", "finish")]);
        map = transition(map, "work", NodeTransition::Bind);
        map = match status {
            NodeStatus::Running => map,
            NodeStatus::Blocked => transition(map, "work", NodeTransition::Block),
            NodeStatus::Completed => transition(map, "work", NodeTransition::Complete),
            _ => unreachable!(),
        };
        let before_hash = map.state_sha256().unwrap();

        let rejection = mutate_graph(&map, late_predecessor_mutation(&map)).unwrap_err();

        assert_eq!(
            rejection.violations[0].code,
            ViolationCode::ExecutionCausalityConflict
        );
        assert_eq!(rejection.violations[0].subjects, ["late->work"]);
        assert_eq!(map.state_sha256().unwrap(), before_hash);
    }
}

#[test]
fn rework_reopens_completed_work_and_demotes_affected_frontier() {
    let map = map(
        &["first", "second"],
        &[("root", "first"), ("first", "second"), ("second", "finish")],
    );
    let map = complete(map, "first");
    assert_eq!(
        map.node(&"second".into()).unwrap().status,
        NodeStatus::Ready
    );

    let committed =
        transition_node(&map, map.revision, "first".into(), NodeTransition::Rework).unwrap();

    assert_eq!(
        committed.map.node(&"first".into()).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        committed.map.node(&"second".into()).unwrap().status,
        NodeStatus::Pending
    );
    assert_eq!(
        committed.events.records[0].event,
        MapEvent::NodeReworked {
            node_id: "first".into()
        }
    );
}

#[test]
fn rework_rejects_when_a_downstream_execution_consumed_the_result() {
    let map = map(
        &["first", "second"],
        &[("root", "first"), ("first", "second"), ("second", "finish")],
    );
    let map = complete(complete(map, "first"), "second");
    let before_hash = map.state_sha256().unwrap();

    let rejection =
        transition_node(&map, map.revision, "first".into(), NodeTransition::Rework).unwrap_err();

    assert_eq!(
        rejection.violations[0].code,
        ViolationCode::ExecutionCausalityConflict
    );
    assert_eq!(rejection.violations[0].subjects, ["second"]);
    assert_eq!(map.state_sha256().unwrap(), before_hash);
}

#[test]
fn same_revision_graph_writes_reject_the_stale_writer() {
    let map = map(&["work"], &[("root", "work"), ("work", "finish")]);
    let first = mutate_graph(
        &map,
        GraphMutation {
            expected_revision: map.revision,
            add_nodes: BTreeMap::from([("side".into(), "side".into())]),
            add_edges: vec![MapEdge::new("root", "side"), MapEdge::new("side", "finish")],
            remove_edges: Vec::new(),
        },
    )
    .unwrap();
    let before_hash = first.map.state_sha256().unwrap();
    let mut stale = late_predecessor_mutation(&first.map);
    stale.expected_revision = map.revision;

    let rejection = mutate_graph(&first.map, stale).unwrap_err();

    assert_eq!(rejection.violations[0].code, ViolationCode::StaleRevision);
    assert_eq!(first.map.state_sha256().unwrap(), before_hash);
}

#[test]
fn invalid_mixed_mutation_has_zero_partial_commit() {
    let map = map(&["work"], &[("root", "work"), ("work", "finish")]);
    let before_hash = map.state_sha256().unwrap();

    let rejection = mutate_graph(
        &map,
        GraphMutation {
            expected_revision: map.revision,
            add_nodes: BTreeMap::new(),
            add_edges: vec![MapEdge::new("missing", "work")],
            remove_edges: vec![MapEdge::new("root", "work")],
        },
    )
    .unwrap_err();

    assert_eq!(rejection.state_commit, false);
    assert_eq!(map.state_sha256().unwrap(), before_hash);
    assert_eq!(
        map.edges,
        [MapEdge::new("root", "work"), MapEdge::new("work", "finish")]
    );
}
