use crate::FreeformTool;
use crate::FreeformToolFormat;
use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolName;
use crate::ToolSpec;
use crate::ToolSpecCapabilityInput;
use crate::nested_tool_public_name;
use crate::project_tool_spec_capabilities;
use codex_code_mode::CodeModeToolKind;
use codex_code_mode::ToolDefinition as CodeModeToolDefinition;
use std::collections::BTreeMap;

/// Augment tool descriptions with code-mode-specific exec samples.
pub fn augment_tool_spec_for_code_mode(spec: ToolSpec) -> ToolSpec {
    match spec {
        ToolSpec::Function(mut tool) => {
            let Some(description) =
                augmented_description_for_spec(&ToolSpec::Function(tool.clone()))
            else {
                return ToolSpec::Function(tool);
            };
            tool.description = description;
            ToolSpec::Function(tool)
        }
        ToolSpec::Freeform(mut tool) => {
            let Some(description) =
                augmented_description_for_spec(&ToolSpec::Freeform(tool.clone()))
            else {
                return ToolSpec::Freeform(tool);
            };
            tool.description = description;
            ToolSpec::Freeform(tool)
        }
        ToolSpec::Namespace(mut namespace) => {
            for tool in &mut namespace.tools {
                match tool {
                    ResponsesApiNamespaceTool::Function(tool) => {
                        let tool_name =
                            ToolName::namespaced(namespace.name.clone(), tool.name.clone());
                        let definition = CodeModeToolDefinition {
                            name: code_mode_name_for_tool_name(&tool_name),
                            tool_name,
                            description: tool.description.clone(),
                            kind: CodeModeToolKind::Function,
                            input_schema: serde_json::to_value(&tool.parameters).ok(),
                            output_schema: tool.output_schema.clone(),
                        };
                        tool.description =
                            codex_code_mode::augment_tool_definition(definition).description;
                    }
                }
            }
            ToolSpec::Namespace(namespace)
        }
        other => other,
    }
}

/// Convert a supported nested tool spec into the code-mode runtime shape,
/// including the code-mode-specific description sample.
pub fn tool_spec_to_code_mode_tool_definition(spec: &ToolSpec) -> Option<CodeModeToolDefinition> {
    let definition = code_mode_tool_definition_for_spec(spec)?;
    codex_code_mode::is_code_mode_nested_tool(&definition.name)
        .then(|| codex_code_mode::augment_tool_definition(definition))
}

pub fn collect_code_mode_tool_definitions<'a>(
    specs: impl IntoIterator<Item = &'a ToolSpec>,
) -> Vec<CodeModeToolDefinition> {
    let mut tool_definitions = specs
        .into_iter()
        .flat_map(code_mode_tool_definitions_for_spec)
        .filter(|definition| codex_code_mode::is_code_mode_nested_tool(&definition.name))
        .map(codex_code_mode::augment_tool_definition)
        .collect::<Vec<_>>();
    tool_definitions.sort_by(|left, right| left.name.cmp(&right.name));
    tool_definitions.dedup_by(|left, right| left.name == right.name);
    tool_definitions
}

pub fn collect_code_mode_exec_prompt_tool_definitions<'a>(
    specs: impl IntoIterator<Item = &'a ToolSpec>,
) -> Vec<CodeModeToolDefinition> {
    let mut tool_definitions = specs
        .into_iter()
        .flat_map(code_mode_tool_definitions_for_spec)
        .filter(|definition| codex_code_mode::is_code_mode_nested_tool(&definition.name))
        .collect::<Vec<_>>();
    tool_definitions.sort_by(|left, right| left.name.cmp(&right.name));
    tool_definitions.dedup_by(|left, right| left.name == right.name);
    tool_definitions
}

pub fn create_wait_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "cell_id".to_string(),
            JsonSchema::string(Some("Identifier of the running exec cell.".to_string())),
        ),
        (
            "yield_time_ms".to_string(),
            JsonSchema::number(Some(
                "How long to wait (in milliseconds) for more output before yielding again."
                    .to_string(),
            )),
        ),
        (
            "max_tokens".to_string(),
            JsonSchema::number(Some(
                "Maximum number of output tokens to return for this wait call.".to_string(),
            )),
        ),
        (
            "terminate".to_string(),
            JsonSchema::boolean(Some(
                "Whether to terminate the running exec cell.".to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: codex_code_mode::WAIT_TOOL_NAME.to_string(),
        description: format!(
            "Waits on a yielded `{}` cell and returns new output or completion.\n{}",
            codex_code_mode::PUBLIC_TOOL_NAME,
            codex_code_mode::build_wait_tool_description().trim()
        ),
        strict: false,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["cell_id".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
        defer_loading: None,
    })
}

pub fn create_code_mode_tool(
    enabled_tools: &[CodeModeToolDefinition],
    namespace_descriptions: &BTreeMap<String, codex_code_mode::ToolNamespaceDescription>,
    code_mode_only: bool,
    exec_as_function: bool,
    deferred_tools_available: bool,
) -> ToolSpec {
    let description = if exec_as_function {
        codex_code_mode::build_function_exec_tool_description(
            enabled_tools,
            namespace_descriptions,
            code_mode_only,
            deferred_tools_available,
        )
    } else {
        codex_code_mode::build_exec_tool_description(
            enabled_tools,
            namespace_descriptions,
            code_mode_only,
            deferred_tools_available,
        )
    };
    if exec_as_function {
        return ToolSpec::Function(ResponsesApiTool {
            name: codex_code_mode::PUBLIC_TOOL_NAME.to_string(),
            description,
            strict: false,
            parameters: JsonSchema::object(
                BTreeMap::from([(
                    "source".to_string(),
                    JsonSchema::string(Some(
                        "JavaScript source to execute. Do not wrap it in Markdown fences."
                            .to_string(),
                    )),
                )]),
                Some(vec!["source".to_string()]),
                Some(false.into()),
            ),
            output_schema: None,
            defer_loading: None,
        });
    }

    const CODE_MODE_FREEFORM_GRAMMAR: &str = r#"
start: pragma_source | plain_source
pragma_source: PRAGMA_LINE NEWLINE SOURCE
plain_source: SOURCE

PRAGMA_LINE: /[ \t]*\/\/ @exec:[^\r\n]*/
NEWLINE: /\r?\n/
SOURCE: /[\s\S]+/
"#;

    ToolSpec::Freeform(FreeformTool {
        name: codex_code_mode::PUBLIC_TOOL_NAME.to_string(),
        description,
        format: FreeformToolFormat {
            r#type: "grammar".to_string(),
            syntax: "lark".to_string(),
            definition: CODE_MODE_FREEFORM_GRAMMAR.to_string(),
        },
    })
}

fn augmented_description_for_spec(spec: &ToolSpec) -> Option<String> {
    code_mode_tool_definition_for_spec(spec)
        .map(codex_code_mode::augment_tool_definition)
        .map(|definition| definition.description)
}

fn code_mode_tool_definition_for_spec(spec: &ToolSpec) -> Option<CodeModeToolDefinition> {
    code_mode_tool_definitions_for_spec(spec).into_iter().next()
}

fn code_mode_tool_definitions_for_spec(spec: &ToolSpec) -> Vec<CodeModeToolDefinition> {
    if matches!(
        spec,
        ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::WebSearch { .. }
    ) {
        return Vec::new();
    }
    project_tool_spec_capabilities(spec)
        .into_iter()
        .map(|capability| match capability.input.clone() {
            ToolSpecCapabilityInput::Function(parameters) => CodeModeToolDefinition {
                name: capability.public_name,
                tool_name: capability.tool_name,
                description: capability.description,
                kind: CodeModeToolKind::Function,
                input_schema: serde_json::to_value(parameters).ok(),
                output_schema: capability.output_schema,
            },
            ToolSpecCapabilityInput::Freeform(format) => {
                let description =
                    if codex_code_mode::is_code_mode_nested_tool(&capability.public_name) {
                        format!(
                            "{}\n\nNested exec input must match this {} grammar:\n```{}\n{}\n```",
                            capability.description.trim_end(),
                            format.syntax,
                            format.syntax,
                            format.definition.trim()
                        )
                    } else {
                        capability.description
                    };
                CodeModeToolDefinition {
                    name: capability.public_name,
                    tool_name: capability.tool_name,
                    description,
                    kind: CodeModeToolKind::Freeform,
                    input_schema: None,
                    output_schema: None,
                }
            }
        })
        .collect()
}

pub fn code_mode_name_for_tool_name(tool_name: &ToolName) -> String {
    nested_tool_public_name(tool_name)
}

#[cfg(test)]
#[path = "code_mode_tests.rs"]
mod tests;
