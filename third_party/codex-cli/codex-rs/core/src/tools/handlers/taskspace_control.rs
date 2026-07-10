use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::action_map::ActionMapInitializeInput;
use crate::action_map::ActionMapInitializeNodeInput;
use crate::action_map::ActionMapNextNodeDraft;
use crate::action_map::NodeKind;
use crate::action_map::TaskSpaceHardGateClass;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::output_reference::OUTPUT_SLICE_MAX_BYTES;
use crate::tools::output_reference::OutputSliceMode;
use crate::tools::output_reference::OutputSliceRequest;
use crate::tools::output_reference::read_output_artifact_slice;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct TaskSpaceControlHandler;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TaskSpaceControlArgs {
    InitializeMap {
        task_title: String,
        task_objective: String,
        initial_nodes: Vec<TaskSpaceInitializeNodeArgs>,
        current_node_key: String,
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
    FinishNode {
        #[serde(default)]
        node_id: Option<String>,
        result_summary: String,
        #[serde(default)]
        next_node_id: Option<String>,
        #[serde(default)]
        next_node_kind: Option<String>,
        #[serde(default)]
        next_node_title: Option<String>,
        #[serde(default)]
        next_node_context_summary: Option<String>,
        #[serde(default)]
        next_dependency_node_ids: Vec<String>,
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

#[derive(Debug, Deserialize)]
struct TaskSpaceInitializeNodeArgs {
    node_key: String,
    kind: String,
    title: String,
    context_summary: String,
    #[serde(default)]
    dependency_keys: Vec<String>,
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
                return Err(protocol_error(
                    "taskspace_control received unsupported payload".into(),
                    "unsupported_payload",
                ));
            }
        };
        let args: TaskSpaceControlArgs = parse_arguments(&arguments)
            .map_err(|error| protocol_error(error.to_string(), "invalid_arguments"))?;

        let message = match args {
            TaskSpaceControlArgs::InitializeMap {
                task_title,
                task_objective,
                initial_nodes,
                current_node_key,
            } => {
                let nodes = initial_nodes
                    .into_iter()
                    .map(|node| {
                        Ok(ActionMapInitializeNodeInput {
                            key: node.node_key,
                            kind: parse_node_kind("initial_nodes.kind", &node.kind)?,
                            title: node.title,
                            context_summary: node.context_summary,
                            dependency_keys: node.dependency_keys,
                        })
                    })
                    .collect::<Result<Vec<_>, FunctionCallError>>()?;
                let outcome = session
                    .initialize_action_map_for_main(
                        &turn,
                        ActionMapInitializeInput {
                            task_title,
                            task_objective,
                            nodes,
                            current_node_key,
                        },
                    )
                    .await
                    .map_err(state_machine_error)?;
                let mappings = outcome
                    .node_ids
                    .iter()
                    .map(|(key, node_id)| format!("{key}={node_id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "TaskSpace map initialized: task={} map={} current_node={} node_ids=[{}]",
                    outcome.task_id, outcome.map_id, outcome.current_node_id, mappings
                )
            }
            TaskSpaceControlArgs::CreateNode {
                kind,
                title,
                context_summary,
                dependency_node_ids,
                bind_current,
            } => {
                let node_id = session
                    .create_action_map_node_for_main_with_kind(
                        &turn,
                        parse_node_kind("kind", &kind)?,
                        title,
                        context_summary,
                        dependency_node_ids,
                        bind_current,
                    )
                    .await
                    .map_err(state_machine_error)?;
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
                    .map_err(state_machine_error)?;
                format!("TaskSpace main node bound: {node_id}")
            }
            TaskSpaceControlArgs::FinishNode {
                node_id,
                result_summary,
                next_node_id,
                next_node_kind,
                next_node_title,
                next_node_context_summary,
                next_dependency_node_ids,
            } => {
                let draft = build_next_node_draft(
                    next_node_kind,
                    next_node_title,
                    next_node_context_summary,
                    next_dependency_node_ids,
                )?;
                let (node_id, outcome) = session
                    .finish_action_map_current_or_named_node_with_next(
                        &turn,
                        node_id.as_deref(),
                        result_summary,
                        next_node_id,
                        draft,
                    )
                    .await
                    .map_err(state_machine_error)?;
                match outcome.next_node_id {
                    Some(next) => format!(
                        "TaskSpace node finished: {node_id} result {} next_node={next}",
                        outcome.result_id
                    ),
                    None => format!(
                        "TaskSpace node finished: {node_id} result {}",
                        outcome.result_id
                    ),
                }
            }
            TaskSpaceControlArgs::BlockNode {
                node_id,
                blocker_summary,
            } => {
                let result_id = session
                    .block_action_map_main_node(&turn, &node_id, blocker_summary)
                    .await
                    .map_err(state_machine_error)?;
                format!("TaskSpace node blocked: {node_id} result {result_id}")
            }
            TaskSpaceControlArgs::ReadOutputRef {
                output_ref,
                mode,
                start_line,
                end_line,
                pattern,
                max_bytes,
            } => {
                let request = OutputSliceRequest {
                    mode: parse_output_slice_mode(&mode, start_line, end_line, pattern)?,
                    max_bytes: max_bytes.unwrap_or(OUTPUT_SLICE_MAX_BYTES),
                };
                let rollout_path = session.current_rollout_path().await.map_err(|error| {
                    resource_error(error.to_string(), "output_reference_store_unavailable")
                })?;
                let slice =
                    read_output_artifact_slice(rollout_path.as_deref(), &output_ref, request)
                        .await
                        .map_err(|error| {
                            resource_error(error.to_string(), "output_reference_read_failed")
                        })?;
                session
                    .record_action_map_output_ref_trace_event(
                        &turn,
                        "output_ref.slice_read",
                        None,
                        output_ref,
                        vec![
                            "output_ref".into(),
                            "slice_read".into(),
                            format!("mode:{mode}"),
                        ],
                    )
                    .await;
                slice
            }
        };
        Ok(TaskSpaceControlOutput { message })
    }
}

fn parse_output_slice_mode(
    mode: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    pattern: Option<String>,
) -> Result<OutputSliceMode, FunctionCallError> {
    match mode {
        "head" => Ok(OutputSliceMode::Head),
        "tail" => Ok(OutputSliceMode::Tail),
        "line_range" => Ok(OutputSliceMode::LineRange {
            start_line: start_line.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires start_line".into(),
                    "missing_argument",
                )
            })?,
            end_line: end_line.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires end_line".into(),
                    "missing_argument",
                )
            })?,
        }),
        "grep" => Ok(OutputSliceMode::Grep {
            pattern: pattern.ok_or_else(|| {
                protocol_error(
                    "read_output_ref requires pattern".into(),
                    "missing_argument",
                )
            })?,
        }),
        _ => Err(protocol_error(
            "read_output_ref mode must be head, tail, line_range, or grep".into(),
            "invalid_argument_value",
        )),
    }
}

fn parse_node_kind(field: &str, value: &str) -> Result<NodeKind, FunctionCallError> {
    NodeKind::from_str(value).ok_or_else(|| {
        protocol_error(
            format!("taskspace_control {field} has invalid node kind `{value}`"),
            "invalid_argument_value",
        )
    })
}

fn build_next_node_draft(
    kind: Option<String>,
    title: Option<String>,
    context_summary: Option<String>,
    dependency_node_ids: Vec<String>,
) -> Result<Option<ActionMapNextNodeDraft>, FunctionCallError> {
    let has_any = kind
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || title
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || context_summary
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || !dependency_node_ids.is_empty();
    if !has_any {
        return Ok(None);
    }
    let kind = parse_node_kind("next_node_kind", kind.as_deref().unwrap_or_default())?;
    let title = title.unwrap_or_default();
    let context_summary = context_summary.unwrap_or_default();
    if title.trim().is_empty() || context_summary.trim().is_empty() {
        return Err(protocol_error(
            "finish_node next-node creation requires kind, title, and context".into(),
            "missing_argument",
        ));
    }
    Ok(Some(ActionMapNextNodeDraft {
        kind,
        title,
        context_summary,
        dependency_node_ids,
    }))
}

fn state_machine_error(message: String) -> FunctionCallError {
    let reason = hard_state_reason(&message)
        .unwrap_or("transition_rejected")
        .to_string();
    gate_error(message, TaskSpaceHardGateClass::StateMachine, &reason)
}

fn protocol_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Protocol, reason)
}

fn resource_error(message: String, reason: &str) -> FunctionCallError {
    gate_error(message, TaskSpaceHardGateClass::Resource, reason)
}

fn gate_error(message: String, class: TaskSpaceHardGateClass, reason: &str) -> FunctionCallError {
    let metadata = serde_json::json!({
        "schema_version": "TaskSpaceGateRecoveryV1",
        "allowed": false,
        "gate_class": class.as_str(),
        "reason": reason,
    });
    FunctionCallError::RespondToModel(format!("{message}\nTaskSpaceGateRecoveryV1: {metadata}"))
}

fn hard_state_reason(message: &str) -> Option<&str> {
    message
        .split_once("hard_state:")?
        .1
        .trim_start()
        .split(|character: char| character.is_whitespace() || ".,;".contains(character))
        .next()
        .filter(|reason| !reason.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_authored_map() {
        let args: TaskSpaceControlArgs = serde_json::from_value(serde_json::json!({
            "action": "initialize_map",
            "task_title": "Patch bug",
            "task_objective": "Fix and verify",
            "initial_nodes": [{
                "node_key": "inspect",
                "kind": "inspect_code_context",
                "title": "Inspect",
                "context_summary": "Read relevant code"
            }],
            "current_node_key": "inspect"
        }))
        .expect("parse initialize_map");
        assert!(matches!(args, TaskSpaceControlArgs::InitializeMap { .. }));
    }

    #[test]
    fn rejects_removed_semantic_action_at_parse_boundary() {
        let error = serde_json::from_value::<TaskSpaceControlArgs>(serde_json::json!({
            "action": "record_fact",
            "claim_id": "fact-1",
            "statement": "legacy"
        }))
        .expect_err("removed action");
        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn finish_without_next_node_has_no_draft() {
        assert_eq!(
            build_next_node_draft(None, None, None, Vec::new()).expect("draft"),
            None
        );
    }

    #[test]
    fn hard_state_reason_is_mechanical() {
        assert_eq!(
            hard_state_reason("blocked. hard_state: node_tool_calls_in_flight. rejected"),
            Some("node_tool_calls_in_flight")
        );
    }
}
