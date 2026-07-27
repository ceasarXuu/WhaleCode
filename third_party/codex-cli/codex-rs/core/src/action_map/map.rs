#![allow(dead_code)]

use std::collections::HashMap;
use std::iter;
use std::ops::Deref;

use codex_protocol::ThreadId;

use super::rooted_dag::EventBatch;
pub(crate) use super::rooted_dag::MapEdge;
pub(crate) use super::rooted_dag::MapNode;
pub(crate) use super::rooted_dag::NodeRole;
pub(crate) use super::rooted_dag::NodeState;
use super::rooted_dag::TaskSpaceMap;
use super::rooted_dag::derive_node_state;
use super::rooted_dag::derive_node_views;
use super::rooted_dag::is_complete;
use super::rooted_dag::node;
use super::rooted_dag::node_role;

pub(crate) type ActionMapId = String;
pub(crate) type MapNodeId = String;
pub(crate) type NodeEventId = String;
pub(crate) type TaskId = String;
pub(crate) type TaskSpaceTraceEventId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskRecord {
    pub(crate) owner_session_id: Option<ThreadId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionClass {
    Read,
    Search,
    Edit,
    Build,
    Test,
    Spawn,
    Wait,
    Review,
    FinalResponse,
    Control,
    Unknown,
}

impl ActionClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ActionClass::Read => "read",
            ActionClass::Search => "search",
            ActionClass::Edit => "edit",
            ActionClass::Build => "build",
            ActionClass::Test => "test",
            ActionClass::Spawn => "spawn",
            ActionClass::Wait => "wait",
            ActionClass::Review => "review",
            ActionClass::FinalResponse => "final_response",
            ActionClass::Control => "control",
            ActionClass::Unknown => "unknown",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        let normalized = normalize_contract_name(value);
        match normalized.as_str() {
            "read" => Some(ActionClass::Read),
            "search" => Some(ActionClass::Search),
            "edit" => Some(ActionClass::Edit),
            "build" => Some(ActionClass::Build),
            "test" => Some(ActionClass::Test),
            "spawn" => Some(ActionClass::Spawn),
            "wait" => Some(ActionClass::Wait),
            "review" => Some(ActionClass::Review),
            "final_response" | "finalresponse" => Some(ActionClass::FinalResponse),
            "control" => Some(ActionClass::Control),
            "unknown" => Some(ActionClass::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolActionDescriptor {
    pub(crate) tool_name: String,
    pub(crate) call_id: Option<String>,
    pub(crate) action_class: ActionClass,
    pub(crate) preview: String,
}

impl ToolActionDescriptor {
    pub(crate) fn new(
        tool_name: impl Into<String>,
        action_class: ActionClass,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            call_id: None,
            action_class,
            preview: preview.into(),
        }
    }

    pub(crate) fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }
}

impl From<&str> for ToolActionDescriptor {
    fn from(tool_name: &str) -> Self {
        Self::new(tool_name, ActionClass::Read, "")
    }
}

impl From<String> for ToolActionDescriptor {
    fn from(tool_name: String) -> Self {
        Self::new(tool_name, ActionClass::Read, "")
    }
}

fn normalize_contract_name(value: &str) -> String {
    value.trim().replace(['-', ' '], "_").to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMapInstance {
    graph: TaskSpaceMap,
    pub(crate) graph_events: Vec<EventBatch>,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) owner_session_id: Option<ThreadId>,
    pub(crate) node_events: HashMap<NodeEventId, NodeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeEvent {
    pub(crate) id: NodeEventId,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) event_kind: String,
    pub(crate) source: String,
    pub(crate) action_class: Option<ActionClass>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) content_sha256: String,
    pub(crate) source_event_id: Option<String>,
    pub(crate) raw_ref: Option<String>,
    pub(crate) artifact_refs: Vec<String>,
    pub(crate) call_id: Option<String>,
    pub(crate) source_thread_id: ThreadId,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceTraceEvent {
    pub(crate) id: TaskSpaceTraceEventId,
    pub(crate) kind: String,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) map_id: ActionMapId,
    pub(crate) node_id: MapNodeId,
    pub(crate) result_id: Option<String>,
    pub(crate) call_id: Option<String>,
    pub(crate) action_class: Option<ActionClass>,
    pub(crate) tool_success: Option<bool>,
    pub(crate) tags: Vec<String>,
    pub(crate) artifact_refs: Vec<String>,
    pub(crate) created_at_ms: i64,
}

impl ActionMapInstance {
    pub(crate) fn from_graph(
        graph: TaskSpaceMap,
        graph_events: Vec<EventBatch>,
        owner_session_id: Option<ThreadId>,
    ) -> Self {
        Self {
            graph,
            graph_events,
            task_id: None,
            owner_session_id,
            node_events: HashMap::new(),
        }
    }

    pub(crate) fn canonical_map(&self) -> &TaskSpaceMap {
        &self.graph
    }

    pub(crate) fn node(&self, node_id: &str) -> Option<&MapNode> {
        node(&self.graph, node_id)
    }

    pub(crate) fn all_nodes(&self) -> impl Iterator<Item = (NodeRole, &MapNode)> {
        iter::once((NodeRole::TaskRoot, &self.graph.root))
            .chain(
                self.graph
                    .work_nodes
                    .iter()
                    .map(|work_node| (NodeRole::Work, work_node)),
            )
            .chain(iter::once((NodeRole::Finish, &self.graph.finish)))
    }

    pub(crate) fn node_role(&self, node_id: &str) -> Option<NodeRole> {
        node_role(&self.graph, node_id)
    }

    pub(crate) fn node_state(&self, node_id: &str) -> Option<NodeState> {
        derive_node_state(&self.graph, node_id)
    }

    pub(crate) fn node_views(&self) -> Vec<codex_protocol::taskspace::TaskSpaceNodeView> {
        derive_node_views(&self.graph)
    }

    pub(crate) fn is_complete(&self) -> bool {
        is_complete(&self.graph)
    }

    pub(crate) fn ready_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::Ready)
    }

    pub(crate) fn inflight_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::InFlight)
    }

    pub(crate) fn completed_work_node_count(&self) -> usize {
        self.count_work_nodes_in_state(NodeState::Completed)
    }

    pub(crate) fn finish_ready(&self) -> bool {
        self.node_state(&self.graph.finish.node_id) == Some(NodeState::Ready)
    }

    pub(crate) fn result_ids_for_node(&self, node_id: &str) -> Vec<String> {
        self.graph
            .result_refs
            .iter()
            .filter(|(_, result)| result.node_id == node_id)
            .map(|(result_id, _)| result_id.clone())
            .collect()
    }

    pub(crate) fn evidence_ids_for_node(&self, node_id: &str) -> Vec<String> {
        self.graph
            .evidence_refs
            .iter()
            .filter(|(_, evidence)| evidence.node_id == node_id)
            .map(|(evidence_id, _)| evidence_id.clone())
            .collect()
    }

    pub(crate) fn event_ids_for_node(&self, node_id: &str) -> Vec<String> {
        let mut event_ids = self
            .node_events
            .values()
            .filter(|event| event.node_id == node_id)
            .map(|event| event.id.clone())
            .collect::<Vec<_>>();
        event_ids.sort();
        event_ids
    }

    pub(crate) fn commit_graph(&mut self, graph: TaskSpaceMap, events: EventBatch) {
        self.graph = graph;
        self.graph_events.push(events);
    }

    fn count_work_nodes_in_state(&self, state: NodeState) -> usize {
        self.graph
            .work_nodes
            .iter()
            .filter(|node| derive_node_state(&self.graph, &node.node_id) == Some(state))
            .count()
    }
}

pub(crate) fn node_state_name(state: NodeState) -> &'static str {
    match state {
        NodeState::Waiting => "waiting",
        NodeState::Ready => "ready",
        NodeState::InFlight => "in_flight",
        NodeState::Blocked => "blocked",
        NodeState::Completed => "completed",
    }
}

impl Deref for ActionMapInstance {
    type Target = TaskSpaceMap;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::rooted_dag::ActionReservation;
    use crate::action_map::rooted_dag::CompletionRecord;
    use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
    use std::collections::BTreeMap;

    fn forked_map() -> TaskSpaceMap {
        TaskSpaceMap {
            schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
            map_id: "map-1".into(),
            root: MapNode {
                node_id: "root".into(),
                goal: "deliver".into(),
                source_refs: vec!["user-turn-1".into()],
            },
            work_nodes: vec![
                MapNode {
                    node_id: "left".into(),
                    goal: "inspect".into(),
                    source_refs: vec![],
                },
                MapNode {
                    node_id: "right".into(),
                    goal: "test".into(),
                    source_refs: vec![],
                },
            ],
            finish: MapNode {
                node_id: "finish".into(),
                goal: "summarize".into(),
                source_refs: vec![],
            },
            edges: vec![
                MapEdge {
                    from: "root".into(),
                    to: "left".into(),
                },
                MapEdge {
                    from: "root".into(),
                    to: "right".into(),
                },
                MapEdge {
                    from: "left".into(),
                    to: "finish".into(),
                },
                MapEdge {
                    from: "right".into(),
                    to: "finish".into(),
                },
            ],
            completion_records: BTreeMap::new(),
            block_records: BTreeMap::new(),
            action_reservations: BTreeMap::new(),
            result_refs: BTreeMap::new(),
            evidence_refs: BTreeMap::new(),
            terminal_record: None,
            revision: 1,
        }
    }

    #[test]
    fn exposes_nodes_and_counts_only_from_canonical_facts() {
        let mut graph = forked_map();
        graph.completion_records.insert(
            "left".into(),
            CompletionRecord {
                action_id: "complete-left".into(),
                result_ref_ids: vec![],
                evidence_ref_ids: vec![],
            },
        );
        graph.action_reservations.insert(
            "reservation-right".into(),
            ActionReservation {
                action_id: "action-right".into(),
                node_id: "right".into(),
                tool_name: "exec_command".into(),
                response_call_index: 1,
            },
        );
        let map = ActionMapInstance::from_graph(graph, vec![], None);

        assert_eq!(
            map.node("left").map(|node| node.goal.as_str()),
            Some("inspect")
        );
        assert_eq!(map.all_nodes().count(), 4);
        assert_eq!(map.node_state("left"), Some(NodeState::Completed));
        assert_eq!(map.node_state("right"), Some(NodeState::InFlight));
        assert_eq!(map.ready_work_node_count(), 0);
        assert_eq!(map.inflight_work_node_count(), 1);
        assert_eq!(map.completed_work_node_count(), 1);
        assert!(!map.finish_ready());
    }

    #[test]
    fn multiple_ready_nodes_exist_without_a_current_node() {
        let map = ActionMapInstance::from_graph(forked_map(), vec![], None);

        assert_eq!(map.ready_work_node_count(), 2);
        assert_eq!(map.inflight_work_node_count(), 0);
        assert_eq!(
            map.node_views()
                .into_iter()
                .filter(|view| view.state == NodeState::Ready)
                .map(|view| view.node_id)
                .collect::<Vec<_>>(),
            vec!["left".to_string(), "right".to_string(), "root".to_string()]
        );
    }
}
