use std::collections::HashSet;

use crate::function_tool::FunctionCallError;
use serde::Deserialize;

#[path = "taskspace_control_args_error.rs"]
mod error;
use error::invalid_argument_error;
use error::normalize_invalid_arguments;
pub(crate) use error::with_argument_error_canonical_revision;

#[path = "taskspace_control_args_wire.rs"]
mod wire;

pub(crate) const TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION: &str = "TaskSpaceControlResultV2";

#[derive(Clone, Debug)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeMap {
        root: TaskSpaceGraphNodeArgs,
        initial_work_node: TaskSpaceGraphNodeArgs,
        finish_identity: TaskSpaceFinishIdentityArgs,
        additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
        edges: Vec<TaskSpaceGraphEdgeArgs>,
    },
    MutateGraph {
        expected_revision: u64,
        add_nodes: Vec<TaskSpaceGraphNodeArgs>,
        add_edges: Vec<TaskSpaceGraphEdgeArgs>,
        remove_edges: Vec<TaskSpaceGraphEdgeArgs>,
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
    BlockNode {
        expected_revision: u64,
        node_id: String,
    },
    UnblockNode {
        expected_revision: u64,
        node_id: String,
    },
    ReworkNode {
        expected_revision: u64,
        node_id: String,
    },
    FinishMap {
        expected_revision: u64,
        terminal_node_id: String,
        final_summary: String,
    },
    ExpandNodes {
        node_ids: Vec<String>,
    },
    ReadOutputRef {
        output_ref: String,
        mode: String,
        start_line: Option<usize>,
        end_line: Option<usize>,
        pattern: Option<String>,
        max_bytes: Option<usize>,
    },
    ReadMap,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceGraphNodeArgs {
    pub(crate) node_id: String,
    pub(crate) goal: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceFinishIdentityArgs {
    pub(crate) id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceGraphEdgeArgs {
    pub(crate) from: String,
    pub(crate) to: String,
}

impl TaskSpaceControlArgs {
    pub(crate) fn action_name(&self) -> &'static str {
        match self {
            Self::InitializeMap { .. } => "initialize_map",
            Self::MutateGraph { .. } => "mutate_graph",
            Self::BindNode { .. } => "bind_node",
            Self::CompleteThenContinue { .. } => "complete_then_continue",
            Self::BlockNode { .. } => "block_node",
            Self::UnblockNode { .. } => "unblock_node",
            Self::ReworkNode { .. } => "rework_node",
            Self::FinishMap { .. } => "finish_map",
            Self::ExpandNodes { .. } => "expand_nodes",
            Self::ReadOutputRef { .. } => "read_output_ref",
            Self::ReadMap => "read_map",
        }
    }

    pub(crate) fn submitted_expected_revision(&self) -> Option<u64> {
        match self {
            Self::InitializeMap { .. } => None,
            Self::MutateGraph {
                expected_revision, ..
            }
            | Self::BindNode {
                expected_revision, ..
            }
            | Self::CompleteThenContinue {
                expected_revision, ..
            }
            | Self::BlockNode {
                expected_revision, ..
            }
            | Self::UnblockNode {
                expected_revision, ..
            }
            | Self::ReworkNode {
                expected_revision, ..
            }
            | Self::FinishMap {
                expected_revision, ..
            } => Some(*expected_revision),
            Self::ExpandNodes { .. } | Self::ReadOutputRef { .. } | Self::ReadMap => None,
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
            Self::MutateGraph {
                add_nodes,
                add_edges,
                remove_edges,
                ..
            } => {
                if add_nodes.is_empty() && add_edges.is_empty() && remove_edges.is_empty() {
                    return invalid(
                        "mutate_graph requires at least one add_nodes, add_edges, or remove_edges item",
                    );
                }
                validate_unique_nodes(add_nodes, "mutate_graph.add_nodes")?;
                validate_edges(add_edges, "mutate_graph.add_edges")?;
                validate_edges(remove_edges, "mutate_graph.remove_edges")?;
                validate_unique_edges(add_edges, "mutate_graph.add_edges")?;
                validate_unique_edges(remove_edges, "mutate_graph.remove_edges")?;
                Ok(())
            }
            Self::BindNode { node_id, .. } => validate_node_id("bind_node", node_id),
            Self::CompleteThenContinue {
                current_node_id,
                next_node_id,
                ..
            } => {
                validate_node_id("complete_then_continue.current_node_id", current_node_id)?;
                validate_node_id("complete_then_continue.next_node_id", next_node_id)
            }
            Self::BlockNode { node_id, .. } => validate_node_id("block_node", node_id),
            Self::UnblockNode { node_id, .. } => validate_node_id("unblock_node", node_id),
            Self::ReworkNode { node_id, .. } => validate_node_id("rework_node", node_id),
            Self::FinishMap {
                terminal_node_id,
                final_summary,
                ..
            } => {
                if terminal_node_id.trim().is_empty() {
                    return invalid("finish_map requires a non-empty terminal_node_id");
                }
                if final_summary.trim().is_empty() {
                    return invalid("finish_map requires a non-empty final_summary");
                }
                Ok(())
            }
            Self::ExpandNodes { node_ids } => {
                require_non_empty(node_ids, "node_ids")?;
                let mut unique_node_ids = HashSet::with_capacity(node_ids.len());
                for node_id in node_ids {
                    if node_id.trim().is_empty() {
                        return invalid("expand_nodes requires non-empty node_ids");
                    }
                    if !unique_node_ids.insert(node_id) {
                        return invalid("expand_nodes requires unique node_ids");
                    }
                }
                Ok(())
            }
            Self::ReadOutputRef {
                mode,
                start_line,
                end_line,
                max_bytes,
                ..
            } => {
                if max_bytes.is_none_or(|value| value == 0) {
                    return invalid("read_output_ref requires max_bytes greater than zero");
                }
                if mode == "line_range"
                    && (start_line.is_none_or(|value| value == 0)
                        || end_line.is_none_or(|value| value == 0))
                {
                    return invalid(
                        "read_output_ref line_range requires positive start_line and end_line",
                    );
                }
                Ok(())
            }
            Self::ReadMap => Ok(()),
        }
    }
}

pub(crate) fn parse_taskspace_control_args(
    arguments: &str,
) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    let args =
        wire::parse(arguments).map_err(|error| normalize_invalid_arguments(arguments, error))?;
    args.validate()
        .map_err(|error| normalize_invalid_arguments(arguments, error))?;
    Ok(args)
}

pub(crate) fn validate_initialize_map(
    root: &TaskSpaceGraphNodeArgs,
    initial_work_node: &TaskSpaceGraphNodeArgs,
    finish_identity: &TaskSpaceFinishIdentityArgs,
    additional_work_nodes: &[TaskSpaceGraphNodeArgs],
    edges: &[TaskSpaceGraphEdgeArgs],
) -> Result<(), FunctionCallError> {
    let mut all_nodes = Vec::with_capacity(additional_work_nodes.len() + 2);
    all_nodes.push(root);
    all_nodes.push(initial_work_node);
    all_nodes.extend(additional_work_nodes);
    validate_unique_nodes(&all_nodes, "initialize_map nodes")?;
    if finish_identity.id.trim().is_empty() {
        return invalid("initialize_map requires non-empty finish_identity.id");
    }
    if all_nodes
        .iter()
        .any(|node| node.node_id == finish_identity.id)
    {
        return invalid("initialize_map nodes requires unique node_id values");
    }
    validate_edges(edges, "initialize_map.edges")?;
    validate_unique_edges(edges, "initialize_map.edges")?;
    Ok(())
}

fn validate_unique_nodes<T>(nodes: &[T], field: &str) -> Result<(), FunctionCallError>
where
    T: std::borrow::Borrow<TaskSpaceGraphNodeArgs>,
{
    let mut unique_node_ids = HashSet::with_capacity(nodes.len());
    for node in nodes {
        let node = node.borrow();
        if node.node_id.trim().is_empty() {
            return invalid(format!("{field} requires non-empty node_id"));
        }
        if node.goal.trim().is_empty() {
            return invalid(format!("{field} requires non-empty goal"));
        }
        if !unique_node_ids.insert(node.node_id.as_str()) {
            return invalid(format!("{field} requires unique node_id values"));
        }
    }
    Ok(())
}

fn validate_edges(edges: &[TaskSpaceGraphEdgeArgs], field: &str) -> Result<(), FunctionCallError> {
    for edge in edges {
        if edge.from.trim().is_empty() || edge.to.trim().is_empty() {
            return invalid(format!("{field} requires non-empty from and to"));
        }
    }
    Ok(())
}

fn validate_unique_edges(
    edges: &[TaskSpaceGraphEdgeArgs],
    field: &str,
) -> Result<(), FunctionCallError> {
    let mut unique_edges = HashSet::with_capacity(edges.len());
    for edge in edges {
        if !unique_edges.insert((edge.from.as_str(), edge.to.as_str())) {
            return invalid(format!("{field} requires unique edges"));
        }
    }
    Ok(())
}

fn require_non_empty<T>(items: &[T], field: &str) -> Result<(), FunctionCallError> {
    if items.is_empty() {
        invalid(format!(
            "taskspace_control {field} must contain at least one item"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_node_id(action: &str, node_id: &str) -> Result<(), FunctionCallError> {
    if node_id.trim().is_empty() {
        invalid(format!("{action} requires a non-empty node_id"))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, FunctionCallError> {
    Err(invalid_argument_error(message.into()))
}

fn invalid_error(message: String) -> FunctionCallError {
    invalid_argument_error(message)
}

#[cfg(test)]
#[path = "taskspace_control_args_tests.rs"]
mod tests;
