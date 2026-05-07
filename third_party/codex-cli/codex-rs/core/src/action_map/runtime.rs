use std::collections::HashMap;
use std::collections::HashSet;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeEvent;
use codex_protocol::protocol::MapRuntimeLeaseAttachedEvent;
use codex_protocol::protocol::MapRuntimeLeaseCreatedEvent;
use codex_protocol::protocol::MapRuntimeLeaseReleasedEvent;
use codex_protocol::protocol::MapRuntimeMapCreatedEvent;
use codex_protocol::protocol::MapRuntimeMapStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeMode;
use codex_protocol::protocol::MapRuntimeNodeResultRecordedEvent;
use codex_protocol::protocol::MapRuntimeNodeStatusChangedEvent;
use codex_protocol::protocol::MapRuntimeTimeoutSummaryRequestedEvent;

use super::basemap::BASE_MAP;
use super::basemap::base_map_metadata_prompt;
use super::map::ActionMapId;
use super::map::ActionMapInstance;
use super::map::AssignmentLease;
use super::map::AssignmentLeaseId;
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

const SEED_NODE_IDS: &[&str] = &[
    "define_scope",
    "inspect_code_context",
    "design_solution",
    "implement_solution",
    "smoke_test",
    "final_synthesis",
];

#[derive(Debug, Clone)]
pub(crate) struct ActionMapRuntimeState {
    mode: MapRuntimeMode,
    pending_transition_notice: Option<String>,
    active_map_id: Option<ActionMapId>,
    maps: HashMap<ActionMapId, ActionMapInstance>,
    next_map_seq: u64,
    next_lease_seq: u64,
    next_result_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SetMapRuntimeModeOutcome {
    pub(crate) previous_mode: MapRuntimeMode,
    pub(crate) current_mode: MapRuntimeMode,
    pub(crate) changed: bool,
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
            active_map_id: None,
            maps: HashMap::new(),
            next_map_seq: 1,
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

    pub(crate) fn restore_mode(&mut self, mode: MapRuntimeMode) {
        self.mode = mode;
        self.pending_transition_notice = None;
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
        let map = seed_map(id.clone(), title, owner_session_id, None);
        self.active_map_id = Some(id.clone());
        self.maps.insert(id.clone(), map);
        id
    }

    pub(crate) fn active_map(&self) -> Option<&ActionMapInstance> {
        self.active_map_id
            .as_ref()
            .and_then(|map_id| self.maps.get(map_id))
            .filter(|map| map.status == MapStatus::Active)
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

        let new_id = self.next_map_id();
        let map = seed_map(
            new_id.clone(),
            title.into(),
            Some(owner_session_id),
            previous_id.clone(),
        );
        self.active_map_id = Some(new_id.clone());
        events.push(map_created_event(&map));
        events.extend(initial_node_events(&map));
        self.maps.insert(new_id.clone(), map);
        (previous_id, new_id, events)
    }

    pub(crate) fn prepare_spawn_assignment(
        &mut self,
        owner_session_id: ThreadId,
        requested_task_name: &str,
    ) -> Result<(Option<ActionMapAssignment>, Vec<MapRuntimeEvent>), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok((None, Vec::new()));
        }

        let mut events = self.ensure_active_seed_map(owner_session_id, requested_task_name);
        let Some(map_id) = self.active_map_id.clone() else {
            return Err(
                "Action Map experiment mode is active but no active map exists.".to_string(),
            );
        };
        let Some(node_id) = self.next_ready_node_id(&map_id) else {
            return Err(
                "Action Map experiment mode is active, but no ready node is available. Wait for running nodes to finish, ask the user for missing context, or restart the map with /map-restart."
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
            let mut events = vec![MapRuntimeEvent::LeaseReleased(
                MapRuntimeLeaseReleasedEvent {
                    map_id: lease.map_id.clone(),
                    node_id: lease.node_id.clone(),
                    lease_id: lease.id.clone(),
                    reason,
                },
            )];
            if let Some(node) = map.nodes.get_mut(&lease.node_id)
                && node.active_lease.as_deref() == Some(lease_id)
            {
                node.active_lease = None;
                if node.status == NodeStatus::Running {
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
        if map
            .nodes
            .values()
            .all(|node| node.status == NodeStatus::Completed)
        {
            let previous_status = map.status;
            map.status = MapStatus::Completed;
            events.push(map_status_changed_event(map, previous_status));
        }
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

        let mut context = String::from("Action Map experiment mode is active.\n");
        if let Some(map) = self.active_map() {
            context.push_str("Active Action Map:\n");
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
            context.push_str("\nNodes:\n");
            for node_id in SEED_NODE_IDS {
                if let Some(node) = map.nodes.get(*node_id) {
                    context.push_str("- ");
                    context.push_str(&node.id);
                    context.push_str(": ");
                    context.push_str(&node.title);
                    context.push_str(" [");
                    context.push_str(node.status.as_str());
                    context.push_str("]\n");
                }
            }
            context.push_str(
                "Subagent actions are bound to ready nodes at spawn time. Node result context stays on the node; use it only when it is relevant to the next step.\n",
            );
        } else {
            context.push_str(
                "No active Action Map exists. Before taking multi-agent action, create or bind an Action Map and a ready node.\n",
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
            "Action Map".to_string()
        } else {
            format!("Action Map: {}", title_hint.trim())
        };
        let map = seed_map(id.clone(), title, Some(owner_session_id), None);
        self.active_map_id = Some(id.clone());
        let events = {
            let mut events = vec![map_created_event(&map)];
            events.extend(initial_node_events(&map));
            events
        };
        self.maps.insert(id, map);
        events
    }

    fn next_ready_node_id(&self, map_id: &str) -> Option<MapNodeId> {
        let map = self.maps.get(map_id)?;
        SEED_NODE_IDS.iter().find_map(|node_id| {
            map.nodes
                .get(*node_id)
                .filter(|node| node.status == NodeStatus::Ready && node.active_lease.is_none())
                .map(|node| node.id.clone())
        })
    }

    fn find_lease_by_thread(
        &self,
        child_thread_id: ThreadId,
    ) -> Option<(ActionMapId, AssignmentLeaseId, MapNodeId)> {
        self.maps.iter().find_map(|(map_id, map)| {
            map.leases.iter().find_map(|(lease_id, lease)| {
                (lease.agent_thread_id == Some(child_thread_id))
                    .then(|| (map_id.clone(), lease_id.clone(), lease.node_id.clone()))
            })
        })
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

    fn next_result_id(&mut self) -> NodeResultId {
        let id = format!("result-{}", self.next_result_seq);
        self.next_result_seq += 1;
        id
    }
}

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

fn map_created_event(map: &ActionMapInstance) -> MapRuntimeEvent {
    MapRuntimeEvent::MapCreated(MapRuntimeMapCreatedEvent {
        map_id: map.id.clone(),
        title: map.title.clone(),
        owner_session_id: map.owner_session_id,
        created_from: map.created_from.clone(),
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
        "Action Map node assignment\n\
Map: {map_id}\n\
Node: {node_id} - {node_title}\n\
Lease: {lease_id}\n\
\n\
You must work only on this node's subtask. Use the provided node context and return a concise, free-form result for this node. If you are blocked, explain the blocker clearly. Do not maintain the map directly.\n\n"
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
            "Action Map experiment mode is now active.\n\
Previous standard-mode conversation remains background context only.\n\
Before taking multi-agent action, create or bind an Action Map and a ready node.\n\
Future subagent work must be map/node driven."
                .to_string()
        }
        (MapRuntimeMode::Experiment, MapRuntimeMode::Standard) => {
            "Action Map experiment mode is now disabled.\n\
Existing maps, nodes, leases, and results remain historical context only.\n\
Do not continue maintaining the map, require node binding, or follow map-driven protocol unless the user enables experiment mode again.\n\
Continue with the standard Codex multi-agent behavior."
                .to_string()
        }
        _ => {
            format!("Action Map runtime mode changed from {previous_mode} to {current_mode}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                .contains("experiment mode is now active")
        );

        let unchanged = state.set_mode(MapRuntimeMode::Experiment);
        assert_eq!(unchanged.previous_mode, MapRuntimeMode::Experiment);
        assert_eq!(unchanged.current_mode, MapRuntimeMode::Experiment);
        assert!(!unchanged.changed);
        assert!(state.take_pending_transition_notice().is_none());
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
        assert!(context.contains("Action Map experiment mode is active"));
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
        assert!(context.contains("Active Action Map"));
        assert!(context.contains("Investigate runtime"));
        assert!(!context.contains("BaseMap metadata version"));
    }

    #[test]
    fn spawn_assignment_claims_one_ready_node() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let (assignment, events) = state
            .prepare_spawn_assignment(owner, "implement maps")
            .expect("assignment succeeds");
        let assignment = assignment.expect("experiment assignment");

        assert_eq!(assignment.node_id, "define_scope");
        assert!(matches!(events[0], MapRuntimeEvent::MapCreated(_)));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MapRuntimeEvent::NodeStatusChanged(event)
                    if event.node_id == "define_scope"
                        && event.previous_status == "pending"
                        && event.current_status == "ready"
            )
        }));
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
                    if event.node_id == "define_scope" && event.lease_id == "lease-1"
            )
        }));
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("define_scope:"));
        assert!(context.contains("[running]"));
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
    fn running_node_blocks_second_claim_until_lease_is_released() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
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
    fn errored_result_blocks_node_and_does_not_unlock_downstream() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
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
    fn unknown_child_result_does_not_mutate_map() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
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
    fn completing_all_seed_nodes_marks_map_completed() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
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
        assert_eq!(map.status, MapStatus::Completed);
        assert!(state.active_map().is_none());
        assert_eq!(map.results.len(), SEED_NODE_IDS.len());
    }

    #[test]
    fn restart_abandons_previous_map_and_creates_new_seed() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
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
        assert_eq!(state.active_map().expect("active").id, next);
    }

    #[test]
    fn restart_without_existing_map_creates_seed_map() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();

        let (previous, next, _) = state.restart_active_map(owner, "Fresh map");

        assert!(previous.is_none());
        let map = state.active_map().expect("active map");
        assert_eq!(map.id, next);
        assert_eq!(map.title, "Fresh map");
        assert_eq!(
            map.nodes.get("define_scope").expect("node").status,
            NodeStatus::Ready
        );
    }
}
