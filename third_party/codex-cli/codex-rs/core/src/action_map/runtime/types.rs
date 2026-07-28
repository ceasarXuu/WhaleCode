use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::MapRuntimeMode;
use serde_json::json;

use crate::action_map::map::ActionMapId;
use crate::action_map::rooted_dag::EventBatch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapControlState {
    pub(crate) map_id: ActionMapId,
    pub(crate) owner_session_id: Option<ThreadId>,
    pub(crate) revision: u64,
    pub(crate) complete: bool,
    pub(crate) ready_work_node_count: usize,
    pub(crate) inflight_work_node_count: usize,
    pub(crate) completed_work_node_count: usize,
    pub(crate) finish_ready: bool,
}

impl ActionMapControlState {
    pub(crate) fn requires_named_taskspace_control(&self) -> bool {
        !self.complete
            && self.finish_ready
            && self.ready_work_node_count == 0
            && self.inflight_work_node_count == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapControlDelta {
    pub(crate) map_id: ActionMapId,
    pub(crate) committed_revision: u64,
    pub(crate) graph_revision_batches: Vec<EventBatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapTerminalOutcome {
    pub(crate) map_id: ActionMapId,
    pub(crate) finish_node_id: String,
    pub(crate) completed_work_node_ids: Vec<String>,
    pub(crate) revision: u64,
    pub(crate) exact_summary: String,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceModeTransition {
    pub(crate) previous_mode: MapRuntimeMode,
    pub(crate) current_mode: MapRuntimeMode,
    pub(crate) changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetTaskSpaceModeOutcome {
    pub(crate) mode: TaskSpaceModeTransition,
    pub(crate) active_map_id: Option<ActionMapId>,
}

pub(crate) fn format_action_map_snapshot(snapshot: &ActionMapSnapshot) -> String {
    json!({
        "schema_version": snapshot.schema_version,
        "mode": snapshot.mode,
        "routing_required": snapshot.routing_required,
        "bootstrap_required": snapshot.bootstrap_required,
        "active_map_id": snapshot.map.as_ref().map(|map| map.id.clone()),
        "revision": snapshot.map.as_ref().map(|map| map.revision),
        "complete": snapshot.map.as_ref().map(|map| map.complete),
    })
    .to_string()
}
