use super::StateRuntime;
use crate::BindTaskSpaceMapRequest;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use crate::TaskSpaceMapBindingRecord;
use crate::TaskSpaceMapRecord;
use crate::TaskSpaceMapRelation;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Row;
use sqlx::Sqlite;
use sqlx::Transaction;

impl StateRuntime {
    pub async fn create_taskspace_map(
        &self,
        request: CreateTaskSpaceMapRequest,
    ) -> anyhow::Result<TaskSpaceMapWriteOutcome> {
        validate_map_identity(&request.map_id, &request.snapshot)?;
        require_nonempty("commit_id", &request.commit_id)?;
        require_nonempty("operation", &request.operation)?;
        let snapshot_json = serde_json::to_string(&request.snapshot)?;
        let snapshot_sha256 = snapshot_sha256(snapshot_json.as_bytes());
        let graph_revision = graph_revision(&request.snapshot);
        let complete = map_complete(&request.snapshot);
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        if let Some(outcome) = replay_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            0,
            &snapshot_sha256,
            &request.operation,
            request.owner_thread_id,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(outcome);
        }

        let inserted = sqlx::query(
            r#"
INSERT INTO taskspace_maps (
    map_id, owner_thread_id, snapshot_json, snapshot_sha256,
    store_revision, graph_revision, complete, created_at_ms, updated_at_ms
) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?)
ON CONFLICT(map_id) DO NOTHING
            "#,
        )
        .bind(&request.map_id)
        .bind(request.owner_thread_id.to_string())
        .bind(snapshot_json)
        .bind(&snapshot_sha256)
        .bind(to_i64(graph_revision, "graph_revision")?)
        .bind(i64::from(complete))
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 0 {
            let current = load_map_in_tx(&mut tx, &request.map_id).await?;
            tx.commit().await?;
            return Ok(TaskSpaceMapWriteOutcome::Conflict { current });
        }

        insert_binding(
            &mut tx,
            &BindTaskSpaceMapRequest {
                thread_id: request.owner_thread_id,
                map_id: request.map_id.clone(),
                relation: TaskSpaceMapRelation::Owner,
                parent_thread_id: None,
                node_id: None,
                lease_id: None,
            },
            now,
        )
        .await?;
        insert_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            0,
            1,
            &snapshot_sha256,
            &request.operation,
            request.owner_thread_id,
            now,
        )
        .await?;
        let record = load_map_in_tx(&mut tx, &request.map_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("created TaskSpace map disappeared before commit"))?;
        tx.commit().await?;
        Ok(TaskSpaceMapWriteOutcome::Applied(record))
    }

    pub async fn load_taskspace_map(
        &self,
        map_id: &str,
    ) -> anyhow::Result<Option<TaskSpaceMapRecord>> {
        let row = sqlx::query(
            r#"
SELECT map_id, owner_thread_id, snapshot_json, snapshot_sha256,
       store_revision, graph_revision, complete, created_at_ms, updated_at_ms
FROM taskspace_maps
WHERE map_id = ?
            "#,
        )
        .bind(map_id)
        .fetch_optional(self.pool.as_ref())
        .await?;
        row.map(|row| decode_map_row(&row)).transpose()
    }

    pub async fn load_taskspace_map_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> anyhow::Result<Option<(TaskSpaceMapRecord, TaskSpaceMapBindingRecord)>> {
        let row = sqlx::query(
            r#"
SELECT
    m.map_id, m.owner_thread_id, m.snapshot_json, m.snapshot_sha256,
    m.store_revision, m.graph_revision, m.complete, m.created_at_ms, m.updated_at_ms,
    b.thread_id AS binding_thread_id, b.relation, b.parent_thread_id,
    b.node_id, b.lease_id, b.created_at_ms AS binding_created_at_ms,
    b.updated_at_ms AS binding_updated_at_ms
FROM taskspace_map_bindings b
JOIN taskspace_maps m ON m.map_id = b.map_id
WHERE b.thread_id = ?
            "#,
        )
        .bind(thread_id.to_string())
        .fetch_optional(self.pool.as_ref())
        .await?;
        row.map(|row| {
            let map = decode_map_row(&row)?;
            let binding = decode_binding_row(&row)?;
            Ok((map, binding))
        })
        .transpose()
    }

    pub async fn bind_thread_to_taskspace_map(
        &self,
        request: BindTaskSpaceMapRequest,
    ) -> anyhow::Result<TaskSpaceMapBindingRecord> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if load_map_in_tx(&mut tx, &request.map_id).await?.is_none() {
            anyhow::bail!("TaskSpace map `{}` does not exist", request.map_id);
        }
        insert_binding(&mut tx, &request, now).await?;
        let binding = load_binding_in_tx(&mut tx, request.thread_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("TaskSpace binding disappeared before commit"))?;
        tx.commit().await?;
        Ok(binding)
    }

    pub async fn compare_and_swap_taskspace_map(
        &self,
        request: CommitTaskSpaceMapRequest,
    ) -> anyhow::Result<TaskSpaceMapWriteOutcome> {
        validate_map_identity(&request.map_id, &request.snapshot)?;
        require_nonempty("commit_id", &request.commit_id)?;
        require_nonempty("operation", &request.operation)?;
        let snapshot_json = serde_json::to_string(&request.snapshot)?;
        let snapshot_sha256 = snapshot_sha256(snapshot_json.as_bytes());
        let graph_revision = graph_revision(&request.snapshot);
        let complete = map_complete(&request.snapshot);
        let next_revision = request
            .expected_store_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("TaskSpace store revision overflow"))?;
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        if let Some(outcome) = replay_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            request.expected_store_revision,
            &snapshot_sha256,
            &request.operation,
            request.actor_thread_id,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(outcome);
        }

        let updated = sqlx::query(
            r#"
UPDATE taskspace_maps
SET snapshot_json = ?,
    snapshot_sha256 = ?,
    store_revision = ?,
    graph_revision = ?,
    complete = ?,
    updated_at_ms = ?
WHERE map_id = ? AND store_revision = ?
            "#,
        )
        .bind(snapshot_json)
        .bind(&snapshot_sha256)
        .bind(to_i64(next_revision, "store_revision")?)
        .bind(to_i64(graph_revision, "graph_revision")?)
        .bind(i64::from(complete))
        .bind(now)
        .bind(&request.map_id)
        .bind(to_i64(
            request.expected_store_revision,
            "expected_store_revision",
        )?)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated == 0 {
            let current = load_map_in_tx(&mut tx, &request.map_id).await?;
            tx.commit().await?;
            return Ok(TaskSpaceMapWriteOutcome::Conflict { current });
        }

        insert_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            request.expected_store_revision,
            next_revision,
            &snapshot_sha256,
            &request.operation,
            request.actor_thread_id,
            now,
        )
        .await?;
        let record = load_map_in_tx(&mut tx, &request.map_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("committed TaskSpace map disappeared before commit"))?;
        tx.commit().await?;
        Ok(TaskSpaceMapWriteOutcome::Applied(record))
    }
}

async fn insert_binding(
    tx: &mut Transaction<'_, Sqlite>,
    request: &BindTaskSpaceMapRequest,
    now: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT INTO taskspace_map_bindings (
    thread_id, map_id, relation, parent_thread_id, node_id, lease_id,
    created_at_ms, updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(thread_id) DO UPDATE SET
    relation = excluded.relation,
    parent_thread_id = excluded.parent_thread_id,
    node_id = excluded.node_id,
    lease_id = excluded.lease_id,
    updated_at_ms = excluded.updated_at_ms
WHERE taskspace_map_bindings.map_id = excluded.map_id
        "#,
    )
    .bind(request.thread_id.to_string())
    .bind(&request.map_id)
    .bind(request.relation.as_str())
    .bind(request.parent_thread_id.map(|id| id.to_string()))
    .bind(request.node_id.as_deref())
    .bind(request.lease_id.as_deref())
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    let binding = load_binding_in_tx(tx, request.thread_id).await?;
    if binding.as_ref().map(|value| value.map_id.as_str()) != Some(request.map_id.as_str()) {
        anyhow::bail!(
            "thread `{}` is already bound to another TaskSpace map",
            request.thread_id
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_commit(
    tx: &mut Transaction<'_, Sqlite>,
    commit_id: &str,
    map_id: &str,
    expected_store_revision: u64,
    result_store_revision: u64,
    snapshot_sha256: &str,
    operation: &str,
    actor_thread_id: ThreadId,
    now: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT INTO taskspace_map_commits (
    commit_id, map_id, expected_store_revision, result_store_revision,
    snapshot_sha256, operation, actor_thread_id, created_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(commit_id)
    .bind(map_id)
    .bind(to_i64(expected_store_revision, "expected_store_revision")?)
    .bind(to_i64(result_store_revision, "result_store_revision")?)
    .bind(snapshot_sha256)
    .bind(operation)
    .bind(actor_thread_id.to_string())
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replay_commit(
    tx: &mut Transaction<'_, Sqlite>,
    commit_id: &str,
    map_id: &str,
    expected_store_revision: u64,
    snapshot_sha256: &str,
    operation: &str,
    actor_thread_id: ThreadId,
) -> anyhow::Result<Option<TaskSpaceMapWriteOutcome>> {
    let row = sqlx::query(
        r#"
SELECT map_id, expected_store_revision, result_store_revision,
       snapshot_sha256, operation, actor_thread_id
FROM taskspace_map_commits
WHERE commit_id = ?
        "#,
    )
    .bind(commit_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let matches = row.try_get::<String, _>("map_id")? == map_id
        && from_i64(
            row.try_get("expected_store_revision")?,
            "expected_store_revision",
        )? == expected_store_revision
        && row.try_get::<String, _>("snapshot_sha256")? == snapshot_sha256
        && row.try_get::<String, _>("operation")? == operation
        && row.try_get::<String, _>("actor_thread_id")? == actor_thread_id.to_string();
    if !matches {
        anyhow::bail!("TaskSpace commit id `{commit_id}` was reused with different input");
    }
    let result_revision = from_i64(
        row.try_get("result_store_revision")?,
        "result_store_revision",
    )?;
    let record = load_map_in_tx(tx, map_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("idempotent TaskSpace commit references a missing map"))?;
    if record.store_revision < result_revision {
        anyhow::bail!("TaskSpace map revision regressed behind an idempotent commit");
    }
    Ok(Some(TaskSpaceMapWriteOutcome::IdempotentReplay(record)))
}

async fn load_map_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
) -> anyhow::Result<Option<TaskSpaceMapRecord>> {
    let row = sqlx::query(
        r#"
SELECT map_id, owner_thread_id, snapshot_json, snapshot_sha256,
       store_revision, graph_revision, complete, created_at_ms, updated_at_ms
FROM taskspace_maps
WHERE map_id = ?
        "#,
    )
    .bind(map_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| decode_map_row(&row)).transpose()
}

async fn load_binding_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    thread_id: ThreadId,
) -> anyhow::Result<Option<TaskSpaceMapBindingRecord>> {
    let row = sqlx::query(
        r#"
SELECT thread_id AS binding_thread_id, map_id, relation, parent_thread_id,
       node_id, lease_id, created_at_ms AS binding_created_at_ms,
       updated_at_ms AS binding_updated_at_ms
FROM taskspace_map_bindings
WHERE thread_id = ?
        "#,
    )
    .bind(thread_id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| decode_binding_row(&row)).transpose()
}

fn decode_map_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskSpaceMapRecord> {
    let map_id: String = row.try_get("map_id")?;
    let owner_thread_id = parse_thread_id(row.try_get("owner_thread_id")?, "owner_thread_id")?;
    let snapshot_json: String = row.try_get("snapshot_json")?;
    let expected_sha256: String = row.try_get("snapshot_sha256")?;
    let actual_sha256 = snapshot_sha256(snapshot_json.as_bytes());
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

fn decode_binding_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskSpaceMapBindingRecord> {
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

fn validate_map_identity(map_id: &str, snapshot: &ActionMapSnapshot) -> anyhow::Result<()> {
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

fn require_nonempty(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("TaskSpace {field} must not be empty");
    }
    Ok(())
}

fn snapshot_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn graph_revision(snapshot: &ActionMapSnapshot) -> u64 {
    snapshot.map.as_ref().map_or(0, |map| map.revision)
}

fn map_complete(snapshot: &ActionMapSnapshot) -> bool {
    snapshot.map.as_ref().is_some_and(|map| map.complete)
}

fn to_i64(value: u64, field: &str) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} exceeds SQLite INTEGER"))
}

fn from_i64(value: i64, field: &str) -> anyhow::Result<u64> {
    u64::try_from(value).map_err(|_| anyhow::anyhow!("TaskSpace {field} is negative"))
}

fn parse_thread_id(value: String, field: &str) -> anyhow::Result<ThreadId> {
    ThreadId::from_string(&value).map_err(|error| anyhow::anyhow!("invalid {field}: {error}"))
}
