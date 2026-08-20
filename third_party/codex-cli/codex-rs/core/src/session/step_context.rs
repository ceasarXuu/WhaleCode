use std::sync::Arc;

use crate::agents_md::LoadedAgentsMd;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::session::turn_context::TurnContext;
use crate::tools::router::ToolRouter;
use codex_exec_server::ExecutorCapabilityDiscoverySnapshot;
use codex_exec_server::ResolvedSelectedCapabilityRoot;
use codex_mcp::McpBinding;

/// Request-scoped state that may change between model sampling requests.
pub(crate) struct StepContext {
    pub(crate) turn: Arc<TurnContext>,
    pub(crate) environments: TurnEnvironmentSnapshot,
    /// Capability roots bound to ready environments in this exact step.
    pub(crate) selected_capability_roots: Vec<ResolvedSelectedCapabilityRoot>,
    /// Executor-materialized capability files shared by MCP and skills in this exact step.
    pub(crate) executor_capability_discovery: Option<Arc<ExecutorCapabilityDiscoverySnapshot>>,
    /// The exact MCP connections, configuration, and catalog captured for this step.
    pub(crate) mcp: Arc<McpBinding>,
    /// The finalized tool plan advertised and executed for this exact sampling request.
    pub(crate) tool_router: Arc<ToolRouter>,
    /// The canonical AGENTS.md value observed with this environment snapshot.
    pub(crate) loaded_agents_md: Option<Arc<LoadedAgentsMd>>,
}

impl StepContext {
    pub(crate) fn with_tool_router(&self, tool_router: Arc<ToolRouter>) -> Self {
        Self {
            turn: Arc::clone(&self.turn),
            environments: self.environments.clone(),
            selected_capability_roots: self.selected_capability_roots.clone(),
            executor_capability_discovery: self.executor_capability_discovery.clone(),
            mcp: Arc::clone(&self.mcp),
            tool_router,
            loaded_agents_md: self.loaded_agents_md.clone(),
        }
    }
}
