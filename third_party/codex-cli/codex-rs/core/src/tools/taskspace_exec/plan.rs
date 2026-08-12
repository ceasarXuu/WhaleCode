use codex_tools::ToolName;
use codex_tools::ToolSpecCapabilityInput;
use serde_json::Value;

use super::MapOperation;
use super::TaskSpaceExecCatalog;
use super::catalog::TaskSpaceClientCapability;
use super::catalog::TaskSpaceClientTransport;
use super::catalog::TaskSpaceToolCapability;
use super::schema_validation::validate_json_schema;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPlan {
    pub(crate) sequence_type: String,
    pub(crate) pre_map: Vec<MapOperation>,
    pub(crate) tools: Vec<ToolAction>,
    pub(crate) terminal_map: Option<MapOperation>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ToolAction {
    Client(ClientCall),
    Hosted(ProviderAction),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClientCall {
    pub(crate) display_name: String,
    pub(crate) tool_name: ToolName,
    pub(crate) node_id: String,
    pub(crate) input: ClientCallInput,
    pub(crate) transport: TaskSpaceClientTransport,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClientCallInput {
    Function(Value),
    Freeform(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderAction {
    pub(crate) tool: String,
    pub(crate) node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecPlanDecodeError {
    InvalidJson(String),
    UnexpectedArgumentsField,
    InvalidEnvelope(String),
    UnknownTool { index: usize, tool: String },
    InvalidCall { index: usize, reason: String },
}

impl TaskSpaceExecPlan {
    pub(super) fn decode(
        arguments: &str,
        catalog: &TaskSpaceExecCatalog,
    ) -> Result<Self, TaskSpaceExecPlanDecodeError> {
        let value: Value = serde_json::from_str(arguments)
            .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidJson(error.to_string()))?;
        if value
            .as_object()
            .is_some_and(|object| object.contains_key("arguments"))
        {
            return Err(TaskSpaceExecPlanDecodeError::UnexpectedArgumentsField);
        }
        validate_json_schema(&value, catalog.input_schema()).map_err(|error| {
            TaskSpaceExecPlanDecodeError::InvalidEnvelope(format!(
                "{}: {}",
                error.path, error.reason
            ))
        })?;

        let sequence_type = string_field(&value, "type")?;
        let mut pre_map = Vec::new();
        let mut tools = Vec::new();
        let mut terminal_map = None;
        match sequence_type {
            "initialize_and_work" => {
                pre_map.push(decode_map(
                    "initialize_map",
                    field(&value, "initialize_map")?,
                )?);
                tools = decode_tools(&value, catalog)?;
            }
            "work" => tools = decode_tools(&value, catalog)?,
            "update_map" => pre_map.push(decode_map("update_map", field(&value, "update_map")?)?),
            "update_and_work" => {
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                tools = decode_tools(&value, catalog)?;
            }
            "update_and_finish" => {
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                terminal_map = Some(decode_map("finish_map", field(&value, "finish_map")?)?);
            }
            "read_map" => pre_map.push(decode_map("read_map", field(&value, "read_map")?)?),
            "reopen_update_and_work" => {
                pre_map.push(decode_map("reopen_map", field(&value, "reopen_map")?)?);
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                tools = decode_tools(&value, catalog)?;
            }
            "finish_map" => {
                terminal_map = Some(decode_map("finish_map", field(&value, "finish_map")?)?)
            }
            _ => unreachable!("sequence type was validated by the closed schema"),
        }
        Ok(Self {
            sequence_type: sequence_type.to_string(),
            pre_map,
            tools,
            terminal_map,
        })
    }
}

fn decode_map(
    operation: &str,
    input: &Value,
) -> Result<MapOperation, TaskSpaceExecPlanDecodeError> {
    serde_json::from_value(serde_json::json!({
        "tool": operation,
        "arguments": input,
    }))
    .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidCall {
        index: 0,
        reason: error.to_string(),
    })
}

fn decode_tools(
    plan: &Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<Vec<ToolAction>, TaskSpaceExecPlanDecodeError> {
    let tools = field(plan, "tools")?
        .as_array()
        .ok_or_else(|| invalid_envelope("`tools` must be an array"))?;
    tools
        .iter()
        .enumerate()
        .map(|(index, value)| decode_tool(index, value, catalog))
        .collect()
}

fn decode_tool(
    index: usize,
    value: &Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<ToolAction, TaskSpaceExecPlanDecodeError> {
    let namespace = value
        .get("namespace")
        .and_then(Value::as_str)
        .map(str::to_string);
    let name = string_field(value, "tool")?.to_string();
    let tool_name = ToolName::new(namespace, name);
    match catalog.tool_capability(&tool_name) {
        Some(TaskSpaceToolCapability::Client(capability)) => {
            decode_client(value, capability, tool_name).map(ToolAction::Client)
        }
        Some(TaskSpaceToolCapability::Hosted(_)) => {
            let node_ids = field(value, "node_ids")?
                .as_array()
                .ok_or_else(|| invalid_envelope("`node_ids` must be an array"))?
                .iter()
                .map(|node| {
                    node.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| invalid_envelope("node id must be a string"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ToolAction::Hosted(ProviderAction {
                tool: tool_name.name,
                node_ids,
            }))
        }
        None => Err(TaskSpaceExecPlanDecodeError::UnknownTool {
            index,
            tool: tool_label(&tool_name),
        }),
    }
}

fn decode_client(
    value: &Value,
    capability: &TaskSpaceClientCapability,
    tool_name: ToolName,
) -> Result<ClientCall, TaskSpaceExecPlanDecodeError> {
    let input = field(value, "input")?;
    let input = match &capability.capability.input {
        ToolSpecCapabilityInput::Function(_) => ClientCallInput::Function(input.clone()),
        ToolSpecCapabilityInput::Freeform(_) => ClientCallInput::Freeform(
            input
                .as_str()
                .ok_or_else(|| invalid_envelope("freeform Tool input must be a string"))?
                .to_string(),
        ),
    };
    Ok(ClientCall {
        display_name: tool_label(&tool_name),
        tool_name,
        node_id: string_field(value, "node_id")?.to_string(),
        input,
        transport: capability.transport,
    })
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, TaskSpaceExecPlanDecodeError> {
    value
        .get(name)
        .ok_or_else(|| invalid_envelope(format!("missing `{name}`")))
}

fn string_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, TaskSpaceExecPlanDecodeError> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| invalid_envelope(format!("`{name}` must be a string")))
}

fn invalid_envelope(reason: impl Into<String>) -> TaskSpaceExecPlanDecodeError {
    TaskSpaceExecPlanDecodeError::InvalidEnvelope(reason.into())
}

fn tool_label(tool_name: &ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) => format!("{namespace} / {}", tool_name.name),
        None => tool_name.name.clone(),
    }
}
