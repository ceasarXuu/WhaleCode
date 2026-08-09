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
struct RawNamespacedFunctionCall {
    tool: String,
    namespace: String,
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
    let namespace = match value.get("namespace") {
        None => None,
        Some(Value::String(namespace)) => Some(namespace.clone()),
        Some(_) => {
            return Err(TaskSpaceExecPlanDecodeError::InvalidCall {
                index,
                reason: "namespace must be a string when present".to_string(),
            });
        }
    };
    if namespace.is_none() && catalog.is_map_operation(&discriminator.tool) {
        return serde_json::from_value(value)
            .map(ExecCall::Map)
            .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidCall {
                index,
                reason: error.to_string(),
            });
    }
    let tool_name = ToolName::new(namespace, discriminator.tool.clone());
    let Some(capability) = catalog.client_capability(&tool_name) else {
        return Err(TaskSpaceExecPlanDecodeError::UnknownTool {
            index,
            tool: tool_label(&tool_name),
        });
    };
    let projected = &capability.capability;
    let (node_id, input) = match (&projected.tool_name.namespace, &projected.input) {
        (None, ToolSpecCapabilityInput::Function(_)) => {
            let raw = serde_json::from_value::<RawFunctionCall>(value).map_err(|error| {
                TaskSpaceExecPlanDecodeError::InvalidCall {
                    index,
                    reason: error.to_string(),
                }
            })?;
            debug_assert_eq!(raw.tool, projected.tool_name.name);
            (raw.node_id, ClientCallInput::Function(raw.arguments))
        }
        (Some(_), ToolSpecCapabilityInput::Function(_)) => {
            let raw =
                serde_json::from_value::<RawNamespacedFunctionCall>(value).map_err(|error| {
                    TaskSpaceExecPlanDecodeError::InvalidCall {
                        index,
                        reason: error.to_string(),
                    }
                })?;
            debug_assert_eq!(raw.tool, projected.tool_name.name);
            debug_assert_eq!(
                Some(raw.namespace.as_str()),
                projected.tool_name.namespace.as_deref()
            );
            (raw.node_id, ClientCallInput::Function(raw.arguments))
        }
        (None, ToolSpecCapabilityInput::Freeform(_)) => {
            let raw = serde_json::from_value::<RawFreeformCall>(value).map_err(|error| {
                TaskSpaceExecPlanDecodeError::InvalidCall {
                    index,
                    reason: error.to_string(),
                }
            })?;
            debug_assert_eq!(raw.tool, projected.tool_name.name);
            (raw.node_id, ClientCallInput::Freeform(raw.input))
        }
        (Some(_), ToolSpecCapabilityInput::Freeform(_)) => {
            unreachable!("namespaced freeform Tool")
        }
    };
    Ok(ExecCall::Client(ClientCall {
        display_name: tool_label(&projected.tool_name),
        tool_name: projected.tool_name.clone(),
        node_id,
        input,
        transport: capability.transport,
    }))
}

fn tool_label(tool_name: &ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) => format!("{namespace} / {}", tool_name.name),
        None => tool_name.name.clone(),
    }
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
