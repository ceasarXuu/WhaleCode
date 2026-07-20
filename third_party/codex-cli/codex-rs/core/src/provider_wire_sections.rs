use std::collections::HashSet;

use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

const ACTIVE_PROJECTION_START: &str = "TaskSpaceMapProjectionR7V1:";
const ACTIVE_PROJECTION_END: &str = "TaskSpaceMapProjectionR7V1 end.";
const TASKSPACE_CONTROL_RESULT_MARKER: &str = "TaskSpaceControlResultV2";

#[derive(Debug, Serialize)]
pub(super) struct ProviderWireSectionCost {
    schema_version: &'static str,
    availability: &'static str,
    unavailable_reason: Option<&'static str>,
    pub(super) section_bytes_total: usize,
    active_projection_identity: ActiveProjectionIdentity,
    sections: Vec<ProviderWireSection>,
}

#[derive(Debug, Serialize)]
struct ActiveProjectionIdentity {
    count: usize,
    kind: &'static str,
    map_id_sha256: Option<String>,
    revision: Option<u64>,
    canonical_sha256: Option<String>,
    projection_sha256: Option<String>,
    unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct ProviderWireSection {
    kind: SectionKind,
    count: usize,
    bytes: usize,
    estimated_tokens: usize,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SectionKind {
    SystemMessages,
    NaturalHistory,
    ActiveProjection,
    TaskspaceControlFeedback,
    OrdinaryToolFeedback,
    Tools,
    ToolChoice,
    OtherPayload,
}

impl SectionKind {
    const ALL: [Self; 8] = [
        Self::SystemMessages,
        Self::NaturalHistory,
        Self::ActiveProjection,
        Self::TaskspaceControlFeedback,
        Self::OrdinaryToolFeedback,
        Self::Tools,
        Self::ToolChoice,
        Self::OtherPayload,
    ];

    const fn index(self) -> usize {
        match self {
            Self::SystemMessages => 0,
            Self::NaturalHistory => 1,
            Self::ActiveProjection => 2,
            Self::TaskspaceControlFeedback => 3,
            Self::OrdinaryToolFeedback => 4,
            Self::Tools => 5,
            Self::ToolChoice => 6,
            Self::OtherPayload => 7,
        }
    }
}

#[derive(Default)]
struct SectionMeasure {
    count: usize,
    bytes: usize,
}

struct SectionBuild {
    measures: [SectionMeasure; 8],
    message_values: [Vec<Value>; 5],
    tools_value: Value,
    tool_choice_value: Value,
    other_value: Map<String, Value>,
}

impl Default for SectionBuild {
    fn default() -> Self {
        Self {
            measures: std::array::from_fn(|_| SectionMeasure::default()),
            message_values: std::array::from_fn(|_| Vec::new()),
            tools_value: Value::Null,
            tool_choice_value: Value::Null,
            other_value: Map::new(),
        }
    }
}

impl ProviderWireSectionCost {
    pub(super) fn measure(wire: &Value, messages_field: &str) -> Self {
        let section_bytes_total = json_bytes(wire).len();
        let mut build = SectionBuild::default();

        let unavailable_reason = match wire.as_object() {
            Some(payload) => measure_object(payload, messages_field, &mut build),
            None => {
                let other = &mut build.measures[SectionKind::OtherPayload.index()];
                other.count = 1;
                other.bytes = section_bytes_total;
                build
                    .other_value
                    .insert("payload".to_string(), wire.clone());
                Some("provider_payload_not_object")
            }
        };

        let active_projection_identity = active_projection_identity(&build.message_values[2]);
        let hashes = section_hashes(
            build.message_values,
            build.tools_value,
            build.tool_choice_value,
            build.other_value,
        );
        let sections = SectionKind::ALL
            .into_iter()
            .zip(build.measures)
            .zip(hashes)
            .map(|((kind, measure), sha256)| ProviderWireSection {
                kind,
                count: measure.count,
                bytes: measure.bytes,
                estimated_tokens: measure.bytes.div_ceil(4),
                sha256,
            })
            .collect::<Vec<_>>();

        debug_assert_eq!(
            sections.iter().map(|section| section.bytes).sum::<usize>(),
            section_bytes_total,
            "provider wire section accounting must assign every payload byte exactly once"
        );

        Self {
            schema_version: "provider-wire-section-cost-v1",
            availability: if unavailable_reason.is_some() {
                "unavailable"
            } else {
                "measured"
            },
            unavailable_reason,
            section_bytes_total,
            active_projection_identity,
            sections,
        }
    }
}

fn measure_object(
    payload: &Map<String, Value>,
    messages_field: &str,
    build: &mut SectionBuild,
) -> Option<&'static str> {
    let other_index = SectionKind::OtherPayload.index();
    build.measures[other_index].bytes = 2 + payload.len().saturating_sub(1);
    let mut unavailable_reason = Some("message_array_missing");

    for (field, value) in payload {
        let field_prefix_bytes = json_bytes(&Value::String(field.clone())).len() + 1;
        if field == messages_field {
            if let Some(messages) = value.as_array() {
                unavailable_reason = None;
                build.measures[other_index].bytes +=
                    field_prefix_bytes + 2 + messages.len().saturating_sub(1);
                measure_messages(messages, &mut build.measures, &mut build.message_values);
            } else {
                unavailable_reason = Some("message_array_not_array");
                build.measures[other_index].count += 1;
                build.measures[other_index].bytes += field_prefix_bytes + json_bytes(value).len();
                build.other_value.insert(field.clone(), value.clone());
            }
        } else if field == "tools" {
            let tools = &mut build.measures[SectionKind::Tools.index()];
            tools.count = value.as_array().map(Vec::len).unwrap_or(1);
            tools.bytes = field_prefix_bytes + json_bytes(value).len();
            build.tools_value.clone_from(value);
        } else if field == "tool_choice" {
            let tool_choice = &mut build.measures[SectionKind::ToolChoice.index()];
            tool_choice.count = 1;
            tool_choice.bytes = field_prefix_bytes + json_bytes(value).len();
            build.tool_choice_value.clone_from(value);
        } else {
            build.measures[other_index].count += 1;
            build.measures[other_index].bytes += field_prefix_bytes + json_bytes(value).len();
            build.other_value.insert(field.clone(), value.clone());
        }
    }

    unavailable_reason
}

fn measure_messages(
    messages: &[Value],
    measures: &mut [SectionMeasure; 8],
    message_values: &mut [Vec<Value>; 5],
) {
    for message in messages {
        let serialized = json_bytes(message);
        let kind = classify_message(message);
        let measure = &mut measures[kind.index()];
        measure.count += 1;
        measure.bytes += serialized.len();
        message_values[kind.index()].push(message.clone());
    }
}

fn classify_message(message: &Value) -> SectionKind {
    match message.get("role").and_then(Value::as_str) {
        Some("system" | "developer" | "user")
            if projection_blocks(message).is_ok_and(|blocks| !blocks.is_empty()) =>
        {
            SectionKind::ActiveProjection
        }
        Some("system" | "developer") => SectionKind::SystemMessages,
        Some("tool") if is_taskspace_control_feedback(message) => {
            SectionKind::TaskspaceControlFeedback
        }
        Some("tool") => SectionKind::OrdinaryToolFeedback,
        Some(_) | None => SectionKind::NaturalHistory,
    }
}

fn is_taskspace_control_feedback(message: &Value) -> bool {
    let Some(content) = message.get("content") else {
        return false;
    };
    let has_control_schema = |value: &Value| {
        value.get("schema_version").and_then(Value::as_str) == Some(TASKSPACE_CONTROL_RESULT_MARKER)
    };
    match content {
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .is_some_and(|value| has_control_schema(&value)),
        value => has_control_schema(value),
    }
}

fn active_projection_identity(messages: &[Value]) -> ActiveProjectionIdentity {
    if messages.is_empty() {
        return ActiveProjectionIdentity::unavailable(0, "active_projection_missing", None);
    }
    let mut projections = Vec::new();
    for message in messages {
        match projection_blocks(message) {
            Ok(blocks) => projections.extend(blocks),
            Err(reason) => {
                return ActiveProjectionIdentity::unavailable(messages.len(), reason, None);
            }
        }
    }
    if projections.is_empty() {
        return ActiveProjectionIdentity::unavailable(0, "active_projection_missing", None);
    }
    if !projection_revision_sequence_valid(&projections) {
        return ActiveProjectionIdentity::unavailable(
            projections.len(),
            "projection_revision_order_invalid",
            None,
        );
    }
    let projection = projections[projections.len() - 1];
    let projection_sha256 = Some(byte_hash(projection.as_bytes()));
    let is_bootstrap = projection.lines().any(|line| line == "- map: none")
        && projection
            .lines()
            .any(|line| line == "- bootstrap_required: true");
    if is_bootstrap {
        return ActiveProjectionIdentity {
            count: projections.len(),
            kind: "bootstrap_required",
            map_id_sha256: None,
            revision: None,
            canonical_sha256: None,
            projection_sha256,
            unavailable_reason: None,
        };
    }

    let Some(map_id) = mechanical_field(projection, "map_id") else {
        return ActiveProjectionIdentity::unavailable(
            projections.len(),
            "projection_map_id_missing",
            projection_sha256,
        );
    };
    let Some(revision) = mechanical_field(projection, "revision") else {
        return ActiveProjectionIdentity::unavailable(
            projections.len(),
            "projection_revision_missing",
            projection_sha256,
        );
    };
    let Ok(revision) = revision.parse::<u64>() else {
        return ActiveProjectionIdentity::unavailable(
            projections.len(),
            "projection_revision_invalid",
            projection_sha256,
        );
    };
    let Some(canonical_sha256) = mechanical_field(projection, "canonical_sha256") else {
        return ActiveProjectionIdentity::unavailable(
            projections.len(),
            "projection_canonical_sha256_missing",
            projection_sha256,
        );
    };

    ActiveProjectionIdentity {
        count: projections.len(),
        kind: match mechanical_field(projection, "projection_kind") {
            Some("request_snapshot") => "request_snapshot",
            Some("current_projection") => "current_projection",
            _ => "unavailable",
        },
        map_id_sha256: Some(byte_hash(map_id.as_bytes())),
        revision: Some(revision),
        canonical_sha256: Some(canonical_sha256.to_string()),
        projection_sha256,
        unavailable_reason: None,
    }
}

fn projection_revision_sequence_valid(projections: &[&str]) -> bool {
    if projections.len() < 2 {
        return true;
    }
    let mut current_map_id = None;
    let mut previous_revision = None;
    let mut closed_map_ids = HashSet::new();
    for projection in projections {
        if projection_is_bootstrap(projection) {
            if current_map_id.is_some() || !closed_map_ids.is_empty() {
                return false;
            }
            continue;
        }
        let Some(map_id) = mechanical_field(projection, "map_id") else {
            return false;
        };
        let Some(revision) =
            mechanical_field(projection, "revision").and_then(|value| value.parse::<u64>().ok())
        else {
            return false;
        };
        if current_map_id.is_some_and(|current| current != map_id) {
            if let Some(current) = current_map_id {
                closed_map_ids.insert(current);
            }
            if closed_map_ids.contains(map_id) {
                return false;
            }
        } else if previous_revision.is_some_and(|previous| previous > revision) {
            return false;
        }
        current_map_id = Some(map_id);
        previous_revision = Some(revision);
    }
    true
}

fn projection_is_bootstrap(projection: &str) -> bool {
    projection.lines().any(|line| line == "- map: none")
        && projection
            .lines()
            .any(|line| line == "- bootstrap_required: true")
}

impl ActiveProjectionIdentity {
    fn unavailable(
        count: usize,
        unavailable_reason: &'static str,
        projection_sha256: Option<String>,
    ) -> Self {
        Self {
            count,
            kind: "unavailable",
            map_id_sha256: None,
            revision: None,
            canonical_sha256: None,
            projection_sha256,
            unavailable_reason: Some(unavailable_reason),
        }
    }
}

fn projection_blocks(message: &Value) -> Result<Vec<&str>, &'static str> {
    let Some(content) = message.get("content") else {
        return Err("projection_content_missing");
    };
    let mut strings = Vec::new();
    collect_strings(content, &mut strings);
    let mut projections = Vec::new();
    for text in strings {
        let mut remainder = text;
        while let Some(start) = remainder.find(ACTIVE_PROJECTION_START) {
            let candidate = &remainder[start..];
            let Some(end) = candidate.find(ACTIVE_PROJECTION_END) else {
                return Err("projection_block_unterminated");
            };
            let end = end + ACTIVE_PROJECTION_END.len();
            projections.push(&candidate[..end]);
            remainder = &candidate[end..];
        }
    }
    Ok(projections)
}

fn collect_strings<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => strings.push(text),
        Value::Array(values) => {
            for value in values {
                collect_strings(value, strings);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_strings(value, strings);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn mechanical_field<'a>(projection: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("- {field}: ");
    projection
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .filter(|value| !value.is_empty())
}

fn section_hashes(
    message_values: [Vec<Value>; 5],
    tools_value: Value,
    tool_choice_value: Value,
    other_value: Map<String, Value>,
) -> [String; 8] {
    let [system, natural, projection, control, ordinary] = message_values;
    [
        json_hash(&Value::Array(system)),
        json_hash(&Value::Array(natural)),
        json_hash(&Value::Array(projection)),
        json_hash(&Value::Array(control)),
        json_hash(&Value::Array(ordinary)),
        json_hash(&tools_value),
        json_hash(&tool_choice_value),
        json_hash(&Value::Object(other_value)),
    ]
}

fn json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

fn json_hash(value: &Value) -> String {
    byte_hash(&json_bytes(value))
}

fn byte_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "provider_wire_sections_tests.rs"]
mod tests;
