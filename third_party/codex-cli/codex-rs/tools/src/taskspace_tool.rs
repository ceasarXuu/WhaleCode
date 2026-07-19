use std::collections::BTreeMap;

use crate::JsonSchema;
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

fn required_next_call_schema(has_patch: bool) -> JsonSchema {
    let mut variants = vec![json!("ordinary_tool")];
    if has_patch {
        variants.push(json!("apply_patch"));
    }
    JsonSchema::string_enum(
        variants,
        Some(
            "Declaration only: emit the selected top-level sibling immediately after taskspace_control in this same response. ordinary_tool requires an ordinary non-control tool; apply_patch requires direct apply_patch. This field does not execute or schedule that call."
                .into(),
        ),
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
                    "Agent-selected initial Work node. Define it only here, not in additional_work_nodes. Declared edges must make it Ready at initialization; Runtime binds it before the required next top-level call executes.",
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
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch),
            ),
        ]),
        vec![
            "root".into(),
            "initial_work_node".into(),
            "finish_identity".into(),
            "additional_work_nodes".into(),
            "edges".into(),
            "required_next_call".into(),
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
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch),
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
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "node_id".into(),
            "transition".into(),
            "required_next_call".into(),
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
                    vec![json!("block"), json!("unblock"), json!("rework")],
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

fn complete_then_continue_schema(has_patch: bool) -> JsonSchema {
    object_variant(
        "complete_then_continue",
        BTreeMap::from([
            (
                "expected_revision".into(),
                JsonSchema::integer(Some("Expected graph revision.".into())),
            ),
            (
                "current_node_id".into(),
                JsonSchema::string(Some(
                    "Current bound Work node to complete atomically.".into(),
                )),
            ),
            (
                "next_node_id".into(),
                JsonSchema::string(Some(
                    "Agent-selected successor to bind after completion makes it Ready.".into(),
                )),
            ),
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "current_node_id".into(),
            "next_node_id".into(),
            "required_next_call".into(),
        ],
    )
}

fn complete_then_end_schema() -> JsonSchema {
    object_variant(
        "complete_then_end",
        BTreeMap::from([
            (
                "expected_revision".into(),
                JsonSchema::integer(Some("Expected graph revision.".into())),
            ),
            (
                "current_node_id".into(),
                JsonSchema::string(Some(
                    "Current bound final Work node to complete atomically.".into(),
                )),
            ),
            (
                "final_summary".into(),
                JsonSchema::string(Some("Exact Agent-authored final summary.".into())),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "current_node_id".into(),
            "final_summary".into(),
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
    let has_patch = visible_tools.iter().any(|spec| match spec {
        ToolSpec::Function(tool) => tool.name == "apply_patch",
        ToolSpec::Freeform(tool) => tool.name == "apply_patch",
        _ => false,
    });
    let mut variants = vec![
        initialize_map_schema(has_patch),
        mutate_graph_schema(has_patch),
        bind_node_schema(has_patch),
        transition_node_schema(),
        complete_then_continue_schema(has_patch),
        complete_then_end_schema(),
        finish_end_schema(),
    ];
    variants.extend(simple_action_schemas());
    let parameters = object_any_of(variants, "One mechanical TaskSpace lifecycle operation.");

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Mandatory mechanical TaskSpace lifecycle tool. When visible TaskSpace bootstrap state has bootstrap_required=true, the first top-level tool call MUST be taskspace_control with action=initialize_map. initialize_map declares and binds the initial rooted DAG. mutate_graph may continue only from an existing binding that remains valid. A running Work node cannot be completed alone: use complete_then_continue to atomically complete it and bind the Agent-selected next Ready node; use complete_then_end to atomically complete the final Work node and close the Map with the exact Agent-authored summary. initialize_map, bind, and complete_then_continue require an immediately following top-level sibling in the same provider response. required_next_call=ordinary_tool declares an ordinary non-control sibling; required_next_call=apply_patch declares direct apply_patch. required_next_call only declares the sibling: it never executes or schedules it, so emit both calls in this response. Never nest tool names, arguments, or patch content in taskspace_control. transition_node handles bind, block, unblock, and rework only. finish_end is reserved for a Map whose Finish is already Ready and cannot continue. read_map returns the exact current full Map projection through the shared renderer; expand_nodes and read_output_ref expose mechanically retained details. For a given map_id, the last visible projection is current and all earlier projections are historical; repeated revision values mean the map did not change between requests. Runtime validates hard state rules and executes only the declared provider order. It does not choose, infer, move, or rewrite actions.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
