use super::invariants::ViolationCode;
use super::model::NodeId;
use super::model::NodeRole;
use super::model::NodeStatus;
use super::model::TaskSpaceMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeTransition {
    Bind,
    Complete,
    Block,
    Unblock,
    Rework,
    ReleaseLease,
}

impl NodeTransition {
    pub(crate) fn operation_name(self) -> &'static str {
        match self {
            Self::Bind => "bind_node",
            Self::Complete => "complete_node",
            Self::Block => "block_node",
            Self::Unblock => "unblock_node",
            Self::Rework => "rework_node",
            Self::ReleaseLease => "release_lease",
        }
    }
}

#[cfg(test)]
mod operation_name_tests {
    use super::NodeTransition;

    #[test]
    fn observable_operation_names_match_direct_control_actions() {
        assert_eq!(NodeTransition::Bind.operation_name(), "bind_node");
        assert_eq!(NodeTransition::Block.operation_name(), "block_node");
        assert_eq!(NodeTransition::Unblock.operation_name(), "unblock_node");
        assert_eq!(NodeTransition::Rework.operation_name(), "rework_node");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReadinessChange {
    pub(super) node_id: NodeId,
    pub(super) from: NodeStatus,
    pub(super) to: NodeStatus,
}

pub(crate) fn transition_target(
    role: NodeRole,
    status: NodeStatus,
    transition: NodeTransition,
) -> Result<NodeStatus, ViolationCode> {
    match (role, status, transition) {
        (NodeRole::Work, NodeStatus::Ready, NodeTransition::Bind) => Ok(NodeStatus::Running),
        (NodeRole::Work, NodeStatus::Running, NodeTransition::Complete) => {
            Ok(NodeStatus::Completed)
        }
        (NodeRole::Work, NodeStatus::Running, NodeTransition::Block) => Ok(NodeStatus::Blocked),
        (NodeRole::Work, NodeStatus::Blocked, NodeTransition::Unblock) => Ok(NodeStatus::Ready),
        (NodeRole::Work, NodeStatus::Completed, NodeTransition::Rework) => Ok(NodeStatus::Ready),
        (NodeRole::Work, NodeStatus::Running, NodeTransition::ReleaseLease) => {
            Ok(NodeStatus::Ready)
        }
        _ => Err(ViolationCode::TransitionInvalid),
    }
}

pub(crate) fn readiness_changes(map: &TaskSpaceMap) -> Vec<ReadinessChange> {
    map.nodes
        .iter()
        .filter_map(|(id, node)| {
            let to = match (node.role, node.status) {
                (NodeRole::Work, NodeStatus::Pending) if predecessors_satisfied(map, id) => {
                    NodeStatus::Ready
                }
                (NodeRole::Finish, NodeStatus::Pending) if predecessors_satisfied(map, id) => {
                    NodeStatus::Ready
                }
                (NodeRole::Work | NodeRole::Finish, NodeStatus::Ready)
                    if !predecessors_satisfied(map, id) =>
                {
                    NodeStatus::Pending
                }
                _ => return None,
            };
            Some(ReadinessChange {
                node_id: id.clone(),
                from: node.status,
                to,
            })
        })
        .collect()
}

pub(crate) fn rework_conflicts(map: &TaskSpaceMap, node_id: &NodeId) -> Vec<NodeId> {
    let mut queue = VecDeque::from([node_id.clone()]);
    let mut visited = BTreeSet::from([node_id.clone()]);
    let mut conflicts = BTreeSet::new();
    while let Some(current) = queue.pop_front() {
        for successor in map
            .edges
            .iter()
            .filter(|edge| edge.from == current)
            .map(|edge| &edge.to)
        {
            if !visited.insert(successor.clone()) {
                continue;
            }
            if map.node(successor).is_some_and(|node| {
                node.role == NodeRole::Work
                    && matches!(
                        node.status,
                        NodeStatus::Running | NodeStatus::Blocked | NodeStatus::Completed
                    )
            }) {
                conflicts.insert(successor.clone());
            }
            queue.push_back(successor.clone());
        }
    }
    conflicts.into_iter().collect()
}

pub(crate) fn predecessors_satisfied(map: &TaskSpaceMap, node_id: &NodeId) -> bool {
    let predecessors: Vec<_> = map
        .edges
        .iter()
        .filter(|edge| &edge.to == node_id)
        .filter_map(|edge| map.node(&edge.from))
        .collect();
    !predecessors.is_empty()
        && predecessors.iter().all(|node| match node.role {
            NodeRole::TaskRoot => node.status == NodeStatus::Open,
            NodeRole::Work => node.status == NodeStatus::Completed,
            NodeRole::Finish => false,
        })
}
