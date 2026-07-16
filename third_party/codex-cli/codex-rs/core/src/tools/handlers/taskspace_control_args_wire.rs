use super::TaskSpaceContinuation;
use super::TaskSpaceControlArgs;
use super::TaskSpaceFinishNodeArgs;
use super::TaskSpaceGraphEdgeArgs;
use super::TaskSpaceGraphNodeArgs;
use super::TaskSpaceNodeTransition;
use super::invalid_error;
use crate::function_tool::FunctionCallError;
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    InitializeMap,
    MutateGraph,
    TransitionNode,
    FinishEnd,
    ExpandNodes,
    ReadOutputRef,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    action: Action,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InitializeMapArgs {
    #[serde(rename = "action")]
    _action: Action,
    root: TaskSpaceGraphNodeArgs,
    initial_work_node: TaskSpaceGraphNodeArgs,
    finish: TaskSpaceFinishNodeArgs,
    additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
    edges: Vec<TaskSpaceGraphEdgeArgs>,
    continuation: TaskSpaceContinuation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutateGraphArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    add_nodes: Vec<TaskSpaceGraphNodeArgs>,
    add_edges: Vec<TaskSpaceGraphEdgeArgs>,
    remove_edges: Vec<TaskSpaceGraphEdgeArgs>,
    #[serde(default)]
    continuation: Option<TaskSpaceContinuation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransitionNodeArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    node_id: String,
    transition: TaskSpaceNodeTransition,
    #[serde(default)]
    continuation: Option<TaskSpaceContinuation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishEndArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    final_summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpandNodesArgs {
    #[serde(rename = "action")]
    _action: Action,
    node_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOutputRefArgs {
    #[serde(rename = "action")]
    _action: Action,
    output_ref: String,
    mode: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    max_bytes: Option<usize>,
}

pub(super) fn parse(arguments: &str) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    match deserialize_arguments::<Envelope>(arguments)?.action {
        Action::InitializeMap => {
            let parsed = deserialize_arguments::<InitializeMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::InitializeMap {
                root: parsed.root,
                initial_work_node: parsed.initial_work_node,
                finish: parsed.finish,
                additional_work_nodes: parsed.additional_work_nodes,
                edges: parsed.edges,
                continuation: parsed.continuation,
            })
        }
        Action::MutateGraph => {
            let parsed = deserialize_arguments::<MutateGraphArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::MutateGraph {
                expected_revision: parsed.expected_revision,
                add_nodes: parsed.add_nodes,
                add_edges: parsed.add_edges,
                remove_edges: parsed.remove_edges,
                continuation: parsed.continuation,
            })
        }
        Action::TransitionNode => {
            let parsed = deserialize_arguments::<TransitionNodeArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::TransitionNode {
                expected_revision: parsed.expected_revision,
                node_id: parsed.node_id,
                transition: parsed.transition,
                continuation: parsed.continuation,
            })
        }
        Action::FinishEnd => {
            let parsed = deserialize_arguments::<FinishEndArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::FinishEnd {
                expected_revision: parsed.expected_revision,
                final_summary: parsed.final_summary,
            })
        }
        Action::ExpandNodes => {
            let parsed = deserialize_arguments::<ExpandNodesArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::ExpandNodes {
                node_ids: parsed.node_ids,
            })
        }
        Action::ReadOutputRef => {
            let parsed = deserialize_arguments::<ReadOutputRefArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::ReadOutputRef {
                output_ref: parsed.output_ref,
                mode: parsed.mode,
                start_line: parsed.start_line,
                end_line: parsed.end_line,
                pattern: parsed.pattern,
                max_bytes: parsed.max_bytes,
            })
        }
    }
}

fn deserialize_arguments<T: DeserializeOwned>(arguments: &str) -> Result<T, FunctionCallError> {
    let mut deserializer = serde_json::Deserializer::from_str(arguments);
    let parsed = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        invalid_error(format!(
            "invalid taskspace_control arguments at {}: {}",
            error.path(),
            error.inner()
        ))
    })?;
    deserializer.end().map_err(|error| {
        invalid_error(format!("invalid taskspace_control arguments at .: {error}"))
    })?;
    Ok(parsed)
}
