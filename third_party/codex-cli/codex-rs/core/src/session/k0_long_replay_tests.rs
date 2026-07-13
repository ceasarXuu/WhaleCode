use super::*;

use super::tests::make_session_and_context;
use crate::action_map::ActionMapCheckpointState;
use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::NodeKind;
use crate::action_map::build_snapshot_delta;
use crate::action_map::snapshot_sha256;
use codex_protocol::ThreadId;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeSnapshotUpdatedEvent;
use codex_protocol::protocol::ResumedHistory;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

const NODE_COUNT: usize = 1_000;
const RESUME_CYCLES: usize = 5;

#[derive(Serialize)]
struct K0LongReplayProbe {
    schema_version: &'static str,
    fixture_kind: &'static str,
    node_count: usize,
    edge_count: usize,
    resume_cycles: usize,
    compaction_boundaries: usize,
    code_revision_count: usize,
    checkpoint_bytes: usize,
    delta_bytes: usize,
    resume_duration_us: u128,
    projection_duration_us: u128,
    exact_replay_count: usize,
    single_projection_outcome_count: usize,
    skeleton_over_budget_count: usize,
    final_snapshot_sha256: String,
}

fn initialize_long_snapshot(session_id: ThreadId) -> ActionMapSnapshot {
    let mut runtime = ActionMapRuntimeState::default();
    runtime.set_mode_for_session(MapRuntimeMode::Experiment, session_id);
    let nodes = (0..NODE_COUNT)
        .map(|index| ActionMapInitializeNodeInput {
            id: format!("node-{index}"),
            kind: match index % 4 {
                0 => NodeKind::InspectCodeContext,
                1 => NodeKind::ImplementSolution,
                2 => NodeKind::SmokeTest,
                _ => NodeKind::RegressionTest,
            },
            title: format!("K0 session-native node {index}"),
            context_summary: format!("Execute session-native replay work item {index}."),
            dependency_node_ids: index
                .checked_sub(1)
                .map(|previous| vec![format!("node-{previous}")])
                .unwrap_or_default(),
        })
        .collect();
    runtime
        .initialize_map_for_main(
            session_id,
            ActionMapInitializeInput {
                task_title: "K0 session-native long replay".into(),
                source_event_ids: vec!["task-event-root".into()],
                nodes,
                current_node_id: "node-0".into(),
            },
        )
        .expect("K0 long map initializes");
    runtime.snapshot()
}

fn checkpoint_event(
    checkpoint_id: &str,
    snapshot: ActionMapSnapshot,
) -> MapRuntimeSnapshotUpdatedEvent {
    MapRuntimeSnapshotUpdatedEvent {
        checkpoint_id: checkpoint_id.to_string(),
        reason: "k0_long_replay".into(),
        snapshot_sha256: snapshot_sha256(&snapshot).unwrap(),
        snapshot,
    }
}

#[tokio::test]
async fn writes_k0_session_native_long_replay_probe() {
    let Some(output_path) = std::env::var_os("TASKSPACE_K0_LONG_REPLAY_OUTPUT").map(PathBuf::from)
    else {
        return;
    };
    let (seed_session, _) = make_session_and_context().await;
    let mut current = initialize_long_snapshot(seed_session.conversation_id);
    let mut checkpoint_bytes = 0usize;
    let mut delta_bytes = 0usize;
    let mut resume_duration_us = 0u128;
    let mut projection_duration_us = 0u128;
    let mut exact_replay_count = 0usize;
    let mut single_projection_outcome_count = 0usize;
    let mut skeleton_over_budget_count = 0usize;

    for cycle in 1..=RESUME_CYCLES {
        let base = current.clone();
        let mut expected = base.clone();
        let map = expected.maps.first_mut().unwrap();
        map.base_map_version = format!("k0-code-revision-{cycle}");
        map.nodes[cycle].source_refs = vec![format!("src/revision_{cycle}.rs")];
        let checkpoint_id = format!("k0-session-checkpoint-{cycle}");
        let checkpoint_hash = snapshot_sha256(&base).unwrap();
        let mut checkpoint = ActionMapCheckpointState::default();
        checkpoint.install(checkpoint_id.clone(), checkpoint_hash, base.clone());
        let delta = build_snapshot_delta(&mut checkpoint, &expected)
            .unwrap()
            .expect("each code revision changes the snapshot");
        checkpoint_bytes += serde_json::to_vec(&base).unwrap().len();
        delta_bytes += serde_json::to_vec(&delta).unwrap().len();

        let rollout_items = vec![
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(
                MapRuntimeModeChangedEvent {
                    previous_mode: MapRuntimeMode::Standard,
                    current_mode: MapRuntimeMode::Experiment,
                },
            ))),
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
                checkpoint_event(&checkpoint_id, base),
            ))),
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(delta))),
            RolloutItem::Compacted(CompactedItem {
                message: format!("K0 compaction boundary {cycle}"),
                replacement_history: Some(Vec::new()),
            }),
        ];
        let (session, turn_context) = make_session_and_context().await;
        let reconstructed = session
            .reconstruct_history_from_rollout(&turn_context, &rollout_items)
            .await;
        if reconstructed.map_runtime_snapshot.as_ref() == Some(&expected) {
            exact_replay_count += 1;
        }
        let resume_started = Instant::now();
        session
            .record_initial_history(InitialHistory::Resumed(ResumedHistory {
                conversation_id: ThreadId::default(),
                history: rollout_items,
                rollout_path: Some(PathBuf::from(format!(
                    "/tmp/k0-session-replay-{cycle}.jsonl"
                ))),
            }))
            .await;
        resume_duration_us += resume_started.elapsed().as_micros();
        let restored = session.action_map_snapshot().await;
        assert_eq!(
            restored.maps[0].base_map_version,
            expected.maps[0].base_map_version
        );
        assert_eq!(
            restored.maps[0].nodes[cycle].source_refs,
            expected.maps[0].nodes[cycle].source_refs
        );

        let projection_started = Instant::now();
        session
            .record_context_updates_and_set_reference_context_item(&turn_context)
            .await;
        projection_duration_us += projection_started.elapsed().as_micros();
        let history = serde_json::to_string(session.clone_history().await.raw_items()).unwrap();
        let projection_count = history
            .matches("ContextProjectionV1 epoch snapshot:")
            .count();
        let over_budget_count = history.matches("TaskSpaceMapProjectionErrorV1:").count();
        if projection_count + over_budget_count == 1 {
            single_projection_outcome_count += 1;
        }
        skeleton_over_budget_count += over_budget_count;
        current = expected;
    }

    assert_eq!(exact_replay_count, RESUME_CYCLES);
    assert_eq!(single_projection_outcome_count, RESUME_CYCLES);
    assert_eq!(skeleton_over_budget_count, RESUME_CYCLES);
    let probe = K0LongReplayProbe {
        schema_version: "taskspace-map-budget-k0-long-replay-v1",
        fixture_kind: "session_native_resume_compaction_code_change",
        node_count: NODE_COUNT,
        edge_count: NODE_COUNT - 1,
        resume_cycles: RESUME_CYCLES,
        compaction_boundaries: RESUME_CYCLES,
        code_revision_count: RESUME_CYCLES,
        checkpoint_bytes,
        delta_bytes,
        resume_duration_us,
        projection_duration_us,
        exact_replay_count,
        single_projection_outcome_count,
        skeleton_over_budget_count,
        final_snapshot_sha256: snapshot_sha256(&current).unwrap(),
    };
    std::fs::write(output_path, serde_json::to_vec_pretty(&probe).unwrap()).unwrap();
}
