use sha2::Digest;
use sha2::Sha256;

#[derive(Clone)]
pub(super) struct ActiveProjectionInput {
    pub(super) map_id: String,
    pub(super) revision: u64,
    pub(super) root_node_id: String,
    pub(super) finish_node_id: String,
    pub(super) complete: bool,
    pub(super) root_source_event_ids: Vec<String>,
    pub(super) current_node_id: Option<String>,
    pub(super) active_frontier: Vec<String>,
    pub(super) map_nodes: Vec<ProjectionNode>,
    pub(super) map_edges: Vec<ProjectionEdge>,
    pub(super) node_details: Vec<ProjectionEventRef>,
}

#[derive(Clone)]
pub(super) struct ProjectionNode {
    pub(super) id: String,
    pub(super) role: String,
    pub(super) status: String,
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

pub(super) fn render_active_projection(input: ActiveProjectionInput) -> RenderedProjection {
    let mut body = String::new();
    let mut size_breakdown = ProjectionSizeBreakdown::default();
    let section_start = body.len();
    body.push_str("TaskSpaceMapEpochSnapshotR6V1:\n");
    push_field(&mut body, "projection_role", "epoch_baseline");
    push_field(&mut body, "map_id", &input.map_id);
    push_field(&mut body, "revision", &input.revision.to_string());
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
    push_field(
        &mut body,
        "current_node",
        input.current_node_id.as_deref().unwrap_or("none"),
    );
    size_breakdown.header_bytes += body.len() - section_start;
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
    body.push_str("TaskSpaceMapEpochSnapshotR6V1 end.\n");
    size_breakdown.footer_bytes = body.len() - section_start;
    size_breakdown.projection_bytes = body.len();
    let estimated_tokens = body.len().div_ceil(4);
    RenderedProjection {
        body,
        estimated_tokens,
        skeleton_estimated_tokens,
        size_breakdown,
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
            let mut rendered = format!(
                "{} role={} status={} goal={:?} result_ids={} event_count={}",
                node.id,
                node.role,
                node.status,
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
mod tests {
    use super::*;

    #[test]
    fn projection_keeps_complete_skeleton_and_typed_details() {
        let rendered = render_active_projection(ActiveProjectionInput {
            map_id: "map-1".into(),
            revision: 2,
            root_node_id: "root".into(),
            finish_node_id: "finish".into(),
            complete: false,
            root_source_event_ids: vec!["task-event-1".into()],
            current_node_id: Some("node-1".into()),
            active_frontier: vec!["node-1".into()],
            map_nodes: vec![ProjectionNode {
                id: "node-1".into(),
                role: "work".into(),
                status: "running".into(),
                goal: "Inspect".into(),
                result_ids: vec!["result-1".into()],
                event_count: 1,
                detail_state: None,
            }],
            map_edges: Vec::new(),
            node_details: vec![ProjectionEventRef {
                id: "task-event-2".into(),
                node_id: "node-1".into(),
                event_kind: "tool_result".into(),
                source: "main_tool".into(),
                detail_tier: "D1".into(),
                evidence_class: "P1".into(),
                action_class: Some("read".into()),
                tool_success: Some(true),
                content_sha256: Some("abc".into()),
                raw_ref: Some("output-ref-1".into()),
                artifact_refs: vec!["src/lib.rs".into()],
            }],
        });

        for field in [
            "map_id: map-1",
            "revision: 2",
            "root_node_id: root",
            "finish_node_id: finish",
            "root_source_event_ids:",
            "current_node: node-1",
            "active_frontier:",
            "node-1 role=work status=running goal=\"Inspect\" result_ids=result-1",
            "map_edges:",
            "node_details:",
            "tier=D1 evidence=P1",
            "content_sha256=abc",
        ] {
            assert!(rendered.body.contains(field), "missing {field}");
        }
        assert!(!rendered.body.contains("projection_id"));
        assert!(rendered.skeleton_estimated_tokens < rendered.estimated_tokens);
        assert_eq!(
            rendered.size_breakdown.projection_bytes,
            rendered.size_breakdown.header_bytes
                + rendered.size_breakdown.root_source_bytes
                + rendered.size_breakdown.active_frontier_bytes
                + rendered.size_breakdown.map_node_bytes
                + rendered.size_breakdown.map_edge_bytes
                + rendered.size_breakdown.node_detail_bytes
                + rendered.size_breakdown.footer_bytes
        );
        assert_eq!(
            rendered.size_breakdown.projection_bytes,
            rendered.body.len()
        );
        assert_eq!(
            rendered.size_breakdown.skeleton_bytes.div_ceil(4),
            rendered.skeleton_estimated_tokens
        );
    }

    #[test]
    fn projection_does_not_page_large_skeletons() {
        let node_count = 1_000;
        let rendered = render_active_projection(ActiveProjectionInput {
            map_id: "map-large".into(),
            revision: 1,
            root_node_id: "node-0".into(),
            finish_node_id: "node-999".into(),
            complete: false,
            root_source_event_ids: vec!["task-event-root".into()],
            current_node_id: Some("node-999".into()),
            active_frontier: vec!["node-999".into()],
            map_nodes: (0..node_count)
                .map(|index| ProjectionNode {
                    id: format!("node-{index}"),
                    role: "work".into(),
                    status: "completed".into(),
                    goal: format!("goal-{index}"),
                    result_ids: vec![format!("result-{index}")],
                    event_count: 1,
                    detail_state: None,
                })
                .collect(),
            map_edges: (1..node_count)
                .map(|index| ProjectionEdge {
                    from: format!("node-{}", index - 1),
                    to: format!("node-{index}"),
                })
                .collect(),
            node_details: Vec::new(),
        });

        assert_eq!(
            rendered
                .body
                .lines()
                .filter(|line| line.contains(" role=work status=completed goal="))
                .count(),
            node_count
        );
        assert_eq!(
            rendered
                .body
                .lines()
                .filter(|line| line.trim_start().starts_with("- node-") && line.contains("->"))
                .count(),
            node_count - 1
        );
        assert!(!rendered.body.contains("omitted"));
        assert!(!rendered.body.contains("cursor"));
    }

    #[test]
    fn fold_does_not_activate_when_reference_metadata_would_increase_context() {
        let baseline = ProjectionNode {
            id: "node-1".into(),
            role: "work".into(),
            status: "completed".into(),
            goal: "Goal".into(),
            result_ids: Vec::new(),
            event_count: 1,
            detail_state: None,
        };
        let event = ProjectionEventRef {
            id: "e".into(),
            node_id: "node-1".into(),
            event_kind: "x".into(),
            source: "x".into(),
            detail_tier: "D3".into(),
            evidence_class: "P3".into(),
            action_class: None,
            tool_success: None,
            content_sha256: None,
            raw_ref: None,
            artifact_refs: Vec::new(),
        };
        let identity = node_detail_identity("map-1", "node-1", std::slice::from_ref(&event));
        let mut folded = baseline.clone();
        folded.detail_state = Some(ProjectionNodeDetailState::Folded {
            hidden_event_count: 1,
            detail_ref: identity.detail_ref,
        });

        assert!(!node_detail_fold_saves_bytes(&baseline, &folded, &[event]));
    }
}
