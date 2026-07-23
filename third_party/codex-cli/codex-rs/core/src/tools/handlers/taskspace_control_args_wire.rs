use super::TaskSpaceControlArgs;
use super::TaskSpaceFinishIdentityArgs;
use super::TaskSpaceGraphEdgeArgs;
use super::TaskSpaceGraphNodeArgs;
use super::invalid_error;
use crate::function_tool::FunctionCallError;
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    InitializeMap,
    MutateGraph,
    BindNode,
    CompleteThenContinue,
    BlockNode,
    UnblockNode,
    ReworkNode,
    FinishMap,
    ExpandNodes,
    ReadOutputRef,
    ReadMap,
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
    finish_identity: TaskSpaceFinishIdentityArgs,
    additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
    edges: Vec<TaskSpaceGraphEdgeArgs>,
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeTransitionArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    node_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompleteThenContinueArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    current_node_id: String,
    next_node_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishMapArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    terminal_node_id: String,
    final_summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpandNodesArgs {
    #[serde(rename = "action")]
    _action: Action,
    node_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadOutputMode {
    Head,
    Tail,
    LineRange,
    Grep,
}

impl ReadOutputMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Tail => "tail",
            Self::LineRange => "line_range",
            Self::Grep => "grep",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReadOutputModeEnvelope {
    mode: ReadOutputMode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOutputHeadOrTailArgs {
    #[serde(rename = "action")]
    _action: Action,
    mode: ReadOutputMode,
    output_ref: String,
    max_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOutputLineRangeArgs {
    #[serde(rename = "action")]
    _action: Action,
    mode: ReadOutputMode,
    output_ref: String,
    start_line: usize,
    end_line: usize,
    max_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOutputGrepArgs {
    #[serde(rename = "action")]
    _action: Action,
    mode: ReadOutputMode,
    output_ref: String,
    pattern: String,
    max_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadMapArgs {
    #[serde(rename = "action")]
    _action: Action,
}

pub(super) fn parse(arguments: &str) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    match deserialize_arguments::<Envelope>(arguments)?.action {
        Action::InitializeMap => {
            let parsed = deserialize_arguments::<InitializeMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::InitializeMap {
                root: parsed.root,
                initial_work_node: parsed.initial_work_node,
                finish_identity: parsed.finish_identity,
                additional_work_nodes: parsed.additional_work_nodes,
                edges: parsed.edges,
            })
        }
        Action::MutateGraph => {
            let parsed = deserialize_arguments::<MutateGraphArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::MutateGraph {
                expected_revision: parsed.expected_revision,
                add_nodes: parsed.add_nodes,
                add_edges: parsed.add_edges,
                remove_edges: parsed.remove_edges,
            })
        }
        Action::BindNode => {
            let (expected_revision, node_id) = parse_node_transition(arguments)?;
            Ok(TaskSpaceControlArgs::BindNode {
                expected_revision,
                node_id,
            })
        }
        Action::CompleteThenContinue => {
            let parsed = deserialize_arguments::<CompleteThenContinueArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::CompleteThenContinue {
                expected_revision: parsed.expected_revision,
                current_node_id: parsed.current_node_id,
                next_node_id: parsed.next_node_id,
            })
        }
        Action::BlockNode => {
            let (expected_revision, node_id) = parse_node_transition(arguments)?;
            Ok(TaskSpaceControlArgs::BlockNode {
                expected_revision,
                node_id,
            })
        }
        Action::UnblockNode => {
            let (expected_revision, node_id) = parse_node_transition(arguments)?;
            Ok(TaskSpaceControlArgs::UnblockNode {
                expected_revision,
                node_id,
            })
        }
        Action::ReworkNode => {
            let (expected_revision, node_id) = parse_node_transition(arguments)?;
            Ok(TaskSpaceControlArgs::ReworkNode {
                expected_revision,
                node_id,
            })
        }
        Action::FinishMap => {
            let parsed = deserialize_arguments::<FinishMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::FinishMap {
                expected_revision: parsed.expected_revision,
                terminal_node_id: parsed.terminal_node_id,
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
            let mode = deserialize_arguments::<ReadOutputModeEnvelope>(arguments)?.mode;
            match mode {
                ReadOutputMode::Head | ReadOutputMode::Tail => {
                    let parsed = deserialize_arguments::<ReadOutputHeadOrTailArgs>(arguments)?;
                    Ok(TaskSpaceControlArgs::ReadOutputRef {
                        output_ref: parsed.output_ref,
                        mode: parsed.mode.as_str().into(),
                        start_line: None,
                        end_line: None,
                        pattern: None,
                        max_bytes: Some(parsed.max_bytes),
                    })
                }
                ReadOutputMode::LineRange => {
                    let parsed = deserialize_arguments::<ReadOutputLineRangeArgs>(arguments)?;
                    Ok(TaskSpaceControlArgs::ReadOutputRef {
                        output_ref: parsed.output_ref,
                        mode: parsed.mode.as_str().into(),
                        start_line: Some(parsed.start_line),
                        end_line: Some(parsed.end_line),
                        pattern: None,
                        max_bytes: Some(parsed.max_bytes),
                    })
                }
                ReadOutputMode::Grep => {
                    let parsed = deserialize_arguments::<ReadOutputGrepArgs>(arguments)?;
                    Ok(TaskSpaceControlArgs::ReadOutputRef {
                        output_ref: parsed.output_ref,
                        mode: parsed.mode.as_str().into(),
                        start_line: None,
                        end_line: None,
                        pattern: Some(parsed.pattern),
                        max_bytes: Some(parsed.max_bytes),
                    })
                }
            }
        }
        Action::ReadMap => {
            let _ = deserialize_arguments::<ReadMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::ReadMap)
        }
    }
}

fn parse_node_transition(arguments: &str) -> Result<(u64, String), FunctionCallError> {
    let parsed = deserialize_arguments::<NodeTransitionArgs>(arguments)?;
    Ok((parsed.expected_revision, parsed.node_id))
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
