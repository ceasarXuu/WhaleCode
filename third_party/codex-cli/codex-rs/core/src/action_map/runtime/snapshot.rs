use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotAction;
use codex_protocol::protocol::ActionMapSnapshotEdge;
use codex_protocol::protocol::ActionMapSnapshotEvidenceRef;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotNode;
use codex_protocol::protocol::ActionMapSnapshotNodeEvent;
use codex_protocol::protocol::ActionMapSnapshotResult;
use codex_protocol::protocol::ActionMapSnapshotSentinelSummary;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;

use crate::action_map::map::ActionMapInstance;
use crate::action_map::map::NodeEvent;
use crate::action_map::map::node_state_name;

use super::state::ActionMapRuntimeState;

impl ActionMapRuntimeState {
    pub(crate) fn snapshot(&self) -> ActionMapSnapshot {
        ActionMapSnapshot {
            schema_version: "taskspace-snapshot-v2".to_string(),
            mode: self.mode,
            routing_required: false,
            bootstrap_required: self.active_map().is_none(),
            map: self.active_map().map(snapshot_map),
            maintenance_barriers: Vec::new(),
            trace_summary: ActionMapSnapshotTraceSummary::default(),
            trace_events: Vec::new(),
            sentinel_summary: ActionMapSnapshotSentinelSummary::default(),
            sentinel_warnings: Vec::new(),
        }
    }
}

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let graph = map.canonical_map();
    ActionMapSnapshotMap {
        id: graph.map_id.clone(),
        task_id: map.task_id.clone(),
        owner_session_id: map.owner_session_id,
        root_node_id: graph.root.node_id.clone(),
        finish_node_id: graph.finish.node_id.clone(),
        revision: graph.revision,
        terminal_summary_ref: graph
            .terminal_record
            .as_ref()
            .map(|terminal| terminal.summary_ref.clone()),
        terminal_history_summary_refs: graph
            .terminal_history
            .iter()
            .map(|terminal| terminal.summary_ref.clone())
            .collect(),
        complete: map.is_complete(),
        ready_work_node_count: map.ready_work_node_count(),
        inflight_work_node_count: map.inflight_work_node_count(),
        completed_work_node_count: map.completed_work_node_count(),
        finish_ready: map.finish_ready(),
        nodes: map
            .all_nodes()
            .map(|(role, node)| {
                let state = map
                    .node_state(&node.node_id)
                    .map(node_state_name)
                    .unwrap_or("unknown");
                ActionMapSnapshotNode {
                    id: node.node_id.clone(),
                    role: role.as_str().to_string(),
                    goal: node.goal.clone(),
                    state: state.to_string(),
                    source_refs: node.source_refs.clone(),
                    result_ids: map.result_ids_for_node(&node.node_id),
                    evidence_ref_ids: map.evidence_ids_for_node(&node.node_id),
                    node_event_ids: map.event_ids_for_node(&node.node_id),
                }
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| ActionMapSnapshotEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
            })
            .collect(),
        actions: graph
            .action_records
            .iter()
            .map(|(action_id, action)| ActionMapSnapshotAction {
                action_id: action_id.clone(),
                node_id: action.node_id.clone(),
            })
            .collect(),
        results: graph
            .result_refs
            .iter()
            .map(|(id, result)| ActionMapSnapshotResult {
                id: id.clone(),
                node_id: result.node_id.clone(),
                action_id: result.action_id.clone(),
                is_error: result.is_error,
            })
            .collect(),
        evidence_refs: graph
            .evidence_refs
            .iter()
            .map(|(id, evidence)| ActionMapSnapshotEvidenceRef {
                id: id.clone(),
                node_id: evidence.node_id.clone(),
                action_id: evidence.action_id.clone(),
                kind: evidence.kind.clone(),
            })
            .collect(),
        node_events: map.node_events.values().map(snapshot_node_event).collect(),
    }
}

fn snapshot_node_event(event: &NodeEvent) -> ActionMapSnapshotNodeEvent {
    ActionMapSnapshotNodeEvent {
        id: event.id.clone(),
        map_id: event.map_id.clone(),
        node_id: event.node_id.clone(),
        event_kind: event.event_kind.clone(),
        source: event.source.clone(),
        action_class: event.action_class.map(|class| class.as_str().to_string()),
        tool_success: event.tool_success,
        content_sha256: event.content_sha256.clone(),
        source_event_id: event.source_event_id.clone(),
        raw_ref: event.raw_ref.clone(),
        artifact_refs: event.artifact_refs.clone(),
        call_id: event.call_id.clone(),
        source_thread_id: event.source_thread_id,
        created_at_ms: event.created_at_ms,
    }
}
