use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub const TASKSPACE_CANONICAL_SCHEMA_VERSION: &str = "taskspace-canonical-map-v2";

pub type TaskSpaceActionId = String;
pub type TaskSpaceEvidenceRefId = String;
pub type TaskSpaceMapId = String;
pub type TaskSpaceNodeId = String;
pub type TaskSpaceReservationId = String;
pub type TaskSpaceResultRefId = String;
pub type TaskSpaceRevision = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceMapNode {
    pub node_id: TaskSpaceNodeId,
    pub goal: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceMapEdge {
    pub from: TaskSpaceNodeId,
    pub to: TaskSpaceNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceCompletionRecord {
    pub action_id: TaskSpaceActionId,
    pub result_ref_ids: Vec<TaskSpaceResultRefId>,
    pub evidence_ref_ids: Vec<TaskSpaceEvidenceRefId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceBlockRecord {
    pub action_id: TaskSpaceActionId,
    pub reason_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceActionReservation {
    pub action_id: TaskSpaceActionId,
    pub node_id: TaskSpaceNodeId,
    pub tool_name: String,
    pub response_call_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceResultRef {
    pub node_id: TaskSpaceNodeId,
    pub action_id: TaskSpaceActionId,
    pub reservation_id: TaskSpaceReservationId,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceEvidenceRef {
    pub node_id: TaskSpaceNodeId,
    pub action_id: TaskSpaceActionId,
    pub reservation_id: TaskSpaceReservationId,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceTerminalRecord {
    pub action_id: TaskSpaceActionId,
    pub summary_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceCanonicalMap {
    pub schema_version: String,
    pub map_id: TaskSpaceMapId,
    pub root: TaskSpaceMapNode,
    pub work_nodes: Vec<TaskSpaceMapNode>,
    pub finish: TaskSpaceMapNode,
    pub edges: Vec<TaskSpaceMapEdge>,
    pub completion_records: BTreeMap<TaskSpaceNodeId, TaskSpaceCompletionRecord>,
    pub block_records: BTreeMap<TaskSpaceNodeId, TaskSpaceBlockRecord>,
    pub action_reservations: BTreeMap<TaskSpaceReservationId, TaskSpaceActionReservation>,
    pub result_refs: BTreeMap<TaskSpaceResultRefId, TaskSpaceResultRef>,
    pub evidence_refs: BTreeMap<TaskSpaceEvidenceRefId, TaskSpaceEvidenceRef>,
    pub terminal_record: Option<TaskSpaceTerminalRecord>,
    pub terminal_history: Vec<TaskSpaceTerminalRecord>,
    pub revision: TaskSpaceRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSpaceNodeState {
    Waiting,
    Ready,
    InFlight,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceNodeView {
    pub node_id: TaskSpaceNodeId,
    pub state: TaskSpaceNodeState,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TaskSpaceCanonicalMap {
        TaskSpaceCanonicalMap {
            schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
            map_id: "map-1".into(),
            root: TaskSpaceMapNode {
                node_id: "root".into(),
                goal: "deliver".into(),
                source_refs: vec!["user-turn-1".into()],
            },
            work_nodes: vec![TaskSpaceMapNode {
                node_id: "work".into(),
                goal: "implement".into(),
                source_refs: vec![],
            }],
            finish: TaskSpaceMapNode {
                node_id: "finish".into(),
                goal: "close the task".into(),
                source_refs: vec![],
            },
            edges: vec![
                TaskSpaceMapEdge {
                    from: "root".into(),
                    to: "work".into(),
                },
                TaskSpaceMapEdge {
                    from: "work".into(),
                    to: "finish".into(),
                },
            ],
            completion_records: BTreeMap::new(),
            block_records: BTreeMap::new(),
            action_reservations: BTreeMap::new(),
            result_refs: BTreeMap::new(),
            evidence_refs: BTreeMap::new(),
            terminal_record: None,
            terminal_history: Vec::new(),
            revision: 1,
        }
    }

    #[test]
    fn canonical_fixture_round_trips_without_derived_state() {
        let value = serde_json::to_value(fixture()).unwrap();
        let text = serde_json::to_string(&value).unwrap();

        for forbidden in ["\"status\"", "\"open\"", "\"active_lease\"", "\"current\""] {
            assert!(!text.contains(forbidden), "{forbidden} leaked into {text}");
        }
        assert_eq!(
            serde_json::from_value::<TaskSpaceCanonicalMap>(value).unwrap(),
            fixture()
        );
    }

    #[test]
    fn legacy_node_state_fields_are_rejected_instead_of_ignored() {
        let mut value = serde_json::to_value(fixture()).unwrap();
        value["root"]["status"] = serde_json::json!("open");

        let error = serde_json::from_value::<TaskSpaceCanonicalMap>(value).unwrap_err();

        assert!(error.to_string().contains("unknown field `status`"));
    }
}

pub type ActionReservation = TaskSpaceActionReservation;
pub type BlockRecord = TaskSpaceBlockRecord;
pub type CompletionRecord = TaskSpaceCompletionRecord;
pub type EvidenceRef = TaskSpaceEvidenceRef;
pub type MapEdge = TaskSpaceMapEdge;
pub type MapId = TaskSpaceMapId;
pub type MapNode = TaskSpaceMapNode;
pub type NodeId = TaskSpaceNodeId;
pub type NodeState = TaskSpaceNodeState;
pub type NodeView = TaskSpaceNodeView;
pub type ReservationId = TaskSpaceReservationId;
pub type ResultRef = TaskSpaceResultRef;
pub type Revision = TaskSpaceRevision;
pub type TerminalRecord = TaskSpaceTerminalRecord;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;

pub type TaskSpaceMap = TaskSpaceCanonicalMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRole {
    TaskRoot,
    Work,
    Finish,
}

impl NodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TaskRoot => "task_root",
            Self::Work => "work",
            Self::Finish => "finish",
        }
    }
}

pub fn new_map(
    map_id: MapId,
    root: MapNode,
    work_nodes: Vec<MapNode>,
    finish: MapNode,
    edges: Vec<MapEdge>,
) -> TaskSpaceMap {
    TaskSpaceMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id,
        root,
        work_nodes,
        finish,
        edges,
        completion_records: Default::default(),
        block_records: Default::default(),
        action_reservations: Default::default(),
        result_refs: Default::default(),
        evidence_refs: Default::default(),
        terminal_record: None,
        terminal_history: Vec::new(),
        revision: 1,
    }
}

pub fn node<'a>(map: &'a TaskSpaceMap, node_id: &str) -> Option<&'a MapNode> {
    if map.root.node_id == node_id {
        return Some(&map.root);
    }
    if map.finish.node_id == node_id {
        return Some(&map.finish);
    }
    map.work_nodes
        .iter()
        .find(|candidate| candidate.node_id == node_id)
}

pub fn node_role(map: &TaskSpaceMap, node_id: &str) -> Option<NodeRole> {
    if map.root.node_id == node_id {
        return Some(NodeRole::TaskRoot);
    }
    if map.finish.node_id == node_id {
        return Some(NodeRole::Finish);
    }
    map.work_nodes
        .iter()
        .any(|candidate| candidate.node_id == node_id)
        .then_some(NodeRole::Work)
}

pub fn node_ids(map: &TaskSpaceMap) -> BTreeSet<&str> {
    std::iter::once(map.root.node_id.as_str())
        .chain(
            map.work_nodes
                .iter()
                .map(|work_node| work_node.node_id.as_str()),
        )
        .chain(std::iter::once(map.finish.node_id.as_str()))
        .collect()
}

pub fn canonicalize(map: &mut TaskSpaceMap) {
    map.work_nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    map.edges.sort();
}

pub fn state_sha256(map: &TaskSpaceMap) -> Result<String, serde_json::Error> {
    let mut canonical = map.clone();
    canonicalize(&mut canonical);
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn is_complete(map: &TaskSpaceMap) -> bool {
    map.terminal_record.is_some()
}

pub fn started_node_ids(map: &TaskSpaceMap) -> BTreeSet<&str> {
    let mut started = BTreeSet::new();
    started.extend(map.completion_records.keys().map(String::as_str));
    started.extend(map.block_records.keys().map(String::as_str));
    started.extend(
        map.action_reservations
            .values()
            .map(|reservation| reservation.node_id.as_str()),
    );
    started.extend(
        map.result_refs
            .values()
            .map(|record| record.node_id.as_str()),
    );
    started.extend(
        map.evidence_refs
            .values()
            .map(|record| record.node_id.as_str()),
    );
    started
}

pub fn map_node(
    node_id: impl Into<String>,
    goal: impl Into<String>,
    source_refs: Vec<String>,
) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: goal.into(),
        source_refs,
    }
}
