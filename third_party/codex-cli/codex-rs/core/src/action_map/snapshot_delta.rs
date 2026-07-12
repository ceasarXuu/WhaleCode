use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::MapRuntimeSnapshotDeltaEvent;
use sha2::Digest;
use sha2::Sha256;

#[derive(Debug, Clone, Default)]
pub(crate) struct ActionMapCheckpointState {
    pub(crate) checkpoint_id: Option<String>,
    pub(crate) snapshot_sha256: Option<String>,
    pub(crate) snapshot: Option<ActionMapSnapshot>,
    pub(crate) delta_sequence: u64,
    pub(crate) provider_responses_since_checkpoint: u64,
}

impl ActionMapCheckpointState {
    pub(crate) fn install(
        &mut self,
        checkpoint_id: String,
        snapshot_sha256: String,
        snapshot: ActionMapSnapshot,
    ) {
        self.checkpoint_id = Some(checkpoint_id);
        self.snapshot_sha256 = Some(snapshot_sha256);
        self.snapshot = Some(snapshot);
        self.delta_sequence = 0;
        self.provider_responses_since_checkpoint = 0;
    }
}

pub(crate) fn snapshot_sha256(snapshot: &ActionMapSnapshot) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(snapshot)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn build_snapshot_delta(
    checkpoint: &mut ActionMapCheckpointState,
    current: &ActionMapSnapshot,
) -> Result<Option<MapRuntimeSnapshotDeltaEvent>, String> {
    let Some(base_snapshot) = checkpoint.snapshot.as_ref() else {
        return Ok(None);
    };
    let base_checkpoint_id = checkpoint
        .checkpoint_id
        .clone()
        .ok_or_else(|| "checkpoint snapshot is missing checkpoint id".to_string())?;
    let base_snapshot_sha256 = checkpoint
        .snapshot_sha256
        .clone()
        .ok_or_else(|| "checkpoint snapshot is missing content hash".to_string())?;
    let base = serde_json::to_value(base_snapshot).map_err(|error| error.to_string())?;
    let target = serde_json::to_value(current).map_err(|error| error.to_string())?;
    let patch = json_patch::diff(&base, &target);
    if patch.0.is_empty() {
        return Ok(None);
    }
    checkpoint.delta_sequence = checkpoint.delta_sequence.saturating_add(1);
    Ok(Some(MapRuntimeSnapshotDeltaEvent {
        base_checkpoint_id,
        sequence: checkpoint.delta_sequence,
        base_snapshot_sha256,
        snapshot_sha256: snapshot_sha256(current).map_err(|error| error.to_string())?,
        patch: serde_json::to_value(patch).map_err(|error| error.to_string())?,
    }))
}

pub(crate) fn apply_snapshot_delta(
    checkpoint_id: &str,
    checkpoint_snapshot_sha256: &str,
    checkpoint_snapshot: &ActionMapSnapshot,
    delta: &MapRuntimeSnapshotDeltaEvent,
) -> Result<ActionMapSnapshot, String> {
    if delta.base_checkpoint_id != checkpoint_id {
        return Err(format!(
            "snapshot delta base checkpoint mismatch: expected {checkpoint_id}, got {}",
            delta.base_checkpoint_id
        ));
    }
    let actual_base_hash =
        snapshot_sha256(checkpoint_snapshot).map_err(|error| error.to_string())?;
    if checkpoint_snapshot_sha256 != actual_base_hash
        || delta.base_snapshot_sha256 != actual_base_hash
    {
        return Err("snapshot delta base hash mismatch".to_string());
    }
    let patch: json_patch::Patch =
        serde_json::from_value(delta.patch.clone()).map_err(|error| error.to_string())?;
    let mut value = serde_json::to_value(checkpoint_snapshot).map_err(|error| error.to_string())?;
    json_patch::patch(&mut value, &patch).map_err(|error| error.to_string())?;
    let snapshot: ActionMapSnapshot =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    let actual_hash = snapshot_sha256(&snapshot).map_err(|error| error.to_string())?;
    if actual_hash != delta.snapshot_sha256 {
        return Err("snapshot delta result hash mismatch".to_string());
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::protocol::MapRuntimeMode;

    fn snapshot(routing_required: bool) -> ActionMapSnapshot {
        ActionMapSnapshot {
            mode: MapRuntimeMode::Experiment,
            routing_required,
            bootstrap_required: true,
            reborn_requested: false,
            active_task_id: None,
            active_map_id: None,
            tasks: Vec::new(),
            maps: Vec::new(),
            maintenance_barriers: Vec::new(),
            trace_summary: Default::default(),
            trace_events: Vec::new(),
            sentinel_summary: Default::default(),
            sentinel_warnings: Vec::new(),
        }
    }

    #[test]
    fn delta_round_trip_reconstructs_exact_snapshot() {
        let base = snapshot(true);
        let expected = snapshot(false);
        let base_hash = snapshot_sha256(&base).unwrap();
        let mut checkpoint = ActionMapCheckpointState::default();
        checkpoint.install(
            "map-checkpoint-test".to_string(),
            base_hash.clone(),
            base.clone(),
        );

        let delta = build_snapshot_delta(&mut checkpoint, &expected)
            .unwrap()
            .unwrap();
        let actual =
            apply_snapshot_delta("map-checkpoint-test", &base_hash, &base, &delta).unwrap();

        assert_eq!(actual, expected);
    }
}
