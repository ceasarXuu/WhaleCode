use std::collections::BTreeMap;

use crate::FreeformTool;
use crate::JsonSchema;
use crate::LoadableToolSpec;
use crate::ResponsesApiNamespace;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use crate::taskspace_tool::initialize_map_schema;
use serde_json::json;
use std::error::Error;
use std::fmt;

pub const TASKSPACE_BINDING_FIELD: &str = "taskspace_binding";

#[derive(Debug, Clone, PartialEq)]
pub enum TaskSpaceToolProjection {
    Visible(ToolSpec),
    Hidden {
        tool_name: String,
        tool_kind: &'static str,
        reason: &'static str,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaskSpaceToolProjectionError {
    pub tool_name: String,
    pub field: &'static str,
}

impl fmt::Display for TaskSpaceToolProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "tool `{}` already defines reserved TaskSpace field `{}`",
            self.tool_name, self.field
        )
    }
}

impl Error for TaskSpaceToolProjectionError {}

pub fn project_taskspace_binding_tool(
    spec: ToolSpec,
) -> Result<TaskSpaceToolProjection, TaskSpaceToolProjectionError> {
    Ok(match spec {
        ToolSpec::Function(tool) if tool.name != "taskspace_control" => {
            TaskSpaceToolProjection::Visible(ToolSpec::Function(decorate_function(tool)?))
        }
        ToolSpec::Function(tool) => TaskSpaceToolProjection::Visible(ToolSpec::Function(tool)),
        ToolSpec::Namespace(mut namespace) => {
            for member in &mut namespace.tools {
                match member {
                    ResponsesApiNamespaceTool::Function(tool) => {
                        *tool = decorate_function(tool.clone())?;
                    }
                }
            }
            TaskSpaceToolProjection::Visible(ToolSpec::Namespace(namespace))
        }
        ToolSpec::ToolSearch {
            execution,
            description,
            mut parameters,
        } => {
            decorate_parameters(&mut parameters, "tool_search")?;
            TaskSpaceToolProjection::Visible(ToolSpec::ToolSearch {
                execution,
                description,
                parameters,
            })
        }
        ToolSpec::Freeform(tool) => project_freeform(tool),
        ToolSpec::LocalShell {} => hidden("local_shell", "local_shell"),
        ToolSpec::ImageGeneration { .. } => hidden("image_generation", "image_generation"),
        ToolSpec::WebSearch { .. } => hidden("web_search", "web_search"),
    })
}

pub fn project_taskspace_binding_loadable_tool(
    spec: LoadableToolSpec,
) -> Result<LoadableToolSpec, TaskSpaceToolProjectionError> {
    match spec {
        LoadableToolSpec::Function(tool) => decorate_function(tool).map(LoadableToolSpec::Function),
        LoadableToolSpec::Namespace(namespace) => {
            decorate_namespace(namespace).map(LoadableToolSpec::Namespace)
        }
    }
}

fn decorate_namespace(
    mut namespace: ResponsesApiNamespace,
) -> Result<ResponsesApiNamespace, TaskSpaceToolProjectionError> {
    for member in &mut namespace.tools {
        match member {
            ResponsesApiNamespaceTool::Function(tool) => {
                *tool = decorate_function(tool.clone())?;
            }
        }
    }
    Ok(namespace)
}

fn decorate_function(
    mut tool: ResponsesApiTool,
) -> Result<ResponsesApiTool, TaskSpaceToolProjectionError> {
    decorate_parameters(&mut tool.parameters, &tool.name)?;
    Ok(tool)
}

fn decorate_parameters(
    parameters: &mut JsonSchema,
    tool_name: &str,
) -> Result<(), TaskSpaceToolProjectionError> {
    let properties = parameters.properties.get_or_insert_default();
    if properties.contains_key(TASKSPACE_BINDING_FIELD) {
        return Err(TaskSpaceToolProjectionError {
            tool_name: tool_name.to_string(),
            field: TASKSPACE_BINDING_FIELD,
        });
    }
    properties.insert(TASKSPACE_BINDING_FIELD.into(), taskspace_binding_schema());
    let required = parameters.required.get_or_insert_default();
    if !required
        .iter()
        .any(|field| field == TASKSPACE_BINDING_FIELD)
    {
        required.push(TASKSPACE_BINDING_FIELD.into());
    }
    Ok(())
}

fn taskspace_binding_schema() -> JsonSchema {
    JsonSchema::any_of(
        vec![
            JsonSchema::string_enum(
                vec![json!("active"), json!("after_boundary")],
                Some(
                    "active serves the existing current Work. after_boundary marks the ordinary Tool immediately following bind_node or complete_then_continue in the same response."
                        .into(),
                ),
            ),
            initialize_map_schema(),
        ],
        Some(
            "Bind this ordinary Tool to TaskSpace. Use active for current Work, after_boundary after a lifecycle boundary control, or the initialize_map object on the first real Tool."
                .to_string(),
        ),
    )
}

fn project_freeform(tool: FreeformTool) -> TaskSpaceToolProjection {
    let input_field = match tool.name.as_str() {
        "apply_patch" => "input",
        name if name == codex_code_mode::PUBLIC_TOOL_NAME => "source",
        _ => {
            return TaskSpaceToolProjection::Hidden {
                tool_name: tool.name,
                tool_kind: "custom",
                reason: "freeform tool cannot carry the TaskSpace binding contract",
            };
        }
    };
    let parameters = JsonSchema::object(
        BTreeMap::from([
            (
                input_field.into(),
                JsonSchema::string(Some(format!("The original raw {} input.", tool.name))),
            ),
            (TASKSPACE_BINDING_FIELD.into(), taskspace_binding_schema()),
        ]),
        Some(vec![input_field.into(), TASKSPACE_BINDING_FIELD.into()]),
        Some(false.into()),
    );
    TaskSpaceToolProjection::Visible(ToolSpec::Function(ResponsesApiTool {
        name: tool.name,
        description: tool.description,
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    }))
}

fn hidden(tool_name: &str, tool_kind: &'static str) -> TaskSpaceToolProjection {
    TaskSpaceToolProjection::Hidden {
        tool_name: tool_name.to_string(),
        tool_kind,
        reason: "provider-native tool cannot enter TaskSpace client preflight",
    }
}

#[cfg(test)]
#[path = "taskspace_binding_tests.rs"]
mod tests;
