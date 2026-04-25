use serde::{Deserialize, Serialize};
use whalecode_protocol::{AgentId, AgentRole, WorkUnitId, WorkflowPhase};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    Never,
    OnRequest,
    OnFailure,
    PreApproved,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionOperation {
    ReadFile { path: String },
    SearchWorkspace { query: String },
    WriteFile { path: String },
    Shell { command: Vec<String> },
    Network { target: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub subject: String,
    pub operation: PermissionOperation,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionEngine;

impl PermissionEngine {
    pub fn decide(
        &self,
        request: &PermissionRequest,
        context: &PermissionContext,
    ) -> PermissionDecision {
        if is_read_only_phase(&context.phase) && !request.operation.is_read_only() {
            return PermissionDecision::Deny {
                reason: format!(
                    "{:?} phase only allows deterministic read tools",
                    context.phase
                ),
            };
        }

        if request.operation.is_read_only() {
            return PermissionDecision::Allow;
        }

        match context.approval_policy {
            ApprovalPolicy::Never => PermissionDecision::Deny {
                reason: "approval required by policy, but approval policy is never".to_owned(),
            },
            ApprovalPolicy::OnRequest | ApprovalPolicy::OnFailure => PermissionDecision::Ask {
                reason: format!("{} requires approval", request.subject),
            },
            ApprovalPolicy::PreApproved => PermissionDecision::Allow,
        }
    }
}

impl PermissionOperation {
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            PermissionOperation::ReadFile { .. } | PermissionOperation::SearchWorkspace { .. }
        )
    }
}

fn is_read_only_phase(phase: &WorkflowPhase) -> bool {
    matches!(phase, WorkflowPhase::Analyze | WorkflowPhase::Plan)
}
