use std::collections::BTreeMap;

use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ToolName;
use codex_tools::ToolSpecCapability;
use codex_tools::ToolSpecCapabilityInput;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::MapNode;
use crate::action_map::rooted_dag::NodePatch;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;

pub(crate) const INITIALIZE_MAP: &str = "initialize_map";
pub(crate) const UPDATE_MAP: &str = "update_map";
pub(crate) const READ_MAP: &str = "read_map";
pub(crate) const REOPEN_MAP: &str = "reopen_map";
pub(crate) const FINISH_MAP: &str = "finish_map";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool", content = "arguments", rename_all = "snake_case")]
pub(crate) enum MapOperation {
    InitializeMap(InitializeMapArgs),
    UpdateMap(UpdateMapArgs),
    ReadMap(EmptyArgs),
    ReopenMap(EmptyArgs),
    FinishMap(FinishMapArgs),
}

impl MapOperation {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::InitializeMap(_) => INITIALIZE_MAP,
            Self::UpdateMap(_) => UPDATE_MAP,
            Self::ReadMap(_) => READ_MAP,
            Self::ReopenMap(_) => REOPEN_MAP,
            Self::FinishMap(_) => FINISH_MAP,
        }
    }

    pub(crate) fn is_initialize(&self) -> bool {
        matches!(self, Self::InitializeMap(_))
    }

    pub(crate) fn is_reopen(&self) -> bool {
        matches!(self, Self::ReopenMap(_))
    }

    pub(crate) fn is_read(&self) -> bool {
        matches!(self, Self::ReadMap(_))
    }

    pub(crate) fn is_finish(&self) -> bool {
        matches!(self, Self::FinishMap(_))
    }

    pub(crate) fn is_noop_update(&self) -> bool {
        matches!(
            self,
            Self::UpdateMap(args)
                if args.add_work_nodes.is_empty()
                    && args.node_patches.iter().all(NodePatchArgs::is_noop)
        )
    }

    pub(crate) fn completes_work_node(&self) -> bool {
        matches!(
            self,
            Self::UpdateMap(args)
                if args
                    .node_patches
                    .iter()
                    .any(|patch| patch.state == Some(NodeState::Completed))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InitializeMapArgs {
    pub(crate) root: BoundaryNodeArgs,
    pub(crate) work_nodes: Vec<WorkNodeArgs>,
    pub(crate) finish: BoundaryNodeArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundaryNodeArgs {
    pub(crate) node_id: String,
    pub(crate) goal: String,
    pub(crate) content: String,
    pub(crate) parents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkNodeArgs {
    pub(crate) node_id: String,
    pub(crate) goal: String,
    pub(crate) content: String,
    pub(crate) parents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateMapArgs {
    pub(crate) add_work_nodes: Vec<WorkNodeArgs>,
    pub(crate) node_patches: Vec<NodePatchArgs>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodePatchArgs {
    pub(crate) node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) goal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) state: Option<NodeState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parents: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct EmptyArgs {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinishMapArgs {
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapOperationEffect {
    Read(TaskSpaceMap),
    Candidate(TaskSpaceMap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapOperationApplyError {
    MapAlreadyInitialized { map_id: String },
    MapNotInitialized { operation: &'static str },
    Rejected(rooted_dag::Rejection),
}

pub(crate) fn apply_map_operation(
    current: Option<&TaskSpaceMap>,
    map_id: &str,
    operation: MapOperation,
) -> Result<MapOperationEffect, MapOperationApplyError> {
    match operation {
        MapOperation::InitializeMap(args) => {
            if current.is_some() {
                return Err(MapOperationApplyError::MapAlreadyInitialized {
                    map_id: map_id.to_string(),
                });
            }
            rooted_dag::initialize(args.into_transaction(map_id))
                .map(|commit| MapOperationEffect::Candidate(commit.map))
                .map_err(MapOperationApplyError::Rejected)
        }
        MapOperation::UpdateMap(args) => {
            let current = require_map(current, UPDATE_MAP)?;
            rooted_dag::execute(current, args.into_transaction(current.revision))
                .map(|commit| MapOperationEffect::Candidate(commit.map))
                .map_err(MapOperationApplyError::Rejected)
        }
        MapOperation::ReadMap(_) => current.cloned().map(MapOperationEffect::Read).ok_or(
            MapOperationApplyError::MapNotInitialized {
                operation: READ_MAP,
            },
        ),
        MapOperation::ReopenMap(_) => {
            let current = require_map(current, REOPEN_MAP)?;
            rooted_dag::reopen_map(
                current,
                rooted_dag::ReopenMap {
                    request_revision: current.revision,
                },
            )
            .map(|commit| MapOperationEffect::Candidate(commit.map))
            .map_err(MapOperationApplyError::Rejected)
        }
        MapOperation::FinishMap(args) => {
            let current = require_map(current, FINISH_MAP)?;
            rooted_dag::finish_map(
                current,
                rooted_dag::FinishMap {
                    request_revision: current.revision,
                    content: args.content,
                },
            )
            .map(|commit| MapOperationEffect::Candidate(commit.map))
            .map_err(MapOperationApplyError::Rejected)
        }
    }
}

fn require_map<'a>(
    current: Option<&'a TaskSpaceMap>,
    operation: &'static str,
) -> Result<&'a TaskSpaceMap, MapOperationApplyError> {
    current.ok_or(MapOperationApplyError::MapNotInitialized { operation })
}

impl InitializeMapArgs {
    fn into_transaction(self, map_id: &str) -> rooted_dag::InitializeMap {
        rooted_dag::InitializeMap {
            map_id: map_id.to_string(),
            root: self.root.into_node(NodeState::InFlight),
            work_nodes: self
                .work_nodes
                .into_iter()
                .map(WorkNodeArgs::into_node)
                .collect(),
            finish: self.finish.into_node(NodeState::Waiting),
        }
    }
}

impl BoundaryNodeArgs {
    fn into_node(self, state: NodeState) -> MapNode {
        MapNode {
            node_id: self.node_id,
            goal: self.goal,
            state,
            content: self.content,
            parents: self.parents,
            actions: Vec::new(),
        }
    }
}

impl WorkNodeArgs {
    fn into_node(self) -> MapNode {
        MapNode {
            node_id: self.node_id,
            goal: self.goal,
            state: NodeState::Waiting,
            content: self.content,
            parents: self.parents,
            actions: Vec::new(),
        }
    }
}

impl UpdateMapArgs {
    fn into_transaction(self, request_revision: u64) -> rooted_dag::ExecuteTransaction {
        rooted_dag::ExecuteTransaction {
            request_revision,
            add_work_nodes: self
                .add_work_nodes
                .into_iter()
                .map(WorkNodeArgs::into_node)
                .collect(),
            patches: self
                .node_patches
                .into_iter()
                .map(NodePatchArgs::into_patch)
                .collect(),
        }
    }
}

impl NodePatchArgs {
    fn is_noop(&self) -> bool {
        self.goal.is_none()
            && self.state.is_none()
            && self.content.is_none()
            && self.parents.is_none()
    }

    fn into_patch(self) -> NodePatch {
        NodePatch {
            node_id: self.node_id,
            goal: self.goal,
            state: self.state,
            content: self.content,
            parents: self.parents,
            append_actions: Vec::new(),
        }
    }
}

pub(crate) fn map_operation_capabilities() -> Vec<ToolSpecCapability> {
    vec![
        map_capability(
            INITIALIZE_MAP,
            "Initialize one rooted TaskSpace Map with at least one Work node and one unique Finish node. This operation is first and accompanies real work.",
            initialize_schema(),
        ),
        map_capability(
            UPDATE_MAP,
            "Add Work nodes or explicitly patch existing node goal, state, content, or parents. Completing a Work node accompanies later work or finish_map. Parent completion mechanically derives dependent-node readiness; do not also patch a waiting dependent to ready or in_flight.",
            update_schema(),
        ),
        map_capability(
            READ_MAP,
            "Read the complete current TaskSpace Map as the only call in this batch.",
            empty_schema(),
        ),
        map_capability(
            REOPEN_MAP,
            "Reopen the current finished TaskSpace Map after user follow-up. This operation is first and accompanies real work.",
            empty_schema(),
        ),
        map_capability(
            FINISH_MAP,
            "Explicitly close the TaskSpace Map after all Work nodes complete. This operation is last and writes the final task summary.",
            object_schema(
                [(
                    "content",
                    JsonSchema::string(Some("Final task summary.".into())),
                )],
                &["content"],
            ),
        ),
    ]
}

fn map_capability(name: &str, description: &str, parameters: JsonSchema) -> ToolSpecCapability {
    ToolSpecCapability {
        public_name: name.to_string(),
        tool_name: ToolName::plain(name),
        description: description.to_string(),
        input: ToolSpecCapabilityInput::Function(parameters),
        output_schema: None,
        deferred: false,
    }
}

fn initialize_schema() -> JsonSchema {
    object_schema(
        [
            (
                "root",
                boundary_node_schema("Root node; parents must be empty."),
            ),
            (
                "work_nodes",
                JsonSchema::array(work_node_schema(), Some("Initial work nodes.".into()))
                    .with_min_items(1),
            ),
            (
                "finish",
                boundary_node_schema("Unique Finish node; parents must be non-empty."),
            ),
        ],
        &["root", "work_nodes", "finish"],
    )
}

fn update_schema() -> JsonSchema {
    let patch = object_schema(
        [
            ("node_id", JsonSchema::string(None)),
            ("goal", JsonSchema::string(None)),
            ("state", node_state_schema()),
            ("content", JsonSchema::string(None)),
            ("parents", JsonSchema::array(JsonSchema::string(None), None)),
        ],
        &["node_id"],
    );
    object_schema(
        [
            (
                "add_work_nodes",
                JsonSchema::array(work_node_schema(), Some("New work nodes.".into())),
            ),
            (
                "node_patches",
                JsonSchema::array(patch, Some("Updates to existing nodes.".into())),
            ),
        ],
        &["add_work_nodes", "node_patches"],
    )
}

fn boundary_node_schema(description: &str) -> JsonSchema {
    let mut schema = object_schema(
        [
            ("node_id", JsonSchema::string(None)),
            ("goal", JsonSchema::string(None)),
            ("content", JsonSchema::string(None)),
            ("parents", JsonSchema::array(JsonSchema::string(None), None)),
        ],
        &["node_id", "goal", "content", "parents"],
    );
    schema.description = Some(description.to_string());
    schema
}

fn work_node_schema() -> JsonSchema {
    object_schema(
        [
            ("node_id", JsonSchema::string(None)),
            ("goal", JsonSchema::string(None)),
            ("content", JsonSchema::string(None)),
            (
                "parents",
                JsonSchema::array(JsonSchema::string(None), None).with_min_items(1),
            ),
        ],
        &["node_id", "goal", "content", "parents"],
    )
}

fn node_state_schema() -> JsonSchema {
    JsonSchema::string_enum(
        ["waiting", "ready", "in_flight", "blocked", "completed"]
            .into_iter()
            .map(|value| json!(value))
            .collect(),
        None,
    )
}

fn empty_schema() -> JsonSchema {
    object_schema(std::iter::empty::<(&str, JsonSchema)>(), &[])
}

fn object_schema<const N: usize>(
    properties: impl IntoIterator<Item = (&'static str, JsonSchema)>,
    required: &[&str; N],
) -> JsonSchema {
    JsonSchema::object(
        properties
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect::<BTreeMap<_, _>>(),
        Some(required.iter().map(|name| (*name).to_string()).collect()),
        Some(AdditionalProperties::Boolean(false)),
    )
}

#[cfg(test)]
mod schema_tests {
    use super::*;

    #[test]
    fn new_work_nodes_hide_runtime_state_while_node_patches_expose_it() {
        let work = serde_json::to_value(work_node_schema()).unwrap();
        assert!(work["properties"].get("state").is_none());

        let update = serde_json::to_value(update_schema()).unwrap();
        assert!(
            update["properties"]["node_patches"]["items"]["properties"]
                .get("state")
                .is_some()
        );
    }
}
