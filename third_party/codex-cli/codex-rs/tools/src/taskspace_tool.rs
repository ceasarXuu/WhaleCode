use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

#[path = "taskspace_tool_simple_actions.rs"]
mod simple_actions;
use simple_actions::simple_action_schemas;

fn action_tag(action: &str) -> JsonSchema {
    JsonSchema::string_enum(vec![json!(action)], None)
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
    JsonSchema::object_any_of(variants, Some(description.into()))
}

fn described_object_variant(
    action: &str,
    properties: BTreeMap<String, JsonSchema>,
    required: Vec<String>,
    description: &str,
) -> JsonSchema {
    let mut schema = object_variant(action, properties, required);
    schema.description = Some(description.into());
    schema
}

fn graph_node_schema(
    node_id_description: Option<&str>,
    goal_description: Option<&str>,
) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(node_id_description.map(str::to_owned)),
            ),
            (
                "goal".into(),
                JsonSchema::string(goal_description.map(str::to_owned)),
            ),
        ]),
        Some(vec!["node_id".into(), "goal".into()]),
        Some(false.into()),
    )
}

fn finish_identity_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([(
            "id".into(),
            JsonSchema::string(Some("Stable Agent-authored Finish identifier.".into())),
        )]),
        Some(vec!["id".into()]),
        Some(false.into()),
    )
}

fn edge_schema(describe_fields: bool) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "from".into(),
                JsonSchema::string(describe_fields.then(|| "Source node identifier.".to_string())),
            ),
            (
                "to".into(),
                JsonSchema::string(describe_fields.then(|| "Target node identifier.".to_string())),
            ),
        ]),
        Some(vec!["from".into(), "to".into()]),
        Some(false.into()),
    )
}

fn revision_schema() -> JsonSchema {
    JsonSchema::integer(None).with_minimum(0)
}

fn initialize_map_schema() -> JsonSchema {
    described_object_variant(
        "initialize_map",
        BTreeMap::from([
            (
                "root".into(),
                graph_node_schema(
                    Some("Stable Agent-authored Root identifier."),
                    Some("The user's overall task goal."),
                ),
            ),
            (
                "initial_work_node".into(),
                graph_node_schema(
                    Some("Stable Agent-authored Work identifier."),
                    Some("The first coherent Work goal."),
                ),
            ),
            ("finish_identity".into(), finish_identity_schema()),
            (
                "additional_work_nodes".into(),
                JsonSchema::array(
                    graph_node_schema(
                        Some("Stable Agent-authored Work identifier."),
                        Some("A coherent Work goal."),
                    ),
                    None,
                ),
            ),
            ("edges".into(), JsonSchema::array(edge_schema(true), None)),
        ]),
        vec![
            "root".into(),
            "initial_work_node".into(),
            "finish_identity".into(),
            "additional_work_nodes".into(),
            "edges".into(),
        ],
        "Create the initial rooted DAG and first active Work binding. This boundary call is invalid by itself. Do not wait for its result: emit the first real action Tool immediately after it in the same response.",
    )
}

fn mutate_graph_schema() -> JsonSchema {
    described_object_variant(
        "mutate_graph",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "add_nodes".into(),
                JsonSchema::array(graph_node_schema(None, None), None),
            ),
            (
                "add_edges".into(),
                JsonSchema::array(edge_schema(false), None),
            ),
            (
                "remove_edges".into(),
                JsonSchema::array(edge_schema(false), None),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "add_nodes".into(),
            "add_edges".into(),
            "remove_edges".into(),
        ],
        "Atomically add Work nodes or dependency edges and remove eligible edges. Keep the existing binding unless the mutation is mechanically incompatible with it.",
    )
}

fn node_transition_schema(action: &str, description: &str) -> JsonSchema {
    described_object_variant(
        action,
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("node_id".into(), JsonSchema::string(None)),
        ]),
        vec!["expected_revision".into(), "node_id".into()],
        description,
    )
}

fn bind_node_schema() -> JsonSchema {
    described_object_variant(
        "bind_node",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("node_id".into(), JsonSchema::string(None)),
        ]),
        vec!["expected_revision".into(), "node_id".into()],
        "Bind one Agent-selected Ready Work node. This boundary call is invalid by itself. Do not wait for its result: emit that node's first real action Tool immediately after it in the same response.",
    )
}

fn complete_then_continue_schema() -> JsonSchema {
    described_object_variant(
        "complete_then_continue",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("current_node_id".into(), JsonSchema::string(None)),
            ("next_node_id".into(), JsonSchema::string(None)),
        ]),
        vec![
            "expected_revision".into(),
            "current_node_id".into(),
            "next_node_id".into(),
        ],
        "Atomically complete the active Work node and bind one Agent-selected Ready successor. This boundary call is invalid by itself. Do not wait for its result: emit the successor's first real action Tool immediately after it in the same response.",
    )
}

fn finish_map_schema() -> JsonSchema {
    described_object_variant(
        "finish_map",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "terminal_node_id".into(),
                JsonSchema::string(Some(
                    "The Agent-selected terminal entry node: the current final Running Work, or the unique Ready Finish when no Work remains active."
                        .into(),
                )),
            ),
            ("final_summary".into(), JsonSchema::string(None)),
        ]),
        vec![
            "expected_revision".into(),
            "terminal_node_id".into(),
            "final_summary".into(),
        ],
        "Explicitly close the current Map through the Agent-selected terminal entry node. A final Running Work is completed in the same atomic transaction; an already Ready Finish is closed directly when no Work remains active. The Runtime validates the submitted revision, node identity, binding, and canonical terminal frontier without interpreting task meaning or choosing a node.",
    )
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    let mut variants = vec![
        initialize_map_schema(),
        mutate_graph_schema(),
        bind_node_schema(),
        complete_then_continue_schema(),
        node_transition_schema(
            "block_node",
            "Mark the currently running Work node blocked. The Runtime does not select an alternative path.",
        ),
        node_transition_schema(
            "unblock_node",
            "Return a blocked Work node to Ready after the Agent has determined that its blocker is cleared.",
        ),
        node_transition_schema(
            "rework_node",
            "Return a completed Work node to Ready because the Agent has decided that more work is required.",
        ),
        finish_map_schema(),
    ];
    variants.extend(simple_action_schemas());
    let parameters = object_any_of(variants, "One mechanical TaskSpace operation.");

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Use taskspace_control for Map lifecycle, graph mutations, terminal closure, expansion, and retained-data reads. initialize_map, bind_node, and complete_then_continue are boundary actions. A boundary call is invalid alone: do not wait for its result; emit at least one real action Tool immediately after it in the same response. Later ordinary Tools serve the canonical active binding without TaskSpace fields. Successful calls return the committed revision and exact delta or read result; rejected calls return a structured error and whether state was committed. The Runtime validates mechanical graph, lifecycle, ordering, binding, and lease invariants but never chooses nodes, repairs arguments, or decides the next action.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
