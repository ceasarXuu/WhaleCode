use crate::action_map::detail_fold::NodeDetailState;
use crate::action_map::detail_fold::node_detail_plan;
use crate::action_map::map::ActionMapInstance;
use crate::action_map::projection::ProjectionEdge;
use crate::action_map::projection::ProjectionEnvelope;
use crate::action_map::projection::ProjectionEventRef;
use crate::action_map::projection::ProjectionInput;
use crate::action_map::projection::ProjectionNode;
use crate::action_map::projection::ProjectionNodeDetailState;
use crate::action_map::projection::render_projection;
use crate::action_map::rooted_dag::state_sha256;

use super::state::ActionMapRuntimeState;

impl ActionMapRuntimeState {
    pub(crate) fn build_developer_context(&self, envelope: ProjectionEnvelope) -> Option<String> {
        let map_id = self.active_map_id.as_deref()?;
        self.build_developer_context_for_map(map_id, envelope)
    }

    pub(crate) fn build_map_handle_context(&self) -> Option<String> {
        let map = self.active_map()?;
        Some(format!(
            "TaskSpaceMapHandleR7V1:\n- map_id: {}\n- revision: {}\n- bootstrap_required: false\n- request_snapshot_tool: taskspace_control\nTaskSpaceMapHandleR7V1 end.\n",
            map.map_id,
            map.canonical_map().revision
        ))
    }

    pub(crate) fn build_developer_context_for_map(
        &self,
        map_id: &str,
        envelope: ProjectionEnvelope,
    ) -> Option<String> {
        let map = self.maps.get(map_id)?;
        let input = projection_input(map).ok()?;
        Some(render_projection(input, envelope).body)
    }
}

fn projection_input(map: &ActionMapInstance) -> Result<ProjectionInput, serde_json::Error> {
    let graph = map.canonical_map();
    let canonical_sha256 = state_sha256(graph)?;
    let detail_plan = node_detail_plan(map);
    let active_frontier = map
        .node_views()
        .into_iter()
        .filter(|view| {
            view.state == crate::action_map::rooted_dag::NodeState::Ready
                || view.state == crate::action_map::rooted_dag::NodeState::InFlight
        })
        .map(|view| view.node_id)
        .collect();
    let map_nodes = map
        .all_nodes()
        .map(|(role, node)| {
            let detail_state = match detail_plan.state(&node.node_id) {
                Some(NodeDetailState::FoldEligible { .. }) => {
                    Some(ProjectionNodeDetailState::Folded {
                        hidden_event_count: map.event_ids_for_node(&node.node_id).len(),
                        detail_ref: format!("detail:{}", node.node_id),
                    })
                }
                Some(NodeDetailState::Expanded { expansion_event_id }) => {
                    Some(ProjectionNodeDetailState::Expanded {
                        expansion_event_id: expansion_event_id.clone(),
                    })
                }
                _ => None,
            };
            ProjectionNode {
                id: node.node_id.clone(),
                role: role.as_str().to_string(),
                state: map
                    .node_state(&node.node_id)
                    .map(crate::action_map::map::node_state_name)
                    .unwrap_or("unknown")
                    .to_string(),
                goal: node.goal.clone(),
                result_ids: map.result_ids_for_node(&node.node_id),
                event_count: map.event_ids_for_node(&node.node_id).len(),
                detail_state,
            }
        })
        .collect();
    Ok(ProjectionInput {
        map_id: graph.map_id.clone(),
        revision: graph.revision,
        canonical_sha256,
        root_node_id: graph.root.node_id.clone(),
        finish_node_id: graph.finish.node_id.clone(),
        complete: map.is_complete(),
        root_source_event_ids: graph.root.source_refs.clone(),
        active_frontier,
        map_nodes,
        map_edges: graph
            .edges
            .iter()
            .map(|edge| ProjectionEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
            })
            .collect(),
        node_details: map
            .node_events
            .values()
            .map(|event| ProjectionEventRef {
                id: event.id.clone(),
                node_id: event.node_id.clone(),
                event_kind: event.event_kind.clone(),
                source: event.source.clone(),
                detail_tier: "exact".to_string(),
                evidence_class: "runtime".to_string(),
                action_class: event.action_class.map(|class| class.as_str().to_string()),
                tool_success: event.tool_success,
                content_sha256: Some(event.content_sha256.clone()),
                raw_ref: event.raw_ref.clone(),
                artifact_refs: event.artifact_refs.clone(),
            })
            .collect(),
    })
}
