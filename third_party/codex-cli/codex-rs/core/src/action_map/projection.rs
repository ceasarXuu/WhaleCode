pub(super) struct ActiveProjectionInput {
    pub(super) task_id: String,
    pub(super) task_title: String,
    pub(super) task_status: String,
    pub(super) map_id: String,
    pub(super) map_status: String,
    pub(super) active_objective: String,
    pub(super) current_node: String,
    pub(super) map_nodes: Vec<String>,
    pub(super) current_node_dependencies: Vec<String>,
    pub(super) current_node_recent_events: Vec<String>,
    pub(super) result_refs_available: Vec<String>,
    pub(super) mechanically_blank: bool,
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
        push_field(&mut body, "task_title", &input.task_title);
        push_field(&mut body, "task_status", &input.task_status);
        push_field(&mut body, "map_status", &input.map_status);
        push_field(&mut body, "active_objective", &input.active_objective);
    }
    push_field(&mut body, "current_node", &input.current_node);
    append_list(&mut body, "map_nodes", &input.map_nodes, false);
    if !input.mechanically_blank {
        append_list(
            &mut body,
            "current_node_dependencies",
            &input.current_node_dependencies,
            false,
        );
    }
    append_list(
        &mut body,
        "current_node_recent_events",
        &input.current_node_recent_events,
        true,
    );
    append_list(
        &mut body,
        "result_refs_available",
        &input.result_refs_available,
        false,
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

fn append_list(body: &mut String, label: &str, values: &[String], multiline: bool) {
    body.push_str("  ");
    body.push_str(label);
    body.push_str(":\n");
    if values.is_empty() {
        body.push_str("    - none\n");
        return;
    }
    for value in values {
        let mut lines = value.lines();
        let first_line = lines.next().unwrap_or_default();
        body.push_str("    - ");
        body.push_str(first_line);
        body.push('\n');
        if multiline {
            for line in lines {
                body.push_str("      ");
                body.push_str(line);
                body.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanical_blank_epoch_base_is_sparse_and_complete() {
        let rendered = render_active_projection(ActiveProjectionInput {
            task_id: "task-1".into(),
            task_title: "TaskSpace blank task".into(),
            task_status: "active".into(),
            map_id: "map-1".into(),
            map_status: "active".into(),
            active_objective: "Agent-authored objective pending".into(),
            current_node: "none".into(),
            map_nodes: Vec::new(),
            current_node_dependencies: Vec::new(),
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
