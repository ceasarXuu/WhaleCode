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
                [
                    ("initialize_map", map_ref("initialize_map")),
                    ("tools", tools_ref()),
                ],
            ),
            sequence("work", [("tools", tools_ref())]),
            sequence("update_map", [("update_map", map_ref("update_map"))]),
            sequence(
                "update_and_work",
                [
                    ("update_map", map_ref("update_map")),
                    ("tools", tools_ref()),
                ],
            ),
            sequence(
                "update_and_finish",
                [
                    ("update_map", map_ref("update_map")),
                    ("finish_map", map_ref("finish_map")),
                ],
            ),
            sequence("read_map", [("read_map", map_ref("read_map"))]),
            sequence(
                "reopen_update_and_work",
                [
                    ("reopen_map", map_ref("reopen_map")),
                    ("update_map", map_ref("update_map")),
                    ("tools", tools_ref()),
                ],
            ),
            sequence("finish_map", [("finish_map", map_ref("finish_map"))]),
        ],
        Some("Choose exactly one legal TaskSpace sequence shape.".into()),
    )
    .with_definitions(definitions)
}

fn sequence<const N: usize>(
    sequence_type: &str,
    fields: [(&'static str, JsonSchema); N],
) -> JsonSchema {
    let mut properties = BTreeMap::from([("type".into(), exact_name(sequence_type))]);
    let mut required = vec!["type".to_string()];
    for (name, schema) in fields {
        properties.insert(name.into(), schema);
        required.push(name.into());
    }
    JsonSchema::object(
        properties,
        Some(required),
        Some(AdditionalProperties::Boolean(false)),
    )
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
        TaskSpaceToolCapability::Hosted(kind) => strict_object(
            [
                ("tool", exact_name(kind.name())),
                (
                    "node_ids",
                    JsonSchema::array(
                        JsonSchema::string(None),
                        Some("Agent-declared owner work nodes.".into()),
                    )
                    .with_min_items(1),
                ),
            ],
            &["tool", "node_ids"],
        ),
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
