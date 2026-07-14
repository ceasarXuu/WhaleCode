use super::projection::ActiveProjectionInput;
use super::projection::ProjectionEdge;
use super::projection::ProjectionEventRef;
use super::projection::ProjectionNode;
use super::projection::render_active_projection;
use super::snapshot_delta::ActionMapCheckpointState;
use super::snapshot_delta::apply_snapshot_delta;
use super::snapshot_delta::build_snapshot_delta;
use super::snapshot_delta::snapshot_sha256;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotEdge;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotNode;
use codex_protocol::protocol::ActionMapSnapshotNodeEvent;
use codex_protocol::protocol::ActionMapSnapshotResult;
use codex_protocol::protocol::ActionMapSnapshotTask;
use codex_protocol::protocol::MapRuntimeMode;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

const NODE_COUNTS: [usize; 3] = [100, 1_000, 10_000];
const BUDGETS: [usize; 5] = [12_000, 16_000, 24_000, 32_000, 48_000];
const EDGE_PROFILES: [(&str, usize); 3] = [("none", 0), ("chain", 1), ("forward_4", 4)];

#[derive(Serialize)]
struct K0ScaleProbe {
    schema_version: &'static str,
    projection_rows: Vec<ProjectionScaleRow>,
    budget_crossings: Vec<BudgetCrossingRow>,
    replay_rows: Vec<ReplayScaleRow>,
}

#[derive(Serialize)]
struct ProjectionScaleRow {
    node_count: usize,
    edge_profile: &'static str,
    edge_count: usize,
    active_frontier_count: usize,
    result_ref_count: usize,
    detail_count: usize,
    input_build_duration_us: u128,
    render_duration_us: u128,
    estimated_tokens: usize,
    skeleton_estimated_tokens: usize,
    header_bytes: usize,
    root_source_bytes: usize,
    active_frontier_bytes: usize,
    map_node_bytes: usize,
    map_edge_bytes: usize,
    node_detail_bytes: usize,
    footer_bytes: usize,
    skeleton_bytes: usize,
    projection_bytes: usize,
}

#[derive(Serialize)]
struct BudgetCrossingRow {
    edge_profile: &'static str,
    max_projection_tokens: usize,
    first_over_budget_node_count: Option<usize>,
    skeleton_tokens_at_crossing: Option<usize>,
}

#[derive(Serialize)]
struct ReplayScaleRow {
    initial_node_count: usize,
    final_node_count: usize,
    checkpoint_cycles: usize,
    nodes_appended_per_cycle: usize,
    checkpoint_bytes: usize,
    delta_bytes: usize,
    final_snapshot_bytes: usize,
    snapshot_generation_duration_us: u128,
    delta_build_duration_us: u128,
    replay_duration_us: u128,
    replay_exact: bool,
    final_snapshot_sha256: String,
}

fn projection_input(node_count: usize, edge_width: usize, details: bool) -> ActiveProjectionInput {
    let current = node_count.saturating_sub(1);
    ActiveProjectionInput {
        task_id: "task-k0-scale".into(),
        task_status: "active".into(),
        map_id: "map-k0-scale".into(),
        map_status: "active".into(),
        root_source_event_ids: vec!["task-event-root".into()],
        current_node_id: Some(format!("node-{current}")),
        active_frontier: vec![format!("node-{current}")],
        map_nodes: (0..node_count)
            .map(|index| ProjectionNode {
                id: format!("node-{index}"),
                kind: node_kind(index).into(),
                status: if index == current {
                    "running".into()
                } else {
                    "completed".into()
                },
                goal: format!("Execute deterministic K0 work item {index}"),
                result_ids: vec![format!("result-{index}")],
                event_count: usize::from(details),
                detail_state: "full".into(),
                frontier_distance: None,
                detail_ref: None,
                detail_sha256: None,
                expansion_event_id: None,
            })
            .collect(),
        map_edges: projection_edges(node_count, edge_width),
        node_details: if details {
            (0..node_count)
                .map(|index| ProjectionEventRef {
                    id: format!("task-event-{index}"),
                    node_id: format!("node-{index}"),
                    event_kind: "tool_result".into(),
                    source: "main_tool".into(),
                    detail_tier: if index == current { "D1" } else { "D3" }.into(),
                    evidence_class: "P1".into(),
                    action_class: Some("read".into()),
                    tool_success: Some(true),
                    content_sha256: Some(format!("{index:064x}")),
                    raw_ref: Some(format!("output-ref-{index}")),
                    artifact_refs: vec![format!("src/module_{index}.rs")],
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn projection_edges(node_count: usize, edge_width: usize) -> Vec<ProjectionEdge> {
    (0..node_count)
        .flat_map(|from| {
            (1..=edge_width)
                .filter(move |step| from + step < node_count)
                .map(move |step| ProjectionEdge {
                    from: format!("node-{from}"),
                    to: format!("node-{}", from + step),
                })
        })
        .collect()
}

fn node_kind(index: usize) -> &'static str {
    match index % 5 {
        0 => "inspect_code_context",
        1 => "implement_solution",
        2 => "smoke_test",
        3 => "regression_test",
        _ => "final_synthesis",
    }
}

fn projection_row(
    node_count: usize,
    edge_profile: &'static str,
    edge_width: usize,
) -> ProjectionScaleRow {
    let input_started = Instant::now();
    let input = projection_input(node_count, edge_width, true);
    let input_build_duration_us = input_started.elapsed().as_micros();
    let edge_count = input.map_edges.len();
    let detail_count = input.node_details.len();
    let render_started = Instant::now();
    let rendered = render_active_projection(input);
    let render_duration_us = render_started.elapsed().as_micros();
    let size = rendered.size_breakdown;
    ProjectionScaleRow {
        node_count,
        edge_profile,
        edge_count,
        active_frontier_count: 1,
        result_ref_count: node_count,
        detail_count,
        input_build_duration_us,
        render_duration_us,
        estimated_tokens: rendered.estimated_tokens,
        skeleton_estimated_tokens: rendered.skeleton_estimated_tokens,
        header_bytes: size.header_bytes,
        root_source_bytes: size.root_source_bytes,
        active_frontier_bytes: size.active_frontier_bytes,
        map_node_bytes: size.map_node_bytes,
        map_edge_bytes: size.map_edge_bytes,
        node_detail_bytes: size.node_detail_bytes,
        footer_bytes: size.footer_bytes,
        skeleton_bytes: size.skeleton_bytes,
        projection_bytes: size.projection_bytes,
    }
}

fn budget_crossing(
    edge_profile: &'static str,
    edge_width: usize,
    budget: usize,
) -> BudgetCrossingRow {
    let mut low = 1usize;
    let mut high = 10_000usize;
    if skeleton_tokens(high, edge_width) <= budget {
        return BudgetCrossingRow {
            edge_profile,
            max_projection_tokens: budget,
            first_over_budget_node_count: None,
            skeleton_tokens_at_crossing: None,
        };
    }
    while low < high {
        let middle = low + (high - low) / 2;
        if skeleton_tokens(middle, edge_width) > budget {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    BudgetCrossingRow {
        edge_profile,
        max_projection_tokens: budget,
        first_over_budget_node_count: Some(low),
        skeleton_tokens_at_crossing: Some(skeleton_tokens(low, edge_width)),
    }
}

fn skeleton_tokens(node_count: usize, edge_width: usize) -> usize {
    render_active_projection(projection_input(node_count, edge_width, false))
        .skeleton_estimated_tokens
}

fn snapshot(node_count: usize, version: usize) -> ActionMapSnapshot {
    let current = node_count.saturating_sub(1);
    let nodes = (0..node_count)
        .map(|index| ActionMapSnapshotNode {
            id: format!("node-{index}"),
            title: format!("K0 node {index}"),
            kind: node_kind(index).into(),
            canonical_kind: "custom".into(),
            status: if index == current {
                "running".into()
            } else {
                "completed".into()
            },
            context_summary: format!("Execute deterministic K0 work item {index}"),
            source_refs: vec![format!("src/module_{index}.rs")],
            active_lease: None,
            result_ids: vec![format!("result-{index}")],
            node_event_ids: vec![format!("node-event-{index}")],
            origin_node_id: None,
        })
        .collect();
    let results = (0..node_count)
        .map(|index| ActionMapSnapshotResult {
            id: format!("result-{index}"),
            assignment_id: format!("lease-{index}"),
            map_id: "map-k0-replay".into(),
            node_id: format!("node-{index}"),
            kind: "result".into(),
            action_class: Some("read".into()),
            tool_success: Some(true),
            source_event_ref: format!("task-event-{index}"),
            artifact_refs: vec![format!("src/module_{index}.rs")],
            source_thread_id: ThreadId::default(),
            created_at_ms: index as i64,
        })
        .collect();
    let node_events = (0..node_count)
        .map(|index| ActionMapSnapshotNodeEvent {
            id: format!("node-event-{index}"),
            map_id: "map-k0-replay".into(),
            node_id: format!("node-{index}"),
            event_kind: "tool_result".into(),
            source: "main_tool".into(),
            action_class: Some("read".into()),
            tool_success: Some(true),
            content_sha256: format!("{index:064x}"),
            source_event_id: Some(format!("task-event-{index}")),
            raw_ref: Some(format!("output-ref-{index}")),
            artifact_refs: vec![format!("src/module_{index}.rs")],
            call_id: Some(format!("call-{index}")),
            source_thread_id: ThreadId::default(),
            created_at_ms: index as i64,
        })
        .collect();
    ActionMapSnapshot {
        mode: MapRuntimeMode::Experiment,
        routing_required: false,
        bootstrap_required: false,
        reborn_requested: false,
        active_task_id: Some("task-k0-replay".into()),
        active_map_id: Some("map-k0-replay".into()),
        tasks: vec![ActionMapSnapshotTask {
            id: "task-k0-replay".into(),
            title: "K0 long replay".into(),
            source_event_ids: vec!["task-event-root".into()],
            status: "active".into(),
            owner_session_id: None,
            active_map_id: Some("map-k0-replay".into()),
            map_ids: vec!["map-k0-replay".into()],
        }],
        maps: vec![ActionMapSnapshotMap {
            id: "map-k0-replay".into(),
            task_id: Some("task-k0-replay".into()),
            title: "K0 long replay map".into(),
            status: "active".into(),
            owner_session_id: None,
            base_map_version: format!("k0-code-revision-{version}"),
            created_from: None,
            ready_node_count: 0,
            running_node_count: 1,
            completed_node_count: node_count.saturating_sub(1),
            nodes,
            edges: (1..node_count)
                .map(|index| ActionMapSnapshotEdge {
                    from: format!("node-{}", index - 1),
                    to: format!("node-{index}"),
                })
                .collect(),
            leases: Vec::new(),
            results,
            node_events,
        }],
        maintenance_barriers: Vec::new(),
        trace_summary: Default::default(),
        trace_events: Vec::new(),
        sentinel_summary: Default::default(),
        sentinel_warnings: Vec::new(),
    }
}

fn replay_row(initial_node_count: usize) -> ReplayScaleRow {
    const CHECKPOINT_CYCLES: usize = 5;
    let append_count = (initial_node_count / 100).max(1);
    let generation_started = Instant::now();
    let mut current = snapshot(initial_node_count, 0);
    let mut snapshot_generation_duration_us = generation_started.elapsed().as_micros();
    let mut checkpoint_bytes = 0usize;
    let mut delta_bytes = 0usize;
    let mut delta_build_duration_us = 0u128;
    let mut replay_duration_us = 0u128;
    let mut replay_exact = true;

    for cycle in 1..=CHECKPOINT_CYCLES {
        let checkpoint_hash = snapshot_sha256(&current).unwrap();
        checkpoint_bytes += serde_json::to_vec(&current).unwrap().len();
        let checkpoint_id = format!("k0-checkpoint-{initial_node_count}-{cycle}");
        let mut checkpoint = ActionMapCheckpointState::default();
        checkpoint.install(
            checkpoint_id.clone(),
            checkpoint_hash.clone(),
            current.clone(),
        );

        let generation_started = Instant::now();
        let next = snapshot(initial_node_count + append_count * cycle, cycle);
        snapshot_generation_duration_us += generation_started.elapsed().as_micros();
        let delta_started = Instant::now();
        let delta = build_snapshot_delta(&mut checkpoint, &next)
            .unwrap()
            .expect("K0 replay cycle must change the snapshot");
        delta_build_duration_us += delta_started.elapsed().as_micros();
        delta_bytes += serde_json::to_vec(&delta).unwrap().len();
        let replay_started = Instant::now();
        let replayed =
            apply_snapshot_delta(&checkpoint_id, &checkpoint_hash, &current, &delta).unwrap();
        replay_duration_us += replay_started.elapsed().as_micros();
        replay_exact &= replayed == next;
        current = replayed;
    }

    ReplayScaleRow {
        initial_node_count,
        final_node_count: initial_node_count + append_count * CHECKPOINT_CYCLES,
        checkpoint_cycles: CHECKPOINT_CYCLES,
        nodes_appended_per_cycle: append_count,
        checkpoint_bytes,
        delta_bytes,
        final_snapshot_bytes: serde_json::to_vec(&current).unwrap().len(),
        snapshot_generation_duration_us,
        delta_build_duration_us,
        replay_duration_us,
        replay_exact,
        final_snapshot_sha256: snapshot_sha256(&current).unwrap(),
    }
}

#[test]
fn writes_k0_scale_probe_artifact() {
    let Some(output_path) = std::env::var_os("TASKSPACE_K0_SCALE_OUTPUT").map(PathBuf::from) else {
        return;
    };
    let mut projection_rows = Vec::new();
    for node_count in NODE_COUNTS {
        for (edge_profile, edge_width) in EDGE_PROFILES {
            projection_rows.push(projection_row(node_count, edge_profile, edge_width));
        }
    }
    let mut budget_crossings = Vec::new();
    for (edge_profile, edge_width) in EDGE_PROFILES {
        for budget in BUDGETS {
            budget_crossings.push(budget_crossing(edge_profile, edge_width, budget));
        }
    }
    let replay_rows = NODE_COUNTS.into_iter().map(replay_row).collect::<Vec<_>>();
    assert!(replay_rows.iter().all(|row| row.replay_exact));
    let artifact = K0ScaleProbe {
        schema_version: "taskspace-map-budget-k0-probe-v1",
        projection_rows,
        budget_crossings,
        replay_rows,
    };
    let bytes = serde_json::to_vec_pretty(&artifact).unwrap();
    std::fs::write(output_path, bytes).unwrap();
}
