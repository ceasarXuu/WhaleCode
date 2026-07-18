use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

#[path = "taskspace_tool_simple_actions.rs"]
mod simple_actions;
use simple_actions::simple_action_schemas;

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

fn referenced_function_action_schema(
    tool_names: Vec<String>,
    namespace: Option<&str>,
) -> JsonSchema {
    let mut arguments = JsonSchema::object(BTreeMap::new(), None, None);
    arguments.description = Some("Exact arguments for the separately exposed tool schema.".into());
    let mut properties = BTreeMap::from([
        (
            "tool_name".into(),
            JsonSchema::string_enum(
                tool_names
                    .into_iter()
                    .map(serde_json::Value::from)
                    .collect(),
                Some("Name of a separately exposed ordinary tool.".into()),
            ),
        ),
        ("arguments".into(), arguments),
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

fn exact_function_action_schema(tool: &ResponsesApiTool) -> JsonSchema {
    let properties = BTreeMap::from([
        (
            "tool_name".into(),
            JsonSchema::string_enum(
                vec![json!(tool.name)],
                Some("Visible ordinary tool name.".into()),
            ),
        ),
        ("arguments".into(), tool.parameters.clone()),
    ]);
    JsonSchema::object(
        properties,
        Some(vec!["tool_name".into(), "arguments".into()]),
        Some(false.into()),
    )
}

fn custom_action_schema(names: Vec<String>) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "tool_name".into(),
                JsonSchema::string_enum(
                    names.into_iter().map(serde_json::Value::from).collect(),
                    Some("Name of a separately exposed ordinary custom tool.".into()),
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
    let mut function_names = Vec::new();
    let mut namespaced_functions = BTreeMap::<String, Vec<String>>::new();
    let mut custom_names = Vec::new();
    let mut patch_variant = None;
    for spec in visible_tools {
        match spec {
            ToolSpec::Function(tool)
                if !matches!(tool.name.as_str(), "taskspace_control" | "update_plan") =>
            {
                if tool.name == "apply_patch" {
                    patch_variant = Some(exact_function_action_schema(tool));
                } else {
                    function_names.push(tool.name.clone());
                }
            }
            ToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let ResponsesApiNamespaceTool::Function(tool) = tool;
                    namespaced_functions
                        .entry(namespace.name.clone())
                        .or_default()
                        .push(tool.name.clone());
                }
            }
            ToolSpec::Freeform(tool) => {
                if tool.name == "apply_patch" {
                    patch_variant = Some(custom_action_schema(vec![tool.name.clone()]));
                } else {
                    custom_names.push(tool.name.clone());
                }
            }
            ToolSpec::Function(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::WebSearch { .. } => {}
        }
    }
    function_names.sort();
    custom_names.sort();
    for names in namespaced_functions.values_mut() {
        names.sort();
    }
    let mut ordinary_variants = Vec::new();
    if !function_names.is_empty() {
        ordinary_variants.push(referenced_function_action_schema(function_names, None));
    }
    ordinary_variants.extend(
        namespaced_functions
            .into_iter()
            .map(|(namespace, names)| referenced_function_action_schema(names, Some(&namespace))),
    );
    if !custom_names.is_empty() {
        ordinary_variants.push(custom_action_schema(custom_names));
    }
    NestedActionSchemas {
        ordinary: object_any_of(
            ordinary_variants,
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

fn finish_identity_schema() -> JsonSchema {
    let mut schema = JsonSchema::object(
        BTreeMap::from([(
            "id".into(),
            JsonSchema::string(Some("Stable Agent-authored Finish identifier.".into())),
        )]),
        Some(vec!["id".into()]),
        Some(false.into()),
    );
    schema.description = Some(
        "The unique terminal graph node identity. Reference id as the graph's only sink in edges; every node must reach it. All executable work, including validation, belongs to Work nodes."
            .into(),
    );
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
            (
                "initial_work_node".into(),
                graph_node_schema(
                    "Agent-selected initial Work node. Define it only here, not in additional_work_nodes. Declared edges must make it Ready at initialization; Runtime binds it before continuation actions execute.",
                ),
            ),
            ("finish_identity".into(), finish_identity_schema()),
            (
                "additional_work_nodes".into(),
                JsonSchema::array(
                    graph_node_schema("Additional Work node."),
                    Some(
                        "Zero or more Work nodes other than initial_work_node. Node IDs must be distinct across the entire graph."
                            .into(),
                    ),
                ),
            ),
            (
                "edges".into(),
                JsonSchema::array(
                    edge_schema("Graph edge."),
                    Some("Directed graph edges.".into()),
                ),
            ),
            ("continuation".into(), continuation_schema(has_patch)),
        ]),
        vec![
            "root".into(),
            "initial_work_node".into(),
            "finish_identity".into(),
            "additional_work_nodes".into(),
            "edges".into(),
            "continuation".into(),
        ],
    )
}

fn mutate_graph_schema(has_patch: bool) -> JsonSchema {
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
            ("continuation".into(), continuation_schema(has_patch)),
        ]),
        vec![
            "expected_revision".into(),
            "add_nodes".into(),
            "add_edges".into(),
            "remove_edges".into(),
        ],
    )
}

fn bind_node_schema(has_patch: bool) -> JsonSchema {
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
                    vec![json!("bind")],
                    Some("Mechanical node transition.".into()),
                ),
            ),
            ("continuation".into(), continuation_schema(has_patch)),
        ]),
        vec![
            "expected_revision".into(),
            "node_id".into(),
            "transition".into(),
            "continuation".into(),
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
                        json!("complete"),
                        json!("block"),
                        json!("unblock"),
                        json!("rework"),
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

pub fn create_taskspace_control_tool(visible_tools: &[ToolSpec]) -> ToolSpec {
    let actions = nested_action_schemas(visible_tools);
    let has_patch = actions.patch.is_some();
    let mut definitions = BTreeMap::from([("ordinaryAction".into(), actions.ordinary)]);
    if let Some(patch) = actions.patch {
        definitions.insert("patchAction".into(), patch);
    }
    let mut variants = vec![
        initialize_map_schema(has_patch),
        mutate_graph_schema(has_patch),
        bind_node_schema(has_patch),
        transition_node_schema(),
        finish_end_schema(),
    ];
    variants.extend(simple_action_schemas());
    let parameters = object_any_of(variants, "One mechanical TaskSpace lifecycle operation.")
        .with_definitions(definitions);

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace lifecycle tool. When visible TaskSpace bootstrap state has bootstrap_required=true, the first top-level tool call MUST be taskspace_control with action=initialize_map; place any immediate ordinary work in initialize_map.continuation. initialize_map declares and binds the initial rooted DAG before its continuation. mutate_graph may continue only from an existing binding that remains valid. transition_node bind requires a continuation; complete, block, unblock, and rework do not accept one. finish_end commits the Agent-authored final summary and cannot continue. read_map returns the exact current full Map projection through the shared renderer; expand_nodes and read_output_ref expose mechanically retained details. For a given map_id, the last visible projection is current and all earlier projections are historical; repeated revision values mean the map did not change between requests. Runtime validates hard state rules and executes only the declared operation order. It does not choose, infer, or rewrite actions.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
