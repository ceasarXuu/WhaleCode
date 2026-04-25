use whalecode_permission::{
    ApprovalPolicy, PermissionContext, PermissionDecision, PermissionEngine, PermissionOperation,
    PermissionRequest,
};
use whalecode_protocol::{AgentId, AgentRole, WorkflowPhase};

fn context(phase: WorkflowPhase, approval_policy: ApprovalPolicy) -> PermissionContext {
    PermissionContext {
        agent_id: AgentId::from("agent-1"),
        role: AgentRole::Supervisor,
        phase,
        work_unit_id: None,
        approval_policy,
    }
}

#[test]
fn read_tools_are_allowed_in_analyze_phase() {
    let engine = PermissionEngine;
    let decision = engine.decide(
        &PermissionRequest {
            subject: "read README.md".to_owned(),
            operation: PermissionOperation::ReadFile {
                path: "README.md".to_owned(),
            },
        },
        &context(WorkflowPhase::Analyze, ApprovalPolicy::Never),
    );

    assert_eq!(decision, PermissionDecision::Allow);
}

#[test]
fn read_only_phase_rejects_write_before_approval() {
    let engine = PermissionEngine;
    let decision = engine.decide(
        &PermissionRequest {
            subject: "write src/lib.rs".to_owned(),
            operation: PermissionOperation::WriteFile {
                path: "src/lib.rs".to_owned(),
            },
        },
        &context(WorkflowPhase::Analyze, ApprovalPolicy::OnRequest),
    );

    assert!(matches!(decision, PermissionDecision::Deny { .. }));
}

#[test]
fn approval_policy_never_rejects_mutating_operations() {
    let engine = PermissionEngine;
    let decision = engine.decide(
        &PermissionRequest {
            subject: "shell cargo test".to_owned(),
            operation: PermissionOperation::Shell {
                command: vec!["cargo".to_owned(), "test".to_owned()],
            },
        },
        &context(WorkflowPhase::Implement, ApprovalPolicy::Never),
    );

    assert!(matches!(decision, PermissionDecision::Deny { .. }));
}

#[test]
fn mutating_operations_ask_when_policy_allows_prompting() {
    let engine = PermissionEngine;
    let decision = engine.decide(
        &PermissionRequest {
            subject: "write src/lib.rs".to_owned(),
            operation: PermissionOperation::WriteFile {
                path: "src/lib.rs".to_owned(),
            },
        },
        &context(WorkflowPhase::Implement, ApprovalPolicy::OnRequest),
    );

    assert!(matches!(decision, PermissionDecision::Ask { .. }));
}
