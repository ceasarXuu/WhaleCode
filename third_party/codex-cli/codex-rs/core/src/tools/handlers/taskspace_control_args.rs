use std::collections::HashSet;

use crate::function_tool::FunctionCallError;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde_json::Value as JsonValue;

pub(crate) const TASKSPACE_CONTROL_RESULT_SCHEMA_VERSION: &str = "TaskSpaceControlResultV2";

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeThenActions {
        initial_nodes: Vec<TaskSpaceInitializeNodeArgs>,
        current_node_id: String,
        continuation: TaskSpaceContinuation,
    },
    FinishNodes {
        finishes: Vec<TaskSpaceNonterminalFinishArgs>,
    },
    FinishThenEnd {
        finish_node_ids: Vec<String>,
        final_candidate: String,
    },
    CreateNode {
        kind: String,
        goal: String,
        #[serde(default)]
        dependency_node_ids: Vec<String>,
        #[serde(default)]
        bind_current: bool,
    },
    BindNode {
        node_id: String,
    },
    BlockNode {
        node_id: String,
    },
    ReadOutputRef {
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
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceInitializeNodeArgs {
    pub(crate) node_id: String,
    pub(crate) kind: String,
    pub(crate) goal: String,
    #[serde(default)]
    pub(crate) dependency_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceNonterminalFinishArgs {
    #[serde(default)]
    pub(crate) node_id: Option<String>,
    pub(crate) next: TaskSpaceNextArgs,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskSpaceNextArgs {
    Existing {
        node_id: String,
    },
    Create {
        node_kind: String,
        goal: String,
        #[serde(default)]
        dependency_node_ids: Vec<String>,
    },
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
            Self::InitializeThenActions { continuation, .. } => continuation.actions(),
            Self::FinishNodes { .. }
            | Self::FinishThenEnd { .. }
            | Self::CreateNode { .. }
            | Self::BindNode { .. }
            | Self::BlockNode { .. }
            | Self::ReadOutputRef { .. } => Vec::new(),
        }
    }

    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::InitializeThenActions {
                initial_nodes,
                continuation,
                ..
            } => {
                require_non_empty(initial_nodes, "initial_nodes")?;
                if initial_nodes.iter().any(|node| node.goal.trim().is_empty()) {
                    return invalid("each initial node requires a non-empty goal");
                }
                continuation.validate()
            }
            Self::FinishNodes { finishes } => {
                require_non_empty(finishes, "finishes")?;
                for finish in finishes {
                    finish.validate()?;
                }
                Ok(())
            }
            Self::FinishThenEnd {
                finish_node_ids,
                final_candidate,
            } => {
                require_non_empty(finish_node_ids, "finish_node_ids")?;
                let mut unique_node_ids = HashSet::with_capacity(finish_node_ids.len());
                for node_id in finish_node_ids {
                    if node_id.trim().is_empty() {
                        return invalid("finish_then_end requires non-empty finish_node_ids");
                    }
                    if !unique_node_ids.insert(node_id) {
                        return invalid("finish_then_end requires unique finish_node_ids");
                    }
                }
                if final_candidate.trim().is_empty() {
                    return invalid("finish_then_end requires a non-empty final_candidate");
                }
                Ok(())
            }
            Self::CreateNode { goal, .. } => {
                if goal.trim().is_empty() {
                    return invalid("create_node requires a non-empty goal");
                }
                Ok(())
            }
            Self::BindNode { .. } | Self::BlockNode { .. } | Self::ReadOutputRef { .. } => Ok(()),
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

impl TaskSpaceNonterminalFinishArgs {
    fn validate(&self) -> Result<(), FunctionCallError> {
        self.next.validate()
    }
}

impl TaskSpaceNextArgs {
    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::Existing { node_id } if node_id.trim().is_empty() => {
                invalid("existing next binding requires a non-empty node_id")
            }
            Self::Create { goal, .. } if goal.trim().is_empty() => {
                invalid("created next binding requires a non-empty goal")
            }
            Self::Existing { .. } | Self::Create { .. } => Ok(()),
        }
    }
}

pub(crate) fn parse_taskspace_control_args(
    arguments: &str,
) -> Result<TaskSpaceControlArgs, FunctionCallError> {
    let args = serde_json::from_str::<TaskSpaceControlArgs>(arguments)
        .map_err(|error| invalid_error(format!("invalid taskspace_control arguments: {error}")))?;
    args.validate()?;
    Ok(args)
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
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_finishes_and_legacy_finish_carrier() {
        let empty = r#"{"action":"finish_nodes","finishes":[]}"#;
        assert!(parse_taskspace_control_args(empty).is_err());
        let legacy = r#"{"action":"finish_then_actions","finishes":[],"actions":[]}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
    }

    #[test]
    fn accepts_complete_finish_barrier() {
        let args = parse_taskspace_control_args(
            r#"{"action":"finish_nodes","finishes":[{"next":{"kind":"existing","node_id":"node-2"}}]}"#,
        )
        .expect("valid args");
        assert!(args.nested_actions().is_empty());
    }

    #[test]
    fn accepts_initialization_without_task_goal() {
        let args = parse_taskspace_control_args(
            r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Inspect"}],"current_node_id":"node-1","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#,
        )
        .expect("valid args");
        assert_eq!(args.nested_actions().len(), 1);
    }

    #[test]
    fn rejects_removed_verbose_map_fields() {
        let legacy = r#"{"action":"initialize_then_actions","task_goal":"goal","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Inspect"}],"current_node_id":"node-1","continuation":{"kind":"actions","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
    }

    #[test]
    fn accepts_one_patch_followed_by_non_patch_actions() {
        let args = parse_taskspace_control_args(
            r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"*** Begin Patch\n*** End Patch"},"actions":[{"tool_name":"exec_command","arguments":{"cmd":"cargo test"}}]}}"#,
        )
        .expect("valid patch continuation");
        let actions = args.nested_actions();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].tool_name(), "apply_patch");
        assert_eq!(actions[1].tool_name(), "exec_command");
    }

    #[test]
    fn rejects_legacy_actions_and_patch_in_ordinary_actions() {
        let legacy = r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","actions":[{"tool_name":"apply_patch","input":"patch"}]}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
        let misplaced = r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","continuation":{"kind":"actions","actions":[{"tool_name":"apply_patch","input":"patch"}]}}"#;
        assert!(parse_taskspace_control_args(misplaced).is_err());
        let repeated = r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"implement_solution","goal":"Edit"}],"current_node_id":"node-1","continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch-1"},"actions":[{"tool_name":"apply_patch","input":"patch-2"}]}}"#;
        assert!(parse_taskspace_control_args(repeated).is_err());
    }

    #[test]
    fn rejects_removed_finish_result_summary() {
        let nonterminal = r#"{"action":"finish_nodes","finishes":[{"result_summary":"done","next":{"kind":"existing","node_id":"node-2"}}]}"#;
        assert!(parse_taskspace_control_args(nonterminal).is_err());
        let terminal = r#"{"action":"finish_then_end","finish_node_ids":["node-2"],"result_summary":"done","final_candidate":"answer"}"#;
        assert!(parse_taskspace_control_args(terminal).is_err());
    }

    #[test]
    fn block_node_only_accepts_node_id() {
        parse_taskspace_control_args(r#"{"action":"block_node","node_id":"node-1"}"#)
            .expect("node-only block args");
        let legacy = r#"{"action":"block_node","node_id":"node-1","blocker_summary":"blocked"}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
    }

    #[test]
    fn accepts_tagged_created_binding_and_rejects_removed_flat_shape() {
        let created = r#"{"action":"finish_nodes","finishes":[{"node_id":"node-1","next":{"kind":"create","node_kind":"smoke_test","goal":"Run tests","dependency_node_ids":["node-1"]}}]}"#;
        parse_taskspace_control_args(created).expect("tagged created binding");

        let removed = r#"{"action":"finish_nodes","finishes":[{"next_node_id":"node-2","next_node_kind":"smoke_test","next_node_goal":"Run tests"}]}"#;
        assert!(parse_taskspace_control_args(removed).is_err());
    }

    #[test]
    fn rejects_cross_variant_and_removed_terminal_wrapper() {
        let cross_variant = r#"{"action":"finish_nodes","finishes":[{"next":{"kind":"existing","node_id":"node-2","goal":"Run tests"}}]}"#;
        assert!(parse_taskspace_control_args(cross_variant).is_err());

        let removed_terminal =
            r#"{"action":"finish_then_end","terminal_finish":{},"final_candidate":"answer"}"#;
        assert!(parse_taskspace_control_args(removed_terminal).is_err());
        parse_taskspace_control_args(
            r#"{"action":"finish_then_end","finish_node_ids":["node-1","node-2"],"final_candidate":"answer"}"#,
        )
        .expect("ordered terminal chain");
        let removed_dual_role = r#"{"action":"finish_then_end","preceding_finishes":[{"node_id":"node-1","next":{"kind":"existing","node_id":"node-2"}}],"terminal_node_id":"node-2","final_candidate":"answer"}"#;
        assert!(parse_taskspace_control_args(removed_dual_role).is_err());
    }

    #[test]
    fn invalid_arguments_return_one_typed_json_payload() {
        let arguments = r#"{"action":"unknown"}"#;
        let source_error = serde_json::from_str::<TaskSpaceControlArgs>(arguments)
            .expect_err("unknown action should fail");
        let error =
            parse_taskspace_control_args(arguments).expect_err("unknown action should fail");
        let FunctionCallError::RespondToModel(payload) = error else {
            panic!("expected model-facing error");
        };
        let value: JsonValue = serde_json::from_str(&payload).expect("single JSON payload");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": "TaskSpaceControlResultV2",
                "status": "protocol_failed",
                "success": false,
                "error": {
                    "class": "protocol",
                    "code": "invalid_arguments",
                    "message": format!("invalid taskspace_control arguments: {source_error}"),
                },
            })
        );
    }
}
