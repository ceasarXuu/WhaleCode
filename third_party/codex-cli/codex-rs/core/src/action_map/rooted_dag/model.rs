use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;

pub(crate) type Revision = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct MapId(String);

impl MapId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
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
pub(crate) struct NodeId(String);

impl NodeId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeRole {
    TaskRoot,
    Work,
    Finish,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MapNode {
    pub(crate) role: NodeRole,
    pub(crate) goal: String,
    pub(crate) source_refs: Vec<String>,
    pub(crate) status: NodeStatus,
}

impl MapNode {
    pub(crate) fn task_root(goal: impl Into<String>, source_refs: Vec<String>) -> Self {
        Self {
            role: NodeRole::TaskRoot,
            goal: goal.into(),
            source_refs,
            status: NodeStatus::Open,
        }
    }

    pub(crate) fn work(goal: impl Into<String>) -> Self {
        Self {
            role: NodeRole::Work,
            goal: goal.into(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
        }
    }

    pub(crate) fn finish(goal: impl Into<String>) -> Self {
        Self {
            role: NodeRole::Finish,
            goal: goal.into(),
            source_refs: Vec::new(),
            status: NodeStatus::Pending,
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
            from: NodeId::new(from),
            to: NodeId::new(to),
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
