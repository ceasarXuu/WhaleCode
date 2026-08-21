use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_protocol::ThreadId;
use tokio::sync::RwLock;

use crate::model::TaskSpaceMap;
use crate::transactions::Commit;
use crate::transactions::ReservationRelease;
use crate::transactions::ResultRefInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSpaceMapRelation {
    Owner,
    Resume,
    Fork,
    Child,
}

impl TaskSpaceMapRelation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Resume => "resume",
            Self::Fork => "fork",
            Self::Child => "child",
        }
    }

    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "owner" => Ok(Self::Owner),
            "resume" => Ok(Self::Resume),
            "fork" => Ok(Self::Fork),
            "child" => Ok(Self::Child),
            _ => anyhow::bail!("invalid TaskSpace map relation `{value}`"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpaceMapBinding {
    pub thread_id: ThreadId,
    pub map_id: String,
    pub relation: TaskSpaceMapRelation,
    pub parent_thread_id: Option<ThreadId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpaceMapRecord {
    pub map: TaskSpaceMap,
    pub owner_thread_id: ThreadId,
    pub canonical_sha256: String,
    pub store_revision: u64,
}

#[derive(Debug, Clone)]
pub struct TaskSpaceMapCommit {
    pub map: TaskSpaceMap,
    pub owner_thread_id: ThreadId,
    pub expected_store_revision: u64,
    pub commit_id: String,
    pub operation: String,
    pub actor_thread_id: ThreadId,
    pub binding: Option<TaskSpaceMapBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpaceMapWriteOutcome {
    Applied(TaskSpaceMapRecord),
    IdempotentReplay(TaskSpaceMapRecord),
    Conflict(Option<TaskSpaceMapRecord>),
}

pub type TaskSpaceStoreFuture<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

pub trait TaskSpaceStore: Send + Sync {
    fn load_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> TaskSpaceStoreFuture<'_, Option<(TaskSpaceMapRecord, TaskSpaceMapBinding)>>;

    fn bind(&self, binding: TaskSpaceMapBinding) -> TaskSpaceStoreFuture<'_, ()>;

    fn compare_and_swap(
        &self,
        commit: TaskSpaceMapCommit,
    ) -> TaskSpaceStoreFuture<'_, TaskSpaceMapWriteOutcome>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedAction {
    pub map_id: String,
    pub call_id: String,
    pub node_id: String,
    pub tool_name: String,
    pub reservation_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedControl {
    pub map_id: String,
    pub action: String,
    pub revision_before: u64,
    pub revision_after: u64,
    pub actions: Vec<PreparedAction>,
}

pub struct TaskSpaceRuntimeHandle {
    thread_id: ThreadId,
    store: Arc<dyn TaskSpaceStore>,
    enabled: AtomicBool,
    active: AtomicBool,
    record: RwLock<Option<TaskSpaceMapRecord>>,
    prepared_actions: RwLock<std::collections::HashMap<String, PreparedAction>>,
    prepared_controls: RwLock<std::collections::HashMap<String, PreparedControl>>,
    event_emitter: crate::event_emitter::TaskSpaceEventEmitter,
}

impl TaskSpaceRuntimeHandle {
    pub(crate) fn new(
        thread_id: ThreadId,
        store: Arc<dyn TaskSpaceStore>,
        event_emitter: crate::event_emitter::TaskSpaceEventEmitter,
    ) -> Self {
        Self {
            thread_id,
            store,
            enabled: AtomicBool::new(false),
            active: AtomicBool::new(false),
            record: RwLock::new(None),
            prepared_actions: RwLock::new(Default::default()),
            prepared_controls: RwLock::new(Default::default()),
            event_emitter,
        }
    }

    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    pub async fn record(&self) -> Option<TaskSpaceMapRecord> {
        self.record.read().await.clone()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    pub(crate) async fn refresh(&self) -> anyhow::Result<Option<TaskSpaceMapRecord>> {
        let record = self
            .store
            .load_for_thread(self.thread_id)
            .await?
            .map(|(record, _)| record);
        let was_active = self.active.swap(record.is_some(), Ordering::AcqRel);
        if record.is_some() && !was_active {
            self.enabled.store(true, Ordering::Release);
        }
        *self.record.write().await = record.clone();
        Ok(record)
    }

    pub(crate) async fn inherit(
        &self,
        parent_thread_id: ThreadId,
        relation: TaskSpaceMapRelation,
    ) -> anyhow::Result<bool> {
        if self.refresh().await?.is_some() {
            return Ok(false);
        }
        let Some((parent, _)) = self.store.load_for_thread(parent_thread_id).await? else {
            return Ok(false);
        };
        self.store
            .bind(TaskSpaceMapBinding {
                thread_id: self.thread_id,
                map_id: parent.map.map_id,
                relation,
                parent_thread_id: Some(parent_thread_id),
            })
            .await?;
        self.refresh().await?;
        Ok(true)
    }

    pub(crate) async fn commit_control(
        &self,
        control_call_id: &str,
        turn_id: &str,
        action: &str,
        operation: &str,
        commit: Commit,
        actions: Vec<PreparedAction>,
    ) -> anyhow::Result<PreparedControl> {
        let current =
            self.record.read().await.clone().ok_or_else(|| {
                anyhow::anyhow!("TaskSpace control requires an active canonical Map")
            })?;
        if commit.map.revision != current.map.revision.saturating_add(1) {
            anyhow::bail!("TaskSpace control was prepared from a stale Map revision");
        }
        let prepared = PreparedControl {
            map_id: commit.map.map_id.clone(),
            action: action.into(),
            revision_before: current.map.revision,
            revision_after: commit.map.revision,
            actions,
        };
        let outcome = self
            .store
            .compare_and_swap(TaskSpaceMapCommit {
                map: commit.map,
                owner_thread_id: current.owner_thread_id,
                expected_store_revision: current.store_revision,
                commit_id: format!("{}:{operation}:{control_call_id}", prepared.map_id),
                operation: operation.into(),
                actor_thread_id: self.thread_id,
                binding: None,
            })
            .await?;
        let record = applied_record(outcome)?;
        if self.store_record_if_newer(record).await {
            self.event_emitter.updated(
                self.thread_id,
                turn_id,
                &prepared.map_id,
                prepared.revision_after,
                action,
            );
        }
        self.remember_prepared(control_call_id, &prepared).await;
        Ok(prepared)
    }

    pub(crate) async fn commit_initialization(
        &self,
        control_call_id: &str,
        turn_id: &str,
        commit: Commit,
        actions: Vec<PreparedAction>,
    ) -> anyhow::Result<PreparedControl> {
        if self.record.read().await.is_some() {
            anyhow::bail!("TaskSpace initialization requires an unbound thread");
        }
        let prepared = PreparedControl {
            map_id: commit.map.map_id.clone(),
            action: "initialize_and_execute".into(),
            revision_before: 0,
            revision_after: commit.map.revision,
            actions,
        };
        let outcome = self
            .store
            .compare_and_swap(TaskSpaceMapCommit {
                map: commit.map,
                owner_thread_id: self.thread_id,
                expected_store_revision: 0,
                commit_id: format!("{}:initialize:{control_call_id}", prepared.map_id),
                operation: "initialize_and_execute".into(),
                actor_thread_id: self.thread_id,
                binding: None,
            })
            .await?;
        let record = applied_record(outcome)?;
        self.active.store(true, Ordering::Release);
        self.enabled.store(true, Ordering::Release);
        if self.store_record_if_newer(record).await {
            self.event_emitter.updated(
                self.thread_id,
                turn_id,
                &prepared.map_id,
                prepared.revision_after,
                "initialize_and_execute",
            );
        }
        self.remember_prepared(control_call_id, &prepared).await;
        Ok(prepared)
    }

    pub(crate) async fn prepared_control(&self, call_id: &str) -> Option<PreparedControl> {
        self.prepared_controls.write().await.remove(call_id)
    }

    pub(crate) async fn release_prepared(
        &self,
        call_id: &str,
        turn_id: &str,
        success: bool,
    ) -> anyhow::Result<()> {
        let Some(action) = self.prepared_actions.read().await.get(call_id).cloned() else {
            return Ok(());
        };
        let mut current =
            self.record.read().await.clone().ok_or_else(|| {
                anyhow::anyhow!("TaskSpace prepared action lost its canonical Map")
            })?;
        for _ in 0..4 {
            let commit = crate::transactions::release_reservation(
                &current.map,
                ReservationRelease {
                    expected_revision: current.map.revision,
                    reservation_id: action.reservation_id.clone(),
                    result_refs: vec![ResultRefInput {
                        result_ref_id: format!("{}:result:{call_id}", action.map_id),
                        is_error: !success,
                    }],
                    evidence_refs: Vec::new(),
                },
            )
            .map_err(|error| anyhow::anyhow!("TaskSpace release rejected: {error:?}"))?;
            match self
                .store
                .compare_and_swap(TaskSpaceMapCommit {
                    map: commit.map,
                    owner_thread_id: current.owner_thread_id,
                    expected_store_revision: current.store_revision,
                    commit_id: format!(
                        "{}:release:{}:{success}",
                        action.map_id, action.reservation_id
                    ),
                    operation: "action_release".into(),
                    actor_thread_id: self.thread_id,
                    binding: None,
                })
                .await?
            {
                TaskSpaceMapWriteOutcome::Applied(record)
                | TaskSpaceMapWriteOutcome::IdempotentReplay(record) => {
                    let revision = record.map.revision;
                    if self.store_record_if_newer(record).await {
                        self.event_emitter.updated(
                            self.thread_id,
                            turn_id,
                            &action.map_id,
                            revision,
                            "action_release",
                        );
                    }
                    self.prepared_actions.write().await.remove(call_id);
                    return Ok(());
                }
                TaskSpaceMapWriteOutcome::Conflict(Some(latest)) => current = latest,
                TaskSpaceMapWriteOutcome::Conflict(None) => {
                    anyhow::bail!("TaskSpace Map disappeared while releasing an action")
                }
            }
        }
        anyhow::bail!("TaskSpace action release exceeded its CAS retry limit")
    }

    async fn store_record_if_newer(&self, candidate: TaskSpaceMapRecord) -> bool {
        let mut record = self.record.write().await;
        let is_newer = record
            .as_ref()
            .is_none_or(|current| current.store_revision < candidate.store_revision);
        if is_newer {
            *record = Some(candidate);
        }
        is_newer
    }

    async fn remember_prepared(&self, control_call_id: &str, prepared: &PreparedControl) {
        self.prepared_controls
            .write()
            .await
            .insert(control_call_id.into(), prepared.clone());
        self.prepared_actions.write().await.extend(
            prepared
                .actions
                .iter()
                .cloned()
                .map(|action| (action.call_id.clone(), action)),
        );
    }
}

fn applied_record(outcome: TaskSpaceMapWriteOutcome) -> anyhow::Result<TaskSpaceMapRecord> {
    match outcome {
        TaskSpaceMapWriteOutcome::Applied(record)
        | TaskSpaceMapWriteOutcome::IdempotentReplay(record) => Ok(record),
        TaskSpaceMapWriteOutcome::Conflict(current) => anyhow::bail!(
            "TaskSpace state conflict at revision {:?}",
            current.map(|record| record.map.revision)
        ),
    }
}

impl std::fmt::Debug for TaskSpaceRuntimeHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskSpaceRuntimeHandle")
            .field("thread_id", &self.thread_id)
            .field("active", &self.is_active())
            .finish_non_exhaustive()
    }
}
