use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::MapRuntimeMode;
use serde_json::json;

use crate::action_map::map::ActionMapId;

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
        "bootstrap_required": snapshot.bootstrap_required,
        "active_map_id": snapshot.map.as_ref().map(|map| map.id.clone()),
        "revision": snapshot.map.as_ref().map(|map| map.revision),
        "complete": snapshot.map.as_ref().map(|map| map.complete),
    })
    .to_string()
}
