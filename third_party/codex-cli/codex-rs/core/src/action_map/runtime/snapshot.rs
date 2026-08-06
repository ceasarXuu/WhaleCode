use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotAction;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotNode;
use codex_protocol::taskspace::TaskSpaceActionOutcome;

use crate::action_map::map::ActionMapInstance;
use crate::action_map::map::node_state_name;

use super::state::ActionMapRuntimeState;

impl ActionMapRuntimeState {
    pub(crate) fn snapshot(&self) -> ActionMapSnapshot {
        ActionMapSnapshot {
            schema_version: "taskspace-snapshot-v3".to_string(),
            mode: self.mode,
            bootstrap_required: self.active_map().is_none(),
            map: self.active_map().map(snapshot_map),
        }
    }
}

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let graph = map.canonical_map();
    ActionMapSnapshotMap {
        id: graph.map_id.clone(),
        root_node_id: graph.root.node_id.clone(),
        finish_node_id: graph.finish.node_id.clone(),
        revision: graph.revision,
        complete: map.is_complete(),
        ready_work_node_count: map.ready_work_node_count(),
        inflight_work_node_count: map.inflight_work_node_count(),
        completed_work_node_count: map.completed_work_node_count(),
        finish_ready: map.finish_ready(),
        nodes: map
            .all_nodes()
            .zip(map.node_views())
            .map(|((role, _), view)| ActionMapSnapshotNode {
                id: view.node_id,
                role: role.as_str().to_string(),
                goal: view.goal,
                state: node_state_name(view.state).to_string(),
                content: view.content,
                parents: view.parents,
                children: view.children,
                actions: view
                    .actions
                    .into_iter()
                    .map(|action| ActionMapSnapshotAction {
                        action_id: action.action_id,
                        tool_name: action.tool_name,
                        outcome: action_outcome_name(action.outcome).to_string(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn action_outcome_name(outcome: TaskSpaceActionOutcome) -> &'static str {
    match outcome {
        TaskSpaceActionOutcome::Pending => "pending",
        TaskSpaceActionOutcome::Succeeded => "succeeded",
        TaskSpaceActionOutcome::Failed => "failed",
        TaskSpaceActionOutcome::Cancelled => "cancelled",
    }
}
