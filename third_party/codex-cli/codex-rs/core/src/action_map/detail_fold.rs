use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use super::map::ActionMapInstance;
use super::map::MapNodeId;
use super::map::NodeStatus;

pub(super) const MINIMUM_FRONTIER_DISTANCE: usize = 3;
pub(super) const NODE_DETAIL_EXPANDED_EVENT_KIND: &str = "node_detail_expanded";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NodeDetailState {
    Full,
    FoldEligible { frontier_distance: usize },
    Expanded { expansion_event_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NodeDetailPlan {
    states: HashMap<MapNodeId, NodeDetailState>,
}

impl NodeDetailPlan {
    pub(super) fn state(&self, node_id: &str) -> Option<&NodeDetailState> {
        self.states.get(node_id)
    }

    pub(super) fn eligible_node_count(&self) -> usize {
        self.states
            .values()
            .filter(|state| matches!(state, NodeDetailState::FoldEligible { .. }))
            .count()
    }
}

pub(super) fn node_detail_plan(
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
) -> NodeDetailPlan {
    let active_frontier = active_frontier(map, current_node_id);
    let distances = graph_distances(map, &active_frontier);
    let graph_roots = graph_roots(map);
    let states = map
        .nodes
        .iter()
        .map(|(node_id, node)| {
            let expansion_event_id = node.node_events.iter().find_map(|event_ref| {
                (event_ref.kind == NODE_DETAIL_EXPANDED_EVENT_KIND).then(|| event_ref.id.clone())
            });
            let state = if let Some(expansion_event_id) = expansion_event_id {
                NodeDetailState::Expanded { expansion_event_id }
            } else if node.status == NodeStatus::Completed
                && !graph_roots.contains(node_id)
                && current_node_id != Some(node_id.as_str())
                && node.active_lease.is_none()
                && !map.leases.values().any(|lease| lease.node_id == *node_id)
                && distances
                    .get(node_id)
                    .is_some_and(|distance| *distance >= MINIMUM_FRONTIER_DISTANCE)
            {
                NodeDetailState::FoldEligible {
                    frontier_distance: distances[node_id],
                }
            } else {
                NodeDetailState::Full
            };
            (node_id.clone(), state)
        })
        .collect();
    NodeDetailPlan { states }
}

fn active_frontier(map: &ActionMapInstance, current_node_id: Option<&str>) -> HashSet<MapNodeId> {
    let mut frontier = map
        .nodes
        .values()
        .filter(|node| node.status != NodeStatus::Completed)
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    if let Some(current_node_id) = current_node_id.filter(|id| map.nodes.contains_key(*id)) {
        frontier.insert(current_node_id.to_string());
    }
    frontier
}

fn graph_roots(map: &ActionMapInstance) -> HashSet<MapNodeId> {
    let mut roots = map.nodes.keys().cloned().collect::<HashSet<_>>();
    for edge in &map.edges {
        if map.nodes.contains_key(&edge.from) && map.nodes.contains_key(&edge.to) {
            roots.remove(&edge.to);
        }
    }
    roots
}

fn graph_distances(
    map: &ActionMapInstance,
    sources: &HashSet<MapNodeId>,
) -> HashMap<MapNodeId, usize> {
    let mut distances = sources
        .iter()
        .map(|node_id| (node_id.clone(), 0))
        .collect::<HashMap<_, _>>();
    let mut queue = sources.iter().cloned().collect::<VecDeque<_>>();
    while let Some(node_id) = queue.pop_front() {
        let next_distance = distances[&node_id] + 1;
        for adjacent in map.edges.iter().filter_map(|edge| {
            if edge.from == node_id {
                Some(edge.to.as_str())
            } else if edge.to == node_id {
                Some(edge.from.as_str())
            } else {
                None
            }
        }) {
            if map.nodes.contains_key(adjacent) && !distances.contains_key(adjacent) {
                distances.insert(adjacent.to_string(), next_distance);
                queue.push_back(adjacent.to_string());
            }
        }
    }
    distances
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::map::MapEdge;
    use crate::action_map::map::MapNode;
    use crate::action_map::map::NodeContext;
    use crate::action_map::map::NodeEvent;
    use crate::action_map::map::NodeEventRef;
    use crate::action_map::map::NodeKind;
    use codex_protocol::ThreadId;

    fn chain(statuses: &[NodeStatus]) -> ActionMapInstance {
        let mut map = ActionMapInstance::new("map-1".into(), "Map".into(), None, "v1");
        for (index, status) in statuses.iter().enumerate() {
            let id = format!("node-{index}");
            map.nodes.insert(
                id.clone(),
                MapNode {
                    id,
                    title: "Node".into(),
                    kind: NodeKind::InspectCodeContext,
                    status: *status,
                    context: NodeContext {
                        summary: "Goal".into(),
                        source_refs: Vec::new(),
                    },
                    active_lease: None,
                    result_context: Vec::new(),
                    node_events: Vec::new(),
                    origin_node_id: None,
                },
            );
        }
        map.edges = (1..statuses.len())
            .map(|index| MapEdge {
                from: format!("node-{}", index - 1),
                to: format!("node-{index}"),
            })
            .collect();
        map
    }

    #[test]
    fn folds_only_completed_non_root_nodes_at_distance_three_or_more() {
        let map = chain(&[
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Ready,
        ]);

        let plan = node_detail_plan(&map, Some("node-4"));

        assert_eq!(plan.state("node-0"), Some(&NodeDetailState::Full));
        assert_eq!(
            plan.state("node-1"),
            Some(&NodeDetailState::FoldEligible {
                frontier_distance: 3
            })
        );
        assert_eq!(plan.state("node-2"), Some(&NodeDetailState::Full));
        assert_eq!(plan.state("node-3"), Some(&NodeDetailState::Full));
        assert_eq!(plan.state("node-4"), Some(&NodeDetailState::Full));
    }

    #[test]
    fn uses_minimum_distance_across_all_active_frontiers() {
        let map = chain(&[
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Ready,
            NodeStatus::Completed,
            NodeStatus::Blocked,
        ]);

        let plan = node_detail_plan(&map, Some("node-5"));

        assert_eq!(plan.state("node-1"), Some(&NodeDetailState::Full));
    }

    #[test]
    fn does_not_fold_disconnected_or_root_nodes() {
        let mut map = chain(&[NodeStatus::Completed, NodeStatus::Ready]);
        map.edges.clear();

        let plan = node_detail_plan(&map, Some("node-1"));

        assert_eq!(plan.eligible_node_count(), 0);
    }

    #[test]
    fn expansion_event_permanently_overrides_fold_eligibility() {
        let mut map = chain(&[
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Completed,
            NodeStatus::Ready,
        ]);
        let event_id = "node-event-1".to_string();
        map.node_events.insert(
            event_id.clone(),
            NodeEvent {
                id: event_id.clone(),
                map_id: map.id.clone(),
                node_id: "node-1".into(),
                event_kind: NODE_DETAIL_EXPANDED_EVENT_KIND.into(),
                source: "agent_taskspace_control".into(),
                action_class: None,
                tool_success: None,
                content_sha256: "hash".into(),
                source_event_id: Some("task-event-1".into()),
                raw_ref: None,
                artifact_refs: Vec::new(),
                call_id: Some("call-1".into()),
                source_thread_id: ThreadId::new(),
                created_at_ms: 1,
            },
        );
        map.nodes
            .get_mut("node-1")
            .expect("expanded node")
            .node_events
            .push(NodeEventRef {
                id: event_id.clone(),
                kind: NODE_DETAIL_EXPANDED_EVENT_KIND.into(),
            });

        let plan = node_detail_plan(&map, Some("node-4"));

        assert_eq!(
            plan.state("node-1"),
            Some(&NodeDetailState::Expanded {
                expansion_event_id: event_id
            })
        );
    }
}
