use super::TaskSpaceActionArgs;
use super::TaskSpaceControlArgs;
use super::TaskSpaceGraphEdgeArgs;
use super::TaskSpaceGraphNodeArgs;
use super::TaskSpaceMutationArgs;
use super::invalid_error;
use crate::function_tool::FunctionCallError;
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    InitializeAndExecute,
    Execute,
    ReadMap,
    ReadOutputRef,
    FinishMap,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    action: Action,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InitializeAndExecuteArgs {
    #[serde(rename = "action")]
    _action: Action,
    root: TaskSpaceGraphNodeArgs,
    work_nodes: Vec<TaskSpaceGraphNodeArgs>,
    finish: TaskSpaceGraphNodeArgs,
    edges: Vec<TaskSpaceGraphEdgeArgs>,
    actions: Vec<TaskSpaceActionArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    mutations: Vec<MutationWire>,
    actions: Vec<TaskSpaceActionArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum MutationWire {
    AddWorkNodes {
        work_nodes: Vec<TaskSpaceGraphNodeArgs>,
    },
    AddEdges {
        edges: Vec<TaskSpaceGraphEdgeArgs>,
    },
    RemoveEdges {
        edges: Vec<TaskSpaceGraphEdgeArgs>,
    },
    CompleteNode {
        node_id: String,
    },
    BlockNode {
        node_id: String,
    },
    UnblockNode {
        node_id: String,
    },
    ReworkNode {
        node_id: String,
    },
}

impl From<MutationWire> for TaskSpaceMutationArgs {
    fn from(value: MutationWire) -> Self {
        match value {
            MutationWire::AddWorkNodes { work_nodes } => Self::AddWorkNodes { work_nodes },
            MutationWire::AddEdges { edges } => Self::AddEdges { edges },
            MutationWire::RemoveEdges { edges } => Self::RemoveEdges { edges },
            MutationWire::CompleteNode { node_id } => Self::CompleteNode { node_id },
            MutationWire::BlockNode { node_id } => Self::BlockNode { node_id },
            MutationWire::UnblockNode { node_id } => Self::UnblockNode { node_id },
            MutationWire::ReworkNode { node_id } => Self::ReworkNode { node_id },
        }
    }
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishMapArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    finish_node_id: String,
    exact_summary: String,
}

pub(super) fn parse(arguments: &str) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    match deserialize_arguments::<Envelope>(arguments)?.action {
        Action::InitializeAndExecute => {
            let parsed = deserialize_arguments::<InitializeAndExecuteArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::InitializeAndExecute {
                root: parsed.root,
                work_nodes: parsed.work_nodes,
                finish: parsed.finish,
                edges: parsed.edges,
                actions: parsed.actions,
            })
        }
        Action::Execute => {
            let parsed = deserialize_arguments::<ExecuteArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::Execute {
                expected_revision: parsed.expected_revision,
                mutations: parsed.mutations.into_iter().map(Into::into).collect(),
                actions: parsed.actions,
            })
        }
        Action::ReadMap => {
            let _ = deserialize_arguments::<ReadMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::ReadMap)
        }
        Action::ReadOutputRef => parse_output_read(arguments),
        Action::FinishMap => {
            let parsed = deserialize_arguments::<FinishMapArgs>(arguments)?;
            Ok(TaskSpaceControlArgs::FinishMap {
                expected_revision: parsed.expected_revision,
                finish_node_id: parsed.finish_node_id,
                exact_summary: parsed.exact_summary,
            })
        }
    }
}

fn parse_output_read(arguments: &str) -> Result<TaskSpaceControlArgs, FunctionCallError> {
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
                max_bytes: parsed.max_bytes,
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
                max_bytes: parsed.max_bytes,
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
