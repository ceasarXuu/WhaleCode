use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::Value;
use serde_json::json;

fn node_kind_values() -> Vec<Value> {
    vec![
        json!("inspect_code_context"),
        json!("implement_solution"),
        json!("smoke_test"),
        json!("regression_test"),
        json!("final_synthesis"),
    ]
}

fn action_tag(action: &str) -> JsonSchema {
    JsonSchema::string_enum(
        vec![json!(action)],
        Some("Mechanical action variant.".into()),
    )
}

fn object_variant(
    action: &str,
    mut properties: BTreeMap<String, JsonSchema>,
    mut required: Vec<String>,
) -> JsonSchema {
    properties.insert("action".into(), action_tag(action));
    required.insert(0, "action".into());
    JsonSchema::object(properties, Some(required), Some(false.into()))
}

fn object_any_of(variants: Vec<JsonSchema>, description: &str) -> JsonSchema {
    let mut schema = JsonSchema::object(BTreeMap::new(), None, None);
    schema.description = Some(description.into());
    schema.any_of = Some(variants);
    schema
}

fn function_action_schema(tool: &ResponsesApiTool, namespace: Option<&str>) -> JsonSchema {
    let mut properties = BTreeMap::from([
        (
            "tool_name".into(),
            JsonSchema::string_enum(
                vec![json!(tool.name)],
                Some("Visible ordinary tool name.".into()),
            ),
        ),
        ("arguments".into(), tool.parameters.clone()),
    ]);
    let mut required = vec!["tool_name".into(), "arguments".into()];
    if let Some(namespace) = namespace {
        properties.insert(
            "namespace".into(),
            JsonSchema::string_enum(
                vec![json!(namespace)],
                Some("Visible tool namespace.".into()),
            ),
        );
        required.insert(0, "namespace".into());
    }
    JsonSchema::object(properties, Some(required), Some(false.into()))
}

fn custom_action_schema(name: &str) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "tool_name".into(),
                JsonSchema::string_enum(
                    vec![json!(name)],
                    Some("Visible ordinary custom tool name.".into()),
                ),
            ),
            (
                "input".into(),
                JsonSchema::string(Some("Exact custom tool input.".into())),
            ),
        ]),
        Some(vec!["tool_name".into(), "input".into()]),
        Some(false.into()),
    )
}

fn nested_action_schema(visible_tools: &[ToolSpec]) -> JsonSchema {
    let mut variants = Vec::new();
    for spec in visible_tools {
        match spec {
            ToolSpec::Function(tool)
                if !matches!(tool.name.as_str(), "taskspace_control" | "update_plan") =>
            {
                variants.push((tool.name.clone(), function_action_schema(tool, None)));
            }
            ToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let ResponsesApiNamespaceTool::Function(tool) = tool;
                    variants.push((
                        format!("{}.{}", namespace.name, tool.name),
                        function_action_schema(tool, Some(&namespace.name)),
                    ));
                }
            }
            ToolSpec::Freeform(tool) => {
                variants.push((tool.name.clone(), custom_action_schema(&tool.name)));
            }
            ToolSpec::Function(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::WebSearch { .. } => {}
        }
    }
    variants.sort_by(|left, right| left.0.cmp(&right.0));
    object_any_of(
        variants.into_iter().map(|(_, schema)| schema).collect(),
        "One Agent-authored ordinary tool call visible in this request.",
    )
}

fn initial_node_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some(
                    "Stable Agent-authored node identifier used by all later actions.".into(),
                )),
            ),
            (
                "kind".into(),
                JsonSchema::string_enum(node_kind_values(), Some("Node type.".into())),
            ),
            (
                "title".into(),
                JsonSchema::string(Some("Agent-authored node title.".into())),
            ),
            (
                "context_summary".into(),
                JsonSchema::string(Some("Agent-authored node context.".into())),
            ),
            (
                "dependency_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Agent-authored prerequisite node identifiers.".into()),
                ),
            ),
        ]),
        Some(vec![
            "node_id".into(),
            "kind".into(),
            "title".into(),
            "context_summary".into(),
        ]),
        Some(false.into()),
    )
}

fn next_existing_finish_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Optional explicit ready finish target.".into())),
            ),
            (
                "result_summary".into(),
                JsonSchema::string(Some("Agent-authored node result.".into())),
            ),
            (
                "next_node_id".into(),
                JsonSchema::string(Some("Existing node bound after this finish.".into())),
            ),
        ]),
        Some(vec!["result_summary".into(), "next_node_id".into()]),
        Some(false.into()),
    )
}

fn next_created_finish_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Optional explicit ready finish target.".into())),
            ),
            (
                "result_summary".into(),
                JsonSchema::string(Some("Agent-authored node result.".into())),
            ),
            (
                "next_node_kind".into(),
                JsonSchema::string_enum(node_kind_values(), Some("Created node type.".into())),
            ),
            (
                "next_node_title".into(),
                JsonSchema::string(Some("Created node title.".into())),
            ),
            (
                "next_node_context_summary".into(),
                JsonSchema::string(Some("Created node context.".into())),
            ),
            (
                "next_dependency_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Created node prerequisites.".into()),
                ),
            ),
        ]),
        Some(vec![
            "result_summary".into(),
            "next_node_kind".into(),
            "next_node_title".into(),
            "next_node_context_summary".into(),
        ]),
        Some(false.into()),
    )
}

fn nonterminal_finish_schema() -> JsonSchema {
    object_any_of(
        vec![next_existing_finish_schema(), next_created_finish_schema()],
        "Finish one node and establish the next binding atomically.",
    )
}

fn terminal_finish_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Optional explicit ready terminal target.".into())),
            ),
            (
                "result_summary".into(),
                JsonSchema::string(Some("Agent-authored terminal node result.".into())),
            ),
        ]),
        Some(vec!["result_summary".into()]),
        Some(false.into()),
    )
}

fn initialize_then_actions_schema(actions: &JsonSchema) -> JsonSchema {
    object_variant(
        "initialize_then_actions",
        BTreeMap::from([
            (
                "task_title".into(),
                JsonSchema::string(Some("Task title.".into())),
            ),
            (
                "task_objective".into(),
                JsonSchema::string(Some("Task objective.".into())),
            ),
            (
                "initial_nodes".into(),
                JsonSchema::array(initial_node_schema(), Some("Initial node graph.".into()))
                    .with_min_items(1),
            ),
            (
                "current_node_id".into(),
                JsonSchema::string(Some("Initial node bound before actions execute.".into())),
            ),
            (
                "actions".into(),
                JsonSchema::array(actions.clone(), Some("Immediate ordinary actions.".into()))
                    .with_min_items(1),
            ),
        ]),
        vec![
            "task_title".into(),
            "task_objective".into(),
            "initial_nodes".into(),
            "current_node_id".into(),
            "actions".into(),
        ],
    )
}

fn finish_then_actions_schema(actions: &JsonSchema) -> JsonSchema {
    object_variant(
        "finish_then_actions",
        BTreeMap::from([
            (
                "finishes".into(),
                JsonSchema::array(
                    nonterminal_finish_schema(),
                    Some("Ordered nonterminal finishes.".into()),
                )
                .with_min_items(1),
            ),
            (
                "actions".into(),
                JsonSchema::array(actions.clone(), Some("Immediate ordinary actions.".into()))
                    .with_min_items(1),
            ),
        ]),
        vec!["finishes".into(), "actions".into()],
    )
}

fn finish_then_end_schema() -> JsonSchema {
    object_variant(
        "finish_then_end",
        BTreeMap::from([
            (
                "preceding_finishes".into(),
                JsonSchema::array(
                    nonterminal_finish_schema(),
                    Some("Optional ordered finishes before the terminal finish.".into()),
                ),
            ),
            ("terminal_finish".into(), terminal_finish_schema()),
            (
                "final_candidate".into(),
                JsonSchema::string(Some("Exact Agent-authored final answer.".into())),
            ),
        ]),
        vec!["terminal_finish".into(), "final_candidate".into()],
    )
}

fn simple_action_schemas() -> Vec<JsonSchema> {
    vec![
        object_variant(
            "create_node",
            BTreeMap::from([
                (
                    "kind".into(),
                    JsonSchema::string_enum(node_kind_values(), None),
                ),
                ("title".into(), JsonSchema::string(None)),
                ("context_summary".into(), JsonSchema::string(None)),
                (
                    "dependency_node_ids".into(),
                    JsonSchema::array(JsonSchema::string(None), None),
                ),
                ("bind_current".into(), JsonSchema::boolean(None)),
            ]),
            vec!["kind".into(), "title".into(), "context_summary".into()],
        ),
        object_variant(
            "bind_node",
            BTreeMap::from([("node_id".into(), JsonSchema::string(None))]),
            vec!["node_id".into()],
        ),
        object_variant(
            "block_node",
            BTreeMap::from([
                ("node_id".into(), JsonSchema::string(None)),
                ("blocker_summary".into(), JsonSchema::string(None)),
            ]),
            vec!["node_id".into(), "blocker_summary".into()],
        ),
        object_variant(
            "read_output_ref",
            BTreeMap::from([
                ("output_ref".into(), JsonSchema::string(None)),
                (
                    "mode".into(),
                    JsonSchema::string_enum(
                        vec![
                            json!("head"),
                            json!("tail"),
                            json!("line_range"),
                            json!("grep"),
                        ],
                        None,
                    ),
                ),
                ("start_line".into(), JsonSchema::integer(None)),
                ("end_line".into(), JsonSchema::integer(None)),
                ("pattern".into(), JsonSchema::string(None)),
                ("max_bytes".into(), JsonSchema::integer(None)),
            ]),
            vec!["output_ref".into(), "mode".into()],
        ),
    ]
}

pub fn create_taskspace_control_tool(visible_tools: &[ToolSpec]) -> ToolSpec {
    let actions = nested_action_schema(visible_tools);
    let mut variants = vec![
        initialize_then_actions_schema(&actions),
        finish_then_actions_schema(&actions),
        finish_then_end_schema(),
    ];
    variants.extend(simple_action_schemas());

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: r#"Mandatory mechanical TaskSpace map tool. The Agent declares every state transition, immediate ordinary action, and final answer. initialize_then_actions initializes and binds the map before executing a non-empty actions list. finish_then_actions commits one or more ordered finishes, each with an explicit next binding, before executing a non-empty actions list. finish_then_end commits optional preceding finishes and one terminal finish before releasing final_candidate unchanged. Runtime executes only the declared sequence, stops after the first failure, and does not choose or infer actions."#.into(),
        strict: false,
        defer_loading: None,
        parameters: object_any_of(variants, "Schema-first TaskSpace control operation."),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_list_dir_tool;

    #[test]
    fn schema_requires_continuation_or_terminal_candidate() {
        let value = serde_json::to_value(create_taskspace_control_tool(&[create_list_dir_tool()]))
            .expect("serialize");
        assert_eq!(value["parameters"]["type"], json!("object"));
        let variants = value["parameters"]["anyOf"].as_array().expect("variants");
        let action_names = variants
            .iter()
            .filter_map(|variant| variant["properties"]["action"]["enum"][0].as_str())
            .collect::<Vec<_>>();
        assert!(action_names.contains(&"initialize_then_actions"));
        assert!(action_names.contains(&"finish_then_actions"));
        assert!(action_names.contains(&"finish_then_end"));
        assert!(!action_names.contains(&"initialize_map"));
        assert!(!action_names.contains(&"finish_node"));

        let finish = variants
            .iter()
            .find(|variant| {
                variant["properties"]["action"]["enum"][0] == json!("finish_then_actions")
            })
            .expect("finish variant");
        assert_eq!(finish["properties"]["finishes"]["minItems"], json!(1));
        assert_eq!(finish["properties"]["actions"]["minItems"], json!(1));
        assert!(
            finish["required"]
                .as_array()
                .expect("required")
                .contains(&json!("actions"))
        );
    }

    #[test]
    fn nested_actions_only_enumerate_visible_ordinary_tools() {
        let list_dir = create_list_dir_tool();
        let list_dir_value = serde_json::to_value(&list_dir).expect("serialize list_dir");
        let value = serde_json::to_value(create_taskspace_control_tool(&[
            list_dir,
            create_taskspace_control_tool(&[create_list_dir_tool()]),
        ]))
        .expect("serialize");
        let text = value.to_string();
        assert!(text.contains("list_dir"));
        assert!(!text.contains("update_plan"));
        assert_eq!(text.matches("taskspace_control").count(), 1);

        let initialize = value["parameters"]["anyOf"]
            .as_array()
            .expect("variants")
            .iter()
            .find(|variant| {
                variant["properties"]["action"]["enum"][0] == json!("initialize_then_actions")
            })
            .expect("initialize variant");
        let nested = initialize["properties"]["actions"]["items"]["anyOf"]
            .as_array()
            .expect("nested variants")
            .iter()
            .find(|variant| variant["properties"]["tool_name"]["enum"][0] == json!("list_dir"))
            .expect("list_dir nested action");
        assert_eq!(
            nested["properties"]["arguments"],
            list_dir_value["parameters"]
        );
    }
}
