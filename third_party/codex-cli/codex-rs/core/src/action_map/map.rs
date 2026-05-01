#![allow(dead_code)]

use std::collections::HashMap;

use codex_protocol::ThreadId;

pub(crate) type ActionMapId = String;
pub(crate) type AssignmentLeaseId = String;
pub(crate) type MapNodeId = String;
pub(crate) type NodeResultId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapStatus {
    Active,
    Completed,
    Abandoned,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssignmentLease {
    pub(crate) id: AssignmentLeaseId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) agent_thread_id: Option<ThreadId>,
    pub(crate) agent_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInstance {
    pub(crate) id: ActionMapId,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeResult {
    pub(crate) id: NodeResultId,
    pub(crate) assignment_id: AssignmentLeaseId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) kind: NodeResultKind,
    pub(crate) body: String,
    pub(crate) source_thread_id: ThreadId,
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
