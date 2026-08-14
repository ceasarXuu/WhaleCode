use std::collections::BTreeMap;

use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ToolSpecCapability;
use codex_tools::ToolSpecCapabilityInput;
use serde_json::json;

use super::catalog::TaskSpaceClientCapability;
use super::catalog::TaskSpaceToolCapability;

pub(super) fn build_sequence_schema(
    tools: &BTreeMap<codex_tools::ToolName, TaskSpaceToolCapability>,
    map_operations: &BTreeMap<String, ToolSpecCapability>,
) -> JsonSchema {
    let mut definitions = map_operations
        .iter()
        .map(|(name, capability)| {
            let ToolSpecCapabilityInput::Function(input) = &capability.input else {
                unreachable!("Map operations are structured Functions")
            };
            (map_definition_name(name), input.clone())
        })
        .collect::<BTreeMap<_, _>>();
    definitions.insert(
        "tool_action".into(),
        JsonSchema::object_any_of(
            tools.values().map(tool_action_schema).collect(),
            Some(
                "One Agent-declared Tool action. Array order is stable identity, not a Tool dependency."
                    .into(),
            ),
        ),
    );

    JsonSchema::object_any_of(
        vec![
            sequence(
                "initialize_and_work",
                "Use only when the TaskSpace Map is blank. Initialize the Map, then perform the first Tool work on nodes made Ready by that initialization.",
                [
                    ("initialize_map", map_ref("initialize_map")),
                    ("tools", tools_ref()),
                ],
            ),
            sequence(
                "work",
                "Use only when every Tool owner is already Ready or InFlight in the current Map. A prior Tool outcome does not complete its owner. If a Map update must complete or change a parent first, use update_and_work instead.",
                [("tools", tools_ref())],
            ),
            sequence(
                "update_map",
                "Update the current Map without Tool work or Map finish.",
                [("update_map", map_ref("update_map"))],
            ),
            sequence(
                "update_and_work",
                "Update the Map first, then perform Tool work that is executable in the resulting Map. Use this to complete or change parent nodes before working on their direct dependents. Only this preceding Map update can unlock Tool owners; Tool outcomes in this sequence do not unlock descendants.",
                [
                    ("update_map", map_ref("update_map")),
                    ("tools", tools_ref()),
                ],
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
            sequence(
                "reopen_update_and_work",
                "Use after user feedback requires continuing a finished Map. Reopen it, update the Agent-authored work structure, then perform executable Tool work.",
                [
                    ("reopen_map", map_ref("reopen_map")),
                    ("update_map", map_ref("update_map")),
                    ("tools", tools_ref()),
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

fn tools_ref() -> JsonSchema {
    JsonSchema::array(JsonSchema::reference("#/$defs/tool_action"), None).with_min_items(1)
}

fn map_ref(operation: &str) -> JsonSchema {
    JsonSchema::reference(format!("#/$defs/{}", map_definition_name(operation)))
}

fn map_definition_name(operation: &str) -> String {
    format!("{operation}_input")
}

fn tool_action_schema(capability: &TaskSpaceToolCapability) -> JsonSchema {
    match capability {
        TaskSpaceToolCapability::Client(client) => client_action_schema(client),
        TaskSpaceToolCapability::Hosted(identity) => {
            let mut schema = strict_object(
                [
                    ("tool", exact_name(&identity.native_name)),
                    ("execution", exact_name("already_executed")),
                    (
                        "node_ids",
                        JsonSchema::array(
                            JsonSchema::string(None),
                            Some(
                                "One or more Agent-declared owner work nodes for this native Provider ToolSpec action."
                                    .into(),
                            ),
                        )
                        .with_min_items(1),
                    ),
                ],
                &["tool", "execution", "node_ids"],
            );
            schema.description = Some(format!(
                "Node-ownership record for a native `{}` Provider result already present in this assistant response. This entry does not call, request, or trigger the Tool. `{}` remains a native Provider ToolSpec, not a Function Tool; never construct or imitate a Function Call with this name. Include exactly one matching entry with `execution` set to `already_executed` and every Agent-declared owner in `node_ids` only when the current response contains the matching native result. Otherwise include no entry. Do not record ownership early, defer it to a later response, or include native Tool input.",
                identity.native_name, identity.native_name
            ));
            schema
        }
    }
}

fn client_action_schema(client: &TaskSpaceClientCapability) -> JsonSchema {
    let capability = &client.capability;
    let input = match &capability.input {
        ToolSpecCapabilityInput::Function(schema) => schema.clone(),
        ToolSpecCapabilityInput::Freeform(format) => JsonSchema::string(Some(format!(
            "Freeform {} input using {} syntax.\n{}",
            format.r#type, format.syntax, format.definition
        ))),
    };
    let mut properties = BTreeMap::from([
        ("tool".into(), exact_name(&capability.tool_name.name)),
        (
            "node_id".into(),
            JsonSchema::string(Some("Agent-declared owner work node.".into())),
        ),
        ("input".into(), input),
    ]);
    let mut required = vec!["tool".into(), "node_id".into(), "input".into()];
    if let Some(namespace) = capability.tool_name.namespace.as_deref() {
        properties.insert("namespace".into(), exact_name(namespace));
        required.insert(1, "namespace".into());
    }
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

fn strict_object<const N: usize>(
    properties: [(&'static str, JsonSchema); N],
    required: &[&str],
) -> JsonSchema {
    JsonSchema::object(
        properties
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect(),
        Some(required.iter().map(|name| (*name).to_string()).collect()),
        Some(AdditionalProperties::Boolean(false)),
    )
}
