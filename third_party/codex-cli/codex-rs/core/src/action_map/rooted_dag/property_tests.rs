use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::MapId;
use super::model::MapNode;
use super::model::NodeId;
use super::model::TaskSpaceMap;
use pretty_assertions::assert_eq;
use proptest::prelude::*;
use proptest::test_runner::RngAlgorithm;
use proptest::test_runner::RngSeed;
use std::collections::BTreeMap;

const ROOT_ID: &str = "root";
const FINISH_ID: &str = "finish";
const MAX_WORK_NODES: usize = 10;
const MAX_ARBITRARY_EDGES: usize = 64;

fn external_node_ids(work_node_count: usize) -> Vec<String> {
    let mut ids = Vec::with_capacity(work_node_count + 2);
    ids.push(ROOT_ID.to_string());
    ids.extend((0..work_node_count).map(|index| format!("work-{index}")));
    ids.push(FINISH_ID.to_string());
    ids
}

fn map_with_external_edges(
    work_node_count: usize,
    external_edges: Vec<(String, String)>,
) -> TaskSpaceMap {
    let ids = external_node_ids(work_node_count);
    let mut nodes = BTreeMap::new();
    nodes.insert(
        NodeId::new(ROOT_ID),
        MapNode::task_root("generated root", Vec::new()),
    );
    for id in ids.iter().skip(1).take(work_node_count) {
        nodes.insert(NodeId::new(id), MapNode::work(id));
    }
    nodes.insert(NodeId::new(FINISH_ID), MapNode::finish("generated finish"));

    TaskSpaceMap {
        id: MapId::new("generated-map"),
        root_node_id: NodeId::new(ROOT_ID),
        finish_node_id: NodeId::new(FINISH_ID),
        nodes,
        edges: external_edges
            .into_iter()
            .map(|(from, to)| MapEdge::new(from, to))
            .collect(),
        revision: 1,
        current_binding: None,
        terminal_summary_ref: None,
    }
}

fn valid_forward_dag_strategy() -> impl Strategy<Value = TaskSpaceMap> {
    (0..=MAX_WORK_NODES).prop_flat_map(|work_node_count| {
        let ids = external_node_ids(work_node_count);
        let optional_edges = (0..ids.len())
            .flat_map(|from| {
                let ids = &ids;
                (from + 2..ids.len()).map(move |to| (ids[from].clone(), ids[to].clone()))
            })
            .collect::<Vec<_>>();
        prop::collection::vec(any::<bool>(), optional_edges.len()).prop_map(move |included| {
            let mut edges = ids
                .windows(2)
                .map(|pair| (pair[0].clone(), pair[1].clone()))
                .collect::<Vec<_>>();
            edges.extend(
                optional_edges
                    .iter()
                    .zip(included)
                    .filter(|(_, include)| *include)
                    .map(|(edge, _)| edge.clone()),
            );
            map_with_external_edges(work_node_count, edges)
        })
    })
}

fn arbitrary_directed_graph_strategy() -> impl Strategy<Value = TaskSpaceMap> {
    (0..=MAX_WORK_NODES).prop_flat_map(|work_node_count| {
        let ids = external_node_ids(work_node_count);
        prop::collection::vec(
            (prop::sample::select(ids.clone()), prop::sample::select(ids)),
            0..=MAX_ARBITRARY_EDGES,
        )
        .prop_map(move |edges| map_with_external_edges(work_node_count, edges))
    })
}

fn graph_with_edge_order_strategy() -> impl Strategy<Value = (TaskSpaceMap, Vec<u64>)> {
    arbitrary_directed_graph_strategy().prop_flat_map(|map| {
        let edge_count = map.edges.len();
        (Just(map), prop::collection::vec(any::<u64>(), edge_count))
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x05ee_dd46_2026),
        ..ProptestConfig::default()
    })]

    #[test]
    fn generated_forward_dags_with_a_mandatory_chain_validate(
        map in valid_forward_dag_strategy(),
    ) {
        assert_eq!(validate(&map), Vec::<Violation>::new());
    }

    #[test]
    fn arbitrary_directed_graphs_never_panic(
        map in arbitrary_directed_graph_strategy(),
    ) {
        let _violations = validate(&map);
    }

    #[test]
    fn adding_finish_to_root_to_a_valid_graph_reports_a_cycle(
        mut map in valid_forward_dag_strategy(),
    ) {
        map.edges.push(MapEdge::new(FINISH_ID, ROOT_ID));

        assert_eq!(
            validate(&map),
            vec![Violation {
                code: ViolationCode::CycleDetected,
                subjects: Vec::new(),
            }]
        );
    }

    #[test]
    fn validation_result_is_independent_of_edge_order(
        (map, order_keys) in graph_with_edge_order_strategy(),
    ) {
        let expected = validate(&map);
        let mut keyed_edges = map
            .edges
            .iter()
            .cloned()
            .zip(order_keys)
            .enumerate()
            .collect::<Vec<_>>();
        keyed_edges.sort_by_key(|(original_order, (_, key))| (*key, *original_order));
        let mut reordered = map;
        reordered.edges = keyed_edges
            .into_iter()
            .map(|(_, (edge, _))| edge)
            .collect();

        assert_eq!(validate(&reordered), expected);
    }
}
