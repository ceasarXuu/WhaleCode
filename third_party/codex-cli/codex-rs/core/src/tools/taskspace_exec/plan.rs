use codex_tools::ToolName;
use codex_tools::ToolSpecCapabilityInput;
use serde_json::Value;

use super::MapOperation;
use super::TaskSpaceExecCatalog;
use super::catalog::TaskSpaceClientCapability;
use super::catalog::TaskSpaceClientTransport;
use super::schema_validation::validate_json_schema;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPlan {
    pub(crate) sequence_type: String,
    pub(crate) pre_map: Vec<MapOperation>,
    pub(crate) actions: Vec<ClientCall>,
    pub(crate) terminal_map: Option<MapOperation>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClientCall {
    pub(crate) action_name: String,
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
pub(crate) enum TaskSpaceExecPlanDecodeError {
    InvalidJson(String),
    UnexpectedArgumentsField,
    InvalidEnvelope(String),
    UnknownAction { index: usize, action: String },
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
        let mut actions = Vec::new();
        let mut terminal_map = None;
        match sequence_type {
            "initialize_and_work" => {
                pre_map.push(decode_map(
                    "initialize_map",
                    field(&value, "initialize_map")?,
                )?);
                actions = decode_actions(&value, catalog)?;
            }
            "work" => actions = decode_actions(&value, catalog)?,
            "update_map" => pre_map.push(decode_map("update_map", field(&value, "update_map")?)?),
            "update_and_work" => {
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                actions = decode_actions(&value, catalog)?;
            }
            "update_and_finish" => {
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                terminal_map = Some(decode_map("finish_map", field(&value, "finish_map")?)?);
            }
            "read_map" => pre_map.push(decode_map("read_map", field(&value, "read_map")?)?),
            "reopen_update_and_work" => {
                pre_map.push(decode_map("reopen_map", field(&value, "reopen_map")?)?);
                pre_map.push(decode_map("update_map", field(&value, "update_map")?)?);
                actions = decode_actions(&value, catalog)?;
            }
            "finish_map" => {
                terminal_map = Some(decode_map("finish_map", field(&value, "finish_map")?)?)
            }
            _ => unreachable!("sequence type was validated by the closed schema"),
        }
        Ok(Self {
            sequence_type: sequence_type.to_string(),
            pre_map,
            actions,
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

fn decode_actions(
    plan: &Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<Vec<ClientCall>, TaskSpaceExecPlanDecodeError> {
    let actions = field(plan, "actions")?
        .as_array()
        .ok_or_else(|| invalid_envelope("`actions` must be an array"))?;
    actions
        .iter()
        .enumerate()
        .map(|(index, value)| decode_action(index, value, catalog))
        .collect()
}

fn decode_action(
    index: usize,
    value: &Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<ClientCall, TaskSpaceExecPlanDecodeError> {
    let action_name = string_field(value, "kind")?;
    match catalog.action_capability(action_name) {
        Some(capability) => decode_client(value, capability),
        None => Err(TaskSpaceExecPlanDecodeError::UnknownAction {
            index,
            action: action_name.to_string(),
        }),
    }
}

fn decode_client(
    value: &Value,
    capability: &TaskSpaceClientCapability,
) -> Result<ClientCall, TaskSpaceExecPlanDecodeError> {
    let input = field(value, "parameters")?;
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
        action_name: capability.action_name.clone(),
        tool_name: capability.capability.tool_name.clone(),
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
