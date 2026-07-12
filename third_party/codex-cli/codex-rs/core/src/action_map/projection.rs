pub(super) struct ActiveProjectionInput {
    pub(super) task_id: String,
    pub(super) task_status: String,
    pub(super) map_id: String,
    pub(super) map_status: String,
    pub(super) source_event_ids: Vec<String>,
    pub(super) current_node_id: Option<String>,
    pub(super) map_nodes: Vec<ProjectionNode>,
    pub(super) map_edges: Vec<ProjectionEdge>,
    pub(super) current_node_recent_events: Vec<ProjectionEventRef>,
    pub(super) result_refs_available: Vec<ProjectionEventRef>,
    pub(super) mechanically_blank: bool,
}

pub(super) struct ProjectionNode {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) status: String,
    pub(super) goal: String,
    pub(super) result_count: usize,
    pub(super) event_count: usize,
}

pub(super) struct ProjectionEdge {
    pub(super) from: String,
    pub(super) to: String,
}

pub(super) struct ProjectionEventRef {
    pub(super) id: String,
    pub(super) node_id: String,
    pub(super) event_kind: String,
    pub(super) source: String,
    pub(super) action_class: Option<String>,
    pub(super) tool_success: Option<bool>,
    pub(super) raw_ref: Option<String>,
    pub(super) artifact_refs: Vec<String>,
}

pub(super) struct RenderedProjection {
    pub(super) body: String,
    pub(super) estimated_tokens: usize,
}

pub(super) fn render_active_projection(input: ActiveProjectionInput) -> RenderedProjection {
    let mut body = String::new();
    body.push_str("ContextProjectionV1 epoch snapshot:\n");
    push_field(&mut body, "task_id", &input.task_id);
    push_field(&mut body, "map_id", &input.map_id);
    if input.mechanically_blank {
        push_field(&mut body, "hard_state", "active_task_path_without_nodes");
    } else {
        push_field(&mut body, "task_status", &input.task_status);
        push_field(&mut body, "map_status", &input.map_status);
        append_list(&mut body, "source_event_ids", &input.source_event_ids);
    }
    push_field(
        &mut body,
        "current_node",
        input.current_node_id.as_deref().unwrap_or("none"),
    );
    append_list(&mut body, "map_nodes", &render_nodes(&input.map_nodes));
    append_list(&mut body, "map_edges", &render_edges(&input.map_edges));
    append_list(
        &mut body,
        "current_node_recent_events",
        &render_event_refs(&input.current_node_recent_events),
    );
    append_list(
        &mut body,
        "result_refs_available",
        &render_event_refs(&input.result_refs_available),
    );
    body.push_str("ContextProjectionV1 epoch snapshot end.\n");
    let estimated_tokens = body.len().div_ceil(4);
    RenderedProjection {
        body,
        estimated_tokens,
    }
}

fn push_field(body: &mut String, label: &str, value: &str) {
    body.push_str("- ");
    body.push_str(label);
    body.push_str(": ");
    body.push_str(value);
    body.push('\n');
}

fn append_list(body: &mut String, label: &str, values: &[String]) {
    body.push_str("  ");
    body.push_str(label);
    body.push_str(":\n");
    if values.is_empty() {
        body.push_str("    - none\n");
        return;
    }
    for value in values {
        body.push_str("    - ");
        body.push_str(value);
        body.push('\n');
    }
}

fn render_nodes(nodes: &[ProjectionNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| {
            format!(
                "{} kind={} status={} goal={:?} result_count={} event_count={}",
                node.id, node.kind, node.status, node.goal, node.result_count, node.event_count,
            )
        })
        .collect()
}

fn render_edges(edges: &[ProjectionEdge]) -> Vec<String> {
    edges
        .iter()
        .map(|edge| format!("{}->{}", edge.from, edge.to))
        .collect()
}

fn render_event_refs(events: &[ProjectionEventRef]) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            let mut rendered = format!(
                "{} node={} event_kind={} source={}",
                event.id, event.node_id, event.event_kind, event.source
            );
            if let Some(action_class) = event.action_class.as_deref() {
                rendered.push_str(&format!(" action_class={action_class}"));
            }
            if let Some(tool_success) = event.tool_success {
                rendered.push_str(&format!(" tool_success={tool_success}"));
            }
            if let Some(raw_ref) = event.raw_ref.as_deref() {
                rendered.push_str(&format!(" raw_ref={raw_ref}"));
            }
            if !event.artifact_refs.is_empty() {
                rendered.push_str(&format!(" artifact_refs={}", event.artifact_refs.join(",")));
            }
            rendered
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanical_blank_epoch_base_is_sparse_and_complete() {
        let rendered = render_active_projection(ActiveProjectionInput {
            task_id: "task-1".into(),
            task_status: "active".into(),
            map_id: "map-1".into(),
            map_status: "active".into(),
            source_event_ids: Vec::new(),
            current_node_id: None,
            map_nodes: Vec::new(),
            map_edges: Vec::new(),
            current_node_recent_events: Vec::new(),
            result_refs_available: Vec::new(),
            mechanically_blank: true,
        });

        for field in [
            "task_id: task-1",
            "map_id: map-1",
            "hard_state: active_task_path_without_nodes",
            "current_node: none",
            "map_nodes:",
            "map_edges:",
            "current_node_recent_events:",
            "result_refs_available:",
        ] {
            assert!(rendered.body.contains(field), "missing {field}");
        }
        assert!(!rendered.body.contains("projection_id"));
        assert!(!rendered.body.contains("TaskSpace blank"));
        assert!(!rendered.body.contains("initialization_contract"));
        assert!(rendered.estimated_tokens < 80);
    }
}
