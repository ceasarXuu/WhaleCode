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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeAndExecute {
        root: TaskSpaceGraphNodeArgs,
        work_nodes: Vec<TaskSpaceGraphNodeArgs>,
        finish: TaskSpaceGraphNodeArgs,
        edges: Vec<TaskSpaceGraphEdgeArgs>,
        actions: Vec<TaskSpaceActionArgs>,
    },
    Execute {
        expected_revision: u64,
        mutations: Vec<TaskSpaceMutationArgs>,
        actions: Vec<TaskSpaceActionArgs>,
    },
    ReopenMap {
        expected_revision: u64,
        work_nodes: Vec<TaskSpaceGraphNodeArgs>,
        edges: Vec<TaskSpaceGraphEdgeArgs>,
        actions: Vec<TaskSpaceActionArgs>,
    },
    ReadMap,
    ReadOutputRef {
        output_ref: String,
        mode: String,
        start_line: Option<usize>,
        end_line: Option<usize>,
        pattern: Option<String>,
        max_bytes: usize,
    },
    FinishMap {
        expected_revision: u64,
        finish_node_id: String,
        complete_work_node_ids: Vec<String>,
        exact_summary: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceGraphNodeArgs {
    pub(crate) node_id: String,
    pub(crate) goal: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceGraphEdgeArgs {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceActionArgs {
    pub(crate) node_id: String,
    pub(crate) tool: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TaskSpaceMutationArgs {
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
}

impl TaskSpaceControlArgs {
    pub(crate) fn action_name(&self) -> &'static str {
        match self {
            Self::InitializeAndExecute { .. } => "initialize_and_execute",
            Self::Execute { .. } => "execute",
            Self::ReopenMap { .. } => "reopen_map",
            Self::ReadMap => "read_map",
            Self::ReadOutputRef { .. } => "read_output_ref",
            Self::FinishMap { .. } => "finish_map",
        }
    }

    pub(crate) fn submitted_expected_revision(&self) -> Option<u64> {
        match self {
            Self::Execute {
                expected_revision, ..
            }
            | Self::ReopenMap {
                expected_revision, ..
            }
            | Self::FinishMap {
                expected_revision, ..
            } => Some(*expected_revision),
            Self::InitializeAndExecute { .. } | Self::ReadMap | Self::ReadOutputRef { .. } => None,
        }
    }

    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::InitializeAndExecute {
                root,
                work_nodes,
                finish,
                edges,
                actions,
            } => validate_initialize_and_execute(root, work_nodes, finish, edges, actions),
            Self::Execute {
                mutations, actions, ..
            } => {
                validate_mutations(mutations)?;
                validate_actions(actions)
            }
            Self::ReopenMap {
                work_nodes,
                edges,
                actions,
                ..
            } => {
                require_non_empty(work_nodes, "reopen_map.work_nodes")?;
                require_non_empty(edges, "reopen_map.edges")?;
                validate_unique_nodes(work_nodes, "reopen_map.work_nodes")?;
                validate_edges(edges, "reopen_map.edges")?;
                validate_unique_edges(edges, "reopen_map.edges")?;
                validate_actions(actions)
            }
            Self::ReadMap => Ok(()),
            Self::ReadOutputRef {
                output_ref,
                mode,
                start_line,
                end_line,
                pattern,
                max_bytes,
            } => validate_output_read(
                output_ref,
                mode,
                *start_line,
                *end_line,
                pattern.as_deref(),
                *max_bytes,
            ),
            Self::FinishMap {
                finish_node_id,
                complete_work_node_ids,
                exact_summary,
                ..
            } => {
                validate_node_id("finish_map.finish_node_id", finish_node_id)?;
                require_non_empty(complete_work_node_ids, "finish_map.complete_work_node_ids")?;
                validate_unique_node_ids(
                    complete_work_node_ids,
                    "finish_map.complete_work_node_ids",
                )?;
                if exact_summary.trim().is_empty() {
                    return invalid("finish_map requires a non-empty exact_summary");
                }
                Ok(())
            }
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

fn validate_initialize_and_execute(
    root: &TaskSpaceGraphNodeArgs,
    work_nodes: &[TaskSpaceGraphNodeArgs],
    finish: &TaskSpaceGraphNodeArgs,
    edges: &[TaskSpaceGraphEdgeArgs],
    actions: &[TaskSpaceActionArgs],
) -> Result<(), FunctionCallError> {
    require_non_empty(work_nodes, "initialize_and_execute.work_nodes")?;
    require_non_empty(edges, "initialize_and_execute.edges")?;

    let mut all_nodes = Vec::with_capacity(work_nodes.len() + 2);
    all_nodes.push(root);
    all_nodes.extend(work_nodes);
    all_nodes.push(finish);
    validate_unique_nodes(&all_nodes, "initialize_and_execute nodes")?;
    validate_edges(edges, "initialize_and_execute.edges")?;
    validate_unique_edges(edges, "initialize_and_execute.edges")?;
    validate_actions(actions)
}

fn validate_mutations(mutations: &[TaskSpaceMutationArgs]) -> Result<(), FunctionCallError> {
    for mutation in mutations {
        match mutation {
            TaskSpaceMutationArgs::AddWorkNodes { work_nodes } => {
                require_non_empty(work_nodes, "execute.add_work_nodes.work_nodes")?;
                validate_unique_nodes(work_nodes, "execute.add_work_nodes.work_nodes")?;
            }
            TaskSpaceMutationArgs::AddEdges { edges } => {
                validate_mutation_edges(edges, "execute.add_edges.edges")?;
            }
            TaskSpaceMutationArgs::RemoveEdges { edges } => {
                validate_mutation_edges(edges, "execute.remove_edges.edges")?;
            }
            TaskSpaceMutationArgs::CompleteNode { node_id } => {
                validate_node_id("execute.complete_node", node_id)?;
            }
            TaskSpaceMutationArgs::BlockNode { node_id } => {
                validate_node_id("execute.block_node", node_id)?;
            }
            TaskSpaceMutationArgs::UnblockNode { node_id } => {
                validate_node_id("execute.unblock_node", node_id)?;
            }
        }
    }
    Ok(())
}

fn validate_mutation_edges(
    edges: &[TaskSpaceGraphEdgeArgs],
    field: &str,
) -> Result<(), FunctionCallError> {
    require_non_empty(edges, field)?;
    validate_edges(edges, field)?;
    validate_unique_edges(edges, field)
}

fn validate_actions(actions: &[TaskSpaceActionArgs]) -> Result<(), FunctionCallError> {
    require_non_empty(actions, "actions")?;
    for action in actions {
        validate_node_id("actions[].node_id", &action.node_id)?;
        if action.tool.trim().is_empty() {
            return invalid("actions[].tool must be non-empty");
        }
    }
    Ok(())
}

fn validate_output_read(
    output_ref: &str,
    mode: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<&str>,
    max_bytes: usize,
) -> Result<(), FunctionCallError> {
    if output_ref.trim().is_empty() {
        return invalid("read_output_ref requires a non-empty output_ref");
    }
    if max_bytes == 0 {
        return invalid("read_output_ref requires max_bytes greater than zero");
    }
    match mode {
        "head" | "tail" => Ok(()),
        "line_range" => {
            let (Some(start_line), Some(end_line)) = (start_line, end_line) else {
                return invalid("read_output_ref line_range requires start_line and end_line");
            };
            if start_line == 0 || end_line == 0 || start_line > end_line {
                return invalid("read_output_ref line_range requires 1 <= start_line <= end_line");
            }
            Ok(())
        }
        "grep" if pattern.is_some_and(|pattern| !pattern.trim().is_empty()) => Ok(()),
        "grep" => invalid("read_output_ref grep requires a non-empty pattern"),
        _ => invalid("read_output_ref mode must be head, tail, line_range, or grep"),
    }
}

fn validate_unique_nodes<T>(nodes: &[T], field: &str) -> Result<(), FunctionCallError>
where
    T: std::borrow::Borrow<TaskSpaceGraphNodeArgs>,
{
    let mut unique_node_ids = HashSet::with_capacity(nodes.len());
    for node in nodes {
        let node = node.borrow();
        validate_node_id(field, &node.node_id)?;
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

fn validate_node_id(field: &str, node_id: &str) -> Result<(), FunctionCallError> {
    if node_id.trim().is_empty() {
        invalid(format!("{field} requires a non-empty node_id"))
    } else {
        Ok(())
    }
}

fn validate_unique_node_ids(node_ids: &[String], field: &str) -> Result<(), FunctionCallError> {
    let mut unique = HashSet::with_capacity(node_ids.len());
    for node_id in node_ids {
        validate_node_id(field, node_id)?;
        if !unique.insert(node_id.as_str()) {
            return invalid(format!("{field} requires unique node_id values"));
        }
    }
    Ok(())
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
