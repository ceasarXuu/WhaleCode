use super::ActionOutcome;
use super::Commit;
use super::NodeAction;
use super::NodeRole;
use super::Rejection;
use super::TaskSpaceMap;
use super::ViolationCode;
use super::canonicalize;
use super::model::node_mut;
use super::node_role;
use super::validate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionBinding {
    pub(crate) action_id: String,
    pub(crate) tool_name: String,
    pub(crate) outcome: ActionOutcome,
    pub(crate) node_ids: Vec<String>,
}

pub(crate) fn attach_actions(
    current: &TaskSpaceMap,
    bindings: &[ActionBinding],
) -> Result<Commit, Rejection> {
    let mut candidate = current.clone();
    let mut changed = false;
    for binding in bindings {
        if binding.action_id.trim().is_empty()
            || binding.tool_name.trim().is_empty()
            || binding.node_ids.is_empty()
        {
            return Err(Rejection::one(
                current.revision,
                ViolationCode::ActionInvalid,
                binding.action_id.clone(),
            ));
        }
        for node_id in &binding.node_ids {
            if node_role(&candidate, node_id) != Some(NodeRole::Work) {
                return Err(Rejection::one(
                    current.revision,
                    ViolationCode::ActionInvalid,
                    node_id.clone(),
                ));
            }
            let node = node_mut(&mut candidate, node_id).expect("validated work node exists");
            match node
                .actions
                .iter()
                .find(|action| action.action_id == binding.action_id)
            {
                Some(action)
                    if action.tool_name == binding.tool_name
                        && action.outcome == binding.outcome => {}
                Some(_) => {
                    return Err(Rejection::one(
                        current.revision,
                        ViolationCode::ActionConflict,
                        binding.action_id.clone(),
                    ));
                }
                None => {
                    node.actions.push(NodeAction {
                        action_id: binding.action_id.clone(),
                        tool_name: binding.tool_name.clone(),
                        outcome: binding.outcome,
                    });
                    changed = true;
                }
            }
        }
    }
    commit_if_changed(current, candidate, changed)
}

pub(crate) fn settle_action(
    current: &TaskSpaceMap,
    action_id: &str,
    tool_name: &str,
    outcome: ActionOutcome,
) -> Result<Commit, Rejection> {
    if outcome == ActionOutcome::Pending {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::ActionInvalid,
            action_id,
        ));
    }
    let mut candidate = current.clone();
    let mut found = false;
    let mut changed = false;
    for node in &mut candidate.work_nodes {
        for action in &mut node.actions {
            if action.action_id != action_id {
                continue;
            }
            found = true;
            if action.tool_name != tool_name
                || (action.outcome != ActionOutcome::Pending && action.outcome != outcome)
            {
                return Err(Rejection::one(
                    current.revision,
                    ViolationCode::ActionConflict,
                    action_id,
                ));
            }
            if action.outcome != outcome {
                action.outcome = outcome;
                changed = true;
            }
        }
    }
    if !found {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::ActionInvalid,
            action_id,
        ));
    }
    commit_if_changed(current, candidate, changed)
}

fn commit_if_changed(
    current: &TaskSpaceMap,
    mut candidate: TaskSpaceMap,
    changed: bool,
) -> Result<Commit, Rejection> {
    if !changed {
        return Ok(Commit { map: candidate });
    }
    candidate.revision = current.revision.checked_add(1).ok_or_else(|| {
        Rejection::one(
            current.revision,
            ViolationCode::RevisionInvalid,
            current.map_id.clone(),
        )
    })?;
    canonicalize(&mut candidate);
    let violations = validate(&candidate);
    if violations.is_empty() {
        Ok(Commit { map: candidate })
    } else {
        Err(Rejection {
            state_commit: false,
            current_revision: current.revision,
            violations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::rooted_dag::NodeState;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;

    fn map() -> TaskSpaceMap {
        new_map(
            "map".into(),
            map_node("root", "root", NodeState::InFlight, "", vec![]),
            vec![
                map_node("left", "left", NodeState::Ready, "", vec!["root".into()]),
                map_node("right", "right", NodeState::Ready, "", vec!["root".into()]),
            ],
            map_node(
                "finish",
                "finish",
                NodeState::Waiting,
                "",
                vec!["left".into(), "right".into()],
            ),
        )
    }

    #[test]
    fn shared_action_settles_on_every_owner_without_changing_node_state() {
        let original = map();
        let pending = attach_actions(
            &original,
            &[ActionBinding {
                action_id: "hosted-1".into(),
                tool_name: "web_search".into(),
                outcome: ActionOutcome::Pending,
                node_ids: vec!["left".into(), "right".into()],
            }],
        )
        .unwrap()
        .map;
        let settled = settle_action(&pending, "hosted-1", "web_search", ActionOutcome::Succeeded)
            .unwrap()
            .map;

        assert_eq!(settled.work_nodes[0].state, NodeState::Ready);
        assert_eq!(settled.work_nodes[1].state, NodeState::Ready);
        assert_eq!(settled.work_nodes[0].actions, settled.work_nodes[1].actions);
        assert_eq!(
            settled.work_nodes[0].actions[0].outcome,
            ActionOutcome::Succeeded
        );
    }

    #[test]
    fn terminal_action_cannot_be_reinterpreted() {
        let pending = attach_actions(
            &map(),
            &[ActionBinding {
                action_id: "call-1".into(),
                tool_name: "read_file".into(),
                outcome: ActionOutcome::Pending,
                node_ids: vec!["left".into()],
            }],
        )
        .unwrap()
        .map;
        let succeeded = settle_action(&pending, "call-1", "read_file", ActionOutcome::Succeeded)
            .unwrap()
            .map;
        let rejected =
            settle_action(&succeeded, "call-1", "read_file", ActionOutcome::Failed).unwrap_err();
        assert_eq!(rejected.violations[0].code, ViolationCode::ActionConflict);
    }
}
