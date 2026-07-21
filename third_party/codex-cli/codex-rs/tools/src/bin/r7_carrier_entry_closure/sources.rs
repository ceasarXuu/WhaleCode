use codex_tools::ToolHandlerKind;
use serde::Serialize;
use sha2::Digest;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use syn::visit::Visit;

pub const TOOL_SPEC_VARIANTS: &[&str] = &[
    "Function",
    "Namespace",
    "ToolSearch",
    "LocalShell",
    "ImageGeneration",
    "WebSearch",
    "Freeform",
];
pub const TOOL_PAYLOAD_VARIANTS: &[&str] =
    &["Function", "ToolSearch", "Custom", "LocalShell", "Mcp"];
pub const TOOL_HANDLER_VARIANTS: &[&str] = &[
    "AgentJobs",
    "ApplyPatch",
    "CloseAgentV1",
    "CloseAgentV2",
    "CodeModeExecute",
    "CodeModeWait",
    "DynamicTool",
    "FollowupTaskV2",
    "Goal",
    "ListAgentsV2",
    "ListDir",
    "Mcp",
    "McpResource",
    "Plan",
    "RequestPermissions",
    "RequestUserInput",
    "ResumeAgentV1",
    "SendInputV1",
    "SendMessageV2",
    "Shell",
    "ShellCommand",
    "SpawnAgentV1",
    "SpawnAgentV2",
    "TaskSpaceControl",
    "TestSync",
    "ToolSearch",
    "ToolSuggest",
    "UnifiedExec",
    "ViewImage",
    "WebFetch",
    "WebSearch",
    "WaitAgentV1",
    "WaitAgentV2",
];

#[derive(Serialize, Clone)]
pub struct SourceBinding {
    symbol: String,
    path: String,
    sha256: String,
}

#[derive(Default)]
pub struct AstIndex {
    enums: BTreeMap<String, Vec<String>>,
    symbols: BTreeMap<String, BTreeSet<String>>,
    source_hashes: BTreeMap<String, String>,
    current_path: String,
}

impl<'ast> Visit<'ast> for AstIndex {
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.enums.insert(
            node.ident.to_string(),
            node.variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect(),
        );
        self.record(&node.ident.to_string());
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.record(&node.ident.to_string());
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record(&node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record(&node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
    }
}

impl AstIndex {
    fn record(&mut self, symbol: &str) {
        self.symbols
            .entry(symbol.to_string())
            .or_default()
            .insert(self.current_path.clone());
    }

    fn binding(&self, root: &Path, symbol: &str, suffix: &str) -> Result<SourceBinding, String> {
        let paths = self
            .symbols
            .get(symbol)
            .ok_or_else(|| format!("required Rust symbol missing: {symbol}"))?;
        let matches = paths
            .iter()
            .filter(|path| path.ends_with(suffix))
            .cloned()
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "symbol {symbol} did not resolve exactly once through {suffix}: {matches:?}"
            ));
        }
        let path = &matches[0];
        Ok(SourceBinding {
            symbol: symbol.to_string(),
            path: path.clone(),
            sha256: sha256(&fs::read(root.join(path)).map_err(|error| error.to_string())?),
        })
    }

    pub fn binding_source_hashes(
        &self,
        bindings: &BTreeMap<String, SourceBinding>,
    ) -> BTreeMap<String, String> {
        bindings
            .values()
            .map(|binding| {
                (
                    binding.path.clone(),
                    self.source_hashes[&binding.path].clone(),
                )
            })
            .collect()
    }
}

pub fn index_sources(root: &Path) -> Result<AstIndex, String> {
    let mut files = Vec::new();
    for relative in [
        "third_party/codex-cli/codex-rs/tools/src",
        "third_party/codex-cli/codex-rs/core/src",
        "third_party/codex-cli/codex-rs/codex-api/src",
    ] {
        collect_rs_files(&root.join(relative), &mut files)?;
    }
    let mut index = AstIndex::default();
    for path in files {
        index.current_path = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        index
            .source_hashes
            .insert(index.current_path.clone(), sha256(source.as_bytes()));
        let syntax = syn::parse_file(&source)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        index.visit_file(&syntax);
    }
    Ok(index)
}

fn collect_rs_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_rs_files(&entry.path(), output)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            output.push(entry.path());
        }
    }
    output.sort();
    Ok(())
}

pub fn assert_variants(index: &AstIndex, name: &str, expected: &[&str]) -> Result<(), String> {
    let actual = index
        .enums
        .get(name)
        .ok_or_else(|| format!("enum missing: {name}"))?;
    let expected = expected.iter().map(ToString::to_string).collect::<Vec<_>>();
    if actual != &expected {
        return Err(format!(
            "{name} capability epoch drifted: expected={expected:?} actual={actual:?}"
        ));
    }
    Ok(())
}

pub fn pipeline_bindings(
    root: &Path,
    index: &AstIndex,
) -> Result<BTreeMap<String, SourceBinding>, String> {
    let required = [
        (
            "registry_plan",
            "build_tool_registry_plan",
            "tools/src/tool_registry_plan.rs",
        ),
        (
            "code_mode_decorator",
            "augment_tool_spec_for_code_mode",
            "tools/src/code_mode.rs",
        ),
        (
            "core_registration",
            "build_specs_with_discoverable_tools",
            "core/src/tools/spec.rs",
        ),
        (
            "invocation_parser",
            "build_tool_call",
            "core/src/tools/router.rs",
        ),
        (
            "alias_router",
            "normalize_native_function_alias",
            "core/src/tools/router.rs",
        ),
        ("approval", "run", "core/src/tools/orchestrator.rs"),
        ("executor", "dispatch_any", "core/src/tools/registry.rs"),
        (
            "output_mapper",
            "into_response",
            "core/src/tools/registry.rs",
        ),
        (
            "responses_mapper",
            "create_tools_json_for_responses_api",
            "tools/src/tool_spec.rs",
        ),
        (
            "deepseek_mapper",
            "chat_tools_from_responses_tools",
            "codex-api/src/endpoint/responses.rs",
        ),
        (
            "dynamic_registry",
            "dynamic_tool_to_loadable_tool_spec",
            "tools/src/responses_api.rs",
        ),
        (
            "mcp_registry",
            "mcp_tool_to_responses_api_tool",
            "tools/src/responses_api.rs",
        ),
    ];
    let mut bindings = required
        .into_iter()
        .map(|(role, symbol, suffix)| Ok((role.to_string(), index.binding(root, symbol, suffix)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    for (kind, symbol, suffix) in handler_sources() {
        bindings.insert(
            format!("handler::{kind:?}"),
            index.binding(root, symbol, suffix)?,
        );
    }
    Ok(bindings)
}

fn handler_sources() -> Vec<(ToolHandlerKind, &'static str, &'static str)> {
    use ToolHandlerKind::*;
    vec![
        (
            AgentJobs,
            "BatchJobHandler",
            "core/src/tools/handlers/agent_jobs.rs",
        ),
        (
            ApplyPatch,
            "ApplyPatchHandler",
            "core/src/tools/handlers/apply_patch.rs",
        ),
        (
            CloseAgentV1,
            "Handler",
            "core/src/tools/handlers/multi_agents/close_agent.rs",
        ),
        (
            CloseAgentV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/close_agent.rs",
        ),
        (
            CodeModeExecute,
            "CodeModeExecuteHandler",
            "core/src/tools/code_mode/execute_handler.rs",
        ),
        (
            CodeModeWait,
            "CodeModeWaitHandler",
            "core/src/tools/code_mode/wait_handler.rs",
        ),
        (
            DynamicTool,
            "DynamicToolHandler",
            "core/src/tools/handlers/dynamic.rs",
        ),
        (
            FollowupTaskV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/followup_task.rs",
        ),
        (Goal, "GoalHandler", "core/src/tools/handlers/goal.rs"),
        (
            ListAgentsV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/list_agents.rs",
        ),
        (
            ListDir,
            "ListDirHandler",
            "core/src/tools/handlers/list_dir.rs",
        ),
        (Mcp, "McpHandler", "core/src/tools/handlers/mcp.rs"),
        (
            McpResource,
            "McpResourceHandler",
            "core/src/tools/handlers/mcp_resource.rs",
        ),
        (Plan, "PlanHandler", "core/src/tools/handlers/plan.rs"),
        (
            RequestPermissions,
            "RequestPermissionsHandler",
            "core/src/tools/handlers/request_permissions.rs",
        ),
        (
            RequestUserInput,
            "RequestUserInputHandler",
            "core/src/tools/handlers/request_user_input.rs",
        ),
        (
            ResumeAgentV1,
            "Handler",
            "core/src/tools/handlers/multi_agents/resume_agent.rs",
        ),
        (
            SendInputV1,
            "Handler",
            "core/src/tools/handlers/multi_agents/send_input.rs",
        ),
        (
            SendMessageV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/send_message.rs",
        ),
        (Shell, "ShellHandler", "core/src/tools/handlers/shell.rs"),
        (
            ShellCommand,
            "ShellCommandHandler",
            "core/src/tools/handlers/shell.rs",
        ),
        (
            SpawnAgentV1,
            "Handler",
            "core/src/tools/handlers/multi_agents/spawn.rs",
        ),
        (
            SpawnAgentV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/spawn.rs",
        ),
        (
            TaskSpaceControl,
            "TaskSpaceControlHandler",
            "core/src/tools/handlers/taskspace_control.rs",
        ),
        (
            TestSync,
            "TestSyncHandler",
            "core/src/tools/handlers/test_sync.rs",
        ),
        (
            ToolSearch,
            "ToolSearchHandler",
            "core/src/tools/handlers/tool_search.rs",
        ),
        (
            ToolSuggest,
            "ToolSuggestHandler",
            "core/src/tools/handlers/tool_suggest.rs",
        ),
        (
            UnifiedExec,
            "UnifiedExecHandler",
            "core/src/tools/handlers/unified_exec.rs",
        ),
        (
            ViewImage,
            "ViewImageHandler",
            "core/src/tools/handlers/view_image.rs",
        ),
        (
            WebFetch,
            "WebFetchHandler",
            "core/src/web_tools/handlers.rs",
        ),
        (
            WebSearch,
            "WebSearchHandler",
            "core/src/web_tools/handlers.rs",
        ),
        (
            WaitAgentV1,
            "Handler",
            "core/src/tools/handlers/multi_agents/wait.rs",
        ),
        (
            WaitAgentV2,
            "Handler",
            "core/src/tools/handlers/multi_agents_v2/wait.rs",
        ),
    ]
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}

pub fn canonical_hash<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| error.to_string())
}
