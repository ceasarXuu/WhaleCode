use codex_tools::ToolName;
use codex_tools::ToolSpecCapabilityInput;
use serde::Deserialize;
use serde::Deserializer;
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
    #[serde(default)]
    hosted_bindings: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMapEnvelope {
    map: RawMapCall,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMapCall {
    operation: String,
    input: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClientEnvelope {
    client: RawClientCall,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClientCall {
    name: String,
    #[serde(default, deserialize_with = "deserialize_namespace")]
    namespace: RawNamespace,
    node_id: String,
    input: Value,
}

#[derive(Default)]
enum RawNamespace {
    #[default]
    Missing,
    Present(String),
}

fn deserialize_namespace<'de, D>(deserializer: D) -> Result<RawNamespace, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(RawNamespace::Present)
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
    if value.get("map").is_some() {
        return decode_map_call(index, value, catalog);
    }
    if value.get("client").is_none() {
        return Err(TaskSpaceExecPlanDecodeError::InvalidCall {
            index,
            reason: "call must contain exactly one `map` or `client` envelope".to_string(),
        });
    }
    decode_client_call(index, value, catalog)
}

fn decode_map_call(
    index: usize,
    value: Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<ExecCall, TaskSpaceExecPlanDecodeError> {
    let raw = serde_json::from_value::<RawMapEnvelope>(value).map_err(|error| {
        TaskSpaceExecPlanDecodeError::InvalidCall {
            index,
            reason: error.to_string(),
        }
    })?;
    if !catalog.is_map_operation(&raw.map.operation) {
        return Err(TaskSpaceExecPlanDecodeError::UnknownTool {
            index,
            tool: raw.map.operation,
        });
    }
    serde_json::from_value(serde_json::json!({
        "tool": raw.map.operation,
        "arguments": raw.map.input,
    }))
    .map(ExecCall::Map)
    .map_err(|error| TaskSpaceExecPlanDecodeError::InvalidCall {
        index,
        reason: error.to_string(),
    })
}

fn decode_client_call(
    index: usize,
    value: Value,
    catalog: &TaskSpaceExecCatalog,
) -> Result<ExecCall, TaskSpaceExecPlanDecodeError> {
    let raw = serde_json::from_value::<RawClientEnvelope>(value).map_err(|error| {
        TaskSpaceExecPlanDecodeError::InvalidCall {
            index,
            reason: error.to_string(),
        }
    })?;
    let namespace = match &raw.client.namespace {
        RawNamespace::Missing => None,
        RawNamespace::Present(namespace) => Some(namespace.clone()),
    };
    let tool_name = ToolName::new(namespace, raw.client.name.clone());
    let Some(capability) = catalog.client_capability(&tool_name) else {
        return Err(TaskSpaceExecPlanDecodeError::UnknownTool {
            index,
            tool: tool_label(&tool_name),
        });
    };
    let projected = &capability.capability;
    let input = match (&projected.tool_name.namespace, &projected.input) {
        (_, ToolSpecCapabilityInput::Function(_)) => ClientCallInput::Function(raw.client.input),
        (None, ToolSpecCapabilityInput::Freeform(_)) => ClientCallInput::Freeform(
            raw.client
                .input
                .as_str()
                .ok_or_else(|| TaskSpaceExecPlanDecodeError::InvalidCall {
                    index,
                    reason: "freeform client input must be a string".to_string(),
                })?
                .to_string(),
        ),
        (Some(_), ToolSpecCapabilityInput::Freeform(_)) => {
            unreachable!("namespaced freeform Tool")
        }
    };
    Ok(ExecCall::Client(ClientCall {
        display_name: tool_label(&projected.tool_name),
        tool_name: projected.tool_name.clone(),
        node_id: raw.client.node_id,
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
