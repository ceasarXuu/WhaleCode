#[derive(Clone)]
pub(super) struct ActiveProjectionInput {
    pub(super) task_id: String,
    pub(super) task_status: String,
    pub(super) map_id: String,
    pub(super) map_status: String,
    pub(super) root_source_event_ids: Vec<String>,
    pub(super) current_node_id: Option<String>,
    pub(super) active_frontier: Vec<String>,
    pub(super) map_nodes: Vec<ProjectionNode>,
    pub(super) map_node_archives: Vec<ProjectionArchiveIndex>,
    pub(super) map_edges: Vec<ProjectionEdge>,
    pub(super) node_details: Vec<ProjectionEventRef>,
}

#[derive(Clone)]
pub(super) struct ProjectionNode {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) status: String,
    pub(super) goal: String,
    pub(super) result_ids: Vec<String>,
    pub(super) event_count: usize,
}

#[derive(Clone)]
pub(super) struct ProjectionArchiveIndex {
    pub(super) archive_ref: String,
    pub(super) covered_node_count: usize,
    pub(super) covered_node_ids_sha256: String,
    pub(super) canonical_payload_sha256: String,
    pub(super) terminal_status: String,
    pub(super) result_refs: Vec<String>,
}

#[derive(Clone)]
pub(super) struct ProjectionEdge {
    pub(super) from: String,
    pub(super) to: String,
}

#[derive(Clone)]
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
    pub(super) map_node_archive_bytes: usize,
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
    body.push_str("ContextProjectionV1 epoch snapshot:\n");
    push_field(&mut body, "task_id", &input.task_id);
    push_field(&mut body, "map_id", &input.map_id);
    push_field(&mut body, "task_status", &input.task_status);
    push_field(&mut body, "map_status", &input.map_status);
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
    if !input.map_node_archives.is_empty() {
        let section_start = body.len();
        append_list(
            &mut body,
            "map_node_archives",
            &render_archive_indexes(&input.map_node_archives),
        );
        size_breakdown.map_node_archive_bytes = body.len() - section_start;
    }
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
    body.push_str("ContextProjectionV1 epoch snapshot end.\n");
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
            format!(
                "{} kind={} status={} goal={:?} result_ids={} event_count={}",
                node.id,
                node.kind,
                node.status,
                node.goal,
                if node.result_ids.is_empty() {
                    "none".to_string()
                } else {
                    node.result_ids.join(",")
                },
                node.event_count,
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

fn render_archive_indexes(archives: &[ProjectionArchiveIndex]) -> Vec<String> {
    archives
        .iter()
        .map(|archive| {
            format!(
                "strategy=S1 archive_ref={} covered_node_count={} covered_node_ids_sha256={} canonical_payload_sha256={} terminal_status={} result_refs={}",
                archive.archive_ref,
                archive.covered_node_count,
                archive.covered_node_ids_sha256,
                archive.canonical_payload_sha256,
                archive.terminal_status,
                if archive.result_refs.is_empty() {
                    "none".to_string()
                } else {
                    archive.result_refs.join(",")
                },
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_keeps_complete_skeleton_and_typed_details() {
        let rendered = render_active_projection(ActiveProjectionInput {
            task_id: "task-1".into(),
            task_status: "active".into(),
            map_id: "map-1".into(),
            map_status: "active".into(),
            root_source_event_ids: vec!["task-event-1".into()],
            current_node_id: Some("node-1".into()),
            active_frontier: vec!["node-1".into()],
            map_nodes: vec![ProjectionNode {
                id: "node-1".into(),
                kind: "inspect_code_context".into(),
                status: "running".into(),
                goal: "Inspect".into(),
                result_ids: vec!["result-1".into()],
                event_count: 1,
            }],
            map_node_archives: Vec::new(),
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
            "task_id: task-1",
            "map_id: map-1",
            "root_source_event_ids:",
            "current_node: node-1",
            "active_frontier:",
            "node-1 kind=inspect_code_context status=running goal=\"Inspect\" result_ids=result-1",
            "map_edges:",
            "node_details:",
            "tier=D1 evidence=P1",
            "content_sha256=abc",
        ] {
            assert!(rendered.body.contains(field), "missing {field}");
        }
        assert!(!rendered.body.contains("projection_id"));
        assert!(!rendered.body.contains("map_node_archives"));
        assert!(rendered.skeleton_estimated_tokens < rendered.estimated_tokens);
        assert_eq!(
            rendered.size_breakdown.projection_bytes,
            rendered.size_breakdown.header_bytes
                + rendered.size_breakdown.root_source_bytes
                + rendered.size_breakdown.active_frontier_bytes
                + rendered.size_breakdown.map_node_bytes
                + rendered.size_breakdown.map_node_archive_bytes
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
            task_id: "task-large".into(),
            task_status: "active".into(),
            map_id: "map-large".into(),
            map_status: "active".into(),
            root_source_event_ids: vec!["task-event-root".into()],
            current_node_id: Some("node-999".into()),
            active_frontier: vec!["node-999".into()],
            map_nodes: (0..node_count)
                .map(|index| ProjectionNode {
                    id: format!("node-{index}"),
                    kind: "custom".into(),
                    status: "completed".into(),
                    goal: format!("goal-{index}"),
                    result_ids: vec![format!("result-{index}")],
                    event_count: 1,
                })
                .collect(),
            map_node_archives: Vec::new(),
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
                .filter(|line| line.contains(" kind=custom status=completed goal="))
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
    fn projection_renders_only_mechanical_archive_index_fields() {
        let rendered = render_active_projection(ActiveProjectionInput {
            task_id: "task-archive".into(),
            task_status: "active".into(),
            map_id: "map-archive".into(),
            map_status: "active".into(),
            root_source_event_ids: vec!["task-event-root".into()],
            current_node_id: Some("active".into()),
            active_frontier: vec!["active".into()],
            map_nodes: Vec::new(),
            map_node_archives: vec![ProjectionArchiveIndex {
                archive_ref: "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                covered_node_count: 3,
                covered_node_ids_sha256: "covered-hash".into(),
                canonical_payload_sha256: "payload-hash".into(),
                terminal_status: "completed".into(),
                result_refs: vec!["result-a".into(), "result-b".into()],
            }],
            map_edges: Vec::new(),
            node_details: Vec::new(),
        });

        assert!(rendered.body.contains("map_node_archives:"));
        assert!(rendered.body.contains("strategy=S1"));
        assert!(rendered.body.contains("covered_node_count=3"));
        assert!(rendered.body.contains("result_refs=result-a,result-b"));
        assert!(!rendered.body.contains("summary="));
        assert!(rendered.size_breakdown.map_node_archive_bytes > 0);
    }
}
