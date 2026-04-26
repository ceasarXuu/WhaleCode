use chrono::Utc;
use tempfile::tempdir;
use whalecode_protocol::{
    EventEnvelope, PrimitiveEvent, PrimitiveId, RedactionSummary, SessionEvent, SessionId,
    ToolCallId, ToolEvent, ToolStatus, TraceId, TranscriptEvent, TurnEvent, TurnFinishStatus,
};
use whalecode_session::{
    read_jsonl, replay_jsonl, JsonlSessionStore, SessionError, TranscriptRole,
};

fn event(sequence: u64, payload: SessionEvent) -> EventEnvelope<SessionEvent> {
    EventEnvelope {
        schema_version: 1,
        session_id: SessionId::from("session-1"),
        trace_id: TraceId::from("trace-1"),
        turn_id: None,
        sequence,
        occurred_at: Utc::now(),
        payload,
        redaction: RedactionSummary::default(),
    }
}

#[test]
fn appends_and_replays_jsonl_events_in_order() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    let mut store = JsonlSessionStore::open(&path).expect("open store");

    store
        .append(&event(
            1,
            SessionEvent::Transcript(TranscriptEvent::UserMessage {
                content: "hello".to_owned(),
            }),
        ))
        .expect("append first");
    store
        .append(&event(
            2,
            SessionEvent::Tool(ToolEvent::OutputRecorded {
                call_id: ToolCallId::from("tool-1"),
                artifact_id: whalecode_protocol::ArtifactId::from("tool-output-1"),
                summary: "ok".to_owned(),
                content_preview: "README.md".to_owned(),
                truncated: false,
            }),
        ))
        .expect("append second");
    store
        .append(&event(
            3,
            SessionEvent::Tool(ToolEvent::CallFinished {
                call_id: ToolCallId::from("tool-1"),
                status: ToolStatus::Succeeded,
                output_artifact: Some(whalecode_protocol::ArtifactId::from("tool-output-1")),
            }),
        ))
        .expect("append third");
    store
        .append(&event(
            4,
            SessionEvent::Turn(TurnEvent::Finished {
                index: 1,
                status: TurnFinishStatus::Completed,
            }),
        ))
        .expect("append fourth");

    let events = read_jsonl(&path).expect("read jsonl");
    assert_eq!(events.len(), 4);
    let replay = replay_jsonl(&path).expect("replay");
    assert_eq!(replay.event_count, 4);
    assert_eq!(replay.transcript.len(), 1);
    assert_eq!(replay.transcript[0].role, TranscriptRole::User);
    assert_eq!(replay.transcript[0].content, "hello");
    assert_eq!(replay.tool_event_count, 2);
    assert_eq!(replay.tool_output_count, 1);
    assert_eq!(replay.turn_event_count, 1);
}

#[test]
fn rejects_non_monotonic_sequence_on_append() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    let mut store = JsonlSessionStore::open(&path).expect("open store");
    let payload = SessionEvent::Transcript(TranscriptEvent::AssistantMessage {
        content: "done".to_owned(),
    });

    store.append(&event(2, payload.clone())).expect("append");
    let err = store.append(&event(2, payload)).expect_err("reject");
    assert!(matches!(err, SessionError::NonMonotonicSequence));
}

#[test]
fn reopens_with_existing_last_sequence() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    let payload = SessionEvent::Transcript(TranscriptEvent::AssistantMessage {
        content: "done".to_owned(),
    });

    {
        let mut store = JsonlSessionStore::open(&path).expect("open store");
        store
            .append(&event(1, payload.clone()))
            .expect("append first");
    }

    let mut reopened = JsonlSessionStore::open(&path).expect("reopen store");
    assert_eq!(reopened.last_sequence(), Some(1));
    reopened
        .append(&event(2, payload))
        .expect("append after reopen");

    let events = read_jsonl(&path).expect("read jsonl");
    assert_eq!(events.len(), 2);
}

#[test]
fn reports_malformed_jsonl_line() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    std::fs::write(&path, "{not-json}\n").expect("write malformed");

    let err = read_jsonl(&path).expect_err("malformed");
    assert!(matches!(err, SessionError::MalformedJson { line: 1, .. }));
}

#[test]
fn replays_primitive_enable_disable_state() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    let mut store = JsonlSessionStore::open(&path).expect("open store");
    let primitive_id = PrimitiveId::from("viewer");

    store
        .append(&event(
            1,
            SessionEvent::Primitive(PrimitiveEvent::Enabled {
                primitive_id: primitive_id.clone(),
            }),
        ))
        .expect("append enable");
    store
        .append(&event(
            2,
            SessionEvent::Primitive(PrimitiveEvent::Disabled {
                primitive_id: primitive_id.clone(),
            }),
        ))
        .expect("append disable");

    let replay = replay_jsonl(&path).expect("replay");
    assert!(!replay.enabled_primitives.contains(&primitive_id));
}

#[test]
fn registered_primitive_does_not_imply_enabled() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("session.jsonl");
    let mut store = JsonlSessionStore::open(&path).expect("open store");
    let primitive_id = PrimitiveId::from("reference-driven");

    store
        .append(&event(
            1,
            SessionEvent::Primitive(PrimitiveEvent::Registered {
                primitive_id: primitive_id.clone(),
            }),
        ))
        .expect("append register");

    let replay = replay_jsonl(&path).expect("replay");
    assert!(replay.registered_primitives.contains(&primitive_id));
    assert!(!replay.enabled_primitives.contains(&primitive_id));
}
