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

fn initial_node_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_key".to_string(),
                JsonSchema::string(Some(
                    "Key used by dependencies and current_node_key.".into(),
                )),
            ),
            (
                "kind".to_string(),
                JsonSchema::string_enum(node_kind_values(), Some("Node type.".into())),
            ),
            (
                "title".to_string(),
                JsonSchema::string(Some("Agent-authored node title.".into())),
            ),
            (
                "context_summary".to_string(),
                JsonSchema::string(Some("Agent-authored node context.".into())),
            ),
            (
                "dependency_keys".to_string(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Keys of prerequisite nodes.".into()),
                ),
            ),
        ]),
        Some(vec![
            "node_key".into(),
            "kind".into(),
            "title".into(),
            "context_summary".into(),
        ]),
        Some(false.into()),
    )
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "action".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("initialize_map"),
                    json!("create_node"),
                    json!("bind_node"),
                    json!("finish_node"),
                    json!("block_node"),
                    json!("read_output_ref"),
                ],
                Some("Mechanical TaskSpace map operation.".into()),
            ),
        ),
        (
            "task_title".to_string(),
            JsonSchema::string(Some("Required for initialize_map.".into())),
        ),
        (
            "task_objective".to_string(),
            JsonSchema::string(Some("Required for initialize_map.".into())),
        ),
        (
            "initial_nodes".to_string(),
            JsonSchema::array(
                initial_node_schema(),
                Some("Required non-empty node graph for initialize_map.".into()),
            ),
        ),
        (
            "current_node_key".to_string(),
            JsonSchema::string(Some(
                "Required for initialize_map. The referenced initial node is bound as current in the same state transition."
                    .into(),
            )),
        ),
        (
            "kind".to_string(),
            JsonSchema::string_enum(node_kind_values(), Some("Required for create_node.".into())),
        ),
        (
            "title".to_string(),
            JsonSchema::string(Some("Required for create_node.".into())),
        ),
        (
            "context_summary".to_string(),
            JsonSchema::string(Some("Required for create_node.".into())),
        ),
        (
            "dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(None),
                Some("Existing prerequisite node ids for create_node.".into()),
            ),
        ),
        (
            "bind_current".to_string(),
            JsonSchema::boolean(Some(
                "Optional for create_node. When true, node creation and current binding commit atomically."
                    .into(),
            )),
        ),
        (
            "node_id".to_string(),
            JsonSchema::string(Some(
                "Required for bind_node and block_node. Optional for finish_node, which defaults to the current binding. With no current binding, an explicit ready finish target is claimed and finished atomically."
                    .into(),
            )),
        ),
        (
            "result_summary".to_string(),
            JsonSchema::string(Some("Agent-authored summary for finish_node.".into())),
        ),
        (
            "final_candidate".to_string(),
            JsonSchema::string(Some(
                "Optional Agent-authored final answer for a terminal finish_node; it is released unchanged only when the finish and final lifecycle gate succeed."
                    .into(),
            )),
        ),
        (
            "next_node_id".to_string(),
            JsonSchema::string(Some(
                "Optional for finish_node. Finishing the current node and binding this existing next node commit atomically."
                    .into(),
            )),
        ),
        (
            "next_node_kind".to_string(),
            JsonSchema::string_enum(
                node_kind_values(),
                Some("Type of an atomically created next node.".into()),
            ),
        ),
        (
            "next_node_title".to_string(),
            JsonSchema::string(Some("Title of an atomically created next node.".into())),
        ),
        (
            "next_node_context_summary".to_string(),
            JsonSchema::string(Some("Context of an atomically created next node.".into())),
        ),
        (
            "next_dependency_node_ids".to_string(),
            JsonSchema::array(
                JsonSchema::string(None),
                Some("Prerequisites of an atomically created next node.".into()),
            ),
        ),
        (
            "blocker_summary".to_string(),
            JsonSchema::string(Some("Agent-authored blocker for block_node.".into())),
        ),
        (
            "output_ref".to_string(),
            JsonSchema::string(Some("OutputReferenceV1 id for read_output_ref.".into())),
        ),
        (
            "mode".to_string(),
            JsonSchema::string_enum(
                vec![
                    json!("head"),
                    json!("tail"),
                    json!("line_range"),
                    json!("grep"),
                ],
                Some("Slice mode for read_output_ref.".into()),
            ),
        ),
        (
            "start_line".to_string(),
            JsonSchema::integer(Some("1-based inclusive start line.".into())),
        ),
        (
            "end_line".to_string(),
            JsonSchema::integer(Some("1-based inclusive end line.".into())),
        ),
        (
            "pattern".to_string(),
            JsonSchema::string(Some("Literal grep pattern.".into())),
        ),
        (
            "max_bytes".to_string(),
            JsonSchema::integer(Some("Bounded output byte limit.".into())),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: r#"Mandatory mechanical map tool used while TaskSpace is enabled.

The Agent owns task semantics and explicitly initializes the map, creates or binds nodes, and records node completion or blockage. Runtime only validates ids, dependencies, status, bindings, leases, and tool/result pairing.

Mechanical action effects:
- initialize_map creates the supplied graph and binds current_node_key in one state transition.
- create_node can create and bind a node atomically with bind_current=true.
- bind_node binds an existing node as current.
- finish_node defaults node_id to the current binding. With no current binding, an explicit ready node_id is claimed and finished atomically. It can finish and bind next_node_id atomically, or atomically create and bind a next node with the next_node_* fields.
- block_node records the blocker and blocks the named node.
- read_output_ref returns a bounded slice of retained tool output.

Ordinary tools require a current node binding. One assistant response may contain any ordered sequence of TaskSpace control and ordinary tool calls, including multiple finish_node calls. Runtime serializes control barriers in provider order; every later call observes state committed by earlier barriers, and later calls are skipped if an earlier barrier fails. Prefer chaining a finish_node without a non-empty final_candidate to another call in the same response. Establish the next binding with next_node_id or next_node_* fields, or follow finish_node immediately with bind_node or create_node with bind_current=true before ordinary work. A standalone nonterminal finish_node remains valid and is only recorded as a cadence inefficiency. A terminal finish_node with final_candidate may be the last call. Runtime does not choose the next action or reinterpret tool feedback."#
            .into(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["action".into()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskspace_control_schema_is_map_lifecycle_only() {
        let value = serde_json::to_value(create_taskspace_control_tool()).expect("serialize");
        let description = value["description"].as_str().expect("description");
        assert!(description.contains("including multiple finish_node calls"));
        assert!(description.contains("every later call observes state committed"));
        assert!(description.contains("Prefer chaining a finish_node"));
        assert!(description.contains("Establish the next binding with next_node_id"));
        assert!(description.contains("standalone nonterminal finish_node remains valid"));
        assert!(description.contains("terminal finish_node with final_candidate may be the last"));
        assert!(description.contains("binds current_node_key in one state transition"));
        assert!(description.contains("finish and bind next_node_id atomically"));
        assert!(description.contains("explicit ready node_id is claimed and finished atomically"));
        let actions = value["parameters"]["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum");
        assert_eq!(
            actions,
            &vec![
                json!("initialize_map"),
                json!("create_node"),
                json!("bind_node"),
                json!("finish_node"),
                json!("block_node"),
                json!("read_output_ref"),
            ]
        );
        let properties = value["parameters"]["properties"]
            .as_object()
            .expect("properties");
        for removed in [
            "schema_version",
            "success_criteria",
            "facts",
            "decisions",
            "result_validities",
            "next_best_action",
        ] {
            assert!(!properties.contains_key(removed));
        }
        assert!(properties.contains_key("final_candidate"));
        assert!(
            properties["current_node_key"]["description"]
                .as_str()
                .expect("current_node_key description")
                .contains("bound as current in the same state transition")
        );
        assert!(
            properties["node_id"]["description"]
                .as_str()
                .expect("node_id description")
                .contains("defaults to the current binding")
        );
        assert!(
            properties["node_id"]["description"]
                .as_str()
                .expect("node_id description")
                .contains("claimed and finished atomically")
        );
        assert!(
            properties["next_node_id"]["description"]
                .as_str()
                .expect("next_node_id description")
                .contains("commit atomically")
        );
    }
}
