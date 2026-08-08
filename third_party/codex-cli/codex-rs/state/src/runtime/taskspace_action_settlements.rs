use super::StateRuntime;
use super::taskspace_map_codec::action_settlement_sha256;
use super::taskspace_map_codec::canonical_sha256;
use super::taskspace_map_codec::require_nonempty;
use super::taskspace_map_repository::compare_and_swap_map;
use super::taskspace_map_repository::load_map_in_tx;
use super::taskspace_maps::insert_commit;
use super::taskspace_maps::replay_commit;
use crate::SettleTaskSpaceActionRequest;
use crate::TaskSpaceMapWriteOutcome;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use std::collections::BTreeSet;
use std::time::Duration;
use std::time::Instant;

const INITIAL_BUSY_RETRY_DELAY: Duration = Duration::from_millis(10);
const MAX_BUSY_RETRY_DELAY: Duration = Duration::from_millis(500);

impl StateRuntime {
    /// Applies one observed Tool outcome without exposing a generic latest-head Map mutation.
    pub async fn settle_taskspace_action_outcome(
        &self,
        request: SettleTaskSpaceActionRequest,
    ) -> anyhow::Result<TaskSpaceMapWriteOutcome> {
        validate_request(&request)?;
        let mut delay = INITIAL_BUSY_RETRY_DELAY;
        let started = Instant::now();
        let mut attempt = 0_u64;
        loop {
            match self.settle_taskspace_action_once(&request).await {
                Ok(outcome) => return Ok(outcome),
                Err(error) if sqlite_busy_codes(&error).is_some() => {
                    attempt = attempt.saturating_add(1);
                    let (raw_code, primary_code) = sqlite_busy_codes(&error)
                        .expect("busy guard must provide SQLite result codes");
                    tracing::warn!(
                        target: "codex_state::taskspace",
                        event_name = "taskspace.action_settlement_store_busy",
                        map_id = request.map_id,
                        action_id = request.action_id,
                        attempt,
                        retry_delay_ms = delay.as_millis(),
                        elapsed_ms = started.elapsed().as_millis(),
                        sqlite_raw_code = raw_code,
                        sqlite_primary_code = primary_code,
                        "waiting to persist an observed TaskSpace Action outcome"
                    );
                    tokio::time::sleep(delay).await;
                    delay = delay.saturating_mul(2).min(MAX_BUSY_RETRY_DELAY);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn settle_taskspace_action_once(
        &self,
        request: &SettleTaskSpaceActionRequest,
    ) -> anyhow::Result<TaskSpaceMapWriteOutcome> {
        let request_sha256 = action_settlement_sha256(request)?;
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

        let current = load_map_in_tx(&mut tx, &request.map_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("TaskSpace map `{}` does not exist", request.map_id))?;
        let mut canonical_map = current.canonical_map.clone().ok_or_else(|| {
            anyhow::anyhow!("TaskSpace map `{}` is not initialized", request.map_id)
        })?;
        let changed = settle_action(&mut canonical_map, request)?;
        if changed {
            canonical_map.revision = canonical_map
                .revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("TaskSpace map revision overflow"))?;
        }
        let canonical_map = Some(canonical_map);
        let canonical_sha256 = canonical_sha256(&canonical_map)?;
        let result_store_revision = if changed {
            let next_revision = current
                .store_revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("TaskSpace store revision overflow"))?;
            if !compare_and_swap_map(
                &mut tx,
                &current,
                &canonical_map,
                next_revision,
                &canonical_sha256,
                now,
            )
            .await?
            {
                anyhow::bail!("TaskSpace Action settlement lost its write transaction");
            }
            next_revision
        } else {
            current.store_revision
        };
        insert_commit(
            &mut tx,
            &request.commit_id,
            &request.map_id,
            current.store_revision,
            result_store_revision,
            &canonical_sha256,
            &request_sha256,
            &request.operation,
            request.actor_thread_id,
            now,
        )
        .await?;
        let record = load_map_in_tx(&mut tx, &request.map_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("settled TaskSpace map disappeared before commit"))?;
        tx.commit().await?;
        if changed {
            Ok(TaskSpaceMapWriteOutcome::Applied(record))
        } else {
            Ok(TaskSpaceMapWriteOutcome::IdempotentReplay(record))
        }
    }
}

fn validate_request(request: &SettleTaskSpaceActionRequest) -> anyhow::Result<()> {
    require_nonempty("map_id", &request.map_id)?;
    require_nonempty("commit_id", &request.commit_id)?;
    require_nonempty("mutation_id", &request.mutation_id)?;
    require_nonempty("action_id", &request.action_id)?;
    require_nonempty("tool_name", &request.tool_name)?;
    require_nonempty("operation", &request.operation)?;
    if request.outcome == TaskSpaceActionOutcome::Pending {
        anyhow::bail!("TaskSpace Action settlement outcome must be terminal");
    }
    if request.node_ids.is_empty()
        || request
            .node_ids
            .iter()
            .any(|node_id| node_id.trim().is_empty())
    {
        anyhow::bail!("TaskSpace Action settlement must name non-empty Work Node ids");
    }
    let unique = request.node_ids.iter().collect::<BTreeSet<_>>();
    if unique.len() != request.node_ids.len() {
        anyhow::bail!("TaskSpace Action settlement contains duplicate Work Node ids");
    }
    Ok(())
}

fn settle_action(
    map: &mut TaskSpaceCanonicalMap,
    request: &SettleTaskSpaceActionRequest,
) -> anyhow::Result<bool> {
    let expected_nodes = request.node_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut actual_nodes = BTreeSet::new();
    let mut changed = false;
    for node in &mut map.work_nodes {
        for action in &mut node.actions {
            if action.action_id != request.action_id {
                continue;
            }
            actual_nodes.insert(node.node_id.clone());
            if action.tool_name != request.tool_name {
                anyhow::bail!(
                    "TaskSpace Action `{}` belongs to Tool `{}`, not `{}`",
                    request.action_id,
                    action.tool_name,
                    request.tool_name
                );
            }
            if action.outcome != TaskSpaceActionOutcome::Pending
                && action.outcome != request.outcome
            {
                anyhow::bail!(
                    "TaskSpace Action `{}` already has a different terminal outcome",
                    request.action_id
                );
            }
            if action.outcome != request.outcome {
                action.outcome = request.outcome;
                changed = true;
            }
        }
    }
    if actual_nodes != expected_nodes {
        anyhow::bail!(
            "TaskSpace Action `{}` node attribution mismatch: expected {:?}, found {:?}",
            request.action_id,
            expected_nodes,
            actual_nodes
        );
    }
    Ok(changed)
}

fn sqlite_busy_codes(error: &anyhow::Error) -> Option<(String, i32)> {
    error.chain().find_map(|cause| {
        let raw = cause
            .downcast_ref::<sqlx::Error>()
            .and_then(|error| match error {
                sqlx::Error::Database(database) => database.code(),
                _ => None,
            })?
            .into_owned();
        let primary = sqlite_primary_result_code(&raw)?;
        matches!(primary, 5 | 6).then_some((raw, primary))
    })
}

pub(super) fn sqlite_primary_result_code(raw: &str) -> Option<i32> {
    raw.parse::<i32>().ok().map(|code| code & 0xff)
}
