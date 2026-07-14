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

struct NestedActionSchemas {
    ordinary: JsonSchema,
    patch: Option<JsonSchema>,
}

fn nested_action_schemas(visible_tools: &[ToolSpec]) -> NestedActionSchemas {
    let mut ordinary_variants = Vec::new();
    let mut patch_variant = None;
    for spec in visible_tools {
        match spec {
            ToolSpec::Function(tool)
                if !matches!(tool.name.as_str(), "taskspace_control" | "update_plan") =>
            {
                let schema = function_action_schema(tool, None);
                if tool.name == "apply_patch" {
                    patch_variant = Some(schema);
                } else {
                    ordinary_variants.push((tool.name.clone(), schema));
                }
            }
            ToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let ResponsesApiNamespaceTool::Function(tool) = tool;
                    ordinary_variants.push((
                        format!("{}.{}", namespace.name, tool.name),
                        function_action_schema(tool, Some(&namespace.name)),
                    ));
                }
            }
            ToolSpec::Freeform(tool) => {
                let schema = custom_action_schema(&tool.name);
                if tool.name == "apply_patch" {
                    patch_variant = Some(schema);
                } else {
                    ordinary_variants.push((tool.name.clone(), schema));
                }
            }
            ToolSpec::Function(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::WebSearch { .. } => {}
        }
    }
    ordinary_variants.sort_by(|left, right| left.0.cmp(&right.0));
    NestedActionSchemas {
        ordinary: object_any_of(
            ordinary_variants
                .into_iter()
                .map(|(_, schema)| schema)
                .collect(),
            "One ordinary non-patch tool call visible in this request.",
        ),
        patch: patch_variant,
    }
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
            ("goal".into(), JsonSchema::string(Some("Node goal.".into()))),
            (
                "dependency_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Agent-authored prerequisite node identifiers.".into()),
                ),
            ),
        ]),
        Some(vec!["node_id".into(), "kind".into(), "goal".into()]),
        Some(false.into()),
    )
}

fn next_variant(
    kind: &str,
    mut properties: BTreeMap<String, JsonSchema>,
    mut required: Vec<String>,
) -> JsonSchema {
    properties.insert(
        "kind".into(),
        JsonSchema::string_enum(vec![json!(kind)], Some("Next binding variant.".into())),
    );
    required.insert(0, "kind".into());
    JsonSchema::object(properties, Some(required), Some(false.into()))
}

fn next_existing_schema() -> JsonSchema {
    next_variant(
        "existing",
        BTreeMap::from([(
            "node_id".into(),
            JsonSchema::string(Some("Existing node bound after this finish.".into())),
        )]),
        vec!["node_id".into()],
    )
}

fn next_created_schema() -> JsonSchema {
    next_variant(
        "create",
        BTreeMap::from([
            (
                "node_kind".into(),
                JsonSchema::string_enum(node_kind_values(), Some("Created node type.".into())),
            ),
            (
                "goal".into(),
                JsonSchema::string(Some("Created node goal.".into())),
            ),
            (
                "dependency_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some("Created node prerequisites.".into()),
                ),
            ),
        ]),
        vec!["node_kind".into(), "goal".into()],
    )
}

fn next_schema() -> JsonSchema {
    object_any_of(
        vec![next_existing_schema(), next_created_schema()],
        "Exactly one Agent-declared next binding.",
    )
}

fn nonterminal_finish_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Optional explicit ready finish target.".into())),
            ),
            ("next".into(), next_schema()),
        ]),
        Some(vec!["next".into()]),
        Some(false.into()),
    )
}

fn continuation_variant(
    kind: &str,
    mut properties: BTreeMap<String, JsonSchema>,
    mut required: Vec<String>,
) -> JsonSchema {
    properties.insert(
        "kind".into(),
        JsonSchema::string_enum(vec![json!(kind)], Some("Continuation variant.".into())),
    );
    required.insert(0, "kind".into());
    JsonSchema::object(properties, Some(required), Some(false.into()))
}

fn continuation_schema(has_patch: bool) -> JsonSchema {
    let ordinary_action = JsonSchema::reference("#/$defs/ordinaryAction");
    let mut variants = vec![continuation_variant(
        "actions",
        BTreeMap::from([(
            "actions".into(),
            JsonSchema::array(
                ordinary_action.clone(),
                Some("Immediate ordinary non-patch actions.".into()),
            )
            .with_min_items(1),
        )]),
        vec!["actions".into()],
    )];
    if has_patch {
        variants.push(continuation_variant(
            "patch_then_actions",
            BTreeMap::from([
                ("patch".into(), JsonSchema::reference("#/$defs/patchAction")),
                (
                    "actions".into(),
                    JsonSchema::array(
                        ordinary_action,
                        Some("Ordinary non-patch actions after the patch.".into()),
                    ),
                ),
            ]),
            vec!["patch".into()],
        ));
    }
    object_any_of(
        variants,
        "Exactly one declared continuation shape; patch_then_actions contains one patch slot.",
    )
}

fn initialize_then_actions_schema(has_patch: bool) -> JsonSchema {
    object_variant(
        "initialize_then_actions",
        BTreeMap::from([
            (
                "initial_nodes".into(),
                JsonSchema::array(initial_node_schema(), Some("Initial node graph.".into()))
                    .with_min_items(1),
            ),
            (
                "current_node_id".into(),
                JsonSchema::string(Some("Initial node bound before actions execute.".into())),
            ),
            ("continuation".into(), continuation_schema(has_patch)),
        ]),
        vec![
            "initial_nodes".into(),
            "current_node_id".into(),
            "continuation".into(),
        ],
    )
}

fn finish_nodes_schema() -> JsonSchema {
    object_variant(
        "finish_nodes",
        BTreeMap::from([(
            "finishes".into(),
            JsonSchema::array(
                nonterminal_finish_schema(),
                Some("Ordered nonterminal finishes.".into()),
            )
            .with_min_items(1),
        )]),
        vec!["finishes".into()],
    )
}

fn finish_then_end_schema() -> JsonSchema {
    object_variant(
        "finish_then_end",
        BTreeMap::from([
            (
                "finish_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some(
                        "Agent-declared finish order; each node binds to the next ID and the last node is terminal."
                            .into(),
                    ),
                )
                .with_min_items(1),
            ),
            (
                "final_candidate".into(),
                JsonSchema::string(Some("Exact Agent-authored final answer.".into())),
            ),
        ]),
        vec!["finish_node_ids".into(), "final_candidate".into()],
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
                ("goal".into(), JsonSchema::string(None)),
                (
                    "dependency_node_ids".into(),
                    JsonSchema::array(JsonSchema::string(None), None),
                ),
                ("bind_current".into(), JsonSchema::boolean(None)),
            ]),
            vec!["kind".into(), "goal".into()],
        ),
        object_variant(
            "bind_node",
            BTreeMap::from([("node_id".into(), JsonSchema::string(None))]),
            vec!["node_id".into()],
        ),
        object_variant(
            "block_node",
            BTreeMap::from([("node_id".into(), JsonSchema::string(None))]),
            vec!["node_id".into()],
        ),
        object_variant(
            "expand_nodes",
            BTreeMap::from([(
                "node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some(
                        "Currently folded node identifiers whose hidden event refs must be restored atomically."
                            .into(),
                    ),
                )
                .with_min_items(1),
            )]),
            vec!["node_ids".into()],
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
    let actions = nested_action_schemas(visible_tools);
    let has_patch = actions.patch.is_some();
    let mut definitions = BTreeMap::from([("ordinaryAction".into(), actions.ordinary)]);
    if let Some(patch) = actions.patch {
        definitions.insert("patchAction".into(), patch);
    }
    let parameters = object_any_of(
        vec![initialize_then_actions_schema(has_patch)],
        "Initialize the TaskSpace map and execute immediate ordinary actions.",
    )
    .with_definitions(definitions);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace bootstrap tool. initialize_then_actions initializes and binds the Agent-authored map, then executes its continuation in order. continuation.actions contains non-patch tools. continuation.patch_then_actions contains exactly one apply_patch slot followed by optional non-patch tools. Runtime executes only the declared sequence and stops after the first failure.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

pub fn create_taskspace_active_control_tool() -> ToolSpec {
    let mut variants = vec![finish_nodes_schema(), finish_then_end_schema()];
    variants.extend(simple_action_schemas());
    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace map tool. finish_nodes commits ordered nonterminal finishes; each finish requires one tagged next binding: kind=existing binds next.node_id, kind=create creates the declared node. Ordinary sibling tool calls later in the same provider response execute after this state barrier under the latest binding. finish_then_end commits the Agent-declared finish_node_ids in order, uses the last ID as the terminal node, and releases final_candidate unchanged. expand_nodes atomically returns the hidden event refs of folded nodes, records the Agent expansion event, and keeps every detail ref visible in later projections. Mutation results report state_commit plus the current and remaining open Map state. Runtime follows the declared order and does not choose or infer actions.".into(),
        strict: false,
        defer_loading: None,
        parameters: object_any_of(variants, "Active TaskSpace lifecycle operation."),
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
