use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;

pub struct TaskSpaceControlHandler;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TaskSpaceControlArgs {
    StartTask {
        task_title: String,
        #[serde(default)]
        task_objective: String,
        node_title: String,
        node_context_summary: String,
        #[serde(default)]
        bind_current: bool,
    },
    RouteTask {
        task_id: String,
    },
    CreateNode {
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
    FinishNode {
        node_id: String,
        result_summary: String,
        #[serde(default)]
        next_node_id: Option<String>,
    },
    BlockNode {
        node_id: String,
        blocker_summary: String,
    },
}

pub struct TaskSpaceControlOutput {
    message: String,
}

impl ToolOutput for TaskSpaceControlOutput {
    fn log_preview(&self) -> String {
        self.message.clone()
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, _payload: &ToolPayload) -> ResponseInputItem {
        let mut output = FunctionCallOutputPayload::from_text(self.message.clone());
        output.success = Some(true);
        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output,
        }
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        JsonValue::String(self.message.clone())
    }
}

impl ToolHandler for TaskSpaceControlHandler {
    type Output = TaskSpaceControlOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "taskspace_control handler received unsupported payload".to_string(),
                ));
            }
        };
        let args: TaskSpaceControlArgs = parse_arguments(&arguments)?;
        let message = match args {
            TaskSpaceControlArgs::StartTask {
                task_title,
                task_objective,
                node_title,
                node_context_summary,
                bind_current,
            } => {
                let (task_id, map_id, node_id) = session
                    .start_action_map_task_for_main(
                        &turn,
                        task_title,
                        task_objective,
                        node_title,
                        node_context_summary,
                        bind_current,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if bind_current {
                    format!(
                        "TaskSpace task started and bound: task={task_id} map={map_id} node={node_id}"
                    )
                } else {
                    format!("TaskSpace task started: task={task_id} map={map_id} node={node_id}")
                }
            }
            TaskSpaceControlArgs::RouteTask { task_id } => {
                session
                    .route_action_map_task_for_main(&turn, &task_id)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace task routed: {task_id}")
            }
            TaskSpaceControlArgs::CreateNode {
                title,
                context_summary,
                dependency_node_ids,
                bind_current,
            } => {
                let node_id = session
                    .create_action_map_node_for_main(
                        &turn,
                        title,
                        context_summary,
                        dependency_node_ids,
                        bind_current,
                    )
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if bind_current {
                    format!("TaskSpace node created and bound: {node_id}")
                } else {
                    format!("TaskSpace node created: {node_id}")
                }
            }
            TaskSpaceControlArgs::BindNode { node_id } => {
                session
                    .bind_action_map_main_node(&turn, &node_id)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace main node bound: {node_id}")
            }
            TaskSpaceControlArgs::FinishNode {
                node_id,
                result_summary,
                next_node_id,
            } => {
                let result_id = session
                    .finish_action_map_main_node(&turn, &node_id, result_summary, next_node_id)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace node finished: {node_id} result {result_id}")
            }
            TaskSpaceControlArgs::BlockNode {
                node_id,
                blocker_summary,
            } => {
                let result_id = session
                    .block_action_map_main_node(&turn, &node_id, blocker_summary)
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                format!("TaskSpace node blocked: {node_id} result {result_id}")
            }
        };
        Ok(TaskSpaceControlOutput { message })
    }
}
