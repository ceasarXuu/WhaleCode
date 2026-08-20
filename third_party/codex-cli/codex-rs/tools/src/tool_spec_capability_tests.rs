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
    assert_eq!(capabilities[0].tool_name, ToolName::plain("exec"));
    assert_eq!(capabilities[0].public_name, "exec");
    assert!(capabilities[0].deferred);
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
        defer_loading: Some(true),
        format: format.clone(),
    }));
    assert_eq!(capabilities[0].description, "Apply one patch.");
    assert_eq!(
        capabilities[0].input,
        ToolSpecCapabilityInput::Freeform(format)
    );
    assert!(capabilities[0].deferred);
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
fn current_nested_public_name_is_not_a_reversible_identity() {
    let namespaced_left = ToolName::namespaced("alpha", "beta_gamma");
    let namespaced_right = ToolName::namespaced("alpha_beta", "gamma");
    let plain = ToolName::plain("alpha_beta_gamma");
    assert_ne!(namespaced_left, namespaced_right);
    assert_ne!(namespaced_left, plain);
    assert_eq!(
        nested_tool_public_name(&namespaced_left),
        nested_tool_public_name(&namespaced_right)
    );
    assert_eq!(
        nested_tool_public_name(&namespaced_left),
        nested_tool_public_name(&plain)
    );
}

#[test]
fn current_boundary_heuristic_also_collides() {
    let namespace_ends_with_separator = ToolName::namespaced("alpha_", "beta");
    let tool_starts_with_separator = ToolName::namespaced("alpha", "_beta");
    assert_ne!(namespace_ends_with_separator, tool_starts_with_separator);
    assert_eq!(
        nested_tool_public_name(&namespace_ends_with_separator),
        nested_tool_public_name(&tool_starts_with_separator)
    );
}

#[test]
fn projects_schema_backed_client_tool_search() {
    let parameters = JsonSchema::object(
        BTreeMap::from([("query".to_string(), JsonSchema::string(None))]),
        Some(vec!["query".to_string()]),
        Some(false.into()),
    );
    let capabilities = project_tool_spec_capabilities(&ToolSpec::ToolSearch {
        execution: "client".to_string(),
        description: "Search deferred tools.".to_string(),
        parameters: parameters.clone(),
    });
    assert_eq!(capabilities[0].public_name, "tool_search");
    assert_eq!(
        capabilities[0].input,
        ToolSpecCapabilityInput::Function(parameters)
    );
}

#[test]
fn excludes_provider_hosted_specs() {
    let capabilities = project_tool_spec_capabilities(&ToolSpec::WebSearch {
        external_web_access: Some(true),
        indexed_web_access: None,
        filters: None,
        user_location: None,
        search_context_size: None,
        search_content_types: None,
    });
    assert!(capabilities.is_empty());
}
