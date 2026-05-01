use std::collections::HashMap;
use std::collections::HashSet;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::AgentStatus;
use codex_protocol::protocol::MapRuntimeMode;

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
    ) -> (Option<ActionMapId>, ActionMapId) {
        let previous_id = self.active_map_id.clone();
        if let Some(previous_id) = previous_id.as_ref()
            && let Some(map) = self.maps.get_mut(previous_id)
        {
            map.status = MapStatus::Abandoned;
        }

        let new_id = self.next_map_id();
        let map = seed_map(
            new_id.clone(),
            title.into(),
            Some(owner_session_id),
            previous_id.clone(),
        );
        self.active_map_id = Some(new_id.clone());
        self.maps.insert(new_id.clone(), map);
        (previous_id, new_id)
    }

    pub(crate) fn prepare_spawn_assignment(
        &mut self,
        owner_session_id: ThreadId,
        requested_task_name: &str,
    ) -> Result<Option<ActionMapAssignment>, String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Ok(None);
        }

        self.ensure_active_seed_map(owner_session_id, requested_task_name);
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

        Ok(Some(ActionMapAssignment {
            message_prefix: assignment_prompt(&map_id, &node_id, &node_title, &lease_id),
            map_id,
            node_id,
            node_title,
            lease_id,
        }))
    }

    pub(crate) fn attach_agent_to_lease(
        &mut self,
        lease_id: &str,
        thread_id: ThreadId,
        agent_path: Option<String>,
    ) {
        for map in self.maps.values_mut() {
            let Some(lease) = map.leases.get_mut(lease_id) else {
                continue;
            };
            lease.agent_thread_id = Some(thread_id);
            lease.agent_path = agent_path;
            return;
        }
    }

    pub(crate) fn release_lease(&mut self, lease_id: &str) {
        for map in self.maps.values_mut() {
            let Some(lease) = map.leases.remove(lease_id) else {
                continue;
            };
            if let Some(node) = map.nodes.get_mut(&lease.node_id)
                && node.active_lease.as_deref() == Some(lease_id)
            {
                node.active_lease = None;
                if node.status == NodeStatus::Running {
                    node.status = NodeStatus::Ready;
                }
            }
            return;
        }
    }

    pub(crate) fn release_lease_for_thread(
        &mut self,
        child_thread_id: ThreadId,
    ) -> Option<AssignmentLeaseId> {
        let (_, lease_id, _) = self.find_lease_by_thread(child_thread_id)?;
        self.release_lease(&lease_id);
        Some(lease_id)
    }

    pub(crate) fn record_child_result(
        &mut self,
        child_thread_id: ThreadId,
        status: &AgentStatus,
    ) -> Option<NodeResultId> {
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
        node.active_lease = None;
        node.status = match kind {
            NodeResultKind::Result | NodeResultKind::MapUpdateRequest => NodeStatus::Completed,
            NodeResultKind::Blocker | NodeResultKind::TimeoutSummary => NodeStatus::Blocked,
        };
        map.leases.remove(&lease_id);
        refresh_ready_nodes(map);
        if map
            .nodes
            .values()
            .all(|node| node.status == NodeStatus::Completed)
        {
            map.status = MapStatus::Completed;
        }
        Some(result_id)
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

    fn ensure_active_seed_map(&mut self, owner_session_id: ThreadId, title_hint: &str) {
        if self.active_map().is_some() {
            return;
        }
        let id = self.next_map_id();
        let title = if title_hint.trim().is_empty() {
            "Action Map".to_string()
        } else {
            format!("Action Map: {}", title_hint.trim())
        };
        let map = seed_map(id.clone(), title, Some(owner_session_id), None);
        self.active_map_id = Some(id.clone());
        self.maps.insert(id, map);
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

fn refresh_ready_nodes(map: &mut ActionMapInstance) {
    if map.status != MapStatus::Active {
        return;
    }
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
            node.status = NodeStatus::Ready;
        }
    }
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

        let assignment = state
            .prepare_spawn_assignment(owner, "implement maps")
            .expect("assignment succeeds")
            .expect("experiment assignment");

        assert_eq!(assignment.node_id, "define_scope");
        let context = state.build_developer_context().expect("context");
        assert!(context.contains("define_scope:"));
        assert!(context.contains("[running]"));
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
            .expect("experiment assignment");
        state.attach_agent_to_lease(&assignment.lease_id, child, Some("/define".to_string()));

        let result = state.record_child_result(
            child,
            &AgentStatus::Completed(Some("scope is clear".to_string())),
        );

        assert_eq!(result.as_deref(), Some("result-1"));
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
    fn restart_abandons_previous_map_and_creates_new_seed() {
        let mut state = ActionMapRuntimeState::default();
        state.set_mode(MapRuntimeMode::Experiment);
        let owner = ThreadId::new();
        let first = state
            .prepare_spawn_assignment(owner, "first")
            .expect("assignment")
            .expect("experiment")
            .map_id;

        let (previous, next) = state.restart_active_map(owner, "Restarted map");

        assert_eq!(previous.as_deref(), Some(first.as_str()));
        assert_ne!(first, next);
        assert_eq!(
            state.maps.get(&first).expect("previous").status,
            MapStatus::Abandoned
        );
        assert_eq!(state.active_map().expect("active").id, next);
    }
}
