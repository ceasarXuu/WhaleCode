use super::rooted_dag::GraphMutation;
use super::rooted_dag::MapEdge;
use super::rooted_dag::MapNode;
use super::rooted_dag::NodeMutation;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use std::collections::HashSet;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapResponseFinalReceipt {
    pub(crate) map_id: String,
    pub(crate) control_call_id: String,
    pub(crate) reservation_revision_after: u64,
    pub(crate) canonical_revision: Option<u64>,
    pub(crate) prepared_action_count: usize,
    pub(crate) attributed_result_count: usize,
    pub(crate) outstanding_reservation_count: usize,
    pub(crate) error: Option<String>,
}

impl ActionMapResponseFinalReceipt {
    pub(crate) fn from_canonical_map(
        prepared: &ActionMapPreparedResponse,
        control_call_id: &str,
        map: &TaskSpaceCanonicalMap,
    ) -> Self {
        let reservation_ids = prepared
            .prepared_calls
            .iter()
            .map(|call| call.reservation_id.as_str())
            .collect::<HashSet<_>>();
        let attributed_result_count = reservation_ids
            .iter()
            .filter(|reservation_id| {
                map.result_refs
                    .values()
                    .any(|result| result.reservation_id == **reservation_id)
            })
            .count();
        let outstanding_reservation_count = reservation_ids
            .iter()
            .filter(|reservation_id| map.action_reservations.contains_key(**reservation_id))
            .count();
        Self {
            map_id: prepared.map_id.clone(),
            control_call_id: control_call_id.to_string(),
            reservation_revision_after: prepared.revision_after,
            canonical_revision: Some(map.revision),
            prepared_action_count: prepared.prepared_calls.len(),
            attributed_result_count,
            outstanding_reservation_count,
            error: None,
        }
    }

    pub(crate) fn unavailable(
        prepared: &ActionMapPreparedResponse,
        control_call_id: &str,
        error: String,
    ) -> Self {
        Self {
            map_id: prepared.map_id.clone(),
            control_call_id: control_call_id.to_string(),
            reservation_revision_after: prepared.revision_after,
            canonical_revision: None,
            prepared_action_count: prepared.prepared_calls.len(),
            attributed_result_count: 0,
            outstanding_reservation_count: prepared.prepared_calls.len(),
            error: Some(error),
        }
    }

    pub(crate) fn complete(&self) -> bool {
        self.error.is_none()
            && self.attributed_result_count == self.prepared_action_count
            && self.outstanding_reservation_count == 0
    }

    pub(crate) fn model_visible_result(&self) -> String {
        let mut result = serde_json::json!({
            "schema_version": "TaskSpaceResponseFinalReceiptV1",
            "status": if self.complete() { "complete" } else { "incomplete" },
            "success": self.complete(),
            "receipt_only": true,
            "map_id": self.map_id,
            "control_call_id": self.control_call_id,
            "reservation_revision_after": self.reservation_revision_after,
            "canonical_revision": self.canonical_revision,
            "prepared_action_count": self.prepared_action_count,
            "attributed_result_count": self.attributed_result_count,
            "outstanding_reservation_count": self.outstanding_reservation_count,
        });
        if let Some(error) = self.error.as_ref() {
            result["error"] = serde_json::json!({
                "class": "resource",
                "code": "taskspace_response_final_receipt_unavailable",
                "detail": error,
            });
        } else if !self.complete() {
            result["error"] = serde_json::json!({
                "class": "state_machine",
                "code": "taskspace_response_attribution_incomplete",
            });
        }
        result.to_string()
    }
}
