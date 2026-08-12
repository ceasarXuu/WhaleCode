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

const PROTOCOL: &str = r#"Use `taskspace_exec` as the single top-level entry point for TaskSpace Map and Tool actions. Choose exactly one sequence `type` allowed by the schema.

Tool contract:
- Put client and provider-hosted Tool actions in the sequence's single `tools` array.
- Each client action keeps its native Tool input in `input` and declares one owner `node_id`; namespaced Tools also declare `namespace`.
- Each provider-hosted action is the node-ownership declaration for one same-response output item already executed by the Provider; its schema defines complete per-item coverage and it carries no native Tool input.
- Tool array order supplies stable action identity and provider-fact pairing. It does not create Tool dependencies; result-dependent work belongs in a later request.

Map contract:
- Map fields use the canonical operation input directly. Node readiness derives from `parents`.
- A Tool action on a Ready owner mechanically starts it; do not also patch that owner to `in_flight` in the same sequence.
- Tool outcomes do not complete nodes. A batch contains at most one `apply_patch` action.
- The complete sequence is preflighted before unexecuted side effects. The Runtime does not add, infer, reorder, or repair Agent actions.

Feedback contract:
- The outer result reports every client and hosted action, preserves native client results and errors without summarization, and returns the complete Map for `read_map`."#;

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
        sections.push(format!(
            "Parent completion and direct-child work example:\n```json\n{}\n```",
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
            "Available provider-hosted Tool actions: {}.",
            hosted_tools.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
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
        "type": "initialize_and_work",
        "initialize_map": map_input(initialize),
        "tools": [{
            "tool": "exec_command",
            "node_id": "inspect",
            "input": {"cmd": "pwd"}
        }]
    })
}

pub(crate) fn canonical_read_example() -> Value {
    json!({
        "type": "read_map",
        "read_map": map_input(MapOperation::ReadMap(EmptyArgs::default()))
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
        "type": "update_and_work",
        "update_map": map_input(complete),
        "tools": [{
            "tool": "exec_command",
            "node_id": "implement",
            "input": {"cmd": "test -f README.md"}
        }]
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
        "type": "update_and_finish",
        "update_map": map_input(complete),
        "finish_map": map_input(finish)
    })
}

fn map_input(operation: MapOperation) -> Value {
    let serialized =
        serde_json::to_value(operation).expect("canonical Map operation must serialize");
    serialized["arguments"].clone()
}
