use super::*;
use crate::FreeformTool;
use crate::ResponsesApiNamespace;
use crate::ResponsesApiTool;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::BTreeMap;

fn function(name: &str) -> ResponsesApiTool {
    ResponsesApiTool {
        name: name.to_string(),
        description: format!("Run {name}."),
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([("value".to_string(), JsonSchema::string(None))]),
            Some(vec!["value".to_string()]),
            Some(false.into()),
        ),
        output_schema: Some(json!({"type": "string"})),
        defer_loading: Some(true),
    }
}

#[test]
fn projects_function_without_container_policy() {
    let capabilities = project_tool_spec_capabilities(&ToolSpec::Function(function("exec")));

    assert_eq!(
        capabilities,
        vec![ToolSpecCapability {
            public_name: "exec".to_string(),
            tool_name: ToolName::plain("exec"),
            description: "Run exec.".to_string(),
            input: ToolSpecCapabilityInput::Function(function("exec").parameters),
            output_schema: Some(json!({"type": "string"})),
            deferred: true,
        }]
    );
}

#[test]
fn projects_freeform_contract_without_rewriting_description() {
    let format = FreeformToolFormat {
        r#type: "grammar".to_string(),
        syntax: "lark".to_string(),
        definition: "start: /[a-z]+/".to_string(),
    };
    let capabilities = project_tool_spec_capabilities(&ToolSpec::Freeform(FreeformTool {
        name: "apply_patch".to_string(),
        description: "Apply one patch.".to_string(),
        format: format.clone(),
    }));

    assert_eq!(capabilities[0].description, "Apply one patch.");
    assert_eq!(
        capabilities[0].input,
        ToolSpecCapabilityInput::Freeform(format)
    );
}

#[test]
fn expands_namespace_and_preserves_native_identity() {
    let capabilities =
        project_tool_spec_capabilities(&ToolSpec::Namespace(ResponsesApiNamespace {
            name: "mcp__sample__".to_string(),
            description: "Sample tools.".to_string(),
            tools: vec![ResponsesApiNamespaceTool::Function(function("lookup"))],
        }));

    assert_eq!(capabilities[0].public_name, "mcp__sample__lookup");
    assert_eq!(
        capabilities[0].tool_name,
        ToolName::namespaced("mcp__sample__", "lookup")
    );
}

#[test]
fn excludes_provider_hosted_specs() {
    let capabilities = project_tool_spec_capabilities(&ToolSpec::WebSearch {
        external_web_access: Some(true),
        filters: None,
        user_location: None,
        search_context_size: None,
        search_content_types: None,
    });

    assert!(capabilities.is_empty());
}
