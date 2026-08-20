use super::model::NodeState;
use super::model::NodeView;
use super::model::TaskSpaceMap;
use super::model::children_by_parent;
use super::model::node;
use super::model::nodes;

pub(crate) fn predecessors_satisfied(map: &TaskSpaceMap, node_id: &str) -> bool {
    let Some(candidate) = node(map, node_id) else {
        return false;
    };
    if candidate.node_id == map.root.node_id {
        return true;
    }
    !candidate.parents.is_empty()
        && candidate.parents.iter().all(|parent_id| {
            parent_id == &map.root.node_id
                || node(map, parent_id).is_some_and(|parent| parent.state == NodeState::Completed)
        })
}

pub(crate) fn normalize_readiness(map: &mut TaskSpaceMap) {
    let states = nodes(map)
        .map(|(_, node)| (node.node_id.clone(), node.state))
        .collect::<std::collections::BTreeMap<_, _>>();
    let root_id = map.root.node_id.clone();
    for candidate in std::iter::once(&mut map.root)
        .chain(map.work_nodes.iter_mut())
        .chain(std::iter::once(&mut map.finish))
    {
        if !matches!(candidate.state, NodeState::Waiting | NodeState::Ready) {
            continue;
        }
        if candidate.node_id == root_id {
            candidate.state = NodeState::InFlight;
            continue;
        }
        let ready = !candidate.parents.is_empty()
            && candidate.parents.iter().all(|parent_id| {
                parent_id == &root_id || states.get(parent_id) == Some(&NodeState::Completed)
            });
        candidate.state = if ready {
            NodeState::Ready
        } else {
            NodeState::Waiting
        };
    }
}

pub(crate) fn derive_node_views(map: &TaskSpaceMap) -> Vec<NodeView> {
    let children = children_by_parent(map);
    nodes(map)
        .map(|(_, node)| NodeView {
            node_id: node.node_id.clone(),
            goal: node.goal.clone(),
            state: node.state,
            content: node.content.clone(),
            parents: node.parents.clone(),
            children: children.get(&node.node_id).cloned().unwrap_or_default(),
            actions: node.actions.clone(),
        })
        .collect()
}
