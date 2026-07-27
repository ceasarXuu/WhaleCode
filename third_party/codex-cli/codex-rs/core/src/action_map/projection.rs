use sha2::Digest;
use sha2::Sha256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionEnvelope {
    CurrentProjection,
    RequestSnapshot,
}

impl ProjectionEnvelope {
    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::CurrentProjection => "current_projection",
            Self::RequestSnapshot => "request_snapshot",
        }
    }
}

#[derive(Clone)]
pub(super) struct ProjectionInput {
    pub(super) map_id: String,
    pub(super) revision: u64,
    pub(super) canonical_sha256: String,
    pub(super) root_node_id: String,
    pub(super) finish_node_id: String,
    pub(super) complete: bool,
    pub(super) root_source_event_ids: Vec<String>,
    pub(super) active_frontier: Vec<String>,
    pub(super) map_nodes: Vec<ProjectionNode>,
    pub(super) map_edges: Vec<ProjectionEdge>,
    pub(super) node_details: Vec<ProjectionEventRef>,
}

#[derive(Clone)]
pub(super) struct ProjectionNode {
    pub(super) id: String,
    pub(super) role: String,
    pub(super) state: String,
    pub(super) goal: String,
    pub(super) result_ids: Vec<String>,
    pub(super) event_count: usize,
    pub(super) detail_state: Option<ProjectionNodeDetailState>,
}

#[derive(Clone)]
pub(super) enum ProjectionNodeDetailState {
    Folded {
        hidden_event_count: usize,
        detail_ref: String,
    },
    Expanded {
        expansion_event_id: String,
    },
}

#[derive(Clone)]
pub(super) struct ProjectionEdge {
    pub(super) from: String,
    pub(super) to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectionEventRef {
    pub(super) id: String,
    pub(super) node_id: String,
    pub(super) event_kind: String,
    pub(super) source: String,
    pub(super) detail_tier: String,
    pub(super) evidence_class: String,
    pub(super) action_class: Option<String>,
    pub(super) tool_success: Option<bool>,
    pub(super) content_sha256: Option<String>,
    pub(super) raw_ref: Option<String>,
    pub(super) artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectionNodeDetailIdentity {
    pub(super) detail_ref: String,
}

pub(super) struct RenderedProjection {
    pub(super) body: String,
    pub(super) projection_sha256: String,
    pub(super) estimated_tokens: usize,
    pub(super) skeleton_estimated_tokens: usize,
    pub(super) size_breakdown: ProjectionSizeBreakdown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ProjectionSizeBreakdown {
    pub(super) header_bytes: usize,
    pub(super) root_source_bytes: usize,
    pub(super) active_frontier_bytes: usize,
    pub(super) map_node_bytes: usize,
    pub(super) map_edge_bytes: usize,
    pub(super) node_detail_bytes: usize,
    pub(super) footer_bytes: usize,
    pub(super) skeleton_bytes: usize,
    pub(super) projection_bytes: usize,
}

pub(super) fn render_projection(
    input: ProjectionInput,
    envelope: ProjectionEnvelope,
) -> RenderedProjection {
    let mut body = String::new();
    let mut size_breakdown = ProjectionSizeBreakdown::default();
    let section_start = body.len();
    body.push_str("TaskSpaceMapProjectionR7V1:\n");
    push_field(
        &mut body,
        "schema_version",
        "taskspace-map-projection-r7-v1",
    );
    push_field(&mut body, "projection_kind", envelope.kind());
    push_field(&mut body, "map_id", &input.map_id);
    push_field(&mut body, "revision", &input.revision.to_string());
    if envelope == ProjectionEnvelope::RequestSnapshot {
        push_field(&mut body, "supersedes_all_prior_projections", "true");
        push_field(&mut body, "current_state_rule", "last_projection_only");
    }
    push_field(&mut body, "canonical_sha256", &input.canonical_sha256);
    push_field(&mut body, "root_node_id", &input.root_node_id);
    push_field(&mut body, "finish_node_id", &input.finish_node_id);
    push_field(
        &mut body,
        "complete",
        if input.complete { "true" } else { "false" },
    );
    size_breakdown.header_bytes += body.len() - section_start;
    let section_start = body.len();
    append_list(
        &mut body,
        "root_source_event_ids",
        &input.root_source_event_ids,
    );
    size_breakdown.root_source_bytes = body.len() - section_start;
    let section_start = body.len();
    append_list(&mut body, "active_frontier", &input.active_frontier);
    size_breakdown.active_frontier_bytes = body.len() - section_start;
    let section_start = body.len();
    append_list(&mut body, "map_nodes", &render_nodes(&input.map_nodes));
    size_breakdown.map_node_bytes = body.len() - section_start;
    let section_start = body.len();
    append_list(&mut body, "map_edges", &render_edges(&input.map_edges));
    size_breakdown.map_edge_bytes = body.len() - section_start;
    size_breakdown.skeleton_bytes = body.len();
    let skeleton_estimated_tokens = body.len().div_ceil(4);
    let section_start = body.len();
    append_list(
        &mut body,
        "node_details",
        &render_event_refs(&input.node_details),
    );
    size_breakdown.node_detail_bytes = body.len() - section_start;
    let section_start = body.len();
    body.push_str("TaskSpaceMapProjectionR7V1 end.\n");
    size_breakdown.footer_bytes = body.len() - section_start;
    size_breakdown.projection_bytes = body.len();
    let estimated_tokens = body.len().div_ceil(4);
    let projection_sha256 = format!(
        "{:x}",
        Sha256::digest(body.trim_end_matches('\n').as_bytes())
    );
    RenderedProjection {
        body,
        projection_sha256,
        estimated_tokens,
        skeleton_estimated_tokens,
        size_breakdown,
    }
}

pub(super) fn render_empty_projection(map_id: &str, envelope: ProjectionEnvelope) -> String {
    let mut body = String::new();
    body.push_str("TaskSpaceMapProjectionR7V1:\n");
    push_field(
        &mut body,
        "schema_version",
        "taskspace-map-projection-r7-v1",
    );
    push_field(&mut body, "projection_kind", envelope.kind());
    push_field(&mut body, "map_id", map_id);
    if envelope == ProjectionEnvelope::RequestSnapshot {
        push_field(&mut body, "supersedes_all_prior_projections", "true");
        push_field(&mut body, "current_state_rule", "last_projection_only");
    }
    push_field(&mut body, "map", "none");
    push_field(&mut body, "bootstrap_required", "true");
    push_field(
        &mut body,
        "required_initialization_action",
        "taskspace_control.initialize_and_execute",
    );
    append_list(&mut body, "active_frontier", &[]);
    append_list(&mut body, "map_nodes", &[]);
    append_list(&mut body, "map_edges", &[]);
    append_list(&mut body, "node_details", &[]);
    body.push_str("TaskSpaceMapProjectionR7V1 end.\n");
    body
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
            let mut rendered = format!(
                "{} role={} state={} goal={:?} result_ids={} event_count={}",
                node.id,
                node.role,
                node.state,
                node.goal,
                if node.result_ids.is_empty() {
                    "none".to_string()
                } else {
                    node.result_ids.join(",")
                },
                node.event_count,
            );
            match &node.detail_state {
                Some(ProjectionNodeDetailState::Folded {
                    hidden_event_count,
                    detail_ref,
                }) => rendered.push_str(&format!(
                    " detail_state=folded hidden_event_count={hidden_event_count} detail_ref={detail_ref}"
                )),
                Some(ProjectionNodeDetailState::Expanded { expansion_event_id }) => rendered
                    .push_str(&format!(
                        " detail_state=expanded expansion_event_id={expansion_event_id}"
                    )),
                None => {}
            }
            rendered
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
                "{} node={} tier={} evidence={} event_kind={} source={}",
                event.id,
                event.node_id,
                event.detail_tier,
                event.evidence_class,
                event.event_kind,
                event.source
            );
            if let Some(action_class) = event.action_class.as_deref() {
                rendered.push_str(&format!(" action_class={action_class}"));
            }
            if let Some(tool_success) = event.tool_success {
                rendered.push_str(&format!(" tool_success={tool_success}"));
            }
            if let Some(content_sha256) = event.content_sha256.as_deref() {
                rendered.push_str(&format!(" content_sha256={content_sha256}"));
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

pub(super) fn node_detail_identity(
    map_id: &str,
    node_id: &str,
    events: &[ProjectionEventRef],
) -> ProjectionNodeDetailIdentity {
    let mut payload = format!("NodeDetailProjectionV1\nmap_id:{map_id}\nnode_id:{node_id}\n");
    append_list(&mut payload, "node_details", &render_event_refs(events));
    let detail_sha256 = format!("{:x}", Sha256::digest(payload.as_bytes()));
    ProjectionNodeDetailIdentity {
        detail_ref: format!("taskspace-detail://sha256/{detail_sha256}"),
    }
}

pub(super) fn node_detail_fold_saves_bytes(
    baseline_node: &ProjectionNode,
    folded_node: &ProjectionNode,
    events: &[ProjectionEventRef],
) -> bool {
    let baseline_node_bytes = render_nodes(std::slice::from_ref(baseline_node))[0].len();
    let folded_node_bytes = render_nodes(std::slice::from_ref(folded_node))[0].len();
    let removed_detail_bytes = render_event_refs(events)
        .iter()
        .map(|event| "    - ".len() + event.len() + '\n'.len_utf8())
        .sum::<usize>();
    folded_node_bytes < baseline_node_bytes.saturating_add(removed_detail_bytes)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
