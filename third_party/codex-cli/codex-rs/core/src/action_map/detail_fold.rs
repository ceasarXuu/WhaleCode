use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use super::map::ActionMapInstance;
use super::map::MapNodeId;
use super::map::NodeRole;
use super::map::NodeState;

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

pub(super) fn node_detail_plan(map: &ActionMapInstance) -> NodeDetailPlan {
    let active_frontier = active_frontier(map);
    let distances = graph_distances(map, &active_frontier);
    let states = map
        .all_nodes()
        .map(|(role, node)| {
            let node_id = &node.node_id;
            let expansion_event_id = map.node_events.values().find_map(|event| {
                (event.node_id == *node_id && event.event_kind == NODE_DETAIL_EXPANDED_EVENT_KIND)
                    .then(|| event.id.clone())
            });
            let state = if let Some(expansion_event_id) = expansion_event_id {
                NodeDetailState::Expanded { expansion_event_id }
            } else if role != NodeRole::TaskRoot
                && map.node_state(node_id) == Some(NodeState::Completed)
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

fn active_frontier(map: &ActionMapInstance) -> HashSet<MapNodeId> {
    let frontier = map
        .all_nodes()
        .filter(|(role, node)| {
            *role == NodeRole::Work && map.node_state(&node.node_id) != Some(NodeState::Completed)
        })
        .map(|(_, node)| node.node_id.clone())
        .collect::<HashSet<_>>();
    if frontier.is_empty() {
        HashSet::from([map.finish.node_id.clone()])
    } else {
        frontier
    }
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
            if map.node(adjacent).is_some() && !distances.contains_key(adjacent) {
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
    use crate::action_map::map::MapNode;
    use crate::action_map::map::NodeEvent;
    use crate::action_map::rooted_dag::ActionReservation;
    use crate::action_map::rooted_dag::CompletionRecord;
    use crate::action_map::rooted_dag::MapEdge;
    use crate::action_map::rooted_dag::TaskSpaceMap;
    use codex_protocol::ThreadId;
    use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
    use std::collections::BTreeMap;

    fn chain(work_count: usize) -> TaskSpaceMap {
        let work_nodes = (1..=work_count)
            .map(|index| MapNode {
                node_id: format!("node-{index}"),
                goal: format!("Goal {index}"),
                source_refs: vec![],
            })
            .collect::<Vec<_>>();
        let mut edges = (1..=work_count)
            .map(|index| MapEdge {
                from: if index == 1 {
                    "root".into()
                } else {
                    format!("node-{}", index - 1)
                },
                to: format!("node-{index}"),
            })
            .collect::<Vec<_>>();
        edges.push(MapEdge {
            from: format!("node-{work_count}"),
            to: "finish".into(),
        });
        TaskSpaceMap {
            schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
            map_id: "map-1".into(),
            root: MapNode {
                node_id: "root".into(),
                goal: "Goal".into(),
                source_refs: vec![],
            },
            work_nodes,
            finish: MapNode {
                node_id: "finish".into(),
                goal: "Finish".into(),
                source_refs: vec![],
            },
            edges,
            completion_records: BTreeMap::new(),
            block_records: BTreeMap::new(),
            action_reservations: BTreeMap::new(),
            result_refs: BTreeMap::new(),
            evidence_refs: BTreeMap::new(),
            terminal_record: None,
            revision: 1,
        }
    }

    fn complete(map: &mut TaskSpaceMap, node_id: &str) {
        map.completion_records.insert(
            node_id.into(),
            CompletionRecord {
                action_id: format!("complete-{node_id}"),
                result_ref_ids: vec![],
                evidence_ref_ids: vec![],
            },
        );
    }

    #[test]
    fn folds_only_completed_non_root_nodes_at_distance_three_or_more() {
        let mut map = chain(4);
        complete(&mut map, "node-1");
        complete(&mut map, "node-2");
        complete(&mut map, "node-3");
        let map = ActionMapInstance::from_graph(map, vec![], None);

        let plan = node_detail_plan(&map);

        assert_eq!(plan.state("root"), Some(&NodeDetailState::Full));
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
    fn uses_distance_from_the_derived_inflight_frontier() {
        let mut map = chain(3);
        complete(&mut map, "node-1");
        complete(&mut map, "node-2");
        map.action_reservations.insert(
            "reservation-3".into(),
            ActionReservation {
                action_id: "action-3".into(),
                node_id: "node-3".into(),
                tool_name: "exec_command".into(),
                response_call_index: 1,
            },
        );
        let map = ActionMapInstance::from_graph(map, vec![], None);

        let plan = node_detail_plan(&map);

        assert_eq!(plan.state("node-1"), Some(&NodeDetailState::Full));
    }

    #[test]
    fn uses_finish_as_frontier_after_all_work_completes() {
        let mut map = chain(4);
        for node_id in ["node-1", "node-2", "node-3", "node-4"] {
            complete(&mut map, node_id);
        }
        let map = ActionMapInstance::from_graph(map, vec![], None);

        let plan = node_detail_plan(&map);

        assert_eq!(
            plan.state("node-1"),
            Some(&NodeDetailState::FoldEligible {
                frontier_distance: 4
            })
        );
        assert_eq!(plan.state("root"), Some(&NodeDetailState::Full));
    }

    #[test]
    fn expansion_evidence_overrides_fold_eligibility() {
        let mut map = chain(4);
        complete(&mut map, "node-1");
        complete(&mut map, "node-2");
        complete(&mut map, "node-3");
        let mut map = ActionMapInstance::from_graph(map, vec![], None);
        let event_id = "node-event-1".to_string();
        map.node_events.insert(
            event_id.clone(),
            NodeEvent {
                id: event_id.clone(),
                map_id: map.map_id.clone(),
                node_id: "node-1".into(),
                event_kind: NODE_DETAIL_EXPANDED_EVENT_KIND.into(),
                source: "agent_taskspace_control".into(),
                action_class: None,
                tool_success: None,
                content_sha256: "hash".into(),
                source_event_id: Some("task-event-1".into()),
                raw_ref: None,
                artifact_refs: vec![],
                call_id: Some("call-1".into()),
                source_thread_id: ThreadId::new(),
                created_at_ms: 1,
            },
        );

        let plan = node_detail_plan(&map);

        assert_eq!(
            plan.state("node-1"),
            Some(&NodeDetailState::Expanded {
                expansion_event_id: event_id
            })
        );
    }
}
