use super::rooted_dag::GraphMutation;
use super::rooted_dag::MapEdge;
use super::rooted_dag::MapNode;
use super::rooted_dag::NodeMutation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapDeclaredCall {
    pub(crate) call_id: String,
    pub(crate) call_index: usize,
    pub(crate) node_id: String,
    pub(crate) tool_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActionMapResponseOperation {
    Initialize {
        root: MapNode,
        work_nodes: Vec<MapNode>,
        finish: MapNode,
        edges: Vec<MapEdge>,
    },
    Execute {
        expected_revision: u64,
        graph: GraphMutation,
        node_mutations: Vec<NodeMutation>,
    },
    Reopen {
        expected_revision: u64,
        work_nodes: Vec<MapNode>,
        edges: Vec<MapEdge>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapPreparedCall {
    pub(crate) map_id: String,
    pub(crate) revision: u64,
    pub(crate) call_id: String,
    pub(crate) call_index: usize,
    pub(crate) node_id: String,
    pub(crate) tool_name: String,
    pub(crate) reservation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapPreparedResponse {
    pub(crate) map_id: String,
    pub(crate) revision_before: u64,
    pub(crate) revision_after: u64,
    pub(crate) action: &'static str,
    pub(crate) prepared_calls: Vec<ActionMapPreparedCall>,
}

impl ActionMapPreparedResponse {
    pub(crate) fn model_visible_result(&self) -> String {
        serde_json::json!({
            "schema_version": "TaskSpaceResponseCommitV1",
            "status": "accepted",
            "success": true,
            "state_commit": true,
            "map_id": self.map_id,
            "action": self.action,
            "revision_before": self.revision_before,
            "revision_after": self.revision_after,
            "reserved_actions": self.prepared_calls.iter().map(|call| {
                serde_json::json!({
                    "call_index": call.call_index,
                    "call_id": call.call_id,
                    "node_id": call.node_id,
                    "tool": call.tool_name,
                    "reservation_id": call.reservation_id,
                })
            }).collect::<Vec<_>>(),
        })
        .to_string()
    }
}
