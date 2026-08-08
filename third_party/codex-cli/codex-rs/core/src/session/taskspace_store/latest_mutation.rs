use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeStoreCommittedEvent;
use codex_state::CommitLatestTaskSpaceFactRequest;
use codex_state::TaskSpaceMapWriteOutcome;
use uuid::Uuid;

use super::canonical_map_for_store;
use super::record_map_revision;
use super::runtime_from_record;
use crate::action_map::ActionMapRuntimeState;
use crate::session::session::Session;

impl Session {
    pub(crate) async fn commit_latest_canonical_action_fact(
        &self,
        operation: &'static str,
        correlation_id: &str,
        fact_id: &str,
        mutate: impl FnOnce(&mut ActionMapRuntimeState, ThreadId) -> Result<(), String>,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let _write_permit = self
            .taskspace_store_write_lock
            .acquire()
            .await
            .map_err(|_| "TaskSpace Store write serializer is closed.".to_string())?;
        let handle = {
            let mut state = self.state.lock().await;
            let Some(handle) = state.action_map_store_handle.clone() else {
                if state.action_map_runtime.mode() == MapRuntimeMode::Experiment {
                    #[cfg(not(test))]
                    return Err(
                        "TaskSpace operation requires a canonical Map Store handle.".to_string()
                    );
                    #[cfg(test)]
                    if self.services.state_db.is_some() {
                        return Err("TaskSpace operation requires a canonical Map Store handle."
                            .to_string());
                    }
                }
                mutate(&mut state.action_map_runtime, self.conversation_id)?;
                return Ok(Vec::new());
            };
            handle
        };
        let state_db = self.require_taskspace_state_db()?;
        let commit_id = Uuid::new_v4().to_string();
        let outcome = state_db
            .commit_latest_taskspace_fact(
                CommitLatestTaskSpaceFactRequest {
                    map_id: handle.map_id.clone(),
                    commit_id: commit_id.clone(),
                    mutation_id: fact_id.to_string(),
                    operation: operation.to_string(),
                    actor_thread_id: self.conversation_id,
                },
                move |record| {
                    let mut runtime = runtime_from_record(record)?;
                    mutate(&mut runtime, record.owner_thread_id).map_err(anyhow::Error::msg)?;
                    Ok(canonical_map_for_store(&runtime))
                },
            )
            .await
            .map_err(|error| format!("TaskSpace latest-head mutation failed: {error}"))?;
        let record = match outcome {
            TaskSpaceMapWriteOutcome::Applied(record)
            | TaskSpaceMapWriteOutcome::IdempotentReplay(record) => record,
            TaskSpaceMapWriteOutcome::Conflict { .. } => {
                return Err(
                    "TaskSpace latest-head mutation returned an impossible revision conflict."
                        .to_string(),
                );
            }
        };
        let installed_runtime = runtime_from_record(&record).map_err(|error| error.to_string())?;
        self.install_store_record(&record, installed_runtime)
            .await?;
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_store_latest_committed",
            map_id = record.map_id,
            actor_thread_id = %self.conversation_id,
            owner_thread_id = %record.owner_thread_id,
            store_revision = record.store_revision,
            map_revision = record_map_revision(&record),
            operation,
            commit_id,
            correlation_id,
            "committed factual TaskSpace mutation on the latest Map head"
        );
        Ok(vec![MapRuntimeEvent::StoreCommitted(
            MapRuntimeStoreCommittedEvent {
                map_id: record.map_id.clone(),
                store_revision: record.store_revision,
                map_revision: record_map_revision(&record),
                operation: operation.to_string(),
                actor_thread_id: self.conversation_id,
                canonical_sha256: record.canonical_sha256.clone(),
            },
        )])
    }
}
