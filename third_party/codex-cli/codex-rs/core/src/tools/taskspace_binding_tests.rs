use super::*;
use crate::tools::context::ToolPayload;
use codex_tools::ToolName;

fn call(name: &str, binding: Option<&str>) -> ToolCall {
    ToolCall {
        tool_name: ToolName::plain(name),
        call_id: "call-1".into(),
        payload: ToolPayload::Function {
            arguments: "{}".into(),
        },
        taskspace_binding: binding.map(str::to_owned),
    }
}

#[test]
fn binding_failure_is_factual_and_never_claims_a_commit() {
    let value: serde_json::Value = serde_json::from_str(&binding_failure(
        "TASKSPACE_BINDING_INVALID",
        "taskspace_binding must be active or after_boundary",
        Some("other"),
    ))
    .expect("binding failure json");

    assert_eq!(
        value["schema_version"],
        "TaskSpaceBindingValidationResultV1"
    );
    assert_eq!(value["success"], false);
    assert_eq!(value["state_commit"], false);
    assert_eq!(value["submitted_binding"], "other");
    assert_eq!(value["error"]["code"], "TASKSPACE_BINDING_INVALID");
}

#[test]
fn only_taskspace_control_is_exempt_from_binding() {
    assert!(is_taskspace_control(&call("taskspace_control", None)));
    assert!(!requires_taskspace_binding(&call(
        "taskspace_control",
        None
    )));
    assert!(requires_taskspace_binding(&call("exec_command", None)));
}

#[test]
fn lightweight_binding_values_are_stable() {
    assert_eq!(ACTIVE_BINDING, "active");
    assert_eq!(AFTER_BOUNDARY_BINDING, "after_boundary");
}
