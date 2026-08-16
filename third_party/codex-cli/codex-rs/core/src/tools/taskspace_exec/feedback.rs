use std::collections::BTreeSet;

use serde::Serialize;

use crate::action_map::rooted_dag;
use crate::action_map::rooted_dag::NodeRole;
use crate::action_map::rooted_dag::NodeState;
use crate::action_map::rooted_dag::TaskSpaceMap;

use super::MapOperation;
use super::TaskSpaceExecPlan;

#[derive(Debug, Serialize)]
pub(super) struct AffectedNodeState {
    node_id: String,
    state_before_sequence: Option<NodeState>,
    state_after_sequence: NodeState,
    unavailable_direct_work_children: Vec<UnavailableDirectWorkChild>,
}

#[derive(Debug, Serialize)]
struct UnavailableDirectWorkChild {
    node_id: String,
    state: NodeState,
    incomplete_parent_ids: Vec<String>,
    message: String,
}

pub(super) fn affected_node_states(
    before: Option<&TaskSpaceMap>,
    after: Option<&TaskSpaceMap>,
    plan: &TaskSpaceExecPlan,
) -> Vec<AffectedNodeState> {
    let Some(after) = after else {
        return Vec::new();
    };
    let mut affected = directly_operated_node_ids(plan);
    for (_, node) in rooted_dag::nodes(after) {
        if before
            .and_then(|map| rooted_dag::node(map, &node.node_id))
            .map(|old| old.state)
            != Some(node.state)
        {
            affected.insert(node.node_id.clone());
        }
    }
    let children = rooted_dag::children_by_parent(after);
    affected
        .into_iter()
        .filter_map(|node_id| {
            let node = rooted_dag::node(after, &node_id)?;
            let unavailable_direct_work_children = if node.state == NodeState::Completed {
                Vec::new()
            } else {
                children
                    .get(&node_id)
                    .into_iter()
                    .flatten()
                    .filter_map(|child_id| unavailable_work_child(after, child_id))
                    .collect()
            };
            Some(AffectedNodeState {
                state_before_sequence: before
                    .and_then(|map| rooted_dag::node(map, &node_id))
                    .map(|old| old.state),
                state_after_sequence: node.state,
                node_id,
                unavailable_direct_work_children,
            })
        })
        .collect()
}

fn directly_operated_node_ids(plan: &TaskSpaceExecPlan) -> BTreeSet<String> {
    let mut node_ids = plan
        .tools
        .iter()
        .map(|call| call.node_id.clone())
        .collect::<BTreeSet<_>>();
    for operation in plan.pre_map.iter().chain(plan.terminal_map.iter()) {
        match operation {
            MapOperation::InitializeMap(args) => {
                node_ids.insert(args.root.node_id.clone());
                node_ids.extend(args.work_nodes.iter().map(|node| node.node_id.clone()));
                node_ids.insert(args.finish.node_id.clone());
            }
            MapOperation::UpdateMap(args) => {
                node_ids.extend(args.add_work_nodes.iter().map(|node| node.node_id.clone()));
                node_ids.extend(args.node_patches.iter().map(|patch| patch.node_id.clone()));
            }
            MapOperation::ReadMap(_) | MapOperation::ReopenMap(_) | MapOperation::FinishMap(_) => {}
        }
    }
    node_ids
}

fn unavailable_work_child(
    map: &TaskSpaceMap,
    child_id: &str,
) -> Option<UnavailableDirectWorkChild> {
    if rooted_dag::node_role(map, child_id) != Some(NodeRole::Work) {
        return None;
    }
    let child = rooted_dag::node(map, child_id)?;
    if child.state != NodeState::Waiting {
        return None;
    }
    let incomplete_parent_ids = child
        .parents
        .iter()
        .filter(|parent_id| {
            *parent_id != &map.root.node_id
                && rooted_dag::node(map, parent_id)
                    .is_none_or(|parent| parent.state != NodeState::Completed)
        })
        .cloned()
        .collect::<Vec<_>>();
    let parents = serde_json::to_string(&incomplete_parent_ids)
        .expect("node identities are always JSON serializable");
    Some(UnavailableDirectWorkChild {
        node_id: child.node_id.clone(),
        state: child.state,
        incomplete_parent_ids,
        message: format!(
            "Node `{}` is not executable; incomplete direct parent nodes: {parents}.",
            child.node_id
        ),
    })
}

#[cfg(test)]
mod tests {
    use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
    use serde_json::json;

    use super::*;

    fn node(id: &str, state: NodeState, parents: &[&str]) -> rooted_dag::MapNode {
        rooted_dag::MapNode {
            node_id: id.into(),
            goal: id.into(),
            state,
            content: String::new(),
            parents: parents.iter().map(|parent| (*parent).into()).collect(),
            actions: Vec::new(),
        }
    }

    fn map(fix: NodeState, verify: NodeState) -> TaskSpaceMap {
        TaskSpaceMap {
            schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
            map_id: "map".into(),
            root: node("root", NodeState::InFlight, &[]),
            work_nodes: vec![
                node("fix", fix, &["root"]),
                node("verify", verify, &["fix"]),
            ],
            finish: node("finish", NodeState::Waiting, &["verify"]),
            revision: 1,
        }
    }

    fn empty_plan() -> TaskSpaceExecPlan {
        TaskSpaceExecPlan {
            sequence_type: "work".into(),
            pre_map: Vec::new(),
            tools: Vec::new(),
            terminal_map: None,
        }
    }

    #[test]
    fn reports_changed_owner_and_exact_unavailable_work_child() {
        let before = map(NodeState::Ready, NodeState::Waiting);
        let after = map(NodeState::InFlight, NodeState::Waiting);

        assert_eq!(
            serde_json::to_value(affected_node_states(
                Some(&before),
                Some(&after),
                &empty_plan()
            ))
            .unwrap(),
            json!([{
                "node_id": "fix",
                "state_before_sequence": "ready",
                "state_after_sequence": "in_flight",
                "unavailable_direct_work_children": [{
                    "node_id": "verify",
                    "state": "waiting",
                    "incomplete_parent_ids": ["fix"],
                    "message": "Node `verify` is not executable; incomplete direct parent nodes: [\"fix\"]."
                }]
            }])
        );
    }

    #[test]
    fn unlocked_child_has_no_stale_unavailable_feedback() {
        let before = map(NodeState::InFlight, NodeState::Waiting);
        let after = map(NodeState::Completed, NodeState::InFlight);
        let feedback = serde_json::to_value(affected_node_states(
            Some(&before),
            Some(&after),
            &empty_plan(),
        ))
        .unwrap();

        assert_eq!(feedback[0]["node_id"], "fix");
        assert_eq!(feedback[0]["unavailable_direct_work_children"], json!([]));
        assert_eq!(feedback[1]["node_id"], "verify");
        assert_eq!(feedback[1]["state_after_sequence"], "in_flight");
        assert_eq!(feedback[1]["unavailable_direct_work_children"], json!([]));
    }
}
