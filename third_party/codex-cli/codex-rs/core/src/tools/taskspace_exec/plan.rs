use codex_tools::ToolName;
use codex_tools::ToolSpecCapabilityInput;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::MapOperation;
use super::TaskSpaceExecCatalog;
use super::catalog::TaskSpaceClientTransport;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecPlan {
    pub(crate) calls: Vec<ExecCall>,
    pub(crate) hosted_bindings: Vec<HostedBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExecCall {
    Map(MapOperation),
    Client(ClientCall),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClientCall {
    pub(crate) public_name: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostedBinding {
    pub(crate) tool: String,
    pub(crate) node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecPlanDecodeError {
    InvalidJson(String),
    EmptyPlan,
    UnknownTool { index: usize, tool: String },
    InvalidCall { index: usize, reason: String },
    InvalidHostedBinding { index: usize, reason: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlan {
    calls: Vec<Value>,
    hosted_bindings: Vec<Value>,
}

#[derive(Deserialize)]
struct ToolDiscriminator {
    tool: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFunctionCall {
    tool: String,
    node_id: String,
    arguments: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFreeformCall {
    tool: String,
    node_id: String,
    input: String,
}

impl TaskSpaceExecPlan {
    pub(super) fn decode(
        arguments: &str,
        catalog: &TaskSpaceExecCatalog,
    ) -> Result<Self, TaskSpaceExecPlanDecodeError> {
        let raw: RawPlan = serde_json::from_str(arguments)
            .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidJson(error.to_string()))?;
        if raw.calls.is_empty() && raw.hosted_bindings.is_empty() {
            return Err(TaskSpaceExecPlanDecodeError::EmptyPlan);
        }
        let calls = raw
            .calls
            .into_iter()
            .enumerate()
            .map(|(index, call)| decode_call(index, call, catalog))
            .collect::<Result<Vec<_>, _>>()?;
        let hosted_bindings = raw
            .hosted_bindings
            .into_iter()
            .enumerate()
            .map(|(index, binding)| decode_hosted_binding(index, binding, catalog))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            calls,
            hosted_bindings,
        })
    }
}

fn decode_call(
    index: usize,
    value: Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<ExecCall, TaskSpaceExecPlanDecodeError> {
    let discriminator =
        serde_json::from_value::<ToolDiscriminator>(value.clone()).map_err(|error| {
            TaskSpaceExecPlanDecodeError::InvalidCall {
                index,
                reason: error.to_string(),
            }
        })?;
    if catalog.is_map_operation(&discriminator.tool) {
        return serde_json::from_value(value)
            .map(ExecCall::Map)
            .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidCall {
                index,
                reason: error.to_string(),
            });
    }
    let Some(capability) = catalog.client_capability(&discriminator.tool) else {
        return Err(TaskSpaceExecPlanDecodeError::UnknownTool {
            index,
            tool: discriminator.tool,
        });
    };
    let projected = &capability.capability;
    let (public_name, node_id, input) = match &projected.input {
        ToolSpecCapabilityInput::Function(_) => {
            let raw = serde_json::from_value::<RawFunctionCall>(value).map_err(|error| {
                TaskSpaceExecPlanDecodeError::InvalidCall {
                    index,
                    reason: error.to_string(),
                }
            })?;
            (
                raw.tool,
                raw.node_id,
                ClientCallInput::Function(raw.arguments),
            )
        }
        ToolSpecCapabilityInput::Freeform(_) => {
            let raw = serde_json::from_value::<RawFreeformCall>(value).map_err(|error| {
                TaskSpaceExecPlanDecodeError::InvalidCall {
                    index,
                    reason: error.to_string(),
                }
            })?;
            (raw.tool, raw.node_id, ClientCallInput::Freeform(raw.input))
        }
    };
    Ok(ExecCall::Client(ClientCall {
        public_name,
        tool_name: projected.tool_name.clone(),
        node_id,
        input,
        transport: capability.transport,
    }))
}

fn decode_hosted_binding(
    index: usize,
    value: Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<HostedBinding, TaskSpaceExecPlanDecodeError> {
    let binding = serde_json::from_value::<HostedBinding>(value).map_err(|error| {
        TaskSpaceExecPlanDecodeError::InvalidHostedBinding {
            index,
            reason: error.to_string(),
        }
    })?;
    if !catalog.is_hosted_tool(&binding.tool) {
        return Err(TaskSpaceExecPlanDecodeError::InvalidHostedBinding {
            index,
            reason: format!("unknown provider-hosted Tool `{}`", binding.tool),
        });
    }
    Ok(binding)
}
