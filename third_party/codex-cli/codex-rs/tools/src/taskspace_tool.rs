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

fn required_next_call_schema(has_patch: bool, description: Option<&str>) -> JsonSchema {
    let mut variants = vec![json!("ordinary_tool")];
    if has_patch {
        variants.push(json!("apply_patch"));
    }
    JsonSchema::string_enum(variants, description.map(str::to_owned))
}

fn revision_schema() -> JsonSchema {
    JsonSchema::integer(None).with_minimum(0)
}

fn initialize_map_schema(has_patch: bool) -> JsonSchema {
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
            (
                "required_next_call".into(),
                required_next_call_schema(
                    has_patch,
                    Some(
                        "Declare the immediately following top-level non-control sibling. This field does not execute or schedule that call.",
                    ),
                ),
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
        "Create the initial rooted DAG, its unique Finish, and the first active Work binding. Emit the first real non-control action as the next top-level sibling in the same response.",
    )
}

fn mutate_graph_schema(has_patch: bool) -> JsonSchema {
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
            (
                "required_next_call".into(),
                required_next_call_schema(
                    has_patch,
                    Some(
                        "Optional declaration of an immediately following top-level non-control sibling under the unchanged binding.",
                    ),
                ),
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

fn bind_node_schema(has_patch: bool) -> JsonSchema {
    described_object_variant(
        "bind_node",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("node_id".into(), JsonSchema::string(None)),
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch, None),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "node_id".into(),
            "required_next_call".into(),
        ],
        "Bind one Agent-selected Ready Work node before its first ordinary action. Emit that real action as the next top-level sibling.",
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

fn complete_then_continue_schema(has_patch: bool) -> JsonSchema {
    described_object_variant(
        "complete_then_continue",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("current_node_id".into(), JsonSchema::string(None)),
            ("next_node_id".into(), JsonSchema::string(None)),
            (
                "required_next_call".into(),
                required_next_call_schema(has_patch, None),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "current_node_id".into(),
            "next_node_id".into(),
            "required_next_call".into(),
        ],
        "Atomically complete the active Work node and bind one Agent-selected Ready successor. Emit the successor's first real action as the next top-level sibling.",
    )
}

fn complete_then_end_schema() -> JsonSchema {
    described_object_variant(
        "complete_then_end",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("current_node_id".into(), JsonSchema::string(None)),
            ("final_summary".into(), JsonSchema::string(None)),
        ]),
        vec![
            "expected_revision".into(),
            "current_node_id".into(),
            "final_summary".into(),
        ],
        "Atomically complete the final active Work node, close the unique Finish and Root, and store the exact Agent-authored final summary.",
    )
}

fn finish_end_schema() -> JsonSchema {
    described_object_variant(
        "finish_end",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            ("final_summary".into(), JsonSchema::string(None)),
        ]),
        vec!["expected_revision".into(), "final_summary".into()],
        "Close a Map whose Finish is already Ready and no Work remains active. Store the exact Agent-authored final summary.",
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
        complete_then_continue_schema(has_patch),
        complete_then_end_schema(),
        finish_end_schema(),
    ];
    variants.extend(simple_action_schemas());
    let parameters = object_any_of(variants, "One mechanical TaskSpace operation.");

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Use taskspace_control to initialize and change the canonical TaskSpace Map, bind Work nodes, commit lifecycle transitions, expand folded node details, and read retained TaskSpace facts. Each call selects one action schema. Successful calls return the committed revision and exact delta or an exact read result; rejected calls return a structured error and whether any state was committed. Use it only for Map state and retained TaskSpace data, not to wrap ordinary tool names, commands, patch content, or reasoning. The Runtime validates mechanical graph and state invariants but never chooses nodes, repairs arguments, or decides the next action.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
