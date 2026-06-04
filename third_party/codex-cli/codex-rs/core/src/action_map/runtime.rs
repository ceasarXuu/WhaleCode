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
use codex_protocol::protocol::ActionMapSnapshotResult;
use codex_protocol::protocol::ActionMapSnapshotTask;
use codex_protocol::protocol::ActionMapSnapshotTraceEventRef;
use codex_protocol::protocol::ActionMapSnapshotTraceSummary;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeLeaseAttachedEvent;
use codex_protocol::protocol::MapRuntimeLeaseCreatedEvent;
use codex_protocol::protocol::MapRuntimeLeaseReleasedEvent;
use codex_protocol::protocol::MapRuntimeMaintenanceBarrierClearedEvent;
use codex_protocol::protocol::MapRuntimeMaintenanceBarrierRaisedEvent;
use codex_protocol::protocol::MapRuntimeMapCreatedEvent;
#[cfg(test)]
use codex_protocol::protocol::MapRuntimeMapStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeModeChangedEvent;
use codex_protocol::protocol::MapRuntimeNodeResultRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTaskCreatedEvent;
use codex_protocol::protocol::MapRuntimeTaskRoutedEvent;
use codex_protocol::protocol::MapRuntimeTaskStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTimeoutSummaryRequestedEvent;
use codex_protocol::protocol::MapRuntimeToolActionBlockedEvent;
use codex_protocol::protocol::MapRuntimeTraceEventRecordedEvent;

use super::basemap::BASE_MAP;
use super::basemap::base_map_metadata_prompt;
use super::basemap::node_kind_selection_prompt;
use super::contracts::contract_for;
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

const SEED_NODE_IDS: &[&str] = &[
    "define_scope",
    "inspect_code_context",
    "design_solution",
    "implement_solution",
    "smoke_test",
    "final_synthesis",
];

/// Test fixture budget for the default inspect-node contract.
#[cfg(test)]
pub(crate) const MAIN_TOOL_RESULT_BUDGET_PER_NODE: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapGateError {
    message: String,
    events: Vec<MapRuntimeEvent>,
}

impl ActionMapGateError {
    fn new(message: impl Into<String>, events: Vec<MapRuntimeEvent>) -> Self {
        Self {
            message: message.into(),
            events,
        }
    }

    pub(crate) fn into_parts(self) -> (String, Vec<MapRuntimeEvent>) {
        (self.message, self.events)
    }
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
    next_task_seq: u64,
    next_map_seq: u64,
    next_node_seq: u64,
    next_lease_seq: u64,
    next_result_seq: u64,
    next_trace_event_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MainToolReservation {
    map_id: ActionMapId,
    node_id: MapNodeId,
    lease_id: AssignmentLeaseId,
    tool_name: String,
    action_class: ActionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChildToolReservation {
    child_thread_id: ThreadId,
    map_id: ActionMapId,
    node_id: MapNodeId,
    lease_id: AssignmentLeaseId,
    tool_name: String,
    action_class: ActionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MainToolTraceDraft {
    task_id: Option<TaskId>,
    map_id: ActionMapId,
    node_id: MapNodeId,
    result_id: NodeResultId,
    call_id: String,
    tool_name: String,
    action_class: Option<ActionClass>,
    tool_success: bool,
    created_at_ms: i64,
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
            next_task_seq: 1,
            next_map_seq: 1,
            next_node_seq: 1,
            next_lease_seq: 1,
            next_result_seq: 1,
            next_trace_event_seq: 1,
        }
    }
}

impl ActionMapRuntimeState {
    #[cfg(test)]
    pub(crate) fn mode(&self) -> MapRuntimeMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: MapRuntimeMode) -> SetMapRuntimeModeOutcome {
        let previous_mode = self.mode;
        self.mode = mode;
        if previous_mode != mode {
            self.pending_transition_notice = Some(transition_notice(previous_mode, mode));
            if mode == MapRuntimeMode::Experiment {
                self.routing_required = true;
                self.bootstrap_required = self.tasks.is_empty();
            } else {
                self.routing_required = false;
                self.bootstrap_required = false;
                self.reborn_requested = false;
            }
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
        _owner_session_id: ThreadId,
    ) -> (SetTaskSpaceModeOutcome, Vec<MapRuntimeEvent>) {
        let mode_outcome = self.set_mode(mode);
        (
            SetTaskSpaceModeOutcome {
                mode: mode_outcome,
                active_map_id: self.active_map_id.clone(),
            },
            Vec::new(),
        )
    }

    pub(crate) fn restore_mode(&mut self, mode: MapRuntimeMode) {
        self.mode = mode;
        self.pending_transition_notice = None;
        self.routing_required = mode == MapRuntimeMode::Experiment && self.active_task_id.is_none();
        self.bootstrap_required = mode == MapRuntimeMode::Experiment && self.tasks.is_empty();
        self.reborn_requested = false;
    }

    pub(crate) fn begin_user_turn(&mut self) -> bool {
        if self.mode != MapRuntimeMode::Experiment {
            return false;
        }
        let previous = (self.routing_required, self.bootstrap_required);
        self.routing_required = true;
        if self.tasks.is_empty() {
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
        if self.tasks.is_empty() {
            self.bootstrap_required = true;
        }
        events
    }

    pub(crate) fn restore_snapshot(&mut self, snapshot: ActionMapSnapshot) {
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
                        objective: task.objective,
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
                                body: result.body,
                                source_thread_id: result.source_thread_id,
                                created_at_ms: result.created_at_ms,
                            },
                        ))
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

        self.maintenance_barriers = snapshot
            .maintenance_barriers
            .into_iter()
            .filter_map(|barrier| {
                let reason = maintenance_barrier_reason_from_str(&barrier.reason)?;
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
        self.next_result_seq = next_numeric_seq(
            self.maps.values().flat_map(|map| map.results.keys()),
            "result-",
        );
        self.next_trace_event_seq = next_numeric_seq(
            self.taskspace_trace_events.iter().map(|event| &event.id),
            "trace-",
        );
        if self.mode != MapRuntimeMode::Experiment {
            self.routing_required = false;
            self.bootstrap_required = false;
            self.reborn_requested = false;
        } else {
            self.bootstrap_required = self.bootstrap_required || self.tasks.is_empty();
            self.routing_required =
                self.routing_required || self.bootstrap_required || self.active_task_id.is_none();
        }
    }

    pub(crate) fn take_pending_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }

    #[cfg(test)]
    pub(crate) fn create_seed_map(
        &mut self,
        id: ActionMapId,
        title: String,
        owner_session_id: Option<ThreadId>,
    ) -> ActionMapId {
        let mut map = seed_map(id.clone(), title, owner_session_id, None);
        let task_id = self.ensure_active_task_state(owner_session_id, &map.title);
        self.register_map_to_task(&task_id, &id);
        map.task_id = Some(task_id);
        self.active_map_id = Some(id.clone());
        self.current_main_node_id = first_open_node_id(&map);
        self.current_main_lease_id = None;
        self.maps.insert(id.clone(), map);
        id
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
        }
    }

    #[cfg(test)]
    pub(crate) fn restart_active_map(
        &mut self,
        owner_session_id: ThreadId,
        title: impl Into<String>,
    ) -> (Option<ActionMapId>, ActionMapId, Vec<MapRuntimeEvent>) {
        let mut events = Vec::new();
        let previous_id = self.active_map_id.clone();
        if let Some(previous_id) = previous_id.as_ref()
            && let Some(map) = self.maps.get_mut(previous_id)
        {
            let previous_status = map.status;
            map.status = MapStatus::Abandoned;
            events.push(map_status_changed_event(map, previous_status));
        }
        if let Some(previous_id) = previous_id.as_ref()
            && let Some(barrier) = self.maintenance_barriers.remove(previous_id)
        {
            events.push(maintenance_barrier_cleared_event(&barrier, "map_restarted"));
        }

        let new_id = self.next_map_id();
        let map_title = title.into();
        let task_id = self.ensure_active_task_state(Some(owner_session_id), &map_title);
        let mut release_events = self
            .release_current_main_lease("map_restarted")
            .expect("test restart should not run while main tool calls are in flight");
        events.append(&mut release_events);
        let mut map = ActionMapInstance::new(
            new_id.clone(),
            map_title,
            Some(owner_session_id),
            BASE_MAP.version,
        );
        map.task_id = Some(task_id.clone());
        map.created_from = previous_id.clone();
        self.register_map_to_task(&task_id, &new_id);
        self.active_map_id = Some(new_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        events.push(map_created_event(&map));
        self.maps.insert(new_id.clone(), map);
        (previous_id, new_id, events)
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
        if descriptor.action_class != ActionClass::Control {
            self.validate_broad_inspect_delegation(owner_session_id)?;
        }
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no active task path exists.")
        })?;
        let node_id = self.current_main_node_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no current node binding exists.")
        })?;
        let lease_id = self.current_main_lease_id.clone().ok_or_else(|| {
            ActionMapGateError::from("TaskSpace mode is active but no current main lease exists.")
        })?;
        let map = self.maps.get(&map_id).ok_or_else(|| {
            ActionMapGateError::from(format!("TaskSpace active task path `{map_id}` is missing."))
        })?;
        let node = map.nodes.get(&node_id).ok_or_else(|| {
            ActionMapGateError::from(format!("TaskSpace current node `{node_id}` is missing."))
        })?;
        let contract = contract_for(node.kind);
        let main_tool_result_count = count_node_results_of_kind(node, NodeResultKind::MainToolCall);
        let reserved_tool_count = self.reserved_tool_calls_for_node(&map_id, &node_id);
        let budget = contract.max_main_tool_results_before_split_hint;
        if !contract.allows(descriptor.action_class) {
            let reason = format!(
                "{} does not allow {}",
                node.kind.as_str(),
                descriptor.action_class.as_str()
            );
            let message = format!(
                "TaskSpace blocked this tool call. Current node `{}` kind: {}. Requested tool `{}` action class: {}. Reason: {}. Call taskspace_control(action=finish_node) to finish the current node and bind or create a suitable node before retrying.",
                node.id,
                node.kind.as_str(),
                descriptor.tool_name,
                descriptor.action_class.as_str(),
                reason
            );
            return Err(ActionMapGateError::new(
                message,
                vec![MapRuntimeEvent::ToolActionBlocked(
                    MapRuntimeToolActionBlockedEvent {
                        map_id,
                        node_id,
                        node_kind: node.kind.as_str().to_string(),
                        tool_name: descriptor.tool_name,
                        action_class: descriptor.action_class.as_str().to_string(),
                        reason,
                    },
                )],
            ));
        }
        if descriptor.action_class != ActionClass::Control
            && main_tool_result_count + reserved_tool_count >= budget
        {
            let barrier = ActionMapMaintenanceBarrier {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                reason: MaintenanceBarrierReason::NodeToolResultBudgetExceeded,
                result_count: main_tool_result_count + reserved_tool_count,
                budget,
            };
            let mut events = Vec::new();
            if !self.maintenance_barriers.contains_key(&map_id) {
                events.push(maintenance_barrier_raised_event(&barrier));
                self.maintenance_barriers.insert(map_id.clone(), barrier);
            }
            return Err(ActionMapGateError::new(
                format!(
                    "TaskSpace blocked this tool call because current node `{}` already has {} recorded and {} in-flight tool results (budget {}). Finish this broad node or create a narrower follow-up node before retrying. If the remaining work contains independent investigation tracks, create separate ready inspect_code_context nodes and use spawn_agent for those nodes instead of continuing all investigation in the main node.",
                    node_id, main_tool_result_count, reserved_tool_count, budget
                ),
                events,
            ));
        }
        if descriptor.action_class != ActionClass::Control
            && let Some(call_id) = descriptor.call_id.as_deref()
        {
            self.reserve_main_tool_call(
                call_id,
                MainToolReservation {
                    map_id,
                    node_id,
                    lease_id,
                    tool_name: descriptor.tool_name,
                    action_class: descriptor.action_class,
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
        tool_name: &str,
        success: bool,
        preview: String,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
        self.record_main_tool_result_with_class(
            owner_session_id,
            call_id,
            tool_name,
            None,
            success,
            preview,
        )
    }

    pub(crate) fn record_main_tool_result_with_class(
        &mut self,
        owner_session_id: ThreadId,
        call_id: &str,
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        preview: String,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }

        let reservation = self.release_main_tool_reservation(call_id);
        let (map_id, node_id, lease_id, recorded_tool_name, recorded_action_class) = if let Some(
            reservation,
        ) =
            reservation
        {
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
            )
        };
        let result_id = self.next_result_id();
        let created_at_ms = now_ms();
        let body = format!(
            "Main tool call\n\
tool: {recorded_tool_name}\n\
call_id: {call_id}\n\
action_class: {}\n\
success: {success}\n\
preview:\n\
{preview}",
            recorded_action_class
                .map(ActionClass::as_str)
                .unwrap_or("unspecified")
        );
        let (task_id, raised_barrier) = {
            let map = self
                .maps
                .get_mut(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            let task_id = map.task_id.clone();
            let node = map
                .nodes
                .get_mut(&node_id)
                .ok_or_else(|| format!("TaskSpace current node `{node_id}` is missing."))?;

            let result = NodeResult {
                id: result_id.clone(),
                assignment_id: lease_id.clone(),
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                kind: NodeResultKind::MainToolCall,
                action_class: recorded_action_class,
                tool_success: Some(success),
                body,
                source_thread_id: owner_session_id,
                created_at_ms,
            };
            map.results.insert(result_id.clone(), result);
            node.result_context.push(NodeResultRef {
                id: result_id.clone(),
                kind: NodeResultKind::MainToolCall,
            });
            let main_tool_result_count =
                count_node_results_of_kind(node, NodeResultKind::MainToolCall);
            let budget = contract_for(node.kind).max_main_tool_results_before_split_hint;
            let raised_barrier = if main_tool_result_count >= budget
                && !self.maintenance_barriers.contains_key(&map_id)
            {
                Some(ActionMapMaintenanceBarrier {
                    map_id: map_id.clone(),
                    node_id: node_id.clone(),
                    reason: MaintenanceBarrierReason::NodeToolResultBudgetExceeded,
                    result_count: main_tool_result_count,
                    budget,
                })
            } else {
                None
            };
            (task_id, raised_barrier)
        };
        let trace_event = self.append_main_tool_trace_event(MainToolTraceDraft {
            task_id,
            map_id: map_id.clone(),
            node_id: node_id.clone(),
            result_id: result_id.clone(),
            call_id: call_id.to_string(),
            tool_name: recorded_tool_name,
            action_class: recorded_action_class,
            tool_success: success,
            created_at_ms,
        });
        let mut events = vec![MapRuntimeEvent::NodeResultRecorded(
            MapRuntimeNodeResultRecordedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id,
                result_id: result_id.clone(),
                kind: NodeResultKind::MainToolCall.as_str().to_string(),
                action_class: recorded_action_class.map(|class| class.as_str().to_string()),
                source_thread_id: owner_session_id,
            },
        )];
        events.push(trace_event);
        if let Some(barrier) = raised_barrier {
            events.push(maintenance_barrier_raised_event(&barrier));
            self.maintenance_barriers.insert(map_id.clone(), barrier);
        }
        Ok(Some((result_id, events)))
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
                    "TaskSpace blocked this subagent tool call because the subagent has no active task node lease. Spawn or reassign the subagent through the parent task map before retrying.",
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

        let contract = contract_for(node.kind);
        let tool_result_count = count_node_results_of_kind(node, NodeResultKind::MainToolCall);
        let reserved_tool_count = self.reserved_tool_calls_for_node(&map_id, &node_id);
        let budget = contract.max_main_tool_results_before_split_hint;
        if !contract.allows(descriptor.action_class) {
            let reason = format!(
                "{} does not allow {}",
                node.kind.as_str(),
                descriptor.action_class.as_str()
            );
            let message = format!(
                "TaskSpace blocked this subagent tool call. Node `{}` kind: {}. Requested tool `{}` action class: {}. Reason: {}. Return a blocker/result for the current node or ask the parent agent to create and assign a suitable node before retrying.",
                node.id,
                node.kind.as_str(),
                descriptor.tool_name,
                descriptor.action_class.as_str(),
                reason
            );
            return Err(ActionMapGateError::new(
                message,
                vec![MapRuntimeEvent::ToolActionBlocked(
                    MapRuntimeToolActionBlockedEvent {
                        map_id,
                        node_id,
                        node_kind: node.kind.as_str().to_string(),
                        tool_name: descriptor.tool_name,
                        action_class: descriptor.action_class.as_str().to_string(),
                        reason,
                    },
                )],
            ));
        }
        if descriptor.action_class != ActionClass::Control
            && tool_result_count + reserved_tool_count >= budget
        {
            let barrier = ActionMapMaintenanceBarrier {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                reason: MaintenanceBarrierReason::NodeToolResultBudgetExceeded,
                result_count: tool_result_count + reserved_tool_count,
                budget,
            };
            let mut events = Vec::new();
            if !self.maintenance_barriers.contains_key(&map_id) {
                events.push(maintenance_barrier_raised_event(&barrier));
                self.maintenance_barriers.insert(map_id.clone(), barrier);
            }
            return Err(ActionMapGateError::new(
                format!(
                    "TaskSpace blocked this subagent tool call because node `{}` already has {} recorded and {} in-flight tool results (budget {}). Return a result for this node and let the parent create a narrower follow-up node.",
                    node_id, tool_result_count, reserved_tool_count, budget
                ),
                events,
            ));
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
                    tool_name: descriptor.tool_name,
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
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        preview: String,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }

        let reservation = self.release_child_tool_reservation(child_thread_id, call_id);
        let (map_id, node_id, lease_id, recorded_tool_name, recorded_action_class) = if let Some(
            reservation,
        ) =
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
                reservation.tool_name,
                Some(reservation.action_class),
            )
        } else {
            let (map_id, lease_id, node_id) = match self.find_lease_by_thread(child_thread_id) {
                Some(target) => target,
                None => return Ok(None),
            };
            (
                map_id,
                node_id,
                lease_id,
                tool_name.to_string(),
                action_class,
            )
        };
        let result_id = self.next_result_id();
        let body = format!(
            "Subagent tool call\n\
agent_thread_id: {child_thread_id}\n\
tool: {recorded_tool_name}\n\
call_id: {call_id}\n\
action_class: {}\n\
success: {success}\n\
preview:\n\
{preview}",
            recorded_action_class
                .map(ActionClass::as_str)
                .unwrap_or("unspecified")
        );
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
            body,
            source_thread_id: child_thread_id,
            created_at_ms: now_ms(),
        };
        map.results.insert(result_id.clone(), result);
        node.result_context.push(NodeResultRef {
            id: result_id.clone(),
            kind: NodeResultKind::MainToolCall,
        });
        let mut events = vec![MapRuntimeEvent::NodeResultRecorded(
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
        let tool_result_count = count_node_results_of_kind(node, NodeResultKind::MainToolCall);
        let budget = contract_for(node.kind).max_main_tool_results_before_split_hint;
        if tool_result_count >= budget && !self.maintenance_barriers.contains_key(&map_id) {
            let barrier = ActionMapMaintenanceBarrier {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                reason: MaintenanceBarrierReason::NodeToolResultBudgetExceeded,
                result_count: tool_result_count,
                budget,
            };
            events.push(maintenance_barrier_raised_event(&barrier));
            self.maintenance_barriers.insert(map_id.clone(), barrier);
        }
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
        if let Some(barrier) = self.maintenance_barriers.get(&map_id)
            && barrier.node_id == node_id
        {
            return Err(format!(
                "TaskSpace maintenance barrier is active for node `{node_id}`; bind a different narrower recovery node or create a follow-up node with taskspace_control(action=create_node, bind_current=true)."
            ));
        }
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
                "TaskSpace current main node `{current_node_id}` is still running. Finish it with taskspace_control(action=finish_node) or block it with taskspace_control(action=block_node) before binding `{node_id}`."
            ));
        }
        if node.status == NodeStatus::Pending {
            return Err(format!(
                "TaskSpace node `{node_id}` is still pending; complete its dependencies before binding it."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace node `{node_id}` is completed; bind an open node or create a follow-up node."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace node `{node_id}` is currently held by a subagent lease; wait for release or bind a different node."
            ));
        }
        let parallel_inspect_node_ids = Self::ready_parallel_inspect_node_ids(map);
        if node.kind == NodeKind::InspectCodeContext
            && node.status == NodeStatus::Ready
            && parallel_inspect_node_ids.len() >= 2
        {
            return Err(format!(
                "TaskSpace has multiple ready inspect nodes. Do not bind one to the main agent for sequential investigation; call spawn_agent with an explicit node_id for each parallel investigation node: {}.",
                self.format_ready_spawn_node_candidates(&map_id, &parallel_inspect_node_ids)
            ));
        }
        let mut events = self.release_current_main_lease("main_rebound")?;
        events.extend(self.claim_main_node(owner_session_id, &map_id, node_id)?);
        events.extend(self.clear_maintenance_barrier_for_recovery(node_id));
        Ok(events)
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
            "TaskSpace blocked nested spawn_agent from a node-bound subagent. Return the current node result to the parent agent so the parent can create or assign another node."
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

    pub(crate) fn create_node_for_main_with_kind(
        &mut self,
        owner_session_id: ThreadId,
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
                "TaskSpace current main node `{current_node_id}` is still running. Finish it with taskspace_control(action=finish_node) or block it with taskspace_control(action=block_node) before creating and binding a new node."
            ));
        }
        if self.active_map().is_none()
            && dependency_node_ids
                .iter()
                .any(|dependency| !dependency.trim().is_empty())
        {
            return Err(
                "TaskSpace cannot create the first node with dependencies because no active task path exists yet."
                    .to_string(),
            );
        }
        let mut events = Vec::new();
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists. Call taskspace_control(action=start_task) to create a new semantic task before creating extra nodes."
                .to_string()
        })?;
        let node_id = self.next_node_id();
        let map = self
            .maps
            .get_mut(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let default_dependency_node_ids = default_dependency_node_ids_for_new_node(map);
        let dependency_node_ids = if dependency_node_ids.is_empty() {
            default_dependency_node_ids
        } else if bind_current {
            let mut merged = dependency_node_ids;
            for dependency in default_dependency_node_ids {
                if !merged.iter().any(|existing| existing == &dependency) {
                    merged.push(dependency);
                }
            }
            merged
        } else {
            dependency_node_ids
        };
        let dependency_node_ids = if bind_current && kind == NodeKind::ImplementSolution {
            let mut merged = dependency_node_ids;
            for dependency in completed_subagent_inspect_node_ids(map, owner_session_id) {
                if !merged.iter().any(|existing| existing == &dependency) {
                    merged.push(dependency);
                }
            }
            merged
        } else {
            dependency_node_ids
        };
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
                "TaskSpace cannot bind the main action to a new node until all dependencies are completed."
                    .to_string(),
            );
        }
        if bind_current && ready && kind == NodeKind::InspectCodeContext {
            let ready_parallel_inspect_node_ids = Self::ready_parallel_inspect_node_ids(map);
            if !ready_parallel_inspect_node_ids.is_empty() {
                return Err(format!(
                    "TaskSpace cannot create and bind a new inspect_code_context node because ready inspect nodes already exist: {}. Create the new inspect node with bind_current=false, or finish the current node without next_node_draft and call spawn_agent with explicit node_id for each ready inspect node.",
                    format_node_candidates(map, &ready_parallel_inspect_node_ids)
                ));
            }
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
        Ok((node_id, events))
    }

    #[allow(dead_code)]
    pub(crate) fn start_task_for_main(
        &mut self,
        owner_session_id: ThreadId,
        task_title: String,
        task_objective: String,
        node_title: String,
        node_context_summary: String,
        bind_current: bool,
    ) -> Result<(TaskId, ActionMapId, MapNodeId, Vec<MapRuntimeEvent>), String> {
        self.start_task_for_main_with_kind(
            owner_session_id,
            NodeKind::InspectCodeContext,
            task_title,
            task_objective,
            node_title,
            node_context_summary,
            bind_current,
        )
    }

    pub(crate) fn start_task_for_main_with_kind(
        &mut self,
        owner_session_id: ThreadId,
        node_kind: NodeKind,
        task_title: String,
        task_objective: String,
        node_title: String,
        node_context_summary: String,
        bind_current: bool,
    ) -> Result<(TaskId, ActionMapId, MapNodeId, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        validate_live_node_kind(node_kind)?;
        let task_title = task_title.trim();
        let task_objective = task_objective.trim();
        let node_title = node_title.trim();
        let node_context_summary = node_context_summary.trim();
        if task_title.is_empty() {
            return Err("TaskSpace task title cannot be empty.".to_string());
        }
        if node_title.is_empty() {
            return Err("TaskSpace first node title cannot be empty.".to_string());
        }
        if node_context_summary.is_empty() {
            return Err("TaskSpace first node context summary cannot be empty.".to_string());
        }

        let mut events = self.release_current_main_lease("task_started")?;
        if let Some(previous_task_id) = self.active_task_id.as_ref()
            && let Some(previous_task) = self.tasks.get_mut(previous_task_id)
        {
            let previous_status = previous_task.status;
            previous_task.status = TaskStatus::Pending;
            if previous_status != TaskStatus::Pending {
                events.push(task_status_changed_event(
                    previous_task_id,
                    previous_status,
                    TaskStatus::Pending,
                ));
            }
        }

        let task_id = self.next_task_id();
        let map_id = self.next_map_id();
        self.tasks.insert(
            task_id.clone(),
            TaskState {
                id: task_id.clone(),
                title: task_title.to_string(),
                objective: if task_objective.is_empty() {
                    task_title.to_string()
                } else {
                    task_objective.to_string()
                },
                status: TaskStatus::Active,
                owner_session_id: Some(owner_session_id),
                active_map_id: None,
                map_ids: Vec::new(),
            },
        );
        let mut map = ActionMapInstance::new(
            map_id.clone(),
            task_title.to_string(),
            Some(owner_session_id),
            BASE_MAP.version,
        );
        map.task_id = Some(task_id.clone());
        self.active_task_id = Some(task_id.clone());
        self.active_map_id = Some(map_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.maps.insert(map_id.clone(), map);
        self.register_map_to_task(&task_id, &map_id);
        self.mark_routing_complete();
        let task = self
            .tasks
            .get(&task_id)
            .expect("new TaskSpace task must exist before event emission");
        events.push(task_created_event(task));
        let map = self
            .maps
            .get(&map_id)
            .expect("new TaskSpace map must be inserted before event emission");
        events.push(map_created_event(map));

        let (node_id, mut node_events) = self.create_node_for_main_with_kind(
            owner_session_id,
            node_kind,
            node_title.to_string(),
            node_context_summary.to_string(),
            Vec::new(),
            bind_current,
        )?;
        events.append(&mut node_events);
        Ok((task_id, map_id, node_id, events))
    }

    pub(crate) fn route_task_for_main(
        &mut self,
        owner_session_id: ThreadId,
        task_id: &str,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Err("TaskSpace route_task task_id cannot be empty.".to_string());
        }

        let target_task = self
            .tasks
            .get(task_id)
            .ok_or_else(|| format!("TaskSpace task `{task_id}` does not exist."))?;
        if let Some(owner) = target_task.owner_session_id
            && owner != owner_session_id
        {
            return Err(format!(
                "TaskSpace task `{task_id}` is owned by another session and cannot be routed here."
            ));
        }
        let target_map_id = target_task.active_map_id.clone().ok_or_else(|| {
            format!("TaskSpace task `{task_id}` has no active task path to route to.")
        })?;
        let target_map = self
            .maps
            .get(&target_map_id)
            .ok_or_else(|| format!("TaskSpace task path `{target_map_id}` is missing."))?;
        if target_map.status != MapStatus::Active {
            return Err(format!(
                "TaskSpace task path `{target_map_id}` is not active and cannot be routed to."
            ));
        }

        if self.active_task_id.as_deref() == Some(task_id)
            && self.active_map_id.as_deref() == Some(target_map_id.as_str())
        {
            self.mark_routing_complete();
            return Ok(vec![MapRuntimeEvent::TaskRouted(
                MapRuntimeTaskRoutedEvent {
                    previous_task_id: Some(task_id.to_string()),
                    current_task_id: task_id.to_string(),
                    previous_map_id: Some(target_map_id.clone()),
                    current_map_id: target_map_id,
                },
            )]);
        }

        let previous_task_id = self.active_task_id.clone();
        let previous_map_id = self.active_map_id.clone();
        let mut events = self.release_current_main_lease("task_routed")?;
        if let Some(previous_task_id) = self.active_task_id.clone()
            && previous_task_id != task_id
            && let Some(previous_task) = self.tasks.get_mut(&previous_task_id)
        {
            let previous_status = previous_task.status;
            previous_task.status = TaskStatus::Pending;
            if previous_status != TaskStatus::Pending {
                events.push(task_status_changed_event(
                    &previous_task_id,
                    previous_status,
                    TaskStatus::Pending,
                ));
            }
        }
        let task = self
            .tasks
            .get_mut(task_id)
            .expect("target TaskSpace task was validated before routing");
        if task.owner_session_id.is_none() {
            task.owner_session_id = Some(owner_session_id);
        }
        let previous_status = task.status;
        task.status = TaskStatus::Active;
        if previous_status != TaskStatus::Active {
            events.push(task_status_changed_event(
                task_id,
                previous_status,
                TaskStatus::Active,
            ));
        }
        self.active_task_id = Some(task_id.to_string());
        self.active_map_id = Some(target_map_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.mark_routing_complete();
        events.push(MapRuntimeEvent::TaskRouted(MapRuntimeTaskRoutedEvent {
            previous_task_id,
            current_task_id: task_id.to_string(),
            previous_map_id,
            current_map_id: target_map_id,
        }));
        Ok(events)
    }

    #[allow(dead_code)]
    pub(crate) fn finish_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        result_summary: String,
        next_node_id: Option<String>,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        self.finish_main_node_with_next(
            owner_session_id,
            node_id,
            result_summary,
            next_node_id,
            None,
        )
    }

    pub(crate) fn finish_main_node_with_next(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        result_summary: String,
        next_node_id: Option<String>,
        next_node_draft: Option<ActionMapNextNodeDraft>,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        let result_summary = result_summary.trim();
        if result_summary.is_empty() {
            return Err("TaskSpace finish_node result_summary cannot be empty.".to_string());
        }
        let next_node_id = next_node_id
            .as_deref()
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        if next_node_id.is_some() && next_node_draft.is_some() {
            return Err(
                "TaskSpace finish_node cannot provide both next_node_id and next node draft fields."
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
        self.validate_completion_evidence(node_id)?;
        let (result_id, mut events) = self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Result,
            result_summary.to_string(),
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
        blocker_summary: String,
    ) -> Result<(NodeResultId, Vec<MapRuntimeEvent>), String> {
        let blocker_summary = blocker_summary.trim();
        if blocker_summary.is_empty() {
            return Err("TaskSpace block_node blocker_summary cannot be empty.".to_string());
        }
        self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Blocker,
            blocker_summary.to_string(),
            NodeStatus::Blocked,
            false,
        )
    }

    fn validate_completion_evidence(&self, node_id: &str) -> Result<(), String> {
        let map_id = self.active_map_id.as_ref().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        self.validate_completion_evidence_for(map_id, node_id)
    }

    fn validate_completion_evidence_for(&self, map_id: &str, node_id: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
        match node.kind {
            NodeKind::ImplementSolution => {
                if !node_has_successful_action(map, node, ActionClass::Edit) {
                    return Err(format!(
                        "TaskSpace implement_solution node `{node_id}` cannot be completed without a recorded successful edit action. Execute the edit in this node, or block the node if the edit cannot be made."
                    ));
                }
            }
            NodeKind::SmokeTest | NodeKind::RegressionTest => {
                if !node_has_successful_action(map, node, ActionClass::Test)
                    && !node_has_successful_action(map, node, ActionClass::Build)
                {
                    return Err(format!(
                        "TaskSpace {} node `{node_id}` cannot be completed without a recorded successful test or build action. Run validation in this node, or block it if validation fails and create a follow-up implementation node.",
                        node.kind.as_str()
                    ));
                }
            }
            NodeKind::InspectCodeContext | NodeKind::FinalSynthesis | NodeKind::Custom => {}
        }
        Ok(())
    }

    pub(crate) fn record_main_final_response(
        &mut self,
        owner_session_id: ThreadId,
        message: &str,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
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
        let node_id = match self.current_main_node_id.clone() {
            Some(node_id) => node_id,
            None => return Ok(None),
        };
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("TaskSpace current node `{node_id}` is missing."))?;
        if node.kind != NodeKind::FinalSynthesis || node.status != NodeStatus::Running {
            return Ok(None);
        }
        self.record_main_node_lifecycle_result(
            owner_session_id,
            &node_id,
            NodeResultKind::Result,
            message.to_string(),
            NodeStatus::Completed,
            true,
        )
        .map(Some)
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
                "TaskSpace mode is active but no active task path exists. Call taskspace_control(action=start_task) for a new task or taskspace_control(action=route_task) for an existing task before spawning a subagent."
                    .to_string(),
            );
        };
        if let Some(current_node_id) = self.current_main_node_id.as_deref() {
            let map = self
                .maps
                .get(&map_id)
                .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
            let node = map
                .nodes
                .get(current_node_id)
                .ok_or_else(|| format!("TaskSpace current node `{current_node_id}` is missing."))?;
            let contract = contract_for(node.kind);
            if !contract.allows(ActionClass::Spawn) {
                return Err(format!(
                    "TaskSpace blocked spawn_agent. Current node `{}` kind: {}. Requested action class: spawn. Reason: {} does not allow spawn. Call taskspace_control(action=finish_node) to finish the current node and bind or create a suitable node before retrying.",
                    node.id,
                    node.kind.as_str(),
                    node.kind.as_str()
                ));
            }
        }
        let requested_node_id = requested_node_id
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        let node_id = self.select_spawn_node_id(&map_id, requested_node_id)?;
        self.validate_spawn_parallelism(&map_id, &node_id)?;
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

        Ok((
            Some(ActionMapAssignment {
                message_prefix: assignment_prompt(
                    &map_id,
                    &node_id,
                    &node_title,
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
        let (mut kind, mut body) = result_from_status(status);
        if matches!(
            kind,
            NodeResultKind::Result | NodeResultKind::MapUpdateRequest
        ) && let Err(error) = self.validate_completion_evidence_for(&map_id, &node_id)
        {
            kind = NodeResultKind::Blocker;
            body = format!("Subagent result could not complete node because {error}");
        }
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
            body,
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

    pub(crate) fn build_developer_context(&self) -> Option<String> {
        if self.mode != MapRuntimeMode::Experiment {
            return None;
        }

        let mut context = String::from("TaskSpace mode is active.\n");
        context.push_str(
            "Runtime slash commands such as /task-reborn and /task-show are UI commands, not shell commands; do not run them via shell_command.\n",
        );
        context.push_str(
            "Before ordinary work, the agent must decide whether the user's current request belongs to an existing task or needs a new task. Runtime exposes task ids and validates structure only; the agent performs semantic task routing with taskspace_control(action=route_task) or taskspace_control(action=start_task).\n",
        );
        context.push_str(
            "Use the minimum sufficient task map. For a simple single-file or single-failure task, prefer one main-agent chain: inspect_code_context -> implement_solution -> smoke_test/regression_test -> final_synthesis. Do not create extra ready inspect nodes or call spawn_agent for simple work unless new evidence shows independent tracks that would materially reduce risk or context load.\n",
        );
        context.push_str(
            "For simple tasks, path correction and reading a small known set of files stay inside the current inspect node. Do not create another inspect node or call spawn_agent merely to read one known file, re-read a file, fix a guessed path, or serialize one evidence item.\n",
        );
        context.push_str(
            "Finish nodes with matching tool evidence, not only a written claim: implement_solution needs a successful edit action before finish_node; smoke_test/regression_test needs a successful test or build action before finish_node. If the needed action is impossible or fails, block the node or create a correctly typed follow-up node.\n",
        );
        context.push_str(
            "Pre-fix diagnostic tests that are expected to fail belong inside inspect_code_context as evidence gathering. Create smoke_test/regression_test nodes for post-implementation validation, not for a separate baseline-failure node on simple bug fixes.\n",
        );
        context.push_str(
            "During inspect_code_context, reconcile product docs, tests, and implementation before editing. If explicit product rules in README/spec docs conflict with existing test expectations, treat the tests as potentially stale, update code and tests together to match the documented rule, and record the rationale in the node result.\n",
        );
        context.push_str(
            "spawn_agent can only claim ready nodes; do not bind a node to the main agent and then hand it off.\n",
        );
        context.push_str(
            "For broad multi-module tasks, create separate inspect/review nodes for independent evidence gathering and delegate ready inspect/review nodes to explorer agents only when at least two independent areas can be checked in parallel and the coordination cost is justified. If the user asks to parallelize independent work, or if independent parser/pricing/review/etc. tracks are visible before editing, do not substitute main-agent parallel shell/file-change calls for collaboration; create the ready nodes and call spawn_agent for those nodes. Do not handle one independent investigation yourself while only one explorer handles the other; when two independent tracks exist, the main agent should coordinate and integrate while two explorer agents own the two investigation nodes. Leave those parallel inspect nodes ready for explorer agents instead of binding one to the main agent unless only one independent area exists. Inspect nodes may run diagnostic tests to gather evidence; keep implementation edits on implementation nodes and final passing validation on explicit test nodes.\n",
        );
        context.push_str(
            "During inspect/review nodes, discover exact paths before reading files. Prefer rg --files, Get-ChildItem -Name, or narrow directory listings; do not read guessed filenames from truncated shell output.\n",
        );
        context.push_str(
            "If a smoke_test or regression_test node reveals a failure that needs edits, record that test result on the test node, finish or block the test node, create or bind an implement_solution node for the fix, then finish that implementation node and create or bind a smoke_test/regression_test node to rerun validation. Do not enter final_synthesis while validation is missing or failing.\n",
        );
        context.push_str(node_kind_selection_prompt());
        context.push('\n');
        if self.bootstrap_required {
            context.push_str(
                "Bootstrap is required now: create the first semantic task with taskspace_control(action=start_task) before ordinary tools or subagent spawn.\n",
            );
        } else if self.routing_required {
            context.push_str(
                "Task routing is required for this user turn: call taskspace_control(action=route_task) for an existing task or taskspace_control(action=start_task) for a new semantic task before ordinary tools or subagent spawn.\n",
            );
        }
        if self.reborn_requested {
            context.push_str(
                "The user requested /task-reborn. Runtime will not create a replacement task path automatically; use taskspace_control to route or start a task before ordinary work, and do not continue the old path unless the user's follow-up intent clearly cancels the reborn request.\n",
            );
        }
        if self.tasks.is_empty() {
            context.push_str(
                "No TaskSpace tasks exist yet. Call taskspace_control(action=start_task) with a concrete first node derived from the user's current request before ordinary tools or subagent spawn.\n",
            );
        } else {
            context.push_str("Task inventory:\n");
            for task_id in ordered_task_ids(&self.tasks) {
                if let Some(task) = self.tasks.get(&task_id) {
                    context.push_str("- ");
                    context.push_str(&task.id);
                    context.push_str(" [");
                    context.push_str(task.status.as_str());
                    context.push_str("] ");
                    context.push_str(&task.title);
                    if let Some(map_id) = task.active_map_id.as_ref() {
                        context.push_str(" active_map=");
                        context.push_str(map_id);
                    }
                    context.push_str("\n  objective: ");
                    context.push_str(&single_line_preview(&task.objective, 180));
                    context.push('\n');
                }
            }
            context.push_str(
                "For each new user turn, route to one listed task if it is the same semantic task, or start_task if it is a new task. Do not use keyword matching in runtime terms; make the routing decision from the conversation and task objectives.\n",
            );
        }
        if let Some(barrier) = self.active_maintenance_barrier() {
            context.push_str("Maintenance barrier:\n- map: ");
            context.push_str(&barrier.map_id);
            context.push_str("\n- node: ");
            context.push_str(&barrier.node_id);
            context.push_str("\n- reason: ");
            context.push_str(barrier.reason.as_str());
            context.push_str("\n- main tool results: ");
            context.push_str(&barrier.result_count.to_string());
            context.push_str(" / budget ");
            context.push_str(&barrier.budget.to_string());
            context.push_str(
                "\nOrdinary tools and spawn_agent are blocked. Recover by using taskspace_control to create or bind a different narrower node, or stop and ask the user to restart/reframe the task.\n",
            );
        }
        if let Some(map) = self.active_map() {
            context.push_str("Active task path:\n");
            context.push_str("- id: ");
            context.push_str(&map.id);
            context.push_str("\n- title: ");
            context.push_str(&map.title);
            context.push_str("\n- status: active\n- ready nodes: ");
            context.push_str(&map.ready_node_count().to_string());
            context.push_str("\n- running nodes: ");
            context.push_str(&map.running_node_count().to_string());
            context.push_str("\n- completed nodes: ");
            context.push_str(&map.completed_node_count().to_string());
            if let Some(node_id) = self.current_main_node_id.as_ref()
                && let Some(node) = map.nodes.get(node_id)
            {
                context.push_str("\n- current main action node: ");
                context.push_str(node_id);
                context.push_str(" kind=");
                context.push_str(node.kind.as_str());
                let contract = contract_for(node.kind);
                context.push_str("\nCurrent node contract:\n- allowed action classes: ");
                context.push_str(
                    &contract
                        .allowed_actions
                        .iter()
                        .map(|action| action.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                context.push_str("\nBefore requesting a blocked action, call taskspace_control(action=finish_node) and bind or create a suitable next node. If the current node kind is implement_solution, shell test commands will be blocked; after implementation edits, finish the implementation node and create or bind a smoke_test or regression_test node before running tests.\n");
            }
            context.push_str("\nNodes:\n");
            for node_id in ordered_node_ids(map) {
                if let Some(node) = map.nodes.get(&node_id) {
                    context.push_str("- ");
                    context.push_str(&node.id);
                    context.push_str(": ");
                    context.push_str(&node.title);
                    context.push_str(" kind=");
                    context.push_str(node.kind.as_str());
                    context.push_str(" [");
                    context.push_str(node.status.as_str());
                    context.push_str("]\n");
                }
            }
            if map.nodes.is_empty() {
                context.push_str(
                    "No nodes exist yet. Before any ordinary tool call or subagent spawn, call taskspace_control(action=create_node) with a concrete node derived from the active task and bind_current=true for the main work node.\n",
                );
                context.push_str(&base_map_metadata_prompt());
            }
            context.push_str(
                "Every action must run on the active task path. Main-agent ordinary tool calls are attributed to the current main action node; subagent actions are bound to ready nodes at spawn time. spawn_agent can only claim ready nodes; do not bind a node to the main agent and then hand it off. If a subagent should own work, create that node with bind_current=false or finish/block the current main node first. If more than one ready node exists, spawn_agent must include node_id for the intended node; if only one ready node exists, runtime may bind it automatically. If a newly discovered subtask does not fit existing nodes, call taskspace_control(action=create_node) before doing that work. Node result context stays on the node; use it only when it is relevant to the next step. Do not spawn an agent merely because TaskSpace is active or because a node exists; spawn only when the node represents a bounded, independent track whose result the main agent will integrate. For inspect_code_context nodes, explorer spawn is for a parallel investigation group, not single-track outsourcing; create at least two ready independent inspect nodes before assigning explorer subagents.\n",
            );
            context.push_str(
                "When the task naturally separates into independent investigation tracks, proactively create separate inspect_code_context nodes and assign subagents instead of waiting for the user to ask for parallel work. Before doing so, verify that the tracks are actually independent and that each track has a distinct evidence surface. Keep dependency edges explicit: independent investigation nodes should not depend on each other, implementation nodes should depend on the investigation nodes they integrate, and validation/final nodes should depend on the implementation or validation predecessor they verify.\n",
            );
        } else {
            context.push_str(
                "No active task path exists. Before any ordinary tool call or subagent spawn, call taskspace_control(action=start_task) for a new semantic task or taskspace_control(action=route_task) for an existing listed task.\n",
            );
            context.push_str(&base_map_metadata_prompt());
        }
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

    #[cfg(test)]
    fn ensure_active_seed_map(
        &mut self,
        owner_session_id: ThreadId,
        title_hint: &str,
    ) -> Vec<MapRuntimeEvent> {
        if self.active_map().is_some() {
            return Vec::new();
        }
        let id = self.next_map_id();
        let title = if title_hint.trim().is_empty() {
            "TaskSpace Path".to_string()
        } else {
            format!("TaskSpace Path: {}", title_hint.trim())
        };
        let task_id = self.ensure_active_task_state(Some(owner_session_id), &title);
        let mut map = seed_map(id.clone(), title, Some(owner_session_id), None);
        map.task_id = Some(task_id.clone());
        self.register_map_to_task(&task_id, &id);
        self.active_map_id = Some(id.clone());
        self.current_main_node_id = first_open_node_id(&map);
        self.current_main_lease_id = None;
        let events = {
            let mut events = vec![map_created_event(&map)];
            events.extend(initial_node_events(&map));
            events
        };
        self.maps.insert(id, map);
        events
    }

    #[cfg(test)]
    fn ensure_active_task_state(
        &mut self,
        owner_session_id: Option<ThreadId>,
        title_hint: impl AsRef<str>,
    ) -> TaskId {
        if let Some(task_id) = self.active_task_id.clone()
            && let Some(task) = self.tasks.get_mut(&task_id)
        {
            let same_owner = owner_session_id.is_none()
                || task.owner_session_id.is_none()
                || task.owner_session_id == owner_session_id;
            if same_owner {
                if task.owner_session_id.is_none() {
                    task.owner_session_id = owner_session_id;
                }
                return task_id;
            }
            task.status = TaskStatus::Pending;
        }
        let task_id = self.next_task_id();
        let title_hint = title_hint.as_ref().trim();
        let title = if title_hint.is_empty() {
            "TaskSpace task".to_string()
        } else {
            title_hint.to_string()
        };
        self.tasks.insert(
            task_id.clone(),
            TaskState {
                id: task_id.clone(),
                title: title.clone(),
                objective: title,
                status: TaskStatus::Active,
                owner_session_id,
                active_map_id: None,
                map_ids: Vec::new(),
            },
        );
        self.active_task_id = Some(task_id.clone());
        task_id
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
                "TaskSpace node `{node_id}` is still pending; complete its dependencies before binding it."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace node `{node_id}` is completed; bind an open node or create a follow-up node."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace node `{node_id}` is currently held by another lease; wait for release or bind a different node."
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

    #[cfg(test)]
    fn ensure_main_binding_for_active_map(
        &mut self,
        owner_session_id: ThreadId,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let Some(map_id) = self.active_map_id.clone() else {
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            return Ok(Vec::new());
        };
        let Some(map) = self.maps.get(&map_id) else {
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            return Ok(Vec::new());
        };
        if map.status != MapStatus::Active {
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            return Ok(Vec::new());
        }
        if let Some(node_id) = self.current_main_node_id.as_ref()
            && let Some(node) = map.nodes.get(node_id)
            && node.status == NodeStatus::Running
            && let Some(lease_id) = self.current_main_lease_id.as_ref()
            && node.active_lease.as_deref() == Some(lease_id.as_str())
            && let Some(lease) = map.leases.get(lease_id)
            && lease.holder == LeaseHolder::Main
            && lease.agent_thread_id == Some(owner_session_id)
        {
            return Ok(Vec::new());
        }
        let Some(node_id) = first_open_node_id(map) else {
            self.current_main_node_id = None;
            self.current_main_lease_id = None;
            return Ok(Vec::new());
        };
        self.claim_main_node(owner_session_id, &map_id, &node_id)
    }

    fn validate_main_binding(&self, owner_session_id: ThreadId) -> Result<(), String> {
        self.validate_routing_complete()?;
        let Some(map_id) = self.active_map_id.as_ref() else {
            return Err(
                "TaskSpace mode is active but no active task path exists. Call taskspace_control(action=start_task) for a new task or taskspace_control(action=route_task) for an existing task before ordinary work."
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
        let Some(node_id) = self.current_main_node_id.as_ref() else {
            return Err(
                "TaskSpace mode is active but no current node binding exists. Call taskspace_control(action=create_node, bind_current=true) or taskspace_control(action=bind_node) before ordinary work."
                    .to_string(),
            );
        };
        let Some(node) = map.nodes.get(node_id) else {
            return Err(format!("TaskSpace current node `{node_id}` is missing."));
        };
        if node.status == NodeStatus::Pending {
            return Err(format!(
                "TaskSpace current node `{node_id}` is still pending; complete dependencies before ordinary work."
            ));
        }
        if node.status == NodeStatus::Completed {
            return Err(format!(
                "TaskSpace current node `{node_id}` is completed; bind an open node or create a follow-up node."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            let Some(lease_id) = self.current_main_lease_id.as_ref() else {
                return Err(format!(
                    "TaskSpace current node `{node_id}` is running without a main lease; bind a different node or restart the task path."
                ));
            };
            let Some(lease) = map.leases.get(lease_id) else {
                return Err(format!(
                    "TaskSpace current main lease `{lease_id}` is missing; bind a different node or restart the task path."
                ));
            };
            if lease.holder != LeaseHolder::Main
                || lease.node_id != *node_id
                || lease.agent_thread_id != Some(owner_session_id)
                || node.active_lease.as_deref() != Some(lease_id.as_str())
            {
                return Err(format!(
                    "TaskSpace current node `{node_id}` is not held by the current main agent lease; wait for release or bind a different node."
                ));
            }
            return Ok(());
        }
        Err(format!(
            "TaskSpace current node `{node_id}` has no main lease. Bind it with taskspace_control(action=bind_node) before ordinary work."
        ))
    }

    fn validate_routing_complete(&self) -> Result<(), String> {
        if self.bootstrap_required {
            return Err(
                "TaskSpace bootstrap is required for this turn. Call taskspace_control(action=start_task) with a concrete first node before ordinary work or subagent spawn."
                    .to_string(),
            );
        }
        if self.routing_required {
            return Err(
                "TaskSpace task routing is required for this user turn. Call taskspace_control(action=route_task) for an existing task or taskspace_control(action=start_task) for a new semantic task before ordinary work or subagent spawn."
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
        body: String,
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
            "TaskSpace mode is active but no current node binding exists. Call taskspace_control(action=create_node, bind_current=true) or taskspace_control(action=bind_node) before finishing or blocking a node."
                .to_string()
        })?;
        if current_node_id != node_id {
            if let Some(map) = self.maps.get(&map_id)
                && let Some(node) = map.nodes.get(node_id)
                && node.status == NodeStatus::Completed
            {
                return Err(format!(
                    "TaskSpace node `{node_id}` is already completed; do not finish it again. Continue from current main node `{current_node_id}` or create and bind a follow-up node."
                ));
            }
            return Err(format!(
                "TaskSpace node `{node_id}` is not the current main action node `{current_node_id}`. Bind it first with taskspace_control(action=bind_node)."
            ));
        }
        let current_lease_id = self.current_main_lease_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no current main lease exists. Bind the node before finishing or blocking it."
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
                    "TaskSpace node `{node_id}` is still pending; complete its dependencies before recording a main result."
                ));
            }
            if node.status == NodeStatus::Completed {
                return Err(format!(
                    "TaskSpace node `{node_id}` is already completed; create or bind an open node."
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
                body,
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
                "TaskSpace next node `{next_node_id}` is completed; bind an open node or create a follow-up node."
            ));
        }
        if node.status == NodeStatus::Running || node.active_lease.is_some() {
            return Err(format!(
                "TaskSpace next node `{next_node_id}` is held by a subagent lease; wait for release or choose a different node."
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
                "TaskSpace maintenance barrier is active for next node `{next_node_id}`; bind a different narrower recovery node or create a follow-up node."
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
        if draft.kind == NodeKind::InspectCodeContext {
            let ready_parallel_inspect_node_ids = Self::ready_parallel_inspect_node_ids(map);
            if !ready_parallel_inspect_node_ids.is_empty() {
                return Err(format!(
                    "TaskSpace cannot finish `{finishing_node_id}` and bind a new inspect_code_context node because ready inspect nodes already exist: {}. Finish the current node without next_node_draft, then call spawn_agent with explicit node_id for each ready inspect node, or create additional inspect nodes with bind_current=false.",
                    format_node_candidates(map, &ready_parallel_inspect_node_ids)
                ));
            }
        }
        Ok(())
    }

    fn validate_maintenance_barrier(&self) -> Result<(), String> {
        let Some(barrier) = self.active_maintenance_barrier() else {
            return Ok(());
        };
        Err(format!(
            "TaskSpace maintenance barrier is active for node `{}` on map `{}`: {} ({} main tool results, budget {}). Ordinary main-agent tools are blocked until you use taskspace_control to create or bind a different narrower node, or stop and ask the user to restart/reframe the task. For broad investigations, split independent tracks into ready inspect_code_context nodes and assign them with spawn_agent.",
            barrier.node_id,
            barrier.map_id,
            barrier.reason.as_str(),
            barrier.result_count,
            barrier.budget
        ))
    }

    fn validate_broad_inspect_delegation(&self, owner_session_id: ThreadId) -> Result<(), String> {
        let Some(map) = self.active_map() else {
            return Ok(());
        };
        if let Some(current_node_id) = self.current_main_node_id.as_deref()
            && let Some(current_node) = map.nodes.get(current_node_id)
            && current_node.kind != NodeKind::InspectCodeContext
        {
            return Ok(());
        }
        let has_subagent_work = map
            .leases
            .values()
            .any(|lease| lease.holder == LeaseHolder::SubAgent)
            || map
                .results
                .values()
                .any(|result| result.source_thread_id != owner_session_id);
        if has_subagent_work {
            return Ok(());
        }
        let has_broad_completed_inspect = map.nodes.values().any(|node| {
            node.kind == NodeKind::InspectCodeContext
                && node.status == NodeStatus::Completed
                && count_node_results_of_kind(node, NodeResultKind::MainToolCall)
                    >= contract_for(node.kind).max_main_tool_results_before_split_hint
        });
        if has_broad_completed_inspect {
            return Err(
                "TaskSpace blocked ordinary main-agent work because a broad inspect_code_context node already exhausted its main-tool budget without any subagent work. Delegate the remaining independent investigation tracks: create ready inspect_code_context nodes if needed, then call spawn_agent with explicit node_id before continuing ordinary tools."
                    .to_string(),
            );
        }
        Ok(())
    }

    fn validate_maintenance_barrier_for_node(&self, node_id: &str) -> Result<(), String> {
        let Some(barrier) = self.active_maintenance_barrier() else {
            return Ok(());
        };
        if barrier.node_id != node_id {
            return Ok(());
        }
        Err(format!(
            "TaskSpace maintenance barrier is active for node `{}` on map `{}`: {} ({} main tool results, budget {}). Select a different ready node or create a narrower recovery node.",
            barrier.node_id,
            barrier.map_id,
            barrier.reason.as_str(),
            barrier.result_count,
            barrier.budget
        ))
    }

    fn validate_maintenance_barrier_for_map_node(
        &self,
        map_id: &str,
        node_id: &str,
    ) -> Result<(), ActionMapGateError> {
        let Some(barrier) = self.maintenance_barriers.get(map_id) else {
            return Ok(());
        };
        if barrier.node_id != node_id {
            return Ok(());
        }
        Err(ActionMapGateError::from(format!(
            "TaskSpace maintenance barrier is active for node `{}` on map `{}`: {} ({} tool results, budget {}). The subagent must return a result/blocker and let the parent create or bind a recovery node.",
            barrier.node_id,
            barrier.map_id,
            barrier.reason.as_str(),
            barrier.result_count,
            barrier.budget
        )))
    }

    fn reserved_main_tool_calls(&self, map_id: &str, node_id: &str) -> usize {
        self.main_tool_reservations
            .values()
            .filter(|reservation| reservation.map_id == map_id && reservation.node_id == node_id)
            .count()
    }

    fn reserved_child_tool_calls(&self, map_id: &str, node_id: &str) -> usize {
        self.child_tool_reservations
            .values()
            .filter(|reservation| reservation.map_id == map_id && reservation.node_id == node_id)
            .count()
    }

    fn reserved_tool_calls_for_node(&self, map_id: &str, node_id: &str) -> usize {
        self.reserved_main_tool_calls(map_id, node_id)
            + self.reserved_child_tool_calls(map_id, node_id)
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
            "TaskSpace node `{node_id}` has {in_flight} in-flight main tool call(s); wait for them to finish before {action}."
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
            [] => Err(
                "TaskSpace mode is active, but no ready node is available. Wait for running nodes to finish, ask the user for missing context, or reborn the task path with /task-reborn."
                    .to_string(),
            ),
            _ => Err(format!(
                "TaskSpace mode has multiple ready nodes. Call spawn_agent with an explicit node_id so the subagent is bound to the intended node: {}.",
                self.format_ready_spawn_node_candidates(map_id, &ready_nodes)
            )),
        }
    }

    fn ready_parallel_inspect_node_ids(map: &ActionMapInstance) -> Vec<String> {
        ordered_node_ids(map)
            .into_iter()
            .filter(|node_id| {
                map.nodes.get(node_id).is_some_and(|node| {
                    node.kind == NodeKind::InspectCodeContext
                        && node.status == NodeStatus::Ready
                        && node.active_lease.is_none()
                })
            })
            .collect()
    }

    fn active_subagent_inspect_node_count(map: &ActionMapInstance) -> usize {
        map.leases
            .values()
            .filter(|lease| lease.holder == LeaseHolder::SubAgent)
            .filter(|lease| {
                map.nodes
                    .get(&lease.node_id)
                    .is_some_and(|node| node.kind == NodeKind::InspectCodeContext)
            })
            .count()
    }

    fn validate_spawn_parallelism(&self, map_id: &str, node_id: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        let Some(node) = map.nodes.get(node_id) else {
            return Ok(());
        };
        if node.kind != NodeKind::InspectCodeContext {
            return Ok(());
        }
        let parallel_capacity = Self::ready_parallel_inspect_node_ids(map).len()
            + Self::active_subagent_inspect_node_count(map);
        if parallel_capacity >= 2 {
            return Ok(());
        }
        let main_holds_running_inspect = self
            .current_main_node_id
            .as_deref()
            .and_then(|current_node_id| map.nodes.get(current_node_id))
            .is_some_and(|current_node| {
                current_node.kind == NodeKind::InspectCodeContext
                    && current_node.status == NodeStatus::Running
                    && current_node.active_lease.is_some()
            });
        let main_inspect_is_barriered = self
            .active_maintenance_barrier()
            .zip(self.current_main_node_id.as_deref())
            .is_some_and(|(barrier, current_node_id)| barrier.node_id == current_node_id);
        if main_holds_running_inspect && !main_inspect_is_barriered {
            return Err(format!(
                "TaskSpace blocked spawn_agent for inspect node `{node_id}` because the main agent is already holding an inspect track and only one ready inspect track is available. The main agent should coordinate parallel inspect work, not own one track while a single explorer owns the other; finish the current inspect or create at least two ready independent inspect_code_context nodes before assigning explorer subagents."
            ));
        }
        let has_completed_narrow_inspect = map.nodes.values().any(|node| {
            let main_tool_results = count_node_results_of_kind(node, NodeResultKind::MainToolCall);
            node.kind == NodeKind::InspectCodeContext
                && node.status == NodeStatus::Completed
                && main_tool_results > 0
                && main_tool_results
                    < contract_for(node.kind).max_main_tool_results_before_split_hint
        });
        if !has_completed_narrow_inspect {
            return Ok(());
        }
        Err(format!(
            "TaskSpace blocked spawn_agent for inspect node `{node_id}` because a completed narrow inspect node already exists and only one follow-up inspect track is available. Keep serial single-track investigation on the main agent; create at least two ready independent inspect_code_context nodes before assigning explorer subagents."
        ))
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
                "TaskSpace node `{node_id}` is already held by an active lease; wait for release or choose another ready node."
            ));
        }
        match node.status {
            NodeStatus::Ready => Ok(()),
            NodeStatus::Pending => Err(format!(
                "TaskSpace node `{node_id}` is still pending; complete its dependencies before assigning it to a subagent."
            )),
            NodeStatus::Running => Err(format!(
                "TaskSpace node `{node_id}` is already running; wait for release or choose another ready node."
            )),
            NodeStatus::Blocked => Err(format!(
                "TaskSpace node `{node_id}` is blocked; resolve or split the blocker before assigning it to a subagent."
            )),
            NodeStatus::Completed => Err(format!(
                "TaskSpace node `{node_id}` is completed; create or choose an open ready node for the subagent."
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

    fn append_main_tool_trace_event(&mut self, draft: MainToolTraceDraft) -> MapRuntimeEvent {
        let id = self.next_trace_event_id();
        let tags = trace_tags_for(draft.action_class, draft.tool_success, &draft.tool_name);
        let event = TaskSpaceTraceEvent {
            id: id.clone(),
            kind: "main_tool_result".to_string(),
            task_id: draft.task_id,
            map_id: draft.map_id,
            node_id: draft.node_id,
            result_id: Some(draft.result_id),
            call_id: Some(draft.call_id),
            action_class: draft.action_class,
            tool_success: Some(draft.tool_success),
            tags,
            artifact_refs: Vec::new(),
            created_at_ms: draft.created_at_ms,
        };
        self.taskspace_trace_events.push(event.clone());
        MapRuntimeEvent::TaskspaceTraceEventRecorded(MapRuntimeTraceEventRecordedEvent {
            trace_event_id: id,
            kind: event.kind,
            task_id: event.task_id,
            map_id: event.map_id,
            node_id: event.node_id,
            result_id: event.result_id,
            call_id: event.call_id,
            action_class: event.action_class.map(|class| class.as_str().to_string()),
            tool_success: event.tool_success,
            tags: event.tags,
            artifact_refs: event.artifact_refs,
            created_at_ms: event.created_at_ms,
        })
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
                output.push_str("\n  ");
                output.push_str(&single_line_preview(&result.body, 220));
                output.push('\n');
            }
        }
    }

    output
}

#[cfg(test)]
fn seed_map(
    id: ActionMapId,
    title: String,
    owner_session_id: Option<ThreadId>,
    created_from: Option<ActionMapId>,
) -> ActionMapInstance {
    let mut map = ActionMapInstance::new(id, title, owner_session_id, BASE_MAP.version);
    map.created_from = created_from;
    for (index, node_id) in SEED_NODE_IDS.iter().enumerate() {
        let candidate = BASE_MAP
            .candidate_nodes
            .iter()
            .find(|candidate| candidate.id == *node_id)
            .expect("seed node must exist in BaseMap metadata");
        map.nodes.insert(
            (*node_id).to_string(),
            MapNode {
                id: (*node_id).to_string(),
                title: candidate.title.to_string(),
                kind: NodeKind::from_node_id_or_title(candidate.id, candidate.title),
                status: if index == 0 {
                    NodeStatus::Ready
                } else {
                    NodeStatus::Pending
                },
                context: NodeContext {
                    summary: candidate.when_to_use.to_string(),
                    source_refs: Vec::new(),
                },
                active_lease: None,
                result_context: Vec::new(),
                origin_node_id: Some((*node_id).to_string()),
            },
        );
    }
    for pair in SEED_NODE_IDS.windows(2) {
        map.edges.push(MapEdge {
            from: pair[0].to_string(),
            to: pair[1].to_string(),
        });
    }
    map
}

#[cfg(test)]
fn first_open_node_id(map: &ActionMapInstance) -> Option<MapNodeId> {
    first_node_with_status(map, NodeStatus::Ready)
}

#[cfg(test)]
fn first_node_with_status(map: &ActionMapInstance, status: NodeStatus) -> Option<MapNodeId> {
    ordered_node_ids(map).into_iter().find_map(|node_id| {
        map.nodes
            .get(&node_id)
            .filter(|node| node.status == status)
            .map(|node| node.id.clone())
    })
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

fn snapshot_task(task: &TaskState) -> ActionMapSnapshotTask {
    ActionMapSnapshotTask {
        id: task.id.clone(),
        title: task.title.clone(),
        objective: task.objective.clone(),
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

fn trace_tags_for(
    action_class: Option<ActionClass>,
    success: bool,
    tool_name: &str,
) -> Vec<String> {
    let mut tags = Vec::new();
    tags.push(if success {
        "tool_success".to_string()
    } else {
        "tool_failure".to_string()
    });
    match action_class {
        Some(ActionClass::Build | ActionClass::Test) if success => {
            tags.push("validator_success".to_string());
        }
        Some(ActionClass::Build | ActionClass::Test) => {
            tags.push("validator_failure".to_string());
        }
        Some(ActionClass::Unknown) | None if looks_like_shell_tool(tool_name) => {
            tags.push("unclassified_shell_action".to_string());
        }
        Some(ActionClass::Unknown) | None => {
            tags.push("unclassified_tool_action".to_string());
        }
        _ => {}
    }
    tags
}

fn sanitize_trace_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .filter(|tag| is_known_trace_tag(tag))
        .collect()
}

fn is_known_trace_tag(tag: &str) -> bool {
    matches!(
        tag,
        "tool_success"
            | "tool_failure"
            | "validator_success"
            | "validator_failure"
            | "unclassified_shell_action"
            | "unclassified_tool_action"
    )
}

fn looks_like_shell_tool(tool_name: &str) -> bool {
    let normalized = tool_name.to_ascii_lowercase();
    normalized.contains("shell") || normalized.contains("command")
}

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let mut nodes = map
        .nodes
        .values()
        .map(|node| ActionMapSnapshotNode {
            id: node.id.clone(),
            title: node.title.clone(),
            kind: node.kind.as_str().to_string(),
            status: node.status.as_str().to_string(),
            context_summary: node.context.summary.clone(),
            source_refs: node.context.source_refs.clone(),
            active_lease: node.active_lease.clone(),
            result_ids: node
                .result_context
                .iter()
                .map(|result| result.id.clone())
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
            body: result.body.clone(),
            source_thread_id: result.source_thread_id,
            created_at_ms: result.created_at_ms,
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| left.id.cmp(&right.id));

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
    let completed = map
        .nodes
        .iter()
        .filter_map(|(id, node)| (node.status == NodeStatus::Completed).then_some(id.clone()))
        .collect::<HashSet<_>>();
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
        if !deps.is_empty()
            && deps
                .iter()
                .all(|dependency_id| completed.contains(dependency_id))
            && let Some(node) = map.nodes.get_mut(&node_id)
        {
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

fn default_dependency_node_ids_for_new_node(map: &ActionMapInstance) -> Vec<String> {
    let nodes_with_outgoing_edges_to_completed = map
        .edges
        .iter()
        .filter(|edge| {
            map.nodes
                .get(&edge.to)
                .is_some_and(|node| node.status == NodeStatus::Completed)
        })
        .map(|edge| edge.from.as_str())
        .collect::<HashSet<_>>();
    let mut leaf_completed_nodes = map
        .nodes
        .values()
        .filter(|node| {
            node.status == NodeStatus::Completed
                && !nodes_with_outgoing_edges_to_completed.contains(node.id.as_str())
        })
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    leaf_completed_nodes.sort();
    leaf_completed_nodes
}

fn completed_subagent_inspect_node_ids(
    map: &ActionMapInstance,
    owner_session_id: ThreadId,
) -> Vec<String> {
    let mut node_ids = map
        .nodes
        .values()
        .filter(|node| {
            node.kind == NodeKind::InspectCodeContext
                && node.status == NodeStatus::Completed
                && node.result_context.iter().any(|result_ref| {
                    map.results
                        .get(&result_ref.id)
                        .is_some_and(|result| result.source_thread_id != owner_session_id)
                })
        })
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    node_ids.sort();
    node_ids
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
        objective: task.objective.clone(),
        owner_session_id: task.owner_session_id,
        active_map_id: task.active_map_id.clone(),
    })
}

fn task_status_changed_event(
    task_id: &str,
    previous_status: TaskStatus,
    current_status: TaskStatus,
) -> MapRuntimeEvent {
    MapRuntimeEvent::TaskStatusChanged(MapRuntimeTaskStatusChangedEvent {
        task_id: task_id.to_string(),
        previous_status: previous_status.as_str().to_string(),
        current_status: current_status.as_str().to_string(),
    })
}

#[cfg(test)]
fn map_status_changed_event(
    map: &ActionMapInstance,
    previous_status: MapStatus,
) -> MapRuntimeEvent {
    MapRuntimeEvent::MapStatusChanged(MapRuntimeMapStatusChangedEvent {
        map_id: map.id.clone(),
        previous_status: previous_status.as_str().to_string(),
        current_status: map.status.as_str().to_string(),
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

fn maintenance_barrier_raised_event(barrier: &ActionMapMaintenanceBarrier) -> MapRuntimeEvent {
    MapRuntimeEvent::MaintenanceBarrierRaised(MapRuntimeMaintenanceBarrierRaisedEvent {
        map_id: barrier.map_id.clone(),
        node_id: barrier.node_id.clone(),
        reason: barrier.reason.as_str().to_string(),
        result_count: barrier.result_count,
        budget: barrier.budget,
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

fn count_node_results_of_kind(node: &MapNode, kind: NodeResultKind) -> usize {
    node.result_context
        .iter()
        .filter(|result| result.kind == kind)
        .count()
}

fn node_has_successful_action(
    map: &ActionMapInstance,
    node: &MapNode,
    action_class: ActionClass,
) -> bool {
    node.result_context.iter().any(|result_ref| {
        let Some(result) = map.results.get(&result_ref.id) else {
            return false;
        };
        result.kind == NodeResultKind::MainToolCall
            && result.action_class == Some(action_class)
            && result.tool_success == Some(true)
    })
}

#[cfg(test)]
fn initial_node_events(map: &ActionMapInstance) -> Vec<MapRuntimeEvent> {
    map.nodes
        .values()
        .filter(|node| node.status == NodeStatus::Ready)
        .map(|node| {
            node_status_changed_event(
                &map.id,
                &node.id,
                &node.title,
                NodeStatus::Pending,
                NodeStatus::Ready,
            )
        })
        .collect()
}

fn assignment_prompt(
    map_id: &str,
    node_id: &str,
    node_title: &str,
    node_kind: NodeKind,
    lease_id: &str,
) -> String {
    let allowed_actions = contract_for(node_kind)
        .allowed_actions
        .iter()
        .map(|action| action.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "TaskSpace node assignment\n\
Map: {map_id}\n\
Node: {node_id} - {node_title}\n\
Node kind: {}\n\
Lease: {lease_id}\n\
Allowed action classes: {allowed_actions}\n\
\n\
        You must work only on this node's subtask. Use the provided node context and return a concise, free-form result for this node. If you are blocked, explain the blocker clearly. Runtime enforces the allowed action classes above. Inspect, test, and final nodes are read-only for repository files; do not edit files or call apply_patch unless this node kind is implement_solution. Implementation nodes allow edits but not validation test runs; after editing, return the result so the parent can create or bind a smoke_test/regression_test node. Do not maintain the map directly. Do not call taskspace_control, spawn_agent, or wait_agent; return findings to the parent agent so it can grow or route the task path.\n\n",
        node_kind.as_str()
    )
}

fn child_tool_reservation_key(child_thread_id: ThreadId, call_id: &str) -> String {
    format!("{child_thread_id}:{call_id}")
}

fn result_from_status(status: &AgentStatus) -> (NodeResultKind, String) {
    match status {
        AgentStatus::Completed(Some(message)) if !message.trim().is_empty() => {
            (NodeResultKind::Result, message.clone())
        }
        AgentStatus::Completed(_) => (
            NodeResultKind::Result,
            "Subagent completed without a final message.".to_string(),
        ),
        AgentStatus::Errored(message) => (NodeResultKind::Blocker, message.clone()),
        AgentStatus::Shutdown => (
            NodeResultKind::Blocker,
            "Subagent was shut down before producing a node result.".to_string(),
        ),
        AgentStatus::NotFound => (
            NodeResultKind::Blocker,
            "Subagent disappeared before producing a node result.".to_string(),
        ),
        AgentStatus::Interrupted | AgentStatus::PendingInit | AgentStatus::Running => (
            NodeResultKind::Blocker,
            format!("Subagent stopped in non-final status: {status:?}"),
        ),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn transition_notice(previous_mode: MapRuntimeMode, current_mode: MapRuntimeMode) -> String {
    match (previous_mode, current_mode) {
        (MapRuntimeMode::Standard, MapRuntimeMode::Experiment) => {
            "TaskSpace mode is now active.\n\
Previous standard-mode conversation remains background context only.\n\
Before taking multi-agent action, create or bind a task path and a ready node.\n\
Future subagent work must be task/node driven."
                .to_string()
        }
        (MapRuntimeMode::Experiment, MapRuntimeMode::Standard) => {
            "TaskSpace mode is now disabled.\n\
Existing task paths, nodes, leases, and results remain historical context only.\n\
Do not continue maintaining the task path, require node binding, or follow task-driven protocol unless the user enables TaskSpace again.\n\
Continue with the standard Codex multi-agent behavior."
                .to_string()
        }
        _ => {
            format!("TaskSpace runtime mode changed from {previous_mode} to {current_mode}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PreflightResultValidity {
        Accepted,
        Questioned,
        Invalid,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PreflightFactProvenance {
        ObservedFromEnvironment,
        ProvidedByUser,
        GeneratedForTestOnly,
        Unknown,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PreflightClearAction {
        FixApplied,
        RiskAcceptedByMainAgent,
        ContractRevised,
    }

    #[derive(Debug, Clone)]
    struct PreflightResultEvidence {
        result_id: &'static str,
        validity: PreflightResultValidity,
        has_claims: bool,
        has_evidence_refs: bool,
    }

    #[derive(Debug, Clone)]
    struct PreflightFactSource {
        id: &'static str,
        provenance: PreflightFactProvenance,
        enters_active_fact: bool,
    }

    #[derive(Debug, Clone)]
    struct PreflightFinalArtifactDependency {
        artifact_id: &'static str,
        result_id: &'static str,
    }

    #[derive(Debug, Clone)]
    struct PreflightSentinelRecord {
        sentinel_id: &'static str,
        affects_final_artifact: bool,
        clear_action: Option<PreflightClearAction>,
    }

    #[derive(Debug, Clone)]
    struct PreflightCognitiveAuditRecord {
        promotion_not_in_mvp: bool,
        output_contract_present: bool,
        result_evidence: Vec<PreflightResultEvidence>,
        fact_sources: Vec<PreflightFactSource>,
        cognitive_state_result_refs: Vec<&'static str>,
        final_artifact_dependencies: Vec<PreflightFinalArtifactDependency>,
        sentinel_records: Vec<PreflightSentinelRecord>,
    }

    impl PreflightCognitiveAuditRecord {
        fn validate_mvp_hard_gates(&self) -> Vec<&'static str> {
            let mut failures = Vec::new();

            if !self.promotion_not_in_mvp {
                failures.push("promotion_not_in_mvp_missing");
            }

            if !self.output_contract_present {
                failures.push("required_output_contract_missing");
            }

            for result in &self.result_evidence {
                if result.validity == PreflightResultValidity::Accepted
                    && (!result.has_claims || !result.has_evidence_refs)
                {
                    failures.push("accepted_result_missing_evidence");
                }
            }

            for fact_source in &self.fact_sources {
                assert!(
                    !fact_source.id.trim().is_empty(),
                    "preflight fact source id should be a join key"
                );
                if fact_source.enters_active_fact
                    && matches!(
                        fact_source.provenance,
                        PreflightFactProvenance::GeneratedForTestOnly
                            | PreflightFactProvenance::Unknown
                    )
                {
                    failures.push("generated_or_unknown_provenance_in_active_fact");
                }
            }

            for result_id in &self.cognitive_state_result_refs {
                if self.result_has_invalid_or_questioned_validity(result_id) {
                    failures.push("questioned_or_invalid_result_in_cognitive_state_update");
                }
            }

            for dependency in &self.final_artifact_dependencies {
                assert!(
                    !dependency.artifact_id.trim().is_empty(),
                    "preflight final artifact id should be a join key"
                );
                if self.result_has_invalid_or_questioned_validity(dependency.result_id) {
                    failures.push("questioned_or_invalid_final_artifact_dependency");
                }
            }

            for sentinel in &self.sentinel_records {
                assert!(
                    !sentinel.sentinel_id.trim().is_empty(),
                    "preflight sentinel id should be a join key"
                );
                if let Some(clear_action) = sentinel.clear_action {
                    match clear_action {
                        PreflightClearAction::FixApplied
                        | PreflightClearAction::RiskAcceptedByMainAgent
                        | PreflightClearAction::ContractRevised => {}
                    }
                }
                if sentinel.affects_final_artifact && sentinel.clear_action.is_none() {
                    failures.push("sentinel_warning_uncleared_for_final_artifact");
                }
            }

            failures
        }

        fn result_has_invalid_or_questioned_validity(&self, result_id: &str) -> bool {
            self.result_evidence
                .iter()
                .find(|result| result.result_id == result_id)
                .map(|result| {
                    matches!(
                        result.validity,
                        PreflightResultValidity::Questioned | PreflightResultValidity::Invalid
                    )
                })
                .unwrap_or(false)
        }
    }

    fn seed_test_map(state: &mut ActionMapRuntimeState, owner: ThreadId) {
        state.ensure_active_seed_map(owner, "test");
        state.mark_routing_complete();
    }

    fn start_test_task(
        state: &mut ActionMapRuntimeState,
        owner: ThreadId,
        title: &str,
        context: &str,
        bind_current: bool,
    ) -> (TaskId, ActionMapId, MapNodeId, Vec<MapRuntimeEvent>) {
        state
            .start_task_for_main(
                owner,
                title.to_string(),
                context.to_string(),
                title.to_string(),
                context.to_string(),
                bind_current,
            )
            .expect("test task starts")
    }

    fn fill_main_tool_budget(
        state: &mut ActionMapRuntimeState,
        owner: ThreadId,
    ) -> Vec<MapRuntimeEvent> {
        let mut last_events = Vec::new();
        for index in 0..MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed while budget remains");
            let (_, events) = state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds")
                .expect("result recorded");
            last_events = events;
        }
        last_events
    }

    #[test]
    fn defaults_to_standard() {
        let state = ActionMapRuntimeState::default();

        assert_eq!(state.mode(), MapRuntimeMode::Standard);
    }

    #[test]
    fn set_mode_reports_whether_state_changed() {
        let mut state = ActionMapRuntimeState::default();

        let changed = state.set_mode(MapRuntimeMode::Experiment);
        assert_eq!(changed.previous_mode, MapRuntimeMode::Standard);
        assert_eq!(changed.current_mode, MapRuntimeMode::Experiment);
        assert!(changed.changed);
        assert!(
            state
                .take_pending_transition_notice()
                .expect("transition notice")
                .contains("TaskSpace mode is now active")
        );

        let unchanged = state.set_mode(MapRuntimeMode::Experiment);
        assert_eq!(unchanged.previous_mode, MapRuntimeMode::Experiment);
        assert_eq!(unchanged.current_mode, MapRuntimeMode::Experiment);
        assert!(!unchanged.changed);
        assert!(state.take_pending_transition_notice().is_none());
    }

    #[test]
    fn set_experiment_mode_for_session_does_not_create_task_path() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();

        let (outcome, events) = state.set_mode_for_session(MapRuntimeMode::Experiment, owner);

        assert!(outcome.mode.changed);
        assert!(outcome.active_map_id.is_none());
        assert!(state.active_map().is_none());
        assert!(events.is_empty());
        assert!(state.current_main_node_id.is_none());
    }

    #[test]
    fn request_reborn_sets_gate_without_creating_task_path() {
        let mut state = ActionMapRuntimeState::default();

        let events = state.request_reborn();
        let snapshot = state.snapshot();

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::ModeChanged(event)
                    if event.current_mode == MapRuntimeMode::Experiment
            )
        }));
        assert!(snapshot.routing_required);
        assert!(snapshot.bootstrap_required);
        assert!(snapshot.reborn_requested);
        assert!(snapshot.tasks.is_empty());
        assert!(snapshot.maps.is_empty());
        let context = state.build_developer_context().expect("developer context");
        assert!(context.contains("The user requested /task-reborn"));
    }

    #[test]
    fn main_tool_call_rejects_work_before_agent_created_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary work requires task node first");

        assert!(error.contains("taskspace_control(action=start_task"));
        assert!(state.maps.is_empty());
    }

    #[test]
    fn main_tool_call_uses_agent_created_current_node_binding() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let (_, _, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Inspect logging",
            "Understand logging before changing code.",
            true,
        );
        assert_eq!(node_id, "node-1");

        let events = state
            .prepare_main_tool_call(owner, "shell")
            .expect("main tool binding");
        assert!(events.is_empty());
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));

        let (result_id, result_events) = state
            .record_main_tool_result(owner, "call-1", "shell", true, "ok".to_string())
            .expect("record succeeds")
            .expect("result recorded");

        assert_eq!(result_id, "result-1");
        assert!(result_events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeResultRecorded(event)
                    if event.node_id == "node-1"
                        && event.lease_id == "lease-1"
                        && event.kind == "main_tool_call"
                        && event.source_thread_id == owner
            )
        }));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(node.result_context.len(), 1);
        let result = map.results.get("result-1").expect("stored result");
        assert_eq!(result.kind, NodeResultKind::MainToolCall);
        assert_eq!(result.assignment_id, "lease-1");
        assert!(result.body.contains("tool: shell"));
        assert!(result.body.contains("preview:\nok"));
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_accepts_clean_mvp_record() {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: true,
            output_contract_present: true,
            result_evidence: vec![PreflightResultEvidence {
                result_id: "result-accepted",
                validity: PreflightResultValidity::Accepted,
                has_claims: true,
                has_evidence_refs: true,
            }],
            fact_sources: vec![
                PreflightFactSource {
                    id: "source-env",
                    provenance: PreflightFactProvenance::ObservedFromEnvironment,
                    enters_active_fact: true,
                },
                PreflightFactSource {
                    id: "source-user",
                    provenance: PreflightFactProvenance::ProvidedByUser,
                    enters_active_fact: true,
                },
            ],
            cognitive_state_result_refs: vec!["result-accepted"],
            final_artifact_dependencies: vec![PreflightFinalArtifactDependency {
                artifact_id: "artifact-report",
                result_id: "result-accepted",
            }],
            sentinel_records: vec![
                PreflightSentinelRecord {
                    sentinel_id: "sentinel-output-contract",
                    affects_final_artifact: true,
                    clear_action: Some(PreflightClearAction::ContractRevised),
                },
                PreflightSentinelRecord {
                    sentinel_id: "sentinel-fix-applied",
                    affects_final_artifact: true,
                    clear_action: Some(PreflightClearAction::FixApplied),
                },
                PreflightSentinelRecord {
                    sentinel_id: "sentinel-risk-accepted",
                    affects_final_artifact: true,
                    clear_action: Some(PreflightClearAction::RiskAcceptedByMainAgent),
                },
            ],
        };

        assert_eq!(record.validate_mvp_hard_gates(), Vec::<&'static str>::new());
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_missing_promotion_scope_marker() {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: false,
            output_contract_present: true,
            result_evidence: Vec::new(),
            fact_sources: Vec::new(),
            cognitive_state_result_refs: Vec::new(),
            final_artifact_dependencies: Vec::new(),
            sentinel_records: Vec::new(),
        };

        assert_eq!(
            record.validate_mvp_hard_gates(),
            vec!["promotion_not_in_mvp_missing"]
        );
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_missing_output_contract() {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: true,
            output_contract_present: false,
            result_evidence: Vec::new(),
            fact_sources: Vec::new(),
            cognitive_state_result_refs: Vec::new(),
            final_artifact_dependencies: Vec::new(),
            sentinel_records: Vec::new(),
        };

        assert_eq!(
            record.validate_mvp_hard_gates(),
            vec!["required_output_contract_missing"]
        );
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_accepted_result_without_claims_or_evidence()
     {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: true,
            output_contract_present: true,
            result_evidence: vec![PreflightResultEvidence {
                result_id: "result-accepted",
                validity: PreflightResultValidity::Accepted,
                has_claims: true,
                has_evidence_refs: false,
            }],
            fact_sources: Vec::new(),
            cognitive_state_result_refs: Vec::new(),
            final_artifact_dependencies: Vec::new(),
            sentinel_records: Vec::new(),
        };

        assert_eq!(
            record.validate_mvp_hard_gates(),
            vec!["accepted_result_missing_evidence"]
        );
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_generated_or_unknown_active_facts() {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: true,
            output_contract_present: true,
            result_evidence: Vec::new(),
            fact_sources: vec![
                PreflightFactSource {
                    id: "generated-fixture",
                    provenance: PreflightFactProvenance::GeneratedForTestOnly,
                    enters_active_fact: true,
                },
                PreflightFactSource {
                    id: "unknown-input",
                    provenance: PreflightFactProvenance::Unknown,
                    enters_active_fact: true,
                },
            ],
            cognitive_state_result_refs: Vec::new(),
            final_artifact_dependencies: Vec::new(),
            sentinel_records: Vec::new(),
        };

        assert_eq!(
            record.validate_mvp_hard_gates(),
            vec![
                "generated_or_unknown_provenance_in_active_fact",
                "generated_or_unknown_provenance_in_active_fact"
            ]
        );
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_questioned_or_invalid_result_dependencies()
    {
        for (validity, expected_gate) in [
            (
                PreflightResultValidity::Questioned,
                "questioned_or_invalid_result_in_cognitive_state_update",
            ),
            (
                PreflightResultValidity::Invalid,
                "questioned_or_invalid_result_in_cognitive_state_update",
            ),
        ] {
            let record = PreflightCognitiveAuditRecord {
                promotion_not_in_mvp: true,
                output_contract_present: true,
                result_evidence: vec![PreflightResultEvidence {
                    result_id: "result-risky",
                    validity,
                    has_claims: true,
                    has_evidence_refs: true,
                }],
                fact_sources: Vec::new(),
                cognitive_state_result_refs: vec!["result-risky"],
                final_artifact_dependencies: Vec::new(),
                sentinel_records: Vec::new(),
            };

            assert_eq!(record.validate_mvp_hard_gates(), vec![expected_gate]);
        }

        for validity in [
            PreflightResultValidity::Questioned,
            PreflightResultValidity::Invalid,
        ] {
            let record = PreflightCognitiveAuditRecord {
                promotion_not_in_mvp: true,
                output_contract_present: true,
                result_evidence: vec![PreflightResultEvidence {
                    result_id: "result-risky",
                    validity,
                    has_claims: true,
                    has_evidence_refs: true,
                }],
                fact_sources: Vec::new(),
                cognitive_state_result_refs: Vec::new(),
                final_artifact_dependencies: vec![PreflightFinalArtifactDependency {
                    artifact_id: "artifact-report",
                    result_id: "result-risky",
                }],
                sentinel_records: Vec::new(),
            };

            assert_eq!(
                record.validate_mvp_hard_gates(),
                vec!["questioned_or_invalid_final_artifact_dependency"]
            );
        }
    }

    #[test]
    fn cognitive_preflight_contract_sketch_audit_rejects_uncleared_sentinel_on_final_artifact() {
        let record = PreflightCognitiveAuditRecord {
            promotion_not_in_mvp: true,
            output_contract_present: true,
            result_evidence: Vec::new(),
            fact_sources: Vec::new(),
            cognitive_state_result_refs: Vec::new(),
            final_artifact_dependencies: Vec::new(),
            sentinel_records: vec![PreflightSentinelRecord {
                sentinel_id: "sentinel-provenance",
                affects_final_artifact: true,
                clear_action: None,
            }],
        };

        assert_eq!(
            record.validate_mvp_hard_gates(),
            vec!["sentinel_warning_uncleared_for_final_artifact"]
        );
    }

    #[test]
    fn cognitive_preflight_runtime_snapshot_results_have_join_keys_for_audit() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_task_id, map_id, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Inspect artifact contract",
            "Confirm output contract and evidence surface.",
            true,
        );

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Test, "pytest"),
            )
            .expect("diagnostic test is allowed in inspect node");
        state
            .record_main_tool_result_with_class(
                owner,
                "call-test",
                "shell_command",
                Some(ActionClass::Test),
                true,
                "pytest passed".to_string(),
            )
            .expect("structured test result records");

        let snapshot = state.snapshot();
        let map = snapshot
            .maps
            .iter()
            .find(|map| map.id == map_id)
            .expect("snapshot map");
        let result = map
            .results
            .iter()
            .find(|result| result.id == "result-1")
            .expect("snapshot result");

        assert_eq!(result.map_id, map_id);
        assert_eq!(result.node_id, node_id);
        assert_eq!(result.kind, "main_tool_call");
        assert_eq!(result.action_class.as_deref(), Some("test"));
        assert_eq!(result.tool_success, Some(true));
        assert_eq!(result.source_thread_id, owner);
    }

    #[test]
    fn standard_mode_does_not_record_taskspace_trace() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();

        let outcome = state
            .record_main_tool_result_with_class(
                owner,
                "call-ignored",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed".to_string(),
            )
            .expect("standard mode ignores taskspace tool result recording");

        let snapshot = state.snapshot();
        assert!(outcome.is_none());
        assert_eq!(snapshot.trace_summary.total_event_count, 0);
        assert_eq!(snapshot.trace_summary.tool_call_count, 0);
        assert!(snapshot.trace_events.is_empty());
    }

    #[test]
    fn experiment_main_tool_result_records_structured_trace_event() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (task_id, map_id, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Run diagnostics",
            "Collect failing test evidence.",
            true,
        );

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Test, "pytest")
                    .with_call_id("call-test"),
            )
            .expect("diagnostic test is allowed");
        let (result_id, result_events) = state
            .record_main_tool_result_with_class(
                owner,
                "call-test",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed".to_string(),
            )
            .expect("structured test result records")
            .expect("result is recorded");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.trace_summary.total_event_count, 1);
        assert_eq!(snapshot.trace_summary.tool_call_count, 1);
        assert_eq!(snapshot.trace_summary.failed_tool_call_count, 1);
        assert_eq!(snapshot.trace_summary.validator_failure_count, 1);
        assert!(matches!(
            &result_events[0],
            MapRuntimeEvent::NodeResultRecorded(event)
                if event.result_id == result_id && event.action_class.as_deref() == Some("test")
        ));
        assert!(matches!(
            &result_events[1],
            MapRuntimeEvent::TaskspaceTraceEventRecorded(event)
                if event.trace_event_id == "trace-1"
                    && event.result_id.as_deref() == Some(result_id.as_str())
                    && event.action_class.as_deref() == Some("test")
                    && event.tool_success == Some(false)
                    && !event.tags.iter().any(|tag| tag.contains("pytest"))
        ));
        let trace = snapshot
            .trace_events
            .first()
            .expect("trace event is exposed");
        assert_eq!(trace.id, "trace-1");
        assert_eq!(trace.kind, "main_tool_result");
        assert_eq!(trace.task_id.as_deref(), Some(task_id.as_str()));
        assert_eq!(trace.map_id, map_id);
        assert_eq!(trace.node_id, node_id);
        assert_eq!(trace.result_id.as_deref(), Some(result_id.as_str()));
        assert_eq!(trace.call_id.as_deref(), Some("call-test"));
        assert_eq!(trace.action_class.as_deref(), Some("test"));
        assert_eq!(trace.tool_success, Some(false));
        assert!(trace.tags.iter().any(|tag| tag == "tool_failure"));
        assert!(trace.tags.iter().any(|tag| tag == "validator_failure"));
        assert!(trace.artifact_refs.is_empty());
    }

    #[test]
    fn experiment_trace_records_read_edit_and_test_actions() {
        let cases = [
            (
                NodeKind::InspectCodeContext,
                "shell_command",
                ActionClass::Read,
                true,
                vec!["tool_success"],
                vec!["validator_success", "validator_failure"],
            ),
            (
                NodeKind::ImplementSolution,
                "apply_patch",
                ActionClass::Edit,
                true,
                vec!["tool_success"],
                vec!["validator_success", "validator_failure"],
            ),
            (
                NodeKind::SmokeTest,
                "shell_command",
                ActionClass::Test,
                false,
                vec!["tool_failure", "validator_failure"],
                vec!["validator_success"],
            ),
        ];

        for (index, (node_kind, tool_name, action_class, success, expected_tags, forbidden_tags)) in
            cases.into_iter().enumerate()
        {
            let mut state = ActionMapRuntimeState::default();
            let owner = ThreadId::new();
            state.set_mode(MapRuntimeMode::Experiment);
            state
                .start_task_for_main_with_kind(
                    owner,
                    node_kind,
                    format!("Trace action {index}"),
                    "Record action trace.".to_string(),
                    format!("Node {index}"),
                    "Current action node.".to_string(),
                    true,
                )
                .expect("task starts");

            state
                .prepare_main_tool_call(
                    owner,
                    ToolActionDescriptor::new(tool_name, action_class, action_class.as_str())
                        .with_call_id(format!("call-{index}")),
                )
                .expect("tool is allowed by node contract");
            let (result_id, events) = state
                .record_main_tool_result_with_class(
                    owner,
                    &format!("call-{index}"),
                    tool_name,
                    Some(action_class),
                    success,
                    format!("{} preview", action_class.as_str()),
                )
                .expect("tool result records")
                .expect("result is recorded");

            let trace_event = events
                .iter()
                .find_map(|event| match event {
                    MapRuntimeEvent::TaskspaceTraceEventRecorded(event) => Some(event),
                    _ => None,
                })
                .expect("trace event emitted");
            assert_eq!(trace_event.result_id.as_deref(), Some(result_id.as_str()));
            assert_eq!(
                trace_event.action_class.as_deref(),
                Some(action_class.as_str())
            );
            assert_eq!(trace_event.tool_success, Some(success));
            for tag in expected_tags {
                assert!(
                    trace_event.tags.iter().any(|actual| actual == tag),
                    "missing expected tag {tag} for {}",
                    action_class.as_str()
                );
            }
            for tag in forbidden_tags {
                assert!(
                    !trace_event.tags.iter().any(|actual| actual == tag),
                    "unexpected tag {tag} for {}",
                    action_class.as_str()
                );
            }

            let snapshot = state.snapshot();
            assert_eq!(snapshot.trace_events.len(), 1);
            assert_eq!(
                snapshot.trace_events[0].action_class.as_deref(),
                Some(action_class.as_str())
            );
        }
    }

    #[test]
    fn trace_event_does_not_parse_shell_preview_as_structured_semantics() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect shell output",
            "Record unclassified command output.",
            true,
        );

        state
            .record_main_tool_result_with_class(
                owner,
                "call-shell",
                "shell_command",
                None,
                true,
                "output_contract_present=true\nartifact_refs=[\"fake-report\"]".to_string(),
            )
            .expect("unreserved shell result records")
            .expect("result is recorded");

        let snapshot = state.snapshot();
        let trace = snapshot
            .trace_events
            .first()
            .expect("trace event is exposed");
        assert_eq!(snapshot.trace_summary.unclassified_shell_action_count, 1);
        assert!(trace.tags.iter().any(|tag| tag == "tool_success"));
        assert!(
            trace
                .tags
                .iter()
                .any(|tag| tag == "unclassified_shell_action")
        );
        assert!(trace.artifact_refs.is_empty());
        let value = serde_json::to_value(trace).expect("trace serializes");
        assert!(value.get("preview").is_none());
        assert!(value.get("body").is_none());
        assert!(!trace.tags.iter().any(|tag| tag.contains("output_contract")));
        assert!(!trace.tags.iter().any(|tag| tag.contains("fake-report")));
        let formatted = format_action_map_snapshot(&snapshot);
        assert!(formatted.contains("trace events: total=1"));
    }

    #[test]
    fn missing_action_class_for_non_shell_tool_is_not_counted_as_unclassified_shell() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Record control result",
            "Check unknown non-shell trace tags.",
            true,
        );

        state
            .record_main_tool_result_with_class(
                owner,
                "call-control",
                "taskspace_control",
                None,
                true,
                "TaskSpace node created".to_string(),
            )
            .expect("unreserved control result records")
            .expect("result is recorded");

        let snapshot = state.snapshot();
        let trace = snapshot
            .trace_events
            .first()
            .expect("trace event is exposed");
        assert_eq!(snapshot.trace_summary.unclassified_shell_action_count, 0);
        assert!(
            trace
                .tags
                .iter()
                .any(|tag| tag == "unclassified_tool_action")
        );
        assert!(
            !trace
                .tags
                .iter()
                .any(|tag| tag == "unclassified_shell_action")
        );
    }

    #[test]
    fn switching_to_standard_preserves_existing_trace_without_recording_new_trace() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Mode transition trace",
            "Record trace before switching modes.",
            true,
        );
        state
            .record_main_tool_result_with_class(
                owner,
                "call-read",
                "shell_command",
                Some(ActionClass::Read),
                true,
                "read files".to_string(),
            )
            .expect("experiment result records");

        state.set_mode(MapRuntimeMode::Standard);
        state
            .record_main_tool_result_with_class(
                owner,
                "call-standard",
                "shell_command",
                Some(ActionClass::Read),
                true,
                "standard mode ignored".to_string(),
            )
            .expect("standard mode does not error");

        let snapshot = state.snapshot();
        assert_eq!(snapshot.mode, MapRuntimeMode::Standard);
        assert_eq!(snapshot.trace_summary.total_event_count, 1);
        assert_eq!(snapshot.trace_events[0].id, "trace-1");
        assert_eq!(
            snapshot.trace_events[0].call_id.as_deref(),
            Some("call-read")
        );
    }

    #[test]
    fn trace_event_is_emitted_before_barrier_event_when_budget_is_exhausted() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Barrier order",
            "Fill the main tool budget.",
            true,
        );

        let events = fill_main_tool_budget(&mut state, owner);

        assert!(matches!(
            &events[0],
            MapRuntimeEvent::NodeResultRecorded(event)
                if event.result_id == format!("result-{MAIN_TOOL_RESULT_BUDGET_PER_NODE}")
        ));
        assert!(matches!(
            &events[1],
            MapRuntimeEvent::TaskspaceTraceEventRecorded(event)
                if event.trace_event_id == format!("trace-{MAIN_TOOL_RESULT_BUDGET_PER_NODE}")
        ));
        assert!(matches!(
            &events[2],
            MapRuntimeEvent::MaintenanceBarrierRaised(event)
                if event.result_count == MAIN_TOOL_RESULT_BUDGET_PER_NODE
        ));
    }

    #[test]
    fn restore_snapshot_preserves_trace_and_next_trace_sequence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Restore trace",
            "Check trace sequence after restore.",
            true,
        );
        state
            .record_main_tool_result_with_class(
                owner,
                "call-read",
                "shell_command",
                Some(ActionClass::Read),
                true,
                "read files".to_string(),
            )
            .expect("first result records");

        let snapshot = state.snapshot();
        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot);
        restored
            .record_main_tool_result_with_class(
                owner,
                "call-test",
                "shell_command",
                Some(ActionClass::Test),
                true,
                "pytest passed".to_string(),
            )
            .expect("second result records after restore");

        let restored_snapshot = restored.snapshot();
        let trace_ids = restored_snapshot
            .trace_events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(trace_ids, vec!["trace-1", "trace-2"]);
        assert_eq!(restored_snapshot.trace_summary.total_event_count, 2);
        assert_eq!(restored_snapshot.trace_summary.validator_failure_count, 0);
    }

    #[test]
    fn restore_snapshot_sanitizes_unknown_trace_tags_before_summary_counts() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Sanitize trace tags",
            "Record trace before restore.",
            true,
        );
        state
            .record_main_tool_result_with_class(
                owner,
                "call-test",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed".to_string(),
            )
            .expect("test result records");
        let mut snapshot = state.snapshot();
        let trace = snapshot
            .trace_events
            .first_mut()
            .expect("trace event exists");
        trace.tags.push("forged_future_semantic_tag".to_string());
        trace.tags.push("output_contract_present".to_string());

        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot);
        let restored_snapshot = restored.snapshot();
        let restored_trace = restored_snapshot
            .trace_events
            .first()
            .expect("restored trace event");

        assert!(restored_trace.tags.iter().any(|tag| tag == "tool_failure"));
        assert!(
            restored_trace
                .tags
                .iter()
                .any(|tag| tag == "validator_failure")
        );
        assert!(
            !restored_trace
                .tags
                .iter()
                .any(|tag| tag == "forged_future_semantic_tag")
        );
        assert!(
            !restored_trace
                .tags
                .iter()
                .any(|tag| tag == "output_contract_present")
        );
        assert_eq!(restored_snapshot.trace_summary.validator_failure_count, 1);
    }

    #[test]
    fn cognitive_preflight_runtime_developer_context_keeps_promotion_and_collapse_out_of_mvp() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Review taskspace MVP",
            "Check the preflight developer context.",
            true,
        );

        let context = state.build_developer_context().expect("developer context");

        assert!(context.contains("TaskSpace mode is active"));
        assert!(!context.contains("promote_taskspace"));
        assert!(!context.contains("promotion_not_in_mvp"));
        assert!(!context.contains("collapsed-direct"));
    }

    #[test]
    fn node_contract_blocks_edit_inside_inspect_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect before edit",
            "Read context before editing.",
            true,
        );

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Test, "pytest"),
            )
            .expect("inspect nodes allow diagnostic tests as evidence");
        let error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "patch"),
            )
            .expect_err("inspect nodes cannot edit");
        let (message, events) = error.into_parts();

        assert!(message.contains("inspect_code_context"));
        assert!(message.contains("edit"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::ToolActionBlocked(blocked)
                    if blocked.node_id == "node-1"
                        && blocked.node_kind == "inspect_code_context"
                        && blocked.action_class == "edit"
            )
        }));
    }

    #[test]
    fn implement_node_allows_edit_but_blocks_test() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Implement fix".to_string(),
                "Apply the known fix.".to_string(),
                "Patch code".to_string(),
                "Modify the target files.".to_string(),
                true,
            )
            .expect("task starts");

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "patch"),
            )
            .expect("implementation nodes allow edit");
        let error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Test, "pytest"),
            )
            .expect_err("implementation nodes do not absorb tests");

        assert!(error.contains("implement_solution"));
        assert!(error.contains("test"));
    }

    #[test]
    fn validation_node_allows_test_but_blocks_edit() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::RegressionTest,
                "Run validation".to_string(),
                "Validate the current implementation.".to_string(),
                "Regression suite".to_string(),
                "Run the regression suite without modifying files.".to_string(),
                true,
            )
            .expect("task starts");

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Build, "cargo build"),
            )
            .expect("validation nodes allow build/test/lint commands");
        let error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "patch"),
            )
            .expect_err("validation nodes do not allow edits");

        assert!(error.contains("regression_test"));
        assert!(error.contains("edit"));
    }

    #[test]
    fn final_synthesis_node_blocks_edit_test_and_spawn() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::FinalSynthesis,
                "Summarize outcome".to_string(),
                "Write the final user-facing synthesis.".to_string(),
                "Final synthesis".to_string(),
                "Summarize completed work without new implementation.".to_string(),
                true,
            )
            .expect("task starts");

        let edit_error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "patch"),
            )
            .expect_err("final synthesis nodes cannot edit");
        assert!(edit_error.contains("final_synthesis"));
        assert!(edit_error.contains("edit"));

        let test_error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell_command", ActionClass::Test, "pytest"),
            )
            .expect_err("final synthesis nodes cannot test");
        assert!(test_error.contains("final_synthesis"));
        assert!(test_error.contains("test"));

        let spawn_error = state
            .prepare_spawn_assignment(owner, "new parallel work", None)
            .expect_err("final synthesis nodes cannot spawn");
        assert!(spawn_error.contains("final_synthesis"));
        assert!(spawn_error.contains("spawn"));
    }

    #[test]
    fn final_response_completes_running_final_synthesis_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::FinalSynthesis,
                "Summarize outcome".to_string(),
                "Write the final user-facing synthesis.".to_string(),
                "Final synthesis".to_string(),
                "Summarize completed work without new implementation.".to_string(),
                true,
            )
            .expect("task starts");

        let (result_id, events) = state
            .record_main_final_response(owner, "Completed the requested review.")
            .expect("final response records")
            .expect("final node should record a result");

        assert_eq!(result_id, "result-1");
        assert!(state.current_main_node_id.is_none());
        assert!(state.current_main_lease_id.is_none());
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(changed)
                    if changed.node_id == "node-1" && changed.current_status == "completed"
            )
        }));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("final node");
        assert_eq!(node.status, NodeStatus::Completed);
        assert_eq!(node.active_lease, None);
        assert_eq!(node.result_context.len(), 1);
        let result = map.results.get(&result_id).expect("stored result");
        assert_eq!(result.kind, NodeResultKind::Result);
        assert_eq!(result.body, "Completed the requested review.");
    }

    #[test]
    fn final_response_does_not_complete_non_final_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect first",
            "Find target files.",
            true,
        );

        let outcome = state
            .record_main_final_response(owner, "This is a normal assistant message.")
            .expect("non-final node should not error");

        assert!(outcome.is_none());
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("inspect node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.result_context.len(), 0);
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
    }

    #[test]
    fn finish_node_can_create_and_bind_next_node_draft_atomically() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect first",
            "Find the correct files.",
            true,
        );

        let (outcome, events) = state
            .finish_main_node_with_next(
                owner,
                "node-1",
                "Inspected files and found target module.".to_string(),
                None,
                Some(ActionMapNextNodeDraft {
                    kind: NodeKind::ImplementSolution,
                    title: "Patch target module".to_string(),
                    context_summary: "Use the inspected evidence from node-1.".to_string(),
                    dependency_node_ids: Vec::new(),
                }),
            )
            .expect("finish creates and binds next node");

        assert_eq!(outcome.result_id, "result-1");
        assert_eq!(outcome.next_node_id.as_deref(), Some("node-2"));
        let map = state.active_map().expect("active map");
        let first = map.nodes.get("node-1").expect("first node");
        let second = map.nodes.get("node-2").expect("second node");
        assert_eq!(first.status, NodeStatus::Completed);
        assert_eq!(second.kind, NodeKind::ImplementSolution);
        assert_eq!(second.status, NodeStatus::Running);
        assert_eq!(map.edges.len(), 1);
        assert_eq!(map.edges[0].from, "node-1");
        assert_eq!(map.edges[0].to, "node-2");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseCreated(created) if created.node_id == "node-2"
            )
        }));
    }

    #[test]
    fn start_task_rejects_custom_live_node_without_mutating_state() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let error = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::Custom,
                "Architecture audit".to_string(),
                "Audit the project architecture.".to_string(),
                "Generic work".to_string(),
                "This should choose a concrete runtime kind.".to_string(),
                true,
            )
            .expect_err("custom is not valid for live task creation");

        assert!(error.contains("concrete node_kind"));
        assert!(state.tasks.is_empty());
        assert!(state.maps.is_empty());
        assert!(state.active_task_id.is_none());
        assert!(state.active_map_id.is_none());
    }

    #[test]
    fn create_node_rejects_custom_live_node_without_mutating_map() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect first",
            "Find the correct files.",
            false,
        );

        let error = state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::Custom,
                "Generic follow-up".to_string(),
                "This should choose a concrete runtime kind.".to_string(),
                Vec::new(),
                false,
            )
            .expect_err("custom is not valid for live node creation");

        assert!(error.contains("concrete node_kind"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes.len(), 1);
        assert!(!map.nodes.contains_key("node-2"));
    }

    #[test]
    fn finish_node_rejects_custom_next_draft_without_finishing_current_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect first",
            "Find the correct files.",
            true,
        );

        let error = state
            .finish_main_node_with_next(
                owner,
                "node-1",
                "Inspected files and found target module.".to_string(),
                None,
                Some(ActionMapNextNodeDraft {
                    kind: NodeKind::Custom,
                    title: "Generic follow-up".to_string(),
                    context_summary: "This should choose a concrete runtime kind.".to_string(),
                    dependency_node_ids: vec!["node-1".to_string()],
                }),
            )
            .expect_err("custom next draft is rejected before lifecycle mutation");

        assert!(error.contains("concrete node_kind"));
        let map = state.active_map().expect("active map");
        let first = map.nodes.get("node-1").expect("first node");
        assert_eq!(first.status, NodeStatus::Running);
        assert!(first.result_context.is_empty());
        assert!(!map.nodes.contains_key("node-2"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
    }

    #[test]
    fn bind_main_node_rejects_switching_while_current_main_lease_is_running() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect first",
            "Current node still running.",
            true,
        );
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Patch target".to_string(),
                "Ready follow-up node.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create follow-up");

        let error = state
            .bind_main_node(owner, "node-2")
            .expect_err("must finish or block current node first");

        assert!(error.contains("node-1"));
        assert!(error.contains("finish_node"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
    }

    #[test]
    fn bind_main_node_rejects_sequential_claim_when_multiple_inspect_nodes_are_ready() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "Overview", "Read project shape.", true);
        state
            .finish_main_node(owner, "node-1", "overview done".to_string(), None)
            .expect("finish overview");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parser investigation".to_string(),
                "Investigate parser behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create parser node");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Pricing investigation".to_string(),
                "Investigate pricing behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create pricing node");

        let error = state
            .bind_main_node(owner, "node-2")
            .expect_err("parallel inspect nodes should be delegated");

        assert!(error.contains("multiple ready inspect nodes"));
        assert!(error.contains("spawn_agent"));
        assert!(state.current_main_node_id.is_none());
    }

    #[test]
    fn implementation_node_directly_depends_on_completed_subagent_inspect_nodes() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "Overview", "Read project shape.", true);
        state
            .finish_main_node(owner, "node-1", "overview done".to_string(), None)
            .expect("finish overview");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parser investigation".to_string(),
                "Investigate parser behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create parser node");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Pricing investigation".to_string(),
                "Investigate pricing behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create pricing node");

        for (node_id, child) in [("node-2", ThreadId::new()), ("node-3", ThreadId::new())] {
            let assignment = state
                .prepare_spawn_assignment(owner, node_id, Some(node_id))
                .expect("claim ready inspect node")
                .0
                .expect("assignment");
            state.attach_agent_to_lease(
                &assignment.lease_id,
                child,
                Some(format!("/root/{node_id}")),
            );
            state.record_child_result(
                child,
                &AgentStatus::Completed(Some(format!("{node_id} done"))),
            );
        }

        let (implementation_node_id, _) = state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Implement combined fix".to_string(),
                "Integrate parser and pricing findings.".to_string(),
                Vec::new(),
                true,
            )
            .expect("create implementation node");

        assert_eq!(implementation_node_id, "node-4");
        let map = state.active_map().expect("active map");
        assert!(
            map.edges
                .iter()
                .any(|edge| edge.from == "node-2" && edge.to == "node-4")
        );
        assert!(
            map.edges
                .iter()
                .any(|edge| edge.from == "node-3" && edge.to == "node-4")
        );
    }

    #[test]
    fn spawn_agent_rejects_serial_single_inspect_after_completed_narrow_inspect() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect scope",
            "Read the initial project context.",
            true,
        );
        state
            .record_main_tool_result(owner, "read-1", "shell", true, "read ok".to_string())
            .expect("record main result")
            .expect("result recorded");
        state
            .finish_main_node(owner, "node-1", "scope done".to_string(), None)
            .expect("finish narrow inspect");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Read one known test file".to_string(),
                "Read one known file and report findings.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create serial follow-up inspect");

        let error = state
            .prepare_spawn_assignment(owner, "read known file", Some("node-2"))
            .expect_err("single inspect track should stay on the main agent");

        assert!(error.contains("completed narrow inspect"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes["node-2"].status, NodeStatus::Ready);
        assert!(map.leases.is_empty());
    }

    #[test]
    fn spawn_agent_rejects_one_ready_inspect_while_main_holds_running_inspect() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect parser",
            "Main agent is already inspecting parser behavior.",
            true,
        );
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Inspect pricing".to_string(),
                "Investigate pricing behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create one ready inspect sibling");

        let error = state
            .prepare_spawn_assignment(owner, "pricing", Some("node-2"))
            .expect_err("one ready inspect sibling is not enough while main holds inspect");

        assert!(error.contains("main agent is already holding an inspect track"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes["node-1"].status, NodeStatus::Running);
        assert_eq!(map.nodes["node-2"].status, NodeStatus::Ready);
        assert_eq!(map.leases.len(), 1);
        assert_eq!(map.leases["lease-1"].holder, LeaseHolder::Main);
    }

    #[test]
    fn spawn_agent_allows_parallel_inspect_group_until_all_tracks_are_claimed() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "Overview", "Read project shape.", true);
        state
            .finish_main_node(owner, "node-1", "overview done".to_string(), None)
            .expect("finish overview");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parser investigation".to_string(),
                "Investigate parser behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create parser node");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Pricing investigation".to_string(),
                "Investigate pricing behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create pricing node");

        let first = state
            .prepare_spawn_assignment(owner, "parser", Some("node-2"))
            .expect("first parallel inspect assignment")
            .0
            .expect("assignment");
        state.attach_agent_to_lease(
            &first.lease_id,
            ThreadId::new(),
            Some("/root/parser".into()),
        );
        let second = state
            .prepare_spawn_assignment(owner, "pricing", Some("node-3"))
            .expect("second parallel inspect assignment")
            .0
            .expect("assignment");

        assert_eq!(first.node_id, "node-2");
        assert_eq!(second.node_id, "node-3");
    }

    #[test]
    fn create_node_bind_current_is_atomic_when_current_main_node_is_running() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Implement first",
            "Current node still owns the main lease.",
            true,
        );

        let error = state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::RegressionTest,
                "Regression validation".to_string(),
                "Run tests after implementation.".to_string(),
                Vec::new(),
                true,
            )
            .expect_err("create+bind must fail before mutating the map");

        assert!(error.contains("node-1"));
        assert!(error.contains("creating and binding"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes.len(), 1);
        assert!(!map.nodes.contains_key("node-2"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
    }

    #[test]
    fn create_node_bind_current_rejects_new_inspect_when_ready_inspect_exists_atomically() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "Overview", "Read project shape.", true);
        state
            .finish_main_node(owner, "node-1", "overview done".to_string(), None)
            .expect("finish overview");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parser investigation".to_string(),
                "Investigate parser behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create ready inspect node");

        let error = state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Pricing investigation".to_string(),
                "Investigate pricing behavior.".to_string(),
                Vec::new(),
                true,
            )
            .expect_err("cannot bind a new inspect node while another is ready");

        assert!(error.contains("ready inspect nodes already exist"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes.len(), 2);
        assert!(!map.nodes.contains_key("node-3"));
        assert!(state.current_main_node_id.is_none());
    }

    #[test]
    fn finish_node_next_inspect_rejects_parallel_ready_bind_without_mutating() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "Overview", "Read project shape.", true);
        state
            .finish_main_node_with_next(
                owner,
                "node-1",
                "overview done".to_string(),
                None,
                Some(ActionMapNextNodeDraft {
                    kind: NodeKind::InspectCodeContext,
                    title: "Read all tests".to_string(),
                    context_summary: "Read the existing tests.".to_string(),
                    dependency_node_ids: Vec::new(),
                }),
            )
            .expect("finish overview and bind next inspect");
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parser investigation".to_string(),
                "Investigate parser behavior.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create ready inspect node");

        let error = state
            .finish_main_node_with_next(
                owner,
                "node-2",
                "tests need delegated readers".to_string(),
                None,
                Some(ActionMapNextNodeDraft {
                    kind: NodeKind::InspectCodeContext,
                    title: "Pricing investigation".to_string(),
                    context_summary: "Investigate pricing behavior.".to_string(),
                    dependency_node_ids: Vec::new(),
                }),
            )
            .expect_err("finish with next inspect must reject before mutation");

        assert!(error.contains("Finish the current node without next_node_draft"));
        let map = state.active_map().expect("active map");
        assert_eq!(map.nodes.len(), 3);
        assert!(!map.nodes.contains_key("node-4"));
        let node = map.nodes.get("node-2").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert!(node.result_context.is_empty());
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));
    }

    #[test]
    fn main_held_node_is_not_claimable_by_subagent() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Main implementation",
            "Main agent owns this node.",
            true,
        );
        assert_eq!(node_id, "node-1");

        let error = state
            .prepare_spawn_assignment(owner, "parallel worker", None)
            .expect_err("main-held node is not claimable by subagents");

        assert!(error.contains("no ready node is available"));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(map.leases["lease-1"].holder, LeaseHolder::Main);
    }

    #[test]
    fn main_rebind_rejects_switching_without_finishing_previous_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(&mut state, owner, "First node", "First main node.", true);

        let error = state
            .create_node_for_main(
                owner,
                "Second node".to_string(),
                "Second main node.".to_string(),
                Vec::new(),
                true,
            )
            .expect_err("current node must be finished or blocked first");

        assert!(error.contains("node-1"));
        assert!(error.contains("finish_node"));
        let map = state.active_map().expect("active map");
        let first = map.nodes.get("node-1").expect("first node");
        assert_eq!(first.status, NodeStatus::Running);
        assert_eq!(first.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(map.leases.len(), 1);
        assert_eq!(map.leases["lease-1"].holder, LeaseHolder::Main);
    }

    #[test]
    fn subagent_result_and_timeout_paths_ignore_main_lease() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        let child = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Main implementation",
            "Main agent owns this node.",
            true,
        );

        assert!(
            state
                .attach_agent_to_lease("lease-1", child, Some("/child".to_string()))
                .is_none()
        );
        assert!(state.active_timeout_targets().is_empty());
        assert!(
            state
                .record_child_result(owner, &AgentStatus::Completed(Some("done".to_string())))
                .is_none()
        );

        let map = state.active_map().expect("active map");
        let lease = map.leases.get("lease-1").expect("main lease");
        assert_eq!(lease.holder, LeaseHolder::Main);
        assert_eq!(lease.agent_thread_id, Some(owner));
        assert!(lease.agent_path.is_none());
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert!(node.result_context.is_empty());
    }

    #[test]
    fn broad_main_node_hits_maintenance_barrier_after_tool_budget() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Inspect architecture",
            "Broad inspection node used by the regression fixture.",
            true,
        );

        let events = fill_main_tool_budget(&mut state, owner);

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierRaised(event)
                    if event.node_id == node_id
                        && event.result_count == MAIN_TOOL_RESULT_BUDGET_PER_NODE
                        && event.budget == MAIN_TOOL_RESULT_BUDGET_PER_NODE
            )
        }));
        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary tools should be blocked by barrier");
        assert!(error.contains("maintenance barrier"));
        let spawn_error = state
            .prepare_spawn_assignment(owner, "parallel follow-up", None)
            .expect_err("spawn has no non-barrier ready node to claim");
        assert!(spawn_error.contains("no ready node"));
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("Maintenance barrier"));
        assert!(context.contains("node_tool_result_budget_exceeded"));
    }

    #[test]
    fn maintenance_barrier_allows_subagent_on_different_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Broad inspection",
            "A broad node that will hit the main tool budget.",
            true,
        );
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Parallel evidence review".to_string(),
                "Ready node for a separate subagent.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create separate ready node");
        fill_main_tool_budget(&mut state, owner);
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());

        let (assignment, events) = state
            .prepare_spawn_assignment(owner, "parallel follow-up", Some("node-2"))
            .expect("barrier node should not block an explicit separate ready node");

        let assignment = assignment.expect("assignment");
        assert_eq!(assignment.node_id, "node-2");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseCreated(event) if event.node_id == "node-2"
            )
        }));
    }

    #[test]
    fn main_tool_budget_counts_inflight_parallel_calls() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Parallel inspection",
            "A node that receives many parallel read calls.",
            true,
        );

        for index in 0..MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(
                    owner,
                    ToolActionDescriptor::new("shell", ActionClass::Read, "")
                        .with_call_id(format!("call-{index}")),
                )
                .expect("in-flight call should reserve remaining budget");
        }

        let error = state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell", ActionClass::Read, "")
                    .with_call_id("call-over-budget"),
            )
            .expect_err("parallel calls beyond budget should be blocked before dispatch");
        let (message, events) = error.into_parts();
        assert!(message.contains("in-flight tool results"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierRaised(event)
                    if event.node_id == "node-1"
                        && event.result_count == MAIN_TOOL_RESULT_BUDGET_PER_NODE
                        && event.budget == MAIN_TOOL_RESULT_BUDGET_PER_NODE
            )
        }));
    }

    #[test]
    fn in_flight_main_tool_blocks_node_lifecycle_until_result_records() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Inspect before finish",
            "The main agent has a read call running.",
            true,
        );

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell", ActionClass::Read, "")
                    .with_call_id("read-call-1"),
            )
            .expect("read call should reserve the main node");

        let error = state
            .finish_main_node(owner, "node-1", "done".to_string(), None)
            .expect_err("node cannot finish while a main tool result is in flight");
        assert!(error.contains("in-flight main tool call"));

        let (result_id, _) = state
            .record_main_tool_result_with_class(
                owner,
                "read-call-1",
                "shell",
                Some(ActionClass::Read),
                true,
                "read completed".to_string(),
            )
            .expect("tool result records against the reserved lease")
            .expect("result should be recorded");
        assert_eq!(result_id, "result-1");
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(
            node.result_context
                .iter()
                .filter(|result| result.kind == NodeResultKind::MainToolCall)
                .count(),
            1
        );

        state
            .finish_main_node(owner, "node-1", "done".to_string(), None)
            .expect("finish succeeds after the tool result is recorded");
    }

    #[test]
    fn requested_main_node_spawn_rejects_running_main_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Running main node",
            "The main node has an in-flight tool call.",
            true,
        );

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("shell", ActionClass::Read, "")
                    .with_call_id("read-call-1"),
            )
            .expect("read call should reserve the main node");

        let error = state
            .prepare_spawn_assignment(owner, "parallel reader", Some("node-1"))
            .expect_err("main-held nodes are not handed off to subagents");
        assert!(error.contains("already held by an active lease"));

        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(map.leases["lease-1"].holder, LeaseHolder::Main);
    }

    #[test]
    fn maintenance_barrier_can_be_cleared_by_finishing_or_blocking_current_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Broad implementation",
            "A broad node that should be split after budget pressure.",
            true,
        );
        fill_main_tool_budget(&mut state, owner);
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());
        let bind_error = state
            .bind_main_node(owner, "node-1")
            .expect_err("binding the same overgrown node should not clear the barrier");
        assert!(bind_error.contains("different narrower recovery node"));

        let (_, events) = state
            .block_main_node(
                owner,
                "node-1",
                "Overgrown node needs narrower follow-up.".to_string(),
            )
            .expect("block overgrown node");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierCleared(event)
                    if event.node_id == "node-1" && event.reason == "node_lifecycle_recorded"
            )
        }));

        let (recovery_node_id, _) = state
            .create_node_for_main(
                owner,
                "Focused follow-up".to_string(),
                "Continue with a narrower recovery node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("recovery node created");

        assert_eq!(recovery_node_id, "node-2");
        state
            .prepare_main_tool_call(owner, "shell")
            .expect("recovery node should be allowed");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));
    }

    #[test]
    fn broad_completed_inspect_does_not_force_subagent_for_bound_implementation_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Simple bug fix",
            "Inspect enough to identify a single-line fix.",
            true,
        );
        fill_main_tool_budget(&mut state, owner);
        let (_, events) = state
            .finish_main_node_with_next(
                owner,
                "node-1",
                "Identified one-line fix.".to_string(),
                None,
                Some(ActionMapNextNodeDraft {
                    kind: NodeKind::ImplementSolution,
                    title: "Apply one-line fix".to_string(),
                    context_summary: "Change the identified rounding expression.".to_string(),
                    dependency_node_ids: vec!["node-1".to_string()],
                }),
            )
            .expect("broad inspect can finish into implementation");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierCleared(event)
                    if event.node_id == "node-1"
            )
        }));
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));

        state
            .prepare_main_tool_call(
                owner,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "single-line edit"),
            )
            .expect("clear implementation node should stay on main agent");
    }

    #[test]
    fn route_task_preserves_previous_task_maintenance_barrier() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main(
                owner,
                "Architecture audit".to_string(),
                "Find architecture risks.".to_string(),
                "Broad architecture audit".to_string(),
                "A deliberately broad node.".to_string(),
                false,
            )
            .expect("first task");
        state
            .start_task_for_main(
                owner,
                "Bug fix".to_string(),
                "Fix a separate bug.".to_string(),
                "Inspect bug".to_string(),
                "A separate task node.".to_string(),
                false,
            )
            .expect("second task");
        state
            .route_task_for_main(owner, "task-1")
            .expect("route to first task");
        state
            .bind_main_node(owner, "node-1")
            .expect("bind broad node");
        fill_main_tool_budget(&mut state, owner);
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());

        state
            .route_task_for_main(owner, "task-2")
            .expect("route away from task with barrier");
        let unbound_error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("routed task still needs an explicit main node binding");
        assert!(unbound_error.contains("no current node binding"));
        state
            .bind_main_node(owner, "node-2")
            .expect("bind second task node");
        state
            .prepare_main_tool_call(owner, "shell")
            .expect("other task should not be blocked by old task barrier after binding");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));
        state
            .route_task_for_main(owner, "task-1")
            .expect("route back to barrier task");

        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("old task barrier should still block ordinary work");
        assert!(error.contains("maintenance barrier"));
        let bind_error = state
            .bind_main_node(owner, "node-1")
            .expect_err("barrier node must not become bindable after routing");
        assert!(bind_error.contains("different narrower recovery node"));
    }

    #[test]
    fn start_task_preserves_previous_task_maintenance_barrier() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main(
                owner,
                "Broad task".to_string(),
                "Task that triggers the maintenance barrier.".to_string(),
                "Broad node".to_string(),
                "A deliberately broad node.".to_string(),
                true,
            )
            .expect("broad task");
        fill_main_tool_budget(&mut state, owner);

        state
            .start_task_for_main(
                owner,
                "Separate task".to_string(),
                "A separate task should not clear the old barrier.".to_string(),
                "Separate node".to_string(),
                "A separate node.".to_string(),
                false,
            )
            .expect("separate task");
        state
            .route_task_for_main(owner, "task-1")
            .expect("route back to broad task");

        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("start_task must not clear old task barrier");
        assert!(error.contains("maintenance barrier"));
    }

    #[test]
    fn restore_snapshot_preserves_task_map_and_barrier_state() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .start_task_for_main(
                owner,
                "Broad task".to_string(),
                "Task that triggers the maintenance barrier.".to_string(),
                "Broad node".to_string(),
                "A deliberately broad node.".to_string(),
                true,
            )
            .expect("broad task");
        fill_main_tool_budget(&mut state, owner);
        let snapshot = state.snapshot();
        assert_eq!(snapshot.maintenance_barriers.len(), 1);

        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot.clone());

        assert_eq!(restored.snapshot(), snapshot);
        let error = restored
            .prepare_main_tool_call(owner, "shell")
            .expect_err("restored barrier should block ordinary work");
        assert!(error.contains("maintenance barrier"));
        restored
            .block_main_node(
                owner,
                "node-1",
                "Overgrown restored node needs narrower follow-up.".to_string(),
            )
            .expect("block restored overgrown node");
        let (recovery_node_id, _) = restored
            .create_node_for_main(
                owner,
                "Recovery node".to_string(),
                "A narrower recovery node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("recovery node after restore");
        assert_eq!(recovery_node_id, "node-2");
    }

    #[test]
    fn restart_clears_maintenance_barrier_with_event() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        start_test_task(
            &mut state,
            owner,
            "Broad task",
            "A node that has become too broad.",
            true,
        );
        fill_main_tool_budget(&mut state, owner);
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());

        let (_, _, events) = state.restart_active_map(owner, "Reborn map");

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierCleared(event)
                    if event.node_id == "node-1" && event.reason == "map_restarted"
            )
        }));
        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("new empty map should require a concrete node, not report old barrier");
        assert!(error.contains("taskspace_control(action=create_node"));
        let context = state.build_developer_context().expect("context");
        assert!(!context.contains("Maintenance barrier"));
    }

    #[test]
    fn start_task_can_create_and_bind_first_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let (_, _, node_id, events) = start_test_task(
            &mut state,
            owner,
            "Review logging",
            "Check logging coverage before implementation.",
            true,
        );

        assert_eq!(node_id, "node-1");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MapCreated(event)
                    if event.map_id == "map-1" && event.title == "Review logging"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "node-1" && event.current_status == "ready"
            )
        }));
        let map = state.active_map().expect("active map");
        assert_eq!(map.title, "Review logging");
        assert_eq!(map.nodes.len(), 1);
        let node = map.nodes.get("node-1").expect("created node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(
            node.context.summary,
            "Check logging coverage before implementation."
        );
    }

    #[test]
    fn control_rejects_create_node_before_task_start_without_creating_map() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let error = state
            .create_node_for_main(
                owner,
                "Regression".to_string(),
                "Run after implementation.".to_string(),
                vec!["missing-upstream".to_string()],
                true,
            )
            .expect_err("create_node cannot bootstrap the first task");

        assert!(error.contains("TaskSpace bootstrap is required"));
        assert!(state.maps.is_empty());
        assert!(state.active_map_id.is_none());
    }

    #[test]
    fn control_rejects_binding_pending_dependency_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);

        let error = state
            .create_node_for_main(
                owner,
                "Regression".to_string(),
                "Run after implementation.".to_string(),
                vec!["implement_solution".to_string()],
                true,
            )
            .expect_err("pending dependency cannot bind");

        assert!(error.contains("cannot bind"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("define_scope"));
    }

    #[test]
    fn bind_main_node_allows_blocked_nodes_for_recovery_work() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        let map_id = state.active_map_id.clone().expect("active map");
        state.current_main_node_id = Some("inspect_code_context".to_string());
        state
            .maps
            .get_mut(&map_id)
            .expect("map")
            .nodes
            .get_mut("define_scope")
            .expect("node")
            .status = NodeStatus::Blocked;

        state
            .bind_main_node(owner, "define_scope")
            .expect("blocked nodes can be rebound");

        assert_eq!(state.current_main_node_id.as_deref(), Some("define_scope"));
    }

    #[test]
    fn finish_main_node_records_result_and_advances_downstream() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        state
            .bind_main_node(owner, "define_scope")
            .expect("bind main node");

        let (outcome, events) = state
            .finish_main_node(
                owner,
                "define_scope",
                "Scope is clear enough to inspect code context.".to_string(),
                Some("inspect_code_context".to_string()),
            )
            .expect("node finished");

        assert_eq!(outcome.result_id, "result-1");
        assert_eq!(
            outcome.next_node_id.as_deref(),
            Some("inspect_code_context")
        );
        assert_eq!(
            state.current_main_node_id.as_deref(),
            Some("inspect_code_context")
        );
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeResultRecorded(event)
                    if event.node_id == "define_scope"
                        && event.kind == "result"
                        && event.source_thread_id == owner
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "define_scope"
                        && event.current_status == "completed"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "inspect_code_context"
                        && event.current_status == "ready"
            )
        }));

        let map = state.active_map().expect("active map");
        let node = map.nodes.get("define_scope").expect("node");
        assert_eq!(node.status, NodeStatus::Completed);
        assert_eq!(node.result_context.len(), 1);
        let result = map.results.get(&outcome.result_id).expect("stored result");
        assert_eq!(result.kind, NodeResultKind::Result);
        assert_eq!(
            result.body,
            "Scope is clear enough to inspect code context."
        );
        assert_eq!(
            map.nodes
                .get("inspect_code_context")
                .expect("downstream")
                .status,
            NodeStatus::Running
        );
        assert_eq!(
            map.nodes
                .get("inspect_code_context")
                .expect("downstream")
                .active_lease
                .as_deref(),
            Some("lease-2")
        );
    }

    #[test]
    fn finish_implement_node_requires_successful_edit_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Fix rounding".to_string(),
                "Fix the tax rounding bug.".to_string(),
                "Apply code fix".to_string(),
                "Change calculate_tax rounding.".to_string(),
                true,
            )
            .expect("task starts");

        let error = state
            .finish_main_node(owner, &node_id, "Fixed rounding.".to_string(), None)
            .expect_err("implementation requires edit evidence");

        assert!(error.contains("cannot be completed without a recorded successful edit action"));
        assert_eq!(
            state.current_main_node_id.as_deref(),
            Some(node_id.as_str())
        );

        state
            .record_main_tool_result_with_class(
                owner,
                "call-edit",
                "apply_patch",
                Some(ActionClass::Edit),
                true,
                "changed round(..., 1) to round(..., 2)".to_string(),
            )
            .expect("edit result records");
        state
            .finish_main_node(owner, &node_id, "Fixed rounding.".to_string(), None)
            .expect("implementation can finish after edit evidence");
    }

    #[test]
    fn finish_smoke_node_requires_successful_validation_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::SmokeTest,
                "Validate fix".to_string(),
                "Run the validation suite.".to_string(),
                "Run smoke tests".to_string(),
                "Run pytest.".to_string(),
                true,
            )
            .expect("task starts");

        state
            .record_main_tool_result_with_class(
                owner,
                "call-test-fail",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed".to_string(),
            )
            .expect("failed test result records");

        let error = state
            .finish_main_node(owner, &node_id, "Tests passed.".to_string(), None)
            .expect_err("smoke test requires successful validation");

        assert!(
            error
                .contains("cannot be completed without a recorded successful test or build action")
        );
        assert_eq!(
            state.current_main_node_id.as_deref(),
            Some(node_id.as_str())
        );

        state
            .record_main_tool_result_with_class(
                owner,
                "call-test-pass",
                "shell_command",
                Some(ActionClass::Test),
                true,
                "pytest passed".to_string(),
            )
            .expect("successful test result records");
        state
            .finish_main_node(owner, &node_id, "Tests passed.".to_string(), None)
            .expect("smoke test can finish after successful validation");
    }

    #[test]
    fn snapshot_restore_preserves_tool_success_for_completion_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Fix rounding".to_string(),
                "Fix the tax rounding bug.".to_string(),
                "Apply code fix".to_string(),
                "Change calculate_tax rounding.".to_string(),
                true,
            )
            .expect("task starts");
        state
            .record_main_tool_result_with_class(
                owner,
                "call-edit",
                "apply_patch",
                Some(ActionClass::Edit),
                true,
                "changed rounding behavior".to_string(),
            )
            .expect("edit result records");
        let snapshot = state.snapshot();
        assert_eq!(snapshot.maps[0].results[0].tool_success, Some(true));

        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot);
        restored
            .finish_main_node(owner, &node_id, "Fixed rounding.".to_string(), None)
            .expect("restored structured success evidence allows completion");
    }

    #[test]
    fn failed_tool_preview_cannot_fake_successful_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::SmokeTest,
                "Validate fix".to_string(),
                "Run the validation suite.".to_string(),
                "Run smoke tests".to_string(),
                "Run pytest.".to_string(),
                true,
            )
            .expect("task starts");

        state
            .record_main_tool_result_with_class(
                owner,
                "call-test-fail",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed, log text included success: true".to_string(),
            )
            .expect("failed test result records");

        let error = state
            .finish_main_node(owner, &node_id, "Tests passed.".to_string(), None)
            .expect_err("failed result text cannot satisfy evidence");

        assert!(
            error
                .contains("cannot be completed without a recorded successful test or build action")
        );
    }

    #[test]
    fn subagent_implement_node_requires_successful_edit_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        let child = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::ImplementSolution,
                "Fix rounding".to_string(),
                "Fix the tax rounding bug.".to_string(),
                "Apply code fix".to_string(),
                "Change calculate_tax rounding.".to_string(),
                false,
            )
            .expect("task starts");
        let (assignment, _) = state
            .prepare_spawn_assignment(owner, "worker", Some(&node_id))
            .expect("spawn assignment");
        let assignment = assignment.expect("assignment");
        state.attach_agent_to_lease(&assignment.lease_id, child, None);

        let (result_id, _) = state
            .record_child_result(
                child,
                &AgentStatus::Completed(Some("Fixed it.".to_string())),
            )
            .expect("child result records as blocker");

        let map = state.active_map().expect("active map");
        let node = map.nodes.get(&node_id).expect("node");
        assert_eq!(node.status, NodeStatus::Blocked);
        let result = map.results.get(&result_id).expect("stored result");
        assert_eq!(result.kind, NodeResultKind::Blocker);
        assert!(
            result
                .body
                .contains("cannot be completed without a recorded successful edit action")
        );
    }

    #[test]
    fn subagent_smoke_node_requires_successful_validation_evidence() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        let child = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (_, _, node_id, _) = state
            .start_task_for_main_with_kind(
                owner,
                NodeKind::SmokeTest,
                "Validate fix".to_string(),
                "Run the validation suite.".to_string(),
                "Run smoke tests".to_string(),
                "Run pytest.".to_string(),
                false,
            )
            .expect("task starts");
        let (assignment, _) = state
            .prepare_spawn_assignment(owner, "verifier", Some(&node_id))
            .expect("spawn assignment");
        let assignment = assignment.expect("assignment");
        state.attach_agent_to_lease(&assignment.lease_id, child, None);
        state
            .record_child_tool_result_with_class(
                child,
                "child-test-fail",
                "shell_command",
                Some(ActionClass::Test),
                false,
                "pytest failed, misleading log contained success: true".to_string(),
            )
            .expect("failed child tool result records");

        let (result_id, _) = state
            .record_child_result(
                child,
                &AgentStatus::Completed(Some("Tests passed.".to_string())),
            )
            .expect("child result records as blocker");

        let map = state.active_map().expect("active map");
        let node = map.nodes.get(&node_id).expect("node");
        assert_eq!(node.status, NodeStatus::Blocked);
        let result = map.results.get(&result_id).expect("stored result");
        assert_eq!(result.kind, NodeResultKind::Blocker);
        assert!(
            result
                .body
                .contains("cannot be completed without a recorded successful test or build action")
        );
    }

    #[test]
    fn block_main_node_records_blocker_without_unlocking_downstream() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        state
            .bind_main_node(owner, "define_scope")
            .expect("bind main node");

        let (result_id, events) = state
            .block_main_node(
                owner,
                "define_scope",
                "Need user decision on the optimization boundary.".to_string(),
            )
            .expect("node blocked");

        assert_eq!(result_id, "result-1");
        assert!(state.current_main_node_id.is_none());
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeResultRecorded(event)
                    if event.node_id == "define_scope"
                        && event.kind == "blocker"
                        && event.source_thread_id == owner
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "define_scope" && event.current_status == "blocked"
            )
        }));

        let map = state.active_map().expect("active map");
        let node = map.nodes.get("define_scope").expect("node");
        assert_eq!(node.status, NodeStatus::Blocked);
        assert_eq!(node.result_context.len(), 1);
        let result = map.results.get(&result_id).expect("stored result");
        assert_eq!(result.kind, NodeResultKind::Blocker);
        assert_eq!(
            result.body,
            "Need user decision on the optimization boundary."
        );
        assert_eq!(
            map.nodes
                .get("inspect_code_context")
                .expect("downstream")
                .status,
            NodeStatus::Pending
        );
    }

    #[test]
    fn finish_main_node_rejects_invalid_next_node_without_mutating_current_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        state
            .bind_main_node(owner, "define_scope")
            .expect("bind main node");

        let error = state
            .finish_main_node(
                owner,
                "define_scope",
                "Scope is clear.".to_string(),
                Some("missing-node".to_string()),
            )
            .expect_err("invalid next node should reject before mutation");

        assert!(error.contains("next node `missing-node` does not exist"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("define_scope"));
        let map = state.active_map().expect("active map");
        assert!(map.results.is_empty());
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Running
        );
        assert_eq!(
            map.nodes
                .get("inspect_code_context")
                .expect("downstream")
                .status,
            NodeStatus::Pending
        );
    }

    #[test]
    fn block_main_node_does_not_auto_rebind_blocked_node_for_ordinary_work() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        state
            .bind_main_node(owner, "define_scope")
            .expect("bind main node");

        state
            .block_main_node(
                owner,
                "define_scope",
                "Need user decision on scope.".to_string(),
            )
            .expect("node blocked");

        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary tools require explicit recovery binding");

        assert!(error.contains("no current node binding"));
        assert!(state.current_main_node_id.is_none());
        let map = state.active_map().expect("active map");
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Blocked
        );
    }

    #[test]
    fn bind_main_node_rejects_subagent_held_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        let map_id = state.active_map_id.clone().expect("active map");
        {
            let map = state.maps.get_mut(&map_id).expect("map");
            let node = map.nodes.get_mut("define_scope").expect("node");
            node.status = NodeStatus::Running;
            node.active_lease = Some("lease-1".to_string());
        }

        let error = state
            .bind_main_node(owner, "define_scope")
            .expect_err("leased node cannot be rebound by main");

        assert!(error.contains("held by a subagent lease"));
    }

    #[test]
    fn main_binding_skips_running_nodes_held_by_subagents() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        let map_id = state.active_map_id.clone().expect("active map");
        {
            let map = state.maps.get_mut(&map_id).expect("map");
            map.nodes
                .get_mut("define_scope")
                .expect("define node")
                .status = NodeStatus::Completed;
            map.nodes
                .get_mut("inspect_code_context")
                .expect("inspect node")
                .status = NodeStatus::Running;
            map.nodes
                .get_mut("design_solution")
                .expect("design node")
                .status = NodeStatus::Ready;
        }
        state.current_main_node_id = Some("define_scope".to_string());

        state
            .ensure_main_binding_for_active_map(owner)
            .expect("main binding refresh");

        assert_eq!(
            state.current_main_node_id.as_deref(),
            Some("design_solution")
        );
    }

    #[test]
    fn main_binding_prefers_ready_nodes_before_blocked_recovery_nodes() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        seed_test_map(&mut state, owner);
        let map_id = state.active_map_id.clone().expect("active map");
        {
            let map = state.maps.get_mut(&map_id).expect("map");
            map.nodes
                .get_mut("define_scope")
                .expect("define node")
                .status = NodeStatus::Completed;
            map.nodes
                .get_mut("inspect_code_context")
                .expect("inspect node")
                .status = NodeStatus::Blocked;
            map.nodes
                .get_mut("design_solution")
                .expect("design node")
                .status = NodeStatus::Ready;
        }
        state.current_main_node_id = Some("define_scope".to_string());

        state
            .ensure_main_binding_for_active_map(owner)
            .expect("main binding refresh");

        assert_eq!(
            state.current_main_node_id.as_deref(),
            Some("design_solution")
        );
    }

    #[test]
    fn restore_mode_does_not_create_transition_notice() {
        let mut state = ActionMapRuntimeState::default();

        state.restore_mode(MapRuntimeMode::Experiment);

        assert_eq!(state.mode(), MapRuntimeMode::Experiment);
        assert!(state.take_pending_transition_notice().is_none());
    }

    #[test]
    fn developer_context_is_experiment_only_and_exposes_basemap_without_active_map() {
        let mut state = ActionMapRuntimeState::default();

        assert!(state.build_developer_context().is_none());

        state.set_mode(MapRuntimeMode::Experiment);
        let context = state.build_developer_context().expect("experiment context");
        assert!(context.contains("TaskSpace mode is active"));
        assert!(context.contains("Node kind selection rules"));
        assert!(context.contains("Do not create custom nodes"));
        assert!(context.contains("Use the minimum sufficient task map"));
        assert!(context.contains(
            "Do not create extra ready inspect nodes or call spawn_agent for simple work"
        ));
        assert!(context.contains(
            "Do not create another inspect node or call spawn_agent merely to read one known file"
        ));
        assert!(context.contains("Finish nodes with matching tool evidence"));
        assert!(context.contains("Pre-fix diagnostic tests"));
        assert!(context.contains("reconcile product docs"));
        assert!(context.contains("spawn_agent can only claim ready nodes"));
        assert!(context.contains("discover exact paths before reading files"));
        assert!(context.contains("do not substitute main-agent parallel shell/file-change calls"));
        assert!(context.contains("while two explorer agents own the two investigation nodes"));
        assert!(context.contains("BaseMap metadata version: base-map-v1"));
        assert!(context.contains("define_scope"));
    }

    #[test]
    fn create_seed_map_sets_active_map_context() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);

        let map_id =
            state.create_seed_map("map-1".to_string(), "Investigate runtime".to_string(), None);

        assert_eq!(map_id, "map-1");
        let context = state.build_developer_context().expect("experiment context");
        assert!(context.contains("Active task path"));
        assert!(context.contains("Investigate runtime"));
        assert!(context.contains("Node kind selection rules"));
        assert!(!context.contains("BaseMap metadata version"));
    }

    #[test]
    fn spawn_assignment_claims_one_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);

        let (assignment, events) = state
            .prepare_spawn_assignment(owner, "implement maps", None)
            .expect("assignment succeeds");
        let assignment = assignment.expect("experiment assignment");

        assert_eq!(assignment.node_id, "define_scope");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "define_scope"
                        && event.previous_status == "ready"
                        && event.current_status == "running"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseCreated(event)
                    if event.node_id == "define_scope"
                        && event.lease_id == "lease-1"
                        && event.holder == "subagent"
            )
        }));
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("define_scope:"));
        assert!(context.contains("[running]"));
    }

    #[test]
    fn dynamic_ready_node_is_visible_and_claimable_by_subagent() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let (_, _, node_id, _) = start_test_task(
            &mut state,
            owner,
            "Parallel review",
            "Review a side task while main work stays on define_scope.",
            false,
        );
        assert_eq!(node_id, "node-1");
        assert!(state.current_main_node_id.is_none());
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("- node-1: Parallel review kind=inspect_code_context [ready]"));

        let (assignment, _) = state
            .prepare_spawn_assignment(owner, "parallel review", None)
            .expect("dynamic assignment succeeds");
        let assignment = assignment.expect("experiment assignment");

        assert_eq!(assignment.node_id, "node-1");
        assert_eq!(assignment.node_title, "Parallel review");
    }

    #[test]
    fn spawn_assignment_requires_node_id_when_multiple_ready_nodes_exist() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Parallel review",
            "Review a side task.",
            false,
        );
        let (second_node_id, _) = state
            .create_node_for_main(
                owner,
                "Parallel implementation".to_string(),
                "Implement another independent side task.".to_string(),
                Vec::new(),
                false,
            )
            .expect("second ready node");
        assert_eq!(second_node_id, "node-2");

        let error = state
            .prepare_spawn_assignment(owner, "parallel work", None)
            .expect_err("ambiguous ready nodes must require an explicit node_id");

        assert!(error.contains("multiple ready nodes"));
        assert!(error.contains("node-1 (Parallel review)"));
        assert!(error.contains("node-2 (Parallel implementation)"));
    }

    #[test]
    fn spawn_assignment_claims_requested_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Parallel review",
            "Review a side task.",
            false,
        );
        state
            .create_node_for_main(
                owner,
                "Parallel implementation".to_string(),
                "Implement another independent side task.".to_string(),
                Vec::new(),
                false,
            )
            .expect("second ready node");

        let (assignment, _) = state
            .prepare_spawn_assignment(owner, "parallel work", Some("node-2"))
            .expect("explicit node assignment succeeds");
        let assignment = assignment.expect("experiment assignment");

        assert_eq!(assignment.node_id, "node-2");
        let map = state.active_map().expect("active map");
        assert_eq!(
            map.nodes.get("node-1").expect("first node").status,
            NodeStatus::Ready
        );
        assert_eq!(
            map.nodes.get("node-2").expect("second node").status,
            NodeStatus::Running
        );
    }

    #[test]
    fn spawn_assignment_rejects_handoff_of_current_main_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Parser investigation",
            "Main agent already owns this node.",
            true,
        );

        let error = state
            .prepare_spawn_assignment(owner, "delegate parser investigation", Some("node-1"))
            .expect_err("main-held node handoff is rejected");

        assert!(error.contains("already held by an active lease"));
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
        assert_eq!(state.current_main_lease_id.as_deref(), Some("lease-1"));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        let lease_id = node.active_lease.as_ref().expect("main lease");
        let lease = map.leases.get(lease_id).expect("lease");
        assert_eq!(lease.holder, LeaseHolder::Main);
    }

    #[test]
    fn spawn_assignment_rejects_requested_non_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);

        let error = state
            .prepare_spawn_assignment(owner, "pending work", Some("inspect_code_context"))
            .expect_err("pending dependency node must not be claimable");

        assert!(error.contains("still pending"));
    }

    #[test]
    fn spawn_assignment_rejects_requested_blocked_completed_running_and_leased_nodes() {
        let cases = [
            (NodeStatus::Blocked, "blocked"),
            (NodeStatus::Completed, "completed"),
            (NodeStatus::Running, "already running"),
        ];

        for (status, expected_error) in cases {
            let mut state = ActionMapRuntimeState::default();
            state.set_mode(MapRuntimeMode::Experiment);
            let owner = ThreadId::new();
            start_test_task(
                &mut state,
                owner,
                "Explicit node test",
                "Verify requested node status validation.",
                false,
            );
            let map_id = state.active_map_id.clone().expect("active map id");
            state
                .maps
                .get_mut(&map_id)
                .expect("active map")
                .nodes
                .get_mut("node-1")
                .expect("node")
                .status = status;
            let before = state.snapshot();

            let error = state
                .prepare_spawn_assignment(owner, "explicit work", Some("node-1"))
                .expect_err("non-ready node must not be claimable");

            assert!(
                error.contains(expected_error),
                "expected error containing {expected_error:?}, got {error:?}"
            );
            let after = state.snapshot();
            assert_eq!(after.maps, before.maps);
        }

        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Explicit node test",
            "Verify requested leased node validation.",
            false,
        );
        let first = state
            .prepare_spawn_assignment(owner, "first holder", Some("node-1"))
            .expect("first claim succeeds")
            .0
            .expect("assignment");
        let before = state.snapshot();
        let error = state
            .prepare_spawn_assignment(owner, "second holder", Some("node-1"))
            .expect_err("leased node must not be claimable");

        assert_eq!(first.node_id, "node-1");
        assert!(error.contains("already held by an active lease"));
        let after = state.snapshot();
        assert_eq!(after.maps, before.maps);
    }

    #[test]
    fn child_result_can_be_recorded_after_late_lease_attach() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "fast child", None)
            .expect("claim")
            .0
            .expect("assignment");

        let missed = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("finished before attach".to_string())),
        );
        assert!(missed.is_none());
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );

        let result = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("finished before attach".to_string())),
        );

        assert_eq!(result.as_ref().map(|(id, _)| id.as_str()), Some("result-1"));
        let map = state.active_map().expect("active map");
        assert!(map.leases.is_empty());
        let node = map.nodes.get("define_scope").expect("node");
        assert_eq!(node.status, NodeStatus::Completed);
        assert_eq!(
            map.results.get("result-1").expect("stored result").body,
            "finished before attach"
        );
    }

    #[test]
    fn attach_agent_to_lease_rejects_different_thread_after_attach() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let first_child = ThreadId::new();
        let second_child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "first child", None)
            .expect("claim")
            .0
            .expect("assignment");

        let first_attach = state.attach_agent_to_lease(
            &assignment.lease_id,
            first_child,
            Some("/root/first".to_string()),
        );
        let second_attach = state.attach_agent_to_lease(
            &assignment.lease_id,
            second_child,
            Some("/root/second".to_string()),
        );
        let repeated_attach = state.attach_agent_to_lease(
            &assignment.lease_id,
            first_child,
            Some("/root/first".to_string()),
        );

        assert!(first_attach.is_some());
        assert!(second_attach.is_none());
        assert!(repeated_attach.is_some());
        let map = state.active_map().expect("active map");
        let lease = map.leases.get(&assignment.lease_id).expect("lease");
        assert_eq!(lease.agent_thread_id, Some(first_child));
        assert_eq!(lease.agent_path.as_deref(), Some("/root/first"));
    }

    #[test]
    fn duplicate_child_result_after_lease_release_is_ignored() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "fast child", None)
            .expect("claim")
            .0
            .expect("assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );
        let first = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("first final result".to_string())),
        );
        assert_eq!(first.as_ref().map(|(id, _)| id.as_str()), Some("result-1"));
        let before = state.snapshot();

        let second = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("duplicate final result".to_string())),
        );

        assert!(second.is_none());
        let after = state.snapshot();
        assert_eq!(after.maps, before.maps);
    }

    #[test]
    fn standard_mode_spawn_assignment_is_disabled() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();

        let assignment = state
            .prepare_spawn_assignment(owner, "standard task", None)
            .expect("standard mode should not fail");

        assert!(assignment.0.is_none());
        assert!(assignment.1.is_empty());
        assert!(state.active_map_id.is_none());
        assert!(state.maps.is_empty());
    }

    #[test]
    fn experiment_spawn_assignment_requires_agent_created_task_path() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let error = state
            .prepare_spawn_assignment(owner, "standard task", None)
            .expect_err("spawn requires a task path first");

        assert!(error.contains("taskspace_control(action=start_task"));
        assert!(state.active_map_id.is_none());
        assert!(state.maps.is_empty());
    }

    #[test]
    fn running_node_blocks_second_claim_until_lease_is_released() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let first = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("first claim succeeds")
            .0
            .expect("first assignment");

        let second = state
            .prepare_spawn_assignment(owner, "second", None)
            .expect_err("no second node should be ready while the first is running");
        assert!(second.contains("no ready node is available"));

        state.release_lease(&first.lease_id, "test_release");
        let reclaimed = state
            .prepare_spawn_assignment(owner, "second", None)
            .expect("released node can be claimed again")
            .0
            .expect("reclaimed assignment");
        assert_eq!(reclaimed.node_id, "define_scope");
        assert_eq!(reclaimed.lease_id, "lease-2");
    }

    #[test]
    fn release_lease_for_thread_returns_node_to_ready() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("claim")
            .0
            .expect("assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );

        let released = state.release_lease_for_thread(child, "test_release");

        assert_eq!(
            released.as_ref().map(|(id, _)| id.as_str()),
            Some("lease-1")
        );
        let map = state.active_map().expect("active map");
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Ready
        );
        assert!(map.leases.is_empty());
    }

    #[test]
    fn completed_result_advances_next_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "implement maps", None)
            .expect("assignment succeeds")
            .0
            .expect("experiment assignment");
        let attach_event =
            state.attach_agent_to_lease(&assignment.lease_id, child, Some("/define".to_string()));
        assert!(matches!(
            attach_event,
            Some(MapRuntimeEvent::LeaseAttached(event))
                if event.node_id == "define_scope"
                    && event.lease_id == "lease-1"
                    && event.agent_thread_id == child
        ));

        let result = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("scope is clear".to_string())),
        );

        let (result_id, events) = result.expect("result should record");
        assert_eq!(result_id, "result-1");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeResultRecorded(event)
                    if event.node_id == "define_scope"
                        && event.lease_id == "lease-1"
                        && event.result_id == "result-1"
                        && event.kind == "result"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseReleased(event)
                    if event.node_id == "define_scope"
                        && event.lease_id == "lease-1"
                        && event.reason == "result_recorded"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "inspect_code_context"
                        && event.previous_status == "pending"
                        && event.current_status == "ready"
            )
        }));
        let map = state.active_map().expect("active map");
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Completed
        );
        assert_eq!(
            map.nodes.get("inspect_code_context").expect("node").status,
            NodeStatus::Ready
        );
    }

    #[test]
    fn snapshot_and_formatter_expose_map_runtime_state() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();

        let assignment = state
            .prepare_spawn_assignment(owner, "inspect runtime", None)
            .expect("assignment succeeds")
            .0
            .expect("experiment assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );
        state.record_child_result(
            child,
            &AgentStatus::Completed(Some("scope is clear".to_string())),
        );

        let snapshot = state.snapshot();
        assert_eq!(snapshot.mode, MapRuntimeMode::Experiment);
        assert_eq!(snapshot.active_task_id.as_deref(), Some("task-1"));
        assert_eq!(
            snapshot.active_map_id.as_deref(),
            Some(assignment.map_id.as_str())
        );
        assert_eq!(snapshot.tasks.len(), 1);
        assert_eq!(snapshot.tasks[0].id, "task-1");
        assert_eq!(snapshot.tasks[0].active_map_id.as_deref(), Some("map-1"));
        assert_eq!(snapshot.tasks[0].map_ids, vec!["map-1".to_string()]);
        assert_eq!(snapshot.maps.len(), 1);
        let map = &snapshot.maps[0];
        assert_eq!(map.id, assignment.map_id);
        assert_eq!(map.task_id.as_deref(), Some("task-1"));
        assert_eq!(map.completed_node_count, 1);
        assert_eq!(map.ready_node_count, 1);
        assert!(map.leases.is_empty());
        assert_eq!(map.results.len(), 1);
        assert_eq!(map.results[0].node_id, "define_scope");
        assert_eq!(map.results[0].kind, "result");
        assert_eq!(map.results[0].body, "scope is clear");
        let completed_node = map
            .nodes
            .iter()
            .find(|node| node.id == "define_scope")
            .expect("completed node");
        assert_eq!(completed_node.status, "completed");
        assert_eq!(completed_node.result_ids, vec!["result-1".to_string()]);

        let formatted = format_action_map_snapshot(&snapshot);
        assert!(formatted.contains("TaskSpace"));
        assert!(formatted.contains("mode: experiment"));
        assert!(formatted.contains("trace events: total=0"));
        assert!(formatted.contains("define_scope"));
        assert!(formatted.contains("result-1 node=define_scope kind=result"));
        assert!(formatted.contains("scope is clear"));
    }

    #[test]
    fn restart_with_different_owner_creates_distinct_task() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let first_owner = ThreadId::new();
        let second_owner = ThreadId::new();

        start_test_task(
            &mut state,
            first_owner,
            "Initial task",
            "Initial owner task.",
            true,
        );

        let (previous_map, next_map, _) = state.restart_active_map(second_owner, "Second task");

        assert_eq!(previous_map.as_deref(), Some("map-1"));
        assert_eq!(next_map, "map-2");
        let snapshot = state.snapshot();
        assert_eq!(snapshot.active_task_id.as_deref(), Some("task-2"));
        assert_eq!(snapshot.tasks.len(), 2);
        let first_task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-1")
            .expect("first task");
        let second_task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-2")
            .expect("second task");
        assert_eq!(first_task.status, "pending");
        assert_eq!(first_task.map_ids, vec!["map-1".to_string()]);
        assert_eq!(second_task.status, "active");
        assert_eq!(second_task.map_ids, vec!["map-2".to_string()]);
        assert_eq!(snapshot.maps[0].task_id.as_deref(), Some("task-1"));
        assert_eq!(snapshot.maps[1].task_id.as_deref(), Some("task-2"));
    }

    #[test]
    fn start_task_creates_distinct_task_map_and_first_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let (task_id, map_id, node_id, events) = state
            .start_task_for_main(
                owner,
                "Architecture quality review".to_string(),
                "Inspect architecture risks before refactoring.".to_string(),
                "Define quality review boundary".to_string(),
                "Clarify scope, key modules, and evidence needed for the review.".to_string(),
                true,
            )
            .expect("task should start");

        assert_eq!(task_id, "task-1");
        assert_eq!(map_id, "map-1");
        assert_eq!(node_id, "node-1");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::TaskCreated(event)
                    if event.task_id == "task-1"
                        && event.active_map_id.as_deref() == Some("map-1")
                        && event.title == "Architecture quality review"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MapCreated(event)
                    if event.map_id == "map-1" && event.title == "Architecture quality review"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseCreated(event)
                    if event.node_id == "node-1" && event.holder == "main"
            )
        }));
        let snapshot = state.snapshot();
        assert_eq!(snapshot.active_task_id.as_deref(), Some("task-1"));
        assert_eq!(snapshot.active_map_id.as_deref(), Some("map-1"));
        assert_eq!(
            snapshot.tasks[0].objective,
            "Inspect architecture risks before refactoring."
        );
        assert_eq!(snapshot.maps[0].nodes[0].id, "node-1");
        assert_eq!(snapshot.maps[0].nodes[0].status, "running");
    }

    #[test]
    fn subagent_tool_calls_are_gated_by_assigned_node_contract() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        let child = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Inspect pricing",
            "Read pricing code and report findings.",
            false,
        );

        let assignment = state
            .prepare_spawn_assignment(owner, "inspect pricing", Some("node-1"))
            .expect("spawn assignment")
            .0
            .expect("experiment assignment");
        assert!(
            assignment
                .message_prefix
                .contains("Node kind: inspect_code_context")
        );
        assert!(
            assignment
                .message_prefix
                .contains("Allowed action classes:")
        );
        assert!(assignment.message_prefix.contains("Runtime enforces"));
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/explorer".to_string()),
        );

        state
            .prepare_child_tool_call(
                child,
                ToolActionDescriptor::new("shell", ActionClass::Read, "Get-ChildItem")
                    .with_call_id("child-read"),
            )
            .expect("read is allowed on inspect node");
        let (result_id, _) = state
            .record_child_tool_result_with_class(
                child,
                "child-read",
                "shell",
                Some(ActionClass::Read),
                true,
                "read ok".to_string(),
            )
            .expect("child result record succeeds")
            .expect("child tool result recorded");

        let map = state.maps.get("map-1").expect("map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(
            node.active_lease.as_deref(),
            Some(assignment.lease_id.as_str())
        );
        let result = map.results.get(&result_id).expect("result");
        assert_eq!(result.source_thread_id, child);
        assert_eq!(result.action_class, Some(ActionClass::Read));

        let error = state
            .prepare_child_tool_call(
                child,
                ToolActionDescriptor::new("apply_patch", ActionClass::Edit, "patch")
                    .with_call_id("child-edit"),
            )
            .expect_err("inspect subagent cannot edit");
        let (message, events) = error.into_parts();
        assert!(message.contains("does not allow edit"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::ToolActionBlocked(event)
                    if event.node_id == "node-1"
                        && event.node_kind == "inspect_code_context"
                        && event.action_class == "edit"
            )
        }));
    }

    #[test]
    fn subagent_without_taskspace_lease_is_blocked_in_experiment_mode() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let child = ThreadId::new();

        let error = state
            .prepare_child_tool_call(
                child,
                ToolActionDescriptor::new("shell", ActionClass::Read, "Get-ChildItem")
                    .with_call_id("orphan-read"),
            )
            .expect_err("orphan subagent is not map-driven");

        assert!(error.contains("no active task node lease"));
    }

    #[test]
    fn node_bound_subagent_cannot_spawn_nested_agent() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        let child = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Inspect pricing",
            "Read pricing code and report findings.",
            false,
        );
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Inspect discounts".to_string(),
                "Read discount code and report findings.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create second inspect track");
        let assignment = state
            .prepare_spawn_assignment(owner, "inspect pricing", Some("node-1"))
            .expect("spawn assignment")
            .0
            .expect("experiment assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/explorer".to_string()),
        );

        let error = state
            .prepare_child_spawn(child)
            .expect_err("node-bound subagent cannot spawn nested agents");

        assert!(error.contains("blocked nested spawn_agent"));
    }

    #[test]
    fn orphan_subagent_spawn_is_blocked_in_experiment_mode() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);

        let error = state
            .prepare_child_spawn(ThreadId::new())
            .expect_err("orphan subagent cannot spawn through TaskSpace");

        assert!(error.contains("no active task node lease"));
    }

    #[test]
    fn child_tool_reservations_are_cleared_when_child_result_finishes_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        let child = ThreadId::new();
        start_test_task(
            &mut state,
            owner,
            "Inspect pricing",
            "Read pricing code and report findings.",
            false,
        );
        state
            .create_node_for_main_with_kind(
                owner,
                NodeKind::InspectCodeContext,
                "Inspect discounts".to_string(),
                "Read discount code and report findings.".to_string(),
                Vec::new(),
                false,
            )
            .expect("create second inspect track");
        let assignment = state
            .prepare_spawn_assignment(owner, "inspect pricing", Some("node-1"))
            .expect("spawn assignment")
            .0
            .expect("experiment assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/explorer".to_string()),
        );

        state
            .prepare_child_tool_call(
                child,
                ToolActionDescriptor::new("shell", ActionClass::Read, "Get-ChildItem")
                    .with_call_id("child-read"),
            )
            .expect("read is allowed on inspect node");
        assert_eq!(state.child_tool_reservations.len(), 1);

        state.record_child_result(
            child,
            &AgentStatus::Completed(Some("inspection complete".to_string())),
        );

        assert!(state.child_tool_reservations.is_empty());
    }

    #[test]
    fn start_task_marks_previous_task_pending_and_releases_main_lease() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        state
            .start_task_for_main(
                owner,
                "First task".to_string(),
                "First objective.".to_string(),
                "First node".to_string(),
                "First node context.".to_string(),
                true,
            )
            .expect("first task");
        let (_, _, _, events) = state
            .start_task_for_main(
                owner,
                "Second task".to_string(),
                "Second objective.".to_string(),
                "Second node".to_string(),
                "Second node context.".to_string(),
                false,
            )
            .expect("second task");

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::TaskStatusChanged(event)
                    if event.task_id == "task-1"
                        && event.previous_status == "active"
                        && event.current_status == "pending"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseReleased(event)
                    if event.node_id == "node-1"
                        && event.holder == "main"
                        && event.reason == "task_started"
            )
        }));
        let snapshot = state.snapshot();
        let first = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-1")
            .expect("first task");
        let second = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-2")
            .expect("second task");
        assert_eq!(first.status, "pending");
        assert_eq!(second.status, "active");
        assert_eq!(snapshot.active_task_id.as_deref(), Some("task-2"));
        assert!(state.current_main_node_id.is_none());
        assert_eq!(
            state
                .maps
                .get("map-1")
                .expect("first map")
                .nodes
                .get("node-1")
                .expect("first node")
                .status,
            NodeStatus::Ready
        );
    }

    #[test]
    fn user_turn_requires_routing_before_work_and_spawn() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        state
            .start_task_for_main(
                owner,
                "Existing task".to_string(),
                "Existing objective.".to_string(),
                "Current node".to_string(),
                "Continue existing work.".to_string(),
                true,
            )
            .expect("task");

        state.begin_user_turn();
        let work_error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary work must wait for routing");
        assert!(work_error.contains("TaskSpace task routing is required"));
        let spawn_error = state
            .prepare_spawn_assignment(owner, "parallel", None)
            .expect_err("spawn must wait for routing");
        assert!(spawn_error.contains("TaskSpace task routing is required"));

        let events = state
            .route_task_for_main(owner, "task-1")
            .expect("same task routing clears the turn gate");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::TaskRouted(event)
                    if event.current_task_id == "task-1"
                        && event.current_map_id == "map-1"
            )
        }));
        state
            .prepare_main_tool_call(owner, "shell")
            .expect("routing cleared");
    }

    #[test]
    fn snapshot_restore_preserves_routing_gate() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        state
            .start_task_for_main(
                owner,
                "Existing task".to_string(),
                "Existing objective.".to_string(),
                "Current node".to_string(),
                "Continue existing work.".to_string(),
                true,
            )
            .expect("task");
        state.begin_user_turn();

        let snapshot = state.snapshot();
        assert!(snapshot.routing_required);
        assert!(!snapshot.bootstrap_required);

        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot);
        let error = restored
            .prepare_main_tool_call(owner, "shell")
            .expect_err("restored routing gate should block ordinary work");
        assert!(error.contains("TaskSpace task routing is required"));
    }

    #[test]
    fn route_task_switches_active_task_and_releases_current_main_lease() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        state
            .start_task_for_main(
                owner,
                "First task".to_string(),
                "First objective.".to_string(),
                "First node".to_string(),
                "First node context.".to_string(),
                false,
            )
            .expect("first task");
        state
            .start_task_for_main(
                owner,
                "Second task".to_string(),
                "Second objective.".to_string(),
                "Second node".to_string(),
                "Second node context.".to_string(),
                true,
            )
            .expect("second task");

        let events = state
            .route_task_for_main(owner, "task-1")
            .expect("route to existing task");

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::TaskRouted(event)
                    if event.previous_task_id.as_deref() == Some("task-2")
                        && event.current_task_id == "task-1"
                        && event.previous_map_id.as_deref() == Some("map-2")
                        && event.current_map_id == "map-1"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseReleased(event)
                    if event.node_id == "node-2"
                        && event.holder == "main"
                        && event.reason == "task_routed"
            )
        }));
        let snapshot = state.snapshot();
        assert_eq!(snapshot.active_task_id.as_deref(), Some("task-1"));
        assert_eq!(snapshot.active_map_id.as_deref(), Some("map-1"));
        let first = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-1")
            .expect("first task");
        let second = snapshot
            .tasks
            .iter()
            .find(|task| task.id == "task-2")
            .expect("second task");
        assert_eq!(first.status, "active");
        assert_eq!(second.status, "pending");
        assert!(state.current_main_node_id.is_none());

        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary work should require explicit binding after routing");
        assert!(error.contains("no current node binding"));
        state
            .bind_main_node(owner, "node-1")
            .expect("agent can explicitly bind routed task node");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-1"));
    }

    #[test]
    fn route_task_rejects_missing_task_without_mutation() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        state
            .start_task_for_main(
                owner,
                "Existing task".to_string(),
                "Existing objective.".to_string(),
                "Existing node".to_string(),
                "Existing node context.".to_string(),
                true,
            )
            .expect("existing task");
        let before = state.snapshot();

        let error = state
            .route_task_for_main(owner, "task-missing")
            .expect_err("missing task should fail");

        assert!(error.contains("does not exist"));
        let after = state.snapshot();
        assert_eq!(after.active_task_id, before.active_task_id);
        assert_eq!(after.active_map_id, before.active_map_id);
        assert_eq!(after.tasks, before.tasks);
        assert_eq!(after.maps, before.maps);
    }

    #[test]
    fn developer_context_exposes_task_inventory_for_agent_routing() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        state
            .start_task_for_main(
                owner,
                "Architecture review".to_string(),
                "Find structural risks.".to_string(),
                "Scope architecture review".to_string(),
                "Collect architecture review scope.".to_string(),
                false,
            )
            .expect("task");

        let context = state.build_developer_context().expect("context");

        assert!(context.contains("Task inventory:"));
        assert!(context.contains("task-1 [active] Architecture review active_map=map-1"));
        assert!(context.contains("objective: Find structural risks."));
        assert!(context.contains("taskspace_control(action=route_task)"));
        assert!(context.contains("taskspace_control(action=start_task)"));
    }

    #[test]
    fn errored_result_blocks_node_and_does_not_unlock_downstream() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("claim")
            .0
            .expect("assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );

        let result = state.record_child_result(child, &AgentStatus::Errored("boom".to_string()));

        assert_eq!(result.as_ref().map(|(id, _)| id.as_str()), Some("result-1"));
        let map = state.active_map().expect("active map");
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Blocked
        );
        assert_eq!(
            map.nodes.get("inspect_code_context").expect("node").status,
            NodeStatus::Pending
        );
        let stored = map.results.get("result-1").expect("stored result");
        assert_eq!(stored.kind, NodeResultKind::Blocker);
        assert_eq!(stored.body, "boom");
    }

    #[test]
    fn non_final_child_result_blocks_node_and_releases_lease() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("claim")
            .0
            .expect("assignment");
        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );

        let result = state.record_child_result(child, &AgentStatus::Running);

        assert_eq!(result.as_ref().map(|(id, _)| id.as_str()), Some("result-1"));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("define_scope").expect("node");
        assert_eq!(node.status, NodeStatus::Blocked);
        assert!(node.active_lease.is_none());
        assert!(map.leases.is_empty());
        let stored = map.results.get("result-1").expect("stored result");
        assert_eq!(stored.kind, NodeResultKind::Blocker);
        assert!(stored.body.contains("non-final status"));
    }

    #[test]
    fn unknown_child_result_does_not_mutate_map() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let assignment = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("claim")
            .0
            .expect("assignment");

        let result = state.record_child_result(
            ThreadId::new(),
            &AgentStatus::Completed(Some("orphan".to_string())),
        );

        assert!(result.is_none());
        let map = state.active_map().expect("active map");
        assert!(map.results.is_empty());
        assert_eq!(
            map.nodes.get(&assignment.node_id).expect("node").status,
            NodeStatus::Running
        );
    }

    #[test]
    fn active_timeout_targets_require_attached_thread_and_parse_agent_path() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let child = ThreadId::new();
        let assignment = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("claim")
            .0
            .expect("assignment");

        assert!(state.active_timeout_targets().is_empty());

        state.attach_agent_to_lease(
            &assignment.lease_id,
            child,
            Some("/root/worker".to_string()),
        );
        let targets = state.active_timeout_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].thread_id, child);
        assert_eq!(targets[0].node_id, "define_scope");
        assert_eq!(
            targets[0].agent_path.as_ref().map(AgentPath::as_str),
            Some("/root/worker")
        );
        assert!(matches!(
            ActionMapRuntimeState::timeout_summary_requested_event(&targets[0]),
            Some(MapRuntimeEvent::TimeoutSummaryRequested(event))
                if event.node_id == "define_scope"
                    && event.lease_id == "lease-1"
                    && event.agent_thread_id == child
                    && event.agent_path == "/root/worker"
        ));
    }

    #[test]
    fn completing_all_known_nodes_keeps_map_active_for_growth() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let mut map_id = None;

        for expected_node in SEED_NODE_IDS {
            let child = ThreadId::new();
            let assignment = state
                .prepare_spawn_assignment(owner, expected_node, None)
                .expect("claim")
                .0
                .expect("assignment");
            map_id = Some(assignment.map_id.clone());
            assert_eq!(assignment.node_id, *expected_node);
            state.attach_agent_to_lease(
                &assignment.lease_id,
                child,
                Some(format!("/root/{expected_node}")),
            );
            match *expected_node {
                "implement_solution" => {
                    state
                        .record_child_tool_result_with_class(
                            child,
                            "child-edit",
                            "apply_patch",
                            Some(ActionClass::Edit),
                            true,
                            "implementation changed".to_string(),
                        )
                        .expect("edit evidence records");
                }
                "smoke_test" => {
                    state
                        .record_child_tool_result_with_class(
                            child,
                            "child-test",
                            "shell_command",
                            Some(ActionClass::Test),
                            true,
                            "smoke test passed".to_string(),
                        )
                        .expect("test evidence records");
                }
                _ => {}
            }
            state.record_child_result(
                child,
                &AgentStatus::Completed(Some(format!("{expected_node} done"))),
            );
        }

        let map_id = map_id.expect("map id");
        let map = state.maps.get(&map_id).expect("map");
        assert_eq!(map.status, MapStatus::Active);
        assert_eq!(state.active_map().expect("active").id, map_id);
        assert_eq!(map.results.len(), SEED_NODE_IDS.len() + 2);
    }

    #[test]
    fn restart_abandons_previous_map_and_creates_empty_task_path() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let first = state
            .prepare_spawn_assignment(owner, "first", None)
            .expect("assignment")
            .0
            .expect("experiment")
            .map_id;

        let (previous, next, events) = state.restart_active_map(owner, "Restarted map");

        assert_eq!(previous.as_deref(), Some(first.as_str()));
        assert_ne!(first, next);
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MapStatusChanged(event)
                    if event.map_id == first
                        && event.previous_status == "active"
                        && event.current_status == "abandoned"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MapCreated(event)
                    if event.map_id == next && event.created_from.as_deref() == Some(first.as_str())
            )
        }));
        assert_eq!(
            state.maps.get(&first).expect("previous").status,
            MapStatus::Abandoned
        );
        let active = state.active_map().expect("active");
        assert_eq!(active.id, next);
        assert!(active.nodes.is_empty());
        assert!(state.current_main_node_id.is_none());
    }

    #[test]
    fn restart_without_existing_map_creates_empty_task_path() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let (previous, next, _) = state.restart_active_map(owner, "Fresh map");

        assert!(previous.is_none());
        let map = state.active_map().expect("active map");
        assert_eq!(map.id, next);
        assert_eq!(map.title, "Fresh map");
        assert!(map.nodes.is_empty());
        assert!(state.current_main_node_id.is_none());
    }
}
