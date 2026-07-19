use std::collections::HashSet;

use crate::function_tool::FunctionCallError;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[path = "taskspace_control_args_wire.rs"]
mod wire;

pub(crate) const TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION: &str = "TaskSpaceControlResultR6V1";

#[derive(Clone, Debug)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeMap {
        root: TaskSpaceGraphNodeArgs,
        initial_work_node: TaskSpaceGraphNodeArgs,
        finish_identity: TaskSpaceFinishIdentityArgs,
        additional_work_nodes: Vec<TaskSpaceGraphNodeArgs>,
        edges: Vec<TaskSpaceGraphEdgeArgs>,
        continuation: TaskSpaceContinuation,
    },
    MutateGraph {
        expected_revision: u64,
        add_nodes: Vec<TaskSpaceGraphNodeArgs>,
        add_edges: Vec<TaskSpaceGraphEdgeArgs>,
        remove_edges: Vec<TaskSpaceGraphEdgeArgs>,
        continuation: Option<TaskSpaceContinuation>,
    },
    TransitionNode {
        expected_revision: u64,
        node_id: String,
        transition: TaskSpaceNodeTransition,
        continuation: Option<TaskSpaceContinuation>,
    },
    CompleteThenContinue {
        expected_revision: u64,
        current_node_id: String,
        next_node_id: String,
        continuation: TaskSpaceContinuation,
    },
    CompleteThenEnd {
        expected_revision: u64,
        current_node_id: String,
        final_summary: String,
    },
    FinishEnd {
        expected_revision: u64,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskSpaceNodeTransition {
    Bind,
    Block,
    Unblock,
    Rework,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskSpaceContinuation {
    Actions {
        actions: Vec<TaskSpaceNestedAction>,
    },
    PatchThenActions {
        patch: TaskSpaceNestedAction,
        #[serde(default)]
        actions: Vec<TaskSpaceNestedAction>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum TaskSpaceNestedAction {
    Function(TaskSpaceNestedFunctionAction),
    Custom(TaskSpaceNestedCustomAction),
}

impl TaskSpaceNestedAction {
    pub(crate) fn tool_name(&self) -> &str {
        match self {
            Self::Function(action) => &action.tool_name,
            Self::Custom(action) => &action.tool_name,
        }
    }

    pub(crate) fn namespace(&self) -> Option<&str> {
        match self {
            Self::Function(action) => action.namespace.as_deref(),
            Self::Custom(_) => None,
        }
    }

    pub(crate) fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    pub(crate) fn to_response_item(&self, call_id: String) -> ResponseItem {
        match self {
            Self::Function(action) => ResponseItem::FunctionCall {
                id: None,
                name: action.tool_name.clone(),
                namespace: action.namespace.clone(),
                arguments: action.arguments.to_string(),
                call_id,
            },
            Self::Custom(action) => ResponseItem::CustomToolCall {
                id: None,
                status: None,
                call_id,
                name: action.tool_name.clone(),
                input: action.input.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceNestedFunctionAction {
    #[serde(default)]
    pub(crate) namespace: Option<String>,
    pub(crate) tool_name: String,
    pub(crate) arguments: JsonValue,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceNestedCustomAction {
    pub(crate) tool_name: String,
    pub(crate) input: String,
}

impl TaskSpaceControlArgs {
    pub(crate) fn nested_actions(&self) -> Vec<TaskSpaceNestedAction> {
        match self {
            Self::InitializeMap { continuation, .. } => continuation.actions(),
            Self::MutateGraph {
                continuation: Some(continuation),
                ..
            }
            | Self::TransitionNode {
                continuation: Some(continuation),
                ..
            }
            | Self::CompleteThenContinue { continuation, .. } => continuation.actions(),
            Self::MutateGraph {
                continuation: None, ..
            }
            | Self::TransitionNode {
                continuation: None, ..
            }
            | Self::CompleteThenEnd { .. }
            | Self::FinishEnd { .. }
            | Self::ExpandNodes { .. }
            | Self::ReadOutputRef { .. }
            | Self::ReadMap => Vec::new(),
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
                continuation,
            } => {
                validate_initialize_map(
                    root,
                    initial_work_node,
                    finish_identity,
                    additional_work_nodes,
                    edges,
                )?;
                continuation.validate()?;
                Ok(())
            }
            Self::MutateGraph {
                add_nodes,
                add_edges,
                remove_edges,
                continuation,
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
                if let Some(continuation) = continuation {
                    continuation.validate()?;
                }
                Ok(())
            }
            Self::TransitionNode {
                node_id,
                transition,
                continuation,
                ..
            } => {
                if node_id.trim().is_empty() {
                    return invalid("transition_node requires a non-empty node_id");
                }
                match (transition, continuation) {
                    (TaskSpaceNodeTransition::Bind, Some(continuation)) => continuation.validate(),
                    (TaskSpaceNodeTransition::Bind, None) => {
                        invalid("transition_node bind requires continuation")
                    }
                    (_, Some(_)) => invalid(
                        "transition_node continuation is only valid with the bind transition",
                    ),
                    (_, None) => Ok(()),
                }
            }
            Self::CompleteThenContinue {
                current_node_id,
                next_node_id,
                continuation,
                ..
            } => {
                if current_node_id.trim().is_empty() || next_node_id.trim().is_empty() {
                    return invalid(
                        "complete_then_continue requires non-empty current_node_id and next_node_id",
                    );
                }
                continuation.validate()
            }
            Self::CompleteThenEnd {
                current_node_id,
                final_summary,
                ..
            } => {
                if current_node_id.trim().is_empty() {
                    return invalid("complete_then_end requires a non-empty current_node_id");
                }
                if final_summary.trim().is_empty() {
                    return invalid("complete_then_end requires a non-empty final_summary");
                }
                Ok(())
            }
            Self::FinishEnd { final_summary, .. } if final_summary.trim().is_empty() => {
                invalid("finish_end requires a non-empty final_summary")
            }
            Self::FinishEnd { .. } => Ok(()),
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
            Self::ReadOutputRef { .. } | Self::ReadMap => Ok(()),
        }
    }
}

impl TaskSpaceContinuation {
    fn actions(&self) -> Vec<TaskSpaceNestedAction> {
        match self {
            Self::Actions { actions } => actions.clone(),
            Self::PatchThenActions { patch, actions } => {
                let mut declared = Vec::with_capacity(actions.len() + 1);
                declared.push(patch.clone());
                declared.extend(actions.iter().cloned());
                declared
            }
        }
    }

    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::Actions { actions } => {
                require_non_empty(actions, "continuation.actions")?;
                validate_nested_actions(actions)
            }
            Self::PatchThenActions { patch, actions } => {
                if !is_plain_apply_patch(patch) {
                    return invalid("continuation.patch must be the unnamespaced apply_patch tool");
                }
                validate_nested_actions(actions)
            }
        }
    }
}

pub(crate) fn parse_taskspace_control_args(
    arguments: &str,
) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    let args = wire::parse(arguments)?;
    args.validate()?;
    Ok(args)
}

fn validate_initialize_map(
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

fn validate_nested_actions(actions: &[TaskSpaceNestedAction]) -> Result<(), FunctionCallError> {
    for action in actions {
        let name = action.tool_name();
        if name.trim().is_empty() {
            return invalid("nested action tool_name cannot be empty");
        }
        if matches!(name, "taskspace_control" | "update_plan") {
            return invalid("nested actions cannot call taskspace_control or update_plan");
        }
        if is_plain_apply_patch(action) {
            return invalid("apply_patch is only valid in continuation.patch");
        }
    }
    Ok(())
}

fn is_plain_apply_patch(action: &TaskSpaceNestedAction) -> bool {
    action.namespace().is_none() && action.tool_name() == "apply_patch"
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

fn invalid<T>(message: impl Into<String>) -> Result<T, FunctionCallError> {
    Err(invalid_error(message.into()))
}

fn invalid_error(message: String) -> FunctionCallError {
    FunctionCallError::RespondToModel(
        serde_json::json!({
            "schema_version": TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION,
            "status": "protocol_failed",
            "success": false,
            "state_commit": false,
            "partial_commit": 0,
            "committed_revision": serde_json::Value::Null,
            "delta": serde_json::Value::Null,
            "error": {
                "class": "protocol",
                "code": "invalid_arguments",
                "message": message,
            },
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "taskspace_control_args_tests.rs"]
mod tests;
