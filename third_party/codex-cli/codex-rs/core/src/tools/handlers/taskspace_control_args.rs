use crate::function_tool::FunctionCallError;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeThenActions {
        task_title: String,
        task_objective: String,
        initial_nodes: Vec<TaskSpaceInitializeNodeArgs>,
        current_node_key: String,
        actions: Vec<TaskSpaceNestedAction>,
    },
    FinishThenActions {
        finishes: Vec<TaskSpaceNonterminalFinishArgs>,
        actions: Vec<TaskSpaceNestedAction>,
    },
    FinishThenEnd {
        #[serde(default)]
        preceding_finishes: Vec<TaskSpaceNonterminalFinishArgs>,
        terminal_finish: TaskSpaceTerminalFinishArgs,
        final_candidate: String,
    },
    CreateNode {
        kind: String,
        title: String,
        context_summary: String,
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
        blocker_summary: String,
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
    pub(crate) node_key: String,
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) context_summary: String,
    #[serde(default)]
    pub(crate) dependency_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceNonterminalFinishArgs {
    #[serde(default)]
    pub(crate) node_id: Option<String>,
    pub(crate) result_summary: String,
    #[serde(default)]
    pub(crate) next_node_id: Option<String>,
    #[serde(default)]
    pub(crate) next_node_kind: Option<String>,
    #[serde(default)]
    pub(crate) next_node_title: Option<String>,
    #[serde(default)]
    pub(crate) next_node_context_summary: Option<String>,
    #[serde(default)]
    pub(crate) next_dependency_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceTerminalFinishArgs {
    #[serde(default)]
    pub(crate) node_id: Option<String>,
    pub(crate) result_summary: String,
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
    pub(crate) fn nested_actions(&self) -> &[TaskSpaceNestedAction] {
        match self {
            Self::InitializeThenActions { actions, .. }
            | Self::FinishThenActions { actions, .. } => actions,
            Self::FinishThenEnd { .. }
            | Self::CreateNode { .. }
            | Self::BindNode { .. }
            | Self::BlockNode { .. }
            | Self::ReadOutputRef { .. } => &[],
        }
    }

    fn validate(&self) -> Result<(), FunctionCallError> {
        match self {
            Self::InitializeThenActions {
                initial_nodes,
                actions,
                ..
            } => {
                require_non_empty(initial_nodes, "initial_nodes")?;
                require_non_empty(actions, "actions")?;
                validate_nested_actions(actions)
            }
            Self::FinishThenActions { finishes, actions } => {
                require_non_empty(finishes, "finishes")?;
                require_non_empty(actions, "actions")?;
                for finish in finishes {
                    finish.validate()?;
                }
                validate_nested_actions(actions)
            }
            Self::FinishThenEnd {
                preceding_finishes,
                final_candidate,
                ..
            } => {
                for finish in preceding_finishes {
                    finish.validate()?;
                }
                if final_candidate.trim().is_empty() {
                    return invalid("finish_then_end requires a non-empty final_candidate");
                }
                Ok(())
            }
            Self::CreateNode { .. }
            | Self::BindNode { .. }
            | Self::BlockNode { .. }
            | Self::ReadOutputRef { .. } => Ok(()),
        }
    }
}

impl TaskSpaceNonterminalFinishArgs {
    fn validate(&self) -> Result<(), FunctionCallError> {
        let has_existing = non_empty(self.next_node_id.as_deref());
        let draft_fields = [
            self.next_node_kind.as_deref(),
            self.next_node_title.as_deref(),
            self.next_node_context_summary.as_deref(),
        ];
        let has_any_draft = draft_fields.iter().any(|value| non_empty(*value))
            || !self.next_dependency_node_ids.is_empty();
        let has_complete_draft = draft_fields.iter().all(|value| non_empty(*value));
        if has_existing == has_any_draft {
            return invalid(
                "each nonterminal finish requires exactly one next_node_id or next_node_* draft",
            );
        }
        if has_any_draft && !has_complete_draft {
            return invalid(
                "next-node creation requires next_node_kind, next_node_title, and next_node_context_summary",
            );
        }
        Ok(())
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
        let name = match action {
            TaskSpaceNestedAction::Function(action) => &action.tool_name,
            TaskSpaceNestedAction::Custom(action) => &action.tool_name,
        };
        if name.trim().is_empty() {
            return invalid("nested action tool_name cannot be empty");
        }
        if matches!(name.as_str(), "taskspace_control" | "update_plan") {
            return invalid("nested actions cannot call taskspace_control or update_plan");
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

fn non_empty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, FunctionCallError> {
    Err(invalid_error(message.into()))
}

fn invalid_error(message: String) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!(
        "{message}\nTaskSpaceGateRecoveryV1: {{\"schema_version\":\"TaskSpaceGateRecoveryV1\",\"allowed\":false,\"gate_class\":\"protocol\",\"reason\":\"invalid_arguments\"}}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_actions_and_legacy_finish() {
        let empty = r#"{"action":"finish_then_actions","finishes":[],"actions":[]}"#;
        assert!(parse_taskspace_control_args(empty).is_err());
        let legacy = r#"{"action":"finish_node","result_summary":"done"}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
    }

    #[test]
    fn accepts_complete_finish_continuation() {
        let args = parse_taskspace_control_args(
            r#"{"action":"finish_then_actions","finishes":[{"result_summary":"done","next_node_id":"node-2"}],"actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}"#,
        )
        .expect("valid args");
        assert_eq!(args.nested_actions().len(), 1);
    }

    #[test]
    fn rejects_recursive_or_ambiguous_continuation() {
        let recursive = r#"{"action":"finish_then_actions","finishes":[{"result_summary":"done","next_node_id":"node-2"}],"actions":[{"tool_name":"taskspace_control","arguments":{}}]}"#;
        assert!(parse_taskspace_control_args(recursive).is_err());
        let ambiguous = r#"{"action":"finish_then_actions","finishes":[{"result_summary":"done","next_node_id":"node-2","next_node_kind":"smoke_test","next_node_title":"Test","next_node_context_summary":"Run tests"}],"actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}"#;
        assert!(parse_taskspace_control_args(ambiguous).is_err());
    }
}
