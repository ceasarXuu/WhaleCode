use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

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

fn graph_node_schema(description: &str) -> JsonSchema {
    let mut schema = JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Stable Agent-authored node identifier.".into())),
            ),
            ("goal".into(), JsonSchema::string(Some("Node goal.".into()))),
        ]),
        Some(vec!["node_id".into(), "goal".into()]),
        Some(false.into()),
    );
    schema.description = Some(description.into());
    schema
}

fn edge_schema(description: &str) -> JsonSchema {
    let mut schema = JsonSchema::object(
        BTreeMap::from([
            (
                "from".into(),
                JsonSchema::string(Some("Source node identifier.".into())),
            ),
            (
                "to".into(),
                JsonSchema::string(Some("Target node identifier.".into())),
            ),
        ]),
        Some(vec!["from".into(), "to".into()]),
        Some(false.into()),
    );
    schema.description = Some(description.into());
    schema
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

fn initialize_map_schema(has_patch: bool) -> JsonSchema {
    object_variant(
        "initialize_map",
        BTreeMap::from([
            ("root".into(), graph_node_schema("Root node.")),
            ("finish".into(), graph_node_schema("Finish node.")),
            (
                "work_nodes".into(),
                JsonSchema::array(graph_node_schema("Work node."), Some("Work nodes.".into())),
            ),
            (
                "edges".into(),
                JsonSchema::array(
                    edge_schema("Graph edge."),
                    Some("Directed graph edges.".into()),
                ),
            ),
            (
                "current_node_id".into(),
                JsonSchema::string(Some("Initial node bound before actions execute.".into())),
            ),
            ("continuation".into(), continuation_schema(has_patch)),
        ]),
        vec![
            "root".into(),
            "finish".into(),
            "work_nodes".into(),
            "edges".into(),
            "current_node_id".into(),
            "continuation".into(),
        ],
    )
}

fn mutate_graph_schema() -> JsonSchema {
    object_variant(
        "mutate_graph",
        BTreeMap::from([
            (
                "expected_revision".into(),
                JsonSchema::integer(Some("Expected graph revision.".into())),
            ),
            (
                "add_nodes".into(),
                JsonSchema::array(
                    graph_node_schema("Node to add."),
                    Some("Nodes to add.".into()),
                ),
            ),
            (
                "add_edges".into(),
                JsonSchema::array(edge_schema("Edge to add."), Some("Edges to add.".into())),
            ),
            (
                "remove_edges".into(),
                JsonSchema::array(
                    edge_schema("Edge to remove."),
                    Some("Edges to remove.".into()),
                ),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "add_nodes".into(),
            "add_edges".into(),
            "remove_edges".into(),
        ],
    )
}

fn transition_node_schema() -> JsonSchema {
    object_variant(
        "transition_node",
        BTreeMap::from([
            (
                "expected_revision".into(),
                JsonSchema::integer(Some("Expected graph revision.".into())),
            ),
            (
                "node_id".into(),
                JsonSchema::string(Some("Target node.".into())),
            ),
            (
                "transition".into(),
                JsonSchema::string_enum(
                    vec![
                        json!("bind"),
                        json!("complete"),
                        json!("block"),
                        json!("unblock"),
                    ],
                    Some("Mechanical node transition.".into()),
                ),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "node_id".into(),
            "transition".into(),
        ],
    )
}

fn finish_end_schema() -> JsonSchema {
    object_variant(
        "finish_end",
        BTreeMap::from([
            (
                "expected_revision".into(),
                JsonSchema::integer(Some("Expected graph revision.".into())),
            ),
            (
                "final_summary".into(),
                JsonSchema::string(Some("Exact Agent-authored final summary.".into())),
            ),
        ]),
        vec!["expected_revision".into(), "final_summary".into()],
    )
}

fn simple_action_schemas() -> Vec<JsonSchema> {
    vec![
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
        vec![initialize_map_schema(has_patch)],
        "Initialize the TaskSpace map and execute immediate ordinary actions.",
    )
    .with_definitions(definitions);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace bootstrap tool. initialize_map declares root, work_nodes, finish, required edges, current_node_id, and continuation. continuation.actions contains non-patch tools. continuation.patch_then_actions contains exactly one apply_patch slot followed by optional non-patch tools. Runtime executes only the declared sequence and stops after the first failure.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

pub fn create_taskspace_active_control_tool() -> ToolSpec {
    let mut variants = vec![
        mutate_graph_schema(),
        transition_node_schema(),
        finish_end_schema(),
    ];
    variants.extend(simple_action_schemas());
    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace map tool. mutate_graph applies required graph transaction arrays under expected_revision. transition_node applies bind, complete, block, or unblock under expected_revision. finish_end releases final_summary unchanged under expected_revision. expand_nodes and read_output_ref are mechanical observation operations. Runtime follows the declared order and does not choose or infer actions.".into(),
        strict: false,
        defer_loading: None,
        parameters: object_any_of(variants, "Active TaskSpace lifecycle operation."),
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
