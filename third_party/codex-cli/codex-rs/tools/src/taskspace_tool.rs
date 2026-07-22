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
        "Create the initial rooted DAG, its unique Finish, and the first active Work binding before executing the carrying Tool.",
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

fn bind_node_schema() -> JsonSchema {
    described_object_variant(
        "bind_node",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("node_id".into(), JsonSchema::string(None)),
        ]),
        vec!["expected_revision".into(), "node_id".into()],
        "Bind one Agent-selected Ready Work node before executing the carrying Tool.",
    )
}

fn continue_current_schema() -> JsonSchema {
    described_object_variant(
        "continue_current",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "current_node_id".into(),
                JsonSchema::string(Some(
                    "The active Work node that this Tool action continues to serve.".into(),
                )),
            ),
        ]),
        vec!["expected_revision".into(), "current_node_id".into()],
        "Explicitly continue the current Work binding without changing lifecycle state before executing the carrying Tool.",
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
        "Atomically complete the active Work node and bind one Agent-selected Ready successor before executing the carrying Tool.",
    )
}

fn finish_map_schema() -> JsonSchema {
    described_object_variant(
        "finish_map",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "terminal_state".into(),
                JsonSchema::string_enum(
                    vec![
                        json!("last_running_work"),
                        json!("no_active_work_ready_finish"),
                    ],
                    Some(
                        "Choose last_running_work when terminal_node_id is the only incomplete Work and is Running while Finish is Pending. Choose no_active_work_ready_finish when terminal_node_id is the unique Ready Finish and every Work is completed."
                            .into(),
                    ),
                ),
            ),
            (
                "terminal_node_id".into(),
                JsonSchema::string(Some(
                    "The last Running Work node for last_running_work, or the unique Ready Finish node for no_active_work_ready_finish."
                        .into(),
                )),
            ),
            (
                "incomplete_work_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some(
                        "Exact incomplete Work node IDs before closure. Use exactly [terminal_node_id] for last_running_work and [] for no_active_work_ready_finish."
                            .into(),
                    ),
                ),
            ),
            (
                "finish_node_id".into(),
                JsonSchema::string(Some("The unique Finish node identifier.".into())),
            ),
            (
                "finish_status".into(),
                JsonSchema::string_enum(
                    vec![json!("pending"), json!("ready")],
                    Some(
                        "Use pending with last_running_work and ready with no_active_work_ready_finish."
                            .into(),
                    ),
                ),
            ),
            ("final_summary".into(), JsonSchema::string(None)),
        ]),
        vec![
            "expected_revision".into(),
            "terminal_state".into(),
            "terminal_node_id".into(),
            "incomplete_work_node_ids".into(),
            "finish_node_id".into(),
            "finish_status".into(),
            "final_summary".into(),
        ],
        "Close the Map from one explicitly declared terminal lifecycle snapshot. last_running_work atomically completes terminal_node_id, Finish, and Root when incomplete_work_node_ids contains only that Work and Finish is Pending. no_active_work_ready_finish closes the named Ready Finish and Root when incomplete_work_node_ids is empty. The Runtime validates the submitted revision, identities, and exact canonical state; it never selects the state.",
    )
}

pub(crate) fn taskspace_action_schema() -> JsonSchema {
    object_any_of(
        vec![
            continue_current_schema(),
            initialize_map_schema(),
            bind_node_schema(),
            complete_then_continue_schema(),
        ],
        "The explicit TaskSpace binding action applied before this Tool executes. Choose continue_current when the Tool still serves the active Work node; otherwise choose the lifecycle transition that binds the Work node served by this Tool.",
    )
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    let mut variants = vec![
        mutate_graph_schema(),
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
        description: "Use taskspace_control for standalone Map mutations, lifecycle states that do not begin a new action, terminal closure, expansion, and retained-data reads. Every ordinary action Tool explicitly declares taskspace_action: continue the current binding, or carry initialization, binding, or complete-then-continue into the action. Successful calls return the committed revision and exact delta or an exact read result; rejected calls return a structured error and whether any state was committed. The Runtime validates mechanical graph and state invariants but never chooses nodes, repairs arguments, or decides the next action.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
