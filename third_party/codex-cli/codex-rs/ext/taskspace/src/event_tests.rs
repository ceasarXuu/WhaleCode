use super::events::EventBatch;
use super::events::MapFact;
use super::events::ReplayError;
use super::events::apply_batch;
use super::events::replay_batches;
use super::model::ActionReservation;
use super::model::MapEdge;
use super::model::map_node;

fn initialization_batch() -> EventBatch {
    EventBatch {
        map_id: "event-map".into(),
        revision: 1,
        facts: vec![
            MapFact::MapInitialized {
                map_id: "event-map".into(),
                root: map_node("root", "deliver", vec!["source".into()]),
                work_nodes: vec![map_node("work", "implement", vec![])],
                finish: map_node("finish", "close", vec![]),
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
            },
            MapFact::ActionReserved {
                reservation_id: "reservation-work".into(),
                reservation: ActionReservation {
                    action_id: "action-work".into(),
                    node_id: "work".into(),
                    tool_name: "exec_command".into(),
                    response_call_index: 0,
                },
            },
        ],
    }
}

#[test]
fn event_batches_round_trip_and_replay_deterministically() {
    let initialized = initialization_batch();
    let expected = apply_batch(None, &initialized).expect("initial batch should apply");
    let encoded = serde_json::to_vec(&[initialized]).expect("journal should encode");
    let restored: Vec<EventBatch> =
        serde_json::from_slice(&encoded).expect("journal should decode");

    assert_eq!(replay_batches(&restored), Ok(expected));
}

#[test]
fn event_wire_contains_facts_without_derived_lifecycle() {
    let wire = serde_json::to_string(&initialization_batch()).expect("batch should encode");

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
fn replay_rejects_revision_gaps_and_empty_batches() {
    let initialized = initialization_batch();
    let map = apply_batch(None, &initialized).expect("initial batch should apply");
    let revision_gap = EventBatch {
        map_id: map.map_id.clone(),
        revision: map.revision + 2,
        facts: vec![MapFact::ActionReleased {
            reservation_id: "reservation-work".into(),
        }],
    };
    let empty = EventBatch {
        map_id: map.map_id.clone(),
        revision: map.revision + 1,
        facts: vec![],
    };

    assert!(matches!(
        apply_batch(Some(&map), &revision_gap),
        Err(ReplayError::RevisionMismatch { .. })
    ));
    assert_eq!(
        apply_batch(Some(&map), &empty),
        Err(ReplayError::EmptyBatch)
    );
}
