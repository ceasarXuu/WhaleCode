use std::sync::Arc;

use codex_extension_api::ExtensionEventSink;
use codex_protocol::ThreadId;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::TaskSpaceUpdatedEvent;

#[derive(Clone)]
pub(crate) struct TaskSpaceEventEmitter {
    sink: Arc<dyn ExtensionEventSink>,
}

impl TaskSpaceEventEmitter {
    pub(crate) fn new(sink: Arc<dyn ExtensionEventSink>) -> Self {
        Self { sink }
    }

    pub(crate) fn updated(
        &self,
        thread_id: ThreadId,
        turn_id: &str,
        map_id: &str,
        revision: u64,
        operation: &str,
    ) {
        self.sink.emit(Event {
            id: format!("taskspace:{map_id}:{revision}"),
            msg: EventMsg::TaskSpaceUpdated(TaskSpaceUpdatedEvent {
                thread_id,
                turn_id: Some(turn_id.into()),
                map_id: map_id.into(),
                revision,
                operation: operation.into(),
            }),
        });
    }
}
