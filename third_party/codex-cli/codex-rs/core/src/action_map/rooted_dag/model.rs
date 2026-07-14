use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;

pub(super) type Revision = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct MapId(String);

impl MapId {
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MapId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct NodeId(String);

impl NodeId {
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NodeRole {
    TaskRoot,
    Work,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NodeStatus {
    Open,
    Closed,
    Pending,
    Ready,
    Running,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MapNode {
    pub(super) role: NodeRole,
    pub(super) goal: String,
    pub(super) source_refs: Vec<String>,
    pub(super) status: NodeStatus,
}

impl MapNode {
    pub(super) fn task_root(goal: impl Into<String>, source_refs: Vec<String>) -> Self {
        Self {
            role: NodeRole::TaskRoot,
            goal: goal.into(),
            source_refs,
            status: NodeStatus::Open,
        }
    }

    pub(super) fn work(goal: impl Into<String>) -> Self {
        Self {
            role: NodeRole::Work,
            goal: goal.into(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
        }
    }

    pub(super) fn finish(goal: impl Into<String>) -> Self {
        Self {
            role: NodeRole::Finish,
            goal: goal.into(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
        }
    }

    pub(super) fn status_allowed(&self) -> bool {
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
pub(super) struct MapEdge {
    pub(super) from: NodeId,
    pub(super) to: NodeId,
}

impl MapEdge {
    pub(super) fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: NodeId::new(from),
            to: NodeId::new(to),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TaskSpaceMap {
    pub(super) id: MapId,
    pub(super) root_node_id: NodeId,
    pub(super) finish_node_id: NodeId,
    pub(super) nodes: BTreeMap<NodeId, MapNode>,
    pub(super) edges: Vec<MapEdge>,
    pub(super) revision: Revision,
    pub(super) current_binding: Option<NodeId>,
    pub(super) terminal_summary_ref: Option<String>,
}

impl TaskSpaceMap {
    pub(super) fn state_sha256(&self) -> Result<String, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub(super) fn node(&self, id: &NodeId) -> Option<&MapNode> {
        self.nodes.get(id)
    }

    pub(super) fn is_complete(&self) -> bool {
        self.node(&self.root_node_id)
            .is_some_and(|node| node.status == NodeStatus::Closed)
            && self
                .node(&self.finish_node_id)
                .is_some_and(|node| node.status == NodeStatus::Closed)
    }

    pub(super) fn canonicalize(&mut self) {
        self.edges.sort();
    }
}
