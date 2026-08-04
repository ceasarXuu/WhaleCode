use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TaskSpaceToolsEnvelope {
    #[serde(default)]
    hosted_node_id: Option<String>,
    #[serde(default)]
    items: Vec<SequenceItem>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SequenceItem {
    ClientCall {
        item_id: String,
        node_id: String,
        tool: String,
        input: Value,
    },
    MapCall {
        item_id: String,
        tool: String,
        input: Value,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum SequenceShape {
    HostedOnly,
    MapReadOnly,
    TerminalOnly,
    ActionsOnly,
    PreludeActions,
    ActionsEpilogue,
    PreludeActionsEpilogue,
}

fn validate(value: Value) -> Result<(TaskSpaceToolsEnvelope, SequenceShape), String> {
    let envelope: TaskSpaceToolsEnvelope =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    let has_hosted_scope = envelope
        .hosted_node_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|node_id| !node_id.is_empty());
    if envelope.hosted_node_id.is_some() && !has_hosted_scope {
        return Err("hosted_node_id must be non-empty".to_string());
    }
    if envelope.items.is_empty() {
        return if has_hosted_scope {
            Ok((envelope, SequenceShape::HostedOnly))
        } else {
            Err("container must declare hosted scope or at least one item".to_string())
        };
    }

    let mut item_ids = BTreeSet::new();
    let mut patch_count = 0;
    let mut map_calls = Vec::new();
    for (index, item) in envelope.items.iter().enumerate() {
        let (item_id, tool) = match item {
            SequenceItem::ClientCall {
                item_id,
                node_id,
                tool,
                ..
            } => {
                if node_id.trim().is_empty() {
                    return Err(format!("items[{index}].node_id must be non-empty"));
                }
                if tool == "taskspace_control" {
                    return Err("taskspace_control must use map_call".to_string());
                }
                (item_id, tool)
            }
            SequenceItem::MapCall {
                item_id,
                tool,
                input,
            } => {
                if tool != "taskspace_control" {
                    return Err("map_call tool must be taskspace_control".to_string());
                }
                let action = input
                    .as_object()
                    .and_then(|input| input.get("action"))
                    .and_then(Value::as_str)
                    .filter(|action| !action.trim().is_empty())
                    .ok_or_else(|| "map_call input requires a non-empty action".to_string())?;
                map_calls.push((index, action));
                (item_id, tool)
            }
        };
        if item_id.trim().is_empty() {
            return Err(format!("items[{index}].item_id must be non-empty"));
        }
        if !item_ids.insert(item_id.as_str()) {
            return Err(format!("duplicate item_id `{item_id}`"));
        }
        if tool.trim().is_empty() {
            return Err(format!("items[{index}].tool must be non-empty"));
        }
        if tool == "taskspace_tools" {
            return Err("taskspace_tools cannot contain itself".to_string());
        }
        patch_count += usize::from(tool == "apply_patch");
    }
    if patch_count > 1 {
        return Err("at most one apply_patch is allowed".to_string());
    }

    let shape = classify_shape(&envelope.items, &map_calls)?;
    Ok((envelope, shape))
}

fn classify_shape(
    items: &[SequenceItem],
    map_calls: &[(usize, &str)],
) -> Result<SequenceShape, String> {
    match map_calls {
        [] => Ok(SequenceShape::ActionsOnly),
        [(0, action)] if items.len() == 1 && is_read(action) => Ok(SequenceShape::MapReadOnly),
        [(0, "finish_map")] if items.len() == 1 => Ok(SequenceShape::TerminalOnly),
        [(0, action)] if items.len() > 1 && is_prelude(action) => Ok(SequenceShape::PreludeActions),
        [(index, action)]
            if items.len() > 1 && *index + 1 == items.len() && is_epilogue(action) =>
        {
            Ok(SequenceShape::ActionsEpilogue)
        }
        [(0, prelude), (epilogue_index, epilogue)]
            if items.len() > 2
                && *epilogue_index + 1 == items.len()
                && is_prelude(prelude)
                && is_epilogue(epilogue) =>
        {
            Ok(SequenceShape::PreludeActionsEpilogue)
        }
        _ => Err("map calls must occupy a supported prelude/read/epilogue boundary".to_string()),
    }
}

fn is_read(action: &str) -> bool {
    matches!(action, "read_map" | "read_output_ref")
}

fn is_prelude(action: &str) -> bool {
    matches!(action, "initialize_and_execute" | "reopen_map" | "execute")
}

fn is_epilogue(action: &str) -> bool {
    matches!(action, "execute" | "finish_map")
}

fn client(item_id: &str, node_id: &str, tool: &str, input: Value) -> Value {
    serde_json::json!({
        "kind": "client_call",
        "item_id": item_id,
        "node_id": node_id,
        "tool": tool,
        "input": input,
    })
}

fn map(item_id: &str, action: &str) -> Value {
    serde_json::json!({
        "kind": "map_call",
        "item_id": item_id,
        "tool": "taskspace_control",
        "input": {"action": action},
    })
}

#[test]
fn accepts_every_supported_boundary_shape() {
    let cases = [
        (
            serde_json::json!({"hosted_node_id": "research"}),
            SequenceShape::HostedOnly,
        ),
        (
            serde_json::json!({"items": [map("read", "read_map")]}),
            SequenceShape::MapReadOnly,
        ),
        (
            serde_json::json!({"items": [map("finish", "finish_map")]}),
            SequenceShape::TerminalOnly,
        ),
        (
            serde_json::json!({"items": [client("read", "inspect", "read_file", serde_json::json!({"path":"README.md"}))]}),
            SequenceShape::ActionsOnly,
        ),
        (
            serde_json::json!({"items": [map("init", "initialize_and_execute"), client("read", "inspect", "read_file", serde_json::json!({}))]}),
            SequenceShape::PreludeActions,
        ),
        (
            serde_json::json!({"items": [client("test", "verify", "exec_command", serde_json::json!({})), map("complete", "execute")]}),
            SequenceShape::ActionsEpilogue,
        ),
        (
            serde_json::json!({"items": [map("advance", "execute"), client("patch", "implement", "apply_patch", Value::String("*** Begin Patch".into())), map("finish", "finish_map")]}),
            SequenceShape::PreludeActionsEpilogue,
        ),
    ];

    for (value, expected) in cases {
        assert_eq!(validate(value).expect("valid shape").1, expected);
    }
}

#[test]
fn preserves_native_function_and_freeform_inputs_without_interpretation() {
    let function_input = serde_json::json!({"cmd": "cargo test", "yield_time_ms": 30000});
    let freeform_input = Value::String("*** Begin Patch\n*** End Patch".to_string());
    let (envelope, _) = validate(serde_json::json!({
        "items": [
            client("exec", "verify", "exec_command", function_input.clone()),
            client("patch", "implement", "apply_patch", freeform_input.clone())
        ]
    }))
    .expect("native payloads");

    assert!(matches!(
        &envelope.items[0],
        SequenceItem::ClientCall { input, .. } if input == &function_input
    ));
    assert!(matches!(
        &envelope.items[1],
        SequenceItem::ClientCall { input, .. } if input == &freeform_input
    ));
}

#[test]
fn rejects_ambiguous_map_positions_and_nonterminal_map_only_calls() {
    let invalid = [
        serde_json::json!({"items": [client("a", "a", "read_file", serde_json::json!({})), map("middle", "execute"), client("b", "b", "exec_command", serde_json::json!({}))]}),
        serde_json::json!({"items": [map("execute", "execute")]}),
        serde_json::json!({"items": [map("read", "read_map"), client("a", "a", "read_file", serde_json::json!({}))]}),
        serde_json::json!({"items": [client("a", "a", "read_file", serde_json::json!({})), map("reopen", "reopen_map")]}),
        serde_json::json!({"items": [map("finish", "finish_map"), client("a", "a", "read_file", serde_json::json!({}))]}),
    ];

    for value in invalid {
        assert!(validate(value).is_err());
    }
}

#[test]
fn rejects_identity_recursion_patch_and_hosted_scope_violations() {
    let invalid = [
        serde_json::json!({}),
        serde_json::json!({"hosted_node_id": " "}),
        serde_json::json!({"hosted_node_id": "research", "provider_item_id": "call_00"}),
        serde_json::json!({"items": [client("same", "a", "read_file", serde_json::json!({})), client("same", "b", "exec_command", serde_json::json!({}))]}),
        serde_json::json!({"items": [client("", "a", "read_file", serde_json::json!({}))]}),
        serde_json::json!({"items": [client("nested", "a", "taskspace_tools", serde_json::json!({}))]}),
        serde_json::json!({"items": [client("control", "a", "taskspace_control", serde_json::json!({}))]}),
        serde_json::json!({"items": [{"kind":"map_call", "item_id":"map", "tool":"read_file", "input":{"action":"read_map"}}]}),
        serde_json::json!({"items": [client("p1", "a", "apply_patch", Value::String("one".into())), client("p2", "b", "apply_patch", Value::String("two".into()))]}),
        serde_json::json!({"items": [{"kind":"provider_result", "item_id":"hosted", "node_id":"research", "provider_item_id":"call_00"}]}),
    ];

    for value in invalid {
        assert!(validate(value).is_err());
    }
}
