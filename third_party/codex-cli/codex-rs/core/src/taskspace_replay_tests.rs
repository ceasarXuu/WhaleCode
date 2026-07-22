use super::taskspace_replay::*;
use crate::action_map::ActionMapCheckpointState;
use crate::action_map::ActionMapEdgeInput;
use crate::action_map::ActionMapInitializeFinishInput;
use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::NodeTransition;
use crate::action_map::build_snapshot_delta;
use crate::action_map::snapshot_sha256;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeSnapshotUpdatedEvent;
use codex_protocol::protocol::MapRuntimeTerminalCommittedEvent;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use codex_protocol::protocol::ThreadRolledBackEvent;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::protocol::UserMessageEvent;
use serde_json::json;

fn snapshot(experiment: bool) -> ActionMapSnapshot {
    let mut runtime = ActionMapRuntimeState::default();
    if experiment {
        runtime.set_mode_for_session(MapRuntimeMode::Experiment, ThreadId::new());
    }
    runtime.snapshot()
}

fn initialized_snapshot() -> ActionMapSnapshot {
    let owner = ThreadId::new();
    let mut runtime = ActionMapRuntimeState::default();
    runtime.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    runtime
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                root: ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "root".into(),
                },
                current_work_node: ActionMapInitializeNodeInput {
                    id: "work".into(),
                    goal: "work".into(),
                },
                finish: ActionMapInitializeFinishInput {
                    id: "finish".into(),
                },
                work_nodes: Vec::new(),
                edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "work".into(),
                    },
                    ActionMapEdgeInput {
                        from: "work".into(),
                        to: "finish".into(),
                    },
                ],
                source_event_ids: Vec::new(),
            },
        )
        .unwrap();
    runtime.snapshot()
}

fn delta_chain_snapshots() -> (ActionMapSnapshot, ActionMapSnapshot, ActionMapSnapshot) {
    let base = initialized_snapshot();
    let mut middle = base.clone();
    middle.routing_required = !base.routing_required;
    let mut expected = middle.clone();
    expected.reborn_requested = true;
    (base, middle, expected)
}

fn checkpoint(id: &str, snapshot: ActionMapSnapshot) -> RolloutItem {
    RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
        MapRuntimeSnapshotUpdatedEvent {
            checkpoint_id: id.to_string(),
            reason: "test".into(),
            snapshot_sha256: snapshot_sha256(&snapshot).unwrap(),
            snapshot,
        },
    )))
}

fn delta_item(delta: codex_protocol::protocol::MapRuntimeSnapshotDeltaEvent) -> RolloutItem {
    RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(delta)))
}

fn terminal_crash_window() -> (
    ActionMapSnapshot,
    ActionMapSnapshot,
    RolloutItem,
    RolloutItem,
) {
    terminal_crash_window_with_atomic_completion(false)
}

fn terminal_crash_window_with_atomic_completion(
    atomic_completion: bool,
) -> (
    ActionMapSnapshot,
    ActionMapSnapshot,
    RolloutItem,
    RolloutItem,
) {
    let owner = ThreadId::new();
    let mut runtime = ActionMapRuntimeState::default();
    runtime.set_mode_for_session(MapRuntimeMode::Experiment, owner);
    runtime
        .initialize_map_for_main(
            owner,
            ActionMapInitializeInput {
                root: ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "root".into(),
                },
                current_work_node: ActionMapInitializeNodeInput {
                    id: "work".into(),
                    goal: "work".into(),
                },
                finish: ActionMapInitializeFinishInput {
                    id: "finish".into(),
                },
                work_nodes: Vec::new(),
                edges: vec![
                    ActionMapEdgeInput {
                        from: "root".into(),
                        to: "work".into(),
                    },
                    ActionMapEdgeInput {
                        from: "work".into(),
                        to: "finish".into(),
                    },
                ],
                source_event_ids: Vec::new(),
            },
        )
        .unwrap();
    let (pre_terminal, events) = if atomic_completion {
        let pre_terminal = runtime.snapshot();
        let (_, events) = runtime
            .complete_then_end_for_main(
                owner,
                2,
                "work".into(),
                "terminal summary".into(),
                "complete-event".into(),
            )
            .unwrap();
        (pre_terminal, events)
    } else {
        runtime
            .transition_node_for_main(
                owner,
                2,
                "work".into(),
                NodeTransition::Complete,
                "complete-event".into(),
            )
            .unwrap();
        let pre_terminal = runtime.snapshot();
        let (_, events) = runtime
            .close_ready_finish_for_main(owner, 3, "terminal summary".into())
            .unwrap();
        (pre_terminal, events)
    };
    let terminal = runtime.snapshot();
    let mut graph_revision = None;
    let mut trace_event = None;
    for event in events {
        match event {
            MapRuntimeEvent::GraphRevisionCommitted(event) => graph_revision = Some(event),
            MapRuntimeEvent::TaskspaceTraceEventRecorded(event) => trace_event = Some(event),
            other => panic!("unexpected terminal event: {other:?}"),
        }
    }
    let graph_revision = graph_revision.expect("finish emits a graph revision event");
    let graph_event = RolloutItem::EventMsg(EventMsg::MapRuntime(
        MapRuntimeEvent::GraphRevisionCommitted(graph_revision.clone()),
    ));
    let snapshot_sha256 = snapshot_sha256(&terminal).unwrap();
    let terminal_event = RolloutItem::EventMsg(EventMsg::MapRuntime(
        MapRuntimeEvent::TerminalCommitted(Box::new(MapRuntimeTerminalCommittedEvent {
            checkpoint_id: format!("map-terminal-{}", &snapshot_sha256[..16]),
            snapshot_sha256,
            snapshot: terminal.clone(),
            graph_revision,
            trace_event: trace_event.expect("finish emits a trace event"),
        })),
    ));
    (pre_terminal, terminal, graph_event, terminal_event)
}

#[test]
fn replay_applies_checkpoint_and_chained_deltas() {
    let (base, middle, expected) = delta_chain_snapshots();
    let mut state = ActionMapCheckpointState::default();
    state.install("cp".into(), snapshot_sha256(&base).unwrap(), base.clone());
    let first = build_snapshot_delta(&mut state, &middle).unwrap().unwrap();
    let second = build_snapshot_delta(&mut state, &expected)
        .unwrap()
        .unwrap();
    let items = vec![
        checkpoint("cp", base),
        delta_item(first),
        delta_item(second),
    ];

    let mut restored = ActionMapRuntimeState::default();
    restored.restore_snapshot(expected.clone()).unwrap();
    assert_eq!(restored.snapshot(), expected);

    let replayed = replay_rollout_items("rollout".into(), 0, &items).unwrap();

    assert_eq!(replayed.state.snapshot, expected);
    assert_eq!(replayed.state.parsed_checkpoint_count, 1);
    assert_eq!(replayed.state.parsed_delta_count, 2);
    assert_eq!(replayed.state.surviving_checkpoint_count, 1);
    assert_eq!(replayed.state.active_chain_applied_delta_count, 2);
    assert_eq!(replayed.state.active_chain_last_delta_sequence, 2);
}

#[test]
fn replay_rejects_terminal_graph_commit_without_transaction_envelope() {
    let (pre_terminal, _, graph_event, _) = terminal_crash_window();
    let error = replay_rollout_items(
        "terminal-crash-window".into(),
        0,
        &[checkpoint("pre-terminal", pre_terminal), graph_event],
    )
    .unwrap_err();

    assert_eq!(error.code, TaskSpaceReplayErrorCode::IncompleteTransaction);
}

#[test]
fn terminal_transaction_envelope_replays_as_one_checkpoint() {
    let (pre_terminal, terminal, _, terminal_event) = terminal_crash_window();
    let replayed = replay_rollout_items(
        "terminal-transaction".into(),
        0,
        &[checkpoint("pre-terminal", pre_terminal), terminal_event],
    )
    .unwrap();

    assert_eq!(replayed.state.snapshot, terminal);
    assert_eq!(replayed.state.parsed_checkpoint_count, 2);
    assert_eq!(replayed.state.active_chain_applied_delta_count, 0);
    assert!(replayed.state.snapshot.map.as_ref().unwrap().complete);
}

#[test]
fn atomic_completion_terminal_envelope_replays_as_one_checkpoint() {
    let (pre_terminal, terminal, _, terminal_event) =
        terminal_crash_window_with_atomic_completion(true);
    let replayed = replay_rollout_items(
        "atomic-terminal-transaction".into(),
        0,
        &[checkpoint("pre-terminal", pre_terminal), terminal_event],
    )
    .unwrap();

    assert_eq!(replayed.state.snapshot, terminal);
    assert!(replayed.state.snapshot.map.as_ref().unwrap().complete);
    assert_eq!(replayed.state.snapshot.map.as_ref().unwrap().revision, 3);
}

#[test]
fn atomic_completion_graph_commit_without_envelope_is_rejected() {
    let (pre_terminal, _, graph_event, _) = terminal_crash_window_with_atomic_completion(true);
    let error = replay_rollout_items(
        "atomic-terminal-crash-window".into(),
        0,
        &[checkpoint("pre-terminal", pre_terminal), graph_event],
    )
    .unwrap_err();

    assert_eq!(error.code, TaskSpaceReplayErrorCode::IncompleteTransaction);
}

#[test]
fn terminal_transaction_corruption_is_fatal_without_partial_restore() {
    let (pre_terminal, _, _, terminal_item) = terminal_crash_window();
    let RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::TerminalCommitted(terminal))) =
        terminal_item
    else {
        panic!("terminal fixture must produce an envelope");
    };

    let mut bad_hash = (*terminal).clone();
    bad_hash.snapshot_sha256 = "bad-hash".into();
    let mut bad_revision = (*terminal).clone();
    bad_revision.graph_revision.revision += 1;
    let mut bad_trace = (*terminal).clone();
    bad_trace.trace_event.kind = "not-terminal".into();

    let cases = [
        (bad_hash, TaskSpaceReplayErrorCode::ResultHash),
        (
            bad_revision,
            TaskSpaceReplayErrorCode::IncompleteTransaction,
        ),
        (bad_trace, TaskSpaceReplayErrorCode::IncompleteTransaction),
    ];
    for (event, expected_code) in cases {
        let error = replay_rollout_items(
            "terminal-corruption".into(),
            0,
            &[
                checkpoint("pre-terminal", pre_terminal.clone()),
                RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::TerminalCommitted(
                    Box::new(event),
                ))),
            ],
        )
        .unwrap_err();
        assert_eq!(error.code, expected_code);
    }
}

#[test]
fn replay_rejects_sequence_gap_or_reorder_before_hash_checks() {
    let (base, middle, expected) = delta_chain_snapshots();
    let mut state = ActionMapCheckpointState::default();
    state.install("cp".into(), snapshot_sha256(&base).unwrap(), base.clone());
    let first = build_snapshot_delta(&mut state, &middle).unwrap().unwrap();
    let second = build_snapshot_delta(&mut state, &expected)
        .unwrap()
        .unwrap();

    let gap = replay_rollout_items(
        "rollout".into(),
        0,
        &[checkpoint("cp", base.clone()), delta_item(second.clone())],
    )
    .unwrap_err();
    let reorder = replay_rollout_items(
        "rollout".into(),
        0,
        &[
            checkpoint("cp", base),
            delta_item(second),
            delta_item(first),
        ],
    )
    .unwrap_err();

    assert_eq!(gap.code, TaskSpaceReplayErrorCode::SequenceGapOrOrder);
    assert_eq!(reorder.code, TaskSpaceReplayErrorCode::SequenceGapOrOrder);
}

#[test]
fn replay_rejects_chain_hashes_and_invalid_patch() {
    let base = snapshot(true);
    let expected = snapshot(false);
    let mut state = ActionMapCheckpointState::default();
    state.install("cp".into(), snapshot_sha256(&base).unwrap(), base.clone());
    let delta = build_snapshot_delta(&mut state, &expected)
        .unwrap()
        .unwrap();

    let mut bad_base_id = delta.clone();
    bad_base_id.base_checkpoint_id = "other-checkpoint".into();
    let mut bad_base = delta.clone();
    bad_base.base_snapshot_sha256 = "bad-base".into();
    let mut bad_previous = delta.clone();
    bad_previous.previous_snapshot_sha256 = "bad-previous".into();
    let mut bad_result = delta.clone();
    bad_result.snapshot_sha256 = "bad-result".into();
    let mut bad_patch = delta;
    bad_patch.patch = json!({"not": "a patch"});

    let cases = [
        (bad_base_id, TaskSpaceReplayErrorCode::BaseIdOrHash),
        (bad_base, TaskSpaceReplayErrorCode::BaseIdOrHash),
        (bad_previous, TaskSpaceReplayErrorCode::PreviousHash),
        (bad_result, TaskSpaceReplayErrorCode::ResultHash),
        (bad_patch, TaskSpaceReplayErrorCode::InvalidPatch),
    ];
    for (delta, code) in cases {
        let err = replay_rollout_items(
            "rollout".into(),
            0,
            &[checkpoint("cp", base.clone()), delta_item(delta)],
        )
        .unwrap_err();
        assert_eq!(err.code, code);
    }
}

#[test]
fn later_checkpoint_resets_active_chain() {
    let first = snapshot(true);
    let second = snapshot(false);
    let replayed = replay_rollout_items(
        "rollout".into(),
        0,
        &[checkpoint("cp1", first), checkpoint("cp2", second.clone())],
    )
    .unwrap();

    assert_eq!(replayed.state.snapshot, second);
    assert_eq!(replayed.state.parsed_checkpoint_count, 2);
    assert_eq!(replayed.state.active_checkpoint_id, "cp2");
    assert_eq!(replayed.state.active_chain_applied_delta_count, 0);
}

#[test]
fn replay_preserves_mechanical_blank_map_snapshot() {
    let blank = snapshot(true);

    let replayed =
        replay_rollout_items("rollout".into(), 0, &[checkpoint("blank", blank.clone())]).unwrap();

    assert_eq!(replayed.state.snapshot, blank);
}

#[test]
fn rollback_drops_newest_user_turn_snapshot() {
    let old = snapshot(false);
    let new = snapshot(true);
    let items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: "t1".into(),
            started_at: None,
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "old".into(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
        })),
        checkpoint("old", old.clone()),
        RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: "t2".into(),
            started_at: None,
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "new".into(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
        })),
        checkpoint("new", new),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent {
            num_turns: 1,
        })),
    ];

    let replayed = replay_rollout_items("rollout".into(), 0, &items).unwrap();

    assert_eq!(replayed.state.snapshot, old);
    assert_eq!(replayed.state.surviving_checkpoint_count, 1);
}

#[test]
fn rollback_drops_newest_user_turn_mode_change() {
    let items = vec![
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Standard,
                current_mode: MapRuntimeMode::Experiment,
            },
        ))),
        RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: "t2".into(),
            started_at: None,
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "new".into(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Experiment,
                current_mode: MapRuntimeMode::Standard,
            },
        ))),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent {
            num_turns: 1,
        })),
    ];

    assert_eq!(replay_surviving_mode(&items), MapRuntimeMode::Experiment);
}

#[test]
fn compaction_keeps_newer_checkpoint_as_active_chain() {
    let old = snapshot(true);
    let new = snapshot(false);
    let items = vec![
        checkpoint("old", old),
        RolloutItem::Compacted(codex_protocol::protocol::CompactedItem {
            message: "summary".into(),
            replacement_history: Some(Vec::new()),
        }),
        checkpoint("new", new.clone()),
    ];

    let replayed = replay_rollout_items("rollout".into(), 0, &items).unwrap();

    assert_eq!(replayed.state.snapshot, new);
    assert_eq!(replayed.state.active_checkpoint_id, "new");
}

#[test]
fn replay_rejects_unsupported_schema_and_domain_invariant() {
    let mut legacy = snapshot(true);
    legacy.schema_version = "TaskSpaceSnapshotR5V1".into();
    let legacy_err =
        replay_rollout_items("rollout".into(), 0, &[checkpoint("legacy", legacy)]).unwrap_err();
    assert_eq!(
        legacy_err.code,
        TaskSpaceReplayErrorCode::UnsupportedSnapshotSchema
    );

    let mut invalid = initialized_snapshot();
    let map = invalid.map.as_mut().unwrap();
    let lease_id = map.leases[0].id.clone();
    let finish = map
        .nodes
        .iter_mut()
        .find(|node| node.id == "finish")
        .unwrap();
    finish.active_lease = Some(lease_id);
    map.leases[0].node_id = "finish".into();
    map.current_node_id = Some("finish".into());

    let invariant_err =
        replay_rollout_items("rollout".into(), 0, &[checkpoint("bad-domain", invalid)])
            .unwrap_err();
    assert_eq!(
        invariant_err.code,
        TaskSpaceReplayErrorCode::DomainInvariant
    );
}

#[test]
fn replay_rejects_noncanonical_restore_roundtrip() {
    let mut noncanonical = snapshot(true);
    noncanonical.sentinel_warnings.push(
        codex_protocol::protocol::ActionMapSnapshotSentinelWarningRef {
            id: "warning-1".into(),
            sentinel_type: "unknown".into(),
            status: "unknown".into(),
            severity: "unknown".into(),
            task_id: None,
            map_id: "map".into(),
            node_id: "node".into(),
            result_id: None,
            trace_event_ids: Vec::new(),
            reason: "invalid warning should not be normalized silently".into(),
            clearance_action: "none".into(),
            clear_action: None,
            created_at_ms: 0,
            cleared_at_ms: None,
        },
    );

    let err = replay_rollout_items(
        "rollout".into(),
        0,
        &[checkpoint("noncanonical", noncanonical)],
    )
    .unwrap_err();

    assert_eq!(err.code, TaskSpaceReplayErrorCode::NoncanonicalSnapshot);
}

#[test]
fn standard_rollout_without_map_is_not_applicable() {
    let err = replay_rollout_items("rollout".into(), 0, &[]).unwrap_err();
    assert_eq!(err.code, TaskSpaceReplayErrorCode::NotApplicable);
}

#[test]
fn delta_without_checkpoint_is_fatal() {
    let base = snapshot(true);
    let expected = snapshot(false);
    let mut state = ActionMapCheckpointState::default();
    state.install("cp".into(), snapshot_sha256(&base).unwrap(), base);
    let delta = build_snapshot_delta(&mut state, &expected)
        .unwrap()
        .unwrap();
    let items = vec![delta_item(delta)];

    let err = replay_rollout_items("rollout".into(), 0, &items).unwrap_err();

    assert_eq!(err.code, TaskSpaceReplayErrorCode::MissingCheckpoint);
}

#[test]
fn parse_error_count_is_fatal_for_loaded_replay() {
    let loaded = LoadedRollout {
        rollout_sha256: "rollout".into(),
        parse_error_count: 1,
        items: vec![checkpoint("cp", snapshot(true))],
    };

    let err = replay_loaded_rollout(&loaded).unwrap_err();

    assert_eq!(err.code, TaskSpaceReplayErrorCode::Parse);
}

#[tokio::test]
async fn loader_reports_raw_sha_and_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout.jsonl");
    let line = serde_json::to_string(&RolloutLine {
        timestamp: "2026-07-16T00:00:00Z".into(),
        item: checkpoint("cp", snapshot(true)),
    })
    .unwrap();
    tokio::fs::write(&path, format!("{line}\n{{broken\n"))
        .await
        .unwrap();

    let loaded = load_rollout(&path).await.unwrap();

    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.parse_error_count, 1);
    assert_eq!(loaded.rollout_sha256.len(), 64);
}

#[tokio::test]
async fn loader_rejects_invalid_utf8_as_parse_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout.jsonl");
    tokio::fs::write(&path, [0xff, 0xfe]).await.unwrap();

    let error = load_rollout(&path).await.unwrap_err();

    assert_eq!(error.code, TaskSpaceReplayErrorCode::Parse);
}
