use codex_protocol::protocol::MapRuntimeEvent;

use crate::action_map::map::ActionMapId;
use crate::action_map::map::MapNodeId;
use crate::action_map::map::TaskId;

use super::state::ActionMapRuntimeState;

#[derive(Debug, Clone)]
pub(crate) struct ActionMapProviderRequestBudgetSnapshot {
    pub(crate) task_id: Option<TaskId>,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: Option<MapNodeId>,
    pub(crate) node_role: Option<String>,
    pub(crate) route_mode: Option<String>,
    pub(crate) profile_name: Option<String>,
    pub(crate) request_phase: Option<String>,
    pub(crate) provider_request_context_missing_reason: Option<String>,
    pub(crate) map_requires_initialization: bool,
    pub(crate) request_count: usize,
    pub(crate) max_requests: usize,
    pub(crate) node_request_count: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) budget_state: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ActionMapProviderRequestBudgetEventInput {
    pub(crate) request_id: String,
    pub(crate) logical_request_id: String,
    pub(crate) parent_request_id: Option<String>,
    pub(crate) attempt_seq: usize,
    pub(crate) transport: String,
    pub(crate) status: String,
    pub(crate) request_count_before: usize,
    pub(crate) request_count_after: usize,
    pub(crate) max_requests: usize,
    pub(crate) budget_state_before: String,
    pub(crate) budget_state_after: String,
    pub(crate) budget_transition_reason: String,
    pub(crate) started_at_ms: i64,
    pub(crate) completed_at_ms: Option<i64>,
    pub(crate) latency_ms: Option<i64>,
    pub(crate) input_tokens: Option<i64>,
    pub(crate) cached_input_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) reasoning_output_tokens: Option<i64>,
    pub(crate) total_tokens: Option<i64>,
    pub(crate) provider_payload_sha256: Option<String>,
    pub(crate) provider_payload_bytes: Option<usize>,
    pub(crate) provider_wire_api: Option<String>,
    pub(crate) tools_count: Option<usize>,
    pub(crate) tools_present: Option<bool>,
    pub(crate) request_shape_classifier: Option<String>,
    pub(crate) messages_hash: Option<String>,
    pub(crate) stable_prefix_hash: Option<String>,
    pub(crate) dynamic_suffix_hash: Option<String>,
    pub(crate) exact_payload_scan_passed: Option<bool>,
    pub(crate) active_projection_present: Option<bool>,
    pub(crate) active_projection_count: Option<usize>,
    pub(crate) large_raw_output_tokens: Option<usize>,
    pub(crate) protected_items_present: Option<bool>,
    pub(crate) replacement_confirmed: Option<bool>,
    pub(crate) exact_payload_scan: Option<ActionMapExactPayloadScanEventInput>,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) request_phase: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActionMapExactPayloadScanEventInput {
    pub(crate) scan_event_id: String,
    pub(crate) request_id: String,
    pub(crate) provider_payload_sha256: String,
    pub(crate) scanner_version: String,
    pub(crate) matcher_version: String,
    pub(crate) checked_byte_ranges: Vec<(usize, usize)>,
    pub(crate) negative_checks_performed: Vec<String>,
    pub(crate) projection_required: bool,
    pub(crate) active_projection_present: bool,
    pub(crate) active_projection_count: usize,
    pub(crate) projection_is_message_tail: bool,
    pub(crate) large_raw_output_tokens: usize,
    pub(crate) runtime_boundary_forbidden_markers: Vec<String>,
    pub(crate) protected_items_present: bool,
    pub(crate) projection_kind: Option<String>,
    pub(crate) projection_map_id_sha256: Option<String>,
    pub(crate) projection_revision: Option<u64>,
    pub(crate) projection_canonical_sha256: Option<String>,
    pub(crate) projection_sha256: Option<String>,
    pub(crate) projection_policy: Option<String>,
    pub(crate) expected_projection_kind: Option<String>,
    pub(crate) expected_projection_map_id_sha256: Option<String>,
    pub(crate) expected_projection_revision: Option<u64>,
    pub(crate) expected_projection_canonical_sha256: Option<String>,
    pub(crate) expected_projection_sha256: Option<String>,
    pub(crate) projection_identity_confirmed: Option<bool>,
    pub(crate) replacement_confirmed: bool,
    pub(crate) passed: bool,
    pub(crate) failure_reasons: Vec<String>,
}

impl ActionMapRuntimeState {
    pub(crate) fn provider_request_budget_snapshot(
        &self,
    ) -> Option<ActionMapProviderRequestBudgetSnapshot> {
        let map_id = self.active_map_id.clone()?;
        let map = self.maps.get(&map_id);
        Some(ActionMapProviderRequestBudgetSnapshot {
            task_id: map.and_then(|map| map.task_id.clone()),
            map_id,
            node_id: None,
            node_role: None,
            route_mode: None,
            profile_name: None,
            request_phase: None,
            provider_request_context_missing_reason: None,
            map_requires_initialization: map.is_none(),
            request_count: 0,
            max_requests: 0,
            node_request_count: 0,
            max_model_requests_per_node: 0,
            budget_state: "unknown".to_string(),
        })
    }

    pub(crate) fn record_provider_request_budget_events(
        &mut self,
        _snapshot: &ActionMapProviderRequestBudgetSnapshot,
        inputs: Vec<ActionMapProviderRequestBudgetEventInput>,
    ) -> Option<Vec<MapRuntimeEvent>> {
        (!inputs.is_empty()).then(Vec::new)
    }
}
