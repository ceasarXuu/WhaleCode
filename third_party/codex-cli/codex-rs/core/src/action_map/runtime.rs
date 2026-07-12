use std::collections::HashMap;
use std::collections::HashSet;
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
use codex_protocol::protocol::ActionMapSnapshotTask;
use codex_protocol::protocol::ActionMapSnapshotTraceEventRef;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeLeaseAttachedEvent;
use codex_protocol::protocol::MapRuntimeLeaseCreatedEvent;
use codex_protocol::protocol::MapRuntimeLeaseReleasedEvent;
use codex_protocol::protocol::MapRuntimeMaintenanceBarrierClearedEvent;
use codex_protocol::protocol::MapRuntimeMapCreatedEvent;
use codex_protocol::protocol::MapRuntimeMapStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeNodeEventRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeResultRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeSentinelWarningRaisedEvent;
use codex_protocol::protocol::MapRuntimeTaskCreatedEvent;
use codex_protocol::protocol::MapRuntimeTaskStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTimeoutSummaryRequestedEvent;
use codex_protocol::protocol::MapRuntimeTraceEventRecordedEvent;

use super::map::ActionClass;
use super::map::ActionMapId;
use super::map::ActionMapInstance;
use super::map::AssignmentLease;
use super::map::AssignmentLeaseId;
use super::map::LeaseHolder;
use super::map::MapEdge;
use super::map::MapNode;
use super::map::MapNodeId;
use super::map::MapStatus;
use super::map::NodeContext;
use super::map::NodeEvent;
use super::map::NodeEventId;
use super::map::NodeEventRef;
use super::map::NodeKind;
use super::map::NodeResult;
use super::map::NodeResultId;
use super::map::NodeResultKind;
use super::map::NodeResultRef;
use super::map::NodeStatus;
use super::map::TaskId;
use super::map::TaskSpaceTraceEvent;
use super::map::TaskState;
use super::map::TaskStatus;
use super::map::ToolActionDescriptor;
use super::projection::ActiveProjectionInput;
use super::projection::ProjectionEdge;
use super::projection::ProjectionEventRef;
use super::projection::ProjectionNode;
use super::projection::render_active_projection;
use super::sentinel::TaskSpaceSentinelSeverity;
use super::sentinel::TaskSpaceSentinelWarning;
use super::sentinel::TaskSpaceSentinelWarningStatus;
use super::sentinel::TaskSpaceSentinelWarningType;

const TASKSPACE_MECHANICAL_BLANK_TASK_TITLE: &str = "TaskSpace blank task";
const TASKSPACE_MECHANICAL_BLANK_MAP_TITLE: &str = "TaskSpace blank map";
const TASKSPACE_MAP_SCHEMA_VERSION: &str = "taskspace-map-v1";
use super::sentinel::warning_drafts_for_trace_event;

const SEED_NODE_IDS: &[&str] = &[
    "define_scope",
    "inspect_code_context",
    "design_solution",
    "implement_solution",
    "smoke_test",
    "final_synthesis",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceHardGateClass {
    StateMachine,
    Protocol,
    Resource,
}

impl TaskSpaceHardGateClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::StateMachine => "state_machine",
            Self::Protocol => "protocol",
            Self::Resource => "resource",
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
pub(crate) struct ActionMapNextNodeDraft {
    pub(crate) kind: NodeKind,
    pub(crate) title: String,
    pub(crate) context_summary: String,
    pub(crate) dependency_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapFinishNodeOutcome {
    pub(crate) result_id: NodeResultId,
    pub(crate) next_node_id: Option<MapNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeNodeInput {
    pub(crate) id: String,
    pub(crate) kind: NodeKind,
    pub(crate) title: String,
    pub(crate) context_summary: String,
    pub(crate) dependency_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeInput {
    pub(crate) task_title: String,
    pub(crate) source_event_ids: Vec<String>,
    pub(crate) nodes: Vec<ActionMapInitializeNodeInput>,
    pub(crate) current_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInitializeOutcome {
    pub(crate) task_id: TaskId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_ids: Vec<MapNodeId>,
    pub(crate) current_node_id: MapNodeId,
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
    pub(crate) node_kind: Option<String>,
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
    pub(crate) active_projection_present: bool,
    pub(crate) active_projection_count: usize,
    pub(crate) large_raw_output_tokens: usize,
    pub(crate) runtime_boundary_forbidden_markers: Vec<String>,
    pub(crate) protected_items_present: bool,
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
    tasks: HashMap<TaskId, TaskState>,
    maps: HashMap<ActionMapId, ActionMapInstance>,
    taskspace_trace_events: Vec<TaskSpaceTraceEvent>,
    provider_request_count: usize,
    active_budget: Option<TaskSpaceActiveBudgetV1>,
    budget_counters: TaskSpaceBudgetCounters,
    blocked_action_repeats: HashMap<String, usize>,
    budget_state: TaskSpaceBudgetState,
    budget_violations: Vec<TaskSpaceBudgetViolation>,
    sentinel_warnings: Vec<TaskSpaceSentinelWarning>,
    next_task_seq: u64,
    next_map_seq: u64,
    next_node_seq: u64,
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
            provider_request_count: 0,
            active_budget: None,
            budget_counters: TaskSpaceBudgetCounters::default(),
            blocked_action_repeats: HashMap::new(),
            budget_state: TaskSpaceBudgetState::Normal,
            budget_violations: Vec::new(),
            sentinel_warnings: Vec::new(),
            next_task_seq: 1,
            next_map_seq: 1,
            next_node_seq: 1,
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

    pub(crate) fn gate_create_node_budget(
        &mut self,
        map_id: &str,
        _candidate_node_kind: NodeKind,
    ) -> TaskSpaceBudgetGateDecision {
        let budget = self.active_budget.as_ref();
        let max_nodes = budget
            .map(|budget| budget.max_nodes)
            .unwrap_or(Self::DEFAULT_ACTIVE_NODE_BUDGET_MAX);
        let node_count = self
            .maps
            .get(map_id)
            .map(|map| map.nodes.len())
            .unwrap_or_default();
        TaskSpaceBudgetGateDecision {
            allowed: true,
            budget_state: self.budget_state,
            reason: if node_count < max_nodes {
                "node_budget_available".to_string()
            } else {
                "node_profile_hint_exceeded".to_string()
            },
            blocking_items: Vec::new(),
            next_valid_actions: Vec::new(),
            recovery_request_phase: None,
            quality_impact_required: false,
        }
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
                self.bootstrap_required = self.tasks.is_empty();
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
        if self.mode != MapRuntimeMode::Experiment || self.active_map().is_some() {
            return Vec::new();
        }

        let task_id = self.next_task_id();
        let map_id = self.next_map_id();
        let task = TaskState {
            id: task_id.clone(),
            title: TASKSPACE_MECHANICAL_BLANK_TASK_TITLE.to_string(),
            source_event_ids: Vec::new(),
            status: TaskStatus::Active,
            owner_session_id: Some(owner_session_id),
            active_map_id: None,
            map_ids: Vec::new(),
        };
        let mut map = ActionMapInstance::new(
            map_id.clone(),
            TASKSPACE_MECHANICAL_BLANK_MAP_TITLE.to_string(),
            Some(owner_session_id),
            TASKSPACE_MAP_SCHEMA_VERSION,
        );
        map.task_id = Some(task_id.clone());
        self.tasks.insert(task_id.clone(), task);
        self.maps.insert(map_id.clone(), map);
        self.register_map_to_task(&task_id, &map_id);
        self.active_task_id = Some(task_id.clone());
        self.active_map_id = Some(map_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.mark_routing_complete();

        let task = self
            .tasks
            .get(&task_id)
            .expect("mechanical blank task must exist before event emission");
        let map = self
            .maps
            .get(&map_id)
            .expect("mechanical blank map must exist before event emission");
        let mut events = vec![task_created_event(task), map_created_event(map)];
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
            self.bootstrap_required = self.tasks.is_empty();
        } else if self.active_map().is_some() {
            self.routing_required = false;
            self.bootstrap_required = false;
        } else {
            self.routing_required = true;
            self.bootstrap_required = self.tasks.is_empty();
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
        if self.tasks.is_empty() {
            self.bootstrap_required = true;
        }
        events
    }

    pub(crate) fn restore_snapshot(&mut self, snapshot: ActionMapSnapshot) {
        let maintenance_barriers = snapshot.maintenance_barriers.clone();
        self.mode = snapshot.mode;
        self.pending_transition_notice = None;
        self.routing_required = snapshot.routing_required;
        self.bootstrap_required = snapshot.bootstrap_required;
        self.reborn_requested = snapshot.reborn_requested;
        self.active_task_id = snapshot.active_task_id;
        self.active_map_id = snapshot.active_map_id;
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.main_tool_reservations.clear();
        self.child_tool_reservations.clear();
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
                tags: sanitize_trace_tags(event.tags),
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

        self.tasks = snapshot
            .tasks
            .into_iter()
            .map(|task| {
                let id = task.id;
                (
                    id.clone(),
                    TaskState {
                        id,
                        title: task.title,
                        source_event_ids: task.source_event_ids,
                        status: task_status_from_str(&task.status).unwrap_or(TaskStatus::Pending),
                        owner_session_id: task.owner_session_id,
                        active_map_id: task.active_map_id,
                        map_ids: task.map_ids,
                    },
                )
            })
            .collect();

        self.maps = snapshot
            .maps
            .into_iter()
            .map(|map| {
                let id = map.id;
                let mut instance = ActionMapInstance::new(
                    id.clone(),
                    map.title,
                    map.owner_session_id,
                    map.base_map_version,
                );
                instance.task_id = map.task_id;
                instance.status = map_status_from_str(&map.status).unwrap_or(MapStatus::Active);
                instance.created_from = map.created_from;
                instance.edges = map
                    .edges
                    .into_iter()
                    .map(|edge| MapEdge {
                        from: edge.from,
                        to: edge.to,
                    })
                    .collect();
                instance.results = map
                    .results
                    .into_iter()
                    .filter_map(|result| {
                        let kind = node_result_kind_from_str(&result.kind)?;
                        let action_class = result
                            .action_class
                            .as_deref()
                            .and_then(ActionClass::from_str);
                        Some((
                            result.id.clone(),
                            NodeResult {
                                id: result.id,
                                assignment_id: result.assignment_id,
                                map_id: result.map_id,
                                node_id: result.node_id,
                                kind,
                                action_class,
                                tool_success: result.tool_success,
                                source_event_ref: result.source_event_ref,
                                artifact_refs: result.artifact_refs,
                                source_thread_id: result.source_thread_id,
                                created_at_ms: result.created_at_ms,
                            },
                        ))
                    })
                    .collect();
                instance.node_events = map
                    .node_events
                    .into_iter()
                    .map(|event| {
                        let action_class = event
                            .action_class
                            .as_deref()
                            .and_then(ActionClass::from_str);
                        (
                            event.id.clone(),
                            NodeEvent {
                                id: event.id,
                                map_id: event.map_id,
                                node_id: event.node_id,
                                event_kind: event.event_kind,
                                source: event.source,
                                action_class,
                                tool_success: event.tool_success,
                                source_event_id: event.source_event_id,
                                raw_ref: event.raw_ref,
                                artifact_refs: event.artifact_refs,
                                call_id: event.call_id,
                                source_thread_id: event.source_thread_id,
                                created_at_ms: event.created_at_ms,
                            },
                        )
                    })
                    .collect();
                instance.nodes = map
                    .nodes
                    .into_iter()
                    .map(|node| {
                        let result_context = node
                            .result_ids
                            .into_iter()
                            .filter_map(|result_id| {
                                let result = instance.results.get(&result_id)?;
                                Some(NodeResultRef {
                                    id: result_id,
                                    kind: result.kind,
                                })
                            })
                            .collect();
                        let id = node.id;
                        let title = node.title;
                        let kind = NodeKind::from_str(&node.kind)
                            .unwrap_or_else(|| NodeKind::from_node_id_or_title(&id, &title));
                        (
                            id.clone(),
                            MapNode {
                                id,
                                title,
                                kind,
                                status: node_status_from_str(&node.status)
                                    .unwrap_or(NodeStatus::Pending),
                                context: NodeContext {
                                    summary: node.context_summary,
                                    source_refs: node.source_refs,
                                },
                                active_lease: node.active_lease,
                                result_context,
                                node_events: node
                                    .node_event_ids
                                    .into_iter()
                                    .filter_map(|node_event_id| {
                                        let event = instance.node_events.get(&node_event_id)?;
                                        Some(NodeEventRef {
                                            id: node_event_id,
                                            kind: event.event_kind.clone(),
                                        })
                                    })
                                    .collect(),
                                origin_node_id: node.origin_node_id,
                            },
                        )
                    })
                    .collect();
                instance.leases = map
                    .leases
                    .into_iter()
                    .filter_map(|lease| {
                        let holder = lease_holder_from_str(&lease.holder)?;
                        let previous_node_status =
                            node_status_from_str(&lease.previous_node_status)?;
                        Some((
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
                    .collect();
                (id, instance)
            })
            .collect();

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

        if self.mode == MapRuntimeMode::Experiment && !self.restored_active_binding_is_coherent() {
            self.active_task_id = None;
            self.active_map_id = None;
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            self.routing_required = true;
            self.bootstrap_required = self.tasks.is_empty();
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
        self.next_node_seq =
            next_numeric_seq(self.maps.values().flat_map(|map| map.nodes.keys()), "node-");
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
        } else {
            self.bootstrap_required = self.bootstrap_required || self.tasks.is_empty();
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
                let Some(task) = self.tasks.get(task_id) else {
                    return false;
                };
                let Some(map) = self.maps.get(map_id) else {
                    return false;
                };
                map.task_id.as_deref() == Some(task_id)
                    && task.active_map_id.as_deref() == Some(map_id)
                    && task.map_ids.iter().any(|candidate| candidate == map_id)
            }
            (Some(task_id), None) => self
                .tasks
                .get(task_id)
                .is_some_and(|task| task.active_map_id.is_none()),
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
            .filter(|map| map.status == MapStatus::Active)
    }

    pub(crate) fn snapshot(&self) -> ActionMapSnapshot {
        let mut maps = self
            .maps
            .values()
            .map(snapshot_map)
            .collect::<Vec<ActionMapSnapshotMap>>();
        maps.sort_by(|left, right| left.id.cmp(&right.id));
        let mut tasks = self
            .tasks
            .values()
            .map(snapshot_task)
            .collect::<Vec<ActionMapSnapshotTask>>();
        tasks.sort_by(|left, right| left.id.cmp(&right.id));
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
            mode: self.mode,
            routing_required: self.routing_required,
            bootstrap_required: self.bootstrap_required,
            reborn_requested: self.reborn_requested,
            active_task_id: self.active_task_id.clone(),
            active_map_id: self.active_map_id.clone(),
            tasks,
            maps,
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
            let node = map
                .nodes
                .get_mut(&node_id)
                .ok_or_else(|| format!("TaskSpace current node `{node_id}` is missing."))?;

            let node_event = NodeEvent {
                id: node_event_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                event_kind: "tool_result".to_string(),
                source: "main_tool".to_string(),
                action_class: recorded_action_class,
                tool_success: Some(success),
                source_event_id: Some(source_event_id),
                raw_ref,
                artifact_refs: artifact_refs.clone(),
                call_id: Some(call_id.to_string()),
                source_thread_id: owner_session_id,
                created_at_ms,
            };
            map.node_events.insert(node_event_id.clone(), node_event);
            node.node_events.push(NodeEventRef {
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
            body: body.clone(),
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
        let node_kind = current_node.map(|node| node.kind.as_str().to_string());
        let provider_request_context_missing_reason =
            self.provider_request_context_missing_reason(&map_id, node_id.as_deref(), phase);
        let task_id = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        let map_requires_initialization = self
            .maps
            .get(&map_id)
            .is_some_and(|map| map.nodes.is_empty());
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
            node_kind,
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
        match node.kind {
            NodeKind::FinalSynthesis => TaskSpaceProviderRequestPhase::FinalSynthesis,
            NodeKind::SmokeTest | NodeKind::RegressionTest => {
                TaskSpaceProviderRequestPhase::ValidationRecovery
            }
            _ => TaskSpaceProviderRequestPhase::ModelSampling,
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
            if input.status == "started" {
                if node_id != "provider-context-missing" {
                    let next_node_count = node_request_count_before + 1;
                    node_request_counts.insert(node_id.clone(), next_node_count);
                    self.budget_counters
                        .model_request_count_by_node
                        .insert(node_id.clone(), next_node_count);
                }
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
                    "node_kind:{}",
                    snapshot.node_kind.as_deref().unwrap_or("unknown")
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
                    format!(
                        "active_projection_present:{}",
                        scan.active_projection_present
                    ),
                    format!("active_projection_count:{}", scan.active_projection_count),
                    format!("large_raw_output_tokens:{}", scan.large_raw_output_tokens),
                    format!(
                        "runtime_boundary_forbidden_markers:{runtime_boundary_forbidden_markers}"
                    ),
                    format!("protected_items_present:{}", scan.protected_items_present),
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
        if let Some(preview) = input.last_agent_message_preview {
            if !preview.trim().is_empty() {
                tags.push(format!(
                    "last_agent_message_preview:{}",
                    sanitize_provider_response_trace_tag_value(&preview)
                ));
            }
        }
        let tool_success = Some(matches!(
            input.response_actionability.as_str(),
            "actionable" | "final_candidate"
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
        if map.status != MapStatus::Active {
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
        let node = map
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| format!("TaskSpace child node `{node_id}` is missing."))?;

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
        node.result_context.push(NodeResultRef {
            id: result_id.clone(),
            kind: NodeResultKind::MainToolCall,
        });
        let events = vec![MapRuntimeEvent::NodeResultRecorded(
            MapRuntimeNodeResultRecordedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id,
                result_id: result_id.clone(),
                kind: NodeResultKind::MainToolCall.as_str().to_string(),
                action_class: recorded_action_class.map(|class| class.as_str().to_string()),
                source_thread_id: child_thread_id,
            },
        )];
        Ok(Some((result_id, events)))
    }

    pub(crate) fn bind_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(Vec::new());
        }
        self.validate_routing_complete()?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
        if self.current_main_node_id.as_deref() == Some(node_id)
            && let Some(lease_id) = self.current_main_lease_id.as_ref()
            && node.active_lease.as_deref() == Some(lease_id.as_str())
            && let Some(lease) = map.leases.get(lease_id)
            && lease.holder == LeaseHolder::Main
            && lease.agent_thread_id == Some(owner_session_id)
        {
            return Ok(Vec::new());
        }
        if let Some(current_node_id) = self.current_main_node_id.as_deref()
            && current_node_id != node_id
            && self.current_main_lease_id.is_some()
        {
            return Err(format!(
                "TaskSpace current main node `{current_node_id}` is still running. hard_state: current_main_node_running. binding target `{node_id}` was not applied."
            ));
        }
        if node.status == NodeStatus::Pending {
            return Err(format!(
                "TaskSpace node `{node_id}` is pending. hard_state: target_node_dependencies_incomplete."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace node `{node_id}` is completed. hard_state: target_node_completed."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace node `{node_id}` is held by another lease. hard_state: target_node_lease_conflict."
            ));
        }
        let mut events = self.release_current_main_lease("main_rebound")?;
        events.extend(self.claim_main_node(owner_session_id, &map_id, node_id)?);
        events.extend(self.clear_maintenance_barrier_for_recovery(node_id));
        Ok(events)
    }

    pub(crate) fn current_main_node_id_for_owner(
        &self,
        owner_session_id: ThreadId,
    ) -> Result<MapNodeId, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        self.validate_main_binding(owner_session_id)?;
        self.current_main_node_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no current node binding exists. hard_state: no_current_node_binding."
                .to_string()
        })
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

    #[allow(dead_code)]
    pub(crate) fn create_node_for_main(
        &mut self,
        owner_session_id: ThreadId,
        title: String,
        context_summary: String,
        dependency_node_ids: Vec<String>,
        bind_current: bool,
    ) -> Result<(MapNodeId, Vec<MapRuntimeEvent>), String> {
        self.create_node_for_main_with_kind(
            owner_session_id,
            NodeKind::InspectCodeContext,
            title,
            context_summary,
            dependency_node_ids,
            bind_current,
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
        self.validate_routing_complete()?;
        let task_id = self.active_task_id.clone().ok_or_else(|| {
            "TaskSpace has no active task. hard_state: no_active_task_path.".to_string()
        })?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace has no active map. hard_state: no_active_task_path.".to_string()
        })?;
        {
            let task = self
                .tasks
                .get(&task_id)
                .ok_or_else(|| format!("TaskSpace active task `{task_id}` is missing."))?;
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            if task.owner_session_id != Some(owner_session_id)
                || map.owner_session_id != Some(owner_session_id)
            {
                return Err(
                    "TaskSpace mechanical blank map is owned by another session. hard_state: map_owner_mismatch."
                        .to_string(),
                );
            }
            if !taskspace_task_path_is_mechanical_blank(task, map) {
                return Err(
                "TaskSpace map initialization requires an untouched runtime mechanical blank map. hard_state: map_already_initialized."
                        .to_string(),
                );
            }
        }

        let task_title = require_nonempty_owned("task_title", input.task_title)?;
        if input.source_event_ids.is_empty() {
            return Err("TaskSpace map initialization requires source_event_ids.".to_string());
        }
        let mut source_event_ids = Vec::with_capacity(input.source_event_ids.len());
        let mut seen_source_event_ids = HashSet::new();
        for source_event_id in input.source_event_ids {
            let source_event_id = require_nonempty_owned("source_event_id", source_event_id)?;
            if !seen_source_event_ids.insert(source_event_id.clone()) {
                return Err(format!(
                    "TaskSpace map initialization source_event_id `{source_event_id}` is duplicated."
                ));
            }
            source_event_ids.push(source_event_id);
        }
        let current_node_id = require_nonempty_owned("current_node_id", input.current_node_id)?;
        if input.nodes.is_empty() {
            return Err(
                "TaskSpace map initialization requires at least one initial node. hard_state: active_task_path_without_nodes."
                    .to_string(),
            );
        }

        let mut node_ids = HashSet::new();
        let mut normalized_nodes = Vec::with_capacity(input.nodes.len());
        for node in input.nodes {
            validate_live_node_kind(node.kind)?;
            let id = require_nonempty_owned("node_id", node.id)?;
            if !node_ids.insert(id.clone()) {
                return Err(format!(
                    "TaskSpace map initialization node_id `{id}` is duplicated."
                ));
            }
            let title = require_nonempty_owned("node title", node.title)?;
            let context_summary =
                require_nonempty_owned("node context_summary", node.context_summary)?;
            let mut dependency_node_ids = Vec::new();
            let mut node_dependencies = HashSet::new();
            for dependency in node.dependency_node_ids {
                let dependency = require_nonempty_owned("dependency_node_id", dependency)?;
                if dependency == id {
                    return Err(format!(
                        "TaskSpace map initialization node `{id}` cannot depend on itself."
                    ));
                }
                if !node_dependencies.insert(dependency.clone()) {
                    return Err(format!(
                        "TaskSpace map initialization node `{id}` repeats dependency `{dependency}`."
                    ));
                }
                dependency_node_ids.push(dependency);
            }
            normalized_nodes.push(ActionMapInitializeNodeInput {
                id,
                kind: node.kind,
                title,
                context_summary,
                dependency_node_ids,
            });
        }
        if !node_ids.contains(&current_node_id) {
            return Err(format!(
                "TaskSpace map initialization current_node_id `{current_node_id}` does not exist."
            ));
        }
        for node in &normalized_nodes {
            for dependency in &node.dependency_node_ids {
                if !node_ids.contains(dependency) {
                    return Err(format!(
                        "TaskSpace map initialization node `{}` references missing dependency node `{dependency}`.",
                        node.id
                    ));
                }
            }
        }

        {
            let task = self
                .tasks
                .get_mut(&task_id)
                .expect("mechanical blank task was validated");
            task.title = task_title.clone();
            task.source_event_ids = source_event_ids;
            let map = self
                .maps
                .get_mut(&map_id)
                .expect("mechanical blank map was validated");
            map.title = task_title;
        }

        let input_order = normalized_nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let mut pending = normalized_nodes;
        let mut created_node_ids = HashSet::new();
        let mut events = Vec::new();
        while !pending.is_empty() {
            let ready_index = pending.iter().position(|node| {
                node.dependency_node_ids
                    .iter()
                    .all(|dependency| created_node_ids.contains(dependency))
            });
            let Some(ready_index) = ready_index else {
                return Err(
                    "TaskSpace map initialization dependency graph contains a cycle.".to_string(),
                );
            };
            let node = pending.remove(ready_index);
            let node_id = node.id;
            let (created_node_id, mut node_events) = self.create_node_for_main_with_id(
                owner_session_id,
                Some(node_id.clone()),
                node.kind,
                node.title,
                node.context_summary,
                node.dependency_node_ids,
                false,
            )?;
            debug_assert_eq!(created_node_id, node_id);
            created_node_ids.insert(node_id);
            events.append(&mut node_events);
        }

        events.extend(self.bind_main_node(owner_session_id, &current_node_id)?);
        let edge_count = self
            .maps
            .get(&map_id)
            .map(|map| map.edges.len())
            .unwrap_or_default();
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
                "action:initialize_then_actions".to_string(),
                format!("node_count:{}", created_node_ids.len()),
                format!("edge_count:{edge_count}"),
                "semantic_source:agent".to_string(),
                "runtime_inferred_semantics:false".to_string(),
            ],
        ));
        Ok((
            ActionMapInitializeOutcome {
                task_id,
                map_id,
                node_ids: input_order,
                current_node_id,
            },
            events,
        ))
    }

    pub(crate) fn create_node_for_main_with_kind(
        &mut self,
        owner_session_id: ThreadId,
        kind: NodeKind,
        title: String,
        context_summary: String,
        dependency_node_ids: Vec<String>,
        bind_current: bool,
    ) -> Result<(MapNodeId, Vec<MapRuntimeEvent>), String> {
        self.create_node_for_main_with_id(
            owner_session_id,
            None,
            kind,
            title,
            context_summary,
            dependency_node_ids,
            bind_current,
        )
    }

    fn create_node_for_main_with_id(
        &mut self,
        owner_session_id: ThreadId,
        requested_node_id: Option<String>,
        kind: NodeKind,
        title: String,
        context_summary: String,
        dependency_node_ids: Vec<String>,
        bind_current: bool,
    ) -> Result<(MapNodeId, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        self.validate_routing_complete()?;
        validate_live_node_kind(kind)?;
        let title = title.trim();
        let context_summary = context_summary.trim();
        if title.is_empty() {
            return Err("TaskSpace node title cannot be empty.".to_string());
        }
        if context_summary.is_empty() {
            return Err("TaskSpace node context summary cannot be empty.".to_string());
        }
        if bind_current
            && let Some(current_node_id) = self.current_main_node_id.as_deref()
            && self.current_main_lease_id.is_some()
        {
            return Err(format!(
                "TaskSpace current main node `{current_node_id}` is still running. hard_state: current_main_node_running. create-and-bind requires no active main lease on another node."
            ));
        }
        if self.active_map().is_none()
            && dependency_node_ids
                .iter()
                .any(|dependency| !dependency.trim().is_empty())
        {
            return Err(
                "TaskSpace cannot create a dependency-bound node without an active task path. hard_state: no_active_task_path."
                    .to_string(),
            );
        }
        let mut events = Vec::new();
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists. hard_state: no_active_task_path. node creation requires an active task path."
                .to_string()
        })?;
        let node_count_before = {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            map.nodes.len()
        };
        let active_budget = self.active_budget.as_ref().cloned().unwrap_or_else(|| {
            taskspace_active_budget_for_route(
                "taskspace-v005-active",
                TaskSpaceRouteMode::DefaultCompact,
            )
        });
        self.budget_counters.node_count = node_count_before;
        let max_nodes = active_budget.max_nodes;
        let gate_decision = self.gate_create_node_budget(&map_id, kind);
        let node_id = match requested_node_id {
            Some(node_id) => {
                let node_id = require_nonempty_owned("node_id", node_id)?;
                if self
                    .maps
                    .values()
                    .any(|map| map.nodes.contains_key(&node_id))
                {
                    return Err(format!("TaskSpace node_id `{node_id}` already exists."));
                }
                node_id
            }
            None => self.next_node_id(),
        };
        let map = self
            .maps
            .get_mut(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let mut dependencies = Vec::new();
        for dependency in dependency_node_ids {
            let dependency = dependency.trim();
            if dependency.is_empty() {
                continue;
            }
            let Some(node) = map.nodes.get(dependency) else {
                return Err(format!(
                    "TaskSpace dependency node `{dependency}` does not exist."
                ));
            };
            dependencies.push((dependency.to_string(), node.status));
        }
        let ready = dependencies
            .iter()
            .all(|(_, status)| *status == NodeStatus::Completed);
        if bind_current && !ready {
            return Err(
                "TaskSpace cannot bind the new node because its dependencies are incomplete. hard_state: target_node_dependencies_incomplete."
                    .to_string(),
            );
        }
        map.nodes.insert(
            node_id.clone(),
            MapNode {
                id: node_id.clone(),
                title: title.to_string(),
                kind,
                status: if ready {
                    NodeStatus::Ready
                } else {
                    NodeStatus::Pending
                },
                context: NodeContext {
                    summary: context_summary.to_string(),
                    source_refs: Vec::new(),
                },
                active_lease: None,
                result_context: Vec::new(),
                node_events: Vec::new(),
                origin_node_id: None,
            },
        );
        for (dependency, _) in dependencies {
            map.edges.push(MapEdge {
                from: dependency,
                to: node_id.clone(),
            });
        }
        if ready {
            events.push(node_status_changed_event(
                &map_id,
                &node_id,
                title,
                NodeStatus::Pending,
                NodeStatus::Ready,
            ));
        }
        if bind_current {
            events.extend(self.bind_main_node(owner_session_id, &node_id)?);
        }
        let task_id_for_budget = self.maps.get(&map_id).and_then(|map| map.task_id.clone());
        self.budget_counters.node_count = node_count_before + 1;
        events.push(self.record_runtime_budget_trace_event(
            "spawn_node_budget",
            task_id_for_budget,
            map_id.clone(),
            node_id.clone(),
            None,
            true,
            vec![
                "schema:taskspace-spawn-node-budget-event-v1".to_string(),
                "producer:runtime".to_string(),
                "budget_kind:node".to_string(),
                "action:create_node".to_string(),
                "status:allowed".to_string(),
                format!("node_count_before:{node_count_before}"),
                format!("node_count_after:{}", node_count_before + 1),
                "active_budget_source:runtime".to_string(),
                "enforcement:advisory".to_string(),
                format!("route_mode:{}", active_budget.route_mode.as_str()),
                format!("profile_name:{}", active_budget.profile_name.as_str()),
                format!("max_nodes:{max_nodes}"),
                "budget_response_action_taken:false".to_string(),
                format!("budget_gate_reason:{}", gate_decision.reason),
            ],
        ));
        Ok((node_id, events))
    }

    pub(crate) fn finish_main_node_with_next(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        agent_conclusion_event_ref: String,
        next_node_id: Option<String>,
        next_node_draft: Option<ActionMapNextNodeDraft>,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        if self.current_main_node_id.is_none() {
            let mut staged = self.clone();
            let mut events = staged.bind_main_node(owner_session_id, node_id)?;
            let (outcome, finish_events) = staged.finish_main_node_with_next(
                owner_session_id,
                node_id,
                agent_conclusion_event_ref,
                next_node_id,
                next_node_draft,
            )?;
            events.extend(finish_events);
            *self = staged;
            return Ok((outcome, events));
        }
        let agent_conclusion_event_ref = agent_conclusion_event_ref.trim();
        if agent_conclusion_event_ref.is_empty() {
            return Err("TaskSpace finish agent conclusion event ref cannot be empty.".to_string());
        }
        let next_node_id = next_node_id
            .as_deref()
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        if next_node_id.is_some() && next_node_draft.is_some() {
            return Err(
                "TaskSpace finish cannot provide both next_node_id and next node draft fields."
                    .to_string(),
            );
        }
        if let Some(next_node_id) = next_node_id {
            self.validate_next_main_binding_after_finish(node_id, next_node_id)?;
        }
        if let Some(draft) = next_node_draft.as_ref() {
            validate_live_node_kind(draft.kind)?;
            if draft.title.trim().is_empty() {
                return Err("TaskSpace next_node_title cannot be empty.".to_string());
            }
            if draft.context_summary.trim().is_empty() {
                return Err("TaskSpace next_node_context_summary cannot be empty.".to_string());
            }
            self.validate_next_node_draft_after_finish(node_id, draft)?;
        }
        let (result_id, mut events) = self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Result,
            agent_conclusion_event_ref.to_string(),
            NodeStatus::Completed,
            true,
        )?;
        let mut bound_next_node_id = None;
        if let Some(next_node_id) = next_node_id {
            let bind_events = self.bind_main_node(owner_session_id, next_node_id)?;
            events.extend(bind_events);
            bound_next_node_id = Some(next_node_id.to_string());
        } else if let Some(draft) = next_node_draft {
            let dependency_node_ids = if draft.dependency_node_ids.is_empty() {
                vec![node_id.to_string()]
            } else {
                draft.dependency_node_ids
            };
            let (created_node_id, node_events) = self.create_node_for_main_with_kind(
                owner_session_id,
                draft.kind,
                draft.title,
                draft.context_summary,
                dependency_node_ids,
                true,
            )?;
            events.extend(node_events);
            bound_next_node_id = Some(created_node_id);
        }
        Ok((
            ActionMapFinishNodeOutcome {
                result_id,
                next_node_id: bound_next_node_id,
            },
            events,
        ))
    }

    pub(crate) fn block_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        agent_conclusion_event_ref: String,
    ) -> Result<(NodeResultId, Vec<MapRuntimeEvent>), String> {
        let agent_conclusion_event_ref = agent_conclusion_event_ref.trim();
        if agent_conclusion_event_ref.is_empty() {
            return Err(
                "TaskSpace block_node agent conclusion event ref cannot be empty.".to_string(),
            );
        }
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        if !map.nodes.contains_key(node_id) {
            return Err(format!("TaskSpace node `{node_id}` is missing."));
        }
        let (blocker_result_id, mut events) = self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Blocker,
            agent_conclusion_event_ref.to_string(),
            NodeStatus::Blocked,
            false,
        )?;
        if let Some(map) = self.maps.get_mut(&map_id) {
            events.extend(refresh_ready_validation_rework_nodes(map, node_id));
        }
        Ok((blocker_result_id, events))
    }

    pub(crate) fn record_main_final_response(
        &mut self,
        _owner_session_id: ThreadId,
        message: &str,
    ) -> Result<Option<Vec<MapRuntimeEvent>>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }
        let message = message.trim();
        if message.is_empty() {
            return Ok(None);
        }
        self.validate_routing_complete()?;
        let map_id = match self.active_map_id.clone() {
            Some(map_id) => map_id,
            None => return Ok(None),
        };
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let task_id = map
            .task_id
            .clone()
            .ok_or_else(|| format!("TaskSpace active task for map `{map_id}` is missing."))?;
        let task = self
            .tasks
            .get(&task_id)
            .ok_or_else(|| format!("TaskSpace active task for map `{map_id}` is missing."))?;
        if taskspace_task_path_is_mechanical_blank(task, map) {
            return Err(
                "TaskSpace final response is unavailable while the task map is mechanically blank. hard_state: active_task_path_without_nodes."
                    .to_string(),
            );
        }
        if let Some(node_id) = self.current_main_node_id.as_deref() {
            let node = map
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("TaskSpace current node `{node_id}` is missing."))?;
            return Err(format!(
                "TaskSpace final response is unavailable while node `{node_id}` is {}. hard_state: active_node_open.",
                node.status.as_str()
            ));
        }
        let mut open_node_ids = map
            .nodes
            .values()
            .filter(|node| matches!(node.status, NodeStatus::Ready | NodeStatus::Running))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        open_node_ids.sort();
        if !open_node_ids.is_empty() {
            return Err(format!(
                "TaskSpace final response is unavailable while runnable nodes remain: {}. hard_state: active_map_has_open_nodes.",
                open_node_ids.join(",")
            ));
        }
        let previous_map_status = map.status;
        let previous_task_status = task.status;
        self.maps
            .get_mut(&map_id)
            .expect("validated TaskSpace map must remain present")
            .status = MapStatus::Completed;
        let task = self
            .tasks
            .get_mut(&task_id)
            .expect("validated TaskSpace task must remain present");
        task.status = TaskStatus::Completed;
        task.active_map_id = None;
        self.active_map_id = None;
        self.active_task_id = None;
        self.routing_required = true;
        self.bootstrap_required = false;

        Ok(Some(vec![
            MapRuntimeEvent::MapStatusChanged(MapRuntimeMapStatusChangedEvent {
                map_id,
                previous_status: previous_map_status.as_str().to_string(),
                current_status: MapStatus::Completed.as_str().to_string(),
            }),
            MapRuntimeEvent::TaskStatusChanged(MapRuntimeTaskStatusChangedEvent {
                task_id,
                previous_status: previous_task_status.as_str().to_string(),
                current_status: TaskStatus::Completed.as_str().to_string(),
            }),
        ]))
    }

    pub(crate) fn finish_main_node_with_terminal_candidate(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        agent_conclusion_event_ref: String,
        final_candidate: &str,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        if final_candidate.trim().is_empty() {
            return Err("TaskSpace terminal candidate must not be empty.".to_string());
        }
        let mut staged = self.clone();
        let (outcome, mut events) = staged.finish_main_node_with_next(
            owner_session_id,
            node_id,
            agent_conclusion_event_ref,
            None,
            None,
        )?;
        let completion_events = staged
            .record_main_final_response(owner_session_id, final_candidate)?
            .ok_or_else(|| "TaskSpace terminal final response was not recorded.".to_string())?;
        events.extend(completion_events);
        *self = staged;
        Ok((outcome, events))
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
        let node = map
            .nodes
            .get_mut(&node_id)
            .expect("ready node id should exist");
        let previous_node_status = node.status;
        node.status = NodeStatus::Running;
        node.active_lease = Some(lease_id.clone());
        let node_title = node.title.clone();
        let node_context_summary = node.context.summary.clone();
        let node_kind = node.kind;
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
        events.push(node_status_changed_event(
            &map_id,
            &node_id,
            &node_title,
            previous_node_status,
            NodeStatus::Running,
        ));
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
                message_prefix: assignment_prompt(
                    &map_id,
                    &node_id,
                    &node_title,
                    &node_context_summary,
                    node_kind,
                    &lease_id,
                ),
                map_id,
                node_id,
                node_title,
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
            if let Some(attached_thread_id) = lease.agent_thread_id {
                if attached_thread_id != thread_id {
                    return None;
                }
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
            let Some(lease) = map.leases.remove(lease_id) else {
                continue;
            };
            let released_main_lease = lease.holder == LeaseHolder::Main;
            let mut events = vec![MapRuntimeEvent::LeaseReleased(
                MapRuntimeLeaseReleasedEvent {
                    map_id: lease.map_id.clone(),
                    node_id: lease.node_id.clone(),
                    lease_id: lease.id.clone(),
                    holder: lease.holder.as_str().to_string(),
                    reason,
                },
            )];
            if let Some(node) = map.nodes.get_mut(&lease.node_id)
                && node.active_lease.as_deref() == Some(lease_id)
            {
                node.active_lease = None;
                if node.status == NodeStatus::Running {
                    let previous_status = node.status;
                    node.status = lease.previous_node_status;
                    events.push(node_status_changed_event(
                        &map.id,
                        &node.id,
                        &node.title,
                        previous_status,
                        node.status,
                    ));
                }
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
        let map = self.maps.get_mut(&map_id)?;
        let node = map.nodes.get_mut(&node_id)?;
        if node.active_lease.as_deref() != Some(lease_id.as_str()) {
            return None;
        }
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
        node.result_context.push(NodeResultRef {
            id: result_id.clone(),
            kind,
        });
        let previous_node_status = node.status;
        node.active_lease = None;
        node.status = match kind {
            NodeResultKind::Result | NodeResultKind::MapUpdateRequest => NodeStatus::Completed,
            NodeResultKind::Blocker | NodeResultKind::TimeoutSummary => NodeStatus::Blocked,
            NodeResultKind::MainToolCall => NodeStatus::Running,
        };
        map.leases.remove(&lease_id);
        self.child_tool_reservations
            .retain(|_, reservation| reservation.lease_id != lease_id);
        let mut events = vec![
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
            node_status_changed_event(
                &map_id,
                &node_id,
                &node.title,
                previous_node_status,
                node.status,
            ),
        ];
        events.extend(refresh_ready_nodes(map));
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

    pub(crate) fn build_developer_context(&mut self) -> Option<String> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }
        if self
            .active_map_id
            .as_ref()
            .and_then(|map_id| self.maps.get(map_id))
            .and_then(|map| {
                map.task_id
                    .as_ref()
                    .and_then(|task_id| self.tasks.get(task_id))
                    .map(|task| taskspace_task_path_is_mechanical_blank(task, map))
            })
            .unwrap_or(false)
        {
            tracing::debug!(
                target: "codex_core::taskspace",
                event_name = "taskspace.blank_context_omitted",
                task_id = ?self.active_task_id,
                map_id = ?self.active_map_id,
                "mechanical blank map omitted from provider developer context"
            );
            return None;
        }
        if let Some(context) = self.build_active_projection_developer_context() {
            return Some(context);
        }
        Some(self.build_bootstrap_compact_developer_context())
    }
    fn build_bootstrap_compact_developer_context(&self) -> String {
        let mut context =
            String::from("TaskSpace v0.0.5 thin bootstrap. Runtime state is authoritative.\n");
        if self.tasks.is_empty() {
            context.push_str(
                "Hard state: runtime mechanical blank map is missing. ordinary tools and spawn_agent are unavailable.\n",
            );
        } else {
            context.push_str(
                "Hard state: no active TaskSpace path is bound for this turn. ordinary tools and spawn_agent require an active task path.\n",
            );
            context.push_str("Task inventory compact:\n");
            for task_id in ordered_task_ids(&self.tasks).into_iter().take(6) {
                if let Some(task) = self.tasks.get(&task_id) {
                    context.push_str("- ");
                    context.push_str(&task.id);
                    context.push_str(" [");
                    context.push_str(task.status.as_str());
                    context.push_str("] ");
                    context.push_str(&single_line_preview(&task.title, 100));
                    context.push_str(" source_events=");
                    context.push_str(&task.source_event_ids.join(","));
                    context.push('\n');
                }
            }
            append_omitted_count(&mut context, self.tasks.len(), 6, "tasks");
        }
        if self.reborn_requested {
            context.push_str(
                "Task reborn status: requested. hard_state: reborn_route_unbound. ordinary tools require an active task path, current node binding, and lease.\n",
            );
        }
        context
    }

    fn build_active_projection_developer_context(&mut self) -> Option<String> {
        let map_id = self.active_map_id.clone()?;
        let (task_id, current_node_id, context, estimated_tokens, max_projection_tokens) = {
            let Some(map) = self.maps.get(&map_id) else {
                return Some(taskspace_projection_integrity_context(
                    &map_id,
                    "active_map_record_missing",
                ));
            };
            let Some(task) = map
                .task_id
                .as_ref()
                .or(self.active_task_id.as_ref())
                .and_then(|task_id| self.tasks.get(task_id))
            else {
                return Some(taskspace_projection_integrity_context(
                    &map_id,
                    "active_task_record_missing",
                ));
            };
            let mut context = String::new();
            if self.bootstrap_required {
                context.push_str(
                    "Bootstrap status: required before ordinary tools or subagent spawn.\n",
                );
            } else if self.routing_required {
                context.push_str(
                    "Task routing status: required before ordinary tools or subagent spawn.\n",
                );
            }
            if let Some(barrier) = self.active_maintenance_barrier() {
                context.push_str("Maintenance barrier:\n- map: ");
                context.push_str(&barrier.map_id);
                context.push_str("\n- node: ");
                context.push_str(&barrier.node_id);
                context.push_str("\n- reason: ");
                context.push_str(barrier.reason.as_str());
                context.push('\n');
            }
            let estimated_tokens = append_context_projection_active(
                &mut context,
                task,
                map,
                self.current_main_node_id.as_deref(),
                self.active_budget.as_ref(),
            );
            (
                task.id.clone(),
                self.current_main_node_id
                    .clone()
                    .unwrap_or_else(|| "projection".to_string()),
                context,
                estimated_tokens,
                self.active_budget
                    .as_ref()
                    .map(|budget| budget.max_projection_tokens)
                    .unwrap_or(usize::MAX),
            )
        };
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
        let status = if estimated_tokens <= max_projection_tokens {
            "within_budget"
        } else {
            "over_budget"
        };
        let _ = self.record_runtime_budget_trace_event(
            "projection_budget",
            Some(task_id),
            map_id,
            current_node_id,
            None,
            estimated_tokens <= max_projection_tokens,
            vec![
                "schema:taskspace-projection-budget-v1".to_string(),
                "producer:runtime".to_string(),
                "active_budget_source:runtime".to_string(),
                format!("route_mode:{route_mode}"),
                format!("profile_name:{profile_name}"),
                format!("projection_tokens:{estimated_tokens}"),
                format!("max_projection_tokens:{max_projection_tokens}"),
                format!("status:{status}"),
            ],
        );
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

    fn register_map_to_task(&mut self, task_id: &str, map_id: &str) {
        let task = self
            .tasks
            .get_mut(task_id)
            .expect("map registration must target an existing TaskSpace task");
        task.status = TaskStatus::Active;
        task.active_map_id = Some(map_id.to_string());
        if !task.map_ids.iter().any(|id| id == map_id) {
            task.map_ids.push(map_id.to_string());
        }
    }

    fn release_current_main_lease(&mut self, reason: &str) -> Result<Vec<MapRuntimeEvent>, String> {
        let Some(lease_id) = self.current_main_lease_id.clone() else {
            self.current_main_node_id = None;
            return Ok(Vec::new());
        };
        if let (Some(map_id), Some(node_id)) = (
            self.active_map_id.as_deref(),
            self.current_main_node_id.as_deref(),
        ) {
            self.validate_no_main_tool_reservations_for_node(map_id, node_id, reason)?;
        }
        Ok(self.release_lease(&lease_id, reason))
    }

    fn mark_routing_complete(&mut self) {
        self.routing_required = false;
        self.bootstrap_required = false;
        self.reborn_requested = false;
    }

    fn claim_main_node(
        &mut self,
        owner_session_id: ThreadId,
        map_id: &str,
        node_id: &str,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let lease_id = self.next_lease_id();
        let map = self
            .maps
            .get_mut(map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
        if node.status == NodeStatus::Pending {
            return Err(format!(
                "TaskSpace node `{node_id}` is pending. hard_state: target_node_dependencies_incomplete."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace node `{node_id}` is completed. hard_state: target_node_completed."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace node `{node_id}` is held by another lease. hard_state: target_node_lease_conflict."
            ));
        }

        let previous_node_status = node.status;
        node.status = NodeStatus::Running;
        node.active_lease = Some(lease_id.clone());
        let node_title = node.title.clone();
        map.leases.insert(
            lease_id.clone(),
            AssignmentLease {
                id: lease_id.clone(),
                map_id: map_id.to_string(),
                node_id: node_id.to_string(),
                holder: LeaseHolder::Main,
                previous_node_status,
                agent_thread_id: Some(owner_session_id),
                agent_path: None,
            },
        );
        self.current_main_node_id = Some(node_id.to_string());
        self.current_main_lease_id = Some(lease_id.clone());
        Ok(vec![
            node_status_changed_event(
                map_id,
                node_id,
                &node_title,
                previous_node_status,
                NodeStatus::Running,
            ),
            MapRuntimeEvent::LeaseCreated(MapRuntimeLeaseCreatedEvent {
                map_id: map_id.to_string(),
                node_id: node_id.to_string(),
                lease_id,
                holder: LeaseHolder::Main.as_str().to_string(),
            }),
        ])
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
        if map.status != MapStatus::Active {
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

    fn record_main_node_lifecycle_result(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        kind: NodeResultKind,
        source_event_ref: String,
        next_status: NodeStatus,
        refresh_downstream: bool,
    ) -> Result<(NodeResultId, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        self.validate_routing_complete()?;
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let current_node_id = self.current_main_node_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no current node binding exists. hard_state: no_current_node_binding. lifecycle records require a current node binding and lease."
                .to_string()
        })?;
        if current_node_id != node_id {
            if let Some(map) = self.maps.get(&map_id)
                && let Some(node) = map.nodes.get(node_id)
                && node.status == NodeStatus::Completed
            {
                return Err(format!(
                    "TaskSpace node `{node_id}` is already completed. hard_state: lifecycle_target_already_completed. lifecycle records apply to current bound node `{current_node_id}`."
                ));
            }
            return Err(format!(
                "TaskSpace node `{node_id}` is not the current main action node `{current_node_id}`. hard_state: lifecycle_target_not_current. lifecycle records apply to the current bound node."
            ));
        }
        let current_lease_id = self.current_main_lease_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no current main lease exists. hard_state: no_current_main_lease."
                .to_string()
        })?;

        {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            if map.status != MapStatus::Active {
                return Err(format!(
                    "TaskSpace active task path `{map_id}` is not active."
                ));
            }
            let node = map
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
            if node.status == NodeStatus::Pending {
                return Err(format!(
                    "TaskSpace node `{node_id}` is pending. hard_state: lifecycle_target_dependencies_incomplete."
                ));
            }
            if node.status == NodeStatus::Completed {
                return Err(format!(
                    "TaskSpace node `{node_id}` is already completed. hard_state: lifecycle_target_already_completed."
                ));
            }
            let lease = map.leases.get(&current_lease_id).ok_or_else(|| {
                format!("TaskSpace current main lease `{current_lease_id}` is missing.")
            })?;
            if lease.holder != LeaseHolder::Main
                || lease.node_id != node_id
                || lease.agent_thread_id != Some(owner_session_id)
                || node.active_lease.as_deref() != Some(current_lease_id.as_str())
            {
                return Err(format!(
                    "TaskSpace node `{node_id}` is not held by the current main agent lease."
                ));
            }
        }
        self.validate_no_main_tool_reservations_for_node(
            &map_id,
            node_id,
            "recording a node lifecycle result",
        )?;

        let result_id = self.next_result_id();
        let mut barrier_to_clear = None;
        let mut events = Vec::new();
        {
            let map = self
                .maps
                .get_mut(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            let node = map
                .nodes
                .get_mut(node_id)
                .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
            let previous_status = node.status;
            map.leases.remove(&current_lease_id);
            let result = NodeResult {
                id: result_id.clone(),
                assignment_id: current_lease_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.to_string(),
                kind,
                action_class: None,
                tool_success: None,
                source_event_ref,
                artifact_refs: Vec::new(),
                source_thread_id: owner_session_id,
                created_at_ms: now_ms(),
            };
            map.results.insert(result_id.clone(), result);
            node.result_context.push(NodeResultRef {
                id: result_id.clone(),
                kind,
            });
            node.status = next_status;
            node.active_lease = None;
            events.push(MapRuntimeEvent::NodeResultRecorded(
                MapRuntimeNodeResultRecordedEvent {
                    map_id: map_id.clone(),
                    node_id: node_id.to_string(),
                    lease_id: current_lease_id.clone(),
                    result_id: result_id.clone(),
                    kind: kind.as_str().to_string(),
                    action_class: None,
                    source_thread_id: owner_session_id,
                },
            ));
            events.push(MapRuntimeEvent::LeaseReleased(
                MapRuntimeLeaseReleasedEvent {
                    map_id: map_id.clone(),
                    node_id: node_id.to_string(),
                    lease_id: current_lease_id.clone(),
                    holder: LeaseHolder::Main.as_str().to_string(),
                    reason: "result_recorded".to_string(),
                },
            ));
            if previous_status != next_status {
                events.push(node_status_changed_event(
                    &map_id,
                    node_id,
                    &node.title,
                    previous_status,
                    next_status,
                ));
            }
            if refresh_downstream {
                events.extend(refresh_ready_nodes(map));
            }
        }

        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        if let Some(barrier) = self.maintenance_barriers.get(&map_id)
            && barrier.map_id == map_id
            && barrier.node_id == node_id
        {
            barrier_to_clear = Some(maintenance_barrier_cleared_event(
                barrier,
                "node_lifecycle_recorded",
            ));
        }
        if let Some(event) = barrier_to_clear {
            self.maintenance_barriers.remove(&map_id);
            events.push(event);
        }
        Ok((result_id, events))
    }

    fn validate_next_main_binding_after_finish(
        &self,
        finishing_node_id: &str,
        next_node_id: &str,
    ) -> Result<(), String> {
        if finishing_node_id == next_node_id {
            return Err(format!(
                "TaskSpace cannot bind next_node_id `{next_node_id}` because the current node will be completed."
            ));
        }
        let map_id = self.active_map_id.as_ref().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(next_node_id)
            .ok_or_else(|| format!("TaskSpace next node `{next_node_id}` does not exist."))?;
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace next node `{next_node_id}` is completed. hard_state: next_node_completed."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace next node `{next_node_id}` is held by another lease. hard_state: next_node_lease_conflict."
            ));
        }
        if node.status == NodeStatus::Pending
            && !dependencies_will_be_completed_after_finish(map, finishing_node_id, next_node_id)
        {
            return Err(format!(
                "TaskSpace next node `{next_node_id}` is still pending after the current node completes."
            ));
        }
        if let Some(barrier) = self.maintenance_barriers.get(map_id)
            && barrier.node_id == next_node_id
        {
            return Err(format!(
                "TaskSpace maintenance barrier is active for next node `{next_node_id}`. hard_state: next_node_maintenance_barrier_active."
            ));
        }
        Ok(())
    }

    fn validate_next_node_draft_after_finish(
        &self,
        finishing_node_id: &str,
        draft: &ActionMapNextNodeDraft,
    ) -> Result<(), String> {
        let map_id = self.active_map_id.as_ref().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let dependency_node_ids = if draft.dependency_node_ids.is_empty() {
            vec![finishing_node_id.to_string()]
        } else {
            draft.dependency_node_ids.clone()
        };
        for dependency in &dependency_node_ids {
            let dependency = dependency.trim();
            if dependency.is_empty() {
                continue;
            }
            let Some(node) = map.nodes.get(dependency) else {
                return Err(format!(
                    "TaskSpace dependency node `{dependency}` does not exist."
                ));
            };
            if dependency != finishing_node_id && node.status != NodeStatus::Completed {
                return Err(format!(
                    "TaskSpace dependency node `{dependency}` will not be completed after finishing `{finishing_node_id}`."
                ));
            }
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
        if map.status != MapStatus::Active {
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
        if map.status != MapStatus::Active {
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

    fn clear_maintenance_barrier_for_recovery(
        &mut self,
        recovery_node_id: &str,
    ) -> Vec<MapRuntimeEvent> {
        let Some(map_id) = self.active_map_id.clone() else {
            return Vec::new();
        };
        let Some(barrier) = self.maintenance_barriers.get(&map_id) else {
            return Vec::new();
        };
        if barrier.node_id == recovery_node_id {
            return Vec::new();
        }
        let event = maintenance_barrier_cleared_event(barrier, "bound_recovery_node");
        self.maintenance_barriers.remove(&map_id);
        vec![event]
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
                    node.status == NodeStatus::Ready && node.active_lease.is_none()
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

    fn next_node_id(&mut self) -> MapNodeId {
        loop {
            let id = format!("node-{}", self.next_node_seq);
            self.next_node_seq += 1;
            if !self.maps.values().any(|map| map.nodes.contains_key(&id)) {
                return id;
            }
        }
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
    output.push_str("TaskSpace\n");
    output.push_str("- mode: ");
    output.push_str(&snapshot.mode.to_string());
    output.push('\n');
    output.push_str("- routing required: ");
    output.push_str(if snapshot.routing_required {
        "yes"
    } else {
        "no"
    });
    output.push('\n');
    output.push_str("- bootstrap required: ");
    output.push_str(if snapshot.bootstrap_required {
        "yes"
    } else {
        "no"
    });
    output.push('\n');
    output.push_str("- reborn requested: ");
    output.push_str(if snapshot.reborn_requested {
        "yes"
    } else {
        "no"
    });
    output.push('\n');
    output.push_str("- active task: ");
    output.push_str(snapshot.active_task_id.as_deref().unwrap_or("none"));
    output.push('\n');
    output.push_str("- active map: ");
    output.push_str(snapshot.active_map_id.as_deref().unwrap_or("none"));
    output.push('\n');
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
    if !snapshot.tasks.is_empty() {
        output.push_str("\nTasks:\n");
        for task in &snapshot.tasks {
            output.push_str("- ");
            output.push_str(&task.id);
            output.push_str(" [");
            output.push_str(&task.status);
            output.push_str("] ");
            output.push_str(&task.title);
            if let Some(map_id) = task.active_map_id.as_ref() {
                output.push_str(" active_map=");
                output.push_str(map_id);
            }
            output.push('\n');
        }
    }
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
    if snapshot.maps.is_empty() {
        output.push_str("\nNo task path has been created in this thread.\n");
        return output;
    }

    for map in &snapshot.maps {
        output.push('\n');
        output.push_str("Map ");
        output.push_str(&map.id);
        output.push_str(": ");
        output.push_str(&map.title);
        output.push('\n');
        output.push_str("- status: ");
        output.push_str(&map.status);
        output.push_str("\n- nodes: ready=");
        output.push_str(&map.ready_node_count.to_string());
        output.push_str(", running=");
        output.push_str(&map.running_node_count.to_string());
        output.push_str(", completed=");
        output.push_str(&map.completed_node_count.to_string());
        output.push('\n');
        output.push_str("- owner session: ");
        output.push_str(
            &map.owner_session_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
        output.push('\n');
        output.push_str("\nNodes:\n");
        for node in &map.nodes {
            output.push_str("- ");
            output.push_str(&node.id);
            output.push_str(" [");
            output.push_str(&node.status);
            output.push_str("] ");
            output.push_str(&node.title);
            output.push_str(" kind=");
            output.push_str(&node.kind);
            if let Some(lease) = node.active_lease.as_ref() {
                output.push_str(" lease=");
                output.push_str(lease);
            }
            if !node.result_ids.is_empty() {
                output.push_str(" results=");
                output.push_str(&node.result_ids.join(","));
            }
            output.push('\n');
        }

        if !map.leases.is_empty() {
            output.push_str("\nLeases:\n");
            for lease in &map.leases {
                output.push_str("- ");
                output.push_str(&lease.id);
                output.push_str(" node=");
                output.push_str(&lease.node_id);
                output.push_str(" holder=");
                output.push_str(&lease.holder);
                output.push_str(" agent=");
                output.push_str(
                    &lease
                        .agent_thread_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "unattached".to_string()),
                );
                if let Some(path) = lease.agent_path.as_ref() {
                    output.push_str(" path=");
                    output.push_str(path);
                }
                output.push('\n');
            }
        }

        if !map.results.is_empty() {
            output.push_str("\nResults:\n");
            for result in &map.results {
                output.push_str("- ");
                output.push_str(&result.id);
                output.push_str(" node=");
                output.push_str(&result.node_id);
                output.push_str(" kind=");
                output.push_str(&result.kind);
                if let Some(action_class) = result.action_class.as_ref() {
                    output.push_str(" action_class=");
                    output.push_str(action_class);
                }
                output.push_str(" from=");
                output.push_str(&result.source_thread_id.to_string());
                output.push_str(" source_event_ref=");
                output.push_str(&result.source_event_ref);
                if !result.artifact_refs.is_empty() {
                    output.push_str(" artifact_refs=");
                    output.push_str(&result.artifact_refs.join(","));
                }
                output.push('\n');
            }
        }
    }

    output
}

fn taskspace_projection_integrity_context(map_id: &str, reason: &str) -> String {
    format!(
        "ContextProjectionV1 epoch snapshot:\n\
- task_id: unavailable\n\
- map_id: {map_id}\n\
- integrity_status: invalid\n\
- integrity_reason: {reason}\n\
- current_node: unavailable\n\
- map_nodes:\n  - none\n\
- map_edges:\n  - none\n\
- current_node_recent_events:\n  - none\n\
- result_refs_available:\n  - none\n\
ContextProjectionV1 epoch snapshot end."
    )
}

fn taskspace_task_path_is_mechanical_blank(task: &TaskState, map: &ActionMapInstance) -> bool {
    task.title == TASKSPACE_MECHANICAL_BLANK_TASK_TITLE
        && task.source_event_ids.is_empty()
        && map.title == TASKSPACE_MECHANICAL_BLANK_MAP_TITLE
        && map.nodes.is_empty()
        && map.edges.is_empty()
        && map.leases.is_empty()
        && map.results.is_empty()
        && map.node_events.is_empty()
}

fn append_context_projection_active(
    context: &mut String,
    task: &TaskState,
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
    _active_budget: Option<&TaskSpaceActiveBudgetV1>,
) -> usize {
    let current_node_id = current_node_id
        .filter(|node_id| map.nodes.contains_key(*node_id))
        .map(str::to_string);
    let node_skeleton = ordered_node_ids(map)
        .into_iter()
        .filter_map(|node_id| map.nodes.get(&node_id))
        .map(|node| ProjectionNode {
            id: node.id.clone(),
            kind: node.kind.as_str().to_string(),
            status: node.status.as_str().to_string(),
            goal: node.context.summary.clone(),
            result_count: node.result_context.len(),
            event_count: node.node_events.len(),
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
    let (recent_tool_feedback, projected_event_ids) =
        projection_recent_event_refs(map, current_node_id.as_deref(), 6);
    let result_refs_available = projection_result_refs_available(map, 8, &projected_event_ids);

    let rendered = render_active_projection(ActiveProjectionInput {
        task_id: task.id.clone(),
        task_status: task.status.as_str().to_string(),
        map_id: map.id.clone(),
        map_status: map.status.as_str().to_string(),
        source_event_ids: task.source_event_ids.clone(),
        current_node_id,
        map_nodes: node_skeleton,
        map_edges,
        current_node_recent_events: recent_tool_feedback,
        result_refs_available,
        mechanically_blank: taskspace_task_path_is_mechanical_blank(task, map),
    });
    context.push_str(&rendered.body);
    rendered.estimated_tokens
}

fn projection_evidence_node_ids<'a>(
    map: &'a ActionMapInstance,
    current_node_id: Option<&'a str>,
) -> Vec<&'a str> {
    let Some(current_node_id) = current_node_id else {
        let mut seen = HashSet::new();
        let mut node_ids = Vec::new();
        for event_id in ordered_node_event_ids(map).into_iter().rev() {
            let Some(event) = map.node_events.get(&event_id) else {
                continue;
            };
            let node_id = event.node_id.as_str();
            if seen.insert(node_id) {
                node_ids.push(node_id);
            }
            if node_ids.len() >= 3 {
                break;
            }
        }
        if node_ids.is_empty() {
            for result_id in ordered_result_ids(map).into_iter().rev() {
                let Some(result) = map.results.get(&result_id) else {
                    continue;
                };
                let node_id = result.node_id.as_str();
                if seen.insert(node_id) {
                    node_ids.push(node_id);
                }
                if node_ids.len() >= 3 {
                    break;
                }
            }
        }
        node_ids.reverse();
        return node_ids;
    };
    let mut node_ids = map
        .edges
        .iter()
        .filter(|edge| edge.to == current_node_id)
        .map(|edge| edge.from.as_str())
        .collect::<Vec<_>>();
    node_ids.push(current_node_id);
    node_ids
}

fn projection_recent_event_refs(
    map: &ActionMapInstance,
    current_node_id: Option<&str>,
    max_results: usize,
) -> (Vec<ProjectionEventRef>, HashSet<String>) {
    let node_ids = projection_evidence_node_ids(map, current_node_id);
    if node_ids.is_empty() {
        return (Vec::new(), HashSet::new());
    }

    let mut selected = ordered_node_event_ids(map)
        .into_iter()
        .rev()
        .filter_map(|event_id| map.node_events.get(&event_id))
        .filter(|event| {
            (event.source == "main_tool" || event.source == "runtime_feedback")
                && node_ids
                    .iter()
                    .any(|node_id| *node_id == event.node_id.as_str())
        })
        .take(max_results)
        .collect::<Vec<_>>();
    selected.reverse();

    let event_ids = selected
        .iter()
        .map(|event| event.id.clone())
        .collect::<HashSet<_>>();
    let entries = selected.into_iter().map(projection_event_ref).collect();
    (entries, event_ids)
}

fn projection_result_refs_available(
    map: &ActionMapInstance,
    max_results: usize,
    excluded_event_ids: &HashSet<String>,
) -> Vec<ProjectionEventRef> {
    let event_ids = ordered_node_event_ids(map);
    let start = event_ids.len().saturating_sub(max_results);
    event_ids
        .into_iter()
        .skip(start)
        .filter_map(|event_id| map.node_events.get(&event_id))
        .filter(|event| !excluded_event_ids.contains(&event.id))
        .map(projection_event_ref)
        .collect()
}

fn projection_event_ref(event: &NodeEvent) -> ProjectionEventRef {
    ProjectionEventRef {
        id: event
            .source_event_id
            .clone()
            .unwrap_or_else(|| event.id.clone()),
        node_id: event.node_id.clone(),
        event_kind: event.event_kind.clone(),
        source: event.source.clone(),
        action_class: event
            .action_class
            .map(|action_class| action_class.as_str().to_string()),
        tool_success: event.tool_success,
        raw_ref: event.raw_ref.clone(),
        artifact_refs: event.artifact_refs.clone(),
    }
}

fn append_omitted_count(context: &mut String, len: usize, limit: usize, label: &str) {
    if len > limit {
        context.push_str("  - ... ");
        context.push_str(&(len - limit).to_string());
        context.push(' ');
        context.push_str(label);
        context.push_str(" omitted\n");
    }
}

fn ordered_node_ids(map: &ActionMapInstance) -> Vec<MapNodeId> {
    let mut ordered = SEED_NODE_IDS
        .iter()
        .filter(|node_id| map.nodes.contains_key(**node_id))
        .map(|node_id| (*node_id).to_string())
        .collect::<Vec<_>>();
    let mut dynamic = map
        .nodes
        .keys()
        .filter(|node_id| !SEED_NODE_IDS.contains(&node_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    dynamic.sort_by(|left, right| node_id_sort_key(left).cmp(&node_id_sort_key(right)));
    ordered.extend(dynamic);
    ordered
}

fn ordered_result_ids(map: &ActionMapInstance) -> Vec<NodeResultId> {
    let mut result_ids = map.results.keys().cloned().collect::<Vec<_>>();
    result_ids.sort_by(|left, right| result_id_sort_key(left).cmp(&result_id_sort_key(right)));
    result_ids
}

fn ordered_node_event_ids(map: &ActionMapInstance) -> Vec<NodeEventId> {
    let mut event_ids = map.node_events.keys().cloned().collect::<Vec<_>>();
    event_ids
        .sort_by(|left, right| node_event_id_sort_key(left).cmp(&node_event_id_sort_key(right)));
    event_ids
}

fn ordered_task_ids(tasks: &HashMap<TaskId, TaskState>) -> Vec<TaskId> {
    let mut task_ids = tasks.keys().cloned().collect::<Vec<_>>();
    task_ids.sort_by(|left, right| task_id_sort_key(left).cmp(&task_id_sort_key(right)));
    task_ids
}

fn task_id_sort_key(task_id: &str) -> (u8, u64, &str) {
    if let Some(number) = task_id
        .strip_prefix("task-")
        .and_then(|suffix| suffix.parse::<u64>().ok())
    {
        return (0, number, task_id);
    }
    (1, 0, task_id)
}

fn next_numeric_seq<'a>(ids: impl Iterator<Item = &'a String>, prefix: &str) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn task_status_from_str(status: &str) -> Option<TaskStatus> {
    match status {
        "active" => Some(TaskStatus::Active),
        "pending" => Some(TaskStatus::Pending),
        "completed" => Some(TaskStatus::Completed),
        _ => None,
    }
}

fn map_status_from_str(status: &str) -> Option<MapStatus> {
    match status {
        "active" => Some(MapStatus::Active),
        "completed" => Some(MapStatus::Completed),
        "abandoned" => Some(MapStatus::Abandoned),
        _ => None,
    }
}

fn node_status_from_str(status: &str) -> Option<NodeStatus> {
    match status {
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

fn validate_live_node_kind(kind: NodeKind) -> Result<(), String> {
    if kind == NodeKind::Custom {
        return Err(
            "TaskSpace live node creation requires a concrete node_kind. `custom` is reserved for restored legacy nodes; choose inspect_code_context, implement_solution, smoke_test, regression_test, or final_synthesis."
                .to_string(),
        );
    }
    Ok(())
}

fn node_id_sort_key(node_id: &str) -> (u8, u64, &str) {
    if let Some(number) = node_id
        .strip_prefix("node-")
        .and_then(|suffix| suffix.parse::<u64>().ok())
    {
        return (0, number, node_id);
    }
    (1, 0, node_id)
}

fn result_id_sort_key(result_id: &str) -> (u8, u64, &str) {
    if let Some(number) = result_id
        .strip_prefix("result-")
        .and_then(|suffix| suffix.parse::<u64>().ok())
    {
        return (0, number, result_id);
    }
    (1, 0, result_id)
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

fn snapshot_task(task: &TaskState) -> ActionMapSnapshotTask {
    ActionMapSnapshotTask {
        id: task.id.clone(),
        title: task.title.clone(),
        source_event_ids: task.source_event_ids.clone(),
        status: task.status.as_str().to_string(),
        owner_session_id: task.owner_session_id,
        active_map_id: task.active_map_id.clone(),
        map_ids: task.map_ids.clone(),
    }
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

fn sanitize_trace_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .filter(|tag| is_known_trace_tag(tag))
        .collect()
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
        "no_action_follow_up" | "tool_feedback_recovery" | "final_rejected"
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

fn is_known_trace_tag(tag: &str) -> bool {
    matches!(
        tag,
        "tool_success"
            | "tool_failure"
            | "validator_success"
            | "validator_failure"
            | "validator_infra_failure"
            | "validator_unconfirmed"
            | "unclassified_tool_action"
    ) || tag.starts_with("schema:")
        || tag.starts_with("producer:")
        || tag.starts_with("active_budget_source:")
        || tag.starts_with("profile_name:")
        || tag.starts_with("route_mode:")
        || tag.starts_with("status:")
        || tag.starts_with("request_count_")
        || tag.starts_with("max_rollout_model_requests:")
        || tag.starts_with("max_requests:")
        || tag.starts_with("node_request_count:")
        || tag.starts_with("max_model_requests_per_node:")
        || tag.starts_with("request_phase:")
        || tag.starts_with("request_phase_missing_reason:")
        || tag.starts_with("provider_request_context_missing_reason:")
        || tag.starts_with("node_kind:")
        || tag.starts_with("feedback_kind:")
        || tag.starts_with("trigger_kind:")
        || tag.starts_with("response_actionability_previous:")
        || tag.starts_with("previous_response_recovery_action:")
        || tag.starts_with("previous_response_trace_event_id:")
        || tag.starts_with("adoption_actor:")
        || tag.starts_with("latest_tool_result_refs:")
        || tag.starts_with("model_visible_feedback_refs:")
        || tag.starts_with("adoption_blockers:")
        || tag.starts_with("projection_bundle_hash:")
        || tag.starts_with("request_reason_delta:")
        || tag.starts_with("repeated_same_reason_count:")
        || tag.starts_with("reason_confidence:")
        || tag.starts_with("provider_request_budget_trace_event_id:")
        || tag.starts_with("logical_request_id:")
        || tag.starts_with("attempt_seq:")
        || tag.starts_with("transport:")
        || tag.starts_with("runtime_budget_state:")
        || tag.starts_with("budget_state_before:")
        || tag.starts_with("budget_state_after:")
        || tag.starts_with("budget_transition_reason:")
        || tag.starts_with("started_at_ms:")
        || tag.starts_with("completed_at_ms:")
        || tag.starts_with("latency_ms:")
        || tag.starts_with("model_request_duration_ms:")
        || tag.starts_with("input_tokens:")
        || tag.starts_with("cached_input_tokens:")
        || tag.starts_with("output_tokens:")
        || tag.starts_with("reasoning_output_tokens:")
        || tag.starts_with("total_tokens:")
        || tag.starts_with("provider_payload_sha256:")
        || tag.starts_with("provider_payload_bytes:")
        || tag.starts_with("provider_wire_api:")
        || tag.starts_with("tools_count:")
        || tag.starts_with("tools_present:")
        || tag.starts_with("request_shape_classifier:")
        || tag.starts_with("messages_hash:")
        || tag.starts_with("stable_prefix_hash:")
        || tag.starts_with("dynamic_suffix_hash:")
        || tag.starts_with("exact_payload_scan_passed:")
        || tag.starts_with("active_projection_present:")
        || tag.starts_with("active_projection_count:")
        || tag.starts_with("large_raw_output_tokens:")
        || tag.starts_with("protected_items_present:")
        || tag.starts_with("replacement_confirmed:")
        || tag.starts_with("exact_payload_scan_event_id:")
        || tag.starts_with("scan_event_id:")
        || tag.starts_with("scanner_version:")
        || tag.starts_with("matcher_version:")
        || tag.starts_with("checked_byte_ranges:")
        || tag.starts_with("negative_checks_performed:")
        || tag.starts_with("passed:")
        || tag.starts_with("failure_reasons:")
        || tag.starts_with("spawn_agent_call_count_")
        || tag.starts_with("max_spawn_agent_calls:")
        || tag.starts_with("max_subagent_results:")
        || tag.starts_with("node_count")
        || tag.starts_with("max_nodes:")
        || tag.starts_with("max_open_leaf_nodes:")
        || tag.starts_with("budget_kind:")
        || tag.starts_with("action:")
        || tag.starts_with("budget_state:")
        || tag.starts_with("budget_gate_reason:")
        || tag.starts_with("reason:")
        || tag.starts_with("projection_tokens")
        || tag.starts_with("max_projection_tokens:")
}

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let mut nodes = map
        .nodes
        .values()
        .map(|node| ActionMapSnapshotNode {
            id: node.id.clone(),
            title: node.title.clone(),
            kind: node.kind.as_str().to_string(),
            canonical_kind: node.kind.canonical_kind().to_string(),
            status: node.status.as_str().to_string(),
            context_summary: node.context.summary.clone(),
            source_refs: node.context.source_refs.clone(),
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
            origin_node_id: node.origin_node_id.clone(),
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
        title: map.title.clone(),
        status: map.status.as_str().to_string(),
        owner_session_id: map.owner_session_id,
        base_map_version: map.base_map_version.clone(),
        created_from: map.created_from.clone(),
        ready_node_count: map.ready_node_count(),
        running_node_count: map.running_node_count(),
        completed_node_count: map.completed_node_count(),
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
            let title = map
                .nodes
                .get(node_id)
                .map(|node| single_line_preview(&node.title, 80))
                .unwrap_or_default();
            format!("{node_id} ({title})")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn refresh_ready_nodes(map: &mut ActionMapInstance) -> Vec<MapRuntimeEvent> {
    if map.status != MapStatus::Active {
        return Vec::new();
    }
    let mut events = Vec::new();
    let pending_ids = map
        .nodes
        .iter()
        .filter_map(|(id, node)| (node.status == NodeStatus::Pending).then_some(id.clone()))
        .collect::<Vec<_>>();
    for node_id in pending_ids {
        let deps = map
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .map(|edge| edge.from.clone())
            .collect::<Vec<_>>();
        let ready = !deps.is_empty()
            && map.nodes.get(&node_id).is_some_and(|node| {
                deps.iter()
                    .all(|dependency_id| node_dependency_ready(map, node, dependency_id))
            });
        if ready && let Some(node) = map.nodes.get_mut(&node_id) {
            let previous_status = node.status;
            node.status = NodeStatus::Ready;
            events.push(node_status_changed_event(
                &map.id,
                &node.id,
                &node.title,
                previous_status,
                node.status,
            ));
        }
    }
    events
}

fn refresh_ready_validation_rework_nodes(
    map: &mut ActionMapInstance,
    validation_node_id: &str,
) -> Vec<MapRuntimeEvent> {
    if map.status != MapStatus::Active
        || !map.nodes.get(validation_node_id).is_some_and(|node| {
            matches!(node.kind, NodeKind::SmokeTest | NodeKind::RegressionTest)
                && node.status == NodeStatus::Blocked
        })
    {
        return Vec::new();
    }
    let mut events = Vec::new();
    let pending_ids = map
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            (node.kind == NodeKind::ImplementSolution
                && node.status == NodeStatus::Pending
                && node.origin_node_id.as_deref() == Some(validation_node_id))
            .then_some(id.clone())
        })
        .collect::<Vec<_>>();
    for node_id in pending_ids {
        let deps = map
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .map(|edge| edge.from.clone())
            .collect::<Vec<_>>();
        let ready = !deps.is_empty()
            && map.nodes.get(&node_id).is_some_and(|node| {
                deps.iter()
                    .all(|dependency_id| node_dependency_ready(map, node, dependency_id))
            });
        if ready && let Some(node) = map.nodes.get_mut(&node_id) {
            let previous_status = node.status;
            node.status = NodeStatus::Ready;
            events.push(node_status_changed_event(
                &map.id,
                &node.id,
                &node.title,
                previous_status,
                node.status,
            ));
        }
    }
    events
}

fn node_dependency_ready(map: &ActionMapInstance, node: &MapNode, dependency_id: &str) -> bool {
    map.nodes.get(dependency_id).is_some_and(|dependency| {
        dependency.status == NodeStatus::Completed
            || new_node_dependency_ready_from_status(
                map,
                node.kind,
                node.origin_node_id.as_deref(),
                dependency_id,
                dependency.status,
            )
    })
}

fn new_node_dependency_ready_from_status(
    map: &ActionMapInstance,
    node_kind: NodeKind,
    origin_node_id: Option<&str>,
    dependency_id: &str,
    dependency_status: NodeStatus,
) -> bool {
    node_kind == NodeKind::ImplementSolution
        && origin_node_id == Some(dependency_id)
        && dependency_status == NodeStatus::Blocked
        && map.nodes.get(dependency_id).is_some_and(|dependency| {
            matches!(
                dependency.kind,
                NodeKind::SmokeTest | NodeKind::RegressionTest
            )
        })
}

fn dependencies_will_be_completed_after_finish(
    map: &ActionMapInstance,
    finishing_node_id: &str,
    target_node_id: &str,
) -> bool {
    let deps = map
        .edges
        .iter()
        .filter(|edge| edge.to == target_node_id)
        .map(|edge| edge.from.as_str())
        .collect::<Vec<_>>();
    !deps.is_empty()
        && deps.iter().all(|dependency_id| {
            *dependency_id == finishing_node_id
                || map
                    .nodes
                    .get(*dependency_id)
                    .is_some_and(|node| node.status == NodeStatus::Completed)
        })
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

fn map_created_event(map: &ActionMapInstance) -> MapRuntimeEvent {
    MapRuntimeEvent::MapCreated(MapRuntimeMapCreatedEvent {
        map_id: map.id.clone(),
        title: map.title.clone(),
        owner_session_id: map.owner_session_id,
        created_from: map.created_from.clone(),
    })
}

fn task_created_event(task: &TaskState) -> MapRuntimeEvent {
    MapRuntimeEvent::TaskCreated(MapRuntimeTaskCreatedEvent {
        task_id: task.id.clone(),
        title: task.title.clone(),
        source_event_ids: task.source_event_ids.clone(),
        owner_session_id: task.owner_session_id,
        active_map_id: task.active_map_id.clone(),
    })
}

fn node_status_changed_event(
    map_id: &str,
    node_id: &str,
    node_title: &str,
    previous_status: NodeStatus,
    current_status: NodeStatus,
) -> MapRuntimeEvent {
    MapRuntimeEvent::NodeStatusChanged(MapRuntimeNodeStatusChangedEvent {
        map_id: map_id.to_string(),
        node_id: node_id.to_string(),
        node_title: node_title.to_string(),
        previous_status: previous_status.as_str().to_string(),
        current_status: current_status.as_str().to_string(),
    })
}

fn maintenance_barrier_cleared_event(
    barrier: &ActionMapMaintenanceBarrier,
    reason: impl Into<String>,
) -> MapRuntimeEvent {
    MapRuntimeEvent::MaintenanceBarrierCleared(MapRuntimeMaintenanceBarrierClearedEvent {
        map_id: barrier.map_id.clone(),
        node_id: barrier.node_id.clone(),
        reason: reason.into(),
    })
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

fn assignment_prompt(
    map_id: &str,
    node_id: &str,
    node_title: &str,
    node_context_summary: &str,
    node_kind: NodeKind,
    lease_id: &str,
) -> String {
    format!(
        "TaskSpace node assignment\n\
Map: {map_id}\n\
Node: {node_id} - {node_title}\n\
Node context: {node_context_summary}\n\
Node kind: {}\n\
Lease: {lease_id}\n\
\n\
Runtime map notes:\n\
- tool calls and results are recorded under this leased node.\n\n",
        node_kind.as_str(),
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
#[path = "runtime_tests.rs"]
mod tests;
