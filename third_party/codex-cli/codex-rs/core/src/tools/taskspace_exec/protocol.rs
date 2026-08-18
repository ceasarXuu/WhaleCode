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

const PROTOCOL: &str = r#"Use `taskspace_exec` as the sole top-level Function Tool for TaskSpace Map operations and client Tool actions. Provider-hosted Tools remain native Provider ToolSpecs; they are not Function Tools. Choose exactly one sequence `type` allowed by the schema.

Tool contract:
- Put only client Tool actions in the sequence's `tools` array. Every work sequence requires a non-empty `tools` array. Native Provider Tool actions remain separate and do not replace this client work.
- Each client action keeps its native Tool input in `input` and declares one owner `node_id`; namespaced Tools also declare `namespace`.
- Tool array order supplies stable action identity. It does not create Tool dependencies; result-dependent work belongs in a later request.

Node state-machine contract:
- Roles: `parents` defines the dependency DAG. Root stays `in_flight` while the Map is open and counts as satisfied for its direct Work children. Work nodes carry the task steps. Finish is the unique terminal node.
- Work states: `waiting` means at least one non-Root parent is incomplete and the node is not executable; `ready` means every non-Root parent is `completed` and the node is executable; `in_flight` means the Agent has started work; `completed` means the Agent has explicitly recorded completion.
- Runtime-derived transitions: initialization, adding a Work node, or changing `parents` derives each not-started Work node as `waiting` or `ready`. Later parent completion rederives eligible `waiting` children as `ready`; changing parents may rederive a not-started node between `waiting` and `ready`.
- Agent-triggered transitions: a state patch may perform only `ready -> in_flight`, `ready -> completed`, or `in_flight -> completed`. No other explicit state transition is accepted.
- Tool-triggered transition: dispatching a Tool action on a `ready` owner mechanically performs `ready -> in_flight`; do not also patch that owner to `in_flight` in the same sequence. Tool success, failure, or cancellation records an outcome but never completes the owner.
- Commit timing: the sequence's Map operation is applied before its Tool actions. Map patches are applied in declared array order, with `waiting`/`ready` rederived after each patch; a later patch may therefore complete a child unlocked by an earlier parent-completion patch. Any invalid patch rejects the whole sequence with no commit. Tool outcomes do not unlock descendants.
- Boundary lifecycle: Finish readiness is Runtime-derived from its parents. Only `finish_map` may change ready Finish and open Root to `completed`. `reopen_map` returns Root to `in_flight` and rederives Finish after user follow-up; completed Work nodes remain completed.
- A batch contains at most one `apply_patch` action.
- The complete sequence is preflighted before unexecuted side effects. The Runtime does not add, infer, reorder, or repair Agent actions.

Feedback contract:
- The outer result reports every client action, preserves native client results and errors without summarization, returns current states for directly operated or mechanically changed nodes, identifies their unavailable direct Work children with exact incomplete parents, and returns the complete Map for `read_map`."#;

pub(super) fn build_description<'a>(client_tool_names: impl Iterator<Item = &'a str>) -> String {
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
        work_nodes: vec![
            WorkNodeArgs {
                node_id: "inspect".into(),
                goal: "Inspect the workspace".into(),
                content: String::new(),
                parents: vec!["root".into()],
            },
            WorkNodeArgs {
                node_id: "implement".into(),
                goal: "Implement the required change".into(),
                content: String::new(),
                parents: vec!["inspect".into()],
            },
        ],
        finish: BoundaryNodeArgs {
            node_id: "finish".into(),
            goal: "Deliver the completed task".into(),
            content: String::new(),
            parents: vec!["implement".into()],
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
