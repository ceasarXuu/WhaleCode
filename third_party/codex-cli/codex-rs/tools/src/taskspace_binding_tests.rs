use std::collections::BTreeMap;

use super::*;
use crate::CommandToolOptions;
use crate::FreeformTool;
use crate::FreeformToolFormat;
use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::create_apply_patch_freeform_tool;
use crate::create_exec_command_tool;

#[test]
fn taskspace_and_standard_share_identical_ordinary_tool_schemas() {
    let tools = vec![
        create_exec_command_tool(CommandToolOptions {
            allow_login_shell: false,
            exec_permission_approvals_enabled: false,
        }),
        create_apply_patch_freeform_tool(),
        ToolSpec::ToolSearch {
            execution: "client".into(),
            description: "search tools".into(),
            parameters: JsonSchema::object(
                BTreeMap::from([(
                    "query".into(),
                    JsonSchema::string(Some("Search query.".into())),
                )]),
                Some(vec!["query".into()]),
                Some(false.into()),
            ),
        },
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
            name: "custom_grammar".into(),
            description: "custom grammar".into(),
            format: FreeformToolFormat {
                r#type: "grammar".into(),
                syntax: "lark".into(),
                definition: "start: WORD".into(),
            },
        }),
    ];

    for standard in tools {
        assert_eq!(
            project_taskspace_binding_tool(standard.clone()),
            Ok(TaskSpaceToolProjection::Visible(standard))
        );
    }
}

#[test]
fn projection_does_not_reserve_or_rewrite_a_business_field() {
    let standard = ToolSpec::Function(ResponsesApiTool {
        name: "business_tool".into(),
        description: "business field identity".into(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            BTreeMap::from([(
                TASKSPACE_BINDING_FIELD.into(),
                JsonSchema::string(Some("ordinary business value".into())),
            )]),
            Some(vec![TASKSPACE_BINDING_FIELD.into()]),
            Some(false.into()),
        ),
        output_schema: None,
    });

    assert_eq!(
        project_taskspace_binding_tool(standard.clone()),
        Ok(TaskSpaceToolProjection::Visible(standard))
    );
}

#[test]
fn taskspace_control_itself_is_returned_unchanged() {
    let control = crate::create_taskspace_control_tool();
    assert_eq!(
        project_taskspace_binding_tool(control.clone()),
        Ok(TaskSpaceToolProjection::Visible(control))
    );
}

#[test]
fn deferred_tool_search_results_keep_the_standard_schema() {
    let standard = LoadableToolSpec::Function(ResponsesApiTool {
        name: "deferred_tool".into(),
        description: "deferred".into(),
        strict: false,
        defer_loading: Some(true),
        parameters: JsonSchema::object(
            BTreeMap::from([("value".into(), JsonSchema::string(None))]),
            Some(vec!["value".into()]),
            Some(false.into()),
        ),
        output_schema: None,
    });

    assert_eq!(
        project_taskspace_binding_loadable_tool(standard.clone()),
        Ok(standard)
    );
}
