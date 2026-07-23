use super::*;
use crate::CommandToolOptions;
use crate::create_apply_patch_freeform_tool;
use crate::create_exec_command_tool;

#[test]
fn function_tool_requires_lightweight_binding_without_changing_business_fields() {
    let original = create_exec_command_tool(CommandToolOptions {
        allow_login_shell: false,
        exec_permission_approvals_enabled: false,
    });
    let ToolSpec::Function(original_tool) = original.clone() else {
        panic!("exec_command must be a function tool");
    };
    let ToolSpec::Function(decorated) = decorate_taskspace_binding_tool(original) else {
        panic!("decorated exec_command must remain a function tool");
    };

    let mut expected = original_tool.parameters.properties.unwrap();
    let actual = decorated.parameters.properties.unwrap();
    assert_eq!(actual.get("cmd"), expected.get("cmd"));
    expected.insert("taskspace_binding".into(), taskspace_binding_schema());
    assert_eq!(actual, expected);
    assert!(
        decorated
            .parameters
            .required
            .unwrap_or_default()
            .contains(&"taskspace_binding".to_string())
    );
    let serialized =
        serde_json::to_string(actual.get("taskspace_binding").expect("binding schema"))
            .expect("serialize binding");
    assert!(serialized.contains("after_boundary"));
    assert!(
        serialized.len() < 320,
        "lightweight binding schema expanded to {} bytes",
        serialized.len()
    );
    assert!(!serialized.contains("expected_revision"));
    assert!(!serialized.contains("node_id"));
    assert!(!serialized.contains("edges"));
}

#[test]
fn taskspace_patch_projection_keeps_raw_patch_as_top_level_input() {
    let ToolSpec::Function(projected) =
        decorate_taskspace_binding_tool(create_apply_patch_freeform_tool())
    else {
        panic!("TaskSpace apply_patch must be projected to a function");
    };

    assert_eq!(projected.name, "apply_patch");
    assert_eq!(
        projected.parameters.required,
        Some(vec!["input".into(), "taskspace_binding".into()])
    );
    let properties = projected.parameters.properties.unwrap();
    assert!(properties.contains_key("input"));
    assert!(properties.contains_key("taskspace_binding"));
}

#[test]
fn tool_search_requires_explicit_taskspace_binding() {
    let original = ToolSpec::ToolSearch {
        execution: "client".into(),
        description: "search tools".into(),
        parameters: JsonSchema::object(BTreeMap::new(), Some(Vec::new()), Some(false.into())),
    };
    let ToolSpec::ToolSearch { parameters, .. } = decorate_taskspace_binding_tool(original) else {
        panic!("tool_search must remain a tool_search spec");
    };

    assert!(
        parameters
            .required
            .unwrap_or_default()
            .contains(&"taskspace_binding".to_string())
    );
    assert!(
        parameters
            .properties
            .unwrap_or_default()
            .contains_key("taskspace_binding")
    );
}

#[test]
fn control_tool_is_not_decorated() {
    let control = crate::create_taskspace_control_tool();
    assert_eq!(decorate_taskspace_binding_tool(control.clone()), control);
}
