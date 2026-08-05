use super::events::EventBatch;
use super::events::MapFact;
use super::events::ReplayError;
use super::events::apply_batch;
use super::events::replay_batches;
use super::model::ActionRecord;
use super::model::CompletionRecord;
use super::model::EvidenceRef;
use super::model::MapEdge;
use super::model::NodeState;
use super::model::TerminalRecord;
use super::model::is_complete;
use super::model::map_node;
use super::model::state_sha256;
use super::transactions::ActionInput;
use super::transactions::AttachActionFacts;
use super::transactions::EvidenceRefInput;
use super::transactions::ExecuteTransaction;
use super::transactions::FinalCompletion;
use super::transactions::FinishMap;
use super::transactions::GraphMutation;
use super::transactions::InitializeMap;
use super::transactions::NodeMutation;
use super::transactions::ReopenMap;
use super::transactions::ResultRefInput;
use super::transactions::attach_action_facts;
use super::transactions::execute;
use super::transactions::finish_map;
use super::transactions::initialize;
use super::transactions::reopen_map;
use super::transitions::derive_node_state;
use pretty_assertions::assert_eq;

fn edge(from: &str, to: &str) -> MapEdge {
    MapEdge {
        from: from.into(),
        to: to.into(),
    }
}

fn action(id: &str, node_id: &str) -> ActionInput {
    ActionInput {
        action: ActionRecord {
            action_id: format!("action-{id}"),
            node_id: node_id.into(),
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
        actions: vec![action("inspect", "inspect")],
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

    let released = attach_action_facts(
        &map,
        AttachActionFacts {
            expected_revision: map.revision,
            action_id: "action-inspect".into(),
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
    assert_eq!(
        derive_node_state(&map, "inspect"),
        Some(NodeState::InFlight)
    );

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
            actions: vec![action("implement", "implement")],
        },
    )
    .unwrap();
    map = advanced.map;
    journal.push(advanced.events);

    let released = attach_action_facts(
        &map,
        AttachActionFacts {
            expected_revision: map.revision,
            action_id: "action-implement".into(),
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
            actions: vec![action("verify", "verify")],
        },
    )
    .unwrap();
    map = advanced.map;
    journal.push(advanced.events);

    let released = attach_action_facts(
        &map,
        AttachActionFacts {
            expected_revision: map.revision,
            action_id: "action-verify".into(),
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
    assert!(wire.contains("action_recorded"));
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
fn replay_rejects_result_attribution_without_its_action() {
    let initialized = initialize_chain();
    let invalid = EventBatch {
        map_id: initialized.map.map_id.clone(),
        revision: initialized.map.revision + 1,
        facts: vec![MapFact::EvidenceAttributed {
            evidence_ref_id: "evidence-other".into(),
            evidence: EvidenceRef {
                node_id: "inspect".into(),
                action_id: "action-other".into(),
                kind: "test".into(),
            },
        }],
    };

    assert!(matches!(
        apply_batch(Some(&initialized.map), &invalid),
        Err(ReplayError::InvalidFact(violation))
            if violation.code == super::invariants::ViolationCode::ActionRecordInvalid
    ));
}

#[test]
fn replay_rejects_revision_gap_and_empty_batches() {
    let initialized = initialize_chain();
    let revision_gap = EventBatch {
        map_id: initialized.map.map_id.clone(),
        revision: initialized.map.revision + 2,
        facts: vec![MapFact::ActionRecorded {
            action: ActionRecord {
                action_id: "action-late".into(),
                node_id: "inspect".into(),
            },
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

#[test]
fn action_result_attachment_does_not_drive_node_lifecycle() {
    let initialized = initialize(InitializeMap {
        map_id: "action-facts-map".into(),
        root: map_node("root", "deliver", vec!["source".into()]),
        work_nodes: vec![map_node("work", "do work", vec![])],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![edge("root", "work"), edge("work", "finish")],
        actions: vec![action("work", "work")],
    })
    .unwrap();

    let attached = attach_action_facts(
        &initialized.map,
        AttachActionFacts {
            expected_revision: initialized.map.revision,
            action_id: "action-work".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-work".into(),
                is_error: false,
            }],
            evidence_refs: vec![],
        },
    )
    .unwrap();

    assert_eq!(
        derive_node_state(&attached.map, "work"),
        Some(NodeState::InFlight)
    );
}

#[test]
fn agent_can_complete_and_finish_while_an_action_has_no_result() {
    let initialized = initialize(InitializeMap {
        map_id: "agent-owned-lifecycle-map".into(),
        root: map_node("root", "deliver", vec!["source".into()]),
        work_nodes: vec![map_node("work", "do work", vec![])],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![edge("root", "work"), edge("work", "finish")],
        actions: vec![action("work", "work")],
    })
    .unwrap();

    let completed = execute(
        &initialized.map,
        ExecuteTransaction {
            expected_revision: initialized.map.revision,
            graph: GraphMutation::default(),
            node_mutations: vec![NodeMutation::Complete {
                node_id: "work".into(),
                record: completion("complete-work", &[], &[]),
            }],
            actions: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        derive_node_state(&completed.map, "work"),
        Some(NodeState::Completed)
    );

    let terminal = finish_map(
        &completed.map,
        FinishMap {
            expected_revision: completed.map.revision,
            finish_node_id: "finish".into(),
            final_completions: vec![],
            terminal: TerminalRecord {
                action_id: "finish-action".into(),
                summary_ref: "summary-ref".into(),
            },
        },
    )
    .unwrap();

    assert!(is_complete(&terminal.map));
    assert!(!terminal.map.result_refs.contains_key("result-work"));
}

#[test]
fn close_reopen_and_close_again_preserves_terminal_and_work_history() {
    let initialized = initialize(InitializeMap {
        map_id: "reopen-map".into(),
        root: map_node("root", "deliver", vec!["source".into()]),
        work_nodes: vec![map_node("initial", "initial work", vec![])],
        finish: map_node("finish", "close task", vec![]),
        edges: vec![edge("root", "initial"), edge("initial", "finish")],
        actions: vec![action("initial", "initial")],
    })
    .unwrap();
    let released = attach_action_facts(
        &initialized.map,
        AttachActionFacts {
            expected_revision: initialized.map.revision,
            action_id: "action-initial".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-initial".into(),
                is_error: false,
            }],
            evidence_refs: vec![],
        },
    )
    .unwrap();
    let first_terminal = TerminalRecord {
        action_id: "finish-first".into(),
        summary_ref: "first summary".into(),
    };
    let closed = finish_map(
        &released.map,
        FinishMap {
            expected_revision: released.map.revision,
            finish_node_id: "finish".into(),
            final_completions: vec![FinalCompletion {
                node_id: "initial".into(),
                record: completion("finish-first", &["result-initial"], &[]),
            }],
            terminal: first_terminal.clone(),
        },
    )
    .unwrap();

    let reopened = reopen_map(
        &closed.map,
        ReopenMap {
            expected_revision: closed.map.revision,
            add_work_nodes: vec![map_node("follow-up", "address feedback", vec![])],
            add_edges: vec![edge("root", "follow-up"), edge("follow-up", "finish")],
            actions: vec![action("follow-up", "follow-up")],
        },
    )
    .unwrap();

    assert_eq!(reopened.map.map_id, closed.map.map_id);
    assert_eq!(reopened.map.terminal_record, None);
    assert_eq!(reopened.map.terminal_history, vec![first_terminal.clone()]);
    assert!(reopened.map.completion_records.contains_key("initial"));
    assert_eq!(
        derive_node_state(&reopened.map, "follow-up"),
        Some(NodeState::InFlight)
    );
    assert_eq!(
        derive_node_state(&reopened.map, "finish"),
        Some(NodeState::Waiting)
    );

    let released_follow_up = attach_action_facts(
        &reopened.map,
        AttachActionFacts {
            expected_revision: reopened.map.revision,
            action_id: "action-follow-up".into(),
            result_refs: vec![ResultRefInput {
                result_ref_id: "result-follow-up".into(),
                is_error: false,
            }],
            evidence_refs: vec![],
        },
    )
    .unwrap();
    let second_terminal = TerminalRecord {
        action_id: "finish-second".into(),
        summary_ref: "second summary".into(),
    };
    let closed_again = finish_map(
        &released_follow_up.map,
        FinishMap {
            expected_revision: released_follow_up.map.revision,
            finish_node_id: "finish".into(),
            final_completions: vec![FinalCompletion {
                node_id: "follow-up".into(),
                record: completion("finish-second", &["result-follow-up"], &[]),
            }],
            terminal: second_terminal.clone(),
        },
    )
    .unwrap();

    assert_eq!(closed_again.map.terminal_record, Some(second_terminal));
    assert_eq!(closed_again.map.terminal_history, vec![first_terminal]);
    assert!(closed_again.map.completion_records.contains_key("initial"));
    assert!(
        closed_again
            .map
            .completion_records
            .contains_key("follow-up")
    );
    assert!(!closed_again.map.completion_records.contains_key("root"));
    assert!(!closed_again.map.completion_records.contains_key("finish"));
    assert!(is_complete(&closed_again.map));
}

#[test]
fn reopen_rejects_an_active_map_without_mutating_it() {
    let initialized = initialize_chain();
    let before = initialized.map.clone();
    let rejection = reopen_map(
        &initialized.map,
        ReopenMap {
            expected_revision: initialized.map.revision,
            add_work_nodes: vec![map_node("follow-up", "address feedback", vec![])],
            add_edges: vec![edge("root", "follow-up"), edge("follow-up", "finish")],
            actions: vec![action("follow-up", "follow-up")],
        },
    )
    .unwrap_err();

    assert_eq!(
        rejection.violations[0].code,
        super::invariants::ViolationCode::TransitionInvalid
    );
    assert_eq!(initialized.map, before);
}
