use thiserror::Error;
use whalecode_protocol::{EventEnvelope, SessionEvent};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session event sequence must be monotonic")]
    NonMonotonicSequence,
    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Default)]
pub struct ReplaySnapshot {
    pub event_count: usize,
}

pub fn replay_events(
    events: &[EventEnvelope<SessionEvent>],
) -> Result<ReplaySnapshot, SessionError> {
    let mut previous = None;
    for event in events {
        if previous.is_some_and(|seq| event.sequence <= seq) {
            return Err(SessionError::NonMonotonicSequence);
        }
        previous = Some(event.sequence);
    }
    Ok(ReplaySnapshot {
        event_count: events.len(),
    })
}
