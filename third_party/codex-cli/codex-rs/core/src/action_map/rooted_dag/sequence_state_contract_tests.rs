use super::ActionReservation;
use super::CompletionRecord;
use super::ExecuteTransaction;
use super::GraphMutation;
use super::InitializeMap;
use super::MapEdge;
use super::NodeMutation;
use super::NodeState;
use super::ReservationInput;
use super::ReservationRelease;
use super::ResultRefInput;
use super::TaskSpaceMap;
use super::ViolationCode;
use super::derive_node_state;
use super::execute;
use super::initialize;
use super::map_node;
use super::release_reservation;

fn initialized() -> TaskSpaceMap {
    initialize(InitializeMap {
        map_id: "map-state-contract".into(),
        root: map_node("root", "solve", Vec::new()),
        work_nodes: vec![map_node("work", "work", Vec::new())],
        finish: map_node("finish", "finish", Vec::new()),
        edges: vec![
            MapEdge {
                from: "root".into(),
                to: "work".into(),
            },
            MapEdge {
                from: "work".into(),
                to: "finish".into(),
            },
        ],
        reservations: vec![ReservationInput {
            reservation_id: "reservation-work".into(),
            reservation: ActionReservation {
                action_id: "action-work".into(),
                node_id: "work".into(),
                tool_name: "exec_command".into(),
                response_call_index: 0,
            },
        }],
    })
    .expect("initial map")
    .map
}

fn release_with_error(is_error: bool) -> TaskSpaceMap {
    let map = initialized();
    release_reservation(
        &map,
        ReservationRelease {
            expected_revision: map.revision,
            reservation_id: "reservation-work".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: format!("result-{is_error}"),
                is_error,
            }],
            evidence_refs: Vec::new(),
        },
    )
    .expect("release result")
    .map
}

#[test]
fn success_and_failure_results_have_identical_node_lifecycle_effects() {
    for is_error in [false, true] {
        let map = release_with_error(is_error);
        assert_eq!(derive_node_state(&map, "work"), Some(NodeState::Ready));
        assert!(map.completion_records.is_empty());
        assert!(map.block_records.is_empty());
        assert_eq!(map.result_refs.len(), 1);
        assert_eq!(map.result_refs.values().next().unwrap().is_error, is_error);
    }
}

#[test]
fn current_map_invariant_blocks_completion_while_a_tool_is_unsettled() {
    let map = initialized();
    let rejection = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "work".into(),
                record: CompletionRecord {
                    action_id: "agent-complete".into(),
                    result_ref_ids: Vec::new(),
                    evidence_ref_ids: Vec::new(),
                },
            }],
            reservations: vec![ReservationInput {
                reservation_id: "reservation-follow-up".into(),
                reservation: ActionReservation {
                    action_id: "action-follow-up".into(),
                    node_id: "work".into(),
                    tool_name: "read_file".into(),
                    response_call_index: 1,
                },
            }],
        },
    )
    .expect_err("current invariant couples completion to tool settlement");

    assert!(
        rejection.violations.iter().any(|violation| matches!(
            violation.code,
            ViolationCode::FactConflict
                | ViolationCode::ReservationInvalid
                | ViolationCode::NodeStateInvalid
                | ViolationCode::TransitionInvalid
        )),
        "unexpected coupling rejection: {:?}",
        rejection.violations
    );
}
