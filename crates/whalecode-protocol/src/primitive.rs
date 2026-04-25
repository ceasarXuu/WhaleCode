use serde::{Deserialize, Serialize};

use crate::{GateId, PrimitiveId, WorkflowPhase};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PrimitiveEvent {
    Registered { primitive_id: PrimitiveId },
    Enabled { primitive_id: PrimitiveId },
    Disabled { primitive_id: PrimitiveId },
    EvalRecorded { primitive_id: PrimitiveId },
}
