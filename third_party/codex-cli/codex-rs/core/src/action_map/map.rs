#![allow(dead_code)]

use std::collections::HashMap;

use codex_protocol::ThreadId;

use super::cognitive::NodeResultEvidencePackage;
use super::cognitive::TaskCognitiveState;
use super::ledger::ProblemStateLedger;

pub(crate) type ActionMapId = String;
pub(crate) type AssignmentLeaseId = String;
pub(crate) type MapNodeId = String;
pub(crate) type NodeResultId = String;
pub(crate) type TaskId = String;
pub(crate) type TaskSpaceTraceEventId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskStatus {
    Active,
    Pending,
}

impl TaskStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Active => "active",
            TaskStatus::Pending => "pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskState {
    pub(crate) id: TaskId,
    pub(crate) title: String,
    pub(crate) objective: String,
    pub(crate) status: TaskStatus,
    pub(crate) owner_session_id: Option<ThreadId>,
    pub(crate) active_map_id: Option<ActionMapId>,
    pub(crate) map_ids: Vec<ActionMapId>,
    pub(crate) cognitive_state: TaskCognitiveState,
    pub(crate) problem_ledger: ProblemStateLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapStatus {
    Active,
    Completed,
    Abandoned,
}

impl MapStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MapStatus::Active => "active",
            MapStatus::Completed => "completed",
            MapStatus::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeStatus {
    Pending,
    Ready,
    Running,
    Blocked,
    Completed,
}

impl NodeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            NodeStatus::Pending => "pending",
            NodeStatus::Ready => "ready",
            NodeStatus::Running => "running",
            NodeStatus::Blocked => "blocked",
            NodeStatus::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    InspectCodeContext,
    ImplementSolution,
    SmokeTest,
    RegressionTest,
    FinalSynthesis,
    Custom,
}

impl NodeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            NodeKind::InspectCodeContext => "inspect_code_context",
            NodeKind::ImplementSolution => "implement_solution",
            NodeKind::SmokeTest => "smoke_test",
            NodeKind::RegressionTest => "regression_test",
            NodeKind::FinalSynthesis => "final_synthesis",
            NodeKind::Custom => "custom",
        }
    }

    pub(crate) fn canonical_kind(self) -> &'static str {
        match self {
            NodeKind::InspectCodeContext => "discover",
            NodeKind::ImplementSolution => "patch",
            NodeKind::SmokeTest | NodeKind::RegressionTest => "validate",
            NodeKind::FinalSynthesis => "synthesize",
            NodeKind::Custom => "custom",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        let normalized = normalize_contract_name(value);
        match normalized.as_str() {
            "inspect_code_context" | "inspectcodecontext" => Some(NodeKind::InspectCodeContext),
            "implement_solution" | "implementsolution" => Some(NodeKind::ImplementSolution),
            "smoke_test" | "smoketest" => Some(NodeKind::SmokeTest),
            "regression_test" | "regressiontest" => Some(NodeKind::RegressionTest),
            "final_synthesis" | "finalsynthesis" => Some(NodeKind::FinalSynthesis),
            "custom" => Some(NodeKind::Custom),
            _ => None,
        }
    }

    pub(crate) fn from_node_id_or_title(id: &str, title: &str) -> Self {
        Self::from_str(id)
            .or_else(|| Self::from_str(title))
            .unwrap_or(NodeKind::Custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionClass {
    Read,
    Search,
    Edit,
    Build,
    Test,
    Spawn,
    Wait,
    Review,
    FinalResponse,
    Control,
    Unknown,
}

impl ActionClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ActionClass::Read => "read",
            ActionClass::Search => "search",
            ActionClass::Edit => "edit",
            ActionClass::Build => "build",
            ActionClass::Test => "test",
            ActionClass::Spawn => "spawn",
            ActionClass::Wait => "wait",
            ActionClass::Review => "review",
            ActionClass::FinalResponse => "final_response",
            ActionClass::Control => "control",
            ActionClass::Unknown => "unknown",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        let normalized = normalize_contract_name(value);
        match normalized.as_str() {
            "read" => Some(ActionClass::Read),
            "search" => Some(ActionClass::Search),
            "edit" => Some(ActionClass::Edit),
            "build" => Some(ActionClass::Build),
            "test" => Some(ActionClass::Test),
            "spawn" => Some(ActionClass::Spawn),
            "wait" => Some(ActionClass::Wait),
            "review" => Some(ActionClass::Review),
            "final_response" | "finalresponse" => Some(ActionClass::FinalResponse),
            "control" => Some(ActionClass::Control),
            "unknown" => Some(ActionClass::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolActionDescriptor {
    pub(crate) tool_name: String,
    pub(crate) call_id: Option<String>,
    pub(crate) action_class: ActionClass,
    pub(crate) preview: String,
}

impl ToolActionDescriptor {
    pub(crate) fn new(
        tool_name: impl Into<String>,
        action_class: ActionClass,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            call_id: None,
            action_class,
            preview: preview.into(),
        }
    }

    pub(crate) fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }
}

impl From<&str> for ToolActionDescriptor {
    fn from(tool_name: &str) -> Self {
        Self::new(tool_name, ActionClass::Read, "")
    }
}

impl From<String> for ToolActionDescriptor {
    fn from(tool_name: String) -> Self {
        Self::new(tool_name, ActionClass::Read, "")
    }
}

fn normalize_contract_name(value: &str) -> String {
    value
        .trim()
        .replace('-', "_")
        .replace(' ', "_")
        .to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeContext {
    pub(crate) summary: String,
    pub(crate) source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeResultRef {
    pub(crate) id: NodeResultId,
    pub(crate) kind: NodeResultKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapNode {
    pub(crate) id: MapNodeId,
    pub(crate) title: String,
    pub(crate) kind: NodeKind,
    pub(crate) status: NodeStatus,
    pub(crate) context: NodeContext,
    pub(crate) active_lease: Option<AssignmentLeaseId>,
    pub(crate) result_context: Vec<NodeResultRef>,
    pub(crate) origin_node_id: Option<MapNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapEdge {
    pub(crate) from: MapNodeId,
    pub(crate) to: MapNodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaseHolder {
    Main,
    SubAgent,
}

impl LeaseHolder {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            LeaseHolder::Main => "main",
            LeaseHolder::SubAgent => "subagent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssignmentLease {
    pub(crate) id: AssignmentLeaseId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) holder: LeaseHolder,
    pub(crate) previous_node_status: NodeStatus,
    pub(crate) agent_thread_id: Option<ThreadId>,
    pub(crate) agent_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInstance {
    pub(crate) id: ActionMapId,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) title: String,
    pub(crate) status: MapStatus,
    pub(crate) owner_session_id: Option<ThreadId>,
    pub(crate) base_map_version: String,
    pub(crate) nodes: HashMap<MapNodeId, MapNode>,
    pub(crate) edges: Vec<MapEdge>,
    pub(crate) created_from: Option<ActionMapId>,
    pub(crate) leases: HashMap<AssignmentLeaseId, AssignmentLease>,
    pub(crate) results: HashMap<NodeResultId, NodeResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeResultKind {
    Result,
    Blocker,
    MapUpdateRequest,
    TimeoutSummary,
    MainToolCall,
}

impl NodeResultKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            NodeResultKind::Result => "result",
            NodeResultKind::Blocker => "blocker",
            NodeResultKind::MapUpdateRequest => "map_update_request",
            NodeResultKind::TimeoutSummary => "timeout_summary",
            NodeResultKind::MainToolCall => "main_tool_call",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeResult {
    pub(crate) id: NodeResultId,
    pub(crate) assignment_id: AssignmentLeaseId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) kind: NodeResultKind,
    pub(crate) action_class: Option<ActionClass>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) body: String,
    pub(crate) evidence_package: NodeResultEvidencePackage,
    pub(crate) source_thread_id: ThreadId,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceTraceEvent {
    pub(crate) id: TaskSpaceTraceEventId,
    pub(crate) kind: String,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) result_id: Option<NodeResultId>,
    pub(crate) call_id: Option<String>,
    pub(crate) action_class: Option<ActionClass>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) tags: Vec<String>,
    pub(crate) artifact_refs: Vec<String>,
    pub(crate) created_at_ms: i64,
}

impl ActionMapInstance {
    pub(crate) fn new(
        id: ActionMapId,
        title: String,
        owner_session_id: Option<ThreadId>,
        base_map_version: impl Into<String>,
    ) -> Self {
        Self {
            id,
            task_id: None,
            title,
            status: MapStatus::Active,
            owner_session_id,
            base_map_version: base_map_version.into(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            created_from: None,
            leases: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub(crate) fn ready_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|node| node.status == NodeStatus::Ready)
            .count()
    }

    pub(crate) fn running_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|node| node.status == NodeStatus::Running)
            .count()
    }

    pub(crate) fn completed_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|node| node.status == NodeStatus::Completed)
            .count()
    }
}
