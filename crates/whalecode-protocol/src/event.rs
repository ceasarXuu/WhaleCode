use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ArtifactId, PrimitiveEvent, SessionId, ToolEvent, TraceId, TurnId, WorkflowPhase};

pub const EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionSummary {
    pub applied_rule_ids: Vec<String>,
    pub has_sensitive_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    pub schema_version: u32,
    pub session_id: SessionId,
    pub trace_id: TraceId,
    pub turn_id: Option<TurnId>,
    pub sequence: u64,
    pub occurred_at: DateTime<Utc>,
    pub payload: T,
    pub redaction: RedactionSummary,
}

impl<T> EventEnvelope<T> {
    pub fn new(session_id: SessionId, trace_id: TraceId, sequence: u64, payload: T) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            session_id,
            trace_id,
            turn_id: None,
            sequence,
            occurred_at: Utc::now(),
            payload,
            redaction: RedactionSummary::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", content = "data", rename_all = "snake_case")]
pub enum SessionEvent {
    Session(SessionLifecycleEvent),
    Transcript(TranscriptEvent),
    Model(ModelEvent),
    Tool(ToolEvent),
    Permission(PermissionEvent),
    Patch(PatchEvent),
    Primitive(PrimitiveEvent),
    Replay(ReplayEvent),
    Phase(PhaseEvent),
    Turn(TurnEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionLifecycleEvent {
    Started { cwd: String },
    Finished { status: SessionFinishStatus },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionFinishStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptEvent {
    UserMessage { content: String },
    AssistantMessage { content: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelEvent {
    RequestStarted { model: String },
    StreamDelta { delta: ModelStreamDelta },
    RequestFinished { usage: Option<ModelUsage> },
    RequestFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelStreamDelta {
    Text {
        content: String,
    },
    Reasoning {
        content: String,
    },
    ToolCall {
        name: String,
        arguments_delta: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionEvent {
    Decision {
        subject: String,
        decision: PermissionDecisionKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecisionKind {
    Allowed,
    Asked,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatchEvent {
    ArtifactCreated {
        artifact_id: ArtifactId,
        touched_files: Vec<String>,
    },
    ApplyResult {
        artifact_id: ArtifactId,
        status: PatchApplyStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchApplyStatus {
    Applied,
    Conflict,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReplayEvent {
    SnapshotBuilt { event_count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhaseEvent {
    Transition {
        from: WorkflowPhase,
        to: WorkflowPhase,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnEvent {
    Started {
        index: u64,
    },
    Finished {
        index: u64,
        status: TurnFinishStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnFinishStatus {
    Completed,
    Continued,
    Failed,
}
