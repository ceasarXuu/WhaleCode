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
    let TaskSpaceToolProjection::Visible(ToolSpec::Function(decorated)) =
        project_taskspace_binding_tool(original).expect("projection")
    else {
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
    assert!(serialized.contains("initialize_map"));
    assert!(
        !serialized.contains("\"type\":\"string\",\"description\":\"active serves"),
        "steady-state bindings must share the discriminated object shape"
    );
    assert!(
        serialized.len() < 1_900,
        "binding schema is {} bytes",
        serialized.len()
    );
    assert!(!serialized.contains("expected_revision"));
    assert!(serialized.contains("initial_work"));
    assert!(serialized.contains("edges"));

    let binding = actual
        .get("taskspace_binding")
        .expect("binding schema must be present");
    let variants = binding
        .any_of
        .as_ref()
        .expect("binding must remain a fixed union");
    assert_eq!(
        variants[0]
            .properties
            .as_ref()
            .and_then(|properties| properties.get("action"))
            .and_then(|action| action.enum_values.as_ref())
            .and_then(|values| values.first()),
        Some(&serde_json::json!("initialize_map")),
        "the fixed schema must present bootstrap before steady-state bindings"
    );
}

#[test]
fn taskspace_patch_projection_keeps_raw_patch_as_top_level_input() {
    let TaskSpaceToolProjection::Visible(ToolSpec::Function(projected)) =
        project_taskspace_binding_tool(create_apply_patch_freeform_tool()).expect("projection")
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
    let binding = serde_json::to_string(&properties["taskspace_binding"])
        .expect("serialize initialization carrier");
    assert!(binding.contains("initialize_map"));
    assert!(binding.contains("additional_work"));
}

#[test]
fn tool_search_requires_explicit_taskspace_binding() {
    let original = ToolSpec::ToolSearch {
        execution: "client".into(),
        description: "search tools".into(),
        parameters: JsonSchema::object(BTreeMap::new(), Some(Vec::new()), Some(false.into())),
    };
    let TaskSpaceToolProjection::Visible(ToolSpec::ToolSearch { parameters, .. }) =
        project_taskspace_binding_tool(original).expect("projection")
    else {
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
    assert_eq!(
        project_taskspace_binding_tool(control.clone()),
        Ok(TaskSpaceToolProjection::Visible(control))
    );
}

#[test]
fn provider_native_and_unknown_freeform_tools_are_hidden() {
    let cases = [
        ToolSpec::LocalShell {},
        ToolSpec::ImageGeneration {
            output_format: "png".into(),
        },
        ToolSpec::WebSearch {
            external_web_access: Some(true),
            filters: None,
            user_location: None,
            search_context_size: None,
            search_content_types: None,
        },
        ToolSpec::Freeform(FreeformTool {
            name: "unknown_custom".into(),
            description: "unsupported custom tool".into(),
            format: crate::FreeformToolFormat {
                r#type: "grammar".into(),
                syntax: "lark".into(),
                definition: "start: WORD".into(),
            },
        }),
    ];

    for spec in cases {
        assert!(matches!(
            project_taskspace_binding_tool(spec).expect("projection"),
            TaskSpaceToolProjection::Hidden { .. }
        ));
    }
}

#[test]
fn reserved_binding_collision_is_a_typed_error() {
    let mut properties = BTreeMap::new();
    properties.insert(
        TASKSPACE_BINDING_FIELD.into(),
        JsonSchema::string(Some("business field".into())),
    );
    let spec = ToolSpec::Function(ResponsesApiTool {
        name: "conflicting_tool".into(),
        description: "conflict".into(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, None, Some(false.into())),
        output_schema: None,
    });

    assert_eq!(
        project_taskspace_binding_tool(spec),
        Err(TaskSpaceToolProjectionError {
            tool_name: "conflicting_tool".into(),
            field: TASKSPACE_BINDING_FIELD,
        })
    );
}

#[test]
fn loadable_tools_use_the_same_binding_projection() {
    let spec = LoadableToolSpec::Function(ResponsesApiTool {
        name: "deferred_tool".into(),
        description: "deferred".into(),
        strict: false,
        defer_loading: Some(true),
        parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
        output_schema: None,
    });
    let LoadableToolSpec::Function(projected) =
        project_taskspace_binding_loadable_tool(spec).expect("projection")
    else {
        panic!("function must remain function");
    };
    assert!(
        projected
            .parameters
            .required
            .unwrap_or_default()
            .contains(&TASKSPACE_BINDING_FIELD.to_string())
    );
}
