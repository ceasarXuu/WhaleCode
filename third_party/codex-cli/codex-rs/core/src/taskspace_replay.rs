use crate::action_map::ActionMapCheckpointState;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::apply_snapshot_delta_typed;
use crate::action_map::snapshot_sha256;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeSnapshotDeltaEvent;
use codex_protocol::protocol::MapRuntimeSnapshotUpdatedEvent;
use codex_protocol::protocol::MapRuntimeTerminalCommittedEvent;
use codex_protocol::protocol::RolloutItem;
use codex_rollout::RolloutRecorder;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSpaceReplayErrorCode {
    Load,
    Parse,
    NotApplicable,
    MissingCheckpoint,
    SequenceGapOrOrder,
    BaseIdOrHash,
    PreviousHash,
    ResultHash,
    InvalidPatch,
    IncompleteTransaction,
    UnsupportedSnapshotSchema,
    DomainInvariant,
    NoncanonicalSnapshot,
}

impl TaskSpaceReplayErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Parse => "parse",
            Self::NotApplicable => "not_applicable",
            Self::MissingCheckpoint => "missing_checkpoint",
            Self::SequenceGapOrOrder => "sequence_gap_or_order",
            Self::BaseIdOrHash => "base_id_or_hash",
            Self::PreviousHash => "previous_hash",
            Self::ResultHash => "result_hash",
            Self::InvalidPatch => "invalid_patch",
            Self::IncompleteTransaction => "incomplete_transaction",
            Self::UnsupportedSnapshotSchema => "unsupported_snapshot_schema",
            Self::DomainInvariant => "domain_invariant",
            Self::NoncanonicalSnapshot => "noncanonical_snapshot",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSpaceReplayError {
    pub code: TaskSpaceReplayErrorCode,
    pub message: String,
}

impl TaskSpaceReplayError {
    fn new(code: TaskSpaceReplayErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for TaskSpaceReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for TaskSpaceReplayError {}

#[derive(Debug, Clone)]
pub struct LoadedRollout {
    pub rollout_sha256: String,
    pub parse_error_count: usize,
    pub items: Vec<RolloutItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayedActionMapState {
    pub rollout_sha256: String,
    pub parse_error_count: usize,
    pub snapshot: ActionMapSnapshot,
    pub checkpoint_id: String,
    pub base_snapshot_sha256: String,
    pub final_snapshot_sha256: String,
    pub parsed_checkpoint_count: usize,
    pub parsed_delta_count: usize,
    pub surviving_checkpoint_count: usize,
    pub surviving_delta_count: usize,
    pub active_checkpoint_id: String,
    pub active_chain_applied_delta_count: usize,
    pub active_chain_last_delta_sequence: u64,
    pub surviving_map_runtime_mode: MapRuntimeMode,
}

#[derive(Debug)]
pub(crate) struct ReplayedActionMapRestore {
    pub(crate) state: ReplayedActionMapState,
    pub(crate) checkpoint: ActionMapCheckpointState,
}

#[derive(Debug, Clone)]
enum ReplayItem {
    Mode(MapRuntimeMode),
    Checkpoint(MapRuntimeSnapshotUpdatedEvent),
    TerminalCheckpoint(Box<MapRuntimeTerminalCommittedEvent>),
    IncompleteTerminalCommit,
    Delta(MapRuntimeSnapshotDeltaEvent),
}

#[derive(Debug, Default)]
struct Segment {
    counts_as_user_turn: bool,
    items: Vec<ReplayItem>,
}

pub async fn load_rollout(path: &Path) -> Result<LoadedRollout, TaskSpaceReplayError> {
    let loaded = RolloutRecorder::load_rollout_source(path)
        .await
        .map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::InvalidData {
                TaskSpaceReplayErrorCode::Parse
            } else {
                TaskSpaceReplayErrorCode::Load
            };
            TaskSpaceReplayError::new(code, error.to_string())
        })?;
    Ok(LoadedRollout {
        rollout_sha256: format!("{:x}", Sha256::digest(&loaded.raw_bytes)),
        parse_error_count: loaded.parse_errors,
        items: loaded.items,
    })
}

pub fn replay_loaded_rollout(
    loaded: &LoadedRollout,
) -> Result<ReplayedActionMapState, TaskSpaceReplayError> {
    replay_loaded_rollout_for_restore(loaded).map(|restore| restore.state)
}

pub(crate) fn replay_loaded_rollout_for_restore(
    loaded: &LoadedRollout,
) -> Result<ReplayedActionMapRestore, TaskSpaceReplayError> {
    if loaded.parse_error_count > 0 {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::Parse,
            format!("rollout contains {} parse errors", loaded.parse_error_count),
        ));
    }
    replay_rollout_items(
        loaded.rollout_sha256.clone(),
        loaded.parse_error_count,
        &loaded.items,
    )
}

pub(crate) fn replay_rollout_items(
    rollout_sha256: String,
    parse_error_count: usize,
    items: &[RolloutItem],
) -> Result<ReplayedActionMapRestore, TaskSpaceReplayError> {
    let parsed_checkpoint_count = items.iter().filter(is_checkpoint).count();
    let parsed_delta_count = items.iter().filter(is_delta).count();
    if parsed_checkpoint_count == 0 && parsed_delta_count == 0 {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::NotApplicable,
            "rollout does not contain TaskSpace snapshot events",
        ));
    }

    let surviving_segments = surviving_segments_newest_first(items);
    let surviving_map_runtime_mode = mode_from_segments(&surviving_segments);
    let surviving_checkpoint_count = surviving_segments
        .iter()
        .flat_map(|segment| segment.iter())
        .filter(|item| {
            matches!(
                item,
                ReplayItem::Checkpoint(_) | ReplayItem::TerminalCheckpoint(_)
            )
        })
        .count();
    let surviving_delta_count = surviving_segments
        .iter()
        .flat_map(|segment| segment.iter())
        .filter(|item| matches!(item, ReplayItem::Delta(_)))
        .count();
    let (snapshot, checkpoint, active_delta_count, last_sequence) =
        replay_segments(&surviving_segments)?;
    validate_snapshot(&snapshot)?;
    let checkpoint_id = checkpoint.checkpoint_id.clone().ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::MissingCheckpoint,
            "missing checkpoint id",
        )
    })?;
    let base_snapshot_sha256 = checkpoint.snapshot_sha256.clone().ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::MissingCheckpoint,
            "missing checkpoint hash",
        )
    })?;
    let final_snapshot_sha256 = snapshot_sha256(&snapshot).map_err(|error| {
        TaskSpaceReplayError::new(TaskSpaceReplayErrorCode::ResultHash, error.to_string())
    })?;
    Ok(ReplayedActionMapRestore {
        state: ReplayedActionMapState {
            rollout_sha256,
            parse_error_count,
            snapshot,
            checkpoint_id: checkpoint_id.clone(),
            base_snapshot_sha256,
            final_snapshot_sha256,
            parsed_checkpoint_count,
            parsed_delta_count,
            surviving_checkpoint_count,
            surviving_delta_count,
            active_checkpoint_id: checkpoint_id,
            active_chain_applied_delta_count: active_delta_count,
            active_chain_last_delta_sequence: last_sequence,
            surviving_map_runtime_mode,
        },
        checkpoint,
    })
}

pub(crate) fn replay_surviving_mode(items: &[RolloutItem]) -> MapRuntimeMode {
    mode_from_segments(&surviving_segments_newest_first(items))
}

fn surviving_segments_newest_first(items: &[RolloutItem]) -> Vec<Vec<ReplayItem>> {
    let mut pending_rollback_turns = 0usize;
    let mut segments = Vec::new();
    let mut active = Segment::default();
    for item in items.iter().rev() {
        match item {
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(event)) => {
                pending_rollback_turns = pending_rollback_turns
                    .saturating_add(usize::try_from(event.num_turns).unwrap_or(usize::MAX));
            }
            RolloutItem::Compacted(_) => {
                finalize_segment(&mut active, &mut pending_rollback_turns, &mut segments);
            }
            RolloutItem::EventMsg(EventMsg::TurnStarted(_)) => {
                finalize_segment(&mut active, &mut pending_rollback_turns, &mut segments);
            }
            RolloutItem::EventMsg(EventMsg::UserMessage(_)) => active.counts_as_user_turn = true,
            RolloutItem::ResponseItem(item) => {
                active.counts_as_user_turn |= crate::context_manager::is_user_turn_boundary(item);
            }
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotUpdated(
                event,
            ))) => {
                active.items.push(ReplayItem::Checkpoint(event.clone()));
            }
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(event))) => {
                active.items.push(ReplayItem::Delta(event.clone()));
            }
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::TerminalCommitted(
                event,
            ))) => {
                active
                    .items
                    .push(ReplayItem::TerminalCheckpoint(event.clone()));
            }
            RolloutItem::EventMsg(EventMsg::MapRuntime(
                MapRuntimeEvent::GraphRevisionCommitted(event),
            )) if matches!(
                event.operation.as_str(),
                "close_finish_with_no_active_work" | "complete_active_work_then_end"
            ) =>
            {
                active.items.push(ReplayItem::IncompleteTerminalCommit);
            }
            RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::ModeChanged(event))) => {
                active.items.push(ReplayItem::Mode(event.current_mode));
            }
            RolloutItem::EventMsg(_)
            | RolloutItem::SessionMeta(_)
            | RolloutItem::TurnContext(_) => {}
        }
    }
    finalize_segment(&mut active, &mut pending_rollback_turns, &mut segments);
    segments
}

fn mode_from_segments(segments_newest_first: &[Vec<ReplayItem>]) -> MapRuntimeMode {
    segments_newest_first
        .iter()
        .rev()
        .flat_map(|segment| segment.iter().rev())
        .filter_map(|item| match item {
            ReplayItem::Mode(mode) => Some(*mode),
            ReplayItem::Checkpoint(_)
            | ReplayItem::TerminalCheckpoint(_)
            | ReplayItem::IncompleteTerminalCommit
            | ReplayItem::Delta(_) => None,
        })
        .next_back()
        .unwrap_or_default()
}

fn finalize_segment(
    active: &mut Segment,
    pending_rollback_turns: &mut usize,
    segments: &mut Vec<Vec<ReplayItem>>,
) {
    if active.items.is_empty() && !active.counts_as_user_turn {
        return;
    }
    if *pending_rollback_turns > 0 {
        if active.counts_as_user_turn {
            *pending_rollback_turns -= 1;
        }
    } else if !active.items.is_empty() {
        segments.push(std::mem::take(&mut active.items));
    }
    active.counts_as_user_turn = false;
    active.items.clear();
}

fn replay_segments(
    segments_newest_first: &[Vec<ReplayItem>],
) -> Result<(ActionMapSnapshot, ActionMapCheckpointState, usize, u64), TaskSpaceReplayError> {
    let mut checkpoint = ActionMapCheckpointState::default();
    let mut restored_snapshot = None;
    let mut active_delta_count = 0usize;
    for item in segments_newest_first
        .iter()
        .rev()
        .flat_map(|segment| segment.iter().rev())
    {
        match item {
            ReplayItem::Mode(_) => {}
            ReplayItem::Checkpoint(event) => {
                install_checkpoint(event, &mut checkpoint)?;
                restored_snapshot = Some(event.snapshot.clone());
                active_delta_count = 0;
            }
            ReplayItem::TerminalCheckpoint(event) => {
                install_terminal_checkpoint(event, &mut checkpoint)?;
                restored_snapshot = Some(event.snapshot.clone());
                active_delta_count = 0;
            }
            ReplayItem::IncompleteTerminalCommit => {
                return Err(TaskSpaceReplayError::new(
                    TaskSpaceReplayErrorCode::IncompleteTransaction,
                    "terminal graph event is missing its terminal transaction envelope",
                ));
            }
            ReplayItem::Delta(event) => {
                let previous_snapshot = checkpoint.latest_snapshot.clone().ok_or_else(|| {
                    TaskSpaceReplayError::new(
                        TaskSpaceReplayErrorCode::MissingCheckpoint,
                        "delta has no surviving checkpoint",
                    )
                })?;
                if event.sequence != checkpoint.delta_sequence.saturating_add(1) {
                    return Err(TaskSpaceReplayError::new(
                        TaskSpaceReplayErrorCode::SequenceGapOrOrder,
                        format!(
                            "expected delta sequence {}, got {}",
                            checkpoint.delta_sequence + 1,
                            event.sequence
                        ),
                    ));
                }
                let snapshot = apply_delta(event, &previous_snapshot, &checkpoint)?;
                checkpoint.delta_sequence = event.sequence;
                checkpoint.latest_snapshot_sha256 = Some(event.snapshot_sha256.clone());
                checkpoint.latest_snapshot = Some(snapshot.clone());
                restored_snapshot = Some(snapshot);
                active_delta_count = active_delta_count.saturating_add(1);
            }
        }
    }
    let snapshot = restored_snapshot.ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::MissingCheckpoint,
            "missing surviving checkpoint",
        )
    })?;
    Ok((
        snapshot,
        checkpoint.clone(),
        active_delta_count,
        checkpoint.delta_sequence,
    ))
}

fn install_checkpoint(
    event: &MapRuntimeSnapshotUpdatedEvent,
    checkpoint: &mut ActionMapCheckpointState,
) -> Result<(), TaskSpaceReplayError> {
    let actual_hash = snapshot_sha256(&event.snapshot).map_err(|error| {
        TaskSpaceReplayError::new(TaskSpaceReplayErrorCode::ResultHash, error.to_string())
    })?;
    if actual_hash != event.snapshot_sha256 {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::ResultHash,
            format!("checkpoint hash mismatch for {}", event.checkpoint_id),
        ));
    }
    checkpoint.install(
        event.checkpoint_id.clone(),
        event.snapshot_sha256.clone(),
        event.snapshot.clone(),
    );
    Ok(())
}

fn install_terminal_checkpoint(
    event: &MapRuntimeTerminalCommittedEvent,
    checkpoint: &mut ActionMapCheckpointState,
) -> Result<(), TaskSpaceReplayError> {
    let actual_hash = snapshot_sha256(&event.snapshot).map_err(|error| {
        TaskSpaceReplayError::new(TaskSpaceReplayErrorCode::ResultHash, error.to_string())
    })?;
    if actual_hash != event.snapshot_sha256 {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::ResultHash,
            format!(
                "terminal checkpoint hash mismatch for {}",
                event.checkpoint_id
            ),
        ));
    }
    let map = event.snapshot.map.as_ref().ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::IncompleteTransaction,
            "terminal checkpoint has no canonical map",
        )
    })?;
    if !matches!(
        event.graph_revision.operation.as_str(),
        "close_finish_with_no_active_work" | "complete_active_work_then_end"
    ) || event.trace_event.kind != "terminal_committed"
        || event.graph_revision.map_id != event.trace_event.map_id
        || event.graph_revision.map_id != map.id
        || event.graph_revision.revision != map.revision
        || !map.complete
    {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::IncompleteTransaction,
            "terminal checkpoint identity or closed state is inconsistent",
        ));
    }
    checkpoint.install(
        event.checkpoint_id.clone(),
        event.snapshot_sha256.clone(),
        event.snapshot.clone(),
    );
    Ok(())
}

fn apply_delta(
    event: &MapRuntimeSnapshotDeltaEvent,
    previous_snapshot: &ActionMapSnapshot,
    checkpoint: &ActionMapCheckpointState,
) -> Result<ActionMapSnapshot, TaskSpaceReplayError> {
    let checkpoint_id = checkpoint.checkpoint_id.as_deref().ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::MissingCheckpoint,
            "missing checkpoint id",
        )
    })?;
    let checkpoint_hash = checkpoint.snapshot_sha256.as_deref().ok_or_else(|| {
        TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::MissingCheckpoint,
            "missing checkpoint hash",
        )
    })?;
    apply_snapshot_delta_typed(checkpoint_id, checkpoint_hash, previous_snapshot, event).map_err(
        |error| {
            let code = match error.code.as_str() {
                "base_checkpoint_mismatch" | "base_hash_mismatch" => {
                    TaskSpaceReplayErrorCode::BaseIdOrHash
                }
                "previous_hash_mismatch" => TaskSpaceReplayErrorCode::PreviousHash,
                "result_hash_mismatch" => TaskSpaceReplayErrorCode::ResultHash,
                "invalid_patch" => TaskSpaceReplayErrorCode::InvalidPatch,
                _ => TaskSpaceReplayErrorCode::InvalidPatch,
            };
            TaskSpaceReplayError::new(code, error.message)
        },
    )
}

fn validate_snapshot(snapshot: &ActionMapSnapshot) -> Result<(), TaskSpaceReplayError> {
    let mut runtime = ActionMapRuntimeState::default();
    runtime
        .restore_snapshot(snapshot.clone())
        .map_err(|message| {
            let code = if message.contains("legacy_schema_unsupported") {
                TaskSpaceReplayErrorCode::UnsupportedSnapshotSchema
            } else {
                TaskSpaceReplayErrorCode::DomainInvariant
            };
            TaskSpaceReplayError::new(code, message)
        })?;
    if runtime.snapshot() != *snapshot {
        return Err(TaskSpaceReplayError::new(
            TaskSpaceReplayErrorCode::NoncanonicalSnapshot,
            "snapshot restore roundtrip changed canonical state",
        ));
    }
    Ok(())
}

fn is_checkpoint(item: &&RolloutItem) -> bool {
    matches!(
        item,
        RolloutItem::EventMsg(EventMsg::MapRuntime(
            MapRuntimeEvent::SnapshotUpdated(_) | MapRuntimeEvent::TerminalCommitted(_)
        ))
    )
}

fn is_delta(item: &&RolloutItem) -> bool {
    matches!(
        item,
        RolloutItem::EventMsg(EventMsg::MapRuntime(MapRuntimeEvent::SnapshotDelta(_)))
    )
}
