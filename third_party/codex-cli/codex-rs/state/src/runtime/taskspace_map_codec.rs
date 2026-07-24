use crate::BindTaskSpaceMapRequest;
use crate::TaskSpaceMapBindingRecord;
use crate::TaskSpaceMapRecord;
use crate::TaskSpaceMapRelation;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Row;

pub(super) fn decode_map_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskSpaceMapRecord> {
    let map_id: String = row.try_get("map_id")?;
    let owner_thread_id = parse_thread_id(row.try_get("owner_thread_id")?, "owner_thread_id")?;
    let snapshot_json: String = row.try_get("snapshot_json")?;
    let expected_sha256: String = row.try_get("snapshot_sha256")?;
    let actual_sha256 = sha256(snapshot_json.as_bytes());
    if actual_sha256 != expected_sha256 {
        anyhow::bail!("TaskSpace map `{map_id}` snapshot hash mismatch");
    }
    let snapshot: ActionMapSnapshot = serde_json::from_str(&snapshot_json)?;
    validate_map_identity(&map_id, &snapshot)?;
    let stored_graph_revision = from_i64(row.try_get("graph_revision")?, "graph_revision")?;
    if graph_revision(&snapshot) != stored_graph_revision {
        anyhow::bail!("TaskSpace map `{map_id}` graph revision mismatch");
    }
    let stored_complete = row.try_get::<i64, _>("complete")? != 0;
    if map_complete(&snapshot) != stored_complete {
        anyhow::bail!("TaskSpace map `{map_id}` terminal state mismatch");
    }
    Ok(TaskSpaceMapRecord {
        map_id,
        owner_thread_id,
        snapshot,
        snapshot_sha256: expected_sha256,
        store_revision: from_i64(row.try_get("store_revision")?, "store_revision")?,
        graph_revision: stored_graph_revision,
        complete: stored_complete,
        created_at_ms: row.try_get("created_at_ms")?,
        updated_at_ms: row.try_get("updated_at_ms")?,
    })
}

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
        node_id: row.try_get("node_id")?,
        lease_id: row.try_get("lease_id")?,
        created_at_ms: row.try_get("binding_created_at_ms")?,
        updated_at_ms: row.try_get("binding_updated_at_ms")?,
    })
}

pub(super) fn request_sha256(
    map_id: &str,
    expected_store_revision: u64,
    snapshot_sha256: &str,
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
            binding.node_id.as_deref(),
            binding.lease_id.as_deref(),
        )
    });
    let bytes = serde_json::to_vec(&(
        map_id,
        expected_store_revision,
        snapshot_sha256,
        operation,
        actor_thread_id.to_string(),
        binding,
    ))?;
    Ok(sha256(&bytes))
}

pub(super) fn validate_map_identity(
    map_id: &str,
    snapshot: &ActionMapSnapshot,
) -> anyhow::Result<()> {
    require_nonempty("map_id", map_id)?;
    if let Some(map) = snapshot.map.as_ref()
        && map.id != map_id
    {
        anyhow::bail!(
            "TaskSpace snapshot map id `{}` does not match store map id `{map_id}`",
            map.id
        );
    }
    Ok(())
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

pub(super) fn graph_revision(snapshot: &ActionMapSnapshot) -> u64 {
    snapshot.map.as_ref().map_or(0, |map| map.revision)
}

pub(super) fn map_complete(snapshot: &ActionMapSnapshot) -> bool {
    snapshot.map.as_ref().is_some_and(|map| map.complete)
}

pub(super) fn to_i64(value: u64, field: &str) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} exceeds SQLite INTEGER"))
}

pub(super) fn from_i64(value: i64, field: &str) -> anyhow::Result<u64> {
    u64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} is negative"))
}

fn parse_thread_id(value: String, field: &str) -> anyhow::Result<ThreadId> {
    ThreadId::from_string(&value).map_err(|error| anyhow::anyhow!("invalid {field}: {error}"))
}
