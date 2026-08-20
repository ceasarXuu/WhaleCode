use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Weak;

use codex_protocol::ThreadId;

use crate::runtime::TaskSpaceMapRecord;
use crate::runtime::TaskSpaceRuntimeHandle;

#[derive(Debug, Clone)]
pub struct TaskSpaceServiceState {
    pub enabled: bool,
    pub record: Option<TaskSpaceMapRecord>,
}

#[derive(Default)]
pub struct TaskSpaceService {
    runtimes: RwLock<HashMap<ThreadId, Weak<TaskSpaceRuntimeHandle>>>,
}

impl TaskSpaceService {
    pub(crate) fn register(&self, runtime: &Arc<TaskSpaceRuntimeHandle>) {
        self.runtimes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(runtime.thread_id(), Arc::downgrade(runtime));
    }

    pub async fn set_enabled(&self, thread_id: ThreadId, enabled: bool) -> anyhow::Result<()> {
        let runtime = self.runtime(thread_id)?;
        runtime.set_enabled(enabled);
        Ok(())
    }

    pub async fn read(&self, thread_id: ThreadId) -> anyhow::Result<TaskSpaceServiceState> {
        let runtime = self.runtime(thread_id)?;
        let record = runtime.refresh().await?;
        Ok(TaskSpaceServiceState {
            enabled: runtime.is_enabled(),
            record,
        })
    }

    fn runtime(&self, thread_id: ThreadId) -> anyhow::Result<Arc<TaskSpaceRuntimeHandle>> {
        self.runtimes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&thread_id)
            .and_then(Weak::upgrade)
            .ok_or_else(|| anyhow::anyhow!("TaskSpace runtime is unavailable for `{thread_id}`"))
    }
}
