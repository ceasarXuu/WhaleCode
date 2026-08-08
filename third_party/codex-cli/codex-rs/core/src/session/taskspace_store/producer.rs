use std::future::Future;
use std::sync::Mutex;

use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;

use super::Session;

pub(super) struct TaskSpaceActionProducerTracker {
    accepting: Mutex<bool>,
    tasks: TaskTracker,
}

impl Default for TaskSpaceActionProducerTracker {
    fn default() -> Self {
        Self {
            accepting: Mutex::new(true),
            tasks: TaskTracker::new(),
        }
    }
}

impl TaskSpaceActionProducerTracker {
    fn spawn<F>(&self, future: F) -> Result<JoinHandle<F::Output>, String>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let accepting = self
            .accepting
            .lock()
            .expect("TaskSpace Action producer gate poisoned");
        if !*accepting {
            return Err("TaskSpace Action producer tracker is shutting down.".to_string());
        }
        Ok(self.tasks.spawn(future))
    }

    async fn close_and_wait(&self) {
        let producer_count = {
            let mut accepting = self
                .accepting
                .lock()
                .expect("TaskSpace Action producer gate poisoned");
            *accepting = false;
            self.tasks.close();
            self.tasks.len()
        };
        if producer_count > 0 {
            tracing::debug!(
                target: "codex_core::taskspace",
                event_name = "taskspace.action_producer_drain_started",
                producer_count,
                "waiting for TaskSpace Action producers"
            );
        }
        self.tasks.wait().await;
        if producer_count > 0 {
            tracing::debug!(
                target: "codex_core::taskspace",
                event_name = "taskspace.action_producer_drain_completed",
                producer_count,
                "TaskSpace Action producers drained"
            );
        }
    }
}

impl Session {
    pub(crate) fn spawn_taskspace_action_producer<F>(
        &self,
        future: F,
    ) -> Result<JoinHandle<F::Output>, String>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.taskspace_action_settlements.producers.spawn(future)
    }

    pub(crate) async fn finish_taskspace_action_producers(&self) {
        self.taskspace_action_settlements
            .producers
            .close_and_wait()
            .await;
    }
}
