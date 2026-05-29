use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

fn node_kind_values() -> Vec<serde_json::Value> {
    vec![
        json!("inspect_code_context"),
        json!("implement_solution"),
        json!("smoke_test"),
        json!("regression_test"),
        json!("final_synthesis"),
    ]
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "action".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("start_task"),
                    json!("route_task"),
                    json!("create_node"),
                    json!("bind_node"),
                    json!("finish_node"),
                    json!("block_node"),
                ],
                Some(
                "One of: start_task, route_task, create_node, bind_node, finish_node, block_node. Use only for TaskSpace runtime control."
                    .to_string(),
                ),
            ),
        ),
        (
            "task_id".to_string(),
            JsonSchema::string(Some(
                "Required for route_task. Existing task id from the TaskSpace task inventory."
                    .to_string(),
            )),
        ),
        (
            "task_title".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Human-readable title for the new semantic task."
                    .to_string(),
            )),
        ),
        (
            "task_objective".to_string(),
            JsonSchema::string(Some(
                "Optional for start_task. Concise objective for the new semantic task."
                    .to_string(),
            )),
        ),
        (
            "node_kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some(
                    "Required for start_task. Runtime kind for the first node."
                        .to_string(),
                ),
            ),
        ),
        (
            "node_title".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Human-readable title for the first concrete node."
                    .to_string(),
            )),
        ),
        (
            "node_context_summary".to_string(),
            JsonSchema::string(Some(
                "Required for start_task. Concise context the first node should carry."
                    .to_string(),
            )),
        ),
        (
            "kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some("Required for create_node. Runtime kind for this node.".to_string()),
            ),
        ),
        (
            "title".to_string(),
            JsonSchema::string(Some(
                "Required for create_node. Human-readable node title.".to_string(),
            )),
        ),
        (
            "context_summary".to_string(),
            JsonSchema::string(Some(
                "Required for create_node. Concise context the node should carry.".to_string(),
            )),
        ),
        (
            "dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing upstream node id.".to_string())),
                Some("Optional dependency node ids for create_node.".to_string()),
            ),
        ),
        (
            "bind_current".to_string(),
            JsonSchema::boolean(Some(
                "For start_task or create_node, bind the main agent to the new node immediately."
                    .to_string(),
            )),
        ),
        (
            "node_id".to_string(),
            JsonSchema::string(Some(
                "Required for bind_node, finish_node, and block_node. Existing node id."
                    .to_string(),
            )),
        ),
        (
            "result_summary".to_string(),
            JsonSchema::string(Some(
                "Required for finish_node. Concise result summary that should stay in the node context."
                    .to_string(),
            )),
        ),
        (
            "next_node_id".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node. Existing node id to bind after the result is recorded."
                    .to_string(),
            )),
        ),
        (
            "next_node_kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some(
                    "Optional for finish_node. When provided with next_node_title and next_node_context_summary, create and bind a new next node atomically."
                        .to_string(),
                ),
            ),
        ),
        (
            "next_node_title".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node with next_node_kind. Human-readable title for the new next node."
                    .to_string(),
            )),
        ),
        (
            "next_node_context_summary".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node with next_node_kind. Concise context for the new next node."
                    .to_string(),
            )),
        ),
        (
            "next_dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Existing upstream node id.".to_string())),
                Some(
                    "Optional for finish_node with next_node_kind. Dependency node ids for the new next node."
                        .to_string(),
                ),
            ),
        ),
        (
            "blocker_summary".to_string(),
            JsonSchema::string(Some(
                "Required for block_node. Concise blocker summary that should stay in the node context."
                    .to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".to_string(),
        description: r#"Internal TaskSpace control tool.

Use this only when TaskSpace is enabled and you need to update task-map structure before ordinary work.

Supported actions:
- `start_task`: create a new semantic task, its active task path, and the first concrete node. Use this when the current user request does not belong to an existing task in the TaskSpace task inventory. Must include `node_kind`.
- `route_task`: switch the active task path to an existing task chosen by the agent from the TaskSpace task inventory. Runtime validates the id but does not perform semantic matching.
- `create_node`: create a concrete node in the active task path. This requires an existing active task path; use `start_task` first when the current request starts a new semantic task. Must include `kind`. BaseMap candidate nodes are guidance, not automatic graph nodes.
- `bind_node`: bind the main agent's next ordinary action to an existing ready or blocked node that is not held by a subagent.
- `finish_node`: record the current main node's result, mark it completed, and optionally bind an existing next node with `next_node_id` or create and bind a new next node with `next_node_kind`, `next_node_title`, and `next_node_context_summary`.
- `block_node`: record why the current main node cannot proceed and mark it blocked.

Node kind selection:
- Use `inspect_code_context` for read-only investigation and subagent investigation nodes.
- Use `implement_solution` for code, test, configuration, or documentation edits.
- Use `smoke_test` or `regression_test` before running test/build/lint commands.
- If validation fails and edits are needed, record the test result, switch to `implement_solution` for the fix, then switch back to a test node for the rerun.
- Use `final_synthesis` only for final wrap-up.
- `custom` is reserved for restored legacy nodes and is not valid for live node creation.

Do not expose this tool's internal map/node terminology to the user unless debugging TaskSpace itself.
"#
        .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["action".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}
