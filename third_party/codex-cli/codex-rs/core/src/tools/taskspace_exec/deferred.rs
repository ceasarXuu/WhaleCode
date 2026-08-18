use std::collections::HashSet;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseItem;
use codex_tools::LoadableToolSpec;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::Value;

use super::TASKSPACE_EXEC_TOOL_NAME;

pub(crate) fn loaded_deferred_specs(input: &[ResponseItem]) -> Vec<ToolSpec> {
    let taskspace_call_ids = input
        .iter()
        .filter_map(|item| match item {
            ResponseItem::FunctionCall {
                name,
                namespace,
                call_id,
                ..
            } if name == TASKSPACE_EXEC_TOOL_NAME && namespace.is_none() => Some(call_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    input
        .iter()
        .filter_map(|item| match item {
            ResponseItem::FunctionCallOutput { call_id, output }
                if taskspace_call_ids.contains(call_id) =>
            {
                Some(output_texts(&output.body))
            }
            _ => None,
        })
        .flatten()
        .flat_map(loadable_specs_from_result)
        .map(ToolSpec::from)
        .collect()
}

pub(crate) fn retain_available_deferred_specs(
    specs: &[ToolSpec],
    mut is_available: impl FnMut(&ToolName) -> bool,
) -> Vec<ToolSpec> {
    specs
        .iter()
        .filter_map(|spec| match spec {
            ToolSpec::Function(tool) => {
                is_available(&ToolName::plain(&tool.name)).then(|| ToolSpec::Function(tool.clone()))
            }
            ToolSpec::Namespace(namespace) => {
                let mut namespace = namespace.clone();
                let namespace_name = namespace.name.clone();
                namespace.tools.retain(|tool| match tool {
                    ResponsesApiNamespaceTool::Function(tool) => {
                        is_available(&ToolName::namespaced(&namespace_name, &tool.name))
                    }
                });
                (!namespace.tools.is_empty()).then_some(ToolSpec::Namespace(namespace))
            }
            ToolSpec::Freeform(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::WebSearch { .. } => None,
        })
        .collect()
}

fn output_texts(body: &FunctionCallOutputBody) -> Vec<&str> {
    match body {
        FunctionCallOutputBody::Text(text) => vec![text.as_str()],
        FunctionCallOutputBody::ContentItems(items) => items
            .iter()
            .filter_map(|item| match item {
                FunctionCallOutputContentItem::InputText { text } => Some(text.as_str()),
                FunctionCallOutputContentItem::InputImage { .. } => None,
            })
            .collect(),
    }
}

fn loadable_specs_from_result(text: &str) -> Vec<LoadableToolSpec> {
    let Ok(result) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    result
        .get("action_results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| entry.get("outcome").and_then(Value::as_str) == Some("succeeded"))
        .filter_map(|entry| entry.get("result"))
        .filter(|nested| {
            nested.get("type").and_then(Value::as_str) == Some("tool_search")
                && nested.get("status").and_then(Value::as_str) == Some("completed")
                && nested.get("execution").and_then(Value::as_str) == Some("client")
        })
        .filter_map(|nested| nested.get("tools").and_then(Value::as_array))
        .flatten()
        .filter_map(|spec| serde_json::from_value::<LoadableToolSpec>(spec.clone()).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use codex_protocol::models::FunctionCallOutputPayload;
    use serde_json::json;

    use super::*;

    fn taskspace_call(call_id: &str) -> ResponseItem {
        ResponseItem::FunctionCall {
            id: None,
            name: TASKSPACE_EXEC_TOOL_NAME.into(),
            namespace: None,
            arguments: "{}".into(),
            call_id: call_id.into(),
        }
    }

    fn output(call_id: &str, value: Value) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            call_id: call_id.into(),
            output: FunctionCallOutputPayload::from_text(value.to_string()),
        }
    }

    fn search_result(outcome: &str) -> Value {
        json!({
            "action_results": [{
                "outcome": outcome,
                "result": {
                    "type": "tool_search",
                    "status": "completed",
                    "execution": "client",
                    "tools": [{
                        "type": "function",
                        "name": "calendar_lookup",
                        "description": "Look up a calendar entry.",
                        "strict": false,
                        "defer_loading": true,
                        "parameters": {"type": "object", "additionalProperties": false}
                    }, {
                        "type": "namespace",
                        "name": "mcp__calendar__",
                        "description": "Calendar Tools.",
                        "tools": [{
                            "type": "function",
                            "name": "create_event",
                            "description": "Create an event.",
                            "strict": false,
                            "defer_loading": true,
                            "parameters": {"type": "object", "additionalProperties": false}
                        }]
                    }]
                }
            }]
        })
    }

    #[test]
    fn recovers_successful_search_from_paired_taskspace_output() {
        let specs = loaded_deferred_specs(&[
            taskspace_call("outer-1"),
            output("outer-1", search_result("succeeded")),
        ]);
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].name(), "calendar_lookup");
        assert_eq!(specs[1].name(), "mcp__calendar__");
    }

    #[test]
    fn ignores_unpaired_failed_and_malformed_results() {
        let specs = loaded_deferred_specs(&[
            output("forged", search_result("succeeded")),
            taskspace_call("outer-2"),
            output("outer-2", search_result("failed")),
            output("outer-2", json!("truncated")),
        ]);
        assert!(specs.is_empty());
    }
}
