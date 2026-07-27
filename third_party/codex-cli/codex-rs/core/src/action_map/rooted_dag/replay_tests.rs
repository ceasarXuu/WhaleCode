use super::events::EventBatch;
use super::events::MapFact;
use super::events::ReplayError;
use super::events::apply_batch;
use super::events::replay_batches;
use super::model::ActionReservation;
use super::model::CompletionRecord;
use super::model::EvidenceRef;
use super::model::MapEdge;
use super::model::NodeState;
use super::model::TerminalRecord;
use super::model::is_complete;
use super::model::map_node;
use super::model::state_sha256;
use super::transactions::EvidenceRefInput;
use super::transactions::ExecuteTransaction;
use super::transactions::FinalCompletion;
use super::transactions::FinishMap;
use super::transactions::GraphMutation;
use super::transactions::InitializeMap;
use super::transactions::NodeMutation;
use super::transactions::ReservationInput;
use super::transactions::ReservationRelease;
use super::transactions::ResultRefInput;
use super::transactions::execute;
use super::transactions::finish_map;
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

fn reservation(id: &str, node_id: &str) -> ReservationInput {
    ReservationInput {
        reservation_id: format!("reservation-{id}"),
        reservation: ActionReservation {
            action_id: format!("action-{id}"),
            node_id: node_id.into(),
            tool_name: "exec_command".into(),
            response_call_index: 0,
        },
    }
}

fn initialize_chain() -> super::transactions::Commit {
    initialize(InitializeMap {
        map_id: "replay-map".into(),
        root: map_node("root", "deliver", vec!["source".into()]),
        work_nodes: vec![
            map_node("inspect", "inspect", vec![]),
            map_node("implement", "implement", vec![]),
            map_node("verify", "verify", vec![]),
        ],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![
            edge("root", "inspect"),
            edge("inspect", "implement"),
            edge("implement", "verify"),
            edge("verify", "finish"),
        ],
        reservations: vec![reservation("inspect", "inspect")],
    })
    .unwrap()
}

fn completion(action_id: &str, results: &[&str], evidence: &[&str]) -> CompletionRecord {
    CompletionRecord {
        action_id: action_id.into(),
        result_ref_ids: results.iter().map(|value| (*value).to_string()).collect(),
        evidence_ref_ids: evidence.iter().map(|value| (*value).to_string()).collect(),
    }
}

#[test]
fn factual_journal_round_trips_to_identical_terminal_map() {
    let initialized = initialize_chain();
    let mut map = initialized.map;
    let mut journal = vec![initialized.events];

    let released = release_reservation(
        &map,
        ReservationRelease {
            expected_revision: map.revision,
            reservation_id: "reservation-inspect".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-inspect".into(),
                is_error: false,
            }],
            evidence_refs: vec![EvidenceRefInput {
                evidence_ref_id: "evidence-inspect".into(),
                kind: "source_read".into(),
            }],
        },
    )
    .unwrap();
    map = released.map;
    journal.push(released.events);
    assert_eq!(derive_node_state(&map, "inspect"), Some(NodeState::Ready));

    let advanced = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "inspect".into(),
                record: completion(
                    "complete-inspect",
                    &["result-inspect"],
                    &["evidence-inspect"],
                ),
            }],
            reservations: vec![reservation("implement", "implement")],
        },
    )
    .unwrap();
    map = advanced.map;
    journal.push(advanced.events);

    let released = release_reservation(
        &map,
        ReservationRelease {
            expected_revision: map.revision,
            reservation_id: "reservation-implement".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-implement".into(),
                is_error: false,
            }],
            evidence_refs: vec![],
        },
    )
    .unwrap();
    map = released.map;
    journal.push(released.events);

    let advanced = execute(
        &map,
        ExecuteTransaction {
            expected_revision: map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "implement".into(),
                record: completion("complete-implement", &["result-implement"], &[]),
            }],
            reservations: vec![reservation("verify", "verify")],
        },
    )
    .unwrap();
    map = advanced.map;
    journal.push(advanced.events);

    let released = release_reservation(
        &map,
        ReservationRelease {
            expected_revision: map.revision,
            reservation_id: "reservation-verify".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-verify".into(),
                is_error: false,
            }],
            evidence_refs: vec![],
        },
    )
    .unwrap();
    map = released.map;
    journal.push(released.events);

    let terminal = finish_map(
        &map,
        FinishMap {
            expected_revision: map.revision,
            finish_node_id: "finish".into(),
            final_completions: vec![FinalCompletion {
                node_id: "verify".into(),
                record: completion("complete-verify", &["result-verify"], &[]),
            }],
            terminal: TerminalRecord {
                action_id: "finish-action".into(),
                summary_ref: "summary-ref".into(),
            },
        },
    )
    .unwrap();
    map = terminal.map;
    journal.push(terminal.events);

    let encoded = serde_json::to_vec(&journal).unwrap();
    let restored: Vec<EventBatch> = serde_json::from_slice(&encoded).unwrap();
    let replayed = replay_batches(&restored).unwrap();

    assert_eq!(replayed, map);
    assert_eq!(
        state_sha256(&replayed).unwrap(),
        state_sha256(&map).unwrap()
    );
    assert_eq!(map.revision, 7);
    assert!(is_complete(&map));
    assert_eq!(derive_node_state(&map, "root"), Some(NodeState::Completed));
    assert_eq!(
        derive_node_state(&map, "finish"),
        Some(NodeState::Completed)
    );
}

#[test]
fn event_wire_contains_facts_but_no_derived_lifecycle() {
    let initialized = initialize_chain();
    let wire = serde_json::to_string(&initialized.events).unwrap();

    assert!(wire.contains("map_initialized"));
    assert!(wire.contains("action_reserved"));
    for forbidden in [
        "\"status\"",
        "\"ready\"",
        "\"waiting\"",
        "\"in_flight\"",
        "\"open\"",
        "\"current\"",
    ] {
        assert!(!wire.contains(forbidden), "{forbidden} leaked: {wire}");
    }
}

#[test]
fn replay_rejects_result_attribution_without_its_reservation() {
    let initialized = initialize_chain();
    let invalid = EventBatch {
        map_id: initialized.map.map_id.clone(),
        revision: initialized.map.revision + 1,
        facts: vec![MapFact::EvidenceAttributed {
            evidence_ref_id: "evidence-other".into(),
            evidence: EvidenceRef {
                node_id: "inspect".into(),
                action_id: "action-other".into(),
                reservation_id: "reservation-other".into(),
                kind: "test".into(),
            },
        }],
    };

    assert!(matches!(
        apply_batch(Some(&initialized.map), &invalid),
        Err(ReplayError::InvalidFact {
            code: super::invariants::ViolationCode::ReservationInvalid,
            ..
        })
    ));
}

#[test]
fn replay_rejects_revision_gap_and_empty_batches() {
    let initialized = initialize_chain();
    let revision_gap = EventBatch {
        map_id: initialized.map.map_id.clone(),
        revision: initialized.map.revision + 2,
        facts: vec![MapFact::ActionReleased {
            reservation_id: "reservation-inspect".into(),
        }],
    };
    let empty = EventBatch {
        map_id: initialized.map.map_id.clone(),
        revision: initialized.map.revision + 1,
        facts: vec![],
    };

    assert!(matches!(
        apply_batch(Some(&initialized.map), &revision_gap),
        Err(ReplayError::RevisionMismatch { .. })
    ));
    assert_eq!(
        apply_batch(Some(&initialized.map), &empty),
        Err(ReplayError::EmptyBatch)
    );
}

#[test]
fn explicit_finish_rejects_before_the_final_frontier_is_complete() {
    let initialized = initialize_chain();
    let rejection = finish_map(
        &initialized.map,
        FinishMap {
            expected_revision: initialized.map.revision,
            finish_node_id: "finish".into(),
            final_completions: vec![],
            terminal: TerminalRecord {
                action_id: "finish-too-early".into(),
                summary_ref: "summary-ref".into(),
            },
        },
    )
    .unwrap_err();

    assert_eq!(
        rejection.violations[0].code,
        super::invariants::ViolationCode::FinishNotReady
    );
    assert!(!is_complete(&initialized.map));
}
