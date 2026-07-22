use crate::function_tool::FunctionCallError;
use crate::tools::handlers::taskspace_control_args::TaskSpaceFinishIdentityArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphEdgeArgs;
use crate::tools::handlers::taskspace_control_args::TaskSpaceGraphNodeArgs;
use crate::tools::handlers::taskspace_control_args::validate_initialize_map;
use crate::tools::handlers::taskspace_control_args::validate_node_id;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub(crate) enum TaskSpaceTransitionArgs {
    InitializeMap {
        root: TaskSpaceGraphNodeArgs,
        initial_work_node: TaskSpaceGraphNodeArgs,
        finish_identity: TaskSpaceFinishIdentityArgs,
        additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
        edges: Vec<TaskSpaceGraphEdgeArgs>,
    },
    BindNode {
        expected_revision: u64,
        node_id: String,
    },
    CompleteThenContinue {
        expected_revision: u64,
        current_node_id: String,
        next_node_id: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    InitializeMap,
    BindNode,
    CompleteThenContinue,
}

#[derive(Deserialize)]
struct Envelope {
    action: Action,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindNodeArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    node_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompleteThenContinueArgs {
    #[serde(rename = "action")]
    _action: Action,
    expected_revision: u64,
    current_node_id: String,
    next_node_id: String,
}

impl TaskSpaceTransitionArgs {
    pub(crate) fn action_name(&self) -> &'static str {
        match self {
            Self::InitializeMap { .. } => "initialize_map",
            Self::BindNode { .. } => "bind_node",
            Self::CompleteThenContinue { .. } => "complete_then_continue",
        }
    }

    pub(crate) fn submitted_expected_revision(&self) -> Option<u64> {
        match self {
            Self::InitializeMap { .. } => None,
            Self::BindNode {
                expected_revision, ..
            }
            | Self::CompleteThenContinue {
                expected_revision, ..
            } => Some(*expected_revision),
        }
    }

    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::InitializeMap {
                root,
                initial_work_node,
                finish_identity,
                additional_work_nodes,
                edges,
            } => validate_initialize_map(
                root,
                initial_work_node,
                finish_identity,
                additional_work_nodes,
                edges,
            ),
            Self::BindNode { node_id, .. } => validate_node_id("bind_node", node_id),
            Self::CompleteThenContinue {
                current_node_id,
                next_node_id,
                ..
            } if current_node_id.trim().is_empty() || next_node_id.trim().is_empty() => {
                Err(FunctionCallError::RespondToModel(
                    "complete_then_continue requires non-empty current_node_id and next_node_id"
                        .into(),
                ))
            }
            Self::CompleteThenContinue { .. } => Ok(()),
        }
    }
}

pub(crate) fn parse_taskspace_transition_args(
    arguments: &str,
) -> Result<TaskSpaceTransitionArgs, FunctionCallError> {
    let action = serde_json::from_str::<Envelope>(arguments)
        .map_err(invalid_transition)?
        .action;
    let args = match action {
        Action::InitializeMap => {
            let parsed =
                serde_json::from_str::<InitializeMapArgs>(arguments).map_err(invalid_transition)?;
            TaskSpaceTransitionArgs::InitializeMap {
                root: parsed.root,
                initial_work_node: parsed.initial_work_node,
                finish_identity: parsed.finish_identity,
                additional_work_nodes: parsed.additional_work_nodes,
                edges: parsed.edges,
            }
        }
        Action::BindNode => {
            let parsed =
                serde_json::from_str::<BindNodeArgs>(arguments).map_err(invalid_transition)?;
            TaskSpaceTransitionArgs::BindNode {
                expected_revision: parsed.expected_revision,
                node_id: parsed.node_id,
            }
        }
        Action::CompleteThenContinue => {
            let parsed = serde_json::from_str::<CompleteThenContinueArgs>(arguments)
                .map_err(invalid_transition)?;
            TaskSpaceTransitionArgs::CompleteThenContinue {
                expected_revision: parsed.expected_revision,
                current_node_id: parsed.current_node_id,
                next_node_id: parsed.next_node_id,
            }
        }
    };
    args.validate()?;
    Ok(args)
}

fn invalid_transition(error: serde_json::Error) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!("invalid taskspace_transition arguments: {error}"))
}

#[cfg(test)]
#[path = "taskspace_transition_args_tests.rs"]
mod tests;
