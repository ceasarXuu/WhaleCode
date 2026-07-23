use std::collections::BTreeMap;

use crate::FreeformTool;
use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use crate::taskspace_tool::taskspace_action_schema;

const ACTION_FIELD: &str = "taskspace_action";

pub fn decorate_taskspace_carrier_tool(spec: ToolSpec) -> ToolSpec {
    match spec {
        ToolSpec::Function(tool) if tool.name != "taskspace_control" => {
            ToolSpec::Function(decorate_function(tool))
        }
        ToolSpec::Namespace(mut namespace) => {
            for member in &mut namespace.tools {
                match member {
                    ResponsesApiNamespaceTool::Function(tool) => {
                        *tool = decorate_function(tool.clone());
                    }
                }
            }
            ToolSpec::Namespace(namespace)
        }
        ToolSpec::ToolSearch {
            execution,
            description,
            mut parameters,
        } => {
            decorate_parameters(&mut parameters, "tool_search");
            ToolSpec::ToolSearch {
                execution,
                description,
                parameters,
            }
        }
        ToolSpec::Freeform(tool) => project_freeform(tool),
        other => other,
    }
}

fn decorate_function(mut tool: ResponsesApiTool) -> ResponsesApiTool {
    decorate_parameters(&mut tool.parameters, &tool.name);
    tool
}

fn decorate_parameters(parameters: &mut JsonSchema, tool_name: &str) {
    let properties = parameters.properties.get_or_insert_default();
    assert!(
        !properties.contains_key(ACTION_FIELD),
        "reserved TaskSpace field collision in {}",
        tool_name
    );
    properties.insert(ACTION_FIELD.into(), taskspace_action_schema());
    let required = parameters.required.get_or_insert_default();
    if !required.iter().any(|field| field == ACTION_FIELD) {
        required.push(ACTION_FIELD.into());
    }
}

fn project_freeform(tool: FreeformTool) -> ToolSpec {
    let input_field = match tool.name.as_str() {
        "apply_patch" => "input",
        name if name == codex_code_mode::PUBLIC_TOOL_NAME => "source",
        _ => return ToolSpec::Freeform(tool),
    };
    let parameters = JsonSchema::object(
        BTreeMap::from([
            (
                input_field.into(),
                JsonSchema::string(Some(format!("The original raw {} input.", tool.name))),
            ),
            (ACTION_FIELD.into(), taskspace_action_schema()),
        ]),
        Some(vec![input_field.into(), ACTION_FIELD.into()]),
        Some(false.into()),
    );
    ToolSpec::Function(ResponsesApiTool {
        name: tool.name,
        description: tool.description,
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_carrier_tests.rs"]
mod tests;
