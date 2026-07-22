use super::*;
use crate::CommandToolOptions;
use crate::create_apply_patch_freeform_tool;
use crate::create_exec_command_tool;

#[test]
fn function_tool_gets_optional_transition_without_changing_business_fields() {
    let original = create_exec_command_tool(CommandToolOptions {
        allow_login_shell: false,
        exec_permission_approvals_enabled: false,
    });
    let ToolSpec::Function(original_tool) = original.clone() else {
        panic!("exec_command must be a function tool");
    };
    let ToolSpec::Function(decorated) = decorate_taskspace_carrier_tool(original) else {
        panic!("decorated exec_command must remain a function tool");
    };

    let mut expected = original_tool.parameters.properties.unwrap();
    let actual = decorated.parameters.properties.unwrap();
    assert_eq!(actual.get("cmd"), expected.get("cmd"));
    expected.insert("taskspace_transition".into(), taskspace_transition_schema());
    assert_eq!(actual, expected);
    assert!(
        !decorated
            .parameters
            .required
            .unwrap_or_default()
            .contains(&"taskspace_transition".to_string())
    );
}

#[test]
fn taskspace_patch_projection_keeps_raw_patch_as_top_level_input() {
    let ToolSpec::Function(projected) =
        decorate_taskspace_carrier_tool(create_apply_patch_freeform_tool())
    else {
        panic!("TaskSpace apply_patch must be projected to a function");
    };

    assert_eq!(projected.name, "apply_patch");
    assert_eq!(projected.parameters.required, Some(vec!["input".into()]));
    let properties = projected.parameters.properties.unwrap();
    assert!(properties.contains_key("input"));
    assert!(properties.contains_key("taskspace_transition"));
}

#[test]
fn control_tool_is_not_decorated() {
    let control = crate::create_taskspace_control_tool();
    assert_eq!(decorate_taskspace_carrier_tool(control.clone()), control);
}
