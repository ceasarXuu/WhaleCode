use super::ActionOutcome;
use super::Commit;
use super::MapNode;
use super::NodeAction;
use super::NodeState;
use super::Rejection;
use super::TaskSpaceMap;
use super::ViolationCode;
use super::canonicalize;
use super::node;
use super::validate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderToolAction {
    pub(crate) action_id: String,
    pub(crate) tool_name: String,
    pub(crate) outcome: ActionOutcome,
}

pub(crate) fn record_provider_tool_actions(
    current: &TaskSpaceMap,
    actions: &[ProviderToolAction],
) -> Result<Commit, Rejection> {
    let mut candidate = current.clone();
    let mut changed = false;

    for action in actions {
        validate_action(current, action)?;
        let node_id = action.tool_name.as_str();
        match node(&candidate, node_id) {
            None => {
                candidate
                    .work_nodes
                    .push(provider_node(&candidate.root.node_id, action));
                candidate.finish.parents.push(node_id.to_string());
                changed = true;
            }
            Some(existing) if is_provider_node(existing, &candidate.root.node_id, node_id) => {
                let existing_action = existing
                    .actions
                    .iter()
                    .find(|candidate| candidate.action_id == action.action_id);
                match existing_action {
                    Some(candidate)
                        if candidate.tool_name == action.tool_name
                            && candidate.outcome == action.outcome => {}
                    Some(_) => {
                        return Err(Rejection::one(
                            current.revision,
                            ViolationCode::ActionConflict,
                            action.action_id.clone(),
                        ));
                    }
                    None => {
                        candidate
                            .work_nodes
                            .iter_mut()
                            .find(|candidate| candidate.node_id == node_id)
                            .expect("provider node existed before mutable lookup")
                            .actions
                            .push(node_action(action));
                        changed = true;
                    }
                }
            }
            Some(_) => {
                return Err(Rejection::one(
                    current.revision,
                    ViolationCode::ActionConflict,
                    node_id,
                ));
            }
        }
    }

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

fn validate_action(current: &TaskSpaceMap, action: &ProviderToolAction) -> Result<(), Rejection> {
    if action.action_id.trim().is_empty()
        || action.tool_name.trim().is_empty()
        || action.outcome == ActionOutcome::Pending
    {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::ActionInvalid,
            action.action_id.clone(),
        ));
    }
    Ok(())
}

fn provider_node(root_id: &str, action: &ProviderToolAction) -> MapNode {
    MapNode {
        node_id: action.tool_name.clone(),
        goal: action.tool_name.clone(),
        state: NodeState::Completed,
        content: String::new(),
        parents: vec![root_id.to_string()],
        actions: vec![node_action(action)],
    }
}

fn is_provider_node(node: &MapNode, root_id: &str, tool_name: &str) -> bool {
    node.node_id == tool_name
        && node.goal == tool_name
        && node.state == NodeState::Completed
        && node.content.is_empty()
        && node.parents == [root_id]
}

fn node_action(action: &ProviderToolAction) -> NodeAction {
    NodeAction {
        action_id: action.action_id.clone(),
        tool_name: action.tool_name.clone(),
        outcome: action.outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::rooted_dag::map_node;
    use crate::action_map::rooted_dag::new_map;

    fn map() -> TaskSpaceMap {
        new_map(
            "map".into(),
            map_node("root", "root", NodeState::InFlight, "", vec![]),
            vec![map_node(
                "work",
                "work",
                NodeState::Ready,
                "",
                vec!["root".into()],
            )],
            map_node(
                "finish",
                "finish",
                NodeState::Waiting,
                "",
                vec!["work".into()],
            ),
        )
    }

    fn action(id: &str, tool: &str, outcome: ActionOutcome) -> ProviderToolAction {
        ProviderToolAction {
            action_id: id.into(),
            tool_name: tool.into(),
            outcome,
        }
    }

    #[test]
    fn creates_completed_root_child_without_blocking_finish_path() {
        let recorded = record_provider_tool_actions(
            &map(),
            &[action(
                "response-1/web_search",
                "web_search",
                ActionOutcome::Succeeded,
            )],
        )
        .unwrap()
        .map;

        let node = recorded
            .work_nodes
            .iter()
            .find(|node| node.node_id == "web_search")
            .unwrap();
        assert_eq!(node.parents, ["root"]);
        assert_eq!(node.state, NodeState::Completed);
        assert!(recorded.finish.parents.contains(&"web_search".into()));
        assert!(validate(&recorded).is_empty());
    }

    #[test]
    fn appends_later_calls_to_the_same_native_tool_node() {
        let first = record_provider_tool_actions(
            &map(),
            &[action(
                "response-1/web_search",
                "web_search",
                ActionOutcome::Succeeded,
            )],
        )
        .unwrap()
        .map;
        let second = record_provider_tool_actions(
            &first,
            &[action(
                "response-2/web_search",
                "web_search",
                ActionOutcome::Failed,
            )],
        )
        .unwrap()
        .map;

        let node = second
            .work_nodes
            .iter()
            .find(|node| node.node_id == "web_search")
            .unwrap();
        assert_eq!(node.actions.len(), 2);
        assert_eq!(
            second
                .finish
                .parents
                .iter()
                .filter(|id| *id == "web_search")
                .count(),
            1
        );
    }

    #[test]
    fn preserves_an_agent_finished_map_when_recording_provider_work() {
        let mut current = map();
        current.work_nodes[0].state = NodeState::Completed;
        current.finish.state = NodeState::Completed;
        current.finish.content = "done".into();
        current.root.state = NodeState::Completed;
        canonicalize(&mut current);
        assert!(validate(&current).is_empty());

        let recorded = record_provider_tool_actions(
            &current,
            &[action(
                "response-1/image_generation",
                "image_generation",
                ActionOutcome::Succeeded,
            )],
        )
        .unwrap()
        .map;

        assert_eq!(recorded.root.state, NodeState::Completed);
        assert_eq!(recorded.finish.state, NodeState::Completed);
        assert_eq!(recorded.finish.content, "done");
        assert!(recorded.finish.parents.contains(&"image_generation".into()));
        assert!(validate(&recorded).is_empty());
    }

    #[test]
    fn refuses_to_reinterpret_an_agent_node_with_the_native_tool_name() {
        let mut current = map();
        current.work_nodes.push(map_node(
            "web_search",
            "research",
            NodeState::Ready,
            "",
            vec!["root".into()],
        ));
        current.finish.parents.push("web_search".into());
        canonicalize(&mut current);

        let error = record_provider_tool_actions(
            &current,
            &[action(
                "response-1/web_search",
                "web_search",
                ActionOutcome::Succeeded,
            )],
        )
        .unwrap_err();

        assert_eq!(error.violations[0].code, ViolationCode::ActionConflict);
    }
}
