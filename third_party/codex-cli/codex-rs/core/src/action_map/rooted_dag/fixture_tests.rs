use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::TaskSpaceMap;
use super::model::canonicalize;
use super::model::map_node;
use super::model::new_map;
use super::model::state_sha256;
use pretty_assertions::assert_eq;

fn valid_fork_join() -> TaskSpaceMap {
    new_map(
        "fixture-map".into(),
        map_node("root", "deliver feature", vec!["user-turn".into()]),
        vec![
            map_node("inspect", "inspect", vec![]),
            map_node("research", "research", vec![]),
            map_node("implement", "implement", vec![]),
        ],
        map_node("finish", "close task", vec![]),
        vec![
            edge("root", "inspect"),
            edge("root", "research"),
            edge("inspect", "implement"),
            edge("research", "implement"),
            edge("implement", "finish"),
        ],
    )
}

fn edge(from: &str, to: &str) -> MapEdge {
    MapEdge {
        from: from.into(),
        to: to.into(),
    }
}

#[test]
fn canonical_json_fixture_is_strict_and_fact_only() {
    let fixture = serde_json::json!({
        "schema_version": "taskspace-canonical-map-v3",
        "map_id": "fixture-map",
        "root": {
            "node_id": "root",
            "goal": "deliver feature",
            "source_refs": ["user-turn"]
        },
        "work_nodes": [
            {"node_id": "inspect", "goal": "inspect", "source_refs": []},
            {"node_id": "implement", "goal": "implement", "source_refs": []}
        ],
        "finish": {
            "node_id": "finish",
            "goal": "close task",
            "source_refs": []
        },
        "edges": [
            {"from": "root", "to": "inspect"},
            {"from": "inspect", "to": "implement"},
            {"from": "implement", "to": "finish"}
        ],
        "completion_records": {},
        "block_records": {},
        "action_records": {},
        "result_refs": {},
        "evidence_refs": {},
        "terminal_record": null,
        "terminal_history": [],
        "revision": 1
    });

    let map: TaskSpaceMap = serde_json::from_value(fixture).unwrap();
    let encoded = serde_json::to_string(&map).unwrap();

    assert_eq!(validate(&map), vec![]);
    for forbidden in ["status", "active_lease", "current_binding", "current_node"] {
        assert!(
            !encoded.contains(forbidden),
            "{forbidden} leaked: {encoded}"
        );
    }
}

#[test]
fn representative_invalid_graph_fixtures_report_mechanical_codes() {
    let cases = [
        (
            "duplicate",
            vec![edge("root", "inspect"), edge("root", "inspect")],
            ViolationCode::DuplicateEdge,
        ),
        (
            "cycle",
            vec![edge("finish", "root")],
            ViolationCode::CycleDetected,
        ),
        (
            "missing",
            vec![edge("missing", "inspect")],
            ViolationCode::EdgeEndpointMissing,
        ),
        (
            "self",
            vec![edge("inspect", "inspect")],
            ViolationCode::SelfLoop,
        ),
    ];

    for (name, extra_edges, expected) in cases {
        let mut map = valid_fork_join();
        map.map_id = name.into();
        map.edges.extend(extra_edges);
        assert!(
            validate(&map)
                .iter()
                .any(|violation| violation.code == expected),
            "{name} did not report {expected:?}"
        );
    }
}

#[test]
fn multi_parent_fixture_preserves_one_root_and_one_finish() {
    let map = valid_fork_join();

    assert_eq!(validate(&map), vec![]);
    assert_eq!(
        map.edges
            .iter()
            .filter(|edge| edge.to == "implement")
            .count(),
        2
    );
    assert_eq!(map.root.node_id, "root");
    assert_eq!(map.finish.node_id, "finish");
}

#[test]
fn canonical_hash_is_independent_of_collection_order() {
    let mut left = valid_fork_join();
    let mut right = left.clone();
    right.edges.reverse();
    right.work_nodes.reverse();

    canonicalize(&mut left);
    canonicalize(&mut right);

    assert_eq!(state_sha256(&left).unwrap(), state_sha256(&right).unwrap());
}

#[test]
fn canonicalization_does_not_rewrite_semantic_source_order() {
    let mut map = valid_fork_join();
    map.root.source_refs = vec!["source-b".into(), "source-a".into(), "source-b".into()];

    canonicalize(&mut map);

    assert_eq!(
        map.root.source_refs,
        vec!["source-b", "source-a", "source-b"]
    );
}
