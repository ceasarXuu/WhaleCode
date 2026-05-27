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
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeLeaseAttachedEvent;
use codex_protocol::protocol::MapRuntimeLeaseCreatedEvent;
use codex_protocol::protocol::MapRuntimeLeaseReleasedEvent;
use codex_protocol::protocol::MapRuntimeMaintenanceBarrierClearedEvent;
use codex_protocol::protocol::MapRuntimeMaintenanceBarrierRaisedEvent;
use codex_protocol::protocol::MapRuntimeMapCreatedEvent;
use codex_protocol::protocol::MapRuntimeMapStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeNodeResultRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTaskCreatedEvent;
use codex_protocol::protocol::MapRuntimeTaskRoutedEvent;
use codex_protocol::protocol::MapRuntimeTaskStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTimeoutSummaryRequestedEvent;

use super::basemap::BASE_MAP;
use super::basemap::base_map_metadata_prompt;
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
use super::map::NodeResult;
use super::map::NodeResultId;
use super::map::NodeResultKind;
use super::map::NodeResultRef;
use super::map::NodeStatus;
use super::map::TaskId;
use super::map::TaskState;
use super::map::TaskStatus;

const SEED_NODE_IDS: &[&str] = &[
    "define_scope",
    "inspect_code_context",
    "design_solution",
    "implement_solution",
    "smoke_test",
    "final_synthesis",
];

/// A main-agent node should stay narrow enough to be reviewable. The runtime
/// raises a maintenance barrier after this many main tool results are exceeded.
pub(crate) const MAIN_TOOL_RESULT_BUDGET_PER_NODE: usize = 12;

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    mode: MapRuntimeMode,
    pending_transition_notice: Option<String>,
    active_task_id: Option<TaskId>,
    active_map_id: Option<ActionMapId>,
    current_main_node_id: Option<MapNodeId>,
    current_main_lease_id: Option<AssignmentLeaseId>,
    maintenance_barriers: HashMap<ActionMapId, ActionMapMaintenanceBarrier>,
    tasks: HashMap<TaskId, TaskState>,
    maps: HashMap<ActionMapId, ActionMapInstance>,
    next_task_seq: u64,
    next_map_seq: u64,
    next_node_seq: u64,
    next_lease_seq: u64,
    next_result_seq: u64,
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
            active_task_id: None,
            active_map_id: None,
            current_main_node_id: None,
            current_main_lease_id: None,
            maintenance_barriers: HashMap::new(),
            tasks: HashMap::new(),
            maps: HashMap::new(),
            next_task_seq: 1,
            next_map_seq: 1,
            next_node_seq: 1,
            next_lease_seq: 1,
            next_result_seq: 1,
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
    }

    pub(crate) fn restore_snapshot(&mut self, snapshot: ActionMapSnapshot) {
        self.mode = snapshot.mode;
        self.pending_transition_notice = None;
        self.active_task_id = snapshot.active_task_id;
        self.active_map_id = snapshot.active_map_id;
        self.current_main_node_id = None;
        self.current_main_lease_id = None;

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
                        Some((
                            result.id.clone(),
                            NodeResult {
                                id: result.id,
                                assignment_id: result.assignment_id,
                                map_id: result.map_id,
                                node_id: result.node_id,
                                kind,
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
                        (
                            id.clone(),
                            MapNode {
                                id,
                                title: node.title,
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
        ActionMapSnapshot {
            mode: self.mode,
            active_task_id: self.active_task_id.clone(),
            active_map_id: self.active_map_id.clone(),
            tasks,
            maps,
            maintenance_barriers,
        }
    }

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
        let mut release_events = self.release_current_main_lease("map_restarted");
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
        _tool_name: &str,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(Vec::new());
        }

        self.validate_maintenance_barrier()?;
        let events = self.ensure_main_binding_for_active_map(owner_session_id)?;
        self.validate_main_binding(owner_session_id)?;
        Ok(events)
    }

    pub(crate) fn record_main_tool_result(
        &mut self,
        owner_session_id: ThreadId,
        call_id: &str,
        tool_name: &str,
        success: bool,
        preview: String,
    ) -> Result<Option<(NodeResultId, Vec<MapRuntimeEvent>)>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }

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
        let result_id = self.next_result_id();
        let body = format!(
            "Main tool call\n\
tool: {tool_name}\n\
call_id: {call_id}\n\
success: {success}\n\
preview:\n\
{preview}"
        );
        let map = self
            .maps
            .get_mut(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
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
            body,
            source_thread_id: owner_session_id,
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
                source_thread_id: owner_session_id,
            },
        )];
        let main_tool_result_count = count_node_results_of_kind(node, NodeResultKind::MainToolCall);
        if main_tool_result_count > MAIN_TOOL_RESULT_BUDGET_PER_NODE
            && !self.maintenance_barriers.contains_key(&map_id)
        {
            let barrier = ActionMapMaintenanceBarrier {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                reason: MaintenanceBarrierReason::NodeToolResultBudgetExceeded,
                result_count: main_tool_result_count,
                budget: MAIN_TOOL_RESULT_BUDGET_PER_NODE,
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
        let mut events = self.release_current_main_lease("main_rebound");
        events.extend(self.claim_main_node(owner_session_id, &map_id, node_id)?);
        events.extend(self.clear_maintenance_barrier_for_recovery(node_id));
        Ok(events)
    }

    pub(crate) fn create_node_for_main(
        &mut self,
        owner_session_id: ThreadId,
        title: String,
        context_summary: String,
        dependency_node_ids: Vec<String>,
        bind_current: bool,
    ) -> Result<(MapNodeId, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        let title = title.trim();
        let context_summary = context_summary.trim();
        if title.is_empty() {
            return Err("TaskSpace node title cannot be empty.".to_string());
        }
        if context_summary.is_empty() {
            return Err("TaskSpace node context summary cannot be empty.".to_string());
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
        let mut events = self.ensure_active_task_path(owner_session_id, title);
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let node_id = self.next_node_id();
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
                "TaskSpace cannot bind the main action to a new node until all dependencies are completed."
                    .to_string(),
            );
        }
        map.nodes.insert(
            node_id.clone(),
            MapNode {
                id: node_id.clone(),
                title: title.to_string(),
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

    pub(crate) fn start_task_for_main(
        &mut self,
        owner_session_id: ThreadId,
        task_title: String,
        task_objective: String,
        node_title: String,
        node_context_summary: String,
        bind_current: bool,
    ) -> Result<(TaskId, ActionMapId, MapNodeId, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
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

        let mut events = self.release_current_main_lease("task_started");
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

        let (node_id, mut node_events) = self.create_node_for_main(
            owner_session_id,
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
            return Ok(Vec::new());
        }

        let previous_task_id = self.active_task_id.clone();
        let previous_map_id = self.active_map_id.clone();
        let mut events = self.release_current_main_lease("task_routed");
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
        events.push(MapRuntimeEvent::TaskRouted(MapRuntimeTaskRoutedEvent {
            previous_task_id,
            current_task_id: task_id.to_string(),
            previous_map_id,
            current_map_id: target_map_id,
        }));
        Ok(events)
    }

    pub(crate) fn finish_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        result_summary: String,
        next_node_id: Option<String>,
    ) -> Result<(NodeResultId, Vec<MapRuntimeEvent>), String> {
        let result_summary = result_summary.trim();
        if result_summary.is_empty() {
            return Err("TaskSpace finish_node result_summary cannot be empty.".to_string());
        }
        let next_node_id = next_node_id
            .as_deref()
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        if let Some(next_node_id) = next_node_id {
            self.validate_next_main_binding_after_finish(node_id, next_node_id)?;
        }
        let (result_id, mut events) = self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Result,
            result_summary.to_string(),
            NodeStatus::Completed,
            true,
        )?;
        if let Some(next_node_id) = next_node_id {
            let bind_events = self.bind_main_node(owner_session_id, next_node_id)?;
            events.extend(bind_events);
        }
        Ok((result_id, events))
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

    pub(crate) fn prepare_spawn_assignment(
        &mut self,
        _owner_session_id: ThreadId,
        _requested_task_name: &str,
    ) -> Result<(Option<ActionMapAssignment>, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok((None, Vec::new()));
        }

        self.validate_maintenance_barrier()?;
        let mut events = Vec::new();
        let Some(map_id) = self.active_map_id.clone() else {
            return Err(
                "TaskSpace mode is active but no active task path exists. Call taskspace_control(action=start_task) for a new task or taskspace_control(action=route_task) for an existing task before spawning a subagent."
                    .to_string(),
            );
        };
        let Some(node_id) = self.next_ready_node_id(&map_id) else {
            return Err(
                "TaskSpace mode is active, but no ready node is available. Wait for running nodes to finish, ask the user for missing context, or reborn the task path with /task-reborn."
                    .to_string(),
            );
        };

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
                message_prefix: assignment_prompt(&map_id, &node_id, &node_title, &lease_id),
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
        let (kind, body) = result_from_status(status);
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
        let mut events = vec![
            MapRuntimeEvent::NodeResultRecorded(MapRuntimeNodeResultRecordedEvent {
                map_id: map_id.clone(),
                node_id: node_id.clone(),
                lease_id: lease_id.clone(),
                result_id: result_id.clone(),
                kind: kind.as_str().to_string(),
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
                && map.nodes.contains_key(node_id)
            {
                context.push_str("\n- current main action node: ");
                context.push_str(node_id);
            }
            context.push_str("\nNodes:\n");
            for node_id in ordered_node_ids(map) {
                if let Some(node) = map.nodes.get(&node_id) {
                    context.push_str("- ");
                    context.push_str(&node.id);
                    context.push_str(": ");
                    context.push_str(&node.title);
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
                "Every action must run on the active task path. Main-agent ordinary tool calls are attributed to the current main action node; subagent actions are bound to ready nodes at spawn time. If a newly discovered subtask does not fit existing nodes, call taskspace_control(action=create_node) before doing that work. Node result context stays on the node; use it only when it is relevant to the next step.\n",
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

    fn ensure_active_task_path(
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
            title_hint.trim().to_string()
        };
        let previous_task_id = self.active_task_id.clone();
        let task_id = self.ensure_active_task_state(Some(owner_session_id), &title);
        let mut map =
            ActionMapInstance::new(id.clone(), title, Some(owner_session_id), BASE_MAP.version);
        map.task_id = Some(task_id.clone());
        self.register_map_to_task(&task_id, &id);
        self.active_map_id = Some(id.clone());
        self.current_main_node_id = None;
        let mut events = Vec::new();
        if previous_task_id.as_deref() != Some(task_id.as_str())
            && let Some(task) = self.tasks.get(&task_id)
        {
            events.push(task_created_event(task));
        }
        events.push(map_created_event(&map));
        self.maps.insert(id, map);
        events
    }

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

    fn release_current_main_lease(&mut self, reason: &str) -> Vec<MapRuntimeEvent> {
        let Some(lease_id) = self.current_main_lease_id.clone() else {
            self.current_main_node_id = None;
            return Vec::new();
        };
        self.release_lease(&lease_id, reason)
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
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        let current_node_id = self.current_main_node_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no current node binding exists. Call taskspace_control(action=create_node, bind_current=true) or taskspace_control(action=bind_node) before finishing or blocking a node."
                .to_string()
        })?;
        if current_node_id != node_id {
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

    fn validate_maintenance_barrier(&self) -> Result<(), String> {
        let Some(barrier) = self.active_maintenance_barrier() else {
            return Ok(());
        };
        Err(format!(
            "TaskSpace maintenance barrier is active for node `{}` on map `{}`: {} ({} main tool results, budget {}). Ordinary tools and spawn_agent are blocked until you use taskspace_control to create or bind a different narrower node, or stop and ask the user to restart/reframe the task.",
            barrier.node_id,
            barrier.map_id,
            barrier.reason.as_str(),
            barrier.result_count,
            barrier.budget
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

    fn next_ready_node_id(&self, map_id: &str) -> Option<MapNodeId> {
        let map = self.maps.get(map_id)?;
        let ready_node = |node_id: &str| {
            map.nodes
                .get(node_id)
                .filter(|node| node.status == NodeStatus::Ready && node.active_lease.is_none())
                .map(|node| node.id.clone())
        };
        ordered_node_ids(map)
            .into_iter()
            .filter(|node_id| self.current_main_node_id.as_deref() != Some(node_id.as_str()))
            .find_map(|node_id| ready_node(&node_id))
            .or_else(|| self.current_main_node_id.as_deref().and_then(ready_node))
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
}

pub(crate) fn format_action_map_snapshot(snapshot: &ActionMapSnapshot) -> String {
    let mut output = String::new();
    output.push_str("TaskSpace\n");
    output.push_str("- mode: ");
    output.push_str(&snapshot.mode.to_string());
    output.push('\n');
    output.push_str("- active task: ");
    output.push_str(snapshot.active_task_id.as_deref().unwrap_or("none"));
    output.push('\n');
    output.push_str("- active map: ");
    output.push_str(snapshot.active_map_id.as_deref().unwrap_or("none"));
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

fn first_open_node_id(map: &ActionMapInstance) -> Option<MapNodeId> {
    first_node_with_status(map, NodeStatus::Ready)
}

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

fn snapshot_map(map: &ActionMapInstance) -> ActionMapSnapshotMap {
    let mut nodes = map
        .nodes
        .values()
        .map(|node| ActionMapSnapshotNode {
            id: node.id.clone(),
            title: node.title.clone(),
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

fn assignment_prompt(map_id: &str, node_id: &str, node_title: &str, lease_id: &str) -> String {
    format!(
        "TaskSpace node assignment\n\
Map: {map_id}\n\
Node: {node_id} - {node_title}\n\
Lease: {lease_id}\n\
\n\
You must work only on this node's subtask. Use the provided node context and return a concise, free-form result for this node. If you are blocked, explain the blocker clearly. Do not maintain the map directly. Do not call taskspace_control, spawn_agent, or wait_agent unless the user task explicitly requires nested delegation.\n\n"
    )
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

    fn seed_test_map(state: &mut ActionMapRuntimeState, owner: ThreadId) {
        state.ensure_active_seed_map(owner, "test");
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

        let (node_id, _) = state
            .create_node_for_main(
                owner,
                "Inspect logging".to_string(),
                "Understand logging before changing code.".to_string(),
                Vec::new(),
                true,
            )
            .expect("node created");
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
    fn main_held_node_is_not_claimable_by_subagent() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        let (node_id, _) = state
            .create_node_for_main(
                owner,
                "Main implementation".to_string(),
                "Main agent owns this node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("main node created");
        assert_eq!(node_id, "node-1");

        let error = state
            .prepare_spawn_assignment(owner, "parallel worker")
            .expect_err("main-held node is not claimable by subagents");

        assert!(error.contains("no ready node is available"));
        let map = state.active_map().expect("active map");
        let node = map.nodes.get("node-1").expect("node");
        assert_eq!(node.status, NodeStatus::Running);
        assert_eq!(node.active_lease.as_deref(), Some("lease-1"));
        assert_eq!(map.leases["lease-1"].holder, LeaseHolder::Main);
    }

    #[test]
    fn main_rebind_releases_previous_node_to_previous_status() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .create_node_for_main(
                owner,
                "First node".to_string(),
                "First main node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("first node");

        let (second_node_id, events) = state
            .create_node_for_main(
                owner,
                "Second node".to_string(),
                "Second main node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("second node");

        assert_eq!(second_node_id, "node-2");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::LeaseReleased(event)
                    if event.node_id == "node-1"
                        && event.lease_id == "lease-1"
                        && event.holder == "main"
                        && event.reason == "main_rebound"
            )
        }));
        let map = state.active_map().expect("active map");
        let first = map.nodes.get("node-1").expect("first node");
        assert_eq!(first.status, NodeStatus::Ready);
        assert!(first.active_lease.is_none());
        let second = map.nodes.get("node-2").expect("second node");
        assert_eq!(second.status, NodeStatus::Running);
        assert_eq!(second.active_lease.as_deref(), Some("lease-2"));
        assert_eq!(map.leases.len(), 1);
        assert_eq!(map.leases["lease-2"].holder, LeaseHolder::Main);
    }

    #[test]
    fn subagent_result_and_timeout_paths_ignore_main_lease() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        let child = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .create_node_for_main(
                owner,
                "Main implementation".to_string(),
                "Main agent owns this node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("main node created");

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
        let (node_id, _) = state
            .create_node_for_main(
                owner,
                "Inspect architecture".to_string(),
                "Broad inspection node used by the regression fixture.".to_string(),
                Vec::new(),
                true,
            )
            .expect("node created");

        for index in 0..MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("below budget");
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
            assert!(
                !events
                    .iter()
                    .any(|event| { matches!(event, MapRuntimeEvent::MaintenanceBarrierRaised(_)) }),
                "barrier should not raise before budget is exceeded"
            );
        }

        state
            .prepare_main_tool_call(owner, "shell")
            .expect("the call that exceeds the budget is allowed then recorded");
        let (_, events) = state
            .record_main_tool_result(owner, "call-over-budget", "shell", true, "ok".to_string())
            .expect("record succeeds")
            .expect("result recorded");

        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierRaised(event)
                    if event.node_id == node_id
                        && event.result_count == MAIN_TOOL_RESULT_BUDGET_PER_NODE + 1
                        && event.budget == MAIN_TOOL_RESULT_BUDGET_PER_NODE
            )
        }));
        let error = state
            .prepare_main_tool_call(owner, "shell")
            .expect_err("ordinary tools should be blocked by barrier");
        assert!(error.contains("maintenance barrier"));
        let spawn_error = state
            .prepare_spawn_assignment(owner, "parallel follow-up")
            .expect_err("spawn should be blocked by barrier");
        assert!(spawn_error.contains("maintenance barrier"));
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("Maintenance barrier"));
        assert!(context.contains("node_tool_result_budget_exceeded"));
    }

    #[test]
    fn maintenance_barrier_can_be_cleared_by_binding_recovery_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state
            .create_node_for_main(
                owner,
                "Broad implementation".to_string(),
                "A broad node that should be split after budget pressure.".to_string(),
                Vec::new(),
                true,
            )
            .expect("node created");
        for index in 0..=MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed before the barrier is recorded");
            state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds");
        }
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());
        let bind_error = state
            .bind_main_node(owner, "node-1")
            .expect_err("binding the same overgrown node should not clear the barrier");
        assert!(bind_error.contains("different narrower recovery node"));

        let (recovery_node_id, events) = state
            .create_node_for_main(
                owner,
                "Focused follow-up".to_string(),
                "Continue with a narrower recovery node.".to_string(),
                Vec::new(),
                true,
            )
            .expect("recovery node created");

        assert_eq!(recovery_node_id, "node-2");
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::MaintenanceBarrierCleared(event)
                    if event.node_id == "node-1" && event.reason == "bound_recovery_node"
            )
        }));
        state
            .prepare_main_tool_call(owner, "shell")
            .expect("recovery node should be allowed");
        assert_eq!(state.current_main_node_id.as_deref(), Some("node-2"));
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
        for index in 0..=MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed before barrier is recorded");
            state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds");
        }
        assert!(state.prepare_main_tool_call(owner, "shell").is_err());

        state
            .route_task_for_main(owner, "task-2")
            .expect("route away from task with barrier");
        state
            .prepare_main_tool_call(owner, "shell")
            .expect("other task should not be blocked by old task barrier");
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
        for index in 0..=MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed before barrier is recorded");
            state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds");
        }

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
        for index in 0..=MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed before barrier is recorded");
            state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds");
        }
        let snapshot = state.snapshot();
        assert_eq!(snapshot.maintenance_barriers.len(), 1);

        let mut restored = ActionMapRuntimeState::default();
        restored.restore_snapshot(snapshot.clone());

        assert_eq!(restored.snapshot(), snapshot);
        let error = restored
            .prepare_main_tool_call(owner, "shell")
            .expect_err("restored barrier should block ordinary work");
        assert!(error.contains("maintenance barrier"));
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
        state
            .create_node_for_main(
                owner,
                "Broad task".to_string(),
                "A node that has become too broad.".to_string(),
                Vec::new(),
                true,
            )
            .expect("node created");
        for index in 0..=MAIN_TOOL_RESULT_BUDGET_PER_NODE {
            state
                .prepare_main_tool_call(owner, "shell")
                .expect("main tool call should be allowed before the barrier is recorded");
            state
                .record_main_tool_result(
                    owner,
                    &format!("call-{index}"),
                    "shell",
                    true,
                    "ok".to_string(),
                )
                .expect("record succeeds");
        }
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
    fn control_can_create_and_bind_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);

        let (node_id, events) = state
            .create_node_for_main(
                owner,
                "Review logging".to_string(),
                "Check logging coverage before implementation.".to_string(),
                Vec::new(),
                true,
            )
            .expect("node created");

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
    fn control_rejects_first_node_with_dependencies_without_creating_map() {
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
            .expect_err("first node cannot depend on missing upstream nodes");

        assert!(error.contains("first node with dependencies"));
        assert!(state.maps.is_empty());
        assert!(state.active_map_id.is_none());
    }

    #[test]
    fn control_rejects_binding_pending_dependency_node() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state.ensure_active_seed_map(owner, "test");

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
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
        state
            .bind_main_node(owner, "define_scope")
            .expect("bind main node");

        let (result_id, events) = state
            .finish_main_node(
                owner,
                "define_scope",
                "Scope is clear enough to inspect code context.".to_string(),
                Some("inspect_code_context".to_string()),
            )
            .expect("node finished");

        assert_eq!(result_id, "result-1");
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
        let result = map.results.get(&result_id).expect("stored result");
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
    fn block_main_node_records_blocker_without_unlocking_downstream() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();
        state.set_mode(MapRuntimeMode::Experiment);
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
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
        state.ensure_active_seed_map(owner, "test");
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
        assert!(!context.contains("BaseMap metadata version"));
    }

    #[test]
    fn spawn_assignment_claims_one_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);

        let (assignment, events) = state
            .prepare_spawn_assignment(owner, "implement maps")
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

        let (node_id, _) = state
            .create_node_for_main(
                owner,
                "Parallel review".to_string(),
                "Review a side task while main work stays on define_scope.".to_string(),
                Vec::new(),
                false,
            )
            .expect("node created");
        assert_eq!(node_id, "node-1");
        assert!(state.current_main_node_id.is_none());
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("- node-1: Parallel review [ready]"));

        let (assignment, _) = state
            .prepare_spawn_assignment(owner, "parallel review")
            .expect("dynamic assignment succeeds");
        let assignment = assignment.expect("experiment assignment");

        assert_eq!(assignment.node_id, "node-1");
        assert_eq!(assignment.node_title, "Parallel review");
    }

    #[test]
    fn standard_mode_spawn_assignment_is_disabled() {
        let mut state = ActionMapRuntimeState::default();
        let owner = ThreadId::new();

        let assignment = state
            .prepare_spawn_assignment(owner, "standard task")
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
            .prepare_spawn_assignment(owner, "standard task")
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
            .prepare_spawn_assignment(owner, "first")
            .expect("first claim succeeds")
            .0
            .expect("first assignment");

        let second = state
            .prepare_spawn_assignment(owner, "second")
            .expect_err("no second node should be ready while the first is running");
        assert!(second.contains("no ready node is available"));

        state.release_lease(&first.lease_id, "test_release");
        let reclaimed = state
            .prepare_spawn_assignment(owner, "second")
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
            .prepare_spawn_assignment(owner, "first")
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
            .prepare_spawn_assignment(owner, "implement maps")
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
            .prepare_spawn_assignment(owner, "inspect runtime")
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

        state
            .create_node_for_main(
                first_owner,
                "Initial task".to_string(),
                "Initial owner task.".to_string(),
                Vec::new(),
                true,
            )
            .expect("initial node");

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

        state
            .prepare_main_tool_call(owner, "shell")
            .expect("ordinary work should auto-bind the routed task's ready node");
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
            .prepare_spawn_assignment(owner, "first")
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
            .prepare_spawn_assignment(owner, "first")
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
            .prepare_spawn_assignment(owner, "first")
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
            .prepare_spawn_assignment(owner, "first")
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
                .prepare_spawn_assignment(owner, expected_node)
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
            state.record_child_result(
                child,
                &AgentStatus::Completed(Some(format!("{expected_node} done"))),
            );
        }

        let map_id = map_id.expect("map id");
        let map = state.maps.get(&map_id).expect("map");
        assert_eq!(map.status, MapStatus::Active);
        assert_eq!(state.active_map().expect("active").id, map_id);
        assert_eq!(map.results.len(), SEED_NODE_IDS.len());
    }

    #[test]
    fn restart_abandons_previous_map_and_creates_empty_task_path() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        seed_test_map(&mut state, owner);
        let first = state
            .prepare_spawn_assignment(owner, "first")
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
