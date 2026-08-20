use crate::FreeformToolFormat;
use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ToolName;
use crate::ToolSpec;
use serde::Serialize;
use serde_json::Value;

/// A client-executable capability mechanically projected from a [`ToolSpec`].
///
/// Container-specific policy, such as excluding recursive `exec` calls, belongs
/// to the container that consumes this projection.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolSpecCapability {
    pub public_name: String,
    pub tool_name: ToolName,
    pub description: String,
    pub input: ToolSpecCapabilityInput,
    pub output_schema: Option<Value>,
    pub deferred: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "contract", rename_all = "snake_case")]
pub enum ToolSpecCapabilityInput {
    Function(JsonSchema),
    Freeform(FreeformToolFormat),
}

/// Project one provider-facing ToolSpec into zero or more schema-backed client
/// capabilities. Transport-specific container policy belongs to each caller.
pub fn project_tool_spec_capabilities(spec: &ToolSpec) -> Vec<ToolSpecCapability> {
    match spec {
        ToolSpec::Function(tool) => vec![ToolSpecCapability {
            public_name: tool.name.clone(),
            tool_name: ToolName::plain(tool.name.clone()),
            description: tool.description.clone(),
            input: ToolSpecCapabilityInput::Function(tool.parameters.clone()),
            output_schema: tool.output_schema.clone(),
            deferred: tool.defer_loading.unwrap_or(false),
        }],
        ToolSpec::Freeform(tool) => vec![ToolSpecCapability {
            public_name: tool.name.clone(),
            tool_name: ToolName::plain(tool.name.clone()),
            description: tool.description.clone(),
            input: ToolSpecCapabilityInput::Freeform(tool.format.clone()),
            output_schema: None,
            deferred: tool.defer_loading.unwrap_or(false),
        }],
        ToolSpec::Namespace(namespace) => namespace
            .tools
            .iter()
            .map(|tool| match tool {
                ResponsesApiNamespaceTool::Function(tool) => {
                    let tool_name = ToolName::namespaced(namespace.name.clone(), tool.name.clone());
                    ToolSpecCapability {
                        public_name: nested_tool_public_name(&tool_name),
                        tool_name,
                        description: tool.description.clone(),
                        input: ToolSpecCapabilityInput::Function(tool.parameters.clone()),
                        output_schema: tool.output_schema.clone(),
                        deferred: tool.defer_loading.unwrap_or(false),
                    }
                }
                ResponsesApiNamespaceTool::Custom(tool) => {
                    let tool_name = ToolName::namespaced(namespace.name.clone(), tool.name.clone());
                    ToolSpecCapability {
                        public_name: nested_tool_public_name(&tool_name),
                        tool_name,
                        description: tool.description.clone(),
                        input: ToolSpecCapabilityInput::Freeform(tool.format.clone()),
                        output_schema: None,
                        deferred: tool.defer_loading.unwrap_or(false),
                    }
                }
            })
            .collect(),
        ToolSpec::ToolSearch {
            description,
            parameters,
            ..
        } => vec![ToolSpecCapability {
            public_name: "tool_search".to_string(),
            tool_name: ToolName::plain("tool_search"),
            description: description.clone(),
            input: ToolSpecCapabilityInput::Function(parameters.clone()),
            output_schema: None,
            deferred: false,
        }],
        ToolSpec::WebSearch { .. } => Vec::new(),
    }
}

pub fn nested_tool_public_name(tool_name: &ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) if namespace.ends_with('_') || tool_name.name.starts_with('_') => {
            format!("{namespace}{}", tool_name.name)
        }
        Some(namespace) => format!("{namespace}_{}", tool_name.name),
        None => tool_name.name.clone(),
    }
}

#[cfg(test)]
#[path = "tool_spec_capability_tests.rs"]
mod tests;
