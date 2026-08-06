use crate::BindTaskSpaceMapRequest;
use crate::TaskSpaceMapBindingRecord;
use crate::TaskSpaceMapRelation;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceNodeState;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Row;

pub(super) fn decode_binding_row(
    row: &sqlx::sqlite::SqliteRow,
) -> anyhow::Result<TaskSpaceMapBindingRecord> {
    Ok(TaskSpaceMapBindingRecord {
        thread_id: parse_thread_id(row.try_get("binding_thread_id")?, "thread_id")?,
        map_id: row.try_get("map_id")?,
        relation: TaskSpaceMapRelation::from_str(&row.try_get::<String, _>("relation")?)?,
        parent_thread_id: row
            .try_get::<Option<String>, _>("parent_thread_id")?
            .map(|value| parse_thread_id(value, "parent_thread_id"))
            .transpose()?,
        created_at_ms: row.try_get("binding_created_at_ms")?,
        updated_at_ms: row.try_get("binding_updated_at_ms")?,
    })
}

pub(super) fn canonical_sha256(
    canonical_map: &Option<TaskSpaceCanonicalMap>,
) -> anyhow::Result<String> {
    Ok(sha256(&serde_json::to_vec(canonical_map)?))
}

pub(super) fn request_sha256(
    map_id: &str,
    expected_store_revision: u64,
    canonical_sha256: &str,
    operation: &str,
    actor_thread_id: ThreadId,
    binding: Option<&BindTaskSpaceMapRequest>,
) -> anyhow::Result<String> {
    let binding = binding.map(|binding| {
        (
            binding.thread_id.to_string(),
            binding.map_id.as_str(),
            binding.relation.as_str(),
            binding.parent_thread_id.map(|id| id.to_string()),
        )
    });
    let bytes = serde_json::to_vec(&(
        map_id,
        expected_store_revision,
        canonical_sha256,
        operation,
        actor_thread_id.to_string(),
        binding,
    ))?;
    Ok(sha256(&bytes))
}

pub(super) fn validate_map_identity(
    map_id: &str,
    canonical_map: Option<&TaskSpaceCanonicalMap>,
) -> anyhow::Result<()> {
    require_nonempty("map_id", map_id)?;
    if let Some(map) = canonical_map {
        if map.schema_version != TASKSPACE_CANONICAL_SCHEMA_VERSION {
            anyhow::bail!(
                "unsupported TaskSpace canonical schema `{}`; expected `{TASKSPACE_CANONICAL_SCHEMA_VERSION}`",
                map.schema_version
            );
        }
        if map.map_id != map_id {
            anyhow::bail!(
                "TaskSpace canonical map id `{}` does not match store map id `{map_id}`",
                map.map_id
            );
        }
    }
    Ok(())
}

pub(super) fn node_state_name(state: TaskSpaceNodeState) -> &'static str {
    match state {
        TaskSpaceNodeState::Waiting => "waiting",
        TaskSpaceNodeState::Ready => "ready",
        TaskSpaceNodeState::InFlight => "in_flight",
        TaskSpaceNodeState::Blocked => "blocked",
        TaskSpaceNodeState::Completed => "completed",
    }
}

pub(super) fn parse_node_state(value: &str) -> anyhow::Result<TaskSpaceNodeState> {
    match value {
        "waiting" => Ok(TaskSpaceNodeState::Waiting),
        "ready" => Ok(TaskSpaceNodeState::Ready),
        "in_flight" => Ok(TaskSpaceNodeState::InFlight),
        "blocked" => Ok(TaskSpaceNodeState::Blocked),
        "completed" => Ok(TaskSpaceNodeState::Completed),
        _ => anyhow::bail!("invalid TaskSpace node state `{value}`"),
    }
}

pub(super) fn action_outcome_name(outcome: TaskSpaceActionOutcome) -> &'static str {
    match outcome {
        TaskSpaceActionOutcome::Pending => "pending",
        TaskSpaceActionOutcome::Succeeded => "succeeded",
        TaskSpaceActionOutcome::Failed => "failed",
        TaskSpaceActionOutcome::Cancelled => "cancelled",
    }
}

pub(super) fn parse_action_outcome(value: &str) -> anyhow::Result<TaskSpaceActionOutcome> {
    match value {
        "pending" => Ok(TaskSpaceActionOutcome::Pending),
        "succeeded" => Ok(TaskSpaceActionOutcome::Succeeded),
        "failed" => Ok(TaskSpaceActionOutcome::Failed),
        "cancelled" => Ok(TaskSpaceActionOutcome::Cancelled),
        _ => anyhow::bail!("invalid TaskSpace action outcome `{value}`"),
    }
}

pub(super) fn require_nonempty(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("TaskSpace {field} must not be empty");
    }
    Ok(())
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn to_i64(value: u64, field: &str) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} exceeds SQLite INTEGER"))
}

pub(super) fn from_i64(value: i64, field: &str) -> anyhow::Result<u64> {
    u64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} is negative"))
}

pub(super) fn parse_thread_id(value: String, field: &str) -> anyhow::Result<ThreadId> {
    ThreadId::from_string(&value).map_err(|error| anyhow::anyhow!("invalid {field}: {error}"))
}
