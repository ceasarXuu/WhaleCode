use super::Session;
use crate::StateDbHandle;
use crate::action_map::ActionMapRuntimeState;
use crate::action_map::ActionMapStoreHandle;
use crate::action_map::SetTaskSpaceModeOutcome;
use crate::action_map::snapshot_sha256;
use codex_protocol::ThreadId;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeStoreCommittedEvent;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_state::BindTaskSpaceMapRequest;
use codex_state::CommitTaskSpaceMapRequest;
use codex_state::CreateTaskSpaceMapRequest;
use codex_state::TaskSpaceMapRecord;
use codex_state::TaskSpaceMapRelation;
use codex_state::TaskSpaceMapWriteOutcome;
use uuid::Uuid;

pub(super) struct HydratedActionMapStore {
    pub(super) runtime: ActionMapRuntimeState,
    pub(super) handle: ActionMapStoreHandle,
}

pub(super) async fn hydrate_action_map_store(
    state_db: Option<&StateDbHandle>,
    thread_id: ThreadId,
    initial_history: &InitialHistory,
    session_source: &SessionSource,
    taskspace_policy_present: bool,
) -> anyhow::Result<Option<HydratedActionMapStore>> {
    let parent_binding = taskspace_policy_present
        .then(|| parent_map_binding(initial_history, session_source))
        .flatten();
    let requires_existing_map = taskspace_policy_present
        && (matches!(
            initial_history,
            InitialHistory::Resumed(_) | InitialHistory::Forked(_)
        ) || parent_binding.is_some());
    let Some(state_db) = state_db else {
        if requires_existing_map {
            anyhow::bail!(
                "TaskSpace Map Store is unavailable; resume, fork, and child sessions require a canonical Map binding."
            );
        }
        return Ok(None);
    };

    let mut loaded = state_db.load_taskspace_map_for_thread(thread_id).await?;
    if loaded.is_none()
        && let Some((parent_thread_id, relation)) = parent_binding
        && let Some((parent_map, _)) = state_db
            .load_taskspace_map_for_thread(parent_thread_id)
            .await?
    {
        state_db
            .bind_thread_to_taskspace_map(BindTaskSpaceMapRequest {
                thread_id,
                map_id: parent_map.map_id.clone(),
                relation,
                parent_thread_id: Some(parent_thread_id),
                node_id: None,
                lease_id: None,
            })
            .await?;
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_store_thread_bound",
            map_id = parent_map.map_id,
            actor_thread_id = %thread_id,
            owner_thread_id = %parent_map.owner_thread_id,
            parent_thread_id = %parent_thread_id,
            relation = relation.as_str(),
            store_revision = parent_map.store_revision,
            graph_revision = parent_map.graph_revision,
            "bound thread to canonical TaskSpace Map"
        );
        loaded = state_db.load_taskspace_map_for_thread(thread_id).await?;
    }

    let Some((record, binding)) = loaded else {
        if requires_existing_map {
            tracing::error!(
                target: "codex_core::taskspace",
                event_name = "taskspace.map_store_binding_missing",
                actor_thread_id = %thread_id,
                parent_thread_id = ?parent_binding.map(|(parent, _)| parent),
                relation = ?parent_binding.map(|(_, relation)| relation.as_str()),
                operation = "hydrate_taskspace",
                reason_code = "binding_missing",
                "TaskSpace session has no canonical Map binding"
            );
            anyhow::bail!("TaskSpace Map Store has no canonical binding for thread `{thread_id}`.");
        }
        return Ok(None);
    };
    let runtime = runtime_from_record(&record)?;
    tracing::info!(
        target: "codex_core::taskspace",
        event_name = "taskspace.map_store_loaded",
        map_id = record.map_id,
        actor_thread_id = %thread_id,
        owner_thread_id = %record.owner_thread_id,
        relation = binding.relation.as_str(),
        store_revision = record.store_revision,
        graph_revision = record.graph_revision,
        complete = record.complete,
        "loaded canonical TaskSpace Map"
    );
    Ok(Some(HydratedActionMapStore {
        runtime,
        handle: ActionMapStoreHandle::from(&record),
    }))
}

fn parent_map_binding(
    initial_history: &InitialHistory,
    session_source: &SessionSource,
) -> Option<(ThreadId, TaskSpaceMapRelation)> {
    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) => Some((*parent_thread_id, TaskSpaceMapRelation::Child)),
        _ if matches!(initial_history, InitialHistory::Forked(_)) => initial_history
            .forked_from_id()
            .map(|parent| (parent, TaskSpaceMapRelation::Fork)),
        _ => None,
    }
}

pub(super) fn runtime_from_record(
    record: &TaskSpaceMapRecord,
) -> anyhow::Result<ActionMapRuntimeState> {
    let mut runtime = ActionMapRuntimeState::default();
    runtime
        .restore_store_snapshot(
            &record.map_id,
            record.owner_thread_id,
            record.snapshot.clone(),
        )
        .map_err(anyhow::Error::msg)?;
    Ok(runtime)
}

impl Session {
    pub(crate) async fn set_persisted_action_map_mode(
        &self,
        mode: MapRuntimeMode,
    ) -> Result<(SetTaskSpaceModeOutcome, Vec<MapRuntimeEvent>), String> {
        let has_handle = self.state.lock().await.action_map_store_handle.is_some();
        if has_handle {
            return self
                .mutate_canonical_action_map("set_mode", move |runtime, principal| {
                    runtime.set_mode_for_session(mode, principal)
                })
                .await;
        }
        if mode != MapRuntimeMode::Experiment {
            let mut state = self.state.lock().await;
            return Ok(state
                .action_map_runtime
                .set_mode_for_session(mode, self.conversation_id));
        }

        let (candidate, outcome, mut events) = {
            let state = self.state.lock().await;
            let mut candidate = state.action_map_runtime.clone();
            let (outcome, events) =
                candidate.set_mode_for_session(MapRuntimeMode::Experiment, self.conversation_id);
            (candidate, outcome, events)
        };
        let map_id = outcome.active_map_id.clone().ok_or_else(|| {
            "TaskSpace activation did not create a mechanical Map identity.".to_string()
        })?;
        let state_db = self.require_taskspace_state_db()?;
        let (record, installed_runtime) = match state_db
            .create_taskspace_map(CreateTaskSpaceMapRequest {
                map_id: map_id.clone(),
                owner_thread_id: self.conversation_id,
                snapshot: candidate.snapshot(),
                commit_id: Uuid::new_v4().to_string(),
                operation: "activate_taskspace".to_string(),
            })
            .await
            .map_err(|error| format!("TaskSpace Map Store create failed: {error}"))?
        {
            TaskSpaceMapWriteOutcome::Applied(record) => (record, candidate),
            TaskSpaceMapWriteOutcome::IdempotentReplay(record) => {
                let runtime = runtime_from_record(&record).map_err(|error| error.to_string())?;
                (record, runtime)
            }
            TaskSpaceMapWriteOutcome::Conflict { current } => {
                return Err(format!(
                    "TaskSpace Map Store identity conflict for `{map_id}` at revision {:?}.",
                    current.map(|record| record.store_revision)
                ));
            }
        };
        self.install_store_record(&record, installed_runtime)
            .await?;
        events.push(MapRuntimeEvent::StoreCommitted(
            MapRuntimeStoreCommittedEvent {
                map_id: record.map_id.clone(),
                store_revision: record.store_revision,
                graph_revision: record.graph_revision,
                operation: "activate_taskspace".to_string(),
                actor_thread_id: self.conversation_id,
                snapshot_sha256: record.snapshot_sha256.clone(),
            },
        ));
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_store_created",
            map_id = record.map_id,
            actor_thread_id = %self.conversation_id,
            owner_thread_id = %record.owner_thread_id,
            store_revision = record.store_revision,
            graph_revision = record.graph_revision,
            operation = "activate_taskspace",
            "created canonical TaskSpace Map"
        );
        Ok((outcome, events))
    }

    pub(crate) async fn mutate_canonical_action_map<T>(
        &self,
        operation: &'static str,
        mutate: impl FnOnce(&mut ActionMapRuntimeState, ThreadId) -> (T, Vec<MapRuntimeEvent>),
    ) -> Result<(T, Vec<MapRuntimeEvent>), String> {
        self.mutate_canonical_action_map_with_binding(operation, None, mutate)
            .await
    }

    pub(crate) async fn mutate_canonical_action_map_with_binding<T>(
        &self,
        operation: &'static str,
        binding: Option<BindTaskSpaceMapRequest>,
        mutate: impl FnOnce(&mut ActionMapRuntimeState, ThreadId) -> (T, Vec<MapRuntimeEvent>),
    ) -> Result<(T, Vec<MapRuntimeEvent>), String> {
        {
            let mut state = self.state.lock().await;
            if state.action_map_store_handle.is_none() {
                if state.action_map_runtime.mode() == MapRuntimeMode::Experiment {
                    #[cfg(not(test))]
                    {
                        return Err("TaskSpace operation requires a canonical Map Store handle."
                            .to_string());
                    }
                    #[cfg(test)]
                    if self.services.state_db.is_some() {
                        return Err("TaskSpace operation requires a canonical Map Store handle."
                            .to_string());
                    }
                }
                return Ok(mutate(&mut state.action_map_runtime, self.conversation_id));
            }
        }

        let (candidate, handle, result, mut events, before_sha256, after_snapshot) = {
            let state = self.state.lock().await;
            let handle = state.action_map_store_handle.clone().ok_or_else(|| {
                "TaskSpace operation requires a canonical Map Store handle.".to_string()
            })?;
            let before_snapshot = state.action_map_runtime.snapshot();
            let before_sha256 = snapshot_sha256(&before_snapshot)
                .map_err(|error| format!("TaskSpace cache hash failed: {error}"))?;
            if before_sha256 != handle.snapshot_sha256 {
                return Err(format!(
                    "TaskSpace Runtime cache is stale for `{}`: handle hash mismatch.",
                    handle.map_id
                ));
            }
            let mut candidate = state.action_map_runtime.clone();
            let (result, events) = mutate(&mut candidate, handle.owner_thread_id);
            let after_snapshot = candidate.snapshot();
            (
                candidate,
                handle,
                result,
                events,
                before_sha256,
                after_snapshot,
            )
        };
        let after_sha256 = snapshot_sha256(&after_snapshot)
            .map_err(|error| format!("TaskSpace candidate hash failed: {error}"))?;
        if after_sha256 == before_sha256 && binding.is_none() {
            let mut state = self.state.lock().await;
            state.action_map_runtime = candidate;
            return Ok((result, events));
        }

        let state_db = self.require_taskspace_state_db()?;
        let commit_id = Uuid::new_v4().to_string();
        let outcome = state_db
            .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
                map_id: handle.map_id.clone(),
                expected_store_revision: handle.store_revision,
                snapshot: after_snapshot,
                commit_id: commit_id.clone(),
                operation: operation.to_string(),
                actor_thread_id: self.conversation_id,
                binding,
            })
            .await;
        let (record, installed_runtime) = match outcome {
            Ok(TaskSpaceMapWriteOutcome::Applied(record)) => (record, candidate),
            Ok(TaskSpaceMapWriteOutcome::IdempotentReplay(record)) => {
                let runtime = runtime_from_record(&record).map_err(|error| error.to_string())?;
                (record, runtime)
            }
            Ok(TaskSpaceMapWriteOutcome::Conflict { current }) => {
                self.refresh_after_store_failure(current.as_ref()).await?;
                let current_revision = current
                    .as_ref()
                    .map(|record| record.store_revision)
                    .unwrap_or_default();
                tracing::warn!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.map_store_conflict",
                    map_id = handle.map_id,
                    actor_thread_id = %self.conversation_id,
                    owner_thread_id = %handle.owner_thread_id,
                    expected_store_revision = handle.store_revision,
                    current_store_revision = current_revision,
                    operation,
                    commit_id,
                    reason_code = "store_revision_conflict",
                    "rejected stale TaskSpace Map commit"
                );
                return Err(format!(
                    "TaskSpace Map Store revision conflict for `{}`: expected {}, current {}.",
                    handle.map_id, handle.store_revision, current_revision
                ));
            }
            Err(error) => {
                self.refresh_after_store_failure(None).await?;
                tracing::error!(
                    target: "codex_core::taskspace",
                    event_name = "taskspace.map_store_integrity_failed",
                    map_id = handle.map_id,
                    actor_thread_id = %self.conversation_id,
                    owner_thread_id = %handle.owner_thread_id,
                    expected_store_revision = handle.store_revision,
                    operation,
                    commit_id,
                    reason_code = "store_commit_failed",
                    %error,
                    "TaskSpace Map Store commit failed"
                );
                return Err(format!("TaskSpace Map Store commit failed: {error}"));
            }
        };
        self.install_store_record(&record, installed_runtime)
            .await?;
        events.push(MapRuntimeEvent::StoreCommitted(
            MapRuntimeStoreCommittedEvent {
                map_id: record.map_id.clone(),
                store_revision: record.store_revision,
                graph_revision: record.graph_revision,
                operation: operation.to_string(),
                actor_thread_id: self.conversation_id,
                snapshot_sha256: record.snapshot_sha256.clone(),
            },
        ));
        tracing::info!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_store_committed",
            map_id = record.map_id,
            actor_thread_id = %self.conversation_id,
            owner_thread_id = %record.owner_thread_id,
            store_revision = record.store_revision,
            graph_revision = record.graph_revision,
            operation,
            commit_id,
            "committed canonical TaskSpace Map"
        );
        Ok((result, events))
    }

    pub(super) fn require_taskspace_state_db(&self) -> Result<StateDbHandle, String> {
        self.state_db().ok_or_else(|| {
            "TaskSpace requires the persistent Map Store; state DB is unavailable.".to_string()
        })
    }

    pub(super) async fn install_store_record(
        &self,
        record: &TaskSpaceMapRecord,
        candidate: ActionMapRuntimeState,
    ) -> Result<(), String> {
        let candidate_sha256 = snapshot_sha256(&candidate.snapshot())
            .map_err(|error| format!("TaskSpace candidate hash failed: {error}"))?;
        if candidate_sha256 != record.snapshot_sha256 {
            return Err("TaskSpace Store record does not match Runtime candidate.".to_string());
        }
        let mut state = self.state.lock().await;
        state.install_action_map_store_record(record, candidate);
        Ok(())
    }

    async fn refresh_after_store_failure(
        &self,
        current: Option<&TaskSpaceMapRecord>,
    ) -> Result<(), String> {
        let owned;
        let record = if let Some(current) = current {
            current
        } else {
            let map_id = self
                .state
                .lock()
                .await
                .action_map_store_handle
                .as_ref()
                .map(|handle| handle.map_id.clone())
                .ok_or_else(|| "TaskSpace Map Store handle disappeared.".to_string())?;
            owned = self
                .require_taskspace_state_db()?
                .load_taskspace_map(&map_id)
                .await
                .map_err(|error| format!("TaskSpace Map Store reload failed: {error}"))?
                .ok_or_else(|| format!("TaskSpace Map Store map `{map_id}` is missing."))?;
            &owned
        };
        let runtime = runtime_from_record(record).map_err(|error| error.to_string())?;
        self.state
            .lock()
            .await
            .install_action_map_store_record(record, runtime);
        Ok(())
    }
}
