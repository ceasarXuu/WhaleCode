use super::invariants::validate;
use super::model::MapEdge;
use super::model::MapNode;
use super::model::NodeRole;
use super::model::NodeStatus;
use super::model::TaskSpaceMap;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

type FixtureNode = (&'static str, NodeRole, NodeStatus);
type FixtureEdge = (&'static str, &'static str);

struct FixtureCase {
    id: &'static str,
    valid: bool,
    root_node_id: &'static str,
    finish_node_id: &'static str,
    nodes: Vec<FixtureNode>,
    edges: Vec<FixtureEdge>,
    expected_codes: Vec<&'static str>,
}

fn fixture(
    id: &'static str,
    valid: bool,
    root_node_id: &'static str,
    finish_node_id: &'static str,
    nodes: Vec<FixtureNode>,
    edges: Vec<FixtureEdge>,
    expected_codes: Vec<&'static str>,
) -> FixtureCase {
    FixtureCase {
        id,
        valid,
        root_node_id,
        finish_node_id,
        nodes,
        edges,
        expected_codes,
    }
}

fn fixtures() -> Vec<FixtureCase> {
    use NodeRole::Finish;
    use NodeRole::TaskRoot;
    use NodeRole::Work;
    use NodeStatus::Completed;
    use NodeStatus::Open;
    use NodeStatus::Pending;
    use NodeStatus::Ready;

    vec![
        fixture(
            "valid_trivial",
            /*valid*/ true,
            "root",
            "finish",
            vec![("root", TaskRoot, Open), ("finish", Finish, Ready)],
            vec![("root", "finish")],
            vec![],
        ),
        fixture(
            "valid_chain",
            /*valid*/ true,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("inspect", Work, Ready),
                ("patch", Work, Pending),
                ("finish", Finish, Pending),
            ],
            vec![
                ("root", "inspect"),
                ("inspect", "patch"),
                ("patch", "finish"),
            ],
            vec![],
        ),
        fixture(
            "valid_fork_join",
            /*valid*/ true,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("read-rules", Work, Ready),
                ("read-tests", Work, Ready),
                ("decide", Work, Pending),
                ("finish", Finish, Pending),
            ],
            vec![
                ("root", "read-rules"),
                ("root", "read-tests"),
                ("read-rules", "decide"),
                ("read-tests", "decide"),
                ("decide", "finish"),
            ],
            vec![],
        ),
        fixture(
            "invalid_multiple_roots",
            /*valid*/ false,
            "root-a",
            "finish",
            vec![
                ("root-a", TaskRoot, Open),
                ("root-b", TaskRoot, Open),
                ("finish", Finish, Pending),
            ],
            vec![("root-a", "finish"), ("root-b", "finish")],
            vec![
                "multiple_roots",
                "non_root_zero_indegree",
                "node_unreachable_from_root",
            ],
        ),
        fixture(
            "invalid_multiple_finishes",
            /*valid*/ false,
            "root",
            "finish-a",
            vec![
                ("root", TaskRoot, Open),
                ("finish-a", Finish, Pending),
                ("finish-b", Finish, Pending),
            ],
            vec![("root", "finish-a"), ("root", "finish-b")],
            vec![
                "multiple_finishes",
                "non_finish_zero_outdegree",
                "finish_unreachable_from_node",
            ],
        ),
        fixture(
            "invalid_additional_source",
            /*valid*/ false,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("orphan-source", Work, Ready),
                ("finish", Finish, Pending),
            ],
            vec![("root", "finish"), ("orphan-source", "finish")],
            vec!["non_root_zero_indegree", "node_unreachable_from_root"],
        ),
        fixture(
            "invalid_additional_sink",
            /*valid*/ false,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("orphan-sink", Work, Pending),
                ("finish", Finish, Pending),
            ],
            vec![("root", "orphan-sink"), ("root", "finish")],
            vec!["non_finish_zero_outdegree", "finish_unreachable_from_node"],
        ),
        fixture(
            "invalid_cycle",
            /*valid*/ false,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("a", Work, Ready),
                ("b", Work, Pending),
                ("finish", Finish, Pending),
            ],
            vec![("root", "a"), ("a", "b"), ("b", "a"), ("b", "finish")],
            vec!["cycle_detected"],
        ),
        fixture(
            "invalid_self_loop",
            /*valid*/ false,
            "root",
            "finish",
            vec![
                ("root", TaskRoot, Open),
                ("work", Work, Ready),
                ("finish", Finish, Pending),
            ],
            vec![("root", "work"), ("work", "work"), ("work", "finish")],
            vec!["self_loop", "cycle_detected"],
        ),
        fixture(
            "invalid_missing_endpoint",
            /*valid*/ false,
            "root",
            "finish",
            vec![("root", TaskRoot, Open), ("finish", Finish, Pending)],
            vec![("root", "missing"), ("missing", "finish")],
            vec![
                "edge_endpoint_missing",
                "non_root_zero_indegree",
                "non_finish_zero_outdegree",
            ],
        ),
        fixture(
            "invalid_duplicate_edge",
            /*valid*/ false,
            "root",
            "finish",
            vec![("root", TaskRoot, Open), ("finish", Finish, Ready)],
            vec![("root", "finish"), ("root", "finish")],
            vec!["duplicate_edge"],
        ),
        fixture(
            "invalid_role_status",
            /*valid*/ false,
            "root",
            "finish",
            vec![("root", TaskRoot, Completed), ("finish", Finish, Ready)],
            vec![("root", "finish")],
            vec!["role_status_invalid"],
        ),
        fixture(
            "invalid_root_id",
            /*valid*/ false,
            "not-root",
            "finish",
            vec![("root", TaskRoot, Open), ("finish", Finish, Ready)],
            vec![("root", "finish")],
            vec!["root_id_mismatch"],
        ),
        fixture(
            "invalid_finish_id",
            /*valid*/ false,
            "root",
            "not-finish",
            vec![("root", TaskRoot, Open), ("finish", Finish, Ready)],
            vec![("root", "finish")],
            vec!["finish_id_mismatch"],
        ),
    ]
}

fn fixture_map(case: &FixtureCase) -> TaskSpaceMap {
    let nodes = case
        .nodes
        .iter()
        .map(|(id, role, status)| {
            (
                (*id).to_string(),
                MapNode {
                    role: *role,
                    goal: if *role == NodeRole::Finish {
                        String::new()
                    } else {
                        (*id).to_string()
                    },
                    source_refs: Vec::new(),
                    status: *status,
                    active_lease: None,
                    result_context: Vec::new(),
                    node_events: Vec::new(),
                    origin_node_id: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    TaskSpaceMap {
        id: (case.id).to_string(),
        root_node_id: (case.root_node_id).to_string(),
        finish_node_id: (case.finish_node_id).to_string(),
        nodes,
        edges: case
            .edges
            .iter()
            .map(|(from, to)| MapEdge::new(*from, *to))
            .collect(),
        revision: 1,
        current_binding: None,
        terminal_summary_ref: None,
    }
}

#[test]
fn phase_a_graph_fixtures_match_the_rust_validator() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 14);
    for case in fixtures {
        let actual = validate(&fixture_map(&case));
        let actual_codes: Vec<_> = actual
            .iter()
            .map(|violation| violation.code.as_str())
            .collect();
        let missing_codes: Vec<_> = case
            .expected_codes
            .iter()
            .copied()
            .filter(|expected| !actual_codes.contains(expected))
            .collect();
        assert_eq!(
            missing_codes,
            Vec::<&str>::new(),
            "fixture {} missing expected codes; actual={actual_codes:?}",
            case.id
        );
        assert_eq!(
            actual.is_empty(),
            case.valid,
            "fixture {} validity mismatch: {actual:?}",
            case.id
        );
    }
}

#[test]
fn canonical_state_hash_is_independent_of_edge_order() {
    let mut left = TaskSpaceMap {
        id: ("map").to_string(),
        root_node_id: ("root").to_string(),
        finish_node_id: ("finish").to_string(),
        nodes: BTreeMap::from([
            (
                ("root").to_string(),
                MapNode::task_root("goal", vec!["source-b".into(), "source-a".into()]),
            ),
            (("work").to_string(), MapNode::work("work")),
            (("finish").to_string(), MapNode::finish()),
        ]),
        edges: vec![MapEdge::new("work", "finish"), MapEdge::new("root", "work")],
        revision: 1,
        current_binding: None,
        terminal_summary_ref: None,
    };
    let mut right = left.clone();
    right.edges.reverse();

    left.canonicalize();
    right.canonicalize();
    assert_eq!(left.state_sha256().unwrap(), right.state_sha256().unwrap());
}

#[test]
fn canonicalization_preserves_source_ref_order_and_duplicates() {
    let mut map = fixture_map(&fixtures()[1]);
    let root = map.nodes.get_mut(&("root").to_string()).unwrap();
    root.source_refs = vec!["source-b".into(), "source-a".into(), "source-b".into()];

    map.canonicalize();

    assert_eq!(
        map.nodes.get(&("root").to_string()).unwrap().source_refs,
        vec!["source-b", "source-a", "source-b"]
    );
}

#[test]
fn finish_goal_is_structurally_forbidden() {
    let mut map = fixture_map(&fixtures()[1]);
    map.nodes.get_mut(&map.finish_node_id).unwrap().goal = "verify and summarize".into();

    let violations = validate(&map);

    assert!(violations.iter().any(|violation| {
        violation.code == super::invariants::ViolationCode::FinishGoalNotEmpty
            && violation.subjects == ["finish"]
    }));
}

#[test]
fn role_status_matrix_accepts_only_contract_combinations() {
    let roles = [NodeRole::TaskRoot, NodeRole::Work, NodeRole::Finish];
    let statuses = [
        NodeStatus::Open,
        NodeStatus::Closed,
        NodeStatus::Pending,
        NodeStatus::Ready,
        NodeStatus::Running,
        NodeStatus::Blocked,
        NodeStatus::Completed,
    ];
    for role in roles {
        for status in statuses {
            let node = MapNode {
                role,
                goal: "goal".into(),
                source_refs: vec![],
                status,
                active_lease: None,
                result_context: Vec::new(),
                node_events: Vec::new(),
                origin_node_id: None,
            };
            let expected = matches!(
                (role, status),
                (NodeRole::TaskRoot, NodeStatus::Open | NodeStatus::Closed)
                    | (
                        NodeRole::Work,
                        NodeStatus::Pending
                            | NodeStatus::Ready
                            | NodeStatus::Running
                            | NodeStatus::Blocked
                            | NodeStatus::Completed
                    )
                    | (
                        NodeRole::Finish,
                        NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Closed
                    )
            );
            assert_eq!(node.status_allowed(), expected, "{role:?}/{status:?}");
        }
    }
}
