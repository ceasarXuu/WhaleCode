use serde::Deserialize;
use serde::Serialize;

pub const TASKSPACE_CANONICAL_SCHEMA_VERSION: &str = "taskspace-canonical-map-v4";

pub type TaskSpaceActionId = String;
pub type TaskSpaceMapId = String;
pub type TaskSpaceNodeId = String;
pub type TaskSpaceRevision = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSpaceNodeState {
    Waiting,
    Ready,
    InFlight,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSpaceActionOutcome {
    Pending,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceNodeAction {
    pub action_id: TaskSpaceActionId,
    pub tool_name: String,
    pub outcome: TaskSpaceActionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceMapNode {
    pub node_id: TaskSpaceNodeId,
    pub goal: String,
    pub state: TaskSpaceNodeState,
    pub content: String,
    pub parents: Vec<TaskSpaceNodeId>,
    pub actions: Vec<TaskSpaceNodeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceCanonicalMap {
    pub schema_version: String,
    pub map_id: TaskSpaceMapId,
    pub root: TaskSpaceMapNode,
    pub work_nodes: Vec<TaskSpaceMapNode>,
    pub finish: TaskSpaceMapNode,
    pub revision: TaskSpaceRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceNodeView {
    pub node_id: TaskSpaceNodeId,
    pub goal: String,
    pub state: TaskSpaceNodeState,
    pub content: String,
    pub parents: Vec<TaskSpaceNodeId>,
    pub children: Vec<TaskSpaceNodeId>,
    pub actions: Vec<TaskSpaceNodeAction>,
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
                state: TaskSpaceNodeState::InFlight,
                content: "User requested delivery.".into(),
                parents: vec![],
                actions: vec![],
            },
            work_nodes: vec![TaskSpaceMapNode {
                node_id: "work".into(),
                goal: "implement".into(),
                state: TaskSpaceNodeState::Ready,
                content: String::new(),
                parents: vec!["root".into()],
                actions: vec![TaskSpaceNodeAction {
                    action_id: "call-1".into(),
                    tool_name: "read_file".into(),
                    outcome: TaskSpaceActionOutcome::Succeeded,
                }],
            }],
            finish: TaskSpaceMapNode {
                node_id: "finish".into(),
                goal: "close the task".into(),
                state: TaskSpaceNodeState::Waiting,
                content: String::new(),
                parents: vec!["work".into()],
                actions: vec![],
            },
            revision: 1,
        }
    }

    #[test]
    fn canonical_fixture_round_trips_without_derived_children() {
        let value = serde_json::to_value(fixture()).unwrap();
        let text = serde_json::to_string(&value).unwrap();

        for forbidden in [
            "\"children\"",
            "\"edges\"",
            "_ref",
            "completion_records",
            "terminal_record",
        ] {
            assert!(!text.contains(forbidden), "{forbidden} leaked into {text}");
        }
        assert_eq!(
            serde_json::from_value::<TaskSpaceCanonicalMap>(value).unwrap(),
            fixture()
        );
    }

    #[test]
    fn legacy_fields_are_rejected_instead_of_ignored() {
        for field in ["edges", "source_refs", "completion_records", "result_refs"] {
            let mut value = serde_json::to_value(fixture()).unwrap();
            value[field] = serde_json::json!([]);

            let error = serde_json::from_value::<TaskSpaceCanonicalMap>(value).unwrap_err();

            assert!(
                error.to_string().contains("unknown field"),
                "legacy field {field} was not rejected: {error}"
            );
        }
    }
}
