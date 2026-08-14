use super::StateRuntime;
use super::taskspace_map_codec::canonical_sha256;
use super::taskspace_map_codec::decode_binding_row;
use super::taskspace_map_codec::from_i64;
use super::taskspace_map_codec::request_sha256;
use super::taskspace_map_codec::require_nonempty;
use super::taskspace_map_codec::to_i64;
use super::taskspace_map_codec::validate_map_identity;
use super::taskspace_map_repository::compare_and_swap_map;
use super::taskspace_map_repository::insert_map_head;
use super::taskspace_map_repository::load_map_in_tx;
use crate::BindTaskSpaceMapRequest;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use crate::TaskSpaceMapBindingRecord;
use crate::TaskSpaceMapRecord;
use crate::TaskSpaceMapRelation;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::ThreadId;
use sqlx::Row;
use sqlx::Sqlite;
use sqlx::Transaction;

impl StateRuntime {
    pub async fn create_taskspace_map(
        &self,
        request: CreateTaskSpaceMapRequest,
    ) -> anyhow::Result<TaskSpaceMapWriteOutcome> {
        validate_map_identity(&request.map_id, request.canonical_map.as_ref())?;
        require_nonempty("commit_id", &request.commit_id)?;
        require_nonempty("operation", &request.operation)?;
        let canonical_sha256 = canonical_sha256(&request.canonical_map)?;
        let request_sha256 = request_sha256(
            &request.map_id,
            0,
            &canonical_sha256,
            &request.operation,
            request.owner_thread_id,
            None,
            &[],
        )?;
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        if let Some(outcome) = replay_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            &request_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(outcome);
        }

        let record = TaskSpaceMapRecord {
            map_id: request.map_id.clone(),
            owner_thread_id: request.owner_thread_id,
            canonical_map: request.canonical_map,
            canonical_sha256: canonical_sha256.clone(),
            store_revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
        };
        if !insert_map_head(&mut tx, &record).await? {
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
            &canonical_sha256,
            &request_sha256,
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
        let mut tx = self.pool.begin().await?;
        let record = load_map_in_tx(&mut tx, map_id).await?;
        tx.commit().await?;
        Ok(record)
    }

    pub async fn load_taskspace_map_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> anyhow::Result<Option<(TaskSpaceMapRecord, TaskSpaceMapBindingRecord)>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
SELECT
    b.map_id, b.thread_id AS binding_thread_id, b.relation, b.parent_thread_id,
    b.created_at_ms AS binding_created_at_ms, b.updated_at_ms AS binding_updated_at_ms
FROM taskspace_map_bindings b
WHERE b.thread_id = ?
            "#,
        )
        .bind(thread_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        let result = if let Some(row) = row {
            let binding = decode_binding_row(&row)?;
            let map = load_map_in_tx(&mut tx, &binding.map_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("TaskSpace binding references a missing map"))?;
            Some((map, binding))
        } else {
            None
        };
        tx.commit().await?;
        Ok(result)
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
        validate_map_identity(&request.map_id, request.canonical_map.as_ref())?;
        require_nonempty("commit_id", &request.commit_id)?;
        require_nonempty("operation", &request.operation)?;
        let canonical_sha256 = canonical_sha256(&request.canonical_map)?;
        let request_sha256 = request_sha256(
            &request.map_id,
            request.expected_store_revision,
            &canonical_sha256,
            &request.operation,
            request.actor_thread_id,
            request.binding.as_ref(),
            &request.consumed_pending_action_ids,
        )?;
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
            &request_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(outcome);
        }

        let Some(current) = load_map_in_tx(&mut tx, &request.map_id).await? else {
            tx.commit().await?;
            return Ok(TaskSpaceMapWriteOutcome::Conflict { current: None });
        };
        super::taskspace_pending_actions::validate_pending_provider_action_set(
            &mut tx,
            request.actor_thread_id,
            &request.map_id,
            &request.consumed_pending_action_ids,
        )
        .await?;
        if current.store_revision != request.expected_store_revision
            || !compare_and_swap_map(
                &mut tx,
                &current,
                &request.canonical_map,
                next_revision,
                &canonical_sha256,
                now,
            )
            .await?
        {
            let current = load_map_in_tx(&mut tx, &request.map_id).await?;
            tx.commit().await?;
            return Ok(TaskSpaceMapWriteOutcome::Conflict { current });
        }

        if let Some(binding) = request.binding.as_ref() {
            if binding.map_id != request.map_id {
                anyhow::bail!("TaskSpace commit binding targets a different map");
            }
            insert_binding(&mut tx, binding, now).await?;
        }
        insert_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            request.expected_store_revision,
            next_revision,
            &canonical_sha256,
            &request_sha256,
            &request.operation,
            request.actor_thread_id,
            now,
        )
        .await?;
        super::taskspace_pending_actions::delete_pending_provider_actions(
            &mut tx,
            &request.consumed_pending_action_ids,
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
    thread_id, map_id, relation, parent_thread_id, created_at_ms, updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT(thread_id) DO UPDATE SET
    relation = excluded.relation,
    parent_thread_id = excluded.parent_thread_id,
    updated_at_ms = excluded.updated_at_ms
WHERE taskspace_map_bindings.map_id = excluded.map_id
        "#,
    )
    .bind(request.thread_id.to_string())
    .bind(&request.map_id)
    .bind(request.relation.as_str())
    .bind(request.parent_thread_id.map(|id| id.to_string()))
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    let binding = load_binding_in_tx(tx, request.thread_id).await?;
    if binding.as_ref().map(|value| value.map_id.as_str()) != Some(request.map_id.as_str()) {
        tracing::warn!(
            target: "codex_state::taskspace",
            event_name = "taskspace.map_store_binding_conflict",
            actor_thread_id = %request.thread_id,
            attempted_map_id = request.map_id,
            current_map_id = binding.as_ref().map(|value| value.map_id.as_str()),
            "refused to bind thread to a different canonical TaskSpace Map"
        );
        anyhow::bail!(
            "thread `{}` is already bound to another TaskSpace map",
            request.thread_id
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_commit(
    tx: &mut Transaction<'_, Sqlite>,
    commit_id: &str,
    map_id: &str,
    expected_store_revision: u64,
    result_store_revision: u64,
    canonical_sha256: &str,
    request_sha256: &str,
    operation: &str,
    actor_thread_id: ThreadId,
    now: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT INTO taskspace_map_commits (
    commit_id, map_id, expected_store_revision, result_store_revision,
    canonical_sha256, request_sha256, operation, actor_thread_id, created_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(commit_id)
    .bind(map_id)
    .bind(to_i64(expected_store_revision, "expected_store_revision")?)
    .bind(to_i64(result_store_revision, "result_store_revision")?)
    .bind(canonical_sha256)
    .bind(request_sha256)
    .bind(operation)
    .bind(actor_thread_id.to_string())
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn replay_commit(
    tx: &mut Transaction<'_, Sqlite>,
    commit_id: &str,
    map_id: &str,
    request_sha256: &str,
) -> anyhow::Result<Option<TaskSpaceMapWriteOutcome>> {
    let row = sqlx::query(
        r#"
SELECT map_id, result_store_revision, request_sha256
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
        && row.try_get::<String, _>("request_sha256")? == request_sha256;
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

async fn load_binding_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    thread_id: ThreadId,
) -> anyhow::Result<Option<TaskSpaceMapBindingRecord>> {
    let row = sqlx::query(
        r#"
SELECT thread_id AS binding_thread_id, map_id, relation, parent_thread_id,
       created_at_ms AS binding_created_at_ms, updated_at_ms AS binding_updated_at_ms
FROM taskspace_map_bindings
WHERE thread_id = ?
        "#,
    )
    .bind(thread_id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| decode_binding_row(&row)).transpose()
}
