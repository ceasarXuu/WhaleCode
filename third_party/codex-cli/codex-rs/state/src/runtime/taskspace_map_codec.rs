use crate::BindTaskSpaceMapRequest;
use crate::TaskSpaceMapBindingRecord;
use crate::TaskSpaceMapRecord;
use crate::TaskSpaceMapRelation;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Row;

pub(super) fn decode_map_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskSpaceMapRecord> {
    let map_id: String = row.try_get("map_id")?;
    let owner_thread_id = parse_thread_id(row.try_get("owner_thread_id")?, "owner_thread_id")?;
    let schema_version: String = row.try_get("canonical_schema_version")?;
    if schema_version != TASKSPACE_CANONICAL_SCHEMA_VERSION {
        anyhow::bail!(
            "TaskSpace map `{map_id}` uses unsupported canonical schema `{schema_version}`"
        );
    }
    let canonical_json: String = row.try_get("canonical_json")?;
    let expected_sha256: String = row.try_get("canonical_sha256")?;
    let actual_sha256 = sha256(canonical_json.as_bytes());
    if actual_sha256 != expected_sha256 {
        anyhow::bail!("TaskSpace map `{map_id}` canonical hash mismatch");
    }
    let canonical_map: Option<TaskSpaceCanonicalMap> = serde_json::from_str(&canonical_json)?;
    validate_map_identity(&map_id, canonical_map.as_ref())?;
    let stored_map_revision = from_i64(row.try_get("map_revision")?, "map_revision")?;
    if map_revision(canonical_map.as_ref()) != stored_map_revision {
        anyhow::bail!("TaskSpace map `{map_id}` revision mismatch");
    }
    let stored_terminal = row.try_get::<i64, _>("terminal")? != 0;
    if map_terminal(canonical_map.as_ref()) != stored_terminal {
        anyhow::bail!("TaskSpace map `{map_id}` terminal state mismatch");
    }
    Ok(TaskSpaceMapRecord {
        map_id,
        owner_thread_id,
        canonical_map,
        canonical_sha256: expected_sha256,
        store_revision: from_i64(row.try_get("store_revision")?, "store_revision")?,
        map_revision: stored_map_revision,
        terminal: stored_terminal,
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
        created_at_ms: row.try_get("binding_created_at_ms")?,
        updated_at_ms: row.try_get("binding_updated_at_ms")?,
    })
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
    if let Some(map) = canonical_map
        && map.map_id != map_id
    {
        anyhow::bail!(
            "TaskSpace canonical map id `{}` does not match store map id `{map_id}`",
            map.map_id
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

pub(super) fn map_revision(canonical_map: Option<&TaskSpaceCanonicalMap>) -> u64 {
    canonical_map.map_or(0, |map| map.revision)
}

pub(super) fn map_terminal(canonical_map: Option<&TaskSpaceCanonicalMap>) -> bool {
    canonical_map.is_some_and(|map| map.terminal_record.is_some())
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
