use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub const TASKSPACE_CANONICAL_SCHEMA_VERSION: &str = "taskspace-canonical-map-v1";

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
