use codex_extension_api::ToolBatchCall;
use serde::Deserialize;

use crate::execute_manifest::ActionManifest;
use crate::initialize_manifest::GraphNode;
use crate::model::CompletionRecord;
use crate::model::MapEdge;
use crate::model::TerminalRecord;
use crate::runtime::PreparedAction;
use crate::transactions::FinalCompletion;
use crate::transactions::FinishMap;
use crate::transactions::ReopenMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishArgs {
    #[serde(rename = "action")]
    _action: FinishAction,
    expected_revision: u64,
    finish_node_id: String,
    complete_work_node_ids: Vec<String>,
    exact_summary: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum FinishAction {
    FinishMap,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReopenArgs {
    #[serde(rename = "action")]
    _action: ReopenAction,
    expected_revision: u64,
    work_nodes: Vec<GraphNode>,
    edges: Vec<MapEdge>,
    actions: Vec<ActionManifest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReopenAction {
    ReopenMap,
}

pub(crate) fn prepare_finish(control_call_id: &str, arguments: &str) -> anyhow::Result<FinishMap> {
    let input: FinishArgs = serde_json::from_str(arguments)?;
    if input.complete_work_node_ids.is_empty() {
        anyhow::bail!("complete_work_node_ids must not be empty");
    }
    let final_completions = input
        .complete_work_node_ids
        .into_iter()
        .map(|node_id| FinalCompletion {
            node_id,
            record: CompletionRecord {
                action_id: control_call_id.into(),
                result_ref_ids: Vec::new(),
                evidence_ref_ids: Vec::new(),
            },
        })
        .collect();
    Ok(FinishMap {
        expected_revision: input.expected_revision,
        finish_node_id: input.finish_node_id,
        final_completions,
        terminal: TerminalRecord {
            action_id: control_call_id.into(),
            summary_ref: input.exact_summary,
        },
    })
}

pub(crate) fn prepare_reopen(
    map_id: &str,
    control_call_id: &str,
    arguments: &str,
    siblings: &[ToolBatchCall],
) -> anyhow::Result<(ReopenMap, Vec<PreparedAction>)> {
    let input: ReopenArgs = serde_json::from_str(arguments)?;
    let (reservations, prepared) =
        crate::execute_manifest::prepare_actions(map_id, control_call_id, input.actions, siblings)?;
    Ok((
        ReopenMap {
            expected_revision: input.expected_revision,
            add_work_nodes: input
                .work_nodes
                .into_iter()
                .map(|node| node.into_map_node(Vec::new()))
                .collect(),
            add_edges: input.edges,
            reservations,
        },
        prepared,
    ))
}
