use super::StateRuntime;
use super::taskspace_map_codec::action_outcome_name;
use super::taskspace_map_codec::parse_action_outcome;
use super::taskspace_map_codec::parse_thread_id;
use super::taskspace_map_codec::require_nonempty;
use crate::EnqueueTaskSpacePendingProviderActionRequest;
use crate::TaskSpacePendingActionWriteOutcome;
use crate::TaskSpacePendingProviderAction;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use sqlx::Row;

impl StateRuntime {
    pub async fn enqueue_taskspace_pending_provider_action(
        &self,
        request: EnqueueTaskSpacePendingProviderActionRequest,
    ) -> anyhow::Result<TaskSpacePendingActionWriteOutcome> {
        validate_enqueue(&request)?;
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let inserted = sqlx::query(
            r#"
INSERT INTO taskspace_pending_provider_actions (
    action_id, origin_thread_id, map_id, provider_response_id,
    provider_action_key, tool_name, outcome, created_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(action_id) DO NOTHING
            "#,
        )
        .bind(&request.action_id)
        .bind(request.origin_thread_id.to_string())
        .bind(request.map_id.as_deref())
        .bind(&request.provider_response_id)
        .bind(&request.provider_action_key)
        .bind(&request.tool_name)
        .bind(action_outcome_name(request.outcome))
        .bind(now)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if !inserted {
            let existing = load_by_id(&mut tx, &request.action_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("pending Provider Action disappeared"))?;
            if !same_request(&existing, &request) {
                anyhow::bail!(
                    "TaskSpace pending Provider Action id `{}` was reused with different facts",
                    request.action_id
                );
            }
        }
        tx.commit().await?;
        Ok(if inserted {
            TaskSpacePendingActionWriteOutcome::Inserted
        } else {
            TaskSpacePendingActionWriteOutcome::IdempotentReplay
        })
    }

    pub async fn load_taskspace_pending_provider_actions(
        &self,
        thread_id: ThreadId,
        map_id: Option<&str>,
    ) -> anyhow::Result<Vec<TaskSpacePendingProviderAction>> {
        let rows = match map_id {
            Some(map_id) => {
                sqlx::query(
                    r#"
SELECT action_id, origin_thread_id, map_id, provider_response_id,
       provider_action_key, tool_name, outcome, created_at_ms
FROM taskspace_pending_provider_actions
WHERE map_id = ? OR (map_id IS NULL AND origin_thread_id = ?)
ORDER BY created_at_ms, action_id
                    "#,
                )
                .bind(map_id)
                .bind(thread_id.to_string())
                .fetch_all(self.pool.as_ref())
                .await?
            }
            None => {
                sqlx::query(
                    r#"
SELECT action_id, origin_thread_id, map_id, provider_response_id,
       provider_action_key, tool_name, outcome, created_at_ms
FROM taskspace_pending_provider_actions
WHERE map_id IS NULL AND origin_thread_id = ?
ORDER BY created_at_ms, action_id
                    "#,
                )
                .bind(thread_id.to_string())
                .fetch_all(self.pool.as_ref())
                .await?
            }
        };
        rows.iter().map(decode_row).collect()
    }
}

async fn load_by_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    action_id: &str,
) -> anyhow::Result<Option<TaskSpacePendingProviderAction>> {
    let row = sqlx::query(
        r#"
SELECT action_id, origin_thread_id, map_id, provider_response_id,
       provider_action_key, tool_name, outcome, created_at_ms
FROM taskspace_pending_provider_actions
WHERE action_id = ?
        "#,
    )
    .bind(action_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.as_ref().map(decode_row).transpose()
}

fn decode_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TaskSpacePendingProviderAction> {
    Ok(TaskSpacePendingProviderAction {
        action_id: row.try_get("action_id")?,
        origin_thread_id: parse_thread_id(row.try_get("origin_thread_id")?, "origin_thread_id")?,
        map_id: row.try_get("map_id")?,
        provider_response_id: row.try_get("provider_response_id")?,
        provider_action_key: row.try_get("provider_action_key")?,
        tool_name: row.try_get("tool_name")?,
        outcome: parse_action_outcome(row.try_get("outcome")?)?,
        created_at_ms: row.try_get("created_at_ms")?,
    })
}

fn validate_enqueue(request: &EnqueueTaskSpacePendingProviderActionRequest) -> anyhow::Result<()> {
    require_nonempty("action_id", &request.action_id)?;
    require_nonempty("provider_response_id", &request.provider_response_id)?;
    require_nonempty("provider_action_key", &request.provider_action_key)?;
    require_nonempty("tool_name", &request.tool_name)?;
    if request.outcome == TaskSpaceActionOutcome::Pending {
        anyhow::bail!("pending Provider Action must have a terminal Tool outcome");
    }
    if request
        .map_id
        .as_ref()
        .is_some_and(|value| value.is_empty())
    {
        anyhow::bail!("map_id must be non-empty when present");
    }
    Ok(())
}

fn same_request(
    existing: &TaskSpacePendingProviderAction,
    request: &EnqueueTaskSpacePendingProviderActionRequest,
) -> bool {
    existing.action_id == request.action_id
        && existing.origin_thread_id == request.origin_thread_id
        && existing.map_id == request.map_id
        && existing.provider_response_id == request.provider_response_id
        && existing.provider_action_key == request.provider_action_key
        && existing.tool_name == request.tool_name
        && existing.outcome == request.outcome
}
