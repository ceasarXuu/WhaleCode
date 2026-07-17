use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;

pub(crate) type Revision = u64;
pub(crate) type MapId = String;
pub(crate) type NodeId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeRole {
    TaskRoot,
    Work,
    Finish,
}

impl NodeRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TaskRoot => "task_root",
            Self::Work => "work",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeStatus {
    Open,
    Closed,
    Pending,
    Ready,
    Running,
    Blocked,
    Completed,
}

impl NodeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
            Self::Result => "result",
            Self::Blocker => "blocker",
            Self::MapUpdateRequest => "map_update_request",
            Self::TimeoutSummary => "timeout_summary",
            Self::MainToolCall => "main_tool_call",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NodeResultRef {
    pub(crate) id: String,
    pub(crate) kind: NodeResultKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NodeEventRef {
    pub(crate) id: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MapNode {
    pub(crate) role: NodeRole,
    pub(crate) goal: String,
    pub(crate) source_refs: Vec<String>,
    pub(crate) status: NodeStatus,
    pub(crate) active_lease: Option<String>,
    pub(crate) result_context: Vec<NodeResultRef>,
    pub(crate) node_events: Vec<NodeEventRef>,
    pub(crate) origin_node_id: Option<NodeId>,
}

impl MapNode {
    pub(crate) fn task_root(goal: impl Into<String>, source_refs: Vec<String>) -> Self {
        Self {
            role: NodeRole::TaskRoot,
            goal: goal.into(),
            source_refs,
            status: NodeStatus::Open,
            active_lease: None,
            result_context: Vec::new(),
            node_events: Vec::new(),
            origin_node_id: None,
        }
    }

    pub(crate) fn work(goal: impl Into<String>) -> Self {
        Self {
            role: NodeRole::Work,
            goal: goal.into(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
            active_lease: None,
            result_context: Vec::new(),
            node_events: Vec::new(),
            origin_node_id: None,
        }
    }

    pub(crate) fn finish() -> Self {
        Self {
            role: NodeRole::Finish,
            goal: String::new(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
            active_lease: None,
            result_context: Vec::new(),
            node_events: Vec::new(),
            origin_node_id: None,
        }
    }

    pub(crate) fn status_allowed(&self) -> bool {
        matches!(
            (self.role, self.status),
            (NodeRole::TaskRoot, NodeStatus::Open | NodeStatus::Closed)
                | (
                    NodeRole::Work,
                    NodeStatus::Pending
                        | NodeStatus::Ready
                        | NodeStatus::Running
                        | NodeStatus::Blocked
                        | NodeStatus::Completed
                )
                | (
                    NodeRole::Finish,
                    NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Closed
                )
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct MapEdge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
}

impl MapEdge {
    pub(crate) fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskSpaceMap {
    pub(crate) id: MapId,
    pub(crate) root_node_id: NodeId,
    pub(crate) finish_node_id: NodeId,
    pub(crate) nodes: BTreeMap<NodeId, MapNode>,
    pub(crate) edges: Vec<MapEdge>,
    pub(crate) revision: Revision,
    pub(crate) current_binding: Option<NodeId>,
    pub(crate) terminal_summary_ref: Option<String>,
}

impl TaskSpaceMap {
    pub(crate) fn state_sha256(&self) -> Result<String, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub(crate) fn node(&self, id: &NodeId) -> Option<&MapNode> {
        self.nodes.get(id)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.node(&self.root_node_id)
            .is_some_and(|node| node.status == NodeStatus::Closed)
            && self
                .node(&self.finish_node_id)
                .is_some_and(|node| node.status == NodeStatus::Closed)
    }

    pub(crate) fn canonicalize(&mut self) {
        self.edges.sort();
    }
}
