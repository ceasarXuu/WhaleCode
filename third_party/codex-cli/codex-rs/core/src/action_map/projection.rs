pub(super) struct ActiveProjectionInput {
    pub(super) projection_id: String,
    pub(super) task_id: String,
    pub(super) task_title: String,
    pub(super) task_status: String,
    pub(super) map_id: String,
    pub(super) map_title: String,
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
    body.push('\n');
    body.push_str("ContextProjectionV1 epoch snapshot:\n");
    push_field(&mut body, "projection_id", &input.projection_id);
    push_field(&mut body, "task_id", &input.task_id);
    push_field(&mut body, "task_title", &input.task_title);
    push_field(&mut body, "task_status", &input.task_status);
    push_field(&mut body, "map_id", &input.map_id);
    push_field(&mut body, "map_title", &input.map_title);
    push_field(&mut body, "map_status", &input.map_status);
    push_field(&mut body, "mode", "taskspace");
    push_field(&mut body, "active_objective", &input.active_objective);
    if input.mechanically_blank {
        push_field(
            &mut body,
            "initialization_source",
            "runtime_mechanical_blank",
        );
        push_field(
            &mut body,
            "semantic_state",
            "agent-authored objective and node plan pending",
        );
        push_field(&mut body, "hard_state", "active_task_path_without_nodes");
        push_field(
            &mut body,
            "initialization_contract",
            "taskspace_control(action=initialize_map)",
        );
    }
    push_field(&mut body, "current_node", &input.current_node);
    body.push_str("- sections:\n");
    append_list(&mut body, "map_nodes", &input.map_nodes, false);
    append_list(
        &mut body,
        "current_node_dependencies",
        &input.current_node_dependencies,
        false,
    );
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
