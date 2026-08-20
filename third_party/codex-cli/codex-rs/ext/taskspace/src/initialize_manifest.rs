use codex_extension_api::ToolBatchCall;
use serde::Deserialize;

use crate::execute_manifest::ActionManifest;
use crate::model::MapEdge;
use crate::model::MapNode;
use crate::runtime::PreparedAction;
use crate::transactions::Commit;
use crate::transactions::InitializeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InitializeArgs {
    #[serde(rename = "action")]
    _action: InitializeAction,
    root: GraphNode,
    work_nodes: Vec<GraphNode>,
    finish: GraphNode,
    edges: Vec<MapEdge>,
    actions: Vec<ActionManifest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum InitializeAction {
    InitializeAndExecute,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GraphNode {
    pub(crate) node_id: String,
    pub(crate) goal: String,
}

impl GraphNode {
    pub(crate) fn into_map_node(self, source_refs: Vec<String>) -> MapNode {
        MapNode {
            node_id: self.node_id,
            goal: self.goal,
            source_refs,
        }
    }
}

pub(crate) fn prepare(
    map_id: &str,
    control_call_id: &str,
    arguments: &str,
    siblings: &[ToolBatchCall],
) -> anyhow::Result<(Commit, Vec<PreparedAction>)> {
    let input: InitializeArgs = serde_json::from_str(arguments)?;
    let (reservations, prepared) =
        crate::execute_manifest::prepare_actions(map_id, control_call_id, input.actions, siblings)?;
    let commit = crate::transactions::initialize(InitializeMap {
        map_id: map_id.into(),
        root: input
            .root
            .into_map_node(vec![format!("taskspace-control:{control_call_id}")]),
        work_nodes: input
            .work_nodes
            .into_iter()
            .map(|node| node.into_map_node(Vec::new()))
            .collect(),
        finish: input.finish.into_map_node(Vec::new()),
        edges: input.edges,
        reservations,
    })
    .map_err(|error| anyhow::anyhow!("TaskSpace initialization rejected: {error:?}"))?;
    Ok((commit, prepared))
}
