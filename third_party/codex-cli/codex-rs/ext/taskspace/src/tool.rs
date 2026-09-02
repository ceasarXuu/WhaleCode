use std::collections::BTreeMap;
use std::sync::Arc;

use codex_tools::FunctionCallError;
use codex_tools::JsonSchema;
use codex_tools::JsonToolOutput;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolCall;
use codex_tools::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolOutput;
use codex_tools::ToolSpec;

use crate::runtime::TaskSpaceRuntimeHandle;

pub(crate) const TASKSPACE_CONTROL_TOOL: &str = "taskspace_control";

pub(crate) struct ReadTaskSpaceTool {
    runtime: Arc<TaskSpaceRuntimeHandle>,
}

impl ReadTaskSpaceTool {
    pub(crate) fn new(runtime: Arc<TaskSpaceRuntimeHandle>) -> Self {
        Self { runtime }
    }
}

impl<'call> ToolExecutor<ToolCall<'call>> for ReadTaskSpaceTool {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(TASKSPACE_CONTROL_TOOL)
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: TASKSPACE_CONTROL_TOOL.into(),
            description: "Read or atomically advance the canonical TaskSpace Map. For execute, this call must be first and actions must match every following tool call in order.".into(),
            strict: false,
            defer_loading: None,
            parameters: control_schema(),
            output_schema: None,
        })
    }

    fn handle<'a>(&'a self, invocation: ToolCall<'call>) -> codex_tools::ToolExecutorFuture<'a>
    where
        'call: 'a,
    {
        Box::pin(async move {
            let arguments: serde_json::Value =
                serde_json::from_str(invocation.function_arguments()?).map_err(|error| {
                    FunctionCallError::RespondToModel(format!("invalid TaskSpace input: {error}"))
                })?;
            match arguments.get("action").and_then(|value| value.as_str()) {
                Some("read_map") => {
                    let record = self.runtime.refresh().await.map_err(|error| {
                        FunctionCallError::RespondToModel(format!(
                            "failed to read TaskSpace Map: {error}"
                        ))
                    })?;
                    Ok(Box::new(JsonToolOutput::new(serde_json::json!({
                        "schemaVersion": "TaskSpaceControlResultV2",
                        "stateCommit": false,
                        "map": record.map(|value| value.map),
                    }))) as Box<dyn ToolOutput>)
                }
                Some("execute" | "initialize_and_execute" | "finish_map" | "reopen_map") => {
                    let prepared = self
                        .runtime
                        .prepared_control(&invocation.call_id)
                        .await
                        .ok_or_else(|| {
                            FunctionCallError::RespondToModel(
                                "TaskSpace control was not committed during response preflight"
                                    .into(),
                            )
                        })?;
                    Ok(Box::new(JsonToolOutput::new(serde_json::json!({
                        "schemaVersion": "TaskSpaceResponseCommitV1",
                        "stateCommit": true,
                        "mapId": prepared.map_id,
                        "action": prepared.action,
                        "revisionBefore": prepared.revision_before,
                        "revisionAfter": prepared.revision_after,
                        "reservedActions": prepared.actions.iter().map(|action| serde_json::json!({
                            "callId": action.call_id,
                            "nodeId": action.node_id,
                            "tool": action.tool_name,
                            "reservationId": action.reservation_id,
                        })).collect::<Vec<_>>(),
                    }))) as Box<dyn ToolOutput>)
                }
                _ => Err(FunctionCallError::RespondToModel(
                    "unsupported taskspace_control action".into(),
                )),
            }
        })
    }
}

fn control_schema() -> JsonSchema {
    let read = JsonSchema::object(
        BTreeMap::from([(
            "action".into(),
            JsonSchema::string_enum(vec![serde_json::json!("read_map")], None),
        )]),
        Some(vec!["action".into()]),
        Some(false.into()),
    );
    let action = JsonSchema::object(
        BTreeMap::from([
            ("node_id".into(), JsonSchema::string(None)),
            ("tool".into(), JsonSchema::string(None)),
        ]),
        Some(vec!["node_id".into(), "tool".into()]),
        Some(false.into()),
    );
    let node = JsonSchema::object(
        BTreeMap::from([
            ("node_id".into(), JsonSchema::string(None)),
            ("goal".into(), JsonSchema::string(None)),
        ]),
        Some(vec!["node_id".into(), "goal".into()]),
        Some(false.into()),
    );
    let edge = JsonSchema::object(
        BTreeMap::from([
            ("from".into(), JsonSchema::string(None)),
            ("to".into(), JsonSchema::string(None)),
        ]),
        Some(vec!["from".into(), "to".into()]),
        Some(false.into()),
    );
    let initialize = JsonSchema::object(
        BTreeMap::from([
            (
                "action".into(),
                JsonSchema::string_enum(vec![serde_json::json!("initialize_and_execute")], None),
            ),
            ("root".into(), node.clone()),
            ("work_nodes".into(), JsonSchema::array(node.clone(), None)),
            ("finish".into(), node.clone()),
            ("edges".into(), JsonSchema::array(edge.clone(), None)),
            ("actions".into(), JsonSchema::array(action.clone(), None)),
        ]),
        Some(vec![
            "action".into(),
            "root".into(),
            "work_nodes".into(),
            "finish".into(),
            "edges".into(),
            "actions".into(),
        ]),
        Some(false.into()),
    );
    let mutation = JsonSchema::object(
        BTreeMap::from([
            (
                "action".into(),
                JsonSchema::string_enum(
                    [
                        "add_work_nodes",
                        "add_edges",
                        "remove_edges",
                        "complete_node",
                        "block_node",
                        "unblock_node",
                    ]
                    .into_iter()
                    .map(serde_json::Value::from)
                    .collect(),
                    None,
                ),
            ),
            ("node_id".into(), JsonSchema::string(None)),
            (
                "work_nodes".into(),
                JsonSchema::array(
                    JsonSchema::object(Default::default(), None, Some(true.into())),
                    None,
                ),
            ),
            (
                "edges".into(),
                JsonSchema::array(
                    JsonSchema::object(Default::default(), None, Some(true.into())),
                    None,
                ),
            ),
        ]),
        Some(vec!["action".into()]),
        Some(false.into()),
    );
    let execute = JsonSchema::object(
        BTreeMap::from([
            (
                "action".into(),
                JsonSchema::string_enum(vec![serde_json::json!("execute")], None),
            ),
            ("expected_revision".into(), JsonSchema::integer(None)),
            ("mutations".into(), JsonSchema::array(mutation, None)),
            ("actions".into(), JsonSchema::array(action.clone(), None)),
        ]),
        Some(vec![
            "action".into(),
            "expected_revision".into(),
            "actions".into(),
        ]),
        Some(false.into()),
    );
    let finish = JsonSchema::object(
        BTreeMap::from([
            (
                "action".into(),
                JsonSchema::string_enum(vec![serde_json::json!("finish_map")], None),
            ),
            ("expected_revision".into(), JsonSchema::integer(None)),
            ("finish_node_id".into(), JsonSchema::string(None)),
            (
                "complete_work_node_ids".into(),
                JsonSchema::array(JsonSchema::string(None), None),
            ),
            ("exact_summary".into(), JsonSchema::string(None)),
        ]),
        Some(vec![
            "action".into(),
            "expected_revision".into(),
            "finish_node_id".into(),
            "complete_work_node_ids".into(),
            "exact_summary".into(),
        ]),
        Some(false.into()),
    );
    let reopen = JsonSchema::object(
        BTreeMap::from([
            (
                "action".into(),
                JsonSchema::string_enum(vec![serde_json::json!("reopen_map")], None),
            ),
            ("expected_revision".into(), JsonSchema::integer(None)),
            ("work_nodes".into(), JsonSchema::array(node, None)),
            ("edges".into(), JsonSchema::array(edge, None)),
            ("actions".into(), JsonSchema::array(action, None)),
        ]),
        Some(vec![
            "action".into(),
            "expected_revision".into(),
            "work_nodes".into(),
            "edges".into(),
            "actions".into(),
        ]),
        Some(false.into()),
    );
    JsonSchema::any_of(vec![read, initialize, execute, finish, reopen], None)
}
