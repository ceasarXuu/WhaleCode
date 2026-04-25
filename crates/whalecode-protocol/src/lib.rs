use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

id_type!(AgentId);
id_type!(ArtifactId);
id_type!(GateId);
id_type!(PrimitiveId);
id_type!(SessionId);
id_type!(ToolCallId);
id_type!(TraceId);
id_type!(TurnId);
id_type!(WorkUnitId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    User,
    Supervisor,
    Implementer,
    Reviewer,
    Viewer,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowPhase {
    Idle,
    Analyze,
    Plan,
    Implement,
    Verify,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEvent {
    SessionStarted,
    UserMessage {
        content: String,
    },
    AssistantMessage {
        content: String,
    },
    ToolCallStarted {
        call_id: ToolCallId,
        tool_name: String,
    },
    ToolCallFinished {
        call_id: ToolCallId,
        status: ToolStatus,
    },
    PatchArtifactCreated {
        artifact_id: ArtifactId,
    },
    PrimitiveRegistered {
        primitive_id: PrimitiveId,
    },
    PrimitiveEnabled {
        primitive_id: PrimitiveId,
    },
    PrimitiveDisabled {
        primitive_id: PrimitiveId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    Succeeded,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionMode {
    ReadOnly,
    Write,
    LongRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub execution_mode: ToolExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveStability {
    Experimental,
    Beta,
    Stable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackPolicy {
    pub supported: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveManifest {
    pub id: PrimitiveId,
    pub name: String,
    pub version: SemVer,
    pub stability: PrimitiveStability,
    pub default_enabled: bool,
    pub dependencies: Vec<PrimitiveId>,
    pub conflicts: Vec<PrimitiveId>,
    pub rollback_policy: RollbackPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRef {
    pub name: String,
    pub version: u32,
}

pub type ArtifactSchemaRef = SchemaRef;
pub type EventSchemaRef = SchemaRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateSpec {
    pub id: GateId,
    pub phase: WorkflowPhase,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseHookSpec {
    pub name: String,
    pub phase: WorkflowPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverlaySpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayReducerSpec {
    pub event_name: String,
    pub snapshot_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerTriggerSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveEvalSpec {
    pub metric: String,
    pub success_condition: String,
}
