use super::Session;
use super::taskspace_store::canonical_map_for_store;
use super::taskspace_store::canonical_map_sha256;
use super::taskspace_store::runtime_from_record;
use crate::action_map::ActionMapRuntimeState;
use codex_protocol::ThreadId;
use codex_protocol::protocol::MapRuntimeMode;

impl Session {
    pub(crate) async fn read_canonical_action_map<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&ActionMapRuntimeState, ThreadId) -> T,
    ) -> Result<T, String> {
        let (mode, handle) = {
            let state = self.state.lock().await;
            (
                state.action_map_runtime.mode(),
                state.action_map_store_handle.clone(),
            )
        };
        let Some(handle) = handle else {
            if mode == MapRuntimeMode::Experiment {
                #[cfg(not(test))]
                {
                    return Err("TaskSpace read requires a canonical Map Store handle.".to_string());
                }
                #[cfg(test)]
                if self.services.state_db.is_some() {
                    return Err("TaskSpace read requires a canonical Map Store handle.".to_string());
                }
            }
            let state = self.state.lock().await;
            return Ok(read(&state.action_map_runtime, self.thread_id));
        };

        let (record, binding) = self
            .require_taskspace_state_db()?
            .load_taskspace_map_for_thread(self.thread_id)
            .await
            .map_err(|error| format!("TaskSpace Map Store read failed: {error}"))?
            .ok_or_else(|| {
                format!(
                    "TaskSpace Map Store has no canonical binding for thread `{}`.",
                    self.thread_id
                )
            })?;
        if record.map_id != handle.map_id {
            return Err(format!(
                "TaskSpace Map binding changed from `{}` to `{}`.",
                handle.map_id, record.map_id
            ));
        }
        let cache_is_current = {
            let state = self.state.lock().await;
            let cache_map = canonical_map_for_store(&state.action_map_runtime);
            let cache_sha256 = canonical_map_sha256(&cache_map)
                .map_err(|error| format!("TaskSpace cache hash failed: {error}"))?;
            state
                .action_map_store_handle
                .as_ref()
                .is_some_and(|current| {
                    current.map_id == record.map_id
                        && current.store_revision == record.store_revision
                        && current.canonical_sha256 == record.canonical_sha256
                        && cache_sha256 == record.canonical_sha256
                })
        };
        let cache_refreshed = if !cache_is_current {
            let runtime = runtime_from_record(&record).map_err(|error| error.to_string())?;
            self.install_store_record(&record, runtime).await?
        } else {
            false
        };
        tracing::debug!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_store_read",
            map_id = record.map_id,
            actor_thread_id = %self.thread_id,
            owner_thread_id = %record.owner_thread_id,
            relation = binding.relation.as_str(),
            store_revision = record.store_revision,
            map_revision = record
                .canonical_map
                .as_ref()
                .map_or(0, |map| map.revision),
            cache_refreshed,
            canonical_sha256 = record.canonical_sha256,
            operation,
            "read canonical TaskSpace Map"
        );
        let state = self.state.lock().await;
        Ok(read(&state.action_map_runtime, record.owner_thread_id))
    }
}
