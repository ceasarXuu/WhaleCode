use crate::function_tool::FunctionCallError;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskSpaceControlArgs {
    InitializeThenActions {
        initial_nodes: Vec<TaskSpaceInitializeNodeArgs>,
        current_node_id: String,
        actions: Vec<TaskSpaceNestedAction>,
    },
    FinishNodes {
        finishes: Vec<TaskSpaceNonterminalFinishArgs>,
    },
    FinishThenEnd {
        #[serde(default)]
        preceding_finishes: Vec<TaskSpaceNonterminalFinishArgs>,
        terminal_finish: TaskSpaceTerminalFinishArgs,
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
    #[serde(default)]
    pub(crate) next_node_id: Option<String>,
    #[serde(default)]
    pub(crate) next_node_kind: Option<String>,
    #[serde(default)]
    pub(crate) next_node_goal: Option<String>,
    #[serde(default)]
    pub(crate) next_dependency_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskSpaceTerminalFinishArgs {
    #[serde(default)]
    pub(crate) node_id: Option<String>,
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
            Self::InitializeThenActions { actions, .. } => actions,
            Self::FinishNodes { .. }
            | Self::FinishThenEnd { .. }
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
                if initial_nodes.iter().any(|node| node.goal.trim().is_empty()) {
                    return invalid("each initial node requires a non-empty goal");
                }
                require_non_empty(actions, "actions")?;
                validate_nested_actions(actions)
            }
            Self::FinishNodes { finishes } => {
                require_non_empty(finishes, "finishes")?;
                for finish in finishes {
                    finish.validate()?;
                }
                Ok(())
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

impl TaskSpaceNonterminalFinishArgs {
    fn validate(&self) -> Result<(), FunctionCallError> {
        let has_existing = non_empty(self.next_node_id.as_deref());
        let draft_fields = [
            self.next_node_kind.as_deref(),
            self.next_node_goal.as_deref(),
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
            return invalid("next-node creation requires next_node_kind and next_node_goal");
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
    FunctionCallError::RespondToModel(
        serde_json::json!({
            "schema_version": "TaskSpaceControlResultV1",
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
            r#"{"action":"finish_nodes","finishes":[{"next_node_id":"node-2"}]}"#,
        )
        .expect("valid args");
        assert!(args.nested_actions().is_empty());
    }

    #[test]
    fn accepts_initialization_without_task_goal() {
        let args = parse_taskspace_control_args(
            r#"{"action":"initialize_then_actions","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Inspect"}],"current_node_id":"node-1","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}"#,
        )
        .expect("valid args");
        assert_eq!(args.nested_actions().len(), 1);
    }

    #[test]
    fn rejects_removed_verbose_map_fields() {
        let legacy = r#"{"action":"initialize_then_actions","task_goal":"goal","initial_nodes":[{"node_id":"node-1","kind":"inspect_code_context","goal":"Inspect"}],"current_node_id":"node-1","actions":[{"tool_name":"exec_command","arguments":{"cmd":"pwd"}}]}"#;
        assert!(parse_taskspace_control_args(legacy).is_err());
    }

    #[test]
    fn rejects_removed_finish_result_summary() {
        let nonterminal = r#"{"action":"finish_nodes","finishes":[{"result_summary":"done","next_node_id":"node-2"}]}"#;
        assert!(parse_taskspace_control_args(nonterminal).is_err());
        let terminal = r#"{"action":"finish_then_end","terminal_finish":{"result_summary":"done"},"final_candidate":"answer"}"#;
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
    fn rejects_ambiguous_finish_binding() {
        let ambiguous = r#"{"action":"finish_nodes","finishes":[{"next_node_id":"node-2","next_node_kind":"smoke_test","next_node_goal":"Run tests"}]}"#;
        assert!(parse_taskspace_control_args(ambiguous).is_err());
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
                "schema_version": "TaskSpaceControlResultV1",
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
