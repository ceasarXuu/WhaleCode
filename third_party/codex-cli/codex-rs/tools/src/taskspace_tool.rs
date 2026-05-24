use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;

pub fn create_taskspace_control_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "action".to_string(),
            JsonSchema::string(Some(
                "One of: create_node, bind_node. Use only for TaskSpace runtime control."
                    .to_string(),
            )),
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
                "For create_node, bind the main agent to the new node immediately.".to_string(),
            )),
        ),
        (
            "node_id".to_string(),
            JsonSchema::string(Some(
                "Required for bind_node. Existing node id to bind as the main action node."
                    .to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".to_string(),
        description: r#"Internal TaskSpace control tool.

Use this only when TaskSpace is enabled and you need to update task-map structure before ordinary work.

Supported actions:
- `create_node`: create a concrete node in the active task path.
- `bind_node`: bind the main agent's next ordinary action to an existing non-pending node.

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
