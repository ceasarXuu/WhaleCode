use std::collections::BTreeMap;

use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ToolSpecCapability;
use codex_tools::ToolSpecCapabilityInput;
use serde_json::json;

use super::catalog::TaskSpaceClientCapability;
use super::catalog::TaskSpaceToolCapability;

pub(super) fn build_sequence_schema(
    capabilities: &BTreeMap<codex_tools::ToolName, TaskSpaceToolCapability>,
    map_operations: &BTreeMap<String, ToolSpecCapability>,
) -> JsonSchema {
    let initialize_map_schema = map_operations
        .get("initialize_map")
        .and_then(|capability| match &capability.input {
            ToolSpecCapabilityInput::Function(input) => Some(input.clone()),
            ToolSpecCapabilityInput::Freeform(_) => None,
        })
        .expect("initialize_map is a structured Function");
    let mut definitions = map_operations
        .iter()
        .filter(|(name, _)| name.as_str() != "initialize_map")
        .map(|(name, capability)| {
            let ToolSpecCapabilityInput::Function(input) = &capability.input else {
                unreachable!("Map operations are structured Functions")
            };
            (map_definition_name(name), input.clone())
        })
        .collect::<BTreeMap<_, _>>();
    definitions.insert(
        "taskspace_action".into(),
        JsonSchema::object_any_of(
            capabilities
                .values()
                .filter_map(|capability| match capability {
                    TaskSpaceToolCapability::Client(client) => Some(action_schema(client)),
                    TaskSpaceToolCapability::Hosted(_) => None,
                })
                .collect(),
            Some(
                "One Agent-declared TaskSpace action. Array order is stable identity, not an action dependency."
                    .into(),
            ),
        ),
    );
    JsonSchema::object_any_of(
        vec![
            work_sequence(
                "initialize_and_work",
                "Use only when the TaskSpace Map is blank. Initialize the Map, then perform the first work declared in the required non-empty `actions` array.",
                [("initialize_map", initialize_map_schema)],
            ),
            work_sequence(
                "work",
                "Perform one or more actions from the required non-empty `actions` array when every owner is already Ready or InFlight in the current Map. A prior action outcome does not complete its owner. If a Map update must complete or change a parent first, use update_and_work instead.",
                [],
            ),
            sequence(
                "update_map",
                "Update the current Map without Tool work or Map finish.",
                [("update_map", map_ref("update_map"))],
            ),
            work_sequence(
                "update_and_work",
                "Update the Map first, then perform one or more executable actions from the required non-empty `actions` array. Use this to complete or change parent nodes before working on their direct dependents. Only this preceding Map update can unlock action owners; action outcomes in this sequence do not unlock descendants.",
                [("update_map", map_ref("update_map"))],
            ),
            sequence(
                "update_and_finish",
                "Update the Map first, then finish it. Use only when that update completes the remaining work and makes the Finish node Ready.",
                [
                    ("update_map", map_ref("update_map")),
                    ("finish_map", map_ref("finish_map")),
                ],
            ),
            sequence(
                "read_map",
                "Read the complete current Map without changing it or performing Tool work.",
                [("read_map", map_ref("read_map"))],
            ),
            work_sequence(
                "reopen_update_and_work",
                "Use after user feedback requires continuing a finished Map. Reopen it, update the Agent-authored work structure, then perform one or more executable actions from the required non-empty `actions` array.",
                [
                    ("reopen_map", map_ref("reopen_map")),
                    ("update_map", map_ref("update_map")),
                ],
            ),
            sequence(
                "finish_map",
                "Finish an open Map whose Finish node is already Ready; use update_and_finish when a preceding Map update is still required.",
                [("finish_map", map_ref("finish_map"))],
            ),
        ],
        Some("Choose exactly one legal TaskSpace sequence shape.".into()),
    )
    .with_definitions(definitions)
}

fn work_sequence<const N: usize>(
    sequence_type: &str,
    description: &str,
    fields: [(&'static str, JsonSchema); N],
) -> JsonSchema {
    let mut schema = sequence(sequence_type, description, fields);
    let properties = schema
        .properties
        .as_mut()
        .expect("sequence schema is an object");
    properties.insert("actions".into(), actions_ref());
    schema
        .required
        .as_mut()
        .expect("sequence schema has required fields")
        .push("actions".into());
    schema
}

fn sequence<const N: usize>(
    sequence_type: &str,
    description: &str,
    fields: [(&'static str, JsonSchema); N],
) -> JsonSchema {
    let mut properties = BTreeMap::from([("type".into(), exact_name(sequence_type))]);
    let mut required = vec!["type".to_string()];
    for (name, schema) in fields {
        properties.insert(name.into(), schema);
        required.push(name.into());
    }
    let mut schema = JsonSchema::object(
        properties,
        Some(required),
        Some(AdditionalProperties::Boolean(false)),
    );
    schema.description = Some(description.into());
    schema
}

fn actions_ref() -> JsonSchema {
    JsonSchema::array(JsonSchema::reference("#/$defs/taskspace_action"), None).with_min_items(1)
}

fn map_ref(operation: &str) -> JsonSchema {
    JsonSchema::reference(format!("#/$defs/{}", map_definition_name(operation)))
}

fn map_definition_name(operation: &str) -> String {
    format!("{operation}_input")
}

fn action_schema(client: &TaskSpaceClientCapability) -> JsonSchema {
    let capability = &client.capability;
    let input = match &capability.input {
        ToolSpecCapabilityInput::Function(schema) => schema.clone(),
        ToolSpecCapabilityInput::Freeform(format) => JsonSchema::string(Some(format!(
            "Freeform {} input using {} syntax.\n{}",
            format.r#type, format.syntax, format.definition
        ))),
    };
    let properties = BTreeMap::from([
        ("kind".into(), exact_name(&client.action_name)),
        (
            "node_id".into(),
            JsonSchema::string(Some("Agent-declared owner work node.".into())),
        ),
        ("parameters".into(), input),
    ]);
    let required = vec!["kind".into(), "node_id".into(), "parameters".into()];
    let mut schema = JsonSchema::object(
        properties,
        Some(required),
        Some(AdditionalProperties::Boolean(false)),
    );
    schema.description = Some(capability.description.clone());
    schema
}

fn exact_name(name: &str) -> JsonSchema {
    JsonSchema::string_enum(vec![json!(name)], None)
}
