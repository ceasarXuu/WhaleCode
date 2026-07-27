use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::TaskSpaceMap;
use super::model::map_node;
use super::model::new_map;
use proptest::prelude::*;
use proptest::test_runner::RngAlgorithm;
use proptest::test_runner::RngSeed;

const ROOT_ID: &str = "root";
const FINISH_ID: &str = "finish";
const MAX_WORK_NODES: usize = 12;
const MAX_ARBITRARY_EDGES: usize = 48;

fn external_node_ids(work_node_count: usize) -> Vec<String> {
    std::iter::once(ROOT_ID.to_string())
        .chain((0..work_node_count).map(|index| format!("work-{index:02}")))
        .chain(std::iter::once(FINISH_ID.to_string()))
        .collect()
}

fn map_with_external_edges(
    work_node_count: usize,
    external_edges: Vec<(String, String)>,
) -> TaskSpaceMap {
    new_map(
        "generated-map".into(),
        map_node(ROOT_ID, "generated root", vec![]),
        (0..work_node_count)
            .map(|index| {
                let id = format!("work-{index:02}");
                map_node(&id, &id, vec![])
            })
            .collect(),
        map_node(FINISH_ID, "finish task", vec![]),
        external_edges
            .into_iter()
            .map(|(from, to)| MapEdge { from, to })
            .collect(),
    )
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

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0xa2b1_fa47_2026),
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
        map.edges.push(MapEdge {
            from: FINISH_ID.into(),
            to: ROOT_ID.into(),
        });

        assert!(validate(&map)
            .iter()
            .any(|violation| violation.code == ViolationCode::CycleDetected));
    }

    #[test]
    fn validation_is_independent_of_edge_and_work_node_order(
        mut map in valid_forward_dag_strategy(),
        edge_seed in any::<u64>(),
        node_seed in any::<u64>(),
    ) {
        let expected = validate(&map);
        map.edges.sort_by_key(|edge| {
            stable_order_key(edge_seed, format!("{}->{}", edge.from, edge.to).as_bytes())
        });
        map.work_nodes.sort_by_key(|node| {
            stable_order_key(node_seed, node.node_id.as_bytes())
        });

        assert_eq!(validate(&map), expected);
    }
}

fn stable_order_key(seed: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(seed, |state, byte| state.rotate_left(5) ^ u64::from(*byte))
}
