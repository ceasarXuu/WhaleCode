use super::*;

#[test]
fn projection_keeps_complete_skeleton_and_typed_details() {
    let input = ProjectionInput {
        map_id: "map-1".into(),
        revision: 2,
        canonical_sha256: "canonical-2".into(),
        root_node_id: "root".into(),
        finish_node_id: "finish".into(),
        complete: false,
        current_terminal: None,
        terminal_history: vec![],
        root_source_event_ids: vec!["task-event-1".into()],
        active_frontier: vec!["node-1".into()],
        map_nodes: vec![ProjectionNode {
            id: "node-1".into(),
            role: "work".into(),
            state: "in_flight".into(),
            goal: "Inspect".into(),
            result_ids: vec!["result-1".into()],
            event_count: 1,
            detail_state: None,
        }],
        map_edges: vec![],
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
    };
    let rendered = render_projection(input.clone(), ProjectionEnvelope::CurrentProjection);
    let rerendered = render_projection(input, ProjectionEnvelope::CurrentProjection);

    for field in [
        "map_id: map-1",
        "revision: 2",
        "canonical_sha256: canonical-2",
        "root_node_id: root",
        "finish_node_id: finish",
        "root_source_event_ids:",
        "active_frontier:",
        "node-1 role=work state=in_flight goal=\"Inspect\" result_ids=result-1",
        "map_edges:",
        "node_details:",
        "tier=D1 evidence=P1",
        "content_sha256=abc",
    ] {
        assert!(rendered.body.contains(field), "missing {field}");
    }
    assert!(!rendered.body.contains("projection_id"));
    assert!(!rendered.body.contains("current_node"));
    assert_eq!(rendered.body, rerendered.body);
    assert_eq!(rendered.projection_sha256, rerendered.projection_sha256);
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
    let rendered = render_projection(
        ProjectionInput {
            map_id: "map-large".into(),
            revision: 1,
            canonical_sha256: "canonical-large".into(),
            root_node_id: "node-0".into(),
            finish_node_id: "node-999".into(),
            complete: false,
            current_terminal: None,
            terminal_history: vec![],
            root_source_event_ids: vec!["task-event-root".into()],
            active_frontier: vec!["node-999".into()],
            map_nodes: (0..node_count)
                .map(|index| ProjectionNode {
                    id: format!("node-{index}"),
                    role: "work".into(),
                    state: "completed".into(),
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
            node_details: vec![],
        },
        ProjectionEnvelope::CurrentProjection,
    );

    assert_eq!(
        rendered
            .body
            .lines()
            .filter(|line| line.contains(" role=work state=completed goal="))
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
fn request_snapshot_envelope_marks_all_prior_projections_superseded() {
    let rendered = render_projection(
        ProjectionInput {
            map_id: "map-1".into(),
            revision: 7,
            canonical_sha256: "canonical-7".into(),
            root_node_id: "root".into(),
            finish_node_id: "finish".into(),
            complete: false,
            current_terminal: None,
            terminal_history: vec![],
            root_source_event_ids: vec![],
            active_frontier: vec!["work".into()],
            map_nodes: vec![],
            map_edges: vec![],
            node_details: vec![],
        },
        ProjectionEnvelope::RequestSnapshot,
    );

    assert!(
        rendered
            .body
            .contains("- projection_kind: request_snapshot")
    );
    assert!(
        rendered
            .body
            .contains("- supersedes_all_prior_projections: true")
    );
    assert!(
        rendered
            .body
            .contains("- current_state_rule: last_projection_only")
    );
    assert!(!rendered.body.contains("authoritative_current"));
}

#[test]
fn fold_does_not_activate_when_reference_metadata_would_increase_context() {
    let baseline = ProjectionNode {
        id: "node-1".into(),
        role: "work".into(),
        state: "completed".into(),
        goal: "Goal".into(),
        result_ids: vec![],
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
        artifact_refs: vec![],
    };
    let identity = node_detail_identity("map-1", "node-1", std::slice::from_ref(&event));
    let mut folded = baseline.clone();
    folded.detail_state = Some(ProjectionNodeDetailState::Folded {
        hidden_event_count: 1,
        detail_ref: identity.detail_ref,
    });

    assert!(!node_detail_fold_saves_bytes(&baseline, &folded, &[event]));
}
