use serde::{Deserialize, Serialize};
use whalecode_protocol::{AgentId, AgentRole, WorkflowPhase, WorkUnitId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    Never,
    OnRequest,
    OnFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow,
    Ask { reason: String },
    Deny { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionContext {
    pub agent_id: AgentId,
    pub role: AgentRole,
    pub phase: WorkflowPhase,
    pub work_unit_id: Option<WorkUnitId>,
    pub approval_policy: ApprovalPolicy,
}
