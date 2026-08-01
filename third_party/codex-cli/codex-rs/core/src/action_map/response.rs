use super::rooted_dag::GraphMutation;
use super::rooted_dag::MapEdge;
use super::rooted_dag::MapNode;
use super::rooted_dag::NodeMutation;
use super::rooted_dag::Rejection;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use std::collections::HashSet;

pub(crate) const ACTION_MAP_RESPONSE_STATE_COMMIT_FAILED_CODE: &str =
    "taskspace_response_state_commit_failed";

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
    fn reserved_actions_json(&self) -> Vec<serde_json::Value> {
        self.prepared_calls
            .iter()
            .map(|call| {
                serde_json::json!({
                    "call_index": call.call_index,
                    "call_id": call.call_id,
                    "node_id": call.node_id,
                    "tool": call.tool_name,
                    "reservation_id": call.reservation_id,
                })
            })
            .collect()
    }

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
            "reserved_actions": self.reserved_actions_json(),
        })
        .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActionMapResponsePrepareError {
    State(Rejection),
    Protocol { code: &'static str, detail: String },
    Resource { code: &'static str, detail: String },
}

impl ActionMapResponsePrepareError {
    pub(crate) fn state(rejection: Rejection) -> Self {
        Self::State(rejection)
    }

    pub(crate) fn protocol(code: &'static str, detail: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            detail: detail.into(),
        }
    }

    pub(crate) fn resource(code: &'static str, detail: impl Into<String>) -> Self {
        Self::Resource {
            code,
            detail: detail.into(),
        }
    }

    pub(crate) fn class(&self) -> &'static str {
        match self {
            Self::State(_) => "state_machine",
            Self::Protocol { .. } => "protocol",
            Self::Resource { .. } => "resource",
        }
    }

    pub(crate) fn reason_code(&self) -> &'static str {
        match self {
            Self::State(_) => ACTION_MAP_RESPONSE_STATE_COMMIT_FAILED_CODE,
            Self::Protocol { code, .. } | Self::Resource { code, .. } => code,
        }
    }

    pub(crate) fn violation_codes(&self) -> Vec<&'static str> {
        match self {
            Self::State(rejection) => rejection
                .violations
                .iter()
                .map(|violation| violation.code.as_str())
                .collect(),
            Self::Protocol { .. } | Self::Resource { .. } => Vec::new(),
        }
    }

    pub(crate) fn violation_facts_json(&self) -> Option<String> {
        match self {
            Self::State(rejection) => serde_json::to_string(&rejection.violations).ok(),
            Self::Protocol { .. } | Self::Resource { .. } => None,
        }
    }

    pub(crate) fn current_revision(&self) -> Option<u64> {
        match self {
            Self::State(rejection) => Some(rejection.current_revision),
            Self::Protocol { .. } | Self::Resource { .. } => None,
        }
    }

    pub(crate) fn model_visible_failure(
        &self,
        canonical_revision: Option<u64>,
        failure_provenance: serde_json::Value,
    ) -> String {
        let mut payload = serde_json::json!({
            "schema_version": "TaskSpaceResponseCommitFailureV3",
            "status": match self {
                Self::State(_) => "state_rejected",
                Self::Protocol { .. } => "protocol_rejected",
                Self::Resource { .. } => "resource_failed",
            },
            "success": false,
            "state_commit": false,
            "canonical_revision": canonical_revision,
            "rejected_candidate_committed": false,
            "executed_tool_call_count": 0,
            "failure_provenance": failure_provenance,
            "error": {
                "class": self.class(),
                "code": self.reason_code(),
            },
        });
        match self {
            Self::State(rejection) => {
                payload["current_revision"] = serde_json::json!(rejection.current_revision);
                payload["error"]["violations"] = model_visible_state_violations(rejection);
            }
            Self::Protocol { detail, .. } | Self::Resource { detail, .. } => {
                payload["error"]["detail"] = serde_json::json!(detail);
            }
        }
        payload.to_string()
    }
}

pub(crate) fn model_visible_state_violations(rejection: &Rejection) -> serde_json::Value {
    serde_json::Value::Array(
        rejection
            .violations
            .iter()
            .map(|violation| {
                let mut value = serde_json::json!({
                    "code": violation.code.as_str(),
                    "subjects": violation.subjects,
                });
                if let Some(node_id) = violation.node_id.as_ref() {
                    value["node_id"] = serde_json::json!(node_id);
                    value["canonical_before_transaction"] = serde_json::json!({
                        "node_present": violation.canonical_node_present_before_transaction,
                        "state": violation.canonical_state_before_transaction,
                        "unsatisfied_predecessor_ids":
                            violation.canonical_unsatisfied_predecessor_ids_before_transaction,
                    });
                    value["rejected_candidate_at_violation"] = serde_json::json!({
                        "committed": false,
                        "state": violation.uncommitted_candidate_state_at_violation,
                        "allowed_states":
                            violation.allowed_uncommitted_candidate_states_at_violation,
                        "unsatisfied_predecessor_ids":
                            violation.uncommitted_candidate_unsatisfied_predecessor_ids_at_violation,
                    });
                }
                value
            })
            .collect(),
    )
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

    pub(crate) fn settlement_status(&self) -> &'static str {
        if self.complete() {
            "settled"
        } else {
            "settlement_incomplete"
        }
    }

    pub(crate) fn reason_code(&self) -> Option<&'static str> {
        if self.error.is_some() {
            Some("taskspace_response_final_state_unavailable")
        } else if !self.complete() {
            Some("taskspace_response_attribution_incomplete")
        } else {
            None
        }
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

    pub(crate) fn finalized_model_visible_result(
        &self,
        prepared: &ActionMapPreparedResponse,
    ) -> String {
        debug_assert_eq!(self.map_id, prepared.map_id);
        let mut result = serde_json::json!({
            "schema_version": "TaskSpaceResponseResultV2",
            "status": self.settlement_status(),
            "success": self.complete(),
            "state_commit": true,
            "map_id": self.map_id,
            "action": prepared.action,
            "canonical_revision": self.canonical_revision,
            "reserved_actions": prepared.reserved_actions_json(),
            "settlement": {
                "prepared_action_count": self.prepared_action_count,
                "attributed_result_count": self.attributed_result_count,
                "outstanding_reservation_count": self.outstanding_reservation_count,
            },
        });
        if let Some(error) = self.error.as_ref() {
            result["error"] = serde_json::json!({
                "class": "resource",
                "code": self.reason_code(),
                "detail": error,
            });
        } else if !self.complete() {
            result["error"] = serde_json::json!({
                "class": "state_machine",
                "code": self.reason_code(),
            });
        }
        result.to_string()
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
