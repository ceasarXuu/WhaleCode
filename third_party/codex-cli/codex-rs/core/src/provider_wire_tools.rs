use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use super::json_bytes;
use super::json_hash;

const TASKSPACE_EXEC: &str = "taskspace_exec";
const TOOL_ACTION_DEF: &str = "tool_action";

#[derive(Debug, Serialize)]
pub(super) struct ProviderWireToolCost {
    kind: ToolCostKind,
    pub(super) count: usize,
    pub(super) bytes: usize,
    estimated_tokens: usize,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ToolCostKind {
    ToolsEnvelope,
    TaskspaceProtocol,
    TaskspaceClientCatalog,
    TaskspaceMapSchema,
    TaskspaceSequenceSchema,
    TaskspaceMetadata,
    NativeClientTool,
    ProviderHostedTool,
    OtherTool,
}

impl ToolCostKind {
    pub(super) const ALL: [Self; 9] = [
        Self::ToolsEnvelope,
        Self::TaskspaceProtocol,
        Self::TaskspaceClientCatalog,
        Self::TaskspaceMapSchema,
        Self::TaskspaceSequenceSchema,
        Self::TaskspaceMetadata,
        Self::NativeClientTool,
        Self::ProviderHostedTool,
        Self::OtherTool,
    ];

    pub(super) const fn index(self) -> usize {
        match self {
            Self::ToolsEnvelope => 0,
            Self::TaskspaceProtocol => 1,
            Self::TaskspaceClientCatalog => 2,
            Self::TaskspaceMapSchema => 3,
            Self::TaskspaceSequenceSchema => 4,
            Self::TaskspaceMetadata => 5,
            Self::NativeClientTool => 6,
            Self::ProviderHostedTool => 7,
            Self::OtherTool => 8,
        }
    }
}

pub(super) fn measure(tools: &Value, section_bytes: usize) -> Vec<ProviderWireToolCost> {
    let mut counts = [0usize; 9];
    let mut bytes = [0usize; 9];
    let mut values: [Vec<Value>; 9] = std::array::from_fn(|_| Vec::new());
    let items = tools.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let item_bytes = items
        .iter()
        .map(|item| json_bytes(item).len())
        .sum::<usize>();
    add(
        ToolCostKind::ToolsEnvelope,
        usize::from(section_bytes > 0),
        section_bytes.saturating_sub(item_bytes),
        tools.clone(),
        &mut counts,
        &mut bytes,
        &mut values,
    );

    for item in items {
        if item.get("name").and_then(Value::as_str) == Some(TASKSPACE_EXEC) {
            measure_taskspace_tool(item, &mut counts, &mut bytes, &mut values);
        } else if item.get("name").and_then(Value::as_str).is_some() {
            add_whole(
                ToolCostKind::NativeClientTool,
                item,
                &mut counts,
                &mut bytes,
                &mut values,
            );
        } else if item.get("type").and_then(Value::as_str).is_some() {
            add_whole(
                ToolCostKind::ProviderHostedTool,
                item,
                &mut counts,
                &mut bytes,
                &mut values,
            );
        } else {
            add_whole(
                ToolCostKind::OtherTool,
                item,
                &mut counts,
                &mut bytes,
                &mut values,
            );
        }
    }

    ToolCostKind::ALL
        .into_iter()
        .zip(counts)
        .zip(bytes)
        .zip(values)
        .map(|(((kind, count), bytes), values)| ProviderWireToolCost {
            kind,
            count,
            bytes,
            estimated_tokens: bytes.div_ceil(4),
            sha256: json_hash(&Value::Array(values)),
        })
        .collect()
}

fn measure_taskspace_tool(
    item: &Value,
    counts: &mut [usize; 9],
    bytes: &mut [usize; 9],
    values: &mut [Vec<Value>; 9],
) {
    let Some(object) = item.as_object() else {
        add_whole(ToolCostKind::OtherTool, item, counts, bytes, values);
        return;
    };
    let protocol_bytes = object
        .get("description")
        .map(|value| field_bytes("description", value))
        .unwrap_or(0);
    if let Some(value) = object.get("description") {
        add(
            ToolCostKind::TaskspaceProtocol,
            1,
            protocol_bytes,
            value.clone(),
            counts,
            bytes,
            values,
        );
    }

    let mut catalog_bytes = 0usize;
    let mut map_bytes = 0usize;
    let mut sequence_bytes = 0usize;
    let mut map_defs = Map::new();
    let mut sequence = Map::new();
    if let Some(parameters) = object.get("parameters").and_then(Value::as_object) {
        for (field, value) in parameters {
            if field == "$defs" {
                if let Some(definitions) = value.as_object() {
                    for (name, definition) in definitions {
                        let measured = field_bytes(name, definition);
                        if name == TOOL_ACTION_DEF {
                            catalog_bytes += measured;
                            add(
                                ToolCostKind::TaskspaceClientCatalog,
                                1,
                                measured,
                                definition.clone(),
                                counts,
                                bytes,
                                values,
                            );
                        } else {
                            map_bytes += measured;
                            map_defs.insert(name.clone(), definition.clone());
                        }
                    }
                }
            } else {
                sequence_bytes += field_bytes(field, value);
                sequence.insert(field.clone(), value.clone());
            }
        }
    }
    if !map_defs.is_empty() {
        add(
            ToolCostKind::TaskspaceMapSchema,
            map_defs.len(),
            map_bytes,
            Value::Object(map_defs),
            counts,
            bytes,
            values,
        );
    }
    if !sequence.is_empty() {
        add(
            ToolCostKind::TaskspaceSequenceSchema,
            sequence.len(),
            sequence_bytes,
            Value::Object(sequence),
            counts,
            bytes,
            values,
        );
    }
    let measured = protocol_bytes + catalog_bytes + map_bytes + sequence_bytes;
    add(
        ToolCostKind::TaskspaceMetadata,
        1,
        json_bytes(item).len().saturating_sub(measured),
        item.clone(),
        counts,
        bytes,
        values,
    );
}

fn add_whole(
    kind: ToolCostKind,
    value: &Value,
    counts: &mut [usize; 9],
    bytes: &mut [usize; 9],
    values: &mut [Vec<Value>; 9],
) {
    add(
        kind,
        1,
        json_bytes(value).len(),
        value.clone(),
        counts,
        bytes,
        values,
    );
}

#[allow(clippy::too_many_arguments)]
fn add(
    kind: ToolCostKind,
    count: usize,
    measured_bytes: usize,
    value: Value,
    counts: &mut [usize; 9],
    bytes: &mut [usize; 9],
    values: &mut [Vec<Value>; 9],
) {
    counts[kind.index()] += count;
    bytes[kind.index()] += measured_bytes;
    values[kind.index()].push(value);
}

fn field_bytes(name: &str, value: &Value) -> usize {
    json_bytes(&Value::String(name.to_string())).len() + 1 + json_bytes(value).len()
}
