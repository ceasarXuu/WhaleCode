use crate::function_tool::FunctionCallError;
use crate::sandboxing::SandboxPermissions;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::AnyToolResult;
use crate::tools::registry::ToolArgumentDiffConsumer;
use crate::tools::registry::ToolRegistry;
use crate::tools::spec::build_specs_with_discoverable_tools;
use codex_mcp::ToolInfo;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::SearchToolCallParams;
use codex_protocol::models::ShellToolCallParams;
use codex_tools::ConfiguredToolSpec;
use codex_tools::DiscoverableTool;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::ToolsConfig;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

const NATIVE_READ_FILE_MAX_LINES: usize = 240;

pub use crate::tools::context::ToolCallSource;

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub tool_name: ToolName,
    pub call_id: String,
    pub payload: ToolPayload,
}

pub struct ToolRouter {
    registry: ToolRegistry,
    specs: Vec<ConfiguredToolSpec>,
    model_visible_specs: Vec<ToolSpec>,
    parallel_mcp_server_names: HashSet<String>,
}

pub(crate) struct ToolRouterParams<'a> {
    pub(crate) mcp_tools: Option<HashMap<String, ToolInfo>>,
    pub(crate) deferred_mcp_tools: Option<HashMap<String, ToolInfo>>,
    pub(crate) unavailable_called_tools: Vec<ToolName>,
    pub(crate) parallel_mcp_server_names: HashSet<String>,
    pub(crate) discoverable_tools: Option<Vec<DiscoverableTool>>,
    pub(crate) dynamic_tools: &'a [DynamicToolSpec],
}

impl ToolRouter {
    pub fn from_config(config: &ToolsConfig, params: ToolRouterParams<'_>) -> Self {
        let ToolRouterParams {
            mcp_tools,
            deferred_mcp_tools,
            unavailable_called_tools,
            parallel_mcp_server_names,
            discoverable_tools,
            dynamic_tools,
        } = params;
        let builder = build_specs_with_discoverable_tools(
            config,
            mcp_tools,
            deferred_mcp_tools,
            unavailable_called_tools,
            discoverable_tools,
            dynamic_tools,
        );
        let (specs, registry) = builder.build();
        let model_visible_specs = if config.code_mode_only_enabled {
            specs
                .iter()
                .filter_map(|configured_tool| {
                    if !codex_code_mode::is_code_mode_nested_tool(configured_tool.name()) {
                        Some(configured_tool.spec.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            specs
                .iter()
                .map(|configured_tool| configured_tool.spec.clone())
                .collect()
        };

        Self {
            registry,
            specs,
            model_visible_specs,
            parallel_mcp_server_names,
        }
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.specs
            .iter()
            .map(|config| config.spec.clone())
            .collect()
    }

    pub fn model_visible_specs(&self) -> Vec<ToolSpec> {
        self.model_visible_specs.clone()
    }

    pub fn find_spec(&self, tool_name: &ToolName) -> Option<ToolSpec> {
        self.specs.iter().find_map(|config| match &config.spec {
            ToolSpec::Function(tool)
                if tool_name.namespace.is_none() && tool.name == tool_name.name =>
            {
                Some(config.spec.clone())
            }
            ToolSpec::Freeform(tool)
                if tool_name.namespace.is_none() && tool.name == tool_name.name =>
            {
                Some(config.spec.clone())
            }
            ToolSpec::Namespace(namespace) => namespace.tools.iter().find_map(|tool| match tool {
                ResponsesApiNamespaceTool::Function(tool)
                    if tool_name.namespace.as_deref() == Some(namespace.name.as_str())
                        && tool.name == tool_name.name =>
                {
                    Some(ToolSpec::Function(tool.clone()))
                }
                _ => None,
            }),
            _ => None,
        })
    }

    pub(crate) fn create_diff_consumer(
        &self,
        tool_name: &ToolName,
    ) -> Option<Box<dyn ToolArgumentDiffConsumer>> {
        self.registry.create_diff_consumer(tool_name)
    }

    fn configured_tool_supports_parallel(&self, tool_name: &ToolName) -> bool {
        if tool_name.namespace.is_some() {
            return false;
        }

        self.specs
            .iter()
            .filter(|config| config.supports_parallel_tool_calls)
            .any(|config| match &config.spec {
                ToolSpec::Function(tool) => tool.name == tool_name.name.as_str(),
                ToolSpec::Freeform(tool) => tool.name == tool_name.name.as_str(),
                ToolSpec::Namespace(_)
                | ToolSpec::ToolSearch { .. }
                | ToolSpec::LocalShell {}
                | ToolSpec::ImageGeneration { .. }
                | ToolSpec::WebSearch { .. } => false,
            })
    }

    pub fn tool_supports_parallel(&self, call: &ToolCall) -> bool {
        match &call.payload {
            // MCP parallel support is configured per server, including for deferred
            // tools that may not have a matching spec entry. Use the parsed payload
            // server so similarly named servers/tools cannot collide.
            ToolPayload::Mcp { server, .. } => self.parallel_mcp_server_names.contains(server),
            _ => self.configured_tool_supports_parallel(&call.tool_name),
        }
    }

    #[instrument(level = "trace", skip_all, err)]
    pub async fn build_tool_call(
        session: &Session,
        item: ResponseItem,
    ) -> Result<Option<ToolCall>, FunctionCallError> {
        match item {
            ResponseItem::FunctionCall {
                name,
                namespace,
                arguments,
                call_id,
                ..
            } => {
                let tool_name = ToolName::new(namespace, name);
                if let Some(tool_info) = session.resolve_mcp_tool_info(&tool_name).await {
                    Ok(Some(ToolCall {
                        tool_name: tool_info.canonical_tool_name(),
                        call_id,
                        payload: ToolPayload::Mcp {
                            server: tool_info.server_name,
                            tool: tool_info.tool.name.to_string(),
                            raw_arguments: arguments,
                        },
                    }))
                } else {
                    let (tool_name, arguments) =
                        normalize_native_function_alias(tool_name, arguments)?;
                    Ok(Some(ToolCall {
                        tool_name,
                        call_id,
                        payload: ToolPayload::Function { arguments },
                    }))
                }
            }
            ResponseItem::ToolSearchCall {
                call_id: Some(call_id),
                execution,
                arguments,
                ..
            } if execution == "client" => {
                let arguments: SearchToolCallParams =
                    serde_json::from_value(arguments).map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "failed to parse tool_search arguments: {err}"
                        ))
                    })?;
                Ok(Some(ToolCall {
                    tool_name: ToolName::plain("tool_search"),
                    call_id,
                    payload: ToolPayload::ToolSearch { arguments },
                }))
            }
            ResponseItem::ToolSearchCall { .. } => Ok(None),
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => Ok(Some(ToolCall {
                tool_name: ToolName::plain(name),
                call_id,
                payload: ToolPayload::Custom { input },
            })),
            ResponseItem::LocalShellCall {
                id,
                call_id,
                action,
                ..
            } => {
                let call_id = call_id
                    .or(id)
                    .ok_or(FunctionCallError::MissingLocalShellCallId)?;

                match action {
                    LocalShellAction::Exec(exec) => {
                        let params = ShellToolCallParams {
                            command: exec.command,
                            workdir: exec.working_directory,
                            timeout_ms: exec.timeout_ms,
                            sandbox_permissions: Some(SandboxPermissions::UseDefault),
                            additional_permissions: None,
                            prefix_rule: None,
                            justification: None,
                        };
                        Ok(Some(ToolCall {
                            tool_name: ToolName::plain("local_shell"),
                            call_id,
                            payload: ToolPayload::LocalShell { params },
                        }))
                    }
                }
            }
            _ => Ok(None),
        }
    }

    #[instrument(level = "trace", skip_all, err)]
    pub async fn dispatch_tool_call_with_code_mode_result(
        &self,
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        cancellation_token: CancellationToken,
        tracker: SharedTurnDiffTracker,
        call: ToolCall,
        source: ToolCallSource,
    ) -> Result<AnyToolResult, FunctionCallError> {
        let ToolCall {
            tool_name,
            call_id,
            payload,
        } = call;

        let invocation = ToolInvocation {
            session,
            turn,
            cancellation_token,
            tracker,
            call_id,
            tool_name,
            source,
            payload,
        };

        self.registry.dispatch_any(invocation).await
    }
}

fn normalize_native_function_alias(
    tool_name: ToolName,
    arguments: String,
) -> Result<(ToolName, String), FunctionCallError> {
    if tool_name.namespace.is_some() {
        return Ok((tool_name, arguments));
    }

    match tool_name.name.as_str() {
        "exec_command" => Ok((
            ToolName::plain("shell_command"),
            normalize_exec_command_arguments(arguments)?,
        )),
        "read_file" => Ok((
            ToolName::plain("shell_command"),
            normalize_read_file_arguments(arguments)?,
        )),
        _ => Ok((tool_name, arguments)),
    }
}

fn normalize_exec_command_arguments(arguments: String) -> Result<String, FunctionCallError> {
    let mut value = parse_alias_arguments("exec_command", &arguments)?;
    let Some(object) = value.as_object_mut() else {
        return Err(FunctionCallError::RespondToModel(
            "failed to parse exec_command arguments: expected object".to_string(),
        ));
    };

    if let Some(command) = shell_command_text_from_alias_object(object) {
        object.insert("command".to_string(), JsonValue::String(command));
        Ok(value.to_string())
    } else {
        Err(FunctionCallError::RespondToModel(
            "failed to parse exec_command arguments: missing `cmd` or `command`".to_string(),
        ))
    }
}

fn normalize_read_file_arguments(arguments: String) -> Result<String, FunctionCallError> {
    let value = parse_alias_arguments("read_file", &arguments)?;
    let Some(object) = value.as_object() else {
        return Err(FunctionCallError::RespondToModel(
            "failed to parse read_file arguments: expected object".to_string(),
        ));
    };
    let path = ["path", "file_path", "filename"]
        .iter()
        .find_map(|key| object.get(*key).and_then(JsonValue::as_str))
        .ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "failed to parse read_file arguments: missing `path` or `file_path`".to_string(),
            )
        })?;

    let mut normalized = serde_json::Map::new();
    normalized.insert(
        "command".to_string(),
        JsonValue::String(native_read_file_shell_command(path)),
    );
    if let Some(workdir) = object
        .get("workdir")
        .or_else(|| object.get("cwd"))
        .or_else(|| object.get("working_directory"))
        .and_then(JsonValue::as_str)
    {
        normalized.insert(
            "workdir".to_string(),
            JsonValue::String(workdir.to_string()),
        );
    }
    if let Some(timeout) = object.get("timeout_ms").or_else(|| object.get("timeout")) {
        normalized.insert("timeout_ms".to_string(), timeout.clone());
    }

    Ok(JsonValue::Object(normalized).to_string())
}

fn parse_alias_arguments(tool_name: &str, arguments: &str) -> Result<JsonValue, FunctionCallError> {
    serde_json::from_str(arguments).map_err(|err| {
        FunctionCallError::RespondToModel(format!("failed to parse {tool_name} arguments: {err}"))
    })
}

fn shell_command_text_from_alias_object(
    object: &serde_json::Map<String, JsonValue>,
) -> Option<String> {
    object
        .get("command")
        .and_then(command_value_to_shell_text)
        .or_else(|| {
            object
                .get("cmd")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
        })
}

fn command_value_to_shell_text(value: &JsonValue) -> Option<String> {
    if let Some(command) = value.as_str() {
        return Some(command.to_string());
    }
    let command = value.as_array()?;
    let parts = command
        .iter()
        .map(|value| value.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    Some(codex_shell_command::parse_command::shlex_join(&parts))
}

fn native_read_file_shell_command(path: &str) -> String {
    if cfg!(windows) {
        let summary_path = path.replace('"', "`\"");
        format!(
            "Get-Content -LiteralPath {path:?} -TotalCount {NATIVE_READ_FILE_MAX_LINES}; \
$ReadFileCount = @(Get-Content -LiteralPath {path:?} -TotalCount {}).Count; \
$ReadFileLines = [Math]::Min($ReadFileCount, {NATIVE_READ_FILE_MAX_LINES}); \
$ReadFileEof = if ($ReadFileCount -le {NATIVE_READ_FILE_MAX_LINES}) {{ 'true' }} else {{ 'false' }}; \
Write-Output \"ReadFileSummaryV1: path={summary_path} lines_read=$ReadFileLines eof_reached=$ReadFileEof max_lines={NATIVE_READ_FILE_MAX_LINES}\"",
            NATIVE_READ_FILE_MAX_LINES + 1,
        )
    } else {
        let sed_args = vec![
            "sed".to_string(),
            "-n".to_string(),
            format!("1,{NATIVE_READ_FILE_MAX_LINES}p"),
            "--".to_string(),
            path.to_string(),
        ];
        let summary_script = format!(
            "NR == {} {{ truncated = 1; exit }} {{ lines = NR }} END {{ eof = truncated ? \"false\" : \"true\"; if ({NATIVE_READ_FILE_MAX_LINES} < lines) lines = {NATIVE_READ_FILE_MAX_LINES}; printf \"\\nReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s max_lines={NATIVE_READ_FILE_MAX_LINES}\\n\", FILENAME, lines + 0, eof }}",
            NATIVE_READ_FILE_MAX_LINES + 1,
        );
        let awk_args = vec!["awk".to_string(), summary_script, path.to_string()];
        format!(
            "{} && {}",
            codex_shell_command::parse_command::shlex_join(&sed_args),
            codex_shell_command::parse_command::shlex_join(&awk_args)
        )
    }
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod tests;
