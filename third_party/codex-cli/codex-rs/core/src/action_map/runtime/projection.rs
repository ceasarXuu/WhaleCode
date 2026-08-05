use super::state::ActionMapRuntimeState;
use crate::action_map::detail_fold::NodeDetailState;
use crate::action_map::detail_fold::node_detail_plan;
use crate::action_map::map::ActionMapInstance;
use crate::action_map::projection::ProjectionEdge;
use crate::action_map::projection::ProjectionEnvelope;
use crate::action_map::projection::ProjectionEventRef;
use crate::action_map::projection::ProjectionInput;
use crate::action_map::projection::ProjectionNode;
use crate::action_map::projection::ProjectionNodeDetailState;
use crate::action_map::projection::ProjectionTerminal;
use crate::action_map::projection::render_empty_projection;
use crate::action_map::projection::render_projection;
use crate::action_map::rooted_dag::state_sha256;

impl ActionMapRuntimeState {
    pub(crate) fn build_developer_context(&self, envelope: ProjectionEnvelope) -> Option<String> {
        let map_id = self.active_map_id.as_deref()?;
        Some(
            self.build_developer_context_for_map(map_id, envelope)
                .unwrap_or_else(|| render_empty_projection(map_id, envelope)),
        )
    }

    pub(crate) fn build_map_handle_context(&self) -> Option<String> {
        let map_id = self.active_map_id.as_deref()?;
        let (revision, bootstrap_required, complete, initialization_action) =
            self.active_map().map_or(
                (
                    "none".to_string(),
                    true,
                    false,
                    Some("taskspace_control.initialize_and_execute"),
                ),
                |map| {
                    (
                        map.canonical_map().revision.to_string(),
                        false,
                        map.is_complete(),
                        None,
                    )
                },
            );
        let mut context = format!(
            "TaskSpaceMapHandleR7V1:\n- taskspace_active: true\n- map_id: {map_id}\n- revision: {revision}\n- bootstrap_required: {bootstrap_required}\n- complete: {complete}\n- available_read_action: taskspace_control.read_map\n"
        );
        if let Some(action) = initialization_action {
            context.push_str("- required_initialization_action: ");
            context.push_str(action);
            context.push('\n');
        }
        if complete {
            context.push_str("- available_reopen_action: taskspace_control.reopen_map\n");
        }
        context.push_str("TaskSpaceMapHandleR7V1 end.\n");
        Some(context)
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
            map.node_role(&view.node_id) == Some(crate::action_map::map::NodeRole::Work)
                && (view.state == crate::action_map::rooted_dag::NodeState::Ready
                    || view.state == crate::action_map::rooted_dag::NodeState::InFlight)
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
        current_terminal: graph.terminal_record.as_ref().map(projection_terminal),
        terminal_history: graph
            .terminal_history
            .iter()
            .map(projection_terminal)
            .collect(),
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

fn projection_terminal(
    terminal: &codex_protocol::taskspace::TaskSpaceTerminalRecord,
) -> ProjectionTerminal {
    ProjectionTerminal {
        action_id: terminal.action_id.clone(),
        summary_ref: terminal.summary_ref.clone(),
    }
}

#[cfg(test)]
mod tests {
    use codex_protocol::ThreadId;

    use super::projection_input;
    use crate::action_map::MapEdge;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;
    use crate::action_map::runtime::ActionMapRuntimeState;

    #[test]
    fn active_frontier_contains_executable_work_but_not_root() {
        let owner = ThreadId::new();
        let mut runtime = ActionMapRuntimeState::default();
        let map = new_map(
            "projection-map".into(),
            map_node("root", "Complete the task", Vec::new()),
            vec![map_node("inspect", "Inspect", Vec::new())],
            map_node("finish", "Finish", Vec::new()),
            vec![
                MapEdge {
                    from: "root".into(),
                    to: "inspect".into(),
                },
                MapEdge {
                    from: "inspect".into(),
                    to: "finish".into(),
                },
            ],
        );
        runtime
            .restore_store_map("projection-map", owner, Some(map))
            .expect("restore canonical map");

        let input =
            projection_input(runtime.active_map().expect("active map")).expect("projection input");

        assert_eq!(input.active_frontier, vec!["inspect"]);
    }
}
