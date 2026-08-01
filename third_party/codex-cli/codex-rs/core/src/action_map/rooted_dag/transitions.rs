use super::model::NodeId;
use super::model::NodeRole;
use super::model::NodeState;
use super::model::NodeView;
use super::model::TaskSpaceMap;
use super::model::node_ids;
use super::model::node_role;
use std::collections::BTreeSet;

pub(crate) fn derive_node_state(map: &TaskSpaceMap, node_id: &str) -> Option<NodeState> {
    let role = node_role(map, node_id)?;
    if map.terminal_record.is_some() && matches!(role, NodeRole::TaskRoot | NodeRole::Finish) {
        return Some(NodeState::Completed);
    }
    if map.completion_records.contains_key(node_id) {
        return Some(NodeState::Completed);
    }
    if map.block_records.contains_key(node_id) {
        return Some(NodeState::Blocked);
    }
    if map
        .action_reservations
        .values()
        .any(|reservation| reservation.node_id == node_id)
    {
        return Some(NodeState::InFlight);
    }
    Some(if predecessors_satisfied(map, node_id) {
        NodeState::Ready
    } else {
        NodeState::Waiting
    })
}

pub(crate) fn derive_node_views(map: &TaskSpaceMap) -> Vec<NodeView> {
    node_ids(map)
        .into_iter()
        .filter_map(|node_id| {
            derive_node_state(map, node_id).map(|state| NodeView {
                node_id: node_id.to_string(),
                state,
            })
        })
        .collect()
}

pub(crate) fn ready_node_ids(map: &TaskSpaceMap) -> Vec<NodeId> {
    derive_node_views(map)
        .into_iter()
        .filter(|view| view.state == NodeState::Ready)
        .map(|view| view.node_id)
        .collect()
}

pub(crate) fn predecessors(map: &TaskSpaceMap, node_id: &str) -> BTreeSet<NodeId> {
    map.edges
        .iter()
        .filter(|edge| edge.to == node_id)
        .map(|edge| edge.from.clone())
        .collect()
}

pub(crate) fn predecessors_satisfied(map: &TaskSpaceMap, node_id: &str) -> bool {
    let Some(role) = node_role(map, node_id) else {
        return false;
    };
    if role == NodeRole::TaskRoot {
        return true;
    }
    let predecessors = predecessors(map, node_id);
    !predecessors.is_empty()
        && predecessors.iter().all(|predecessor| {
            predecessor == &map.root.node_id || map.completion_records.contains_key(predecessor)
        })
}

#[cfg(test)]
mod tests {
    use super::super::model::MapEdge;
    use super::super::model::map_node;
    use super::super::model::new_map;
    use super::*;

    #[test]
    fn multiple_root_children_are_ready_without_a_current_node() {
        let map = new_map(
            "map".into(),
            map_node("root", "goal", vec![]),
            vec![
                map_node("left", "left", vec![]),
                map_node("right", "right", vec![]),
            ],
            map_node("finish", "finish", vec![]),
            vec![
                MapEdge {
                    from: "root".into(),
                    to: "left".into(),
                },
                MapEdge {
                    from: "root".into(),
                    to: "right".into(),
                },
                MapEdge {
                    from: "left".into(),
                    to: "finish".into(),
                },
                MapEdge {
                    from: "right".into(),
                    to: "finish".into(),
                },
            ],
        );

        assert_eq!(
            ready_node_ids(&map),
            vec!["left".to_string(), "right".to_string(), "root".to_string()]
        );
    }
}
