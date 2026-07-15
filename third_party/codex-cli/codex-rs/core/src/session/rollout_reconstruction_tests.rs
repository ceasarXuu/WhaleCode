use super::*;

use super::tests::make_session_and_context;
use crate::action_map::ActionMapCheckpointState;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::TaskSpaceEventStore;
use crate::action_map::build_snapshot_delta;
use crate::action_map::snapshot_sha256;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeSnapshotUpdatedEvent;
use codex_protocol::protocol::MapRuntimeTaskContextOwnershipChangedEvent;
use codex_protocol::protocol::ResumedHistory;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        end_turn: None,
        phase: None,
    }
}

fn map_checkpoint_event(
    checkpoint_id: &str,
    snapshot: ActionMapSnapshot,
) -> MapRuntimeSnapshotUpdatedEvent {
    MapRuntimeSnapshotUpdatedEvent {
        checkpoint_id: checkpoint_id.to_string(),
        reason: "test".to_string(),
        snapshot_sha256: snapshot_sha256(&snapshot).unwrap(),
        snapshot,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        end_turn: None,
        phase: None,
    }
}

fn inter_agent_assistant_message(text: &str) -> ResponseItem {
    let communication = InterAgentCommunication::new(
        AgentPath::root(),
        AgentPath::root().join("worker").unwrap(),
        Vec::new(),
        text.to_string(),
        /*trigger_turn*/ true,
    );
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: serde_json::to_string(&communication).unwrap(),
        }],
        end_turn: None,
        phase: None,
    }
}

#[tokio::test]
async fn reconstruct_history_restores_latest_map_runtime_mode() {
    let (session, turn_context) = make_session_and_context().await;
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Standard,
                current_mode: MapRuntimeMode::Experiment,
            },
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Experiment,
                current_mode: MapRuntimeMode::Standard,
            },
        ))),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.map_runtime_mode, MapRuntimeMode::Standard);
}

#[tokio::test]
async fn reconstruct_history_replays_taskspace_compaction_checkpoint_from_raw_events() {
    let (session, turn_context) = make_session_and_context().await;
    let mut store = TaskSpaceEventStore::new();
    let original = user_message("raw task body");
    let initial_event = store.record_item(&original, None, None, 1).unwrap();
    let summary = assistant_message("checkpoint summary");
    let checkpoint_event = store
        .install_compaction_checkpoint(vec![summary.clone()], 2)
        .unwrap();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Standard,
                current_mode: MapRuntimeMode::Experiment,
            },
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(
            MapRuntimeEvent::TaskContextOwnershipChanged(
                MapRuntimeTaskContextOwnershipChangedEvent {
                    active: true,
                    events: vec![initial_event.to_protocol()],
                },
            ),
        )),
        RolloutItem::EventMsg(EventMsg::MapRuntime(
            MapRuntimeEvent::TaskContextEventRecorded(checkpoint_event.to_protocol()),
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert!(reconstructed.history.is_empty());
    assert_eq!(reconstructed.taskspace_events.len(), 2);
    let restored = TaskSpaceEventStore::restore(reconstructed.taskspace_events).unwrap();
    let visible = restored.linearize();
    assert_eq!(visible.len(), 2);
    assert!(
        serde_json::to_string(&visible[0])
            .unwrap()
            .contains("covered_sequence_range: 1-1")
    );
    assert_eq!(visible[1], summary);
    assert!(
        !serde_json::to_string(&visible)
            .unwrap()
            .contains("raw task body")
    );
}

#[tokio::test]
async fn reconstruct_history_restores_latest_map_runtime_snapshot() {
    let (session, turn_context) = make_session_and_context().await;
    let mut runtime = ActionMapRuntimeState::default();
    runtime.set_mode_for_session(MapRuntimeMode::Experiment, session.conversation_id);
    runtime
        .initialize_map_for_main(
            session.conversation_id,
            crate::action_map::ActionMapInitializeInput {
                root: crate::action_map::ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "Architecture review".into(),
                },
                finish: crate::action_map::ActionMapInitializeNodeInput {
                    id: "finish".into(),
                    goal: "Finish architecture review".into(),
                },
                source_event_ids: vec!["task-event-1".to_string()],
                work_nodes: vec![crate::action_map::ActionMapInitializeNodeInput {
                    id: "scope".to_string(),
                    goal: "Collect architecture scope.".to_string(),
                }],
                edges: vec![
                    crate::action_map::ActionMapEdgeInput {
                        from: "root".into(),
                        to: "scope".into(),
                    },
                    crate::action_map::ActionMapEdgeInput {
                        from: "scope".into(),
                        to: "finish".into(),
                    },
                ],
                current_node_id: "scope".to_string(),
            },
        )
        .expect("map initializes");
    let snapshot = runtime.snapshot();
    let rollout_items = vec![RolloutItem::EventMsg(EventMsg::MapRuntime(
        MapRuntimeEvent::SnapshotUpdated(map_checkpoint_event(
            "checkpoint-latest",
            snapshot.clone(),
        )),
    ))];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.map_runtime_snapshot, Some(snapshot));
}

#[tokio::test]
async fn reconstruct_history_applies_chained_snapshot_deltas_to_checkpoint() {
    let (session, turn_context) = make_session_and_context().await;
    let base = ActionMapRuntimeState::default().snapshot();
    let mut middle = base.clone();
    middle.routing_required = !base.routing_required;
    let mut expected = middle.clone();
    expected.reborn_requested = true;
    let base_hash = snapshot_sha256(&base).unwrap();
    let mut checkpoint = ActionMapCheckpointState::default();
    checkpoint.install("checkpoint-base".to_string(), base_hash, base.clone());
    let first_delta = build_snapshot_delta(&mut checkpoint, &middle)
        .unwrap()
        .unwrap();
    let second_delta = build_snapshot_delta(&mut checkpoint, &expected)
        .unwrap()
        .unwrap();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
            map_checkpoint_event("checkpoint-base", base),
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(
            first_delta,
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(
            second_delta,
        ))),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.map_runtime_snapshot, Some(expected));
    assert_eq!(reconstructed.map_runtime_checkpoint.delta_sequence, 2);
}

#[tokio::test]
async fn resumed_compacted_map_rebuilds_one_projection_from_checkpoint_and_deltas() {
    let (session, turn_context) = make_session_and_context().await;
    let mut runtime = ActionMapRuntimeState::default();
    runtime.set_mode_for_session(MapRuntimeMode::Experiment, session.conversation_id);
    runtime
        .initialize_map_for_main(
            session.conversation_id,
            crate::action_map::ActionMapInitializeInput {
                root: crate::action_map::ActionMapInitializeNodeInput {
                    id: "root".into(),
                    goal: "Resume canonical map".into(),
                },
                finish: crate::action_map::ActionMapInitializeNodeInput {
                    id: "finish".into(),
                    goal: "Finish resumed map".into(),
                },
                source_event_ids: vec!["task-event-root".to_string()],
                work_nodes: vec![crate::action_map::ActionMapInitializeNodeInput {
                    id: "resume-node".to_string(),
                    goal: "Restore this goal from the canonical map.".to_string(),
                }],
                edges: vec![
                    crate::action_map::ActionMapEdgeInput {
                        from: "root".into(),
                        to: "resume-node".into(),
                    },
                    crate::action_map::ActionMapEdgeInput {
                        from: "resume-node".into(),
                        to: "finish".into(),
                    },
                ],
                current_node_id: "resume-node".to_string(),
            },
        )
        .expect("map initializes");
    let base = runtime.snapshot();
    let mut middle = base.clone();
    middle.routing_required = !base.routing_required;
    let mut expected = middle.clone();
    expected.reborn_requested = true;
    let base_hash = snapshot_sha256(&base).unwrap();
    let mut checkpoint = ActionMapCheckpointState::default();
    checkpoint.install("checkpoint-resume".to_string(), base_hash, base.clone());
    let first_delta = build_snapshot_delta(&mut checkpoint, &middle)
        .unwrap()
        .unwrap();
    let second_delta = build_snapshot_delta(&mut checkpoint, &expected)
        .unwrap()
        .unwrap();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
            MapRuntimeModeChangedEvent {
                previous_mode: MapRuntimeMode::Standard,
                current_mode: MapRuntimeMode::Experiment,
            },
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
            map_checkpoint_event("checkpoint-resume", base),
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(
            first_delta,
        ))),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(
            second_delta,
        ))),
        RolloutItem::Compacted(CompactedItem {
            message: "compacted".to_string(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume-map.jsonl")),
        }))
        .await
        .expect("restore initial history");
    let restored = session.action_map_snapshot().await;
    assert_eq!(restored.mode, MapRuntimeMode::Experiment);
    assert!(restored.reborn_requested);
    let map = restored.map.as_ref().expect("restored rooted map");
    assert_eq!(map.id, "map-1");
    assert_eq!(map.root_node_id, "root");
    assert_eq!(map.finish_node_id, "finish");
    assert_eq!(map.nodes.len(), 3);
    assert_eq!(
        map.nodes
            .iter()
            .find(|node| node.id == "resume-node")
            .unwrap()
            .goal,
        "Restore this goal from the canonical map."
    );

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let first_history = serde_json::to_string(session.clone_history().await.raw_items()).unwrap();
    assert_eq!(
        first_history
            .matches("ContextProjectionV1 epoch snapshot:")
            .count(),
        1
    );
    assert!(first_history.contains("task_id: task-1"));
    assert!(first_history.contains("map_id: map-1"));
    assert!(first_history.contains("resume-node"));
    assert!(first_history.contains("Restore this goal from the canonical map."));

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let second_history = serde_json::to_string(session.clone_history().await.raw_items()).unwrap();
    assert_eq!(second_history, first_history);
}

#[tokio::test]
async fn resumed_history_rejects_snapshot_delta_without_checkpoint() {
    let (session, _turn_context) = make_session_and_context().await;
    let state_before = session.action_map_snapshot().await;
    let history_before = session.clone_history().await.raw_items().to_vec();
    let base = ActionMapRuntimeState::default().snapshot();
    let mut expected = base.clone();
    expected.routing_required = !base.routing_required;
    let base_hash = snapshot_sha256(&base).unwrap();
    let mut checkpoint = ActionMapCheckpointState::default();
    checkpoint.install("missing-checkpoint".to_string(), base_hash, base);
    let delta = build_snapshot_delta(&mut checkpoint, &expected)
        .unwrap()
        .unwrap();

    let error = session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: vec![RolloutItem::EventMsg(EventMsg::MapRuntime(
                MapRuntimeEvent::SnapshotDelta(delta),
            ))],
            rollout_path: Some(PathBuf::from("/tmp/resume-missing-checkpoint.jsonl")),
        }))
        .await
        .expect_err("delta without checkpoint must fail");

    assert_eq!(error.phase, "map_checkpoint_delta_chain");
    assert!(error.message.contains("has no surviving checkpoint"));
    assert_eq!(session.action_map_snapshot().await, state_before);
    assert_eq!(session.clone_history().await.raw_items(), history_before);
}

#[tokio::test]
async fn resumed_history_rejects_missing_middle_snapshot_delta() {
    let (session, _turn_context) = make_session_and_context().await;
    let state_before = session.action_map_snapshot().await;
    let history_before = session.clone_history().await.raw_items().to_vec();
    let base = ActionMapRuntimeState::default().snapshot();
    let mut middle = base.clone();
    middle.routing_required = !base.routing_required;
    let mut expected = middle.clone();
    expected.reborn_requested = true;
    let base_hash = snapshot_sha256(&base).unwrap();
    let mut checkpoint = ActionMapCheckpointState::default();
    checkpoint.install("checkpoint-gap".to_string(), base_hash, base.clone());
    let _first_delta = build_snapshot_delta(&mut checkpoint, &middle)
        .unwrap()
        .unwrap();
    let second_delta = build_snapshot_delta(&mut checkpoint, &expected)
        .unwrap()
        .unwrap();

    let error = session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: vec![
                RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
                    map_checkpoint_event("checkpoint-gap", base),
                ))),
                RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(
                    second_delta,
                ))),
            ],
            rollout_path: Some(PathBuf::from("/tmp/resume-missing-delta.jsonl")),
        }))
        .await
        .expect_err("missing middle delta must fail");

    assert_eq!(error.phase, "map_checkpoint_delta_chain");
    assert!(
        error
            .message
            .contains("snapshot delta previous hash mismatch")
    );
    assert_eq!(session.action_map_snapshot().await, state_before);
    assert_eq!(session.clone_history().await.raw_items(), history_before);
}

#[tokio::test]
async fn resumed_history_rejects_corrupt_checkpoint_hash_without_partial_restore() {
    let (session, _turn_context) = make_session_and_context().await;
    let state_before = session.action_map_snapshot().await;
    let history_before = session.clone_history().await.raw_items().to_vec();
    let mut checkpoint = map_checkpoint_event("corrupt-checkpoint", state_before.clone());
    checkpoint.snapshot_sha256 = "0".repeat(64);

    let error = session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: vec![RolloutItem::EventMsg(EventMsg::MapRuntime(
                MapRuntimeEvent::SnapshotUpdated(checkpoint),
            ))],
            rollout_path: Some(PathBuf::from("/tmp/resume-corrupt-checkpoint.jsonl")),
        }))
        .await
        .expect_err("corrupt checkpoint hash must fail");

    assert_eq!(error.phase, "map_checkpoint_delta_chain");
    assert!(error.message.contains("map checkpoint hash mismatch"));
    assert_eq!(session.action_map_snapshot().await, state_before);
    assert_eq!(session.clone_history().await.raw_items(), history_before);
}

#[tokio::test]
async fn resumed_history_rejects_invalid_taskspace_ownership_sequence() {
    let (session, _turn_context) = make_session_and_context().await;
    let state_before = session.action_map_snapshot().await;
    let history_before = session.clone_history().await.raw_items().to_vec();
    let mut store = TaskSpaceEventStore::new();
    let mut event = store
        .record_item(&user_message("owned task"), None, None, 1)
        .expect("record canonical event")
        .to_protocol();
    event.sequence = 2;

    let error = session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: vec![RolloutItem::EventMsg(EventMsg::MapRuntime(
                MapRuntimeEvent::TaskContextOwnershipChanged(
                    MapRuntimeTaskContextOwnershipChangedEvent {
                        active: true,
                        events: vec![event],
                    },
                ),
            ))],
            rollout_path: Some(PathBuf::from("/tmp/resume-invalid-ownership.jsonl")),
        }))
        .await
        .expect_err("invalid ownership sequence must fail");

    assert_eq!(error.phase, "taskspace_ownership_checkpoint");
    assert!(error.message.contains("sequence"));
    assert_eq!(session.action_map_snapshot().await, state_before);
    assert_eq!(session.clone_history().await.raw_items(), history_before);
}

#[tokio::test]
async fn resumed_history_rejects_invalid_taskspace_event_sequence() {
    let (session, _turn_context) = make_session_and_context().await;
    let state_before = session.action_map_snapshot().await;
    let history_before = session.clone_history().await.raw_items().to_vec();
    let mut store = TaskSpaceEventStore::new();
    let first = store
        .record_item(&user_message("first"), None, None, 1)
        .expect("record first event");
    let mut second = store
        .record_item(&assistant_message("second"), None, None, 2)
        .expect("record second event")
        .to_protocol();
    second.sequence = 3;

    let error = session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: vec![
                RolloutItem::EventMsg(EventMsg::MapRuntime(
                    MapRuntimeEvent::TaskContextOwnershipChanged(
                        MapRuntimeTaskContextOwnershipChangedEvent {
                            active: true,
                            events: vec![first.to_protocol()],
                        },
                    ),
                )),
                RolloutItem::EventMsg(EventMsg::MapRuntime(
                    MapRuntimeEvent::TaskContextEventRecorded(second),
                )),
            ],
            rollout_path: Some(PathBuf::from("/tmp/resume-invalid-event.jsonl")),
        }))
        .await
        .expect_err("invalid event sequence must fail");

    assert_eq!(error.phase, "taskspace_event_sequence");
    assert!(error.message.contains("sequence"));
    assert_eq!(session.action_map_snapshot().await, state_before);
    assert_eq!(session.clone_history().await.raw_items(), history_before);
}

#[tokio::test]
async fn reconstruct_history_rollback_restores_latest_surviving_map_snapshot() {
    let (session, turn_context) = make_session_and_context().await;
    let mut first_snapshot = ActionMapRuntimeState::default().snapshot();
    first_snapshot.routing_required = false;
    let mut rolled_back_snapshot = first_snapshot.clone();
    rolled_back_snapshot.routing_required = true;
    let first_turn_id = "turn-1".to_string();
    let rolled_back_turn_id = "turn-2".to_string();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "first".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
            map_checkpoint_event("checkpoint-first", first_snapshot.clone()),
        ))),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "rolled back".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
            map_checkpoint_event("checkpoint-rolled-back", rolled_back_snapshot),
        ))),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: rolled_back_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.map_runtime_snapshot, Some(first_snapshot));
}

#[tokio::test]
async fn record_initial_history_resumed_bare_turn_context_does_not_hydrate_previous_turn_settings()
{
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let rollout_items = vec![RolloutItem::TurnContext(previous_context_item)];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_hydrates_previous_turn_settings_from_lifecycle_turn_with_missing_turn_context_id()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let mut previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    previous_context_item.turn_id = None;

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_keeps_history_and_metadata_in_sync_for_completed_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let mut rolled_back_context_item = first_context_item.clone();
    rolled_back_context_item.turn_id = Some("rolled-back-turn".to_string());
    rolled_back_context_item.model = "rolled-back-model".to_string();
    let rolled_back_turn_id = rolled_back_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(rolled_back_context_item),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: rolled_back_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_keeps_history_and_metadata_in_sync_for_incomplete_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "incomplete-rolled-back-turn".to_string();
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_skips_non_user_turns_for_history_and_metadata() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let second_turn_id = "rolled-back-user-turn".to_string();
    let standalone_turn_id = "standalone-turn".to_string();
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");
    let standalone_assistant = assistant_message("standalone assistant");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: second_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: second_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: standalone_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::ResponseItem(standalone_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: standalone_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_counts_inter_agent_assistant_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let assistant_turn_id = "assistant-instruction-turn".to_string();
    let assistant_turn_context = TurnContextItem {
        turn_id: Some(assistant_turn_id.clone()),
        ..first_context_item.clone()
    };
    let assistant_instruction = inter_agent_assistant_message("continue");
    let assistant_reply = assistant_message("worker reply");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 1 user")),
        RolloutItem::ResponseItem(assistant_message("turn 1 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: assistant_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::TurnContext(assistant_turn_context),
        RolloutItem::ResponseItem(assistant_instruction),
        RolloutItem::ResponseItem(assistant_reply),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: assistant_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![
            user_message("turn 1 user"),
            assistant_message("turn 1 assistant")
        ]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_clears_history_and_metadata_when_exceeding_user_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let only_context_item = turn_context.to_turn_context_item();
    let only_turn_id = only_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: only_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "only user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(only_context_item),
        RolloutItem::ResponseItem(user_message("only user")),
        RolloutItem::ResponseItem(assistant_message("only assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: only_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 99 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.history, Vec::new());
    assert_eq!(reconstructed.previous_turn_settings, None);
    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_rollback_skips_only_user_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let user_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let standalone_turn_id = "standalone-task-turn".to_string();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: user_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: user_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        // Standalone task turn (no UserMessage) should not consume rollback skips.
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: standalone_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: standalone_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_rollback_drops_incomplete_user_turn_compaction_metadata() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "incomplete-compacted-user-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "rolled back".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(previous_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_bare_turn_context_does_not_seed_reference_context_item() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let rollout_items = vec![RolloutItem::TurnContext(previous_context_item.clone())];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_does_not_seed_reference_context_item_after_compaction() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let rollout_items = vec![
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn reconstruct_history_legacy_compaction_without_replacement_history_does_not_inject_current_initial_context()
 {
    let (session, turn_context) = make_session_and_context().await;
    let rollout_items = vec![
        RolloutItem::ResponseItem(user_message("before compact")),
        RolloutItem::ResponseItem(assistant_message("assistant reply")),
        RolloutItem::Compacted(CompactedItem {
            message: "legacy summary".to_string(),
            replacement_history: None,
        }),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![
            user_message("before compact"),
            user_message("legacy summary"),
        ]
    );
    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn reconstruct_history_legacy_compaction_without_replacement_history_clears_later_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let current_context_item = turn_context.to_turn_context_item();
    let current_turn_id = current_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::ResponseItem(user_message("before compact")),
        RolloutItem::Compacted(CompactedItem {
            message: "legacy summary".to_string(),
            replacement_history: None,
        }),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "after legacy compact".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(current_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: current_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_turn_context_after_compaction_reestablishes_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        // Compaction clears baseline until a later TurnContextItem re-establishes it.
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(TurnContextItem {
            turn_id: Some(turn_context.sub_id.clone()),
            trace_id: turn_context.trace_id.clone(),
            cwd: turn_context.cwd.to_path_buf(),
            current_date: turn_context.current_date.clone(),
            timezone: turn_context.timezone.clone(),
            approval_policy: turn_context.approval_policy.value(),
            sandbox_policy: turn_context.sandbox_policy.get().clone(),
            permission_profile: None,
            network: None,
            file_system_sandbox_policy: None,
            model: previous_model.to_string(),
            personality: turn_context.personality,
            collaboration_mode: Some(turn_context.collaboration_mode.clone()),
            realtime_active: Some(turn_context.realtime_active),
            effort: turn_context.reasoning_effort,
            summary: turn_context.reasoning_summary,
            user_instructions: None,
            developer_instructions: None,
            final_output_json_schema: None,
            truncation_policy: Some(turn_context.truncation_policy),
        }))
        .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_aborted_turn_without_id_clears_active_turn_for_compaction_accounting()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let aborted_turn_id = "aborted-turn-without-id".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: aborted_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "aborted".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnAborted(
            codex_protocol::protocol::TurnAbortedEvent {
                turn_id: None,
                reason: TurnAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_unmatched_abort_preserves_active_turn_for_later_turn_context()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let current_model = "current-rollout-model";
    let current_turn_id = "current-turn".to_string();
    let unmatched_abort_turn_id = "other-turn".to_string();
    let current_context_item = TurnContextItem {
        turn_id: Some(current_turn_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: current_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "current".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnAborted(
            codex_protocol::protocol::TurnAbortedEvent {
                turn_id: Some(unmatched_abort_turn_id),
                reason: TurnAbortReason::Interrupted,
                completed_at: None,
                duration_ms: None,
            },
        )),
        RolloutItem::TurnContext(current_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: current_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: current_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(current_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_trailing_incomplete_turn_compaction_clears_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "trailing-incomplete-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "incomplete".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_trailing_incomplete_turn_preserves_turn_context_item() {
    let (session, turn_context) = make_session_and_context().await;
    let current_context_item = turn_context.to_turn_context_item();
    let current_turn_id = current_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "incomplete".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(current_context_item.clone()),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(current_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_replaced_incomplete_compacted_turn_clears_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let compacted_incomplete_turn_id = "compacted-incomplete-turn".to_string();
    let replacing_turn_id = "replacing-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: compacted_incomplete_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "compacted".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        // A newer TurnStarted replaces the incomplete compacted turn without a matching
        // completion/abort for the old one.
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: replacing_turn_id,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await
        .expect("restore initial history");

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}
