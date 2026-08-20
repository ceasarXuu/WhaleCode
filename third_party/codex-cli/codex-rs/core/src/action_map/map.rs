use super::rooted_dag::MapNode;
use super::rooted_dag::NodeRole;
use super::rooted_dag::NodeState;
use super::rooted_dag::TaskSpaceMap;
use super::rooted_dag::derive_node_views;
use super::rooted_dag::is_complete;
use super::rooted_dag::nodes;

pub(crate) type ActionMapId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInstance {
    graph: TaskSpaceMap,
}

impl ActionMapInstance {
    pub(crate) fn new(graph: TaskSpaceMap) -> Self {
        Self { graph }
    }

    pub(crate) fn canonical_map(&self) -> &TaskSpaceMap {
        &self.graph
    }

    pub(crate) fn all_nodes(&self) -> impl Iterator<Item = (NodeRole, &MapNode)> {
        nodes(&self.graph)
    }

    pub(crate) fn node_views(&self) -> Vec<codex_protocol::taskspace::TaskSpaceNodeView> {
        derive_node_views(&self.graph)
    }

    pub(crate) fn is_complete(&self) -> bool {
        is_complete(&self.graph)
    }

    pub(crate) fn ready_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::Ready)
    }

    pub(crate) fn inflight_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::InFlight)
    }

    pub(crate) fn completed_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::Completed)
    }

    pub(crate) fn finish_ready(&self) -> bool {
        self.graph.finish.state == NodeState::Ready
    }

    fn count_work_nodes_in_state(&self, state: NodeState) -> usize {
        self.graph
            .work_nodes
            .iter()
            .filter(|node| node.state == state)
            .count()
    }
}

pub(crate) fn node_state_name(state: NodeState) -> &'static str {
    match state {
        NodeState::Waiting => "waiting",
        NodeState::Ready => "ready",
        NodeState::InFlight => "in_flight",
        NodeState::Completed => "completed",
    }
}
