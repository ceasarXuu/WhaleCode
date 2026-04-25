use std::{fs, path::Path};

use chrono::Utc;
use whalecode_protocol::{EventEnvelope, SessionEvent, SessionId, TraceId};
use whalecode_session::JsonlSessionStore;

use crate::AgentError;

pub(crate) fn ensure_parent_dir(path: &Path) -> Result<(), AgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AgentError::CreateSessionDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

pub(crate) struct EventRecorder {
    store: JsonlSessionStore,
    session_id: SessionId,
    trace_id: TraceId,
    sequence: u64,
}

impl EventRecorder {
    pub(crate) fn open(path: &Path) -> Result<Self, AgentError> {
        Ok(Self {
            store: JsonlSessionStore::open(path)?,
            session_id: SessionId::from(format!("session-{}", Utc::now().timestamp_micros())),
            trace_id: TraceId::from(format!("trace-{}", Utc::now().timestamp_micros())),
            sequence: 0,
        })
    }

    pub(crate) fn append(&mut self, payload: SessionEvent) -> Result<(), AgentError> {
        self.sequence += 1;
        let event = EventEnvelope::new(
            self.session_id.clone(),
            self.trace_id.clone(),
            self.sequence,
            payload,
        );
        self.store.append(&event)?;
        Ok(())
    }

    pub(crate) fn next_sequence(&self) -> u64 {
        self.sequence + 1
    }

    pub(crate) fn events_written(&self) -> u64 {
        self.sequence
    }
}
