use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ActionMapSnapshot;
use codex_protocol::protocol::ActionMapSnapshotEdge;
use codex_protocol::protocol::ActionMapSnapshotLease;
use codex_protocol::protocol::ActionMapSnapshotMaintenanceBarrier;
use codex_protocol::protocol::ActionMapSnapshotMap;
use codex_protocol::protocol::ActionMapSnapshotNode;
use codex_protocol::protocol::ActionMapSnapshotNodeEvent;
use codex_protocol::protocol::ActionMapSnapshotResult;
use codex_protocol::protocol::ActionMapSnapshotSentinelSummary;
use codex_protocol::protocol::ActionMapSnapshotSentinelWarningRef;
use codex_protocol::protocol::ActionMapSnapshotTraceEventRef;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeGraphRevisionCommittedEvent;
use codex_protocol::protocol::MapRuntimeLeaseAttachedEvent;
use codex_protocol::protocol::MapRuntimeLeaseCreatedEvent;
use codex_protocol::protocol::MapRuntimeLeaseReleasedEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeNodeDetailExpandedEvent;
use codex_protocol::protocol::MapRuntimeNodeEventRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeResultRecordedEvent;
use codex_protocol::protocol::MapRuntimeSentinelWarningRaisedEvent;
use codex_protocol::protocol::MapRuntimeTimeoutSummaryRequestedEvent;
use codex_protocol::protocol::MapRuntimeTraceEventRecordedEvent;
use sha2::Digest;
use sha2::Sha256;

use super::detail_fold::NODE_DETAIL_EXPANDED_EVENT_KIND;
use super::detail_fold::NodeDetailState;
use super::detail_fold::node_detail_plan;
use super::map::ActionClass;
use super::map::ActionMapId;
use super::map::ActionMapInstance;
use super::map::AssignmentLease;
use super::map::AssignmentLeaseId;
use super::map::LeaseHolder;
use super::map::MapEdge;
use super::map::MapNode;
use super::map::MapNodeId;
use super::map::NodeEvent;
use super::map::NodeEventId;
use super::map::NodeEventRef;
use super::map::NodeResult;
use super::map::NodeResultId;
use super::map::NodeResultKind;
use super::map::NodeResultRef;
use super::map::NodeStatus;
use super::map::TaskId;
use super::map::TaskRecord;
use super::map::TaskSpaceTraceEvent;
use super::map::ToolActionDescriptor;
use super::projection::ProjectionEdge;
use super::projection::ProjectionEnvelope;
use super::projection::ProjectionEventRef;
use super::projection::ProjectionInput;
use super::projection::ProjectionNode;
use super::projection::ProjectionNodeDetailIdentity;
use super::projection::ProjectionNodeDetailState;
use super::projection::ProjectionSizeBreakdown;
use super::projection::node_detail_fold_saves_bytes;
use super::projection::node_detail_identity;
use super::projection::render_projection;
use super::rooted_dag::EventBatch;
use super::rooted_dag::GraphMutation;
use super::rooted_dag::InitializeMap;
use super::rooted_dag::NodeRole;
use super::rooted_dag::NodeTransition;
use super::rooted_dag::Rejection;
use super::rooted_dag::TaskSpaceMap;
use super::rooted_dag::close_finish_with_no_active_work;
use super::rooted_dag::complete_active_work_then_end;
use super::rooted_dag::complete_then_bind;
use super::rooted_dag::initialize;
use super::rooted_dag::mutate_graph;
use super::rooted_dag::transition_node;
use super::rooted_dag::validate;
use super::sentinel::TaskSpaceSentinelSeverity;
use super::sentinel::TaskSpaceSentinelWarning;
use super::sentinel::TaskSpaceSentinelWarningStatus;
use super::sentinel::TaskSpaceSentinelWarningType;

const TASKSPACE_SNAPSHOT_SCHEMA_VERSION: &str = "TaskSpaceSnapshotR6V1";
use super::sentinel::warning_drafts_for_trace_event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceHardGateClass {
    StateMachine,
    Protocol,
}

impl TaskSpaceHardGateClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::StateMachine => "state_machine",
            Self::Protocol => "protocol",
        }
    }
}

fn gate_recovery_message(
    message: &str,
    gate_class: TaskSpaceHardGateClass,
    reason: &str,
    blocking_items: Vec<String>,
    missing_evidence: Vec<String>,
) -> String {
    serde_json::json!({
        "schema_version": "TaskSpaceGateResultV1",
        "status": format!("{}_failed", gate_class.as_str()),
        "success": false,
        "error": {
            "class": gate_class.as_str(),
            "code": reason,
            "message": message,
            "blocking_items": blocking_items,
            "missing_evidence": missing_evidence,
        },
    })
    .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapGateError {
    message: String,
    events: Vec<MapRuntimeEvent>,
}

impl ActionMapGateError {
    fn new(message: impl Into<String>, events: Vec<MapRuntimeEvent>) -> Self {
        let message = message.into();
        let reason = hard_state_reason_from_message(&message)
            .unwrap_or_else(|| "state_machine_transition_rejected".to_string());
        let message = gate_recovery_message(
            &message,
            TaskSpaceHardGateClass::StateMachine,
            &reason,
            Vec::new(),
            Vec::new(),
        );
        Self { message, events }
    }

    pub(crate) fn into_parts(self) -> (String, Vec<MapRuntimeEvent>) {
        (self.message, self.events)
    }
}

fn hard_state_reason_from_message(message: &str) -> Option<String> {
    let (_, after_marker) = message.split_once("hard_state:")?;
    let reason = after_marker
        .trim_start()
        .split(|character: char| character.is_whitespace() || matches!(character, '.' | ',' | ';'))
        .next()
        .unwrap_or_default()
        .trim();
    (!reason.is_empty()).then(|| reason.to_string())
}

fn rooted_rejection_message(rejection: Rejection) -> String {
    serde_json::json!({
        "schema_version": "TaskSpaceStateRejectionV1",
        "status": "state_machine_failed",
        "success": false,
        "state_commit": rejection.state_commit,
        "partial_commit": false,
        "current_revision": rejection.current_revision,
        "violations": rejection
            .violations
            .into_iter()
            .map(|violation| serde_json::json!({
                "code": violation.code.as_str(),
                "subjects": violation.subjects,
            }))
            .collect::<Vec<_>>(),
    })
    .to_string()
}

impl From<String> for ActionMapGateError {
    fn from(message: String) -> Self {
        Self::new(message, Vec::new())
    }
}

impl From<&str> for ActionMapGateError {
    fn from(message: &str) -> Self {
        Self::new(message, Vec::new())
    }
}

impl std::ops::Deref for ActionMapGateError {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl std::fmt::Display for ActionMapGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapControlState {
    pub(crate) task_id: TaskId,
    pub(crate) map_id: ActionMapId,
    pub(crate) revision: u64,
    pub(crate) root_node_id: MapNodeId,
    pub(crate) finish_node_id: MapNodeId,
    pub(crate) complete: bool,
    pub(crate) current_node_id: Option<MapNodeId>,
    pub(crate) pending_work_node_ids: Vec<MapNodeId>,
    pub(crate) ready_work_node_ids: Vec<MapNodeId>,
    pub(crate) running_work_node_ids: Vec<MapNodeId>,
    pub(crate) blocked_work_node_ids: Vec<MapNodeId>,
    pub(crate) finish_ready: bool,
    pub(crate) completed_work_node_count: usize,
    pub(crate) total_node_count: usize,
}

impl ActionMapControlState {
    pub(crate) fn requires_named_taskspace_control(&self) -> bool {
        !self.complete
            && self.finish_ready
            && self.current_node_id.is_none()
            && self.pending_work_node_ids.is_empty()
            && self.ready_work_node_ids.is_empty()
            && self.running_work_node_ids.is_empty()
            && self.blocked_work_node_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeNodeInput {
    pub(crate) id: String,
    pub(crate) goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeFinishInput {
    pub(crate) id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapEdgeInput {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeInput {
    pub(crate) root: ActionMapInitializeNodeInput,
    pub(crate) current_work_node: ActionMapInitializeNodeInput,
    pub(crate) finish: ActionMapInitializeFinishInput,
    pub(crate) work_nodes: Vec<ActionMapInitializeNodeInput>,
    pub(crate) edges: Vec<ActionMapEdgeInput>,
    pub(crate) source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapGraphMutationInput {
    pub(crate) expected_revision: u64,
    pub(crate) add_nodes: Vec<ActionMapInitializeNodeInput>,
    pub(crate) add_edges: Vec<ActionMapEdgeInput>,
    pub(crate) remove_edges: Vec<ActionMapEdgeInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeOutcome {
    pub(crate) task_id: TaskId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_ids: Vec<MapNodeId>,
    pub(crate) current_node_id: MapNodeId,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapGraphMutationOutcome {
    pub(crate) map_id: ActionMapId,
    pub(crate) revision: u64,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapTransitionOutcome {
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) revision: u64,
    pub(crate) status: String,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapCompleteHandoffOutcome {
    pub(crate) map_id: ActionMapId,
    pub(crate) current_node_id: MapNodeId,
    pub(crate) next_node_id: MapNodeId,
    pub(crate) revision: u64,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapTerminalOutcome {
    pub(crate) map_id: ActionMapId,
    pub(crate) revision: u64,
    pub(crate) final_summary: String,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapNodeDetailExpansionOutcome {
    pub(crate) node_id: MapNodeId,
    pub(crate) expansion_event_id: NodeEventId,
    pub(crate) detail_ref: String,
    pub(crate) restored_details: Vec<ActionMapExpandedDetailRef>,
    pub(crate) delta: ActionMapControlDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapControlDelta {
    pub(crate) map_id: ActionMapId,
    pub(crate) committed_revision: u64,
    pub(crate) graph_revision_batches: Vec<MapRuntimeGraphRevisionCommittedEvent>,
    pub(crate) node_detail_events: Vec<MapRuntimeNodeDetailExpandedEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapExpandedDetailRef {
    pub(crate) event_id: String,
    pub(crate) event_kind: String,
    pub(crate) source: String,
    pub(crate) detail_tier: String,
    pub(crate) evidence_class: String,
    pub(crate) action_class: Option<String>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) content_sha256: Option<String>,
    pub(crate) raw_ref: Option<String>,
    pub(crate) artifact_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceProviderRequestPhase {
    ValidationRecovery,
    FinalSynthesis,
    ModelSampling,
    Unknown,
}

impl TaskSpaceProviderRequestPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ValidationRecovery => "validation_recovery",
            Self::FinalSynthesis => "final_synthesis",
            Self::ModelSampling => "model_sampling",
            Self::Unknown => "unknown",
        }
    }
}

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

#[derive(Debug, Clone)]
pub(crate) struct ActionMapProviderResponseActionabilityInput {
    pub(crate) response_actionability: String,
    pub(crate) end_turn: Option<bool>,
    pub(crate) saw_actionable_output: bool,
    pub(crate) assistant_message_present: bool,
    pub(crate) recovery_action: String,
    pub(crate) last_agent_message_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceRouteMode {
    Thin,
    VerificationFirst,
    DefaultCompact,
    SubagentAssisted,
    Deep,
}

impl TaskSpaceRouteMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TaskSpaceRouteMode::Thin => "thin",
            TaskSpaceRouteMode::VerificationFirst => "verification_first",
            TaskSpaceRouteMode::DefaultCompact => "default_compact",
            TaskSpaceRouteMode::SubagentAssisted => "subagent_assisted",
            TaskSpaceRouteMode::Deep => "deep",
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "thin" => Some(TaskSpaceRouteMode::Thin),
            "verification_first" => Some(TaskSpaceRouteMode::VerificationFirst),
            "default_compact" => Some(TaskSpaceRouteMode::DefaultCompact),
            "subagent_assisted" => Some(TaskSpaceRouteMode::SubagentAssisted),
            "deep" => Some(TaskSpaceRouteMode::Deep),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceBudgetState {
    Normal,
    Warned,
    CompactCheckpointRequired,
    ThinDowngraded,
    OverProfileHint,
}

impl TaskSpaceBudgetState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TaskSpaceBudgetState::Normal => "normal",
            TaskSpaceBudgetState::Warned => "warned",
            TaskSpaceBudgetState::CompactCheckpointRequired => "compact_checkpoint_required",
            TaskSpaceBudgetState::ThinDowngraded => "thin_downgraded",
            TaskSpaceBudgetState::OverProfileHint => "over_profile_hint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceActiveBudgetV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) profile_name: String,
    pub(crate) route_mode: TaskSpaceRouteMode,
    pub(crate) max_rollout_model_requests: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) max_spawn_agent_calls: usize,
    pub(crate) max_subagent_results: usize,
    pub(crate) max_nodes: usize,
    pub(crate) max_open_leaf_nodes: usize,
    pub(crate) max_projection_tokens: usize,
    pub(crate) max_avg_input_tokens_per_request: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TaskSpaceBudgetCounters {
    pub(crate) rollout_model_request_count: usize,
    pub(crate) model_request_count_by_node: HashMap<String, usize>,
    pub(crate) spawn_agent_call_count: usize,
    pub(crate) subagent_result_count: usize,
    pub(crate) node_count: usize,
    pub(crate) open_leaf_node_count: usize,
    pub(crate) projection_tokens_last: usize,
    pub(crate) projection_tokens_max: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceBudgetViolation {
    pub(crate) violation_id: String,
    pub(crate) counter_name: String,
    pub(crate) counter_value: usize,
    pub(crate) counter_limit: usize,
    pub(crate) state_before: TaskSpaceBudgetState,
    pub(crate) state_after: TaskSpaceBudgetState,
    pub(crate) action_taken: String,
    pub(crate) created_at_ms: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceBudgetGateDecision {
    pub(crate) allowed: bool,
    pub(crate) budget_state: TaskSpaceBudgetState,
    pub(crate) reason: String,
    pub(crate) blocking_items: Vec<String>,
    pub(crate) next_valid_actions: Vec<String>,
    pub(crate) recovery_request_phase: Option<String>,
    pub(crate) quality_impact_required: bool,
}

fn default_budget_common(
    profile_name: &str,
    route_mode: TaskSpaceRouteMode,
) -> TaskSpaceActiveBudgetV1 {
    TaskSpaceActiveBudgetV1 {
        schema_version: "taskspace-active-budget-v1",
        profile_name: profile_name.to_string(),
        route_mode,
        max_rollout_model_requests: 10,
        max_model_requests_per_node: 3,
        max_spawn_agent_calls: 2,
        max_subagent_results: 2,
        max_nodes: 8,
        max_open_leaf_nodes: 4,
        max_projection_tokens: 24_000,
        max_avg_input_tokens_per_request: 16_000,
    }
}

pub(crate) fn taskspace_active_budget_for_route(
    profile_name: &str,
    route_mode: TaskSpaceRouteMode,
) -> TaskSpaceActiveBudgetV1 {
    let mut budget = default_budget_common(profile_name, route_mode);
    match route_mode {
        TaskSpaceRouteMode::Thin => {
            budget.max_rollout_model_requests = 8;
            budget.max_model_requests_per_node = 3;
            budget.max_spawn_agent_calls = 0;
            budget.max_subagent_results = 0;
            budget.max_nodes = 4;
            budget.max_open_leaf_nodes = 2;
            budget.max_projection_tokens = 12_000;
            budget.max_avg_input_tokens_per_request = 12_000;
        }
        TaskSpaceRouteMode::VerificationFirst => {
            budget.max_rollout_model_requests = 6;
            budget.max_model_requests_per_node = 2;
            budget.max_spawn_agent_calls = 0;
            budget.max_subagent_results = 0;
            budget.max_nodes = 5;
            budget.max_open_leaf_nodes = 2;
            budget.max_projection_tokens = 16_000;
            budget.max_avg_input_tokens_per_request = 14_000;
        }
        TaskSpaceRouteMode::DefaultCompact => {}
        TaskSpaceRouteMode::SubagentAssisted => {
            budget.max_rollout_model_requests = 14;
            budget.max_model_requests_per_node = 4;
            budget.max_spawn_agent_calls = 3;
            budget.max_subagent_results = 3;
            budget.max_nodes = 10;
            budget.max_open_leaf_nodes = 5;
            budget.max_projection_tokens = 32_000;
            budget.max_avg_input_tokens_per_request = 18_000;
        }
        TaskSpaceRouteMode::Deep => {
            budget.max_rollout_model_requests = 20;
            budget.max_model_requests_per_node = 10;
            budget.max_spawn_agent_calls = 4;
            budget.max_subagent_results = 4;
            budget.max_nodes = 14;
            budget.max_open_leaf_nodes = 7;
            budget.max_projection_tokens = 48_000;
            budget.max_avg_input_tokens_per_request = 24_000;
        }
    }
    budget
}

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    mode: MapRuntimeMode,
    pending_transition_notice: Option<String>,
    routing_required: bool,
    bootstrap_required: bool,
    reborn_requested: bool,
    active_task_id: Option<TaskId>,
    active_map_id: Option<ActionMapId>,
    current_main_node_id: Option<MapNodeId>,
    current_main_lease_id: Option<AssignmentLeaseId>,
    maintenance_barriers: HashMap<ActionMapId, ActionMapMaintenanceBarrier>,
    main_tool_reservations: HashMap<String, MainToolReservation>,
    child_tool_reservations: HashMap<String, ChildToolReservation>,
    tasks: HashMap<TaskId, TaskRecord>,
    maps: HashMap<ActionMapId, ActionMapInstance>,
    taskspace_trace_events: Vec<TaskSpaceTraceEvent>,
    pending_projection_trace_events: Vec<MapRuntimeEvent>,
    provider_request_count: usize,
    active_budget: Option<TaskSpaceActiveBudgetV1>,
    budget_counters: TaskSpaceBudgetCounters,
    blocked_action_repeats: HashMap<String, usize>,
    budget_state: TaskSpaceBudgetState,
    budget_violations: Vec<TaskSpaceBudgetViolation>,
    sentinel_warnings: Vec<TaskSpaceSentinelWarning>,
    next_task_seq: u64,
    next_map_seq: u64,
    next_lease_seq: u64,
    next_node_event_seq: u64,
    next_result_seq: u64,
    next_trace_event_seq: u64,
    next_sentinel_warning_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MainToolReservation {
    map_id: ActionMapId,
    node_id: MapNodeId,
    lease_id: AssignmentLeaseId,
    tool_name: String,
    action_class: ActionClass,
    artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChildToolReservation {
    child_thread_id: ThreadId,
    map_id: ActionMapId,
    node_id: MapNodeId,
    lease_id: AssignmentLeaseId,
    action_class: ActionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MainToolTraceDraft {
    task_id: Option<TaskId>,
    map_id: ActionMapId,
    node_id: MapNodeId,
    node_event_id: NodeEventId,
    call_id: String,
    tool_name: String,
    action_class: Option<ActionClass>,
    tool_success: bool,
    body: String,
    artifact_refs: Vec<String>,
    created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderResponseActionabilityTrace {
    trace_event_id: String,
    response_actionability: String,
    recovery_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderRequestReasonFields {
    trigger_kind: String,
    response_actionability_previous: String,
    previous_response_trace_event_id: Option<String>,
    latest_tool_result_refs: Vec<String>,
    model_visible_feedback_refs: Vec<String>,
    adoption_blockers: Vec<String>,
    projection_bundle_hash: String,
    request_reason_delta: String,
    repeated_same_reason_count: usize,
    reason_confidence: String,
}

struct ProviderRequestReasonFingerprint<'a> {
    trigger_kind: &'a str,
    adoption_blockers: &'a [String],
    latest_tool_result_refs: &'a [String],
    projection_bundle_hash: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionMapMaintenanceBarrier {
    map_id: ActionMapId,
    node_id: MapNodeId,
    reason: MaintenanceBarrierReason,
    result_count: usize,
    budget: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum MaintenanceBarrierReason {
    NodeToolResultBudgetExceeded,
}

impl MaintenanceBarrierReason {
    fn as_str(self) -> &'static str {
        match self {
            MaintenanceBarrierReason::NodeToolResultBudgetExceeded => {
                "node_tool_result_budget_exceeded"
            }
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "node_tool_result_budget_exceeded" => Some(Self::NodeToolResultBudgetExceeded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SetMapRuntimeModeOutcome {
    pub(crate) previous_mode: MapRuntimeMode,
    pub(crate) current_mode: MapRuntimeMode,
    pub(crate) changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetTaskSpaceModeOutcome {
    pub(crate) mode: SetMapRuntimeModeOutcome,
    pub(crate) active_map_id: Option<ActionMapId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapAssignment {
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) node_title: String,
    pub(crate) lease_id: AssignmentLeaseId,
    pub(crate) message_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapTimeoutTarget {
    pub(crate) thread_id: ThreadId,
    pub(crate) agent_path: Option<AgentPath>,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) lease_id: AssignmentLeaseId,
}

impl Default for ActionMapRuntimeState {
    fn default() -> Self {
        Self {
            mode: MapRuntimeMode::Standard,
            pending_transition_notice: None,
            routing_required: false,
            bootstrap_required: false,
            reborn_requested: false,
            active_task_id: None,
            active_map_id: None,
            current_main_node_id: None,
            current_main_lease_id: None,
            maintenance_barriers: HashMap::new(),
            main_tool_reservations: HashMap::new(),
            child_tool_reservations: HashMap::new(),
            tasks: HashMap::new(),
            maps: HashMap::new(),
            taskspace_trace_events: Vec::new(),
            pending_projection_trace_events: Vec::new(),
            provider_request_count: 0,
            active_budget: None,
            budget_counters: TaskSpaceBudgetCounters::default(),
            blocked_action_repeats: HashMap::new(),
            budget_state: TaskSpaceBudgetState::Normal,
            budget_violations: Vec::new(),
            sentinel_warnings: Vec::new(),
            next_task_seq: 1,
            next_map_seq: 1,
            next_lease_seq: 1,
            next_node_event_seq: 1,
            next_result_seq: 1,
            next_trace_event_seq: 1,
            next_sentinel_warning_seq: 1,
        }
    }
}

impl ActionMapRuntimeState {
    #[allow(dead_code)]
    pub(crate) const DEFAULT_ACTIVE_SPAWN_AGENT_BUDGET_MAX: usize = 2;
    #[allow(dead_code)]
    pub(crate) const DEFAULT_ACTIVE_NODE_BUDGET_MAX: usize = 8;

    pub(crate) fn mode(&self) -> MapRuntimeMode {
        self.mode
    }

    pub(crate) fn context_owner_node_id(&self) -> Option<&str> {
        self.current_main_node_id.as_deref()
    }

    pub(crate) fn take_pending_projection_trace_events(&mut self) -> Vec<MapRuntimeEvent> {
        std::mem::take(&mut self.pending_projection_trace_events)
    }

    fn ensure_default_active_budget(&mut self) {
        if self.active_budget.is_none() {
            let route_mode = std::env::var("WHALE_TASKSPACE_ROUTE_MODE")
                .ok()
                .and_then(|value| TaskSpaceRouteMode::from_str(value.trim()))
                .unwrap_or(TaskSpaceRouteMode::DefaultCompact);
            let profile_name = std::env::var("WHALE_TASKSPACE_PROFILE_NAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "taskspace-v005-active".to_string());
            self.install_active_budget_for_route(profile_name.trim(), route_mode);
        }
    }

    fn clear_blocked_action_repeats_for_node(&mut self, map_id: &str, node_id: &str) {
        let prefix = format!("{map_id}\x1f{node_id}\x1f");
        self.blocked_action_repeats
            .retain(|key, _| !key.starts_with(&prefix));
    }

    fn install_active_budget_for_route(
        &mut self,
        profile_name: &str,
        route_mode: TaskSpaceRouteMode,
    ) {
        let budget = taskspace_active_budget_for_route(profile_name, route_mode);
        self.active_budget = Some(budget.clone());
        self.budget_state = budget_state_for_counter(
            self.budget_counters.rollout_model_request_count,
            budget.max_rollout_model_requests,
        );
    }

    #[allow(dead_code)]
    pub(crate) fn activate_active_budget_for_route(
        &mut self,
        profile_name: &str,
        route_mode: TaskSpaceRouteMode,
    ) -> Vec<MapRuntimeEvent> {
        self.install_active_budget_for_route(profile_name, route_mode);
        self.record_active_budget_trace_event_if_bound()
            .into_iter()
            .collect()
    }

    #[allow(dead_code)]
    fn record_active_budget_trace_event_if_bound(&mut self) -> Option<MapRuntimeEvent> {
        let budget = self.active_budget.as_ref()?.clone();
        let map_id = self.active_map_id.clone()?;
        let node_id = self.current_main_node_id.clone()?;
        let task_id = self.active_task_id.clone();
        if self
            .taskspace_trace_events
            .iter()
            .any(|event| event.kind == "active_budget" && event.map_id == map_id)
        {
            return None;
        }
        Some(self.record_runtime_budget_trace_event(
            "active_budget",
            task_id,
            map_id,
            node_id,
            None,
            true,
            vec![
                "schema:taskspace-active-budget-v1".to_string(),
                "producer:runtime".to_string(),
                "active_budget_source:runtime".to_string(),
                "enforcement:advisory".to_string(),
                format!("profile_name:{}", budget.profile_name),
                format!("route_mode:{}", budget.route_mode.as_str()),
                format!(
                    "max_rollout_model_requests:{}",
                    budget.max_rollout_model_requests
                ),
                format!(
                    "max_model_requests_per_node:{}",
                    budget.max_model_requests_per_node
                ),
                format!("max_spawn_agent_calls:{}", budget.max_spawn_agent_calls),
                format!("max_subagent_results:{}", budget.max_subagent_results),
                format!("max_nodes:{}", budget.max_nodes),
                format!("max_open_leaf_nodes:{}", budget.max_open_leaf_nodes),
                format!("max_projection_tokens:{}", budget.max_projection_tokens),
                format!(
                    "max_avg_input_tokens_per_request:{}",
                    budget.max_avg_input_tokens_per_request
                ),
                format!("budget_state:{}", self.budget_state.as_str()),
            ],
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn active_budget(&self) -> Option<&TaskSpaceActiveBudgetV1> {
        self.active_budget.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn budget_counters(&self) -> &TaskSpaceBudgetCounters {
        &self.budget_counters
    }

    #[allow(dead_code)]
    pub(crate) fn update_budget_state_for_counter(
        &mut self,
        counter_name: &str,
        counter_value: usize,
        counter_limit: usize,
        action_context: &str,
    ) -> Option<MapRuntimeEvent> {
        let state_before = self.budget_state;
        let state_after = budget_state_for_counter(counter_value, counter_limit);
        self.budget_state = state_after;
        if state_before == state_after {
            return None;
        }
        let violation = TaskSpaceBudgetViolation {
            violation_id: format!("budget-violation-{}", self.budget_violations.len() + 1),
            counter_name: counter_name.to_string(),
            counter_value,
            counter_limit,
            state_before,
            state_after,
            action_taken: action_context.to_string(),
            created_at_ms: now_ms(),
        };
        self.budget_violations.push(violation.clone());
        let map_id = self
            .active_map_id
            .clone()
            .unwrap_or_else(|| "budget-not-bound".to_string());
        let node_id = self
            .current_main_node_id
            .clone()
            .unwrap_or_else(|| "budget-not-bound".to_string());
        let task_id = self.active_task_id.clone();
        Some(self.record_runtime_budget_trace_event(
            "budget_state_transition",
            task_id,
            map_id,
            node_id,
            None,
            true,
            vec![
                "schema:taskspace-budget-state-transition-v1".to_string(),
                "producer:runtime".to_string(),
                format!("counter_name:{counter_name}"),
                format!("counter_value:{counter_value}"),
                format!("counter_limit:{counter_limit}"),
                format!("state_before:{}", state_before.as_str()),
                format!("state_after:{}", state_after.as_str()),
                format!("action_taken:{action_context}"),
                format!("violation_id:{}", violation.violation_id),
            ],
        ))
    }

    pub(crate) fn gate_spawn_budget(
        &mut self,
        _map_id: &str,
        _parent_node_id: &str,
    ) -> TaskSpaceBudgetGateDecision {
        let budget = self.active_budget.as_ref();
        let max_spawn = budget
            .map(|budget| budget.max_spawn_agent_calls)
            .unwrap_or(Self::DEFAULT_ACTIVE_SPAWN_AGENT_BUDGET_MAX);
        let spawn_count = self.budget_counters.spawn_agent_call_count;
        TaskSpaceBudgetGateDecision {
            allowed: true,
            budget_state: self.budget_state,
            reason: if spawn_count < max_spawn {
                "spawn_budget_available".to_string()
            } else if max_spawn == 0 {
                "route_spawn_profile_hint_exceeded".to_string()
            } else {
                "spawn_profile_hint_exceeded".to_string()
            },
            blocking_items: Vec::new(),
            next_valid_actions: Vec::new(),
            recovery_request_phase: None,
            quality_impact_required: false,
        }
    }

    fn reconstruct_budget_state_from_trace_events(&mut self) {
        let mut route_mode = None;
        let mut profile_name = None;
        let mut counters = TaskSpaceBudgetCounters::default();
        for event in &self.taskspace_trace_events {
            route_mode = trace_tag_value(&event.tags, "route_mode")
                .and_then(TaskSpaceRouteMode::from_str)
                .or(route_mode);
            profile_name = trace_tag_value(&event.tags, "profile_name")
                .map(str::to_string)
                .or(profile_name);
            match event.kind.as_str() {
                "active_budget" => {}
                "provider_request_budget" => {
                    let request_count_after =
                        trace_tag_usize(&event.tags, "request_count_after").unwrap_or_default();
                    counters.rollout_model_request_count = counters
                        .rollout_model_request_count
                        .max(request_count_after);
                    if trace_tag_value(&event.tags, "status") == Some("started") {
                        let node_count =
                            trace_tag_usize(&event.tags, "node_request_count").unwrap_or_default();
                        if !event.node_id.is_empty() && event.node_id != "provider-context-missing"
                        {
                            counters
                                .model_request_count_by_node
                                .insert(event.node_id.clone(), node_count);
                        }
                    }
                }
                "spawn_node_budget" => match trace_tag_value(&event.tags, "budget_kind") {
                    Some("spawn") => {
                        counters.spawn_agent_call_count = counters.spawn_agent_call_count.max(
                            trace_tag_usize(&event.tags, "spawn_agent_call_count_after")
                                .unwrap_or_default(),
                        );
                    }
                    Some("node") => {
                        counters.node_count = counters.node_count.max(
                            trace_tag_usize(&event.tags, "node_count_after").unwrap_or_default(),
                        );
                    }
                    _ => {}
                },
                "projection_budget" => {
                    counters.projection_tokens_last =
                        trace_tag_usize(&event.tags, "projection_tokens").unwrap_or_default();
                    counters.projection_tokens_max = counters
                        .projection_tokens_max
                        .max(trace_tag_usize(&event.tags, "projection_tokens").unwrap_or_default());
                }
                _ => {}
            }
        }
        let route_mode = route_mode.unwrap_or(TaskSpaceRouteMode::DefaultCompact);
        let profile_name = profile_name.unwrap_or_else(|| "taskspace-v005-active".to_string());
        self.active_budget = Some(taskspace_active_budget_for_route(&profile_name, route_mode));
        self.budget_counters = counters;
        self.provider_request_count = self.budget_counters.rollout_model_request_count;
        if let Some(budget) = self.active_budget.as_ref() {
            self.budget_state = budget_state_for_counter(
                self.budget_counters.rollout_model_request_count,
                budget.max_rollout_model_requests,
            );
        }
    }

    pub(crate) fn set_mode(&mut self, mode: MapRuntimeMode) -> SetMapRuntimeModeOutcome {
        let previous_mode = self.mode;
        self.mode = mode;
        if previous_mode != mode {
            self.pending_transition_notice = transition_notice(previous_mode, mode);
            if mode == MapRuntimeMode::Experiment {
                self.routing_required = true;
                self.bootstrap_required = self.active_map_id.is_none();
            } else {
                self.routing_required = false;
                self.bootstrap_required = false;
                self.reborn_requested = false;
                self.active_budget = None;
                self.budget_counters = TaskSpaceBudgetCounters::default();
                self.budget_state = TaskSpaceBudgetState::Normal;
                self.budget_violations.clear();
            }
        }
        if self.mode == MapRuntimeMode::Experiment {
            self.ensure_default_active_budget();
        }
        SetMapRuntimeModeOutcome {
            previous_mode,
            current_mode: self.mode,
            changed: previous_mode != self.mode,
        }
    }

    pub(crate) fn set_mode_for_session(
        &mut self,
        mode: MapRuntimeMode,
        owner_session_id: ThreadId,
    ) -> (SetTaskSpaceModeOutcome, Vec<MapRuntimeEvent>) {
        let trace_len_before = self.taskspace_trace_events.len();
        let mode_outcome = self.set_mode(mode);
        let mut events = self.taskspace_trace_events[trace_len_before..]
            .iter()
            .cloned()
            .map(map_runtime_event_from_trace_event)
            .collect::<Vec<_>>();
        if mode == MapRuntimeMode::Experiment {
            events.extend(self.ensure_mechanical_blank_task_path_for_main(owner_session_id));
        }
        (
            SetTaskSpaceModeOutcome {
                mode: mode_outcome,
                active_map_id: self.active_map_id.clone(),
            },
            events,
        )
    }

    fn ensure_mechanical_blank_task_path_for_main(
        &mut self,
        owner_session_id: ThreadId,
    ) -> Vec<MapRuntimeEvent> {
        if self.mode != MapRuntimeMode::Experiment || self.active_map_id.is_some() {
            return Vec::new();
        }

        let task_id = self.next_task_id();
        let map_id = self.next_map_id();
        let task = TaskRecord {
            owner_session_id: Some(owner_session_id),
        };
        self.tasks.insert(task_id.clone(), task);
        self.active_task_id = Some(task_id.clone());
        self.active_map_id = Some(map_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.routing_required = false;
        self.bootstrap_required = true;
        self.reborn_requested = false;

        let mut events = Vec::new();
        events.push(self.record_runtime_budget_trace_event(
            "mechanical_blank_map_initialized",
            Some(task_id),
            map_id,
            "map-uninitialized".to_string(),
            None,
            true,
            vec![
                "schema:taskspace-mechanical-init-v1".to_string(),
                "producer:runtime".to_string(),
                "initialization:mechanical_blank".to_string(),
                "semantic_seed:false".to_string(),
            ],
        ));
        events
    }

    pub(crate) fn restore_mode(&mut self, mode: MapRuntimeMode) {
        *self = Self::default();
        self.mode = mode;
        self.routing_required = mode == MapRuntimeMode::Experiment;
        self.bootstrap_required = mode == MapRuntimeMode::Experiment;
    }

    pub(crate) fn begin_user_turn(&mut self) -> bool {
        if self.mode != MapRuntimeMode::Experiment {
            return false;
        }
        let previous = (self.routing_required, self.bootstrap_required);
        if self.reborn_requested {
            self.routing_required = true;
            self.bootstrap_required = self.active_map_id.is_none();
        } else if self.active_map().is_some() {
            self.routing_required = false;
            self.bootstrap_required = false;
        } else if self.active_map_id.is_some() {
            self.routing_required = false;
            self.bootstrap_required = true;
        } else {
            self.routing_required = true;
            self.bootstrap_required = true;
        }
        previous != (self.routing_required, self.bootstrap_required)
    }

    pub(crate) fn request_reborn(&mut self) -> Vec<MapRuntimeEvent> {
        let mut events = Vec::new();
        if self.mode != MapRuntimeMode::Experiment {
            let outcome = self.set_mode(MapRuntimeMode::Experiment);
            events.push(MapRuntimeEvent::ModeChanged(MapRuntimeModeChangedEvent {
                previous_mode: outcome.previous_mode,
                current_mode: outcome.current_mode,
            }));
        }
        self.reborn_requested = true;
        self.routing_required = true;
        if self.active_map_id.is_none() {
            self.bootstrap_required = true;
        }
        events
    }

    pub(crate) fn restore_snapshot(&mut self, snapshot: ActionMapSnapshot) -> Result<(), String> {
        if snapshot.schema_version != TASKSPACE_SNAPSHOT_SCHEMA_VERSION {
            return Err(serde_json::json!({
                "schema_version": TASKSPACE_SNAPSHOT_SCHEMA_VERSION,
                "status": "fatal",
                "error": {
                    "code": "legacy_schema_unsupported",
                    "received_schema_version": snapshot.schema_version,
                },
            })
            .to_string());
        }
        let restored_blank_task_path = if snapshot.mode == MapRuntimeMode::Experiment
            && snapshot.map.is_none()
            && snapshot.bootstrap_required
            && !snapshot.routing_required
        {
            snapshot.trace_events.iter().rev().find_map(|event| {
                if event.kind != "mechanical_blank_map_initialized" {
                    return None;
                }
                Some((event.task_id.clone()?, event.map_id.clone()))
            })
        } else {
            None
        };
        let maintenance_barriers = snapshot.maintenance_barriers;
        self.mode = snapshot.mode;
        self.pending_transition_notice = None;
        self.routing_required = snapshot.routing_required;
        self.bootstrap_required = snapshot.bootstrap_required;
        self.reborn_requested = snapshot.reborn_requested;
        self.active_task_id = None;
        self.active_map_id = None;
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.main_tool_reservations.clear();
        self.child_tool_reservations.clear();
        self.pending_projection_trace_events.clear();
        self.taskspace_trace_events = snapshot
            .trace_events
            .into_iter()
            .map(|event| TaskSpaceTraceEvent {
                id: event.id,
                kind: event.kind,
                task_id: event.task_id,
                map_id: event.map_id,
                node_id: event.node_id,
                result_id: event.result_id,
                call_id: event.call_id,
                action_class: event
                    .action_class
                    .as_deref()
                    .and_then(ActionClass::from_str),
                tool_success: event.tool_success,
                tags: event.tags,
                artifact_refs: event.artifact_refs,
                created_at_ms: event.created_at_ms,
            })
            .collect();
        self.sentinel_warnings = snapshot
            .sentinel_warnings
            .into_iter()
            .filter_map(|warning| {
                Some(TaskSpaceSentinelWarning {
                    id: warning.id,
                    warning_type: TaskSpaceSentinelWarningType::from_str(&warning.sentinel_type)?,
                    status: TaskSpaceSentinelWarningStatus::from_str(&warning.status)?,
                    severity: TaskSpaceSentinelSeverity::from_str(&warning.severity)?,
                    task_id: warning.task_id,
                    map_id: warning.map_id,
                    node_id: warning.node_id,
                    result_id: warning.result_id,
                    trace_event_ids: warning.trace_event_ids,
                    reason: warning.reason,
                    clearance_action: warning.clearance_action,
                    clear_action: warning.clear_action,
                    created_at_ms: warning.created_at_ms,
                    cleared_at_ms: warning.cleared_at_ms,
                })
            })
            .collect();

        self.tasks.clear();
        self.maps.clear();
        if let Some((task_id, map_id)) = restored_blank_task_path.as_ref() {
            self.tasks.insert(
                task_id.clone(),
                TaskRecord {
                    owner_session_id: None,
                },
            );
            self.active_task_id = Some(task_id.clone());
            self.active_map_id = Some(map_id.clone());
        }
        if let Some(map) = snapshot.map {
            let map_id = map.id.clone();
            let task_id = map
                .task_id
                .clone()
                .unwrap_or_else(|| format!("task-{map_id}"));
            let results = map
                .results
                .into_iter()
                .map(|result| {
                    let kind = node_result_kind_from_str(&result.kind).ok_or_else(|| {
                        format!(
                            "TaskSpace snapshot result kind `{}` is invalid.",
                            result.kind
                        )
                    })?;
                    Ok((
                        result.id.clone(),
                        NodeResult {
                            id: result.id,
                            assignment_id: result.assignment_id,
                            map_id: result.map_id,
                            node_id: result.node_id,
                            kind,
                            action_class: result
                                .action_class
                                .as_deref()
                                .and_then(ActionClass::from_str),
                            tool_success: result.tool_success,
                            source_event_ref: result.source_event_ref,
                            artifact_refs: result.artifact_refs,
                            source_thread_id: result.source_thread_id,
                            created_at_ms: result.created_at_ms,
                        },
                    ))
                })
                .collect::<Result<HashMap<_, _>, String>>()?;
            let node_events = map
                .node_events
                .into_iter()
                .map(|event| {
                    (
                        event.id.clone(),
                        NodeEvent {
                            id: event.id,
                            map_id: event.map_id,
                            node_id: event.node_id,
                            event_kind: event.event_kind,
                            source: event.source,
                            action_class: event
                                .action_class
                                .as_deref()
                                .and_then(ActionClass::from_str),
                            tool_success: event.tool_success,
                            content_sha256: event.content_sha256,
                            source_event_id: event.source_event_id,
                            raw_ref: event.raw_ref,
                            artifact_refs: event.artifact_refs,
                            call_id: event.call_id,
                            source_thread_id: event.source_thread_id,
                            created_at_ms: event.created_at_ms,
                        },
                    )
                })
                .collect::<HashMap<_, _>>();
            let nodes = map
                .nodes
                .into_iter()
                .map(|node| {
                    let role = node_role_from_str(&node.role).ok_or_else(|| {
                        format!("TaskSpace node role `{}` is invalid.", node.role)
                    })?;
                    let status = node_status_from_str(&node.status).ok_or_else(|| {
                        format!("TaskSpace node status `{}` is invalid.", node.status)
                    })?;
                    let result_context = node
                        .result_ids
                        .into_iter()
                        .map(|result_id| {
                            let kind = results
                                .get(&result_id)
                                .ok_or_else(|| {
                                    format!("TaskSpace result `{result_id}` is missing.")
                                })?
                                .kind;
                            Ok(NodeResultRef {
                                id: result_id,
                                kind,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    let event_refs = node
                        .node_event_ids
                        .into_iter()
                        .map(|event_id| {
                            let kind = node_events
                                .get(&event_id)
                                .ok_or_else(|| {
                                    format!("TaskSpace node event `{event_id}` is missing.")
                                })?
                                .event_kind
                                .clone();
                            Ok(NodeEventRef { id: event_id, kind })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok((
                        node.id,
                        MapNode {
                            role,
                            goal: node.goal,
                            source_refs: node.source_refs,
                            status,
                            active_lease: node.active_lease,
                            result_context,
                            node_events: event_refs,
                            origin_node_id: None,
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            let graph = TaskSpaceMap {
                id: map_id.clone(),
                root_node_id: map.root_node_id,
                finish_node_id: map.finish_node_id,
                nodes,
                edges: map
                    .edges
                    .into_iter()
                    .map(|edge| MapEdge::new(edge.from, edge.to))
                    .collect(),
                revision: map.revision,
                current_binding: map.current_node_id,
                terminal_summary_ref: map.terminal_summary_ref,
            };
            let violations = validate(&graph);
            if !violations.is_empty() {
                return Err(rooted_rejection_message(Rejection {
                    state_commit: false,
                    current_revision: graph.revision,
                    violations,
                }));
            }
            let mut instance =
                ActionMapInstance::from_graph(graph, Vec::new(), map.owner_session_id);
            instance.task_id = Some(task_id.clone());
            instance.results = results;
            instance.node_events = node_events;
            instance.leases = map
                .leases
                .into_iter()
                .map(|lease| {
                    let holder = lease_holder_from_str(&lease.holder).ok_or_else(|| {
                        format!("TaskSpace lease holder `{}` is invalid.", lease.holder)
                    })?;
                    let previous_node_status = node_status_from_str(&lease.previous_node_status)
                        .ok_or_else(|| {
                            format!(
                                "TaskSpace lease status `{}` is invalid.",
                                lease.previous_node_status
                            )
                        })?;
                    Ok((
                        lease.id.clone(),
                        AssignmentLease {
                            id: lease.id,
                            map_id: lease.map_id,
                            node_id: lease.node_id,
                            holder,
                            previous_node_status,
                            agent_thread_id: lease.agent_thread_id,
                            agent_path: lease.agent_path,
                        },
                    ))
                })
                .collect::<Result<HashMap<_, _>, String>>()?;
            validate_active_frontier_leases(&instance)?;
            let complete = instance.is_complete();
            self.tasks.insert(
                task_id.clone(),
                TaskRecord {
                    owner_session_id: instance.owner_session_id,
                },
            );
            self.maps.insert(map_id.clone(), instance);
            if !complete {
                self.active_task_id = Some(task_id);
                self.active_map_id = Some(map_id);
            }
        }

        self.maintenance_barriers = maintenance_barriers
            .into_iter()
            .filter_map(|barrier| {
                let reason = MaintenanceBarrierReason::from_str(&barrier.reason)?;
                Some((
                    barrier.map_id.clone(),
                    ActionMapMaintenanceBarrier {
                        map_id: barrier.map_id,
                        node_id: barrier.node_id,
                        reason,
                        result_count: barrier.result_count,
                        budget: barrier.budget,
                    },
                ))
            })
            .collect();

        let blank_task_path_is_coherent = restored_blank_task_path.is_some()
            && self.active_task_id.is_some()
            && self.active_map_id.is_some()
            && self.maps.is_empty();
        if self.mode == MapRuntimeMode::Experiment
            && !blank_task_path_is_coherent
            && !self.restored_active_binding_is_coherent()
        {
            self.active_task_id = None;
            self.active_map_id = None;
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            self.routing_required = true;
            self.bootstrap_required = self.maps.is_empty();
        }

        if let Some(active_map_id) = self.active_map_id.as_ref()
            && let Some(map) = self.maps.get(active_map_id)
            && let Some(lease) = map
                .leases
                .values()
                .find(|lease| lease.holder == LeaseHolder::Main)
        {
            self.current_main_node_id = Some(lease.node_id.clone());
            self.current_main_lease_id = Some(lease.id.clone());
        }

        self.next_task_seq = next_numeric_seq(self.tasks.keys(), "task-");
        self.next_map_seq = next_numeric_seq(self.maps.keys(), "map-");
        self.next_lease_seq = next_numeric_seq(
            self.maps.values().flat_map(|map| map.leases.keys()),
            "lease-",
        );
        self.next_node_event_seq = next_numeric_seq(
            self.maps.values().flat_map(|map| map.node_events.keys()),
            "node-event-",
        );
        self.next_result_seq = next_numeric_seq(
            self.maps.values().flat_map(|map| map.results.keys()),
            "result-",
        );
        self.next_trace_event_seq = next_numeric_seq(
            self.taskspace_trace_events.iter().map(|event| &event.id),
            "trace-",
        );
        self.next_sentinel_warning_seq = next_numeric_seq(
            self.sentinel_warnings.iter().map(|warning| &warning.id),
            "sentinel-",
        );
        if self.mode != MapRuntimeMode::Experiment {
            self.routing_required = false;
            self.bootstrap_required = false;
            self.reborn_requested = false;
        } else if blank_task_path_is_coherent {
            self.bootstrap_required = true;
        } else {
            self.bootstrap_required = self.bootstrap_required || self.maps.is_empty();
            self.routing_required =
                self.routing_required || self.bootstrap_required || self.active_task_id.is_none();
            if self
                .taskspace_trace_events
                .iter()
                .any(|event| event.kind == "active_budget")
            {
                self.reconstruct_budget_state_from_trace_events();
            } else {
                self.ensure_default_active_budget();
                self.reconstruct_budget_state_from_trace_events();
            }
        }
        Ok(())
    }

    pub(crate) fn rebind_after_fork(&mut self, owner_session_id: ThreadId) -> usize {
        for task in self.tasks.values_mut() {
            task.owner_session_id = Some(owner_session_id);
        }
        for map in self.maps.values_mut() {
            map.owner_session_id = Some(owner_session_id);
        }

        let stale_child_leases = self
            .maps
            .values()
            .flat_map(|map| map.leases.values())
            .filter(|lease| lease.holder == LeaseHolder::SubAgent)
            .map(|lease| lease.id.clone())
            .collect::<Vec<_>>();
        for lease_id in &stale_child_leases {
            self.release_lease(lease_id, "fork_owner_rebind");
        }

        for map in self.maps.values_mut() {
            for lease in map.leases.values_mut() {
                if lease.holder == LeaseHolder::Main {
                    lease.agent_thread_id = Some(owner_session_id);
                    lease.agent_path = None;
                }
            }
        }
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        if let Some(active_map_id) = self.active_map_id.as_ref()
            && let Some(map) = self.maps.get(active_map_id)
            && let Some(lease) = map
                .leases
                .values()
                .find(|lease| lease.holder == LeaseHolder::Main)
        {
            self.current_main_node_id = Some(lease.node_id.clone());
            self.current_main_lease_id = Some(lease.id.clone());
        }
        stale_child_leases.len()
    }

    fn restored_active_binding_is_coherent(&self) -> bool {
        match (
            self.active_task_id.as_deref(),
            self.active_map_id.as_deref(),
        ) {
            (None, None) => true,
            (Some(task_id), Some(map_id)) => {
                if !self.tasks.contains_key(task_id) {
                    return false;
                }
                let Some(map) = self.maps.get(map_id) else {
                    return false;
                };
                map.task_id.as_deref() == Some(task_id) && !map.is_complete()
            }
            (Some(_), None) => false,
            (None, Some(_)) => false,
        }
    }

    pub(crate) fn take_pending_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }

    pub(crate) fn active_map(&self) -> Option<&ActionMapInstance> {
        self.active_map_id
            .as_ref()
            .and_then(|map_id| self.maps.get(map_id))
            .filter(|map| !map.is_complete())
    }

    pub(crate) fn control_state(&self, map_id_hint: Option<&str>) -> Option<ActionMapControlState> {
        let map_id = map_id_hint
            .map(str::to_string)
            .or_else(|| self.active_map_id.clone())?;
        let map = self.maps.get(&map_id)?;
        let task_id = map.task_id.as_ref()?.clone();
        let work_nodes_with_status = |status| {
            map.nodes
                .iter()
                .filter(|(_, node)| node.role == NodeRole::Work && node.status == status)
                .map(|(node_id, _)| node_id.clone())
                .collect::<Vec<_>>()
        };
        let mut pending_work_node_ids = work_nodes_with_status(NodeStatus::Pending);
        let mut ready_work_node_ids = work_nodes_with_status(NodeStatus::Ready);
        let mut running_work_node_ids = work_nodes_with_status(NodeStatus::Running);
        let mut blocked_work_node_ids = work_nodes_with_status(NodeStatus::Blocked);
        pending_work_node_ids.sort();
        ready_work_node_ids.sort();
        running_work_node_ids.sort();
        blocked_work_node_ids.sort();
        let finish_ready = map
            .nodes
            .get(&map.finish_node_id)
            .is_some_and(|node| node.status == NodeStatus::Ready);
        Some(ActionMapControlState {
            task_id,
            map_id,
            revision: map.revision,
            root_node_id: map.root_node_id.clone(),
            finish_node_id: map.finish_node_id.clone(),
            complete: map.is_complete(),
            current_node_id: map.current_binding.clone(),
            pending_work_node_ids,
            ready_work_node_ids,
            running_work_node_ids,
            blocked_work_node_ids,
            finish_ready,
            completed_work_node_count: map
                .nodes
                .values()
                .filter(|node| node.role == NodeRole::Work && node.status == NodeStatus::Completed)
                .count(),
            total_node_count: map.nodes.len(),
        })
    }

    pub(crate) fn snapshot(&self) -> ActionMapSnapshot {
        let map = self
            .active_map_id
            .as_ref()
            .and_then(|map_id| self.maps.get(map_id))
            .or_else(|| {
                self.maps
                    .values()
                    .max_by(|left, right| left.id.cmp(&right.id))
            })
            .map(snapshot_map);
        let mut maintenance_barriers = self
            .maintenance_barriers
            .values()
            .map(snapshot_maintenance_barrier)
            .collect::<Vec<_>>();
        maintenance_barriers.sort_by(|left, right| left.map_id.cmp(&right.map_id));
        let trace_events = self
            .taskspace_trace_events
            .iter()
            .map(snapshot_trace_event_ref)
            .collect::<Vec<_>>();
        let sentinel_warnings = self
            .sentinel_warnings
            .iter()
            .map(snapshot_sentinel_warning_ref)
            .collect::<Vec<_>>();
        ActionMapSnapshot {
            schema_version: TASKSPACE_SNAPSHOT_SCHEMA_VERSION.to_string(),
            mode: self.mode,
            routing_required: self.routing_required,
            bootstrap_required: self.bootstrap_required,
            reborn_requested: self.reborn_requested,
            map,
            maintenance_barriers,
            trace_summary: trace_summary(&trace_events),
            trace_events,
            sentinel_summary: sentinel_summary(&sentinel_warnings),
            sentinel_warnings,
        }
    }

    pub(crate) fn prepare_main_tool_call(
        &mut self,
        owner_session_id: ThreadId,
        descriptor: impl Into<ToolActionDescriptor>,
    ) -> Result<Vec<MapRuntimeEvent>, ActionMapGateError> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(Vec::new());
        }

        let descriptor = descriptor.into();
        self.validate_routing_complete()?;
        if descriptor.action_class != ActionClass::Control {
            self.validate_maintenance_barrier()?;
        }
        self.validate_main_binding(owner_session_id)?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no active task path exists.")
        })?;
        let node_id = self.current_main_node_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no current node binding exists.")
        })?;
        let lease_id = self.current_main_lease_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no current main lease exists.")
        })?;
        if descriptor.action_class != ActionClass::Control
            && let Some(call_id) = descriptor.call_id.as_deref()
        {
            let artifact_refs = tool_action_descriptor_artifact_refs(&descriptor);
            self.reserve_main_tool_call(
                call_id,
                MainToolReservation {
                    map_id,
                    node_id,
                    lease_id,
                    tool_name: descriptor.tool_name,
                    action_class: descriptor.action_class,
                    artifact_refs,
                },
            );
        }
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    pub(crate) fn record_main_tool_result(
        &mut self,
        owner_session_id: ThreadId,
        call_id: &str,
        source_event_id: String,
        tool_name: &str,
        success: bool,
        body: String,
    ) -> Result<Option<(NodeEventId, Vec<MapRuntimeEvent>)>, String> {
        self.record_main_tool_result_with_class(
            owner_session_id,
            call_id,
            source_event_id,
            tool_name,
            None,
            success,
            body,
        )
    }

    pub(crate) fn record_main_tool_result_with_class(
        &mut self,
        owner_session_id: ThreadId,
        call_id: &str,
        source_event_id: String,
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        body: String,
    ) -> Result<Option<(NodeEventId, Vec<MapRuntimeEvent>)>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }
        let reservation = self.release_main_tool_reservation(call_id);
        let source_event_id = require_nonempty_owned("source_event_id", source_event_id)?;
        let (
            map_id,
            node_id,
            lease_id,
            recorded_tool_name,
            recorded_action_class,
            reserved_artifact_refs,
        ) = if let Some(reservation) = reservation {
            self.validate_main_tool_reservation(owner_session_id, &reservation)?;
            if let Some(observed_action_class) = action_class
                && observed_action_class != reservation.action_class
            {
                return Err(format!(
                    "TaskSpace tool result `{call_id}` action class changed from {} to {} while in flight.",
                    reservation.action_class.as_str(),
                    observed_action_class.as_str()
                ));
            }
            (
                reservation.map_id,
                reservation.node_id,
                reservation.lease_id,
                reservation.tool_name,
                Some(reservation.action_class),
                reservation.artifact_refs,
            )
        } else {
            self.validate_main_binding(owner_session_id)?;
            let map_id = self.active_map_id.clone().ok_or_else(|| {
                "TaskSpace mode is active but no active task path exists.".to_string()
            })?;
            let node_id = self.current_main_node_id.clone().ok_or_else(|| {
                "TaskSpace mode is active but no current node binding exists.".to_string()
            })?;
            let lease_id = self.current_main_lease_id.clone().ok_or_else(|| {
                "TaskSpace mode is active but no current main lease exists.".to_string()
            })?;
            (
                map_id,
                node_id,
                lease_id,
                tool_name.to_string(),
                action_class,
                Vec::new(),
            )
        };
        let node_event_id = self.next_node_event_id();
        let created_at_ms = now_ms();
        let mut artifact_refs = match (recorded_action_class, success) {
            (Some(ActionClass::Edit), true) => merge_artifact_refs(
                reserved_artifact_refs,
                extract_edit_changed_artifacts_from_tool_body(&body),
            ),
            (Some(ActionClass::Read), true) => reserved_artifact_refs,
            _ => Vec::new(),
        };
        if recorded_action_class == Some(ActionClass::Read) && success {
            for artifact in result_body_taskspace_marker_artifact_refs(&body) {
                push_unique_artifact_ref(&mut artifact_refs, artifact);
            }
            if let Some(command) = result_body_command_from_body(&body)
                && let Some(artifact) = read_command_artifact_ref(&command)
            {
                push_unique_artifact_ref(&mut artifact_refs, artifact);
            }
        }
        let raw_ref = first_output_ref_in_text(&body);
        let task_id = {
            let map = self
                .maps
                .get_mut(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            let task_id = map.task_id.clone();
            let node_event = NodeEvent {
                id: node_event_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                event_kind: "tool_result".to_string(),
                source: "main_tool".to_string(),
                action_class: recorded_action_class,
                tool_success: Some(success),
                content_sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
                source_event_id: Some(source_event_id),
                raw_ref,
                artifact_refs: artifact_refs.clone(),
                call_id: Some(call_id.to_string()),
                source_thread_id: owner_session_id,
                created_at_ms,
            };
            map.node_events.insert(node_event_id.clone(), node_event);
            map.nodes
                .get_mut(&node_id)
                .ok_or_else(|| format!("TaskSpace current node `{node_id}` is missing."))?
                .node_events
                .push(NodeEventRef {
                    id: node_event_id.clone(),
                    kind: "tool_result".to_string(),
                });
            task_id
        };
        self.clear_blocked_action_repeats_for_node(&map_id, &node_id);
        let trace_events = self.append_main_tool_trace_events(MainToolTraceDraft {
            task_id,
            map_id: map_id.clone(),
            node_id: node_id.clone(),
            node_event_id: node_event_id.clone(),
            call_id: call_id.to_string(),
            tool_name: recorded_tool_name,
            action_class: recorded_action_class,
            tool_success: success,
            body,
            artifact_refs,
            created_at_ms,
        });
        let mut events = vec![MapRuntimeEvent::NodeEventRecorded(
            MapRuntimeNodeEventRecordedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id,
                node_event_id: node_event_id.clone(),
                event_kind: "tool_result".to_string(),
                action_class: recorded_action_class.map(|class| class.as_str().to_string()),
                tool_success: Some(success),
                source_thread_id: owner_session_id,
            },
        )];
        events.extend(trace_events);
        Ok(Some((node_event_id, events)))
    }

    pub(crate) fn record_output_ref_trace_event(
        &mut self,
        kind: &str,
        call_id: Option<String>,
        artifact_ref: String,
        tags: Vec<String>,
    ) -> Option<Vec<MapRuntimeEvent>> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let map_id = self.active_map_id.clone()?;
        let node_id = self.current_main_node_id.clone().or_else(|| {
            self.maps.get(&map_id).and_then(|map| {
                map.nodes
                    .iter()
                    .filter(|(_, node)| node.status == NodeStatus::Ready)
                    .min_by_key(|(id, _)| id.as_str())
                    .map(|(id, _)| id.clone())
            })
        })?;
        let task_id = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        let id = self.next_trace_event_id();
        let created_at_ms = now_ms();
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: kind.to_string(),
            task_id,
            map_id,
            node_id,
            result_id: None,
            call_id,
            action_class: None,
            tool_success: Some(true),
            tags,
            artifact_refs: vec![artifact_ref],
            created_at_ms,
        };
        self.taskspace_trace_events.push(event.clone());
        Some(vec![MapRuntimeEvent::TaskspaceTraceEventRecorded(
            MapRuntimeTraceEventRecordedEvent {
                trace_event_id: id,
                kind: event.kind.clone(),
                task_id: event.task_id.clone(),
                map_id: event.map_id.clone(),
                node_id: event.node_id.clone(),
                result_id: event.result_id.clone(),
                call_id: event.call_id.clone(),
                action_class: None,
                tool_success: event.tool_success,
                tags: event.tags.clone(),
                artifact_refs: event.artifact_refs.clone(),
                created_at_ms: event.created_at_ms,
            },
        )])
    }

    pub(crate) fn record_map_read_trace_event(
        &mut self,
        call_id: String,
        policy: &str,
        map_id: Option<String>,
        revision: Option<u64>,
        canonical_sha256: Option<String>,
        projection_sha256: String,
    ) -> Option<Vec<MapRuntimeEvent>> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let map_id = map_id.unwrap_or_else(|| "map-uninitialized".to_string());
        let previous_revision = self
            .taskspace_trace_events
            .iter()
            .rev()
            .find(|event| event.kind == "map.read_completed" && event.map_id == map_id)
            .and_then(|event| {
                event
                    .tags
                    .iter()
                    .find_map(|tag| tag.strip_prefix("revision:"))
                    .and_then(|value| value.parse::<u64>().ok())
            });
        let revision_advance_since_previous_read = previous_revision
            .zip(revision)
            .map(|(previous, current)| current.saturating_sub(previous));
        let repeated_revision = previous_revision.is_some() && previous_revision == revision;
        let task_id = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        let node_id = self
            .current_main_node_id
            .clone()
            .unwrap_or_else(|| "map-read".to_string());
        let id = self.next_trace_event_id();
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: "map.read_completed".to_string(),
            task_id,
            map_id,
            node_id,
            result_id: None,
            call_id: Some(call_id),
            action_class: None,
            tool_success: Some(true),
            tags: vec![
                "schema:taskspace-map-read-event-v1".to_string(),
                "status:completed".to_string(),
                format!("policy:{policy}"),
                format!(
                    "revision:{}",
                    revision.map_or_else(|| "none".to_string(), |value| value.to_string())
                ),
                format!(
                    "previous_read_revision:{}",
                    previous_revision.map_or_else(|| "none".to_string(), |value| value.to_string())
                ),
                format!(
                    "revision_advance_since_previous_read:{}",
                    revision_advance_since_previous_read
                        .map_or_else(|| "none".to_string(), |value| value.to_string())
                ),
                "canonical_revision_lag:0".to_string(),
                format!("repeated_revision:{repeated_revision}"),
                format!(
                    "canonical_sha256:{}",
                    canonical_sha256.unwrap_or_else(|| "none".to_string())
                ),
                format!("projection_sha256:{projection_sha256}"),
            ],
            artifact_refs: Vec::new(),
            created_at_ms: now_ms(),
        };
        self.taskspace_trace_events.push(event.clone());
        Some(vec![MapRuntimeEvent::TaskspaceTraceEventRecorded(
            MapRuntimeTraceEventRecordedEvent {
                trace_event_id: id,
                kind: event.kind.clone(),
                task_id: event.task_id.clone(),
                map_id: event.map_id.clone(),
                node_id: event.node_id.clone(),
                result_id: None,
                call_id: event.call_id.clone(),
                action_class: None,
                tool_success: Some(true),
                tags: event.tags.clone(),
                artifact_refs: Vec::new(),
                created_at_ms: event.created_at_ms,
            },
        )])
    }

    fn record_runtime_budget_trace_event(
        &mut self,
        kind: &str,
        task_id: Option<TaskId>,
        map_id: ActionMapId,
        node_id: MapNodeId,
        call_id: Option<String>,
        tool_success: bool,
        tags: Vec<String>,
    ) -> MapRuntimeEvent {
        let id = self.next_trace_event_id();
        let created_at_ms = now_ms();
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: kind.to_string(),
            task_id,
            map_id,
            node_id,
            result_id: None,
            call_id,
            action_class: None,
            tool_success: Some(tool_success),
            tags,
            artifact_refs: Vec::new(),
            created_at_ms,
        };
        self.taskspace_trace_events.push(event.clone());
        MapRuntimeEvent::TaskspaceTraceEventRecorded(MapRuntimeTraceEventRecordedEvent {
            trace_event_id: id,
            kind: event.kind.clone(),
            task_id: event.task_id.clone(),
            map_id: event.map_id.clone(),
            node_id: event.node_id.clone(),
            result_id: event.result_id.clone(),
            call_id: event.call_id.clone(),
            action_class: None,
            tool_success: event.tool_success,
            tags: event.tags.clone(),
            artifact_refs: event.artifact_refs.clone(),
            created_at_ms: event.created_at_ms,
        })
    }

    pub(crate) fn provider_request_budget_snapshot(
        &self,
    ) -> Option<ActionMapProviderRequestBudgetSnapshot> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let budget = self.active_budget.as_ref()?;
        let map_id = self.active_map_id.clone()?;
        let node_id = self.current_main_node_id.clone();
        let phase = self.next_provider_request_phase(&map_id, node_id.as_deref());
        let request_phase = Some(phase.as_str().to_string());
        let current_node = node_id.as_deref().and_then(|node_id| {
            self.maps
                .get(&map_id)
                .and_then(|map| map.nodes.get(node_id))
        });
        let node_role = current_node.map(|node| node.role.as_str().to_string());
        let provider_request_context_missing_reason =
            self.provider_request_context_missing_reason(&map_id, node_id.as_deref(), phase);
        let task_id = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        let map_requires_initialization = self.maps.get(&map_id).is_none();
        let node_request_count = node_id
            .as_ref()
            .and_then(|node_id| {
                self.budget_counters
                    .model_request_count_by_node
                    .get(node_id)
                    .copied()
            })
            .unwrap_or(0);
        Some(ActionMapProviderRequestBudgetSnapshot {
            task_id,
            map_id,
            node_id,
            node_role,
            route_mode: Some(budget.route_mode.as_str().to_string()),
            profile_name: Some(budget.profile_name.clone()),
            request_phase,
            provider_request_context_missing_reason,
            map_requires_initialization,
            request_count: self.budget_counters.rollout_model_request_count,
            max_requests: budget.max_rollout_model_requests,
            node_request_count,
            max_model_requests_per_node: budget.max_model_requests_per_node,
            budget_state: self.budget_state.as_str().to_string(),
        })
    }

    pub(crate) fn next_provider_request_phase(
        &self,
        map_id: &str,
        node_id: Option<&str>,
    ) -> TaskSpaceProviderRequestPhase {
        let Some(node) = node_id
            .and_then(|node_id| self.maps.get(map_id).and_then(|map| map.nodes.get(node_id)))
        else {
            return TaskSpaceProviderRequestPhase::Unknown;
        };
        match node.role {
            NodeRole::Work => TaskSpaceProviderRequestPhase::ModelSampling,
            NodeRole::TaskRoot | NodeRole::Finish => TaskSpaceProviderRequestPhase::Unknown,
        }
    }

    fn provider_request_context_missing_reason(
        &self,
        map_id: &str,
        node_id: Option<&str>,
        phase: TaskSpaceProviderRequestPhase,
    ) -> Option<String> {
        if node_id.is_none() {
            return Some("current_main_node_missing".to_string());
        }
        if phase == TaskSpaceProviderRequestPhase::Unknown {
            return Some("current_main_node_not_found".to_string());
        }
        if !self.maps.contains_key(map_id) {
            return Some("active_map_missing".to_string());
        }
        None
    }

    pub(crate) fn record_provider_request_budget_events(
        &mut self,
        snapshot: &ActionMapProviderRequestBudgetSnapshot,
        inputs: Vec<ActionMapProviderRequestBudgetEventInput>,
    ) -> Option<Vec<MapRuntimeEvent>> {
        if self.mode != MapRuntimeMode::Experiment || inputs.is_empty() {
            return None;
        }
        let mut events = Vec::new();
        let mut node_request_counts = self.budget_counters.model_request_count_by_node.clone();
        for input in inputs {
            self.provider_request_count =
                self.provider_request_count.max(input.request_count_after);
            self.budget_counters.rollout_model_request_count = self
                .budget_counters
                .rollout_model_request_count
                .max(input.request_count_after);
            let task_id = input.task_id.clone().or_else(|| snapshot.task_id.clone());
            let map_id = input
                .map_id
                .clone()
                .unwrap_or_else(|| snapshot.map_id.clone());
            let node_id = input
                .node_id
                .clone()
                .or_else(|| snapshot.node_id.clone())
                .unwrap_or_else(|| "provider-context-missing".to_string());
            let node_request_count_before = if node_id == "provider-context-missing" {
                snapshot.node_request_count
            } else {
                node_request_counts
                    .get(&node_id)
                    .copied()
                    .unwrap_or(snapshot.node_request_count)
            };
            if input.status == "started" && node_id != "provider-context-missing" {
                let next_node_count = node_request_count_before + 1;
                node_request_counts.insert(node_id.clone(), next_node_count);
                self.budget_counters
                    .model_request_count_by_node
                    .insert(node_id.clone(), next_node_count);
            }
            if let Some(budget) = self.active_budget.as_ref() {
                self.budget_state = budget_state_for_counter(
                    self.budget_counters.rollout_model_request_count,
                    budget.max_rollout_model_requests,
                );
            }
            let provider_context_missing_reason =
                if input.node_id.is_none() && snapshot.node_id.is_none() {
                    snapshot
                        .provider_request_context_missing_reason
                        .as_deref()
                        .unwrap_or("provider_context_missing")
                } else {
                    ""
                };
            let request_phase = input
                .request_phase
                .clone()
                .filter(|phase| !phase.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string());
            let reason_fields = self.provider_request_reason_fields(
                snapshot,
                &input,
                map_id.as_str(),
                node_id.as_str(),
                request_phase.as_str(),
            );
            let effective_node_request_count = if node_id == "provider-context-missing" {
                snapshot.node_request_count
            } else {
                node_request_counts
                    .get(&node_id)
                    .copied()
                    .unwrap_or(snapshot.node_request_count)
            };
            let id = self.next_trace_event_id();
            let created_at_ms = now_ms();
            let tool_success = Some(!matches!(
                input.status.as_str(),
                "blocked" | "failed" | "response_failed" | "cancelled"
            ));
            let exact_payload_scan = input.exact_payload_scan.clone();
            let mut tags = vec![
                "schema:taskspace-provider-request-budget-event-v1".to_string(),
                format!("transport:{}", input.transport),
                format!("status:{}", input.status),
                format!("request_count_before:{}", input.request_count_before),
                format!("request_count_after:{}", input.request_count_after),
                format!("max_requests:{}", input.max_requests),
                "active_budget_source:runtime".to_string(),
                format!(
                    "route_mode:{}",
                    snapshot.route_mode.as_deref().unwrap_or("unknown")
                ),
                format!(
                    "profile_name:{}",
                    snapshot.profile_name.as_deref().unwrap_or("unknown")
                ),
                format!("node_request_count:{effective_node_request_count}"),
                format!(
                    "max_model_requests_per_node:{}",
                    snapshot.max_model_requests_per_node
                ),
                format!("runtime_budget_state:{}", self.budget_state.as_str()),
                format!("budget_state_before:{}", input.budget_state_before),
                format!("budget_state_after:{}", input.budget_state_after),
                format!(
                    "budget_transition_reason:{}",
                    input.budget_transition_reason
                ),
                format!("started_at_ms:{}", input.started_at_ms),
                format!("request_phase:{request_phase}"),
                "schema:taskspace-provider-request-reason-v1".to_string(),
                format!(
                    "node_role:{}",
                    snapshot.node_role.as_deref().unwrap_or("unknown")
                ),
                format!("trigger_kind:{}", reason_fields.trigger_kind),
                format!(
                    "response_actionability_previous:{}",
                    reason_fields.response_actionability_previous
                ),
                format!(
                    "previous_response_recovery_action:{}",
                    self.latest_provider_response_actionability_trace(
                        map_id.as_str(),
                        node_id.as_str()
                    )
                    .map(|trace| trace.recovery_action)
                    .unwrap_or_else(|| "none".to_string())
                ),
                format!(
                    "latest_tool_result_refs:{}",
                    provider_request_reason_join(&reason_fields.latest_tool_result_refs)
                ),
                format!(
                    "model_visible_feedback_refs:{}",
                    provider_request_reason_join(&reason_fields.model_visible_feedback_refs)
                ),
                format!(
                    "adoption_blockers:{}",
                    provider_request_reason_join(&reason_fields.adoption_blockers)
                ),
                format!(
                    "projection_bundle_hash:{}",
                    reason_fields.projection_bundle_hash
                ),
                format!(
                    "request_reason_delta:{}",
                    reason_fields.request_reason_delta
                ),
                format!(
                    "repeated_same_reason_count:{}",
                    reason_fields.repeated_same_reason_count
                ),
                format!("reason_confidence:{}", reason_fields.reason_confidence),
                "producer:provider_lifecycle".to_string(),
                format!("logical_request_id:{}", input.logical_request_id),
                format!("attempt_seq:{}", input.attempt_seq),
            ];
            if let Some(previous_trace_id) = reason_fields.previous_response_trace_event_id {
                tags.push(format!(
                    "previous_response_trace_event_id:{previous_trace_id}"
                ));
            }
            if request_phase == "unknown" {
                tags.push("request_phase_missing_reason:provider_context_missing".to_string());
            }
            if !provider_context_missing_reason.is_empty() {
                tags.push(format!(
                    "provider_request_context_missing_reason:{provider_context_missing_reason}"
                ));
            }
            if let Some(parent_request_id) = input.parent_request_id.as_ref() {
                tags.push(format!("parent_request_id:{parent_request_id}"));
            }
            if let Some(completed_at_ms) = input.completed_at_ms {
                tags.push(format!("completed_at_ms:{completed_at_ms}"));
            }
            if let Some(latency_ms) = input.latency_ms {
                tags.push(format!("latency_ms:{latency_ms}"));
                tags.push(format!("model_request_duration_ms:{latency_ms}"));
            }
            if let Some(input_tokens) = input.input_tokens {
                tags.push(format!("input_tokens:{input_tokens}"));
            }
            if let Some(cached_input_tokens) = input.cached_input_tokens {
                tags.push(format!("cached_input_tokens:{cached_input_tokens}"));
            }
            if let Some(output_tokens) = input.output_tokens {
                tags.push(format!("output_tokens:{output_tokens}"));
            }
            if let Some(reasoning_output_tokens) = input.reasoning_output_tokens {
                tags.push(format!("reasoning_output_tokens:{reasoning_output_tokens}"));
            }
            if let Some(total_tokens) = input.total_tokens {
                tags.push(format!("total_tokens:{total_tokens}"));
            }
            if let Some(provider_payload_sha256) = input.provider_payload_sha256 {
                tags.push(format!("provider_payload_sha256:{provider_payload_sha256}"));
            }
            if let Some(provider_payload_bytes) = input.provider_payload_bytes {
                tags.push(format!("provider_payload_bytes:{provider_payload_bytes}"));
            }
            if let Some(provider_wire_api) = input.provider_wire_api {
                tags.push(format!("provider_wire_api:{provider_wire_api}"));
            }
            if let Some(tools_count) = input.tools_count {
                tags.push(format!("tools_count:{tools_count}"));
            }
            if let Some(tools_present) = input.tools_present {
                tags.push(format!("tools_present:{tools_present}"));
            }
            if let Some(request_shape_classifier) = input.request_shape_classifier {
                tags.push(format!(
                    "request_shape_classifier:{request_shape_classifier}"
                ));
            }
            if let Some(messages_hash) = input.messages_hash {
                tags.push(format!("messages_hash:{messages_hash}"));
            }
            if let Some(stable_prefix_hash) = input.stable_prefix_hash {
                tags.push(format!("stable_prefix_hash:{stable_prefix_hash}"));
            }
            if let Some(dynamic_suffix_hash) = input.dynamic_suffix_hash {
                tags.push(format!("dynamic_suffix_hash:{dynamic_suffix_hash}"));
            }
            if let Some(exact_payload_scan_passed) = input.exact_payload_scan_passed {
                tags.push(format!(
                    "exact_payload_scan_passed:{exact_payload_scan_passed}"
                ));
            }
            if let Some(active_projection_present) = input.active_projection_present {
                tags.push(format!(
                    "active_projection_present:{active_projection_present}"
                ));
            }
            if let Some(active_projection_count) = input.active_projection_count {
                tags.push(format!("active_projection_count:{active_projection_count}"));
            }
            if let Some(large_raw_output_tokens) = input.large_raw_output_tokens {
                tags.push(format!("large_raw_output_tokens:{large_raw_output_tokens}"));
            }
            if let Some(protected_items_present) = input.protected_items_present {
                tags.push(format!("protected_items_present:{protected_items_present}"));
            }
            if let Some(replacement_confirmed) = input.replacement_confirmed {
                tags.push(format!("replacement_confirmed:{replacement_confirmed}"));
            }
            if let Some(scan) = exact_payload_scan.as_ref() {
                tags.push(format!(
                    "exact_payload_scan_event_id:{}",
                    scan.scan_event_id
                ));
            }
            let event = TaskSpaceTraceEvent {
                id: id.clone(),
                kind: "provider_request_budget".to_string(),
                task_id,
                map_id,
                node_id,
                result_id: None,
                call_id: Some(input.request_id),
                action_class: None,
                tool_success,
                tags,
                artifact_refs: Vec::new(),
                created_at_ms,
            };
            self.taskspace_trace_events.push(event.clone());
            events.push(MapRuntimeEvent::TaskspaceTraceEventRecorded(
                MapRuntimeTraceEventRecordedEvent {
                    trace_event_id: id.clone(),
                    kind: event.kind.clone(),
                    task_id: event.task_id.clone(),
                    map_id: event.map_id.clone(),
                    node_id: event.node_id.clone(),
                    result_id: event.result_id.clone(),
                    call_id: event.call_id.clone(),
                    action_class: None,
                    tool_success: event.tool_success,
                    tags: event.tags.clone(),
                    artifact_refs: event.artifact_refs.clone(),
                    created_at_ms: event.created_at_ms,
                },
            ));
            if let Some(scan) = exact_payload_scan {
                let scan_trace_id = self.next_trace_event_id();
                let checked_byte_ranges = scan
                    .checked_byte_ranges
                    .iter()
                    .map(|(start, end)| format!("{start}-{end}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let provider_payload_bytes = scan
                    .checked_byte_ranges
                    .iter()
                    .map(|(_, end)| *end)
                    .max()
                    .unwrap_or(0);
                let negative_checks_performed = scan.negative_checks_performed.join(",");
                let failure_reasons = if scan.failure_reasons.is_empty() {
                    "none".to_string()
                } else {
                    scan.failure_reasons.join(",")
                };
                let runtime_boundary_forbidden_markers =
                    if scan.runtime_boundary_forbidden_markers.is_empty() {
                        "none".to_string()
                    } else {
                        scan.runtime_boundary_forbidden_markers.join("|")
                    };
                let scan_tags = vec![
                    "schema:taskspace-exact-payload-scan-event-v1".to_string(),
                    "producer:provider_payload_scanner".to_string(),
                    format!("scan_event_id:{}", scan.scan_event_id),
                    format!("provider_request_budget_trace_event_id:{id}"),
                    format!("provider_payload_sha256:{}", scan.provider_payload_sha256),
                    format!("provider_payload_bytes:{provider_payload_bytes}"),
                    format!("scanner_version:{}", scan.scanner_version),
                    format!("matcher_version:{}", scan.matcher_version),
                    format!("checked_byte_ranges:{checked_byte_ranges}"),
                    format!("negative_checks_performed:{negative_checks_performed}"),
                    format!("projection_required:{}", scan.projection_required),
                    format!(
                        "active_projection_present:{}",
                        scan.active_projection_present
                    ),
                    format!("active_projection_count:{}", scan.active_projection_count),
                    format!(
                        "projection_is_message_tail:{}",
                        scan.projection_is_message_tail
                    ),
                    format!("large_raw_output_tokens:{}", scan.large_raw_output_tokens),
                    format!(
                        "runtime_boundary_forbidden_markers:{runtime_boundary_forbidden_markers}"
                    ),
                    format!("protected_items_present:{}", scan.protected_items_present),
                    format!(
                        "projection_kind:{}",
                        scan.projection_kind.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "projection_map_id_sha256:{}",
                        scan.projection_map_id_sha256.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "projection_revision:{}",
                        scan.projection_revision
                            .map_or_else(|| "none".to_string(), |value| value.to_string())
                    ),
                    format!(
                        "projection_canonical_sha256:{}",
                        scan.projection_canonical_sha256
                            .as_deref()
                            .unwrap_or("none")
                    ),
                    format!(
                        "projection_sha256:{}",
                        scan.projection_sha256.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "projection_policy:{}",
                        scan.projection_policy.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "expected_projection_kind:{}",
                        scan.expected_projection_kind.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "expected_projection_map_id_sha256:{}",
                        scan.expected_projection_map_id_sha256
                            .as_deref()
                            .unwrap_or("none")
                    ),
                    format!(
                        "expected_projection_revision:{}",
                        scan.expected_projection_revision
                            .map_or_else(|| "none".to_string(), |value| value.to_string())
                    ),
                    format!(
                        "expected_projection_canonical_sha256:{}",
                        scan.expected_projection_canonical_sha256
                            .as_deref()
                            .unwrap_or("none")
                    ),
                    format!(
                        "expected_projection_sha256:{}",
                        scan.expected_projection_sha256.as_deref().unwrap_or("none")
                    ),
                    format!(
                        "projection_identity_confirmed:{}",
                        scan.projection_identity_confirmed
                            .map_or("unavailable", |value| if value { "true" } else { "false" })
                    ),
                    format!("replacement_confirmed:{}", scan.replacement_confirmed),
                    format!("passed:{}", scan.passed),
                    format!("failure_reasons:{failure_reasons}"),
                ];
                let scan_event = TaskSpaceTraceEvent {
                    id: scan_trace_id,
                    kind: "exact_payload_scan".to_string(),
                    task_id: event.task_id.clone(),
                    map_id: event.map_id.clone(),
                    node_id: event.node_id.clone(),
                    result_id: None,
                    call_id: Some(scan.request_id),
                    action_class: None,
                    tool_success: Some(scan.passed),
                    tags: scan_tags,
                    artifact_refs: Vec::new(),
                    created_at_ms,
                };
                self.taskspace_trace_events.push(scan_event.clone());
                events.push(map_runtime_event_from_trace_event(scan_event));
            }
            let quality_id = self.next_trace_event_id();
            let budget_action = "observe";
            let final_classification = if input.status == "blocked" {
                "blocked_input_observed"
            } else {
                "score_eligible"
            };
            let score_eligible = input.status != "blocked";
            let quality_tags = vec![
                "schema:taskspace-budget-quality-impact-v1".to_string(),
                format!("provider_request_budget_trace_event_id:{id}"),
                format!("budget_action:{budget_action}"),
                format!("provider_request_status:{}", input.status),
                format!("counter_name:provider_request_count"),
                format!("counter_value:{}", input.request_count_after),
                format!("counter_limit:{}", input.max_requests),
                "active_budget_source:runtime".to_string(),
                format!(
                    "route_mode:{}",
                    snapshot.route_mode.as_deref().unwrap_or("unknown")
                ),
                format!("budget_state_before:{}", input.budget_state_before),
                format!("budget_state_after:{}", input.budget_state_after),
                format!(
                    "budget_transition_reason:{}",
                    input.budget_transition_reason
                ),
                format!("request_phase:{request_phase}"),
                format!("logical_request_id:{}", input.logical_request_id),
                format!("attempt_seq:{}", input.attempt_seq),
                format!("score_eligible:{score_eligible}"),
                "budget_induced_validation_skip:false".to_string(),
                "manual_override_used:false".to_string(),
                "bounded_recovery_used:false".to_string(),
                format!("final_classification:{final_classification}"),
            ];
            let quality_event = TaskSpaceTraceEvent {
                id: quality_id.clone(),
                kind: "budget_quality_impact".to_string(),
                task_id: event.task_id.clone(),
                map_id: event.map_id.clone(),
                node_id: event.node_id.clone(),
                result_id: None,
                call_id: event.call_id.clone(),
                action_class: None,
                tool_success: Some(score_eligible),
                tags: quality_tags,
                artifact_refs: Vec::new(),
                created_at_ms,
            };
            self.taskspace_trace_events.push(quality_event.clone());
            events.push(MapRuntimeEvent::TaskspaceTraceEventRecorded(
                MapRuntimeTraceEventRecordedEvent {
                    trace_event_id: quality_id,
                    kind: quality_event.kind.clone(),
                    task_id: quality_event.task_id.clone(),
                    map_id: quality_event.map_id.clone(),
                    node_id: quality_event.node_id.clone(),
                    result_id: quality_event.result_id.clone(),
                    call_id: quality_event.call_id.clone(),
                    action_class: None,
                    tool_success: quality_event.tool_success,
                    tags: quality_event.tags.clone(),
                    artifact_refs: quality_event.artifact_refs.clone(),
                    created_at_ms: quality_event.created_at_ms,
                },
            ));
        }
        Some(events)
    }

    fn provider_request_reason_fields(
        &self,
        snapshot: &ActionMapProviderRequestBudgetSnapshot,
        input: &ActionMapProviderRequestBudgetEventInput,
        map_id: &str,
        node_id: &str,
        request_phase: &str,
    ) -> ProviderRequestReasonFields {
        let previous_actionability =
            self.latest_provider_response_actionability_trace(map_id, node_id);
        let latest_tool_result_refs = self.latest_main_tool_result_refs(map_id, node_id, 5);
        let adoption_blockers = provider_request_adoption_blockers(snapshot, request_phase);
        let projection_bundle_hash = input
            .dynamic_suffix_hash
            .as_deref()
            .or(input.provider_payload_sha256.as_deref())
            .unwrap_or("unavailable")
            .to_string();
        let response_actionability_previous = previous_actionability
            .as_ref()
            .map(|trace| trace.response_actionability.clone())
            .unwrap_or_else(|| "none".to_string());
        let trigger_kind = provider_request_trigger_kind(
            snapshot,
            input,
            request_phase,
            response_actionability_previous.as_str(),
            &adoption_blockers,
        );
        let mut model_visible_feedback_refs = latest_tool_result_refs.clone();
        if let Some(trace) = previous_actionability.as_ref() {
            model_visible_feedback_refs.push(trace.trace_event_id.clone());
        }
        let (request_reason_delta, repeated_same_reason_count) = self
            .provider_request_reason_delta_and_count(
                map_id,
                node_id,
                input.status.as_str(),
                input.logical_request_id.as_str(),
                ProviderRequestReasonFingerprint {
                    trigger_kind: trigger_kind.as_str(),
                    adoption_blockers: &adoption_blockers,
                    latest_tool_result_refs: &latest_tool_result_refs,
                    projection_bundle_hash: projection_bundle_hash.as_str(),
                },
            );
        let reason_confidence = provider_request_reason_confidence(
            request_phase,
            trigger_kind.as_str(),
            &previous_actionability,
            &adoption_blockers,
        );
        ProviderRequestReasonFields {
            trigger_kind,
            response_actionability_previous,
            previous_response_trace_event_id: previous_actionability
                .as_ref()
                .map(|trace| trace.trace_event_id.clone()),
            latest_tool_result_refs,
            model_visible_feedback_refs,
            adoption_blockers,
            projection_bundle_hash,
            request_reason_delta,
            repeated_same_reason_count,
            reason_confidence,
        }
    }

    fn latest_provider_response_actionability_trace(
        &self,
        map_id: &str,
        node_id: &str,
    ) -> Option<ProviderResponseActionabilityTrace> {
        self.taskspace_trace_events
            .iter()
            .rev()
            .find(|event| {
                event.kind == "provider_response_actionability"
                    && event.map_id == map_id
                    && event.node_id == node_id
            })
            .map(|event| ProviderResponseActionabilityTrace {
                trace_event_id: event.id.clone(),
                response_actionability: trace_tag_value(&event.tags, "response_actionability")
                    .unwrap_or("unknown")
                    .to_string(),
                recovery_action: trace_tag_value(&event.tags, "recovery_action")
                    .unwrap_or("unknown")
                    .to_string(),
            })
    }

    fn latest_main_tool_result_refs(
        &self,
        map_id: &str,
        node_id: &str,
        limit: usize,
    ) -> Vec<String> {
        let mut refs = self
            .taskspace_trace_events
            .iter()
            .rev()
            .filter(|event| {
                event.kind == "main_tool_result"
                    && event.map_id == map_id
                    && event.node_id == node_id
            })
            .filter_map(|event| event.result_id.clone())
            .take(limit)
            .collect::<Vec<_>>();
        refs.reverse();
        refs
    }

    fn provider_request_reason_delta_and_count(
        &self,
        map_id: &str,
        node_id: &str,
        status: &str,
        logical_request_id: &str,
        current: ProviderRequestReasonFingerprint<'_>,
    ) -> (String, usize) {
        let Some(previous) = self.taskspace_trace_events.iter().rev().find(|event| {
            event.kind == "provider_request_budget"
                && event.map_id == map_id
                && event.node_id == node_id
                && trace_tag_value(&event.tags, "status") == Some(status)
                && trace_tag_value(&event.tags, "logical_request_id") != Some(logical_request_id)
        }) else {
            return ("initial_request".to_string(), 0);
        };
        let previous_latest_tool_result_refs =
            trace_tag_value(&previous.tags, "latest_tool_result_refs").unwrap_or("none");
        let current_latest_tool_result_refs =
            provider_request_reason_join(current.latest_tool_result_refs);
        if previous_latest_tool_result_refs != current_latest_tool_result_refs {
            return ("new_tool_result_refs".to_string(), 0);
        }
        let previous_projection_bundle_hash =
            trace_tag_value(&previous.tags, "projection_bundle_hash").unwrap_or("unavailable");
        if current.projection_bundle_hash != "unavailable"
            && previous_projection_bundle_hash != current.projection_bundle_hash
        {
            return ("changed_projection".to_string(), 0);
        }
        let previous_adoption_blockers =
            trace_tag_value(&previous.tags, "adoption_blockers").unwrap_or("none");
        let current_adoption_blockers = provider_request_reason_join(current.adoption_blockers);
        if previous_adoption_blockers != current_adoption_blockers {
            return ("changed_adoption_blockers".to_string(), 0);
        }
        let previous_trigger_kind =
            trace_tag_value(&previous.tags, "trigger_kind").unwrap_or("unknown");
        if previous_trigger_kind != current.trigger_kind {
            return ("changed_trigger".to_string(), 0);
        }
        let repeated_same_reason_count =
            trace_tag_usize(&previous.tags, "repeated_same_reason_count").unwrap_or(0) + 1;
        ("none".to_string(), repeated_same_reason_count)
    }

    pub(crate) fn record_provider_response_actionability(
        &mut self,
        snapshot: &ActionMapProviderRequestBudgetSnapshot,
        input: ActionMapProviderResponseActionabilityInput,
    ) -> Option<Vec<MapRuntimeEvent>> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let node_id = snapshot
            .node_id
            .clone()
            .unwrap_or_else(|| "provider-context-missing".to_string());
        let request_phase = snapshot
            .request_phase
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let id = self.next_trace_event_id();
        let created_at_ms = now_ms();
        let mut tags = vec![
            "schema:taskspace-provider-response-actionability-v1".to_string(),
            "producer:provider_response".to_string(),
            format!("response_actionability:{}", input.response_actionability),
            format!("request_phase:{request_phase}"),
            format!("request_count:{}", snapshot.request_count),
            format!("max_requests:{}", snapshot.max_requests),
            format!("saw_actionable_output:{}", input.saw_actionable_output),
            format!(
                "assistant_message_present:{}",
                input.assistant_message_present
            ),
            format!("recovery_action:{}", input.recovery_action),
        ];
        let end_turn_tag = input
            .end_turn
            .map(|value| value.to_string())
            .unwrap_or_else(|| "missing".to_string());
        tags.push(format!("end_turn:{end_turn_tag}"));
        if let Some(reason) = snapshot.provider_request_context_missing_reason.as_deref() {
            tags.push(format!("provider_request_context_missing_reason:{reason}"));
        }
        if let Some(preview) = input.last_agent_message_preview
            && !preview.trim().is_empty()
        {
            tags.push(format!(
                "last_agent_message_preview:{}",
                sanitize_provider_response_trace_tag_value(&preview)
            ));
        }
        let tool_success = Some(matches!(
            input.response_actionability.as_str(),
            "actionable" | "turn_complete"
        ));
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: "provider_response_actionability".to_string(),
            task_id: snapshot.task_id.clone(),
            map_id: snapshot.map_id.clone(),
            node_id,
            result_id: None,
            call_id: None,
            action_class: None,
            tool_success,
            tags,
            artifact_refs: Vec::new(),
            created_at_ms,
        };
        self.taskspace_trace_events.push(event.clone());
        Some(vec![MapRuntimeEvent::TaskspaceTraceEventRecorded(
            MapRuntimeTraceEventRecordedEvent {
                trace_event_id: id,
                kind: event.kind,
                task_id: event.task_id,
                map_id: event.map_id,
                node_id: event.node_id,
                result_id: event.result_id,
                call_id: event.call_id,
                action_class: None,
                tool_success: event.tool_success,
                tags: event.tags,
                artifact_refs: event.artifact_refs,
                created_at_ms: event.created_at_ms,
            },
        )])
    }

    pub(crate) fn prepare_child_tool_call(
        &mut self,
        child_thread_id: ThreadId,
        descriptor: impl Into<ToolActionDescriptor>,
    ) -> Result<Vec<MapRuntimeEvent>, ActionMapGateError> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(Vec::new());
        }

        let descriptor = descriptor.into();
        let (map_id, lease_id, node_id) =
            self.find_lease_by_thread(child_thread_id).ok_or_else(|| {
                ActionMapGateError::from(
                    "TaskSpace blocked this subagent tool call because the subagent has no active task node lease. hard_state: subagent_node_lease_missing.",
                )
            })?;
        if descriptor.action_class != ActionClass::Control {
            self.validate_maintenance_barrier_for_map_node(&map_id, &node_id)?;
        }
        let map = self.maps.get(&map_id).ok_or_else(|| {
            ActionMapGateError::from(format!("TaskSpace child task path `{map_id}` is missing."))
        })?;
        if map.is_complete() {
            return Err(ActionMapGateError::from(format!(
                "TaskSpace child task path `{map_id}` is not active."
            )));
        }
        let node = map.nodes.get(&node_id).ok_or_else(|| {
            ActionMapGateError::from(format!("TaskSpace child node `{node_id}` is missing."))
        })?;
        let lease = map.leases.get(&lease_id).ok_or_else(|| {
            ActionMapGateError::from(format!("TaskSpace child lease `{lease_id}` is missing."))
        })?;
        if lease.holder != LeaseHolder::SubAgent
            || lease.agent_thread_id != Some(child_thread_id)
            || node.active_lease.as_deref() != Some(lease_id.as_str())
            || node.status != NodeStatus::Running
        {
            return Err(ActionMapGateError::from(format!(
                "TaskSpace child node `{node_id}` is no longer held by subagent lease `{lease_id}`."
            )));
        }

        if descriptor.action_class != ActionClass::Control
            && let Some(call_id) = descriptor.call_id.as_deref()
        {
            self.reserve_child_tool_call(
                child_thread_id,
                call_id,
                ChildToolReservation {
                    child_thread_id,
                    map_id,
                    node_id,
                    lease_id,
                    action_class: descriptor.action_class,
                },
            );
        }
        Ok(Vec::new())
    }

    pub(crate) fn record_child_tool_result_with_class(
        &mut self,
        child_thread_id: ThreadId,
        call_id: &str,
        _tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        body: String,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }

        let reservation = self.release_child_tool_reservation(child_thread_id, call_id);
        let (map_id, node_id, lease_id, recorded_action_class) = if let Some(reservation) =
            reservation
        {
            self.validate_child_tool_reservation(child_thread_id, &reservation)?;
            if let Some(observed_action_class) = action_class
                && observed_action_class != reservation.action_class
            {
                return Err(format!(
                    "TaskSpace subagent tool result `{call_id}` action class changed from {} to {} while in flight.",
                    reservation.action_class.as_str(),
                    observed_action_class.as_str()
                ));
            }
            (
                reservation.map_id,
                reservation.node_id,
                reservation.lease_id,
                Some(reservation.action_class),
            )
        } else {
            let (map_id, lease_id, node_id) = match self.find_lease_by_thread(child_thread_id) {
                Some(target) => target,
                None => return Ok(None),
            };
            (map_id, node_id, lease_id, action_class)
        };
        let result_id = self.next_result_id();
        let artifact_refs = tool_result_artifact_refs(recorded_action_class, success, &body);
        let map = self
            .maps
            .get_mut(&map_id)
            .ok_or_else(|| format!("TaskSpace child task path `{map_id}` is missing."))?;
        let result = NodeResult {
            id: result_id.clone(),
            assignment_id: lease_id.clone(),
            map_id: map_id.clone(),
            node_id: node_id.clone(),
            kind: NodeResultKind::MainToolCall,
            action_class: recorded_action_class,
            tool_success: Some(success),
            source_event_ref: child_tool_source_event_ref(child_thread_id, call_id),
            artifact_refs,
            source_thread_id: child_thread_id,
            created_at_ms: now_ms(),
        };
        map.results.insert(result_id.clone(), result);
        map.nodes
            .get_mut(&node_id)
            .ok_or_else(|| format!("TaskSpace child node `{node_id}` is missing."))?
            .result_context
            .push(NodeResultRef {
                id: result_id.clone(),
                kind: NodeResultKind::MainToolCall,
            });
        let events = vec![MapRuntimeEvent::NodeResultRecorded(
            MapRuntimeNodeResultRecordedEvent {
                map_id: map_id.clone(),
                node_id,
                lease_id,
                result_id: result_id.clone(),
                kind: NodeResultKind::MainToolCall.as_str().to_string(),
                action_class: recorded_action_class.map(|class| class.as_str().to_string()),
                source_thread_id: child_thread_id,
            },
        )];
        Ok(Some((result_id, events)))
    }

    pub(crate) fn prepare_child_spawn(&self, child_thread_id: ThreadId) -> Result<(), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(());
        }
        if self.find_lease_by_thread(child_thread_id).is_none() {
            return Err(
                "TaskSpace blocked this subagent spawn because the subagent has no active task node lease in the parent task map."
                    .to_string(),
            );
        }
        Err(
            "TaskSpace blocked nested spawn_agent from a node-bound subagent. hard_state: nested_spawn_not_supported."
                .to_string(),
        )
    }

    pub(crate) fn initialize_map_for_main(
        &mut self,
        owner_session_id: ThreadId,
        input: ActionMapInitializeInput,
    ) -> Result<(ActionMapInitializeOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.initialize_map_for_main_inner(owner_session_id, input)?;
        *self = candidate;
        Ok(outcome)
    }

    fn initialize_map_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        input: ActionMapInitializeInput,
    ) -> Result<(ActionMapInitializeOutcome, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        if self.routing_required || !self.bootstrap_required {
            return Err(
                "TaskSpace rooted map initialization requires an empty bootstrap state. hard_state: map_already_initialized."
                    .to_string(),
            );
        }
        let task_id = self.active_task_id.clone().ok_or_else(|| {
            "TaskSpace has no active task. hard_state: no_active_task_path.".to_string()
        })?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace has no active map. hard_state: no_active_task_path.".to_string()
        })?;
        self.tasks
            .get(&task_id)
            .ok_or_else(|| format!("TaskSpace active task `{task_id}` is missing."))?;
        if self.tasks[&task_id].owner_session_id != Some(owner_session_id)
            || self.maps.contains_key(&map_id)
        {
            return Err(
                "TaskSpace rooted map initialization requires the reserved empty map owned by this session. hard_state: map_owner_mismatch."
                    .to_string(),
            );
        }

        let root_id = require_nonempty_owned("root.node_id", input.root.id)?;
        let root_goal = require_nonempty_owned("root.goal", input.root.goal)?;
        let finish_id = require_nonempty_owned("finish.node_id", input.finish.id)?;
        let current_node_id =
            require_nonempty_owned("current_work_node.node_id", input.current_work_node.id)?;
        let current_node_goal =
            require_nonempty_owned("current_work_node.goal", input.current_work_node.goal)?;
        let mut work_nodes = BTreeMap::new();
        let mut input_order = Vec::with_capacity(input.work_nodes.len() + 3);
        input_order.push(root_id.clone());
        work_nodes.insert(current_node_id.clone(), current_node_goal);
        input_order.push(current_node_id.clone());
        for node in input.work_nodes {
            let id = require_nonempty_owned("work_node.node_id", node.id)?;
            let goal = require_nonempty_owned("work_node.goal", node.goal)?;
            if work_nodes.insert(id.clone(), goal).is_some() {
                return Err(format!("TaskSpace work node `{id}` is duplicated."));
            }
            input_order.push(id);
        }
        input_order.push(finish_id.clone());
        let edges = input
            .edges
            .into_iter()
            .map(|edge| {
                Ok(MapEdge::new(
                    require_nonempty_owned("edge.from", edge.from)?,
                    require_nonempty_owned("edge.to", edge.to)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let initialized = initialize(InitializeMap {
            map_id: map_id.clone(),
            root_node_id: root_id,
            root_goal,
            source_refs: input.source_event_ids,
            finish_node_id: finish_id,
            work_nodes,
            edges,
        })
        .map_err(rooted_rejection_message)?;
        let bound = transition_node(
            &initialized.map,
            initialized.map.revision,
            current_node_id.clone(),
            NodeTransition::Bind,
        )
        .map_err(|mut rejection| {
            rejection.current_revision = 0;
            tracing::warn!(
                target: "codex_core::taskspace",
                requested_work_node_id = current_node_id,
                current_revision = 0,
                state_commit = false,
                violation_count = rejection.violations.len(),
                "taskspace.map_initial_binding_rejected"
            );
            rooted_rejection_message(rejection)
        })?;
        let graph_events = vec![initialized.events, bound.events];
        let lease_id = self.next_lease_id();
        let mut map =
            ActionMapInstance::from_graph(bound.map, graph_events.clone(), Some(owner_session_id));
        map.task_id = Some(task_id.clone());
        map.leases.insert(
            lease_id.clone(),
            AssignmentLease {
                id: lease_id.clone(),
                map_id: map_id.clone(),
                node_id: current_node_id.clone(),
                holder: LeaseHolder::Main,
                previous_node_status: NodeStatus::Ready,
                agent_thread_id: Some(owner_session_id),
                agent_path: None,
            },
        );
        map.nodes
            .get_mut(&current_node_id)
            .expect("rooted transaction bound an existing work node")
            .active_lease = Some(lease_id.clone());
        let edge_count = map.edges.len();
        let node_count = map.nodes.len();
        self.maps.insert(map_id.clone(), map);
        self.current_main_node_id = Some(current_node_id.clone());
        self.current_main_lease_id = Some(lease_id.clone());
        self.mark_routing_complete();

        let graph_revision_batches = graph_events
            .iter()
            .map(|batch| graph_revision_committed_record(batch, "initialize_map"))
            .collect::<Vec<_>>();
        let committed_revision = graph_revision_batches
            .last()
            .map_or(0, |batch| batch.revision);
        let mut events = graph_revision_batches
            .iter()
            .cloned()
            .map(MapRuntimeEvent::GraphRevisionCommitted)
            .collect::<Vec<_>>();
        events.push(MapRuntimeEvent::LeaseCreated(MapRuntimeLeaseCreatedEvent {
            map_id: map_id.clone(),
            node_id: current_node_id.clone(),
            lease_id,
            holder: LeaseHolder::Main.as_str().to_string(),
        }));
        events.push(self.record_runtime_budget_trace_event(
            "agent_map_initialized",
            Some(task_id.clone()),
            map_id.clone(),
            current_node_id.clone(),
            None,
            true,
            vec![
                "schema:taskspace-agent-map-initialized-v1".to_string(),
                "producer:agent_taskspace_control".to_string(),
                "action:initialize_map".to_string(),
                format!("node_count:{node_count}"),
                format!("edge_count:{edge_count}"),
                "semantic_source:agent".to_string(),
                "runtime_inferred_semantics:false".to_string(),
            ],
        ));
        Ok((
            ActionMapInitializeOutcome {
                task_id,
                map_id: map_id.clone(),
                node_ids: input_order,
                current_node_id,
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision,
                    graph_revision_batches,
                    node_detail_events: Vec::new(),
                },
            },
            events,
        ))
    }

    pub(crate) fn mutate_graph_for_main(
        &mut self,
        owner_session_id: ThreadId,
        input: ActionMapGraphMutationInput,
    ) -> Result<(ActionMapGraphMutationOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.mutate_graph_for_main_inner(owner_session_id, input)?;
        *self = candidate;
        Ok(outcome)
    }

    fn mutate_graph_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        input: ActionMapGraphMutationInput,
    ) -> Result<(ActionMapGraphMutationOutcome, Vec<MapRuntimeEvent>), String> {
        self.validate_routing_complete()?;
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| "TaskSpace has no active rooted map.".to_string())?;
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace rooted map `{map_id}` is missing."))?;
        if map.owner_session_id != Some(owner_session_id) {
            return Err("TaskSpace rooted map owner mismatch.".to_string());
        }
        let mut add_nodes = BTreeMap::new();
        for node in input.add_nodes {
            let id = require_nonempty_owned("add_node.node_id", node.id)?;
            let goal = require_nonempty_owned("add_node.goal", node.goal)?;
            if add_nodes.insert(id.clone(), goal).is_some() {
                return Err(format!("TaskSpace graph mutation repeats node `{id}`."));
            }
        }
        let convert_edges = |edges: Vec<ActionMapEdgeInput>| {
            edges
                .into_iter()
                .map(|edge| {
                    Ok(MapEdge::new(
                        require_nonempty_owned("edge.from", edge.from)?,
                        require_nonempty_owned("edge.to", edge.to)?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()
        };
        let committed = mutate_graph(
            map,
            GraphMutation {
                expected_revision: input.expected_revision,
                add_nodes,
                add_edges: convert_edges(input.add_edges)?,
                remove_edges: convert_edges(input.remove_edges)?,
            },
        )
        .map_err(rooted_rejection_message)?;
        let revision = committed.map.revision;
        let graph_batch = graph_revision_committed_record(&committed.events, "mutate_graph");
        let graph_event = MapRuntimeEvent::GraphRevisionCommitted(graph_batch.clone());
        self.maps
            .get_mut(&map_id)
            .expect("validated rooted map remains present")
            .commit_graph(committed.map, committed.events);
        let event = self.record_runtime_budget_trace_event(
            "graph_mutation_committed",
            self.active_task_id.clone(),
            map_id.clone(),
            self.current_main_node_id
                .clone()
                .unwrap_or_else(|| "map".to_string()),
            None,
            true,
            vec![
                "schema:taskspace-rooted-graph-event-v1".to_string(),
                "operation:mutate_graph".to_string(),
                format!("revision:{revision}"),
                "state_commit:true".to_string(),
                "runtime_inferred_semantics:false".to_string(),
            ],
        );
        Ok((
            ActionMapGraphMutationOutcome {
                map_id: map_id.clone(),
                revision,
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision: revision,
                    graph_revision_batches: vec![graph_batch],
                    node_detail_events: Vec::new(),
                },
            },
            vec![graph_event, event],
        ))
    }

    pub(crate) fn transition_node_for_main(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        node_id: String,
        transition: NodeTransition,
        source_event_ref: String,
    ) -> Result<(ActionMapTransitionOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.transition_node_for_main_inner(
            owner_session_id,
            expected_revision,
            node_id,
            transition,
            source_event_ref,
        )?;
        *self = candidate;
        Ok(outcome)
    }

    fn transition_node_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        node_id: String,
        transition: NodeTransition,
        source_event_ref: String,
    ) -> Result<(ActionMapTransitionOutcome, Vec<MapRuntimeEvent>), String> {
        self.validate_routing_complete()?;
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| "TaskSpace has no active rooted map.".to_string())?;
        {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace rooted map `{map_id}` is missing."))?;
            if map.owner_session_id != Some(owner_session_id) {
                return Err("TaskSpace rooted map owner mismatch.".to_string());
            }
            map.nodes
                .get(&node_id)
                .ok_or_else(|| format!("TaskSpace node `{node_id}` is missing."))?;
        }
        if matches!(transition, NodeTransition::Complete | NodeTransition::Block) {
            self.validate_main_binding(owner_session_id)?;
            if self.current_main_node_id.as_deref() != Some(node_id.as_str()) {
                return Err(format!(
                    "TaskSpace node `{node_id}` is not the current binding."
                ));
            }
            self.validate_no_main_tool_reservations_for_node(
                &map_id,
                &node_id,
                transition.operation_name(),
            )?;
        }
        let committed = {
            let map = self
                .maps
                .get(&map_id)
                .expect("rooted map was validated before transition");
            transition_node(map, expected_revision, node_id.clone(), transition)
                .map_err(rooted_rejection_message)?
        };
        let revision = committed.map.revision;
        let status = committed
            .map
            .nodes
            .get(&node_id)
            .expect("transition target remains present")
            .status;
        let graph_batch =
            graph_revision_committed_record(&committed.events, transition.operation_name());
        let mut events = vec![MapRuntimeEvent::GraphRevisionCommitted(graph_batch.clone())];
        let lease_id = match transition {
            NodeTransition::Bind => Some(self.next_lease_id()),
            NodeTransition::Complete | NodeTransition::Block => Some(
                self.current_main_lease_id
                    .clone()
                    .ok_or_else(|| "TaskSpace current binding has no main lease.".to_string())?,
            ),
            NodeTransition::Unblock | NodeTransition::Rework => None,
            NodeTransition::ReleaseLease => self.current_main_lease_id.clone(),
        };
        let result_id = matches!(transition, NodeTransition::Complete | NodeTransition::Block)
            .then(|| self.next_result_id());
        let map = self
            .maps
            .get_mut(&map_id)
            .expect("validated rooted map remains present");
        map.commit_graph(committed.map, committed.events);
        match transition {
            NodeTransition::Bind => {
                let lease_id = lease_id.expect("bind allocated a lease");
                map.nodes
                    .get_mut(&node_id)
                    .expect("bound node exists")
                    .active_lease = Some(lease_id.clone());
                map.leases.insert(
                    lease_id.clone(),
                    AssignmentLease {
                        id: lease_id.clone(),
                        map_id: map_id.clone(),
                        node_id: node_id.clone(),
                        holder: LeaseHolder::Main,
                        previous_node_status: NodeStatus::Ready,
                        agent_thread_id: Some(owner_session_id),
                        agent_path: None,
                    },
                );
                self.current_main_node_id = Some(node_id.clone());
                self.current_main_lease_id = Some(lease_id.clone());
                events.push(MapRuntimeEvent::LeaseCreated(MapRuntimeLeaseCreatedEvent {
                    map_id: map_id.clone(),
                    node_id: node_id.clone(),
                    lease_id,
                    holder: LeaseHolder::Main.as_str().to_string(),
                }));
            }
            NodeTransition::Complete | NodeTransition::Block => {
                let lease_id = lease_id.expect("bound transition has a lease");
                map.leases.remove(&lease_id);
                let kind = if transition == NodeTransition::Complete {
                    NodeResultKind::Result
                } else {
                    NodeResultKind::Blocker
                };
                let result_id = result_id.expect("bound terminal transition records a result");
                map.results.insert(
                    result_id.clone(),
                    NodeResult {
                        id: result_id.clone(),
                        assignment_id: lease_id.clone(),
                        map_id: map_id.clone(),
                        node_id: node_id.clone(),
                        kind,
                        action_class: None,
                        tool_success: None,
                        source_event_ref,
                        artifact_refs: Vec::new(),
                        source_thread_id: owner_session_id,
                        created_at_ms: now_ms(),
                    },
                );
                let node = map
                    .nodes
                    .get_mut(&node_id)
                    .expect("transition target exists");
                node.active_lease = None;
                node.result_context.push(NodeResultRef {
                    id: result_id,
                    kind,
                });
                self.current_main_node_id = None;
                self.current_main_lease_id = None;
                events.push(MapRuntimeEvent::LeaseReleased(
                    MapRuntimeLeaseReleasedEvent {
                        map_id: map_id.clone(),
                        node_id: node_id.clone(),
                        lease_id,
                        holder: LeaseHolder::Main.as_str().to_string(),
                        reason: "node_transition_committed".to_string(),
                    },
                ));
            }
            NodeTransition::Unblock | NodeTransition::Rework => {}
            NodeTransition::ReleaseLease => {
                if let Some(lease_id) = lease_id {
                    map.leases.remove(&lease_id);
                }
                if let Some(node) = map.nodes.get_mut(&node_id) {
                    node.active_lease = None;
                }
                self.current_main_node_id = None;
                self.current_main_lease_id = None;
            }
        }
        Ok((
            ActionMapTransitionOutcome {
                map_id: map_id.clone(),
                node_id,
                revision,
                status: status.as_str().to_string(),
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision: revision,
                    graph_revision_batches: vec![graph_batch],
                    node_detail_events: Vec::new(),
                },
            },
            events,
        ))
    }

    pub(crate) fn complete_then_bind_for_main(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        current_node_id: String,
        next_node_id: String,
        source_event_ref: String,
    ) -> Result<(ActionMapCompleteHandoffOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.complete_then_bind_for_main_inner(
            owner_session_id,
            expected_revision,
            current_node_id,
            next_node_id,
            source_event_ref,
        )?;
        *self = candidate;
        Ok(outcome)
    }

    fn complete_then_bind_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        current_node_id: String,
        next_node_id: String,
        source_event_ref: String,
    ) -> Result<(ActionMapCompleteHandoffOutcome, Vec<MapRuntimeEvent>), String> {
        self.validate_routing_complete()?;
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| "TaskSpace has no active rooted map.".to_string())?;
        {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace rooted map `{map_id}` is missing."))?;
            if map.owner_session_id != Some(owner_session_id) {
                return Err("TaskSpace rooted map owner mismatch.".to_string());
            }
        }
        self.validate_main_binding(owner_session_id)?;
        if self.current_main_node_id.as_deref() != Some(current_node_id.as_str()) {
            return Err(format!(
                "TaskSpace node `{current_node_id}` is not the current binding."
            ));
        }
        self.validate_no_main_tool_reservations_for_node(
            &map_id,
            &current_node_id,
            "complete_then_continue",
        )?;
        let committed = {
            let map = self
                .maps
                .get(&map_id)
                .expect("rooted map was validated before complete handoff");
            complete_then_bind(
                map,
                expected_revision,
                current_node_id.clone(),
                next_node_id.clone(),
            )
            .map_err(rooted_rejection_message)?
        };
        let revision = committed.map.revision;
        let graph_batch =
            graph_revision_committed_record(&committed.events, "complete_then_continue");
        let old_lease_id = self
            .current_main_lease_id
            .clone()
            .ok_or_else(|| "TaskSpace current binding has no main lease.".to_string())?;
        let next_lease_id = self.next_lease_id();
        let result_id = self.next_result_id();
        let map = self
            .maps
            .get_mut(&map_id)
            .expect("validated rooted map remains present");
        map.commit_graph(committed.map, committed.events);
        map.leases.remove(&old_lease_id);
        map.results.insert(
            result_id.clone(),
            NodeResult {
                id: result_id.clone(),
                assignment_id: old_lease_id.clone(),
                map_id: map_id.clone(),
                node_id: current_node_id.clone(),
                kind: NodeResultKind::Result,
                action_class: None,
                tool_success: None,
                source_event_ref,
                artifact_refs: Vec::new(),
                source_thread_id: owner_session_id,
                created_at_ms: now_ms(),
            },
        );
        let completed_node = map
            .nodes
            .get_mut(&current_node_id)
            .expect("completed handoff node exists");
        completed_node.active_lease = None;
        completed_node.result_context.push(NodeResultRef {
            id: result_id,
            kind: NodeResultKind::Result,
        });
        map.nodes
            .get_mut(&next_node_id)
            .expect("next handoff node exists")
            .active_lease = Some(next_lease_id.clone());
        map.leases.insert(
            next_lease_id.clone(),
            AssignmentLease {
                id: next_lease_id.clone(),
                map_id: map_id.clone(),
                node_id: next_node_id.clone(),
                holder: LeaseHolder::Main,
                previous_node_status: NodeStatus::Ready,
                agent_thread_id: Some(owner_session_id),
                agent_path: None,
            },
        );
        self.current_main_node_id = Some(next_node_id.clone());
        self.current_main_lease_id = Some(next_lease_id.clone());
        let events = vec![
            MapRuntimeEvent::GraphRevisionCommitted(graph_batch.clone()),
            MapRuntimeEvent::LeaseReleased(MapRuntimeLeaseReleasedEvent {
                map_id: map_id.clone(),
                node_id: current_node_id.clone(),
                lease_id: old_lease_id,
                holder: LeaseHolder::Main.as_str().to_string(),
                reason: "complete_then_continue_committed".to_string(),
            }),
            MapRuntimeEvent::LeaseCreated(MapRuntimeLeaseCreatedEvent {
                map_id: map_id.clone(),
                node_id: next_node_id.clone(),
                lease_id: next_lease_id,
                holder: LeaseHolder::Main.as_str().to_string(),
            }),
        ];
        Ok((
            ActionMapCompleteHandoffOutcome {
                map_id: map_id.clone(),
                current_node_id,
                next_node_id,
                revision,
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision: revision,
                    graph_revision_batches: vec![graph_batch],
                    node_detail_events: Vec::new(),
                },
            },
            events,
        ))
    }

    pub(crate) fn complete_active_work_then_end_for_main(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        current_node_id: String,
        final_summary: String,
        source_event_ref: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.complete_active_work_then_end_for_main_inner(
            owner_session_id,
            expected_revision,
            current_node_id,
            final_summary,
            source_event_ref,
        )?;
        *self = candidate;
        Ok(outcome)
    }

    fn complete_active_work_then_end_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        current_node_id: String,
        final_summary: String,
        source_event_ref: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        self.validate_routing_complete()?;
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| "TaskSpace has no active rooted map.".to_string())?;
        {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace rooted map `{map_id}` is missing."))?;
            if map.owner_session_id != Some(owner_session_id) {
                return Err("TaskSpace rooted map owner mismatch.".to_string());
            }
        }
        self.validate_main_binding(owner_session_id)?;
        if self.current_main_node_id.as_deref() != Some(current_node_id.as_str()) {
            return Err(format!(
                "TaskSpace node `{current_node_id}` is not the current binding."
            ));
        }
        self.validate_no_main_tool_reservations_for_node(
            &map_id,
            &current_node_id,
            "complete_active_work_then_end",
        )?;
        let committed = {
            let map = self
                .maps
                .get(&map_id)
                .expect("rooted map was validated before terminal completion");
            complete_active_work_then_end(
                map,
                expected_revision,
                current_node_id.clone(),
                final_summary.clone(),
            )
            .map_err(rooted_rejection_message)?
        };
        let revision = committed.map.revision;
        let graph_batch =
            graph_revision_committed_record(&committed.events, "complete_active_work_then_end");
        let old_lease_id = self
            .current_main_lease_id
            .clone()
            .ok_or_else(|| "TaskSpace current binding has no main lease.".to_string())?;
        let result_id = self.next_result_id();
        let map = self
            .maps
            .get_mut(&map_id)
            .expect("validated rooted map remains present");
        map.commit_graph(committed.map, committed.events);
        map.leases.remove(&old_lease_id);
        map.results.insert(
            result_id.clone(),
            NodeResult {
                id: result_id.clone(),
                assignment_id: old_lease_id.clone(),
                map_id: map_id.clone(),
                node_id: current_node_id.clone(),
                kind: NodeResultKind::Result,
                action_class: None,
                tool_success: None,
                source_event_ref,
                artifact_refs: Vec::new(),
                source_thread_id: owner_session_id,
                created_at_ms: now_ms(),
            },
        );
        let completed_node = map
            .nodes
            .get_mut(&current_node_id)
            .expect("completed terminal node exists");
        completed_node.active_lease = None;
        completed_node.result_context.push(NodeResultRef {
            id: result_id,
            kind: NodeResultKind::Result,
        });
        self.active_map_id = None;
        self.active_task_id = None;
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.routing_required = true;
        self.bootstrap_required = false;
        let graph_event = MapRuntimeEvent::GraphRevisionCommitted(graph_batch.clone());
        let trace_event = self.record_runtime_budget_trace_event(
            "terminal_committed",
            None,
            map_id.clone(),
            "finish".to_string(),
            None,
            true,
            vec![
                "schema:taskspace-rooted-terminal-event-v1".to_string(),
                "operation:complete_active_work_then_end".to_string(),
                format!("completed_node_id:{current_node_id}"),
                format!("revision:{revision}"),
                "state_commit:true".to_string(),
                "summary_source:agent".to_string(),
                "runtime_inferred_semantics:false".to_string(),
            ],
        );
        Ok((
            ActionMapTerminalOutcome {
                map_id: map_id.clone(),
                revision,
                final_summary,
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision: revision,
                    graph_revision_batches: vec![graph_batch],
                    node_detail_events: Vec::new(),
                },
            },
            vec![graph_event, trace_event],
        ))
    }

    pub(crate) fn close_finish_with_no_active_work_for_main(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        final_summary: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        let mut candidate = self.clone();
        let outcome = candidate.close_finish_with_no_active_work_for_main_inner(
            owner_session_id,
            expected_revision,
            final_summary,
        )?;
        *self = candidate;
        Ok(outcome)
    }

    fn close_finish_with_no_active_work_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        expected_revision: u64,
        final_summary: String,
    ) -> Result<(ActionMapTerminalOutcome, Vec<MapRuntimeEvent>), String> {
        self.validate_routing_complete()?;
        let map_id = self
            .active_map_id
            .clone()
            .ok_or_else(|| "TaskSpace has no active rooted map.".to_string())?;
        let committed = {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace rooted map `{map_id}` is missing."))?;
            if map.owner_session_id != Some(owner_session_id) {
                return Err("TaskSpace rooted map owner mismatch.".to_string());
            }
            close_finish_with_no_active_work(map, expected_revision, final_summary.clone())
                .map_err(rooted_rejection_message)?
        };
        let revision = committed.map.revision;
        let graph_batch =
            graph_revision_committed_record(&committed.events, "close_finish_with_no_active_work");
        let graph_event = MapRuntimeEvent::GraphRevisionCommitted(graph_batch.clone());
        self.maps
            .get_mut(&map_id)
            .expect("validated rooted map remains present")
            .commit_graph(committed.map, committed.events);
        self.active_map_id = None;
        self.active_task_id = None;
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.routing_required = true;
        self.bootstrap_required = false;
        let event = self.record_runtime_budget_trace_event(
            "terminal_committed",
            None,
            map_id.clone(),
            "finish".to_string(),
            None,
            true,
            vec![
                "schema:taskspace-rooted-terminal-event-v1".to_string(),
                "operation:close_finish_with_no_active_work".to_string(),
                format!("revision:{revision}"),
                "state_commit:true".to_string(),
                "summary_source:agent".to_string(),
                "runtime_inferred_semantics:false".to_string(),
            ],
        );
        Ok((
            ActionMapTerminalOutcome {
                map_id: map_id.clone(),
                revision,
                final_summary,
                delta: ActionMapControlDelta {
                    map_id,
                    committed_revision: revision,
                    graph_revision_batches: vec![graph_batch],
                    node_detail_events: Vec::new(),
                },
            },
            vec![graph_event, event],
        ))
    }

    pub(crate) fn expand_node_details_for_main(
        &mut self,
        owner_session_id: ThreadId,
        node_ids: Vec<MapNodeId>,
        call_id: String,
        source_event_id: String,
    ) -> Result<
        (
            Vec<ActionMapNodeDetailExpansionOutcome>,
            Vec<MapRuntimeEvent>,
        ),
        String,
    > {
        let mut candidate = self.clone();
        let outcome = candidate.expand_node_details_for_main_inner(
            owner_session_id,
            node_ids,
            call_id,
            source_event_id,
        )?;
        *self = candidate;
        Ok(outcome)
    }

    fn expand_node_details_for_main_inner(
        &mut self,
        owner_session_id: ThreadId,
        node_ids: Vec<MapNodeId>,
        call_id: String,
        source_event_id: String,
    ) -> Result<
        (
            Vec<ActionMapNodeDetailExpansionOutcome>,
            Vec<MapRuntimeEvent>,
        ),
        String,
    > {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        self.validate_routing_complete()?;
        if node_ids.is_empty() {
            return Err(
                "TaskSpace expand_nodes requires at least one node_id. hard_state: empty_expand_nodes."
                    .to_string(),
            );
        }
        let call_id = require_nonempty_owned("call_id", call_id)?;
        let source_event_id = require_nonempty_owned("source_event_id", source_event_id)?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace has no active map. hard_state: no_active_task_path.".to_string()
        })?;
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        if map.owner_session_id != Some(owner_session_id) {
            return Err(
                "TaskSpace active map is owned by another session. hard_state: map_owner_mismatch."
                    .to_string(),
            );
        }
        let committed_revision = map.revision;
        let mut unique_node_ids = HashSet::with_capacity(node_ids.len());
        let detail_plan = node_detail_plan(map, self.current_main_node_id.as_deref());
        let baseline_details =
            baseline_projection_node_details(map, self.current_main_node_id.as_deref());
        let all_details = projection_all_node_details(map, self.current_main_node_id.as_deref());
        let mut detail_payloads = Vec::with_capacity(node_ids.len());
        for node_id in &node_ids {
            if node_id.trim().is_empty() {
                return Err(
                    "TaskSpace expand_nodes requires non-empty node_ids. hard_state: empty_expand_node_id."
                        .to_string(),
                );
            }
            if !unique_node_ids.insert(node_id.clone()) {
                return Err(format!(
                    "TaskSpace expand_nodes repeats node `{node_id}`. hard_state: duplicate_expand_node_id."
                ));
            }
            let Some(node) = map.nodes.get(node_id) else {
                return Err(format!(
                    "TaskSpace expand_nodes target `{node_id}` does not exist. hard_state: expand_node_missing."
                ));
            };
            let Some(NodeDetailState::FoldEligible { .. }) = detail_plan.state(node_id) else {
                return Err(format!(
                    "TaskSpace expand_nodes target `{node_id}` is not folded. hard_state: node_detail_not_folded."
                ));
            };
            let hidden_details = hidden_node_details(node_id, &all_details, &baseline_details);
            let Some((_, detail_identity)) =
                folded_projection_node(&map_id, node_id, node, &hidden_details)
            else {
                return Err(format!(
                    "TaskSpace expand_nodes target `{node_id}` is not folded. hard_state: node_detail_not_folded."
                ));
            };
            detail_payloads.push((detail_identity, hidden_details));
        }

        let created_at_ms = now_ms();
        let mut outcomes = Vec::with_capacity(node_ids.len());
        let mut events = Vec::with_capacity(node_ids.len());
        for (node_id, (detail_identity, hidden_details)) in
            node_ids.into_iter().zip(detail_payloads)
        {
            let expansion_event_id = self.next_node_event_id();
            let content = format!(
                "NodeDetailExpandedV1\nmap_id:{map_id}\nnode_id:{node_id}\ncall_id:{call_id}\nsource_event_id:{source_event_id}"
            );
            let event = NodeEvent {
                id: expansion_event_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                event_kind: NODE_DETAIL_EXPANDED_EVENT_KIND.to_string(),
                source: "agent_taskspace_control".to_string(),
                action_class: None,
                tool_success: None,
                content_sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
                source_event_id: Some(source_event_id.clone()),
                raw_ref: None,
                artifact_refs: Vec::new(),
                call_id: Some(call_id.clone()),
                source_thread_id: owner_session_id,
                created_at_ms,
            };
            let map = self
                .maps
                .get_mut(&map_id)
                .expect("active map was validated before expansion commit");
            map.node_events.insert(expansion_event_id.clone(), event);
            map.nodes
                .get_mut(&node_id)
                .expect("expansion target was validated before commit")
                .node_events
                .push(NodeEventRef {
                    id: expansion_event_id.clone(),
                    kind: NODE_DETAIL_EXPANDED_EVENT_KIND.to_string(),
                });
            let committed_event = MapRuntimeNodeDetailExpandedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                expansion_event_id: expansion_event_id.clone(),
                call_id: call_id.clone(),
                source_event_id: source_event_id.clone(),
                source_thread_id: owner_session_id,
            };
            outcomes.push(ActionMapNodeDetailExpansionOutcome {
                node_id: node_id.clone(),
                expansion_event_id: expansion_event_id.clone(),
                detail_ref: detail_identity.detail_ref,
                restored_details: hidden_details.iter().map(expanded_detail_ref).collect(),
                delta: ActionMapControlDelta {
                    map_id: map_id.clone(),
                    committed_revision,
                    graph_revision_batches: Vec::new(),
                    node_detail_events: vec![committed_event.clone()],
                },
            });
            events.push(MapRuntimeEvent::NodeDetailExpanded(committed_event));
        }
        Ok((outcomes, events))
    }

    pub(crate) fn prepare_spawn_assignment(
        &mut self,
        _owner_session_id: ThreadId,
        _requested_task_name: &str,
        requested_node_id: Option<&str>,
    ) -> Result<(Option<ActionMapAssignment>, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok((None, Vec::new()));
        }

        self.validate_routing_complete()?;
        let mut events = Vec::new();
        let Some(map_id) = self.active_map_id.clone() else {
            return Err(
                "TaskSpace mode is active but no active task path exists. hard_state: no_active_task_path. subagent spawn requires an active task path and bindable node."
                    .to_string(),
            );
        };
        let (node_count_for_budget, budget_trace_node_id) = {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            (
                map.nodes.len(),
                self.current_main_node_id
                    .clone()
                    .or_else(|| ordered_node_ids(map).into_iter().next())
                    .unwrap_or_else(|| "spawn-budget".to_string()),
            )
        };
        let active_budget = self.active_budget.as_ref().cloned().unwrap_or_else(|| {
            taskspace_active_budget_for_route(
                "taskspace-v005-active",
                TaskSpaceRouteMode::DefaultCompact,
            )
        });
        self.budget_counters.node_count = node_count_for_budget;
        let max_spawn_agent_calls = active_budget.max_spawn_agent_calls;
        let max_nodes = active_budget.max_nodes;
        let spawn_count_before = self.budget_counters.spawn_agent_call_count;
        let gate_decision = self.gate_spawn_budget(&map_id, &budget_trace_node_id);
        let requested_node_id = requested_node_id
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        let node_id = self.select_spawn_node_id(&map_id, requested_node_id)?;
        self.validate_maintenance_barrier_for_node(&node_id)?;

        let lease_id = self.next_lease_id();
        let map = self
            .maps
            .get_mut(&map_id)
            .expect("active map id should exist");
        let previous_node_status = map
            .nodes
            .get(&node_id)
            .expect("ready node id should exist")
            .status;
        let committed = transition_node(map, map.revision, node_id.clone(), NodeTransition::Bind)
            .map_err(rooted_rejection_message)?;
        let graph_event = graph_revision_committed_event(&committed.events, "subagent_bind");
        map.commit_graph(committed.map, committed.events);
        let node = map
            .nodes
            .get_mut(&node_id)
            .expect("bound node remains present");
        node.active_lease = Some(lease_id.clone());
        let node_goal = node.goal.clone();
        map.leases.insert(
            lease_id.clone(),
            AssignmentLease {
                id: lease_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                holder: LeaseHolder::SubAgent,
                previous_node_status,
                agent_thread_id: None,
                agent_path: None,
            },
        );
        events.push(graph_event);
        events.push(MapRuntimeEvent::LeaseCreated(MapRuntimeLeaseCreatedEvent {
            map_id: map_id.clone(),
            node_id: node_id.clone(),
            lease_id: lease_id.clone(),
            holder: LeaseHolder::SubAgent.as_str().to_string(),
        }));
        let task_id_for_budget = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        self.budget_counters.spawn_agent_call_count = spawn_count_before + 1;
        events.push(self.record_runtime_budget_trace_event(
            "spawn_node_budget",
            task_id_for_budget,
            map_id.clone(),
            node_id.clone(),
            Some(lease_id.clone()),
            true,
            vec![
                "schema:taskspace-spawn-node-budget-event-v1".to_string(),
                "producer:runtime".to_string(),
                "budget_kind:spawn".to_string(),
                "action:spawn_agent".to_string(),
                "status:allowed".to_string(),
                format!("spawn_agent_call_count_before:{spawn_count_before}"),
                format!("spawn_agent_call_count_after:{}", spawn_count_before + 1),
                "active_budget_source:runtime".to_string(),
                "enforcement:advisory".to_string(),
                format!("route_mode:{}", active_budget.route_mode.as_str()),
                format!("profile_name:{}", active_budget.profile_name.as_str()),
                format!("max_spawn_agent_calls:{max_spawn_agent_calls}"),
                format!("node_count:{node_count_for_budget}"),
                format!("max_nodes:{max_nodes}"),
                "budget_response_action_taken:false".to_string(),
                format!("budget_gate_reason:{}", gate_decision.reason),
            ],
        ));

        Ok((
            Some(ActionMapAssignment {
                message_prefix: assignment_prompt(&map_id, &node_id, &node_goal, &lease_id),
                map_id,
                node_id,
                node_title: node_goal,
                lease_id,
            }),
            events,
        ))
    }

    pub(crate) fn attach_agent_to_lease(
        &mut self,
        lease_id: &str,
        thread_id: ThreadId,
        agent_path: Option<String>,
    ) -> Option<MapRuntimeEvent> {
        for map in self.maps.values_mut() {
            let Some(lease) = map.leases.get_mut(lease_id) else {
                continue;
            };
            if lease.holder != LeaseHolder::SubAgent {
                return None;
            }
            if let Some(attached_thread_id) = lease.agent_thread_id
                && attached_thread_id != thread_id
            {
                return None;
            }
            lease.agent_thread_id = Some(thread_id);
            lease.agent_path = agent_path.clone();
            return Some(MapRuntimeEvent::LeaseAttached(
                MapRuntimeLeaseAttachedEvent {
                    map_id: lease.map_id.clone(),
                    node_id: lease.node_id.clone(),
                    lease_id: lease.id.clone(),
                    agent_thread_id: thread_id,
                    agent_path,
                },
            ));
        }
        None
    }

    pub(crate) fn release_lease(
        &mut self,
        lease_id: &str,
        reason: impl Into<String>,
    ) -> Vec<MapRuntimeEvent> {
        let reason = reason.into();
        for map in self.maps.values_mut() {
            let Some(lease) = map.leases.get(lease_id).cloned() else {
                continue;
            };
            let committed = transition_node(
                map,
                map.revision,
                lease.node_id.clone(),
                NodeTransition::ReleaseLease,
            )
            .expect("an active lease must own the canonical running binding");
            let graph_event = graph_revision_committed_event(&committed.events, "release_lease");
            map.commit_graph(committed.map, committed.events);
            map.leases.remove(lease_id);
            let released_main_lease = lease.holder == LeaseHolder::Main;
            let events = vec![
                graph_event,
                MapRuntimeEvent::LeaseReleased(MapRuntimeLeaseReleasedEvent {
                    map_id: lease.map_id.clone(),
                    node_id: lease.node_id.clone(),
                    lease_id: lease.id.clone(),
                    holder: lease.holder.as_str().to_string(),
                    reason,
                }),
            ];
            if let Some(node) = map.nodes.get_mut(&lease.node_id)
                && node.active_lease.as_deref() == Some(lease_id)
            {
                node.active_lease = None;
            }
            if released_main_lease {
                self.current_main_lease_id = None;
                if self.current_main_node_id.as_deref() == Some(lease.node_id.as_str()) {
                    self.current_main_node_id = None;
                }
            }
            self.child_tool_reservations
                .retain(|_, reservation| reservation.lease_id != lease.id);
            return events;
        }
        Vec::new()
    }

    pub(crate) fn release_lease_for_thread(
        &mut self,
        child_thread_id: ThreadId,
        reason: impl Into<String>,
    ) -> Option<(AssignmentLeaseId, Vec<MapRuntimeEvent>)> {
        let (_, lease_id, _) = self.find_lease_by_thread(child_thread_id)?;
        let events = self.release_lease(&lease_id, reason);
        Some((lease_id, events))
    }

    pub(crate) fn record_child_result(
        &mut self,
        child_thread_id: ThreadId,
        status: &AgentStatus,
    ) -> Option<(NodeResultId, Vec<MapRuntimeEvent>)> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let (map_id, lease_id, node_id) = self.find_lease_by_thread(child_thread_id)?;
        let result_id = self.next_result_id();
        let kind = result_kind_from_status(status);
        let transition = match kind {
            NodeResultKind::Result | NodeResultKind::MapUpdateRequest => NodeTransition::Complete,
            NodeResultKind::Blocker | NodeResultKind::TimeoutSummary => NodeTransition::Block,
            NodeResultKind::MainToolCall => return None,
        };
        let map = self.maps.get_mut(&map_id)?;
        let node = map.nodes.get(&node_id)?;
        if node.active_lease.as_deref() != Some(lease_id.as_str()) {
            return None;
        }
        let committed = transition_node(map, map.revision, node_id.clone(), transition).ok()?;
        let graph_event = graph_revision_committed_event(&committed.events, "subagent_result");
        map.commit_graph(committed.map, committed.events);
        let result = NodeResult {
            id: result_id.clone(),
            assignment_id: lease_id.clone(),
            map_id: map_id.clone(),
            node_id: node_id.clone(),
            kind,
            action_class: None,
            tool_success: None,
            source_event_ref: child_thread_source_event_ref(child_thread_id),
            artifact_refs: Vec::new(),
            source_thread_id: child_thread_id,
            created_at_ms: now_ms(),
        };
        map.results.insert(result_id.clone(), result);
        let node = map.nodes.get_mut(&node_id)?;
        node.result_context.push(NodeResultRef {
            id: result_id.clone(),
            kind,
        });
        node.active_lease = None;
        map.leases.remove(&lease_id);
        self.child_tool_reservations
            .retain(|_, reservation| reservation.lease_id != lease_id);
        let events = vec![
            graph_event,
            MapRuntimeEvent::NodeResultRecorded(MapRuntimeNodeResultRecordedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id: lease_id.clone(),
                result_id: result_id.clone(),
                kind: kind.as_str().to_string(),
                action_class: None,
                source_thread_id: child_thread_id,
            }),
            MapRuntimeEvent::LeaseReleased(MapRuntimeLeaseReleasedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id: lease_id.clone(),
                holder: LeaseHolder::SubAgent.as_str().to_string(),
                reason: "result_recorded".to_string(),
            }),
        ];
        Some((result_id, events))
    }

    pub(crate) fn active_timeout_targets(&self) -> Vec<ActionMapTimeoutTarget> {
        if self.mode != MapRuntimeMode::Experiment {
            return Vec::new();
        }
        self.active_map()
            .map(|map| {
                map.leases
                    .values()
                    .filter_map(|lease| {
                        if lease.holder != LeaseHolder::SubAgent {
                            return None;
                        }
                        let thread_id = lease.agent_thread_id?;
                        let agent_path = lease
                            .agent_path
                            .as_deref()
                            .and_then(|path| AgentPath::try_from(path).ok());
                        Some(ActionMapTimeoutTarget {
                            thread_id,
                            agent_path,
                            map_id: lease.map_id.clone(),
                            node_id: lease.node_id.clone(),
                            lease_id: lease.id.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn build_developer_context(
        &mut self,
        envelope: ProjectionEnvelope,
    ) -> Option<String> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        if self.bootstrap_required {
            return Some(self.build_bootstrap_compact_developer_context(envelope));
        }
        if let Some(context) = self.build_active_projection_developer_context(envelope) {
            return Some(context);
        }
        Some(self.build_bootstrap_compact_developer_context(envelope))
    }

    pub(crate) fn build_map_handle_context(&self) -> Option<String> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        let active_map = self
            .active_map_id
            .as_deref()
            .and_then(|map_id| self.maps.get(map_id));
        let map_id = active_map.map_or("none", |map| map.id.as_str());
        let revision =
            active_map.map_or_else(|| "none".to_string(), |map| map.revision.to_string());
        let bootstrap_required = self.bootstrap_required || active_map.is_none();
        let mut context = format!(
            "TaskSpaceMapHandleR7V1:\n- schema_version: taskspace-map-handle-r7-v1\n- taskspace_active: true\n- map_id: {map_id}\n- revision: {revision}\n- bootstrap_required: {bootstrap_required}\n- available_read_action: taskspace_control.read_map\n"
        );
        if bootstrap_required {
            context.push_str(
                "- bootstrap_action: initialize_map\n- action_carrier: ordinary_tool.taskspace_action\n- ordinary_tools_allowed: with_explicit_taskspace_action\n- ordinary_tool_without_action_failure: TASKSPACE_ACTION_REQUIRED\n",
            );
        }
        context.push_str("TaskSpaceMapHandleR7V1 end.\n");
        Some(context)
    }

    #[allow(dead_code)]
    pub(crate) fn build_developer_context_for_map(
        &mut self,
        map_id: &str,
        envelope: ProjectionEnvelope,
    ) -> Option<String> {
        if self.mode != MapRuntimeMode::Experiment || !self.maps.contains_key(map_id) {
            return None;
        }
        self.build_projection_developer_context(map_id.to_string(), envelope, false)
    }

    fn build_bootstrap_compact_developer_context(&self, envelope: ProjectionEnvelope) -> String {
        let request_snapshot_fields = if envelope == ProjectionEnvelope::RequestSnapshot {
            "- supersedes_all_prior_projections: true\n- current_state_rule: last_projection_only\n"
        } else {
            ""
        };
        format!(
            "TaskSpaceMapProjectionR7V1:\n- schema_version: taskspace-map-projection-r7-v1\n- projection_kind: bootstrap_required\n- map: none\n- bootstrap_required: true\n- bootstrap_action: initialize_map\n- action_carrier: ordinary_tool.taskspace_action\n- ordinary_tools_allowed: with_explicit_taskspace_action\n- ordinary_tool_without_action_failure: TASKSPACE_ACTION_REQUIRED\n{request_snapshot_fields}TaskSpaceMapProjectionR7V1 end.\n"
        )
    }

    fn build_active_projection_developer_context(
        &mut self,
        envelope: ProjectionEnvelope,
    ) -> Option<String> {
        let map_id = self.active_map_id.clone()?;
        self.build_projection_developer_context(map_id, envelope, true)
    }

    fn build_projection_developer_context(
        &mut self,
        map_id: String,
        envelope: ProjectionEnvelope,
        include_runtime_status: bool,
    ) -> Option<String> {
        let (task_id, current_node_id, revision, context, projection, max_projection_tokens) = {
            let Some(map) = self.maps.get(&map_id) else {
                return Some(taskspace_projection_integrity_context(
                    &map_id,
                    "active_map_record_missing",
                ));
            };
            let mut context = String::new();
            if include_runtime_status && self.bootstrap_required {
                context.push_str(
                    "Bootstrap status: required before ordinary tools or subagent spawn.\n",
                );
            } else if include_runtime_status && self.routing_required {
                context.push_str(
                    "Task routing status: required before ordinary tools or subagent spawn.\n",
                );
            }
            if include_runtime_status && let Some(barrier) = self.active_maintenance_barrier() {
                context.push_str("Maintenance barrier:\n- map: ");
                context.push_str(&barrier.map_id);
                context.push_str("\n- node: ");
                context.push_str(&barrier.node_id);
                context.push_str("\n- reason: ");
                context.push_str(barrier.reason.as_str());
                context.push('\n');
            }
            let projection = match append_context_projection_active(
                &mut context,
                map,
                (self.active_map_id.as_deref() == Some(map_id.as_str()))
                    .then_some(self.current_main_node_id.as_deref())
                    .flatten(),
                self.active_budget.as_ref(),
                envelope,
            ) {
                Ok(projection) => projection,
                Err(error) => {
                    return Some(taskspace_projection_integrity_context(&map_id, &error));
                }
            };
            (
                map.task_id.clone().unwrap_or_else(|| map.id.clone()),
                self.current_main_node_id
                    .clone()
                    .unwrap_or_else(|| "projection".to_string()),
                map.revision,
                context,
                projection,
                self.active_budget
                    .as_ref()
                    .map(|budget| budget.max_projection_tokens)
                    .unwrap_or(usize::MAX),
            )
        };
        let projection = projection.stats;
        let estimated_tokens = projection.estimated_tokens;
        self.budget_counters.projection_tokens_last = estimated_tokens;
        self.budget_counters.projection_tokens_max = self
            .budget_counters
            .projection_tokens_max
            .max(estimated_tokens);
        let route_mode = self
            .active_budget
            .as_ref()
            .map(|budget| budget.route_mode.as_str())
            .unwrap_or("unknown");
        let profile_name = self
            .active_budget
            .as_ref()
            .map(|budget| budget.profile_name.as_str())
            .unwrap_or("unknown");
        let status = if projection.skeleton_over_budget {
            "map_skeleton_over_budget"
        } else if projection.projection_over_budget {
            "map_projection_over_budget"
        } else if estimated_tokens <= max_projection_tokens {
            "within_budget"
        } else {
            "over_budget"
        };
        tracing::debug!(
            target: "codex_core::taskspace",
            event_name = "taskspace.map_budget_measured",
            task_id,
            map_id,
            revision,
            projection_kind = envelope.kind(),
            projection_bytes = projection.size_breakdown.projection_bytes,
            skeleton_bytes = projection.size_breakdown.skeleton_bytes,
            projection_tokens = estimated_tokens,
            skeleton_tokens = projection.skeleton_estimated_tokens,
            header_bytes = projection.size_breakdown.header_bytes,
            root_source_bytes = projection.size_breakdown.root_source_bytes,
            active_frontier_bytes = projection.size_breakdown.active_frontier_bytes,
            map_node_bytes = projection.size_breakdown.map_node_bytes,
            map_edge_bytes = projection.size_breakdown.map_edge_bytes,
            node_detail_bytes = projection.size_breakdown.node_detail_bytes,
            footer_bytes = projection.size_breakdown.footer_bytes,
            strategy_id = "S4.2",
            strategy_activation_count = usize::from(
                projection.folded_node_count > 0 || projection.expanded_node_count > 0
            ),
            fold_eligible_node_count = projection.fold_eligible_node_count,
            folded_node_count = projection.folded_node_count,
            expanded_node_count = projection.expanded_node_count,
            recoverable_hidden_event_count = projection.recoverable_hidden_event_count,
            folded_hidden_event_count = projection.folded_hidden_event_count,
            b0_projection_bytes = projection.b0_projection_bytes,
            projection_bytes_before_strategy = projection.projection_bytes_before_strategy,
            projection_bytes_after_strategy = projection.size_breakdown.projection_bytes,
            node_detail_bytes_before_strategy = projection.node_detail_bytes_before_strategy,
            node_detail_bytes_after_strategy = projection.size_breakdown.node_detail_bytes,
            skeleton_bytes_before_strategy = projection.b0_skeleton_bytes,
            skeleton_bytes_after_strategy = projection.size_breakdown.skeleton_bytes,
            max_projection_tokens,
            status,
            "measured TaskSpace map projection budget"
        );
        let projection_trace_event = self.record_runtime_budget_trace_event(
            "projection_budget",
            Some(task_id),
            map_id,
            current_node_id,
            None,
            !projection.skeleton_over_budget && estimated_tokens <= max_projection_tokens,
            vec![
                "schema:taskspace-projection-budget-v1".to_string(),
                "producer:runtime".to_string(),
                format!("projection_kind:{}", envelope.kind()),
                format!("revision:{revision}"),
                "active_budget_source:runtime".to_string(),
                format!("route_mode:{route_mode}"),
                format!("profile_name:{profile_name}"),
                format!("projection_tokens:{estimated_tokens}"),
                format!(
                    "projection_bytes:{}",
                    projection.size_breakdown.projection_bytes
                ),
                format!(
                    "skeleton_projection_tokens:{}",
                    projection.skeleton_estimated_tokens
                ),
                format!(
                    "skeleton_projection_bytes:{}",
                    projection.size_breakdown.skeleton_bytes
                ),
                format!(
                    "projection_header_bytes:{}",
                    projection.size_breakdown.header_bytes
                ),
                format!(
                    "projection_root_source_bytes:{}",
                    projection.size_breakdown.root_source_bytes
                ),
                format!(
                    "projection_frontier_bytes:{}",
                    projection.size_breakdown.active_frontier_bytes
                ),
                format!(
                    "projection_node_bytes:{}",
                    projection.size_breakdown.map_node_bytes
                ),
                format!(
                    "projection_edge_bytes:{}",
                    projection.size_breakdown.map_edge_bytes
                ),
                format!(
                    "projection_detail_bytes:{}",
                    projection.size_breakdown.node_detail_bytes
                ),
                format!(
                    "projection_footer_bytes:{}",
                    projection.size_breakdown.footer_bytes
                ),
                format!("max_projection_tokens:{max_projection_tokens}"),
                format!("status:{status}"),
                "strategy_id:S4.2".to_string(),
                format!(
                    "strategy_activation_count:{}",
                    usize::from(
                        projection.folded_node_count > 0 || projection.expanded_node_count > 0
                    )
                ),
                format!(
                    "fold_eligible_node_count:{}",
                    projection.fold_eligible_node_count
                ),
                format!("folded_node_count:{}", projection.folded_node_count),
                format!("expanded_node_count:{}", projection.expanded_node_count),
                format!(
                    "recoverable_hidden_event_count:{}",
                    projection.recoverable_hidden_event_count
                ),
                format!(
                    "folded_hidden_event_count:{}",
                    projection.folded_hidden_event_count
                ),
                format!("b0_projection_bytes:{}", projection.b0_projection_bytes),
                format!(
                    "projection_bytes_before_strategy:{}",
                    projection.projection_bytes_before_strategy
                ),
                format!(
                    "projection_bytes_after_strategy:{}",
                    projection.size_breakdown.projection_bytes
                ),
                format!(
                    "node_detail_bytes_before_strategy:{}",
                    projection.node_detail_bytes_before_strategy
                ),
                format!(
                    "node_detail_bytes_after_strategy:{}",
                    projection.size_breakdown.node_detail_bytes
                ),
                format!(
                    "skeleton_bytes_before_strategy:{}",
                    projection.b0_skeleton_bytes
                ),
                format!(
                    "skeleton_bytes_after_strategy:{}",
                    projection.size_breakdown.skeleton_bytes
                ),
            ],
        );
        self.pending_projection_trace_events
            .push(projection_trace_event);
        Some(context)
    }

    pub(crate) fn timeout_summary_requested_event(
        target: &ActionMapTimeoutTarget,
    ) -> Option<MapRuntimeEvent> {
        let agent_path = target.agent_path.as_ref()?;
        Some(MapRuntimeEvent::TimeoutSummaryRequested(
            MapRuntimeTimeoutSummaryRequestedEvent {
                map_id: target.map_id.clone(),
                node_id: target.node_id.clone(),
                lease_id: target.lease_id.clone(),
                agent_thread_id: target.thread_id,
                agent_path: agent_path.to_string(),
            },
        ))
    }

    fn mark_routing_complete(&mut self) {
        self.routing_required = false;
        self.bootstrap_required = false;
        self.reborn_requested = false;
    }

    fn validate_main_binding(&self, owner_session_id: ThreadId) -> Result<(), String> {
        self.validate_routing_complete()?;
        let Some(map_id) = self.active_map_id.as_ref() else {
            return Err(
                "TaskSpace mode is active but no active task path exists. hard_state: no_active_task_path. ordinary tools require an active task path, current node binding, and lease."
                    .to_string(),
            );
        };
        let Some(map) = self.maps.get(map_id) else {
            return Err(format!("TaskSpace active task path `{map_id}` is missing."));
        };
        if map.is_complete() {
            return Err(format!(
                "TaskSpace active task path `{map_id}` is not active."
            ));
        }
        if map.nodes.is_empty() {
            return Err(
                "TaskSpace active task path has no nodes. hard_state: active_task_path_without_nodes."
                    .to_string(),
            );
        }
        let Some(node_id) = self.current_main_node_id.as_ref() else {
            return Err(
                "TaskSpace mode is active but no current node binding exists. hard_state: no_current_node_binding. ordinary tools require a current node binding and lease."
                    .to_string(),
            );
        };
        let Some(node) = map.nodes.get(node_id) else {
            return Err(format!("TaskSpace current node `{node_id}` is missing."));
        };
        if node.role != NodeRole::Work {
            return Err(format!(
                "TaskSpace current node `{node_id}` cannot hold a main lease. hard_state: current_node_role_invalid."
            ));
        }
        if node.status == NodeStatus::Pending {
            return Err(format!(
                "TaskSpace current node `{node_id}` is still pending. hard_state: current_node_dependencies_incomplete."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace current node `{node_id}` is completed. hard_state: current_node_completed."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            let Some(lease_id) = self.current_main_lease_id.as_ref() else {
                return Err(format!(
                    "TaskSpace current node `{node_id}` is running without a main lease. hard_state: current_node_lease_missing."
                ));
            };
            let Some(lease) = map.leases.get(lease_id) else {
                return Err(format!(
                    "TaskSpace current main lease `{lease_id}` is missing. hard_state: current_node_lease_missing."
                ));
            };
            if lease.holder != LeaseHolder::Main
                || lease.node_id != *node_id
                || lease.agent_thread_id != Some(owner_session_id)
                || node.active_lease.as_deref() != Some(lease_id.as_str())
            {
                return Err(format!(
                    "TaskSpace current node `{node_id}` is not held by the current main lease. hard_state: current_node_lease_mismatch."
                ));
            }
            return Ok(());
        }
        Err(format!(
            "TaskSpace current node `{node_id}` has no main lease. hard_state: no_current_main_lease. ordinary tools require a current node binding and lease."
        ))
    }

    fn validate_routing_complete(&self) -> Result<(), String> {
        if self.bootstrap_required {
            return Err(
                "TaskSpace bootstrap is required for this turn. hard_state: no_task_path. ordinary tools and subagent spawn require an active task path, current node binding, and lease."
                    .to_string(),
            );
        }
        if self.routing_required {
            return Err(
                "TaskSpace task routing is required for this user turn. hard_state: route_unbound. ordinary tools and subagent spawn require an active task path, current node binding, and lease."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn validate_maintenance_barrier(&self) -> Result<(), String> {
        Ok(())
    }

    fn validate_maintenance_barrier_for_node(&self, _node_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn validate_maintenance_barrier_for_map_node(
        &self,
        _map_id: &str,
        _node_id: &str,
    ) -> Result<(), ActionMapGateError> {
        Ok(())
    }

    fn reserved_main_tool_calls(&self, map_id: &str, node_id: &str) -> usize {
        self.main_tool_reservations
            .values()
            .filter(|reservation| reservation.map_id == map_id && reservation.node_id == node_id)
            .count()
    }

    fn reserve_main_tool_call(&mut self, call_id: &str, reservation: MainToolReservation) {
        self.main_tool_reservations
            .insert(call_id.to_string(), reservation);
    }

    fn release_main_tool_reservation(&mut self, call_id: &str) -> Option<MainToolReservation> {
        self.main_tool_reservations.remove(call_id)
    }

    fn reserve_child_tool_call(
        &mut self,
        child_thread_id: ThreadId,
        call_id: &str,
        reservation: ChildToolReservation,
    ) {
        self.child_tool_reservations.insert(
            child_tool_reservation_key(child_thread_id, call_id),
            reservation,
        );
    }

    fn release_child_tool_reservation(
        &mut self,
        child_thread_id: ThreadId,
        call_id: &str,
    ) -> Option<ChildToolReservation> {
        self.child_tool_reservations
            .remove(&child_tool_reservation_key(child_thread_id, call_id))
    }

    fn validate_main_tool_reservation(
        &self,
        owner_session_id: ThreadId,
        reservation: &MainToolReservation,
    ) -> Result<(), String> {
        let map = self.maps.get(&reservation.map_id).ok_or_else(|| {
            format!(
                "TaskSpace tool result target map `{}` is missing.",
                reservation.map_id
            )
        })?;
        if map.is_complete() {
            return Err(format!(
                "TaskSpace tool result target map `{}` is not active.",
                reservation.map_id
            ));
        }
        let node = map.nodes.get(&reservation.node_id).ok_or_else(|| {
            format!(
                "TaskSpace tool result target node `{}` is missing.",
                reservation.node_id
            )
        })?;
        let lease = map.leases.get(&reservation.lease_id).ok_or_else(|| {
            format!(
                "TaskSpace tool result target lease `{}` is missing.",
                reservation.lease_id
            )
        })?;
        if lease.holder != LeaseHolder::Main
            || lease.node_id != reservation.node_id
            || lease.agent_thread_id != Some(owner_session_id)
            || node.active_lease.as_deref() != Some(reservation.lease_id.as_str())
        {
            return Err(format!(
                "TaskSpace tool result target node `{}` is no longer held by its original main lease `{}`.",
                reservation.node_id, reservation.lease_id
            ));
        }
        Ok(())
    }

    fn validate_child_tool_reservation(
        &self,
        child_thread_id: ThreadId,
        reservation: &ChildToolReservation,
    ) -> Result<(), String> {
        let map = self.maps.get(&reservation.map_id).ok_or_else(|| {
            format!(
                "TaskSpace subagent tool result target map `{}` is missing.",
                reservation.map_id
            )
        })?;
        if map.is_complete() {
            return Err(format!(
                "TaskSpace subagent tool result target map `{}` is not active.",
                reservation.map_id
            ));
        }
        let node = map.nodes.get(&reservation.node_id).ok_or_else(|| {
            format!(
                "TaskSpace subagent tool result target node `{}` is missing.",
                reservation.node_id
            )
        })?;
        let lease = map.leases.get(&reservation.lease_id).ok_or_else(|| {
            format!(
                "TaskSpace subagent tool result target lease `{}` is missing.",
                reservation.lease_id
            )
        })?;
        if reservation.child_thread_id != child_thread_id
            || lease.holder != LeaseHolder::SubAgent
            || lease.node_id != reservation.node_id
            || lease.agent_thread_id != Some(child_thread_id)
            || node.active_lease.as_deref() != Some(reservation.lease_id.as_str())
        {
            return Err(format!(
                "TaskSpace subagent tool result target node `{}` is no longer held by its original subagent lease `{}`.",
                reservation.node_id, reservation.lease_id
            ));
        }
        Ok(())
    }

    fn validate_no_main_tool_reservations_for_node(
        &self,
        map_id: &str,
        node_id: &str,
        action: &str,
    ) -> Result<(), String> {
        let in_flight = self.reserved_main_tool_calls(map_id, node_id);
        if in_flight == 0 {
            return Ok(());
        }
        Err(format!(
            "TaskSpace node `{node_id}` has {in_flight} in-flight main tool call(s). hard_state: node_tool_calls_in_flight. rejected_action: {action}."
        ))
    }

    fn active_maintenance_barrier(&self) -> Option<&ActionMapMaintenanceBarrier> {
        self.active_map_id
            .as_ref()
            .and_then(|map_id| self.maintenance_barriers.get(map_id))
    }

    fn ready_spawn_node_ids(&self, map_id: &str) -> Vec<MapNodeId> {
        let Some(map) = self.maps.get(map_id) else {
            return Vec::new();
        };
        ordered_node_ids(map)
            .into_iter()
            .filter(|node_id| {
                map.nodes.get(node_id).is_some_and(|node| {
                    node.role == NodeRole::Work
                        && node.status == NodeStatus::Ready
                        && node.active_lease.is_none()
                })
            })
            .collect()
    }

    fn select_spawn_node_id(
        &self,
        map_id: &str,
        requested_node_id: Option<&str>,
    ) -> Result<MapNodeId, String> {
        let requested_node_id = requested_node_id
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        if let Some(node_id) = requested_node_id {
            self.validate_requested_spawn_node(map_id, node_id)?;
            return Ok(node_id.to_string());
        }

        let ready_nodes = self.ready_spawn_node_ids(map_id);
        match ready_nodes.as_slice() {
            [node_id] => Ok(node_id.clone()),
            [] => Err("TaskSpace mode is active, but no ready node is available.".to_string()),
            _ => Err(format!(
                "TaskSpace mode has multiple ready nodes; spawn_agent requires an explicit node_id: {}.",
                self.format_ready_spawn_node_candidates(map_id, &ready_nodes)
            )),
        }
    }

    fn validate_requested_spawn_node(&self, map_id: &str, node_id: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let node = map.nodes.get(node_id).ok_or_else(|| {
            format!("TaskSpace node `{node_id}` does not exist on active task path `{map_id}`.")
        })?;
        if node.role != NodeRole::Work {
            return Err(format!(
                "TaskSpace node `{node_id}` cannot hold a worker lease. hard_state: target_node_role_invalid."
            ));
        }
        if node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace node `{node_id}` is held by an active lease. hard_state: target_node_lease_conflict."
            ));
        }
        match node.status {
            NodeStatus::Ready => Ok(()),
            NodeStatus::Pending => Err(format!(
                "TaskSpace node `{node_id}` is pending. hard_state: target_node_dependencies_incomplete."
            )),
            NodeStatus::Running => Err(format!(
                "TaskSpace node `{node_id}` is running. hard_state: target_node_lease_conflict."
            )),
            NodeStatus::Blocked => Err(format!(
                "TaskSpace node `{node_id}` is blocked. hard_state: target_node_blocked."
            )),
            NodeStatus::Completed => Err(format!(
                "TaskSpace node `{node_id}` is completed. hard_state: target_node_completed."
            )),
            NodeStatus::Open | NodeStatus::Closed => Err(format!(
                "TaskSpace node `{node_id}` cannot hold a worker lease. hard_state: target_node_role_invalid."
            )),
        }
    }

    fn format_ready_spawn_node_candidates(&self, map_id: &str, node_ids: &[MapNodeId]) -> String {
        let Some(map) = self.maps.get(map_id) else {
            return node_ids.join(", ");
        };
        format_node_candidates(map, node_ids)
    }

    fn find_lease_by_thread(
        &self,
        child_thread_id: ThreadId,
    ) -> Option<(ActionMapId, AssignmentLeaseId, MapNodeId)> {
        self.maps.iter().find_map(|(map_id, map)| {
            map.leases.iter().find_map(|(lease_id, lease)| {
                (lease.holder == LeaseHolder::SubAgent
                    && lease.agent_thread_id == Some(child_thread_id))
                .then(|| (map_id.clone(), lease_id.clone(), lease.node_id.clone()))
            })
        })
    }

    fn append_main_tool_trace_events(&mut self, draft: MainToolTraceDraft) -> Vec<MapRuntimeEvent> {
        let id = self.next_trace_event_id();
        let tags = trace_tags_for(draft.action_class, draft.tool_success, &draft.body);
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: "main_tool_result".to_string(),
            task_id: draft.task_id,
            map_id: draft.map_id,
            node_id: draft.node_id,
            result_id: Some(draft.node_event_id),
            call_id: Some(draft.call_id),
            action_class: draft.action_class,
            tool_success: Some(draft.tool_success),
            tags,
            artifact_refs: draft.artifact_refs,
            created_at_ms: draft.created_at_ms,
        };
        self.taskspace_trace_events.push(event.clone());
        let mut events = vec![MapRuntimeEvent::TaskspaceTraceEventRecorded(
            MapRuntimeTraceEventRecordedEvent {
                trace_event_id: id,
                kind: event.kind.clone(),
                task_id: event.task_id.clone(),
                map_id: event.map_id.clone(),
                node_id: event.node_id.clone(),
                result_id: event.result_id.clone(),
                call_id: event.call_id.clone(),
                action_class: event.action_class.map(|class| class.as_str().to_string()),
                tool_success: event.tool_success,
                tags: event.tags.clone(),
                artifact_refs: event.artifact_refs.clone(),
                created_at_ms: event.created_at_ms,
            },
        )];
        self.clear_validator_failure_sentinels_after_success(&event);
        for draft in warning_drafts_for_trace_event(&event) {
            let sentinel_id = self.next_sentinel_warning_id();
            let warning = TaskSpaceSentinelWarning {
                id: sentinel_id.clone(),
                warning_type: draft.warning_type,
                status: TaskSpaceSentinelWarningStatus::Active,
                severity: draft.severity,
                task_id: event.task_id.clone(),
                map_id: event.map_id.clone(),
                node_id: event.node_id.clone(),
                result_id: event.result_id.clone(),
                trace_event_ids: vec![event.id.clone()],
                reason: draft.reason.to_string(),
                clearance_action: draft.clearance_action.to_string(),
                clear_action: None,
                created_at_ms: event.created_at_ms,
                cleared_at_ms: None,
            };
            self.sentinel_warnings.push(warning.clone());
            events.push(MapRuntimeEvent::SentinelWarningRaised(
                sentinel_warning_raised_event(&warning),
            ));
        }
        events
    }

    fn clear_validator_failure_sentinels_after_success(&mut self, event: &TaskSpaceTraceEvent) {
        if !event.tags.iter().any(|tag| tag == "validator_success") {
            return;
        }
        for warning in &mut self.sentinel_warnings {
            if warning.warning_type != TaskSpaceSentinelWarningType::ValidatorFailure
                || warning.status != TaskSpaceSentinelWarningStatus::Active
                || warning.map_id != event.map_id
            {
                continue;
            }
            if warning.task_id != event.task_id {
                continue;
            }
            warning.status = TaskSpaceSentinelWarningStatus::Cleared;
            warning.clear_action = Some("FixApplied".to_string());
            warning.cleared_at_ms = Some(event.created_at_ms);
            if !warning.trace_event_ids.iter().any(|id| id == &event.id) {
                warning.trace_event_ids.push(event.id.clone());
            }
        }
    }

    fn next_sentinel_warning_id(&mut self) -> String {
        let id = format!("sentinel-{}", self.next_sentinel_warning_seq);
        self.next_sentinel_warning_seq += 1;
        id
    }

    fn next_task_id(&mut self) -> TaskId {
        let id = format!("task-{}", self.next_task_seq);
        self.next_task_seq += 1;
        id
    }

    fn next_map_id(&mut self) -> ActionMapId {
        let id = format!("map-{}", self.next_map_seq);
        self.next_map_seq += 1;
        id
    }

    fn next_lease_id(&mut self) -> AssignmentLeaseId {
        let id = format!("lease-{}", self.next_lease_seq);
        self.next_lease_seq += 1;
        id
    }

    fn next_node_event_id(&mut self) -> NodeEventId {
        let id = format!("node-event-{}", self.next_node_event_seq);
        self.next_node_event_seq += 1;
        id
    }

    fn next_result_id(&mut self) -> NodeResultId {
        let id = format!("result-{}", self.next_result_seq);
        self.next_result_seq += 1;
        id
    }

    fn next_trace_event_id(&mut self) -> String {
        let id = format!("trace-{}", self.next_trace_event_seq);
        self.next_trace_event_seq += 1;
        id
    }
}

pub(crate) fn format_action_map_snapshot(snapshot: &ActionMapSnapshot) -> String {
    let mut output = String::new();
    output.push_str("TaskSpace R6\n");
    output.push_str(&format!("- schema: {}\n", snapshot.schema_version));
    output.push_str(&format!("- mode: {}\n", snapshot.mode));
    output.push_str(&format!(
        "- bootstrap_required: {}\n",
        snapshot.bootstrap_required
    ));
    output.push_str("- trace events: total=");
    output.push_str(&snapshot.trace_summary.total_event_count.to_string());
    output.push_str(", tools=");
    output.push_str(&snapshot.trace_summary.tool_call_count.to_string());
    output.push_str(", failed_tools=");
    output.push_str(&snapshot.trace_summary.failed_tool_call_count.to_string());
    output.push_str(", validator_failures=");
    output.push_str(&snapshot.trace_summary.validator_failure_count.to_string());
    output.push_str(", unclassified_shell=");
    output.push_str(
        &snapshot
            .trace_summary
            .unclassified_shell_action_count
            .to_string(),
    );
    output.push('\n');
    output.push_str("- sentinel warnings: total=");
    output.push_str(&snapshot.sentinel_summary.total_warning_count.to_string());
    output.push_str(", active=");
    output.push_str(&snapshot.sentinel_summary.active_warning_count.to_string());
    output.push_str(", validator_failures=");
    output.push_str(
        &snapshot
            .sentinel_summary
            .validator_failure_warning_count
            .to_string(),
    );
    output.push_str(", unclassified_shell=");
    output.push_str(
        &snapshot
            .sentinel_summary
            .unclassified_shell_warning_count
            .to_string(),
    );
    output.push('\n');
    if !snapshot.maintenance_barriers.is_empty() {
        output.push_str("\nMaintenance barriers:\n");
        for barrier in &snapshot.maintenance_barriers {
            output.push_str("- map=");
            output.push_str(&barrier.map_id);
            output.push_str(" node=");
            output.push_str(&barrier.node_id);
            output.push_str(" reason=");
            output.push_str(&barrier.reason);
            output.push_str(" results=");
            output.push_str(&barrier.result_count.to_string());
            output.push('/');
            output.push_str(&barrier.budget.to_string());
            output.push('\n');
        }
    }
    let Some(map) = snapshot.map.as_ref() else {
        output.push_str("\nMap: none\n");
        return output;
    };
    output.push_str(&format!(
        "\nMap {} revision={} root={} finish={} complete={}\n",
        map.id, map.revision, map.root_node_id, map.finish_node_id, map.complete
    ));
    output.push_str(&format!(
        "- current_node: {}\n",
        map.current_node_id.as_deref().unwrap_or("none")
    ));
    output.push_str("Nodes:\n");
    for node in &map.nodes {
        output.push_str(&format!(
            "- {} role={} status={} goal={:?}",
            node.id, node.role, node.status, node.goal
        ));
        if let Some(lease) = node.active_lease.as_ref() {
            output.push_str(&format!(" lease={lease}"));
        }
        if !node.result_ids.is_empty() {
            output.push_str(&format!(" results={}", node.result_ids.join(",")));
        }
        output.push('\n');
    }
    output.push_str("Edges:\n");
    for edge in &map.edges {
        output.push_str(&format!("- {} -> {}\n", edge.from, edge.to));
    }
    output
}

fn taskspace_projection_integrity_context(map_id: &str, reason: &str) -> String {
    format!(
        "TaskSpaceMapProjectionR7V1:\n\
- schema_version: taskspace-map-projection-r7-v1\n\
- projection_kind: integrity_error\n\
- map_id: {map_id}\n\
- integrity_status: invalid\n\
- integrity_reason: {reason}\n\
- current_node: unavailable\n\
- active_frontier:\n  - none\n\
- map_nodes:\n  - none\n\
- map_edges:\n  - none\n\
- node_details:\n  - none\n\
TaskSpaceMapProjectionR7V1 end."
    )
}

fn baseline_projection_node(node_id: &str, node: &MapNode) -> ProjectionNode {
    ProjectionNode {
        id: node_id.to_string(),
        role: node.role.as_str().to_string(),
        status: node.status.as_str().to_string(),
        goal: node.goal.clone(),
        result_ids: node
            .result_context
            .iter()
            .map(|result| result.id.clone())
            .collect(),
        event_count: node
            .node_events
            .iter()
            .filter(|event| event.kind != NODE_DETAIL_EXPANDED_EVENT_KIND)
            .count(),
        detail_state: None,
    }
}

fn folded_projection_node(
    map_id: &str,
    node_id: &str,
    node: &MapNode,
    hidden_details: &[ProjectionEventRef],
) -> Option<(ProjectionNode, ProjectionNodeDetailIdentity)> {
    if hidden_details.is_empty() {
        return None;
    }
    let baseline = baseline_projection_node(node_id, node);
    let identity = node_detail_identity(map_id, node_id, hidden_details);
    let mut folded = baseline.clone();
    folded.detail_state = Some(ProjectionNodeDetailState::Folded {
        hidden_event_count: hidden_details.len(),
        detail_ref: identity.detail_ref.clone(),
    });
    node_detail_fold_saves_bytes(&baseline, &folded, hidden_details).then_some((folded, identity))
}

fn expanded_detail_ref(event: &ProjectionEventRef) -> ActionMapExpandedDetailRef {
    ActionMapExpandedDetailRef {
        event_id: event.id.clone(),
        event_kind: event.event_kind.clone(),
        source: event.source.clone(),
        detail_tier: event.detail_tier.clone(),
        evidence_class: event.evidence_class.clone(),
        action_class: event.action_class.clone(),
        tool_success: event.tool_success,
        content_sha256: event.content_sha256.clone(),
        raw_ref: event.raw_ref.clone(),
        artifact_refs: event.artifact_refs.clone(),
    }
}

fn append_context_projection_active(
    context: &mut String,
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
    active_budget: Option<&TaskSpaceActiveBudgetV1>,
    envelope: ProjectionEnvelope,
) -> Result<ProjectionRenderOutcome, String> {
    let current_node_id = current_node_id
        .filter(|node_id| map.nodes.contains_key(*node_id))
        .map(str::to_string);
    let detail_plan = node_detail_plan(map, current_node_id.as_deref());
    let ordered_node_ids = ordered_node_ids(map);
    let baseline_node_skeleton = ordered_node_ids
        .iter()
        .filter_map(|node_id| {
            map.nodes
                .get(node_id)
                .map(|node| baseline_projection_node(node_id, node))
        })
        .collect::<Vec<_>>();
    let map_edges = map
        .edges
        .iter()
        .map(|edge| ProjectionEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
        })
        .collect::<Vec<_>>();
    let active_frontier = ordered_node_ids
        .iter()
        .filter(|&node_id| {
            map.nodes.get(node_id).is_some_and(|node| {
                node.role == NodeRole::Work
                    && matches!(node.status, NodeStatus::Ready | NodeStatus::Running)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let baseline_node_details = baseline_projection_node_details(map, current_node_id.as_deref());
    let all_node_details = projection_all_node_details(map, current_node_id.as_deref());
    let mut node_skeleton = baseline_node_skeleton.clone();
    let mut visible_detail_ids = baseline_node_details
        .iter()
        .map(|event| event.id.clone())
        .collect::<HashSet<_>>();
    let mut full_recoverable_detail_ids = visible_detail_ids.clone();
    let mut folded_node_ids = HashSet::new();
    let mut recoverable_hidden_event_count = 0;
    let mut folded_hidden_event_count = 0;
    let mut expanded_node_count = 0;
    for node in &mut node_skeleton {
        match detail_plan.state(&node.id) {
            Some(NodeDetailState::FoldEligible { .. }) => {
                let hidden_details =
                    hidden_node_details(&node.id, &all_node_details, &baseline_node_details);
                recoverable_hidden_event_count += hidden_details.len();
                full_recoverable_detail_ids
                    .extend(hidden_details.iter().map(|event| event.id.clone()));
                let Some(map_node) = map.nodes.get(&node.id) else {
                    continue;
                };
                let Some((folded, _)) =
                    folded_projection_node(&map.id, &node.id, map_node, &hidden_details)
                else {
                    visible_detail_ids.extend(hidden_details.iter().map(|event| event.id.clone()));
                    continue;
                };
                *node = folded;
                folded_hidden_event_count += hidden_details.len();
                folded_node_ids.insert(node.id.clone());
            }
            Some(NodeDetailState::Expanded { expansion_event_id }) => {
                let hidden_details =
                    hidden_node_details(&node.id, &all_node_details, &baseline_node_details);
                recoverable_hidden_event_count += hidden_details.len();
                visible_detail_ids.extend(hidden_details.iter().map(|event| event.id.clone()));
                full_recoverable_detail_ids
                    .extend(hidden_details.iter().map(|event| event.id.clone()));
                node.detail_state = Some(ProjectionNodeDetailState::Expanded {
                    expansion_event_id: expansion_event_id.clone(),
                });
                expanded_node_count += 1;
            }
            Some(NodeDetailState::Full) | None => {}
        }
    }
    let node_details = all_node_details
        .iter()
        .filter(|event| visible_detail_ids.contains(&event.id))
        .cloned()
        .collect::<Vec<_>>();
    let full_recoverable_node_details = all_node_details
        .iter()
        .filter(|event| full_recoverable_detail_ids.contains(&event.id))
        .cloned()
        .collect::<Vec<_>>();
    let canonical_sha256 = map
        .state_sha256()
        .map_err(|error| format!("canonical_map_hash_failed:{error}"))?;
    let baseline_input = ProjectionInput {
        map_id: map.id.clone(),
        revision: map.revision,
        canonical_sha256: canonical_sha256.clone(),
        root_node_id: map.root_node_id.clone(),
        finish_node_id: map.finish_node_id.clone(),
        complete: map.is_complete(),
        root_source_event_ids: map
            .nodes
            .get(&map.root_node_id)
            .map(|node| node.source_refs.clone())
            .unwrap_or_default(),
        current_node_id: current_node_id.clone(),
        active_frontier: active_frontier.clone(),
        map_nodes: baseline_node_skeleton,
        map_edges: map_edges.clone(),
        node_details: baseline_node_details,
    };
    let b0_projection = render_projection(baseline_input.clone(), envelope);
    let before_strategy = render_projection(
        ProjectionInput {
            node_details: full_recoverable_node_details,
            ..baseline_input
        },
        envelope,
    );
    let rendered = render_projection(
        ProjectionInput {
            map_id: map.id.clone(),
            revision: map.revision,
            canonical_sha256: canonical_sha256.clone(),
            root_node_id: map.root_node_id.clone(),
            finish_node_id: map.finish_node_id.clone(),
            complete: map.is_complete(),
            root_source_event_ids: map
                .nodes
                .get(&map.root_node_id)
                .map(|node| node.source_refs.clone())
                .unwrap_or_default(),
            current_node_id,
            active_frontier,
            map_nodes: node_skeleton,
            map_edges,
            node_details,
        },
        envelope,
    );
    tracing::debug!(
        target: "codex_core::taskspace",
        event_name = "taskspace.projection_rendered",
        map_id = %map.id,
        revision = map.revision,
        canonical_sha256 = %canonical_sha256,
        projection_sha256 = %rendered.projection_sha256,
        projection_bytes = rendered.size_breakdown.projection_bytes,
        "rendered canonical TaskSpace projection"
    );
    let max_projection_tokens = active_budget
        .map(|budget| budget.max_projection_tokens)
        .unwrap_or(usize::MAX);
    let skeleton_over_budget = rendered.skeleton_estimated_tokens > max_projection_tokens;
    let projection_over_budget =
        !skeleton_over_budget && rendered.estimated_tokens > max_projection_tokens;
    if skeleton_over_budget {
        context.push_str("TaskSpaceMapProjectionErrorV1:\n");
        context.push_str("- error: map_skeleton_over_budget\n");
        context.push_str(&format!("- map_id: {}\n", map.id));
        context.push_str(&format!("- node_count: {}\n", map.nodes.len()));
        context.push_str(&format!("- edge_count: {}\n", map.edges.len()));
        context.push_str(&format!(
            "- skeleton_projection_tokens: {}\n",
            rendered.skeleton_estimated_tokens
        ));
        context.push_str(&format!(
            "- max_projection_tokens: {max_projection_tokens}\n"
        ));
    } else if projection_over_budget {
        context.push_str("TaskSpaceMapProjectionErrorV1:\n");
        context.push_str("- error: map_projection_over_budget\n");
        context.push_str(&format!("- map_id: {}\n", map.id));
        context.push_str(&format!(
            "- projection_tokens: {}\n",
            rendered.estimated_tokens
        ));
        context.push_str(&format!(
            "- max_projection_tokens: {max_projection_tokens}\n"
        ));
        context.push_str(&format!("- folded_node_count: {}\n", folded_node_ids.len()));
        context.push_str("- automatic_refold_of_expanded_nodes: false\n");
    } else {
        context.push_str(&rendered.body);
    }
    Ok(ProjectionRenderOutcome {
        stats: ProjectionRenderStats {
            estimated_tokens: if skeleton_over_budget {
                context.len().div_ceil(4)
            } else {
                rendered.estimated_tokens
            },
            skeleton_estimated_tokens: rendered.skeleton_estimated_tokens,
            skeleton_over_budget,
            projection_over_budget,
            size_breakdown: rendered.size_breakdown,
            fold_eligible_node_count: detail_plan.eligible_node_count(),
            folded_node_count: folded_node_ids.len(),
            expanded_node_count,
            recoverable_hidden_event_count,
            folded_hidden_event_count,
            b0_projection_bytes: b0_projection.size_breakdown.projection_bytes,
            b0_skeleton_bytes: b0_projection.size_breakdown.skeleton_bytes,
            projection_bytes_before_strategy: before_strategy.size_breakdown.projection_bytes,
            node_detail_bytes_before_strategy: before_strategy.size_breakdown.node_detail_bytes,
        },
    })
}

struct ProjectionRenderOutcome {
    stats: ProjectionRenderStats,
}

#[derive(Debug, Clone, Copy)]
struct ProjectionRenderStats {
    estimated_tokens: usize,
    skeleton_estimated_tokens: usize,
    skeleton_over_budget: bool,
    projection_over_budget: bool,
    size_breakdown: ProjectionSizeBreakdown,
    fold_eligible_node_count: usize,
    folded_node_count: usize,
    expanded_node_count: usize,
    recoverable_hidden_event_count: usize,
    folded_hidden_event_count: usize,
    b0_projection_bytes: usize,
    b0_skeleton_bytes: usize,
    projection_bytes_before_strategy: usize,
    node_detail_bytes_before_strategy: usize,
}

fn baseline_projection_node_details(
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
) -> Vec<ProjectionEventRef> {
    let distances = projection_graph_distances(map, current_node_id);
    let mut selected_ids = HashSet::new();
    for node_id in ordered_node_ids(map) {
        let Some(node) = map.nodes.get(&node_id) else {
            continue;
        };
        let tier = projection_detail_tier(
            &node_id,
            node,
            current_node_id,
            distances.get(&node_id).copied(),
        );
        let limit = match tier {
            "D1" => 8,
            "D2" => 4,
            _ => 1,
        };
        let events = ordered_node_event_ids(map)
            .into_iter()
            .filter_map(|event_id| map.node_events.get(&event_id))
            .filter(|event| {
                event.node_id == node_id && event.event_kind != NODE_DETAIL_EXPANDED_EVENT_KIND
            })
            .collect::<Vec<_>>();
        for event in events
            .iter()
            .filter(|event| projection_evidence_class(event) == "P0")
        {
            selected_ids.insert(event.id.clone());
        }
        for event in events.iter().rev().take(limit) {
            selected_ids.insert(event.id.clone());
        }
    }
    ordered_node_event_ids(map)
        .into_iter()
        .filter_map(|event_id| map.node_events.get(&event_id))
        .filter(|event| selected_ids.contains(&event.id))
        .map(|event| {
            let tier = map
                .nodes
                .get(&event.node_id)
                .map(|node| {
                    projection_detail_tier(
                        &event.node_id,
                        node,
                        current_node_id,
                        distances.get(&event.node_id).copied(),
                    )
                })
                .unwrap_or("D3");
            projection_event_ref(event, tier, projection_evidence_class(event))
        })
        .collect()
}

fn projection_all_node_details(
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
) -> Vec<ProjectionEventRef> {
    let distances = projection_graph_distances(map, current_node_id);
    ordered_node_event_ids(map)
        .into_iter()
        .filter_map(|event_id| map.node_events.get(&event_id))
        .filter(|event| event.event_kind != NODE_DETAIL_EXPANDED_EVENT_KIND)
        .map(|event| {
            let tier = map
                .nodes
                .get(&event.node_id)
                .map(|node| {
                    projection_detail_tier(
                        &event.node_id,
                        node,
                        current_node_id,
                        distances.get(&event.node_id).copied(),
                    )
                })
                .unwrap_or("D3");
            projection_event_ref(event, tier, projection_evidence_class(event))
        })
        .collect()
}

fn hidden_node_details(
    node_id: &str,
    all_details: &[ProjectionEventRef],
    baseline_details: &[ProjectionEventRef],
) -> Vec<ProjectionEventRef> {
    let visible_ids = baseline_details
        .iter()
        .filter(|event| event.node_id == node_id)
        .map(|event| event.id.as_str())
        .collect::<HashSet<_>>();
    all_details
        .iter()
        .filter(|event| event.node_id == node_id && !visible_ids.contains(event.id.as_str()))
        .cloned()
        .collect()
}

fn projection_graph_distances(
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
) -> HashMap<String, usize> {
    let Some(current_node_id) = current_node_id.filter(|node_id| map.nodes.contains_key(*node_id))
    else {
        return HashMap::new();
    };
    let mut distances = HashMap::from([(current_node_id.to_string(), 0)]);
    let mut queue = VecDeque::from([current_node_id.to_string()]);
    while let Some(node_id) = queue.pop_front() {
        let next_distance = distances[&node_id] + 1;
        for adjacent in map.edges.iter().filter_map(|edge| {
            if edge.from == node_id {
                Some(edge.to.as_str())
            } else if edge.to == node_id {
                Some(edge.from.as_str())
            } else {
                None
            }
        }) {
            if !distances.contains_key(adjacent) {
                distances.insert(adjacent.to_string(), next_distance);
                queue.push_back(adjacent.to_string());
            }
        }
    }
    distances
}

fn projection_detail_tier(
    node_id: &str,
    node: &MapNode,
    current_node_id: Option<&str>,
    distance: Option<usize>,
) -> &'static str {
    if current_node_id == Some(node_id)
        || node.status != NodeStatus::Completed
        || distance.is_some_and(|distance| distance <= 1)
    {
        "D1"
    } else if distance == Some(2) {
        "D2"
    } else {
        "D3"
    }
}

fn projection_evidence_class(event: &NodeEvent) -> &'static str {
    if event.tool_success == Some(false) || event.source == "runtime_feedback" {
        "P0"
    } else if matches!(
        event.action_class,
        Some(ActionClass::Read | ActionClass::Edit | ActionClass::Test)
    ) {
        "P1"
    } else if event.tool_success.is_some() {
        "P2"
    } else {
        "P3"
    }
}

fn projection_event_ref(
    event: &NodeEvent,
    detail_tier: &str,
    evidence_class: &str,
) -> ProjectionEventRef {
    ProjectionEventRef {
        id: event
            .source_event_id
            .clone()
            .unwrap_or_else(|| event.id.clone()),
        node_id: event.node_id.clone(),
        event_kind: event.event_kind.clone(),
        source: event.source.clone(),
        detail_tier: detail_tier.to_string(),
        evidence_class: evidence_class.to_string(),
        action_class: event
            .action_class
            .map(|action_class| action_class.as_str().to_string()),
        tool_success: event.tool_success,
        content_sha256: Some(event.content_sha256.clone()),
        raw_ref: event.raw_ref.clone(),
        artifact_refs: event.artifact_refs.clone(),
    }
}

fn ordered_node_ids(map: &ActionMapInstance) -> Vec<MapNodeId> {
    let mut indegree = map
        .nodes
        .keys()
        .map(|node_id| (node_id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for edge in &map.edges {
        if let Some(count) = indegree.get_mut(&edge.to) {
            *count += 1;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(map.nodes.len());
    while let Some(node_id) = ready.pop_first() {
        ordered.push(node_id.clone());
        for edge in map.edges.iter().filter(|edge| edge.from == node_id) {
            let count = indegree
                .get_mut(&edge.to)
                .expect("edge target belongs to canonical map");
            *count -= 1;
            if *count == 0 {
                ready.insert(edge.to.clone());
            }
        }
    }
    for node_id in map.nodes.keys() {
        if !ordered.contains(node_id) {
            ordered.push(node_id.clone());
        }
    }
    ordered
}

fn ordered_node_event_ids(map: &ActionMapInstance) -> Vec<NodeEventId> {
    let mut event_ids = map.node_events.keys().cloned().collect::<Vec<_>>();
    event_ids
        .sort_by(|left, right| node_event_id_sort_key(left).cmp(&node_event_id_sort_key(right)));
    event_ids
}

fn next_numeric_seq<'a>(ids: impl Iterator<Item = &'a String>, prefix: &str) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn node_role_from_str(role: &str) -> Option<NodeRole> {
    match role {
        "task_root" => Some(NodeRole::TaskRoot),
        "work" => Some(NodeRole::Work),
        "finish" => Some(NodeRole::Finish),
        _ => None,
    }
}

fn node_status_from_str(status: &str) -> Option<NodeStatus> {
    match status {
        "open" => Some(NodeStatus::Open),
        "closed" => Some(NodeStatus::Closed),
        "pending" => Some(NodeStatus::Pending),
        "ready" => Some(NodeStatus::Ready),
        "running" => Some(NodeStatus::Running),
        "blocked" => Some(NodeStatus::Blocked),
        "completed" => Some(NodeStatus::Completed),
        _ => None,
    }
}

fn lease_holder_from_str(holder: &str) -> Option<LeaseHolder> {
    match holder {
        "main" => Some(LeaseHolder::Main),
        "subagent" => Some(LeaseHolder::SubAgent),
        _ => None,
    }
}

fn node_result_kind_from_str(kind: &str) -> Option<NodeResultKind> {
    match kind {
        "result" => Some(NodeResultKind::Result),
        "blocker" => Some(NodeResultKind::Blocker),
        "map_update_request" => Some(NodeResultKind::MapUpdateRequest),
        "timeout_summary" => Some(NodeResultKind::TimeoutSummary),
        "main_tool_call" => Some(NodeResultKind::MainToolCall),
        _ => None,
    }
}

#[allow(dead_code)]
fn maintenance_barrier_reason_from_str(reason: &str) -> Option<MaintenanceBarrierReason> {
    match reason {
        "node_tool_result_budget_exceeded" => {
            Some(MaintenanceBarrierReason::NodeToolResultBudgetExceeded)
        }
        _ => None,
    }
}

fn node_event_id_sort_key(event_id: &str) -> (u8, u64, &str) {
    if let Some(number) = event_id
        .strip_prefix("node-event-")
        .and_then(|suffix| suffix.parse::<u64>().ok())
    {
        return (0, number, event_id);
    }
    (1, 0, event_id)
}

fn require_nonempty(field: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("TaskSpace {field} cannot be empty."));
    }
    Ok(value.to_string())
}

fn require_nonempty_owned(field: &str, value: String) -> Result<String, String> {
    require_nonempty(field, &value)
}

fn snapshot_maintenance_barrier(
    barrier: &ActionMapMaintenanceBarrier,
) -> ActionMapSnapshotMaintenanceBarrier {
    ActionMapSnapshotMaintenanceBarrier {
        map_id: barrier.map_id.clone(),
        node_id: barrier.node_id.clone(),
        reason: barrier.reason.as_str().to_string(),
        result_count: barrier.result_count,
        budget: barrier.budget,
    }
}

fn snapshot_trace_event_ref(event: &TaskSpaceTraceEvent) -> ActionMapSnapshotTraceEventRef {
    ActionMapSnapshotTraceEventRef {
        id: event.id.clone(),
        kind: event.kind.clone(),
        task_id: event.task_id.clone(),
        map_id: event.map_id.clone(),
        node_id: event.node_id.clone(),
        result_id: event.result_id.clone(),
        call_id: event.call_id.clone(),
        action_class: event.action_class.map(|class| class.as_str().to_string()),
        tool_success: event.tool_success,
        tags: event.tags.clone(),
        artifact_refs: event.artifact_refs.clone(),
        created_at_ms: event.created_at_ms,
    }
}

fn snapshot_sentinel_warning_ref(
    warning: &TaskSpaceSentinelWarning,
) -> ActionMapSnapshotSentinelWarningRef {
    ActionMapSnapshotSentinelWarningRef {
        id: warning.id.clone(),
        sentinel_type: warning.warning_type.as_str().to_string(),
        status: warning.status.as_str().to_string(),
        severity: warning.severity.as_str().to_string(),
        task_id: warning.task_id.clone(),
        map_id: warning.map_id.clone(),
        node_id: warning.node_id.clone(),
        result_id: warning.result_id.clone(),
        trace_event_ids: warning.trace_event_ids.clone(),
        reason: warning.reason.clone(),
        clearance_action: warning.clearance_action.clone(),
        clear_action: warning.clear_action.clone(),
        created_at_ms: warning.created_at_ms,
        cleared_at_ms: warning.cleared_at_ms,
    }
}

fn trace_summary(events: &[ActionMapSnapshotTraceEventRef]) -> ActionMapSnapshotTraceSummary {
    ActionMapSnapshotTraceSummary {
        total_event_count: events.len(),
        tool_call_count: events
            .iter()
            .filter(|event| event.kind == "main_tool_result")
            .count(),
        failed_tool_call_count: events
            .iter()
            .filter(|event| event.tool_success == Some(false))
            .count(),
        validator_failure_count: events
            .iter()
            .filter(|event| event.tags.iter().any(|tag| tag == "validator_failure"))
            .count(),
        unclassified_shell_action_count: events
            .iter()
            .filter(|event| {
                event
                    .tags
                    .iter()
                    .any(|tag| tag == "unclassified_shell_action")
            })
            .count(),
    }
}

fn sentinel_summary(
    warnings: &[ActionMapSnapshotSentinelWarningRef],
) -> ActionMapSnapshotSentinelSummary {
    ActionMapSnapshotSentinelSummary {
        total_warning_count: warnings.len(),
        active_warning_count: warnings
            .iter()
            .filter(|warning| warning.status == "active")
            .count(),
        validator_failure_warning_count: warnings
            .iter()
            .filter(|warning| warning.sentinel_type == "validator_failure")
            .count(),
        unclassified_shell_warning_count: warnings
            .iter()
            .filter(|warning| warning.sentinel_type == "unclassified_shell_action")
            .count(),
    }
}

fn sentinel_warning_raised_event(
    warning: &TaskSpaceSentinelWarning,
) -> MapRuntimeSentinelWarningRaisedEvent {
    MapRuntimeSentinelWarningRaisedEvent {
        sentinel_id: warning.id.clone(),
        sentinel_type: warning.warning_type.as_str().to_string(),
        status: warning.status.as_str().to_string(),
        severity: warning.severity.as_str().to_string(),
        task_id: warning.task_id.clone(),
        map_id: warning.map_id.clone(),
        node_id: warning.node_id.clone(),
        result_id: warning.result_id.clone(),
        trace_event_ids: warning.trace_event_ids.clone(),
        reason: warning.reason.clone(),
        clearance_action: warning.clearance_action.clone(),
        created_at_ms: warning.created_at_ms,
    }
}

fn trace_tags_for(action_class: Option<ActionClass>, success: bool, _body: &str) -> Vec<String> {
    let mut tags = vec![if success {
        "tool_success".to_string()
    } else {
        "tool_failure".to_string()
    }];
    if matches!(action_class, Some(ActionClass::Unknown) | None) {
        tags.push("unclassified_tool_action".to_string());
    }
    tags
}

fn trace_tag_value<'a>(tags: &'a [String], key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    tags.iter()
        .find_map(|tag| tag.strip_prefix(prefix.as_str()))
}

fn trace_tag_usize(tags: &[String], key: &str) -> Option<usize> {
    trace_tag_value(tags, key).and_then(|value| value.parse::<usize>().ok())
}

fn provider_request_adoption_blockers(
    snapshot: &ActionMapProviderRequestBudgetSnapshot,
    request_phase: &str,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if request_phase == "unknown" {
        blockers.push("request_phase_unknown".to_string());
    }
    if let Some(reason) = snapshot.provider_request_context_missing_reason.as_deref() {
        blockers.push(format!(
            "provider_context_missing:{}",
            sanitize_provider_response_trace_tag_value(reason)
        ));
    }
    if blockers.is_empty() {
        blockers.push("none".to_string());
    }
    blockers
}

fn provider_request_trigger_kind(
    _snapshot: &ActionMapProviderRequestBudgetSnapshot,
    input: &ActionMapProviderRequestBudgetEventInput,
    request_phase: &str,
    response_actionability_previous: &str,
    adoption_blockers: &[String],
) -> String {
    if input.status == "blocked" {
        return "hard_baseline_stop".to_string();
    }
    if request_phase == "budget_recovery" {
        return "budget_recovery".to_string();
    }
    if matches!(
        response_actionability_previous,
        "no_action_follow_up" | "tool_feedback_recovery"
    ) {
        return "response_recovery".to_string();
    }
    if adoption_blockers.iter().any(|blocker| blocker != "none") {
        return "open_adoption_blocker".to_string();
    }
    if request_phase == "final_synthesis" {
        return "final_synthesis".to_string();
    }
    if request_phase == "unknown" {
        return "unknown".to_string();
    }
    "model_sampling".to_string()
}

fn provider_request_reason_confidence(
    request_phase: &str,
    trigger_kind: &str,
    previous_actionability: &Option<ProviderResponseActionabilityTrace>,
    adoption_blockers: &[String],
) -> String {
    if trigger_kind == "unknown" || request_phase == "unknown" {
        return "unknown".to_string();
    }
    if previous_actionability.is_some() || adoption_blockers.iter().any(|blocker| blocker != "none")
    {
        return "direct".to_string();
    }
    "derived".to_string()
}

fn provider_request_reason_join(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .map(|value| sanitize_provider_response_trace_tag_value(value))
        .collect::<Vec<_>>()
        .join("|")
}

fn map_runtime_event_from_trace_event(event: TaskSpaceTraceEvent) -> MapRuntimeEvent {
    MapRuntimeEvent::TaskspaceTraceEventRecorded(MapRuntimeTraceEventRecordedEvent {
        trace_event_id: event.id,
        kind: event.kind,
        task_id: event.task_id,
        map_id: event.map_id,
        node_id: event.node_id,
        result_id: event.result_id,
        call_id: event.call_id,
        action_class: event.action_class.map(|action| action.as_str().to_string()),
        tool_success: event.tool_success,
        tags: event.tags,
        artifact_refs: event.artifact_refs,
        created_at_ms: event.created_at_ms,
    })
}

fn sanitize_provider_response_trace_tag_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\r' | '\n' | '\t' => ' ',
            _ => ch,
        })
        .take(160)
        .collect::<String>()
}

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let mut nodes = map
        .nodes
        .iter()
        .map(|(node_id, node)| ActionMapSnapshotNode {
            id: node_id.clone(),
            role: node.role.as_str().to_string(),
            goal: node.goal.clone(),
            status: node.status.as_str().to_string(),
            source_refs: node.source_refs.clone(),
            active_lease: node.active_lease.clone(),
            result_ids: node
                .result_context
                .iter()
                .map(|result| result.id.clone())
                .collect(),
            node_event_ids: node
                .node_events
                .iter()
                .map(|event| event.id.clone())
                .collect(),
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    let mut leases = map
        .leases
        .values()
        .map(|lease| ActionMapSnapshotLease {
            id: lease.id.clone(),
            map_id: lease.map_id.clone(),
            node_id: lease.node_id.clone(),
            holder: lease.holder.as_str().to_string(),
            previous_node_status: lease.previous_node_status.as_str().to_string(),
            agent_thread_id: lease.agent_thread_id,
            agent_path: lease.agent_path.clone(),
        })
        .collect::<Vec<_>>();
    leases.sort_by(|left, right| left.id.cmp(&right.id));

    let mut results = map
        .results
        .values()
        .map(|result| ActionMapSnapshotResult {
            id: result.id.clone(),
            assignment_id: result.assignment_id.clone(),
            map_id: result.map_id.clone(),
            node_id: result.node_id.clone(),
            kind: result.kind.as_str().to_string(),
            action_class: result.action_class.map(|class| class.as_str().to_string()),
            tool_success: result.tool_success,
            source_event_ref: result.source_event_ref.clone(),
            artifact_refs: result.artifact_refs.clone(),
            source_thread_id: result.source_thread_id,
            created_at_ms: result.created_at_ms,
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| left.id.cmp(&right.id));

    let mut node_events = map
        .node_events
        .values()
        .map(|event| ActionMapSnapshotNodeEvent {
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
        })
        .collect::<Vec<_>>();
    node_events.sort_by(|left, right| {
        node_event_id_sort_key(&left.id).cmp(&node_event_id_sort_key(&right.id))
    });

    ActionMapSnapshotMap {
        id: map.id.clone(),
        task_id: map.task_id.clone(),
        owner_session_id: map.owner_session_id,
        root_node_id: map.root_node_id.clone(),
        finish_node_id: map.finish_node_id.clone(),
        revision: map.revision,
        current_node_id: map.current_binding.clone(),
        terminal_summary_ref: map.terminal_summary_ref.clone(),
        complete: map.is_complete(),
        ready_work_node_count: map.ready_work_node_count(),
        running_work_node_count: map.running_work_node_count(),
        completed_work_node_count: map.completed_work_node_count(),
        finish_ready: map
            .nodes
            .get(&map.finish_node_id)
            .is_some_and(|node| node.status == NodeStatus::Ready),
        nodes,
        edges: map
            .edges
            .iter()
            .map(|edge| ActionMapSnapshotEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
            })
            .collect(),
        leases,
        results,
        node_events,
    }
}

fn validate_active_frontier_leases(map: &ActionMapInstance) -> Result<(), String> {
    let mut leased_node_ids = BTreeSet::new();
    let mut main_lease_node_id = None;
    for lease in map.leases.values() {
        let node = map.nodes.get(&lease.node_id).ok_or_else(|| {
            format!(
                "TaskSpace lease `{}` references missing node `{}`.",
                lease.id, lease.node_id
            )
        })?;
        if node.role != NodeRole::Work || node.status != NodeStatus::Running {
            return Err(format!(
                "TaskSpace lease `{}` does not reference a running work node.",
                lease.id
            ));
        }
        if node.active_lease.as_deref() != Some(lease.id.as_str())
            || !leased_node_ids.insert(lease.node_id.clone())
        {
            return Err(format!(
                "TaskSpace lease `{}` is inconsistent with node `{}`.",
                lease.id, lease.node_id
            ));
        }
        if lease.holder == LeaseHolder::Main
            && (main_lease_node_id.replace(lease.node_id.clone()).is_some()
                || lease.agent_thread_id != map.owner_session_id)
        {
            return Err("TaskSpace main lease owner is inconsistent.".to_string());
        }
    }
    for (node_id, node) in &map.nodes {
        if node.role != NodeRole::Work && node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace non-work node `{node_id}` cannot hold an active lease."
            ));
        }
        if node.role == NodeRole::Work
            && ((node.status == NodeStatus::Running) != node.active_lease.is_some())
        {
            return Err(format!(
                "TaskSpace work node `{node_id}` has inconsistent running and lease state."
            ));
        }
        if let Some(lease_id) = node.active_lease.as_ref()
            && !map.leases.contains_key(lease_id)
        {
            return Err(format!(
                "TaskSpace node `{node_id}` references missing lease `{lease_id}`."
            ));
        }
    }
    if map.current_binding != main_lease_node_id {
        return Err("TaskSpace current binding and main lease are inconsistent.".to_string());
    }
    Ok(())
}

fn single_line_preview(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn format_node_candidates(map: &ActionMapInstance, node_ids: &[MapNodeId]) -> String {
    node_ids
        .iter()
        .map(|node_id| {
            let goal = map
                .nodes
                .get(node_id)
                .map(|node| single_line_preview(&node.goal, 80))
                .unwrap_or_default();
            format!("{node_id} ({goal})")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn budget_state_for_counter(counter_value: usize, counter_limit: usize) -> TaskSpaceBudgetState {
    if counter_limit == 0 || counter_value >= counter_limit {
        return TaskSpaceBudgetState::OverProfileHint;
    }
    let remaining = counter_limit.saturating_sub(counter_value);
    if remaining <= 1 {
        return TaskSpaceBudgetState::CompactCheckpointRequired;
    }
    if counter_value.saturating_mul(4) >= counter_limit.saturating_mul(3) {
        return TaskSpaceBudgetState::ThinDowngraded;
    }
    if counter_value.saturating_mul(2) >= counter_limit {
        return TaskSpaceBudgetState::Warned;
    }
    TaskSpaceBudgetState::Normal
}

fn merge_artifact_refs(mut left: Vec<String>, right: Vec<String>) -> Vec<String> {
    for artifact in right {
        if !left.iter().any(|existing| existing == &artifact) {
            left.push(artifact);
        }
    }
    left
}

fn tool_action_descriptor_artifact_refs(descriptor: &ToolActionDescriptor) -> Vec<String> {
    match descriptor.action_class {
        ActionClass::Read => descriptor
            .preview
            .parse::<serde_json::Value>()
            .ok()
            .and_then(|value| {
                value
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .and_then(read_command_artifact_ref)
            })
            .into_iter()
            .collect(),
        ActionClass::Edit => extract_edit_changed_artifacts_from_tool_body(&descriptor.preview),
        _ => Vec::new(),
    }
}

fn result_body_command_from_body(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.strip_prefix("command: ").map(str::trim))
        .map(str::to_string)
}

fn artifact_refs_equivalent(left: &str, right: &str) -> bool {
    normalize_artifact_ref(left).eq_ignore_ascii_case(&normalize_artifact_ref(right))
}

fn push_unique_artifact_ref(artifact_refs: &mut Vec<String>, artifact_ref: String) {
    if !artifact_ref.is_empty()
        && !artifact_refs
            .iter()
            .any(|existing| artifact_refs_match(existing, &artifact_ref))
    {
        artifact_refs.push(artifact_ref);
    }
}

fn artifact_refs_match(left: &str, right: &str) -> bool {
    if artifact_refs_equivalent(left, right) {
        return true;
    }
    let left_variants = artifact_command_match_variants(left);
    let right_variants = artifact_command_match_variants(right);
    left_variants
        .iter()
        .any(|left| right_variants.iter().any(|right| left == right))
}

fn artifact_command_match_variants(artifact: &str) -> Vec<String> {
    let normalized = normalize_artifact_ref(artifact).to_ascii_lowercase();
    let mut variants = vec![normalized.clone()];
    for marker in ["/app/src/", "/app/", "app/src/", "app/"] {
        if let Some((_, suffix)) = normalized.split_once(marker) {
            variants.push(suffix.trim_matches('/').to_string());
        }
    }
    if let Some(file_name) = normalized.rsplit('/').next()
        && file_name.len() >= 4
    {
        variants.push(file_name.to_string());
    }
    variants.sort();
    variants.dedup();
    variants
}

fn read_command_artifact_ref(command: &str) -> Option<String> {
    if command.contains("Get-Content") {
        return powershell_path_arg(command)
            .as_deref()
            .map(normalize_artifact_ref);
    }
    if let Some(path) = sed_read_command_artifact_ref(command) {
        return Some(path);
    }
    command
        .strip_prefix("cat ")
        .or_else(|| command.strip_prefix("type "))
        .map(str::trim)
        .map(|path| normalize_artifact_ref(path.trim_matches('"').trim_matches('\'')))
}

fn sed_read_command_artifact_ref(command: &str) -> Option<String> {
    let tokens = shlex::split(command)?;
    if tokens.first().map(String::as_str) != Some("sed") {
        return None;
    }

    let mut paths = Vec::new();
    let mut tokens = tokens.iter().skip(1).peekable();
    let mut after_double_dash = false;
    while let Some(token) = tokens.next() {
        if matches!(token.as_str(), "&&" | ";" | "||") {
            break;
        }
        if after_double_dash {
            paths.push(token.as_str());
            continue;
        }
        if token == "--" {
            after_double_dash = true;
            continue;
        }
        if token == "-n" {
            let _ = tokens.next();
            continue;
        }
        if token.starts_with("-n") {
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        paths.push(token.as_str());
    }

    (paths.len() == 1).then(|| normalize_artifact_ref(paths[0]))
}

fn powershell_path_arg(command: &str) -> Option<String> {
    let marker = if command.contains("-LiteralPath") {
        "-LiteralPath"
    } else if command.contains("-Path") {
        "-Path"
    } else {
        return None;
    };
    let start = command.find(marker)? + marker.len();
    let rest = command[start..].trim_start();
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn extract_edit_changed_artifacts_from_tool_body(body: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    let normalized_body = body.replace("\\n", "\n");
    for line in normalized_body.lines() {
        let trimmed = line.trim();
        let Some(path) = trimmed
            .strip_prefix("M ")
            .or_else(|| trimmed.strip_prefix("A "))
            .or_else(|| trimmed.strip_prefix("D "))
            .or_else(|| trimmed.strip_prefix("*** Update File: "))
            .or_else(|| trimmed.strip_prefix("*** Add File: "))
            .or_else(|| trimmed.strip_prefix("+++ b/"))
            .or_else(|| trimmed.strip_prefix("+++ "))
        else {
            continue;
        };
        let path = path.trim();
        if path.is_empty()
            || path.starts_with("user-request")
            || path.starts_with("output-ref://")
            || path == "/dev/null"
            || path.contains('\0')
        {
            continue;
        }
        let path = normalize_artifact_ref(path);
        if !artifacts.iter().any(|existing| existing == &path) {
            artifacts.push(path);
        }
    }
    artifacts
}

fn normalize_artifact_ref(path: &str) -> String {
    path.trim()
        .strip_prefix("path=")
        .unwrap_or(path.trim())
        .trim_start_matches("a/")
        .trim_start_matches("b/")
        .trim_start_matches("./")
        .replace('\\', "/")
}

fn graph_revision_committed_event(batch: &EventBatch, operation: &str) -> MapRuntimeEvent {
    MapRuntimeEvent::GraphRevisionCommitted(graph_revision_committed_record(batch, operation))
}

fn graph_revision_committed_record(
    batch: &EventBatch,
    operation: &str,
) -> MapRuntimeGraphRevisionCommittedEvent {
    MapRuntimeGraphRevisionCommittedEvent {
        map_id: batch.map_id.clone(),
        revision: batch.revision,
        operation: operation.to_string(),
        event_ids: batch
            .records
            .iter()
            .map(|record| record.event_id.clone())
            .collect(),
        events: batch
            .records
            .iter()
            .map(|record| {
                serde_json::to_value(&record.event)
                    .expect("rooted DAG domain event is serializable")
            })
            .collect(),
    }
}

fn result_body_taskspace_marker_artifact_refs(body: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    for line in result_body_raw_output_section(body).lines().map(str::trim) {
        if !(line.starts_with("TaskSpaceReadFileSummaryV1:")
            || line.starts_with("TaskSpaceStructuredFilePreviewV1:"))
        {
            continue;
        }
        let Some(path) = taskspace_marker_field(line, "path") else {
            continue;
        };
        let artifact_ref = normalize_artifact_ref(&path);
        if !artifact_ref.is_empty() && !artifacts.iter().any(|existing| existing == &artifact_ref) {
            artifacts.push(artifact_ref);
        }
    }
    artifacts
}

fn result_body_raw_output_section(body: &str) -> &str {
    body.split_once("\nraw_output:\n")
        .map(|(_, rest)| rest)
        .or_else(|| body.split_once("\r\nraw_output:\r\n").map(|(_, rest)| rest))
        .unwrap_or(body)
}

fn first_output_ref_in_text(body: &str) -> Option<String> {
    body.split_whitespace()
        .find(|token| token.starts_with("output-ref://"))
        .map(|token| token.trim_matches(|ch: char| matches!(ch, ',' | ';' | ')' | ']' | '}')))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

fn taskspace_marker_field(line: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}=");
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
        .map(|value| value.trim_matches(['"', '\'']).to_string())
        .filter(|value| !value.is_empty())
}

fn assignment_prompt(map_id: &str, node_id: &str, node_goal: &str, lease_id: &str) -> String {
    format!(
        "TaskSpace node assignment\n\
Map: {map_id}\n\
Node: {node_id}\n\
Goal: {node_goal}\n\
Lease: {lease_id}\n",
    )
}

fn child_tool_reservation_key(child_thread_id: ThreadId, call_id: &str) -> String {
    format!("{child_thread_id}:{call_id}")
}

fn child_thread_source_event_ref(child_thread_id: ThreadId) -> String {
    format!("thread:{child_thread_id}")
}

fn child_tool_source_event_ref(child_thread_id: ThreadId, call_id: &str) -> String {
    format!(
        "{}/call:{call_id}",
        child_thread_source_event_ref(child_thread_id)
    )
}

fn tool_result_artifact_refs(
    action_class: Option<ActionClass>,
    success: bool,
    body: &str,
) -> Vec<String> {
    if !success {
        return Vec::new();
    }
    match action_class {
        Some(ActionClass::Edit) => extract_edit_changed_artifacts_from_tool_body(body),
        Some(ActionClass::Read) => {
            let mut artifact_refs = result_body_taskspace_marker_artifact_refs(body);
            if let Some(command) = result_body_command_from_body(body)
                && let Some(artifact_ref) = read_command_artifact_ref(&command)
            {
                push_unique_artifact_ref(&mut artifact_refs, artifact_ref);
            }
            artifact_refs
        }
        _ => Vec::new(),
    }
}

fn result_kind_from_status(status: &AgentStatus) -> NodeResultKind {
    match status {
        AgentStatus::Completed(_) => NodeResultKind::Result,
        AgentStatus::Errored(_)
        | AgentStatus::Shutdown
        | AgentStatus::NotFound
        | AgentStatus::Interrupted
        | AgentStatus::PendingInit
        | AgentStatus::Running => NodeResultKind::Blocker,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn transition_notice(
    previous_mode: MapRuntimeMode,
    current_mode: MapRuntimeMode,
) -> Option<String> {
    match (previous_mode, current_mode) {
        (MapRuntimeMode::Standard, MapRuntimeMode::Experiment) => None,
        (MapRuntimeMode::Experiment, MapRuntimeMode::Standard) => Some(
            "TaskSpace mode is now disabled.\n\
Existing task paths, nodes, leases, and results remain historical context only.\n\
TaskSpace maintenance status: disabled. Task paths no longer impose node binding or task-driven protocol unless the user enables TaskSpace again.\n\
Active runtime mode: standard Codex multi-agent behavior."
                .to_string(),
        ),
        _ => Some(format!(
            "TaskSpace runtime mode changed from {previous_mode} to {current_mode}."
        )),
    }
}

#[cfg(test)]
#[path = "runtime_phase_d_tests.rs"]
mod phase_d_tests;
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
