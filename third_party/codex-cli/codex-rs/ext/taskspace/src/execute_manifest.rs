use codex_extension_api::ToolBatchCall;
use serde::Deserialize;
use std::collections::HashSet;

use crate::initialize_manifest::GraphNode;
use crate::model::ActionReservation;
use crate::model::BlockRecord;
use crate::model::CompletionRecord;
use crate::model::MapEdge;
use crate::runtime::PreparedAction;
use crate::transactions::ExecuteTransaction;
use crate::transactions::GraphMutation;
use crate::transactions::NodeMutation;
use crate::transactions::ReservationInput;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteArgs {
    #[serde(rename = "action")]
    _action: ExecuteAction,
    expected_revision: u64,
    #[serde(default)]
    mutations: Vec<Mutation>,
    actions: Vec<ActionManifest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExecuteAction {
    Execute,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActionManifest {
    pub(crate) node_id: String,
    pub(crate) tool: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Mutation {
    AddWorkNodes { work_nodes: Vec<GraphNode> },
    AddEdges { edges: Vec<MapEdge> },
    RemoveEdges { edges: Vec<MapEdge> },
    CompleteNode { node_id: String },
    BlockNode { node_id: String },
    UnblockNode { node_id: String },
}

pub(crate) fn prepare(
    map_id: &str,
    control_call_id: &str,
    arguments: &str,
    siblings: &[ToolBatchCall],
) -> anyhow::Result<(ExecuteTransaction, Vec<PreparedAction>)> {
    let input: ExecuteArgs = serde_json::from_str(arguments)?;
    let (reservations, prepared) =
        prepare_actions(map_id, control_call_id, input.actions, siblings)?;
    let mut graph = GraphMutation::default();
    let mut node_mutations = Vec::new();
    for (index, mutation) in input.mutations.into_iter().enumerate() {
        let action_id = format!("{map_id}:control:{control_call_id}:mutation:{index}");
        match mutation {
            Mutation::AddWorkNodes { work_nodes } => graph.add_work_nodes.extend(
                work_nodes
                    .into_iter()
                    .map(|node| node.into_map_node(Vec::new())),
            ),
            Mutation::AddEdges { edges } => graph.add_edges.extend(edges),
            Mutation::RemoveEdges { edges } => graph.remove_edges.extend(edges),
            Mutation::CompleteNode { node_id } => node_mutations.push(NodeMutation::Complete {
                node_id,
                record: CompletionRecord {
                    action_id,
                    result_ref_ids: Vec::new(),
                    evidence_ref_ids: Vec::new(),
                },
            }),
            Mutation::BlockNode { node_id } => node_mutations.push(NodeMutation::Block {
                node_id,
                record: BlockRecord {
                    reason_ref: action_id.clone(),
                    action_id,
                },
            }),
            Mutation::UnblockNode { node_id } => {
                node_mutations.push(NodeMutation::Unblock { node_id });
            }
        }
    }
    Ok((
        ExecuteTransaction {
            expected_revision: input.expected_revision,
            graph,
            node_mutations,
            reservations,
        },
        prepared,
    ))
}

pub(crate) fn prepare_actions(
    map_id: &str,
    control_call_id: &str,
    manifests: Vec<ActionManifest>,
    siblings: &[ToolBatchCall],
) -> anyhow::Result<(Vec<ReservationInput>, Vec<PreparedAction>)> {
    if manifests.len() != siblings.len() || siblings.is_empty() {
        anyhow::bail!("actions must contain exactly one entry for each non-control tool call");
    }
    let mut reservations = Vec::with_capacity(siblings.len());
    let mut prepared = Vec::with_capacity(siblings.len());
    let mut call_ids = HashSet::from([control_call_id]);
    for (offset, (manifest, call)) in manifests.into_iter().zip(siblings).enumerate() {
        let call_index = offset + 1;
        let actual_tool = call.tool_name.to_string();
        if manifest.node_id.is_empty()
            || manifest.tool != actual_tool
            || call.call_id.is_empty()
            || !call_ids.insert(&call.call_id)
        {
            anyhow::bail!(
                "action {offset} must name a node and match sibling tool `{actual_tool}`"
            );
        }
        let reservation_id = format!(
            "{map_id}:reservation:{control_call_id}:{call_index}:{}",
            call.call_id
        );
        let action_id = format!("{map_id}:action:{call_index}:{}", call.call_id);
        reservations.push(ReservationInput {
            reservation_id: reservation_id.clone(),
            reservation: ActionReservation {
                action_id,
                node_id: manifest.node_id.clone(),
                tool_name: actual_tool.clone(),
                response_call_index: call_index as u32,
            },
        });
        prepared.push(PreparedAction {
            map_id: map_id.into(),
            call_id: call.call_id.clone(),
            node_id: manifest.node_id,
            tool_name: actual_tool,
            reservation_id,
        });
    }
    Ok((reservations, prepared))
}
