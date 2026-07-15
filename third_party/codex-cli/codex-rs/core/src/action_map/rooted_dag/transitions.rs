use super::invariants::ViolationCode;
use super::model::NodeId;
use super::model::NodeRole;
use super::model::NodeStatus;
use super::model::TaskSpaceMap;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeTransition {
    Bind,
    Complete,
    Block,
    Unblock,
    ReleaseLease,
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
