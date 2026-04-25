use chrono::Utc;
use whalecode_protocol::{
    ArtifactId, EventEnvelope, ModelEvent, ModelStreamDelta, PatchApplyStatus, PatchEvent,
    PermissionDecisionKind, PermissionEvent, PrimitiveEvent, PrimitiveId, SessionEvent, SessionId,
    SessionLifecycleEvent, ToolCallId, ToolEvent, ToolStatus, TraceId, TranscriptEvent, TurnEvent,
    TurnFinishStatus,
};

fn envelope(payload: SessionEvent) -> EventEnvelope<SessionEvent> {
    EventEnvelope {
        schema_version: 1,
        session_id: SessionId::from("session-1"),
        trace_id: TraceId::from("trace-1"),
        turn_id: None,
        sequence: 1,
        occurred_at: Utc::now(),
        payload,
        redaction: Default::default(),
    }
}

fn assert_roundtrip(payload: SessionEvent) {
    let event = envelope(payload);
    let json = serde_json::to_string(&event).expect("serialize event");
    let decoded: EventEnvelope<SessionEvent> =
        serde_json::from_str(&json).expect("deserialize event");
    assert_eq!(decoded.payload, event.payload);
    assert_eq!(decoded.schema_version, 1);
}

#[test]
fn roundtrips_session_family() {
    assert_roundtrip(SessionEvent::Session(SessionLifecycleEvent::Started {
        cwd: "/repo".to_owned(),
    }));
}

#[test]
fn roundtrips_transcript_family() {
    assert_roundtrip(SessionEvent::Transcript(TranscriptEvent::UserMessage {
        content: "fix the bug".to_owned(),
    }));
}

#[test]
fn roundtrips_model_family() {
    assert_roundtrip(SessionEvent::Model(ModelEvent::StreamDelta {
        delta: ModelStreamDelta::ToolCall {
            name: "read_file".to_owned(),
            arguments_delta: "{\"path\"".to_owned(),
        },
    }));
}

#[test]
fn roundtrips_tool_family() {
    assert_roundtrip(SessionEvent::Tool(ToolEvent::CallFinished {
        call_id: ToolCallId::from("tool-1"),
        status: ToolStatus::Succeeded,
        output_artifact: Some(ArtifactId::from("artifact-1")),
    }));
    assert_roundtrip(SessionEvent::Tool(ToolEvent::OutputRecorded {
        call_id: ToolCallId::from("tool-1"),
        artifact_id: ArtifactId::from("artifact-1"),
        summary: "3 lines".to_owned(),
        content_preview: "README.md".to_owned(),
        truncated: false,
    }));
}

#[test]
fn roundtrips_permission_family() {
    assert_roundtrip(SessionEvent::Permission(PermissionEvent::Decision {
        subject: "shell".to_owned(),
        decision: PermissionDecisionKind::Denied,
    }));
}

#[test]
fn roundtrips_patch_family() {
    assert_roundtrip(SessionEvent::Patch(PatchEvent::ApplyResult {
        artifact_id: ArtifactId::from("patch-1"),
        status: PatchApplyStatus::Rejected,
    }));
}

#[test]
fn roundtrips_primitive_family() {
    assert_roundtrip(SessionEvent::Primitive(PrimitiveEvent::Enabled {
        primitive_id: PrimitiveId::from("reference-driven"),
    }));
}

#[test]
fn roundtrips_turn_family() {
    assert_roundtrip(SessionEvent::Turn(TurnEvent::Finished {
        index: 1,
        status: TurnFinishStatus::Completed,
    }));
}
