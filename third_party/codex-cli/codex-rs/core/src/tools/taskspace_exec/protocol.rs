use std::collections::BTreeSet;

use serde_json::Value;
use serde_json::json;

const PROTOCOL: &str = r#"Use `taskspace_exec` as the single top-level entry point for TaskSpace Map operations and client Tool calls. Submit one Agent-authored batch; the Runtime only validates and executes it.

Call contract:
- Put every Map operation and client Tool invocation in `calls`, in the order you declare.
- A plain function client Tool call is `{"tool":"<name>","node_id":"<work-node>","arguments":{...}}`; a namespaced function call also has `"namespace":"<namespace>"` and keeps only the leaf Tool name in `tool`. A freeform client Tool uses `input` instead of `arguments`. `node_id` is TaskSpace ownership metadata outside the Tool's native input.
- A Map operation is `{"tool":"<map-operation>","arguments":{...}}` and has no outer `node_id`.
- `hosted_bindings` contains one binding for each provider-hosted output, in provider output order. One hosted output may name multiple owner work nodes. Use an empty array when there is no hosted output.

Sequence contract:
- `initialize_map` or `reopen_map`, when present, is first and shares the batch with real client or hosted work.
- `read_map` is the only call in its batch and cannot accompany hosted output.
- `finish_map` is last.
- An `update_map` that completes a work node shares the batch with later client work, hosted work, or `finish_map`.
- A batch contains at most one `apply_patch` call.
- The complete batch is preflighted before side effects. The Runtime does not add, infer, reorder, or repair Agent actions."#;

pub(super) fn build_description<'a>(
    client_tool_names: impl Iterator<Item = &'a str>,
    hosted_tools: &BTreeSet<String>,
) -> String {
    let has_exec_command = client_tool_names
        .into_iter()
        .any(|name| name == "exec_command");
    let mut sections = vec![PROTOCOL.to_string()];
    if has_exec_command {
        sections.push(format!(
            "First-turn initialization and work example:\n```json\n{}\n```",
            canonical_first_turn_example()
        ));
    }
    sections.push(format!(
        "Read-only example:\n```json\n{}\n```",
        json!({
            "calls": [{"tool": "read_map", "arguments": {}}],
            "hosted_bindings": []
        })
    ));
    if !hosted_tools.is_empty() {
        sections.push(format!(
            "Available provider-hosted Tool types for `hosted_bindings`: {}.",
            hosted_tools.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    sections.join("\n\n")
}

pub(crate) fn canonical_first_turn_example() -> Value {
    json!({
        "calls": [
            {
                "tool": "initialize_map",
                "arguments": {
                    "root": {
                        "node_id": "root",
                        "goal": "Complete the user task",
                        "content": "",
                        "parents": []
                    },
                    "work_nodes": [{
                        "node_id": "inspect",
                        "goal": "Inspect the workspace",
                        "content": "",
                        "parents": ["root"]
                    }],
                    "finish": {
                        "node_id": "finish",
                        "goal": "Deliver the completed task",
                        "content": "",
                        "parents": ["inspect"]
                    }
                }
            },
            {
                "tool": "exec_command",
                "node_id": "inspect",
                "arguments": {"cmd": "pwd"}
            }
        ],
        "hosted_bindings": []
    })
}
