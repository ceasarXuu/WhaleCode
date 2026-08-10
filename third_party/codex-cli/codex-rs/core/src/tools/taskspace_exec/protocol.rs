use std::collections::BTreeSet;

use serde_json::Value;
use serde_json::json;

use crate::action_map::rooted_dag::NodeState;

use super::MapOperation;
use super::map_operations::BoundaryNodeArgs;
use super::map_operations::EmptyArgs;
use super::map_operations::FinishMapArgs;
use super::map_operations::InitializeMapArgs;
use super::map_operations::NodePatchArgs;
use super::map_operations::UpdateMapArgs;
use super::map_operations::WorkNodeArgs;

const PROTOCOL: &str = r#"Use `taskspace_exec` as the single top-level entry point for TaskSpace Map operations and client Tool calls. Submit one Agent-authored batch; the Runtime only validates and executes it.

Call contract:
- Put every Map operation and client Tool invocation in `calls`, in the order you declare.
- `calls` order defines Map boundaries, not a second dependency graph for ordinary work. Work dependencies come only from Map node `parents`; independent client calls may use native parallel execution, while result-dependent work waits for a later request.
- A client Tool call is `{"client":{"name":"<name>","node_id":"<work-node>","input":<native-input>}}`. A namespaced function call also has `"namespace":"<namespace>"` and keeps only the leaf Tool name in `name`. `node_id` is TaskSpace ownership metadata outside the Tool's native input.
- A Map operation is `{"map":{"operation":"<map-operation>","input":{...}}}` and has no owner `node_id`.
- Include `hosted_bindings` only when the response contains provider-hosted output. It contains one binding for each hosted output, in provider output order; one output may name multiple owner work nodes.

Sequence contract:
- `initialize_map` or `reopen_map`, when present, is first and shares the batch with real client or hosted work.
- `read_map` is the only call in its batch and cannot accompany hosted output.
- `finish_map` is last.
- An `update_map` that completes a work node shares the batch with later client work, hosted work, or `finish_map`.
- When parent completion unlocks a dependent Work node, patch only the parent to `completed` and put the dependent node's client work later in the same batch. The Map derives the dependent node's readiness after the update; do not also patch that `waiting` node to `ready` or `in_flight`.
- A batch contains at most one `apply_patch` call.
- The complete batch is preflighted before side effects. The Runtime does not add, infer, reorder, or repair Agent actions."#;

pub(super) fn build_description<'a>(
    client_tool_names: impl Iterator<Item = &'a str>,
    hosted_tools: &BTreeSet<String>,
    result_contract: &str,
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
        sections.push(format!(
            "Parent completion and dependent-node work example:\n```json\n{}\n```",
            canonical_handoff_example()
        ));
    }
    sections.push(format!(
        "Read-only example:\n```json\n{}\n```",
        canonical_read_example()
    ));
    sections.push(format!(
        "Final work-node completion and explicit Map finish example:\n```json\n{}\n```",
        canonical_finish_example()
    ));
    if !hosted_tools.is_empty() {
        sections.push(format!(
            "Available provider-hosted Tool types for `hosted_bindings`: {}.",
            hosted_tools.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    sections.push(result_contract.to_string());
    sections.join("\n\n")
}

pub(crate) fn canonical_first_turn_example() -> Value {
    let initialize = MapOperation::InitializeMap(InitializeMapArgs {
        root: BoundaryNodeArgs {
            node_id: "root".into(),
            goal: "Complete the user task".into(),
            content: String::new(),
            parents: Vec::new(),
        },
        work_nodes: vec![WorkNodeArgs {
            node_id: "inspect".into(),
            goal: "Inspect the workspace".into(),
            content: String::new(),
            parents: vec!["root".into()],
        }],
        finish: BoundaryNodeArgs {
            node_id: "finish".into(),
            goal: "Deliver the completed task".into(),
            content: String::new(),
            parents: vec!["inspect".into()],
        },
    });
    json!({
        "calls": [
            map_call(initialize),
            {
                "client": {
                    "name": "exec_command",
                    "node_id": "inspect",
                    "input": {"cmd": "pwd"}
                }
            }
        ]
    })
}

pub(crate) fn canonical_read_example() -> Value {
    json!({
        "calls": [map_call(MapOperation::ReadMap(EmptyArgs::default()))]
    })
}

pub(crate) fn canonical_handoff_example() -> Value {
    let complete = MapOperation::UpdateMap(UpdateMapArgs {
        add_work_nodes: Vec::new(),
        node_patches: vec![NodePatchArgs {
            node_id: "inspect".into(),
            goal: None,
            state: Some(NodeState::Completed),
            content: Some("Inspection complete.".into()),
            parents: None,
        }],
    });
    json!({
        "calls": [
            map_call(complete),
            {
                "client": {
                    "name": "exec_command",
                    "node_id": "implement",
                    "input": {"cmd": "test -f README.md"}
                }
            }
        ]
    })
}

pub(crate) fn canonical_finish_example() -> Value {
    let complete = MapOperation::UpdateMap(UpdateMapArgs {
        add_work_nodes: Vec::new(),
        node_patches: vec![NodePatchArgs {
            node_id: "implement".into(),
            goal: None,
            state: Some(NodeState::Completed),
            content: Some("Implementation and verification complete.".into()),
            parents: None,
        }],
    });
    let finish = MapOperation::FinishMap(FinishMapArgs {
        content: "Task completed and verified.".into(),
    });
    json!({
        "calls": [map_call(complete), map_call(finish)]
    })
}

fn map_call(operation: MapOperation) -> Value {
    let serialized =
        serde_json::to_value(operation).expect("canonical Map operation must serialize");
    json!({
        "map": {
            "operation": serialized["tool"],
            "input": serialized["arguments"]
        }
    })
}
