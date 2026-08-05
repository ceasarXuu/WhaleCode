use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use codex_code_mode::CodeModeToolKind;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::collect_code_mode_exec_prompt_tool_definitions;
use serde::Serialize;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use super::TASKSPACE_EXEC_PLAN_VERSION;
use super::TASKSPACE_EXEC_TOOL_NAME;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct TaskspaceExecCapability {
    pub(crate) public_name: String,
    pub(crate) tool_name: ToolName,
    pub(crate) description: String,
    pub(crate) kind: CodeModeToolKind,
    pub(crate) input_schema: Option<Value>,
    pub(crate) output_schema: Option<Value>,
    pub(crate) deferred: bool,
    pub(crate) canonical_source_spec: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TaskspaceExecCatalog {
    pub(crate) identity: String,
    pub(crate) capabilities: Vec<TaskspaceExecCapability>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskspaceExecCatalogError {
    DuplicatePublicName(String),
    InvalidToolSpec(String),
}

impl TaskspaceExecCatalog {
    pub(crate) fn from_tool_specs(specs: &[ToolSpec]) -> Result<Self, TaskspaceExecCatalogError> {
        let mut by_public_name = BTreeMap::new();
        for spec in specs {
            let canonical_source_spec = serde_json::to_value(spec)
                .map_err(|error| TaskspaceExecCatalogError::InvalidToolSpec(error.to_string()))?;
            for definition in collect_code_mode_exec_prompt_tool_definitions([spec]) {
                if definition.name == TASKSPACE_EXEC_TOOL_NAME {
                    continue;
                }
                let public_name = definition.name.clone();
                let capability = TaskspaceExecCapability {
                    deferred: is_deferred(spec, &definition.tool_name),
                    canonical_source_spec: canonical_source_spec.clone(),
                    public_name: definition.name,
                    tool_name: definition.tool_name,
                    description: definition.description,
                    kind: definition.kind,
                    input_schema: definition.input_schema,
                    output_schema: definition.output_schema,
                };
                match by_public_name.entry(public_name) {
                    Entry::Vacant(entry) => {
                        entry.insert(capability);
                    }
                    Entry::Occupied(entry) => {
                        return Err(TaskspaceExecCatalogError::DuplicatePublicName(
                            entry.key().clone(),
                        ));
                    }
                }
            }
        }
        let capabilities = by_public_name.into_values().collect::<Vec<_>>();

        let canonical = serde_json::to_vec(&capabilities)
            .expect("TaskSpace Exec capability catalog must serialize");
        let identity = format!("sha256:{:x}", Sha256::digest(canonical));
        Ok(Self {
            identity,
            capabilities,
        })
    }

    pub(crate) fn contains(&self, public_name: &str) -> bool {
        self.capabilities
            .binary_search_by_key(&public_name, |capability| capability.public_name.as_str())
            .is_ok()
    }
}

pub(crate) fn create_taskspace_exec_tool(catalog: &TaskspaceExecCatalog) -> ToolSpec {
    let names = catalog
        .capabilities
        .iter()
        .map(|capability| capability.public_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let description = format!(
        "Submit one complete TaskSpace action plan before any client Tool runs. The source must use {version}, capability identity `{identity}`, and may call only these mechanically derived client Tools: {names}. Provider-hosted results are declarations for reconciliation, not client executions.",
        version = TASKSPACE_EXEC_PLAN_VERSION,
        identity = catalog.identity,
    );

    ToolSpec::Function(ResponsesApiTool {
        name: TASKSPACE_EXEC_TOOL_NAME.to_string(),
        description,
        strict: false,
        parameters: JsonSchema::object(
            BTreeMap::from([(
                "source".to_string(),
                JsonSchema::string(Some(
                    "A complete declarative TaskSpace Exec plan. Do not wrap it in Markdown fences."
                        .to_string(),
                )),
            )]),
            Some(vec!["source".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
        defer_loading: None,
    })
}

fn is_deferred(spec: &ToolSpec, tool_name: &ToolName) -> bool {
    match spec {
        ToolSpec::Function(tool) => {
            tool_name.namespace.is_none()
                && tool.name == tool_name.name
                && tool.defer_loading.unwrap_or(false)
        }
        ToolSpec::Namespace(namespace) => {
            tool_name.namespace.as_deref() == Some(namespace.name.as_str())
                && namespace.tools.iter().any(|tool| match tool {
                    ResponsesApiNamespaceTool::Function(tool) => {
                        tool.name == tool_name.name && tool.defer_loading.unwrap_or(false)
                    }
                })
        }
        ToolSpec::ToolSearch { .. }
        | ToolSpec::LocalShell {}
        | ToolSpec::ImageGeneration { .. }
        | ToolSpec::WebSearch { .. }
        | ToolSpec::Freeform(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use codex_tools::FreeformTool;
    use codex_tools::FreeformToolFormat;
    use codex_tools::ResponsesApiNamespace;
    use codex_tools::ResponsesApiNamespaceTool;

    use super::*;

    fn function(name: &str, description: &str, deferred: bool) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: name.to_string(),
            description: description.to_string(),
            strict: false,
            parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
            output_schema: None,
            defer_loading: Some(deferred),
        })
    }

    fn fixture_specs() -> Vec<ToolSpec> {
        vec![
            function("read_file", "Read a file.", false),
            ToolSpec::Freeform(FreeformTool {
                name: "apply_patch".to_string(),
                description: "Apply one patch.".to_string(),
                format: FreeformToolFormat {
                    r#type: "grammar".to_string(),
                    syntax: "lark".to_string(),
                    definition: "start: /[\\s\\S]+/".to_string(),
                },
            }),
            ToolSpec::Namespace(ResponsesApiNamespace {
                name: "mcp".to_string(),
                description: "MCP tools.".to_string(),
                tools: vec![ResponsesApiNamespaceTool::Function(ResponsesApiTool {
                    name: "search".to_string(),
                    description: "Search MCP.".to_string(),
                    strict: false,
                    parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
                    output_schema: Some(serde_json::json!({"type": "object"})),
                    defer_loading: Some(true),
                })],
            }),
            ToolSpec::WebSearch {
                external_web_access: Some(true),
                filters: None,
                user_location: None,
                search_context_size: None,
                search_content_types: None,
            },
        ]
    }

    #[test]
    fn catalog_is_sorted_and_excludes_provider_hosted_specs() {
        let catalog =
            TaskspaceExecCatalog::from_tool_specs(&fixture_specs()).expect("valid catalog");

        assert_eq!(
            catalog
                .capabilities
                .iter()
                .map(|capability| capability.public_name.as_str())
                .collect::<Vec<_>>(),
            vec!["apply_patch", "mcp_search", "read_file"]
        );
        assert!(catalog.contains("mcp_search"));
        assert!(!catalog.contains("web_search"));
        assert!(catalog.capabilities[1].deferred);
    }

    #[test]
    fn identity_changes_for_every_executable_contract_axis() {
        let base = fixture_specs();
        let base_identity = TaskspaceExecCatalog::from_tool_specs(&base)
            .expect("valid catalog")
            .identity;

        let mut description = base.clone();
        if let ToolSpec::Function(tool) = &mut description[0] {
            tool.description.push_str(" Exact bytes.");
        }
        let mut input = base.clone();
        if let ToolSpec::Function(tool) = &mut input[0] {
            tool.parameters = JsonSchema::string(None);
        }
        let mut output = base.clone();
        if let ToolSpec::Namespace(namespace) = &mut output[2] {
            let ResponsesApiNamespaceTool::Function(tool) = &mut namespace.tools[0];
            tool.output_schema = Some(serde_json::json!({"type": "string"}));
        }
        let mut deferred = base.clone();
        if let ToolSpec::Namespace(namespace) = &mut deferred[2] {
            let ResponsesApiNamespaceTool::Function(tool) = &mut namespace.tools[0];
            tool.defer_loading = Some(false);
        }

        for changed in [description, input, output, deferred] {
            assert_ne!(
                TaskspaceExecCatalog::from_tool_specs(&changed)
                    .expect("valid changed catalog")
                    .identity,
                base_identity
            );
        }
    }

    #[test]
    fn tool_spec_uses_the_same_catalog_identity_and_keeps_source_only() {
        let catalog =
            TaskspaceExecCatalog::from_tool_specs(&fixture_specs()).expect("valid catalog");
        let ToolSpec::Function(tool) = create_taskspace_exec_tool(&catalog) else {
            panic!("TaskSpace Exec must be a Function Tool");
        };

        assert_eq!(tool.name, TASKSPACE_EXEC_TOOL_NAME);
        assert!(tool.description.contains(&catalog.identity));
        assert_eq!(
            serde_json::to_value(tool.parameters).expect("schema"),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "A complete declarative TaskSpace Exec plan. Do not wrap it in Markdown fences."
                    }
                },
                "required": ["source"],
                "additionalProperties": false
            })
        );
    }

    #[test]
    fn identity_covers_strict_freeform_grammar_and_namespace_description() {
        let base = fixture_specs();
        let base_identity = TaskspaceExecCatalog::from_tool_specs(&base)
            .expect("valid catalog")
            .identity;

        let mut strict = base.clone();
        if let ToolSpec::Function(tool) = &mut strict[0] {
            tool.strict = true;
        }
        let mut grammar = base.clone();
        if let ToolSpec::Freeform(tool) = &mut grammar[1] {
            tool.format.definition.push_str("\nextra: /x/");
        }
        let mut namespace = base.clone();
        if let ToolSpec::Namespace(tool) = &mut namespace[2] {
            tool.description.push_str(" Exact namespace bytes.");
        }

        for changed in [strict, grammar, namespace] {
            assert_ne!(
                TaskspaceExecCatalog::from_tool_specs(&changed)
                    .expect("valid changed catalog")
                    .identity,
                base_identity
            );
        }
    }

    #[test]
    fn duplicate_public_names_fail_closed() {
        let duplicate = vec![
            function("mcp_search", "Plain.", false),
            ToolSpec::Namespace(ResponsesApiNamespace {
                name: "mcp".to_string(),
                description: "MCP tools.".to_string(),
                tools: vec![ResponsesApiNamespaceTool::Function(ResponsesApiTool {
                    name: "search".to_string(),
                    description: "Namespaced.".to_string(),
                    strict: false,
                    parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
                    output_schema: None,
                    defer_loading: None,
                })],
            }),
        ];

        assert_eq!(
            TaskspaceExecCatalog::from_tool_specs(&duplicate),
            Err(TaskspaceExecCatalogError::DuplicatePublicName(
                "mcp_search".to_string()
            ))
        );
    }
}
