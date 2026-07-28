use std::collections::BTreeMap;

use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use serde_json::json;

#[path = "taskspace_tool_simple_actions.rs"]
mod simple_actions;
use simple_actions::read_action_schemas;

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

fn graph_node_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some("Agent-declared Map node identifier.".into())),
            ),
            (
                "goal".into(),
                JsonSchema::string(Some("Exact goal retained for this Map node.".into())),
            ),
        ]),
        Some(vec!["node_id".into(), "goal".into()]),
        Some(false.into()),
    )
}

fn edge_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "from".into(),
                JsonSchema::string(Some("Dependency source node identifier.".into())),
            ),
            (
                "to".into(),
                JsonSchema::string(Some("Dependency target node identifier.".into())),
            ),
        ]),
        Some(vec!["from".into(), "to".into()]),
        Some(false.into()),
    )
}

fn action_manifest_item_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            (
                "node_id".into(),
                JsonSchema::string(Some(
                    "Map node that owns the matching ordinary sibling Tool call.".into(),
                )),
            ),
            (
                "tool".into(),
                JsonSchema::string(Some(
                    "Exact name of the matching ordinary sibling Tool call.".into(),
                )),
            ),
        ]),
        Some(vec!["node_id".into(), "tool".into()]),
        Some(false.into()),
    )
}

fn actions_schema() -> JsonSchema {
    JsonSchema::array(
        action_manifest_item_schema(),
        Some(
            "Ordered ownership manifest. Item i matches ordinary sibling Tool call i in this response; ordinary Tool arguments remain native and are not copied here."
                .into(),
        ),
    )
    .with_min_items(1)
}

fn revision_schema() -> JsonSchema {
    JsonSchema::integer(None).with_minimum(0)
}

fn node_mutation_schema(action: &str, description: &str) -> JsonSchema {
    described_object_variant(
        action,
        BTreeMap::from([("node_id".into(), JsonSchema::string(None))]),
        vec!["node_id".into()],
        description,
    )
}

fn mutation_schema() -> JsonSchema {
    JsonSchema::object_any_of(
        vec![
            described_object_variant(
                "add_work_nodes",
                BTreeMap::from([(
                    "work_nodes".into(),
                    JsonSchema::array(graph_node_schema(), None).with_min_items(1),
                )]),
                vec!["work_nodes".into()],
                "Add one or more Agent-authored Work nodes.",
            ),
            described_object_variant(
                "add_edges",
                BTreeMap::from([(
                    "edges".into(),
                    JsonSchema::array(edge_schema(), None).with_min_items(1),
                )]),
                vec!["edges".into()],
                "Add one or more dependency edges.",
            ),
            described_object_variant(
                "remove_edges",
                BTreeMap::from([(
                    "edges".into(),
                    JsonSchema::array(edge_schema(), None).with_min_items(1),
                )]),
                vec!["edges".into()],
                "Remove one or more dependency edges.",
            ),
            node_mutation_schema(
                "complete_node",
                "Record Agent-declared completion of one Work node. That node cannot own an ordinary sibling Tool call in the same response.",
            ),
            node_mutation_schema(
                "block_node",
                "Record that one Work node is mechanically blocked.",
            ),
            node_mutation_schema(
                "unblock_node",
                "Remove the active block record from one Work node.",
            ),
        ],
        Some("One Agent-declared nonterminal Map mutation.".into()),
    )
}

fn initialize_and_execute_schema() -> JsonSchema {
    described_object_variant(
        "initialize_and_execute",
        BTreeMap::from([
            ("root".into(), graph_node_schema()),
            (
                "work_nodes".into(),
                JsonSchema::array(graph_node_schema(), None).with_min_items(1),
            ),
            ("finish".into(), graph_node_schema()),
            (
                "edges".into(),
                JsonSchema::array(
                    edge_schema(),
                    Some(
                        "Complete Agent-authored DAG edges from Root through every Work node to Finish."
                            .into(),
                    ),
                )
                .with_min_items(1),
            ),
            ("actions".into(), actions_schema()),
        ]),
        vec![
            "root".into(),
            "work_nodes".into(),
            "finish".into(),
            "edges".into(),
            "actions".into(),
        ],
        "Initialize the rooted TaskSpace Map and declare one or more native ordinary sibling Tool calls in the same response.",
    )
}

fn execute_schema() -> JsonSchema {
    described_object_variant(
        "execute",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "mutations".into(),
                JsonSchema::array(
                    mutation_schema(),
                    Some(
                        "Ordered nonterminal Map mutations committed before sibling Tool dispatch."
                            .into(),
                    ),
                ),
            ),
            ("actions".into(), actions_schema()),
        ]),
        vec!["expected_revision".into(), "actions".into()],
        "Declare optional nonterminal Map mutations and one or more native ordinary sibling Tool calls in the same response. Omit mutations when no Map change is needed. A node completed or blocked by this transaction cannot own a sibling action. At most one sibling may be apply_patch.",
    )
}

fn reopen_map_schema() -> JsonSchema {
    described_object_variant(
        "reopen_map",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "work_nodes".into(),
                JsonSchema::array(graph_node_schema(), None).with_min_items(1),
            ),
            (
                "edges".into(),
                JsonSchema::array(
                    edge_schema(),
                    Some(
                        "Dependency edges connecting every new Work node to the existing Root and unique Finish."
                            .into(),
                    ),
                )
                .with_min_items(1),
            ),
            ("actions".into(), actions_schema()),
        ]),
        vec![
            "expected_revision".into(),
            "work_nodes".into(),
            "edges".into(),
            "actions".into(),
        ],
        "Continue the same closed TaskSpace Map after user feedback by adding new Work and immediately performing useful native ordinary sibling Tool calls. Prior terminal and Work facts remain immutable.",
    )
}

fn finish_map_schema() -> JsonSchema {
    described_object_variant(
        "finish_map",
        BTreeMap::from([
            ("expected_revision".into(), revision_schema()),
            (
                "finish_node_id".into(),
                JsonSchema::string(Some(
                    "The unique Agent-authored Finish node identifier.".into(),
                )),
            ),
            (
                "complete_work_node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(Some(
                        "Agent-declared Work node completed by this terminal transaction.".into(),
                    )),
                    Some(
                        "One or more final Work nodes completed atomically before closing Finish and Root."
                            .into(),
                    ),
                )
                .with_min_items(1),
            ),
            (
                "exact_summary".into(),
                JsonSchema::string(Some(
                    "Agent-authored final Map summary retained without reinterpretation.".into(),
                )),
            ),
        ]),
        vec![
            "expected_revision".into(),
            "finish_node_id".into(),
            "complete_work_node_ids".into(),
            "exact_summary".into(),
        ],
        "Explicitly complete the declared final Work and close the unique Finish and Root in one terminal transaction. This action has no ordinary sibling Tool calls.",
    )
}

pub fn create_taskspace_control_tool() -> ToolSpec {
    let mut variants = vec![
        initialize_and_execute_schema(),
        execute_schema(),
        reopen_map_schema(),
    ];
    variants.extend(read_action_schemas());
    variants.push(finish_map_schema());
    let parameters = JsonSchema::object_any_of(
        variants,
        Some("One TaskSpace response manifest, read, or explicit terminal closure.".into()),
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "taskspace_control".into(),
        description: "Declare TaskSpace initialization, continued execution, or user-feedback reopen together with the ordered node ownership of native ordinary sibling Tool calls. Use the exact provider-visible sibling Tool name in each action. A response may contain at most one apply_patch sibling. Use read_map or read_output_ref for factual reads, and finish_map to complete final Work and close the Map. The Runtime validates graph, revision, action count, Tool name, order, and reservation invariants without choosing nodes or interpreting ordinary Tool arguments.".into(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "taskspace_tool_tests.rs"]
mod tests;
