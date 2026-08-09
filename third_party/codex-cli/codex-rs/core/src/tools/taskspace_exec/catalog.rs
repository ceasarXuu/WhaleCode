use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use codex_tools::ToolSpecCapability;
use codex_tools::ToolSpecCapabilityInput;
use codex_tools::create_tools_json_for_responses_api;
use codex_tools::project_tool_spec_capabilities;
use serde::Serialize;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use std::sync::Arc;

use super::TaskSpaceExecPlan;
use super::TaskSpaceExecPlanDecodeError;
use super::map_operation_capabilities;
use super::protocol::build_description;

pub(crate) const TASKSPACE_EXEC_TOOL_NAME: &str = "taskspace_exec";
const RECURSIVE_TOOL_NAMES: [&str; 3] = [TASKSPACE_EXEC_TOOL_NAME, "exec", "wait"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskSpaceExecCatalog {
    declaration: ResponsesApiTool,
    capability_identity: Arc<str>,
    client_capabilities: BTreeMap<codex_tools::ToolName, TaskSpaceClientCapability>,
    map_capabilities: BTreeMap<String, ToolSpecCapability>,
    hosted_tools: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskSpaceClientTransport {
    Function,
    Freeform,
    ToolSearch,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct TaskSpaceClientCapability {
    pub(super) capability: ToolSpecCapability,
    pub(super) transport: TaskSpaceClientTransport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceExecCatalogError {
    DuplicateCapability { tool_name: codex_tools::ToolName },
    MapCapabilityCollision { public_name: String },
    UnsupportedToolSpec { tool_name: String },
    CapabilityIdentitySerialization { message: String },
}

impl TaskSpaceExecCatalog {
    pub(crate) fn build(specs: &[ToolSpec]) -> Result<Self, TaskSpaceExecCatalogError> {
        let mut client_capabilities = BTreeMap::new();
        let mut hosted_tools = BTreeSet::new();
        let mut hosted_specs = Vec::new();
        for spec in specs {
            match spec {
                ToolSpec::WebSearch { .. } | ToolSpec::ImageGeneration { .. } => {
                    hosted_tools.insert(spec.name().to_string());
                    hosted_specs.push(spec.clone());
                }
                ToolSpec::Function(_)
                | ToolSpec::Freeform(_)
                | ToolSpec::Namespace(_)
                | ToolSpec::ToolSearch { .. } => {
                    let transport = match spec {
                        ToolSpec::Freeform(_) => TaskSpaceClientTransport::Freeform,
                        ToolSpec::ToolSearch { .. } => TaskSpaceClientTransport::ToolSearch,
                        ToolSpec::Function(_) | ToolSpec::Namespace(_) => {
                            TaskSpaceClientTransport::Function
                        }
                        _ => unreachable!("matched client ToolSpec"),
                    };
                    for capability in project_tool_spec_capabilities(spec) {
                        if capability.tool_name.namespace.is_none()
                            && RECURSIVE_TOOL_NAMES.contains(&capability.tool_name.name.as_str())
                        {
                            continue;
                        }
                        let tool_name = capability.tool_name.clone();
                        if client_capabilities
                            .insert(
                                tool_name.clone(),
                                TaskSpaceClientCapability {
                                    capability,
                                    transport,
                                },
                            )
                            .is_some()
                        {
                            return Err(TaskSpaceExecCatalogError::DuplicateCapability {
                                tool_name,
                            });
                        }
                    }
                }
                ToolSpec::LocalShell {} => {
                    return Err(TaskSpaceExecCatalogError::UnsupportedToolSpec {
                        tool_name: spec.name().to_string(),
                    });
                }
            }
        }

        let map_capabilities = map_operation_capabilities()
            .into_iter()
            .map(|capability| (capability.public_name.clone(), capability))
            .collect::<BTreeMap<_, _>>();
        if let Some(public_name) = map_capabilities.keys().find(|name| {
            client_capabilities.contains_key(&codex_tools::ToolName::plain((*name).clone()))
        }) {
            return Err(TaskSpaceExecCatalogError::MapCapabilityCollision {
                public_name: public_name.clone(),
            });
        }

        let declaration = build_declaration(
            client_capabilities.values(),
            map_capabilities.values(),
            &hosted_tools,
        );
        let capability_identity = build_capability_identity(
            &declaration,
            &hosted_specs,
            &client_capabilities,
            &map_capabilities,
        )?;
        Ok(Self {
            declaration,
            capability_identity,
            client_capabilities,
            map_capabilities,
            hosted_tools,
        })
    }

    pub(crate) fn declaration(&self) -> &ResponsesApiTool {
        &self.declaration
    }

    pub(crate) fn capability_identity(&self) -> &str {
        &self.capability_identity
    }

    pub(crate) fn capability_identity_arc(&self) -> Arc<str> {
        Arc::clone(&self.capability_identity)
    }

    pub(crate) fn decode_plan(
        &self,
        arguments: &str,
    ) -> Result<TaskSpaceExecPlan, TaskSpaceExecPlanDecodeError> {
        TaskSpaceExecPlan::decode(arguments, self)
    }

    pub(super) fn client_capability(
        &self,
        tool_name: &codex_tools::ToolName,
    ) -> Option<&TaskSpaceClientCapability> {
        self.client_capabilities.get(tool_name)
    }

    pub(super) fn is_map_operation(&self, name: &str) -> bool {
        self.map_capabilities.contains_key(name)
    }

    pub(super) fn is_hosted_tool(&self, name: &str) -> bool {
        self.hosted_tools.contains(name)
    }
}

#[derive(Serialize)]
struct CapabilityIdentityInput<'a> {
    schema: &'static str,
    provider_declarations: Vec<serde_json::Value>,
    client_capabilities: Vec<&'a TaskSpaceClientCapability>,
    map_capabilities: &'a BTreeMap<String, ToolSpecCapability>,
}

fn build_capability_identity(
    declaration: &ResponsesApiTool,
    hosted_specs: &[ToolSpec],
    client_capabilities: &BTreeMap<codex_tools::ToolName, TaskSpaceClientCapability>,
    map_capabilities: &BTreeMap<String, ToolSpecCapability>,
) -> Result<Arc<str>, TaskSpaceExecCatalogError> {
    let provider_specs = std::iter::once(ToolSpec::Function(declaration.clone()))
        .chain(hosted_specs.iter().cloned())
        .collect::<Vec<_>>();
    let provider_declarations =
        create_tools_json_for_responses_api(&provider_specs).map_err(|error| {
            TaskSpaceExecCatalogError::CapabilityIdentitySerialization {
                message: error.to_string(),
            }
        })?;
    let bytes = serde_json::to_vec(&CapabilityIdentityInput {
        schema: "taskspace-capability-identity-v2",
        provider_declarations,
        client_capabilities: client_capabilities.values().collect(),
        map_capabilities,
    })
    .map_err(
        |error| TaskSpaceExecCatalogError::CapabilityIdentitySerialization {
            message: error.to_string(),
        },
    )?;
    Ok(Arc::from(format!("{:x}", Sha256::digest(bytes))))
}

fn build_declaration<'a>(
    clients: impl Iterator<Item = &'a TaskSpaceClientCapability>,
    map_operations: impl Iterator<Item = &'a ToolSpecCapability>,
    hosted_tools: &BTreeSet<String>,
) -> ResponsesApiTool {
    let clients = clients.collect::<Vec<_>>();
    let client_labels = clients
        .iter()
        .map(|client| client_tool_label(&client.capability.tool_name))
        .collect::<Vec<_>>();
    let call_variants = map_operations
        .map(map_call_schema)
        .chain(clients.iter().copied().map(client_call_schema))
        .collect::<Vec<_>>();
    let calls = JsonSchema::array(
        JsonSchema::object_any_of(
            call_variants,
            Some("Map operations and client Tool calls in Agent-declared order.".into()),
        ),
        None,
    );
    let hosted_bindings = JsonSchema::array(
        hosted_binding_schema(hosted_tools),
        Some("Bindings for provider-hosted outputs in provider output order.".into()),
    );
    ResponsesApiTool {
        name: TASKSPACE_EXEC_TOOL_NAME.to_string(),
        description: build_description(client_labels.iter().map(String::as_str), hosted_tools),
        strict: false,
        parameters: strict_object(
            [("calls", calls), ("hosted_bindings", hosted_bindings)],
            &["calls", "hosted_bindings"],
        ),
        output_schema: None,
        defer_loading: None,
    }
}

fn map_call_schema(capability: &ToolSpecCapability) -> JsonSchema {
    let ToolSpecCapabilityInput::Function(arguments) = &capability.input else {
        unreachable!("Map operations are structured Functions")
    };
    described(
        strict_object(
            [
                ("tool", exact_name_schema(&capability.public_name)),
                ("arguments", arguments.clone()),
            ],
            &["tool", "arguments"],
        ),
        &capability.description,
    )
}

fn client_call_schema(client: &TaskSpaceClientCapability) -> JsonSchema {
    let capability = &client.capability;
    let (input_name, input_schema) = match &capability.input {
        ToolSpecCapabilityInput::Function(arguments) => ("arguments", arguments.clone()),
        ToolSpecCapabilityInput::Freeform(format) => (
            "input",
            JsonSchema::string(Some(format!(
                "Freeform {} input using {} syntax.\n{}",
                format.r#type, format.syntax, format.definition
            ))),
        ),
    };
    let node_id = JsonSchema::string(Some("Agent-declared owner node.".into()));
    let schema = match capability.tool_name.namespace.as_deref() {
        Some(namespace) => strict_object(
            [
                ("tool", exact_name_schema(&capability.tool_name.name)),
                ("namespace", exact_name_schema(namespace)),
                ("node_id", node_id),
                (input_name, input_schema),
            ],
            &["tool", "namespace", "node_id", input_name],
        ),
        None => strict_object(
            [
                ("tool", exact_name_schema(&capability.tool_name.name)),
                ("node_id", node_id),
                (input_name, input_schema),
            ],
            &["tool", "node_id", input_name],
        ),
    };
    described(schema, &capability.description)
}

fn client_tool_label(tool_name: &codex_tools::ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) => format!("{namespace} / {}", tool_name.name),
        None => tool_name.name.clone(),
    }
}

fn hosted_binding_schema(hosted_tools: &BTreeSet<String>) -> JsonSchema {
    let tool_schema = if hosted_tools.is_empty() {
        JsonSchema::string(Some(
            "No provider-hosted Tool is available in this request.".into(),
        ))
    } else {
        JsonSchema::string_enum(
            hosted_tools.iter().map(|name| json!(name)).collect(),
            Some("Provider-hosted Tool type.".into()),
        )
    };
    strict_object(
        [
            ("tool", tool_schema),
            (
                "node_ids",
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Agent-declared owner nodes for this hosted output.".into()),
                )
                .with_min_items(1),
            ),
        ],
        &["tool", "node_ids"],
    )
}

fn exact_name_schema(name: &str) -> JsonSchema {
    JsonSchema::string_enum(vec![json!(name)], None)
}

fn described(mut schema: JsonSchema, description: &str) -> JsonSchema {
    schema.description = Some(description.to_string());
    schema
}

fn strict_object<const N: usize>(
    properties: impl IntoIterator<Item = (&'static str, JsonSchema)>,
    required: &[&str; N],
) -> JsonSchema {
    JsonSchema::object(
        properties
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect(),
        Some(required.iter().map(|name| (*name).to_string()).collect()),
        Some(AdditionalProperties::Boolean(false)),
    )
}
