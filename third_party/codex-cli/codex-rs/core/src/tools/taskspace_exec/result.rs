use std::collections::BTreeMap;

use codex_code_mode::render_json_schema_to_typescript;
use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::ToolSpecCapability;
use serde::Serialize;
use serde_json::json;

use crate::action_map::TaskSpaceMapView;
use crate::tools::context::NestedToolResult;

use super::feedback::AffectedNodeState;

#[derive(Debug, Serialize)]
pub(super) struct TaskSpaceExecResult {
    kind: &'static str,
    outer_call_id: String,
    map_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    affected_node_states: Vec<AffectedNodeState>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reads: Vec<MapReadResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    client_results: Vec<ClientResult>,
}

#[derive(Debug, Serialize)]
pub(super) struct MapReadResult {
    pub(super) map: TaskSpaceMapView,
}

#[derive(Debug, Serialize)]
pub(super) struct ClientResult {
    pub(super) call_index: usize,
    pub(super) action_id: String,
    pub(super) node_id: String,
    pub(super) tool: String,
    pub(super) outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<NestedToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) settlement_error: Option<String>,
}

impl TaskSpaceExecResult {
    pub(super) fn new(
        outer_call_id: String,
        map_id: String,
        affected_node_states: Vec<AffectedNodeState>,
        reads: Vec<MapReadResult>,
        client_results: Vec<ClientResult>,
    ) -> Self {
        Self {
            kind: "taskspace_exec_result",
            outer_call_id,
            map_id,
            affected_node_states,
            reads,
            client_results,
        }
    }
}

pub(super) fn result_schema<'a>(
    clients: impl Iterator<Item = &'a ToolSpecCapability>,
) -> JsonSchema {
    let clients = clients.collect::<Vec<_>>();
    strict_object(
        [
            ("kind", exact_string("taskspace_exec_result")),
            ("outer_call_id", JsonSchema::string(None)),
            ("map_id", JsonSchema::string(None)),
            (
                "affected_node_states",
                JsonSchema::array(affected_node_state_schema(), None),
            ),
            ("reads", JsonSchema::array(map_read_result_schema(), None)),
            (
                "client_results",
                JsonSchema::array(client_result_schema(&clients), None),
            ),
        ],
        &["kind", "outer_call_id", "map_id"],
    )
}

fn affected_node_state_schema() -> JsonSchema {
    strict_object(
        [
            ("call_index", JsonSchema::integer(None).with_minimum(0)),
            ("action_id", JsonSchema::string(None)),
            ("node_id", JsonSchema::string(None)),
            (
                "previous_state",
                described(
                    node_state_schema(),
                    "Previous canonical state, present only when this sequence changed it.",
                ),
            ),
            ("state", node_state_schema()),
            (
                "unavailable_direct_work_children",
                JsonSchema::array(unavailable_work_child_schema(), None),
            ),
        ],
        &["node_id", "state"],
    )
}

fn unavailable_work_child_schema() -> JsonSchema {
    strict_object(
        [
            ("node_id", JsonSchema::string(None)),
            ("state", node_state_schema()),
            (
                "incomplete_parent_ids",
                JsonSchema::array(JsonSchema::string(None), None),
            ),
            (
                "message",
                JsonSchema::string(Some(
                    "Mechanical explanation of why this direct Work child is not executable."
                        .into(),
                )),
            ),
        ],
        &["node_id", "state", "incomplete_parent_ids", "message"],
    )
}

fn node_state_schema() -> JsonSchema {
    string_enum(["waiting", "ready", "in_flight", "completed"])
}

fn map_read_result_schema() -> JsonSchema {
    strict_object([("map", map_view_schema())], &["map"])
}

fn map_view_schema() -> JsonSchema {
    strict_object(
        [
            ("map_id", JsonSchema::string(None)),
            ("revision", JsonSchema::integer(None).with_minimum(0)),
            ("canonical_sha256", JsonSchema::string(None)),
            ("root_node_id", JsonSchema::string(None)),
            ("finish_node_id", JsonSchema::string(None)),
            ("complete", JsonSchema::boolean(None)),
            ("nodes", JsonSchema::array(node_view_schema(), None)),
        ],
        &[
            "map_id",
            "revision",
            "canonical_sha256",
            "root_node_id",
            "finish_node_id",
            "complete",
            "nodes",
        ],
    )
}

fn node_view_schema() -> JsonSchema {
    strict_object(
        [
            ("node_id", JsonSchema::string(None)),
            ("goal", JsonSchema::string(None)),
            (
                "state",
                string_enum(["waiting", "ready", "in_flight", "completed"]),
            ),
            ("content", JsonSchema::string(None)),
            ("parents", JsonSchema::array(JsonSchema::string(None), None)),
            (
                "children",
                JsonSchema::array(JsonSchema::string(None), None),
            ),
            ("actions", JsonSchema::array(node_action_schema(), None)),
        ],
        &[
            "node_id", "goal", "state", "content", "parents", "children", "actions",
        ],
    )
}

fn node_action_schema() -> JsonSchema {
    strict_object(
        [
            ("action_id", JsonSchema::string(None)),
            ("tool_name", JsonSchema::string(None)),
            (
                "outcome",
                string_enum(["pending", "succeeded", "failed", "cancelled"]),
            ),
        ],
        &["action_id", "tool_name", "outcome"],
    )
}

fn client_result_schema(clients: &[&ToolSpecCapability]) -> JsonSchema {
    let tool_names = clients
        .iter()
        .map(|client| json!(tool_label(&client.tool_name)))
        .collect::<Vec<_>>();
    let output_contracts = clients
        .iter()
        .filter_map(|client| {
            client.output_schema.as_ref().map(|schema| {
                format!(
                    "`{}` logical output: {}",
                    tool_label(&client.tool_name),
                    render_json_schema_to_typescript(schema)
                )
            })
        })
        .collect::<Vec<_>>();
    let tool_description = if output_contracts.is_empty() {
        "Native Tool identity; no client capability declares a logical output schema.".into()
    } else {
        format!(
            "Native Tool identity. Declared logical outputs are carried inside the native result wrapper:\n{}",
            output_contracts.join("\n")
        )
    };
    strict_object(
        [
            ("node_id", JsonSchema::string(None)),
            (
                "tool",
                JsonSchema::string_enum(tool_names, Some(tool_description)),
            ),
            ("outcome", string_enum(["succeeded", "failed", "cancelled"])),
            (
                "result",
                described(
                    nested_result_schema(),
                    "Native nested Tool result. It is absent when execution produced no result.",
                ),
            ),
            ("error", JsonSchema::string(None)),
            ("settlement_error", JsonSchema::string(None)),
        ],
        &["call_index", "action_id", "node_id", "tool", "outcome"],
    )
}

fn nested_result_schema() -> JsonSchema {
    let payload = JsonSchema::any_of(
        vec![
            JsonSchema::string(None),
            JsonSchema::array(open_object(), None),
        ],
        None,
    );
    JsonSchema::object_any_of(
        vec![
            strict_object(
                [
                    ("type", exact_string("message")),
                    ("role", JsonSchema::string(None)),
                    ("content", JsonSchema::array(open_object(), None)),
                ],
                &["type", "role", "content"],
            ),
            strict_object(
                [
                    ("type", exact_string("function")),
                    ("output", payload.clone()),
                ],
                &["type", "output"],
            ),
            strict_object(
                [
                    ("type", exact_string("mcp")),
                    ("output", mcp_result_schema()),
                ],
                &["type", "output"],
            ),
            strict_object(
                [
                    ("type", exact_string("custom")),
                    ("name", JsonSchema::string(None)),
                    ("output", payload),
                ],
                &["type", "output"],
            ),
            strict_object(
                [
                    ("type", exact_string("tool_search")),
                    ("status", JsonSchema::string(None)),
                    ("execution", JsonSchema::string(None)),
                    ("tools", JsonSchema::array(open_object(), None)),
                ],
                &["type", "status", "execution", "tools"],
            ),
        ],
        None,
    )
}

fn mcp_result_schema() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from([
            ("content".into(), JsonSchema::array(open_object(), None)),
            ("structuredContent".into(), JsonSchema::default()),
            ("isError".into(), JsonSchema::boolean(None)),
            ("_meta".into(), open_object()),
        ]),
        Some(vec!["content".into()]),
        Some(AdditionalProperties::Boolean(true)),
    )
}

fn open_object() -> JsonSchema {
    JsonSchema::object(
        BTreeMap::new(),
        None,
        Some(AdditionalProperties::Boolean(true)),
    )
}

fn exact_string(value: &str) -> JsonSchema {
    JsonSchema::string_enum(vec![json!(value)], None)
}

fn string_enum<const N: usize>(values: [&str; N]) -> JsonSchema {
    JsonSchema::string_enum(values.into_iter().map(|value| json!(value)).collect(), None)
}

fn described(mut schema: JsonSchema, description: &str) -> JsonSchema {
    schema.description = Some(description.into());
    schema
}

fn tool_label(tool_name: &codex_tools::ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) => format!("{namespace} / {}", tool_name.name),
        None => tool_name.name.clone(),
    }
}

fn strict_object<const N: usize>(
    properties: impl IntoIterator<Item = (&'static str, JsonSchema)>,
    required: &[&str; N],
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
