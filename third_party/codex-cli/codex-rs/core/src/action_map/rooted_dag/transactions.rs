use super::events::EventBatch;
use super::events::MapEvent;
use super::events::ReplayError;
use super::events::apply_batch;
use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::MapId;
use super::model::MapNode;
use super::model::NodeId;
use super::model::NodeRole;
use super::model::NodeStatus;
use super::model::Revision;
use super::model::TaskSpaceMap;
use super::transitions::NodeTransition;
use super::transitions::readiness_changes;
use super::transitions::transition_target;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializeMap {
    pub(crate) map_id: MapId,
    pub(crate) root_node_id: NodeId,
    pub(crate) root_goal: String,
    pub(crate) source_refs: Vec<String>,
    pub(crate) finish_node_id: NodeId,
    pub(crate) finish_goal: String,
    pub(crate) work_nodes: BTreeMap<NodeId, String>,
    pub(crate) edges: Vec<MapEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphMutation {
    pub(crate) expected_revision: Revision,
    pub(crate) add_nodes: BTreeMap<NodeId, String>,
    pub(crate) add_edges: Vec<MapEdge>,
    pub(crate) remove_edges: Vec<MapEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) map: TaskSpaceMap,
    pub(crate) events: EventBatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rejection {
    pub(crate) state_commit: bool,
    pub(crate) current_revision: Revision,
    pub(crate) violations: Vec<Violation>,
}

impl Rejection {
    fn one(current_revision: Revision, code: ViolationCode, subject: impl Into<String>) -> Self {
        Self {
            state_commit: false,
            current_revision,
            violations: vec![Violation {
                code,
                subjects: vec![subject.into()],
            }],
        }
    }

    fn many(current_revision: Revision, violations: Vec<Violation>) -> Self {
        Self {
            state_commit: false,
            current_revision,
            violations,
        }
    }
}

pub(crate) fn initialize(input: InitializeMap) -> Result<Commit, Rejection> {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        input.root_node_id.clone(),
        MapNode::task_root(input.root_goal, input.source_refs),
    );
    if input.root_node_id == input.finish_node_id {
        return Err(Rejection::one(
            0,
            ViolationCode::FinishIdMismatch,
            input.finish_node_id.to_string(),
        ));
    }
    nodes.insert(
        input.finish_node_id.clone(),
        MapNode::finish(input.finish_goal),
    );
    for (id, goal) in input.work_nodes {
        if nodes.contains_key(&id) {
            return Err(Rejection::one(
                0,
                ViolationCode::TransitionInvalid,
                id.to_string(),
            ));
        }
        nodes.insert(id, MapNode::work(goal));
    }
    let mut candidate = TaskSpaceMap {
        id: input.map_id.clone(),
        root_node_id: input.root_node_id,
        finish_node_id: input.finish_node_id,
        nodes,
        edges: input.edges,
        revision: 0,
        current_binding: None,
        terminal_summary_ref: None,
    };
    candidate.canonicalize();
    let violations = validate(&candidate);
    if !violations.is_empty() {
        return Err(Rejection::many(0, violations));
    }
    let mut events = vec![MapEvent::MapInitialized {
        map: candidate.clone(),
    }];
    events.extend(readiness_events(&candidate));
    commit(None, EventBatch::new(input.map_id, 1, events))
}

pub(crate) fn mutate_graph(
    current: &TaskSpaceMap,
    mutation: GraphMutation,
) -> Result<Commit, Rejection> {
    require_revision(current, mutation.expected_revision)?;
    let mut added_nodes = BTreeMap::new();
    for (id, goal) in mutation.add_nodes {
        if current.nodes.contains_key(&id) {
            return Err(Rejection::one(
                current.revision,
                ViolationCode::TransitionInvalid,
                id.to_string(),
            ));
        }
        added_nodes.insert(id, MapNode::work(goal));
    }
    let event = MapEvent::GraphMutationCommitted {
        add_nodes: added_nodes,
        add_edges: mutation.add_edges,
        remove_edges: mutation.remove_edges,
    };
    let revision = next_revision(current)?;
    let provisional = EventBatch::new(current.id.clone(), revision, vec![event.clone()]);
    let candidate = apply_batch(Some(current), &provisional)
        .map_err(|error| rejection_from_replay(current.revision, error))?;
    let mut events = vec![event];
    events.extend(readiness_events(&candidate));
    commit(
        Some(current),
        EventBatch::new(current.id.clone(), revision, events),
    )
}

pub(crate) fn transition_node(
    current: &TaskSpaceMap,
    expected_revision: Revision,
    node_id: NodeId,
    transition: NodeTransition,
) -> Result<Commit, Rejection> {
    require_revision(current, expected_revision)?;
    let Some(node) = current.node(&node_id) else {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            node_id.to_string(),
        ));
    };
    transition_target(node.role, node.status, transition)
        .map_err(|code| Rejection::one(current.revision, code, node_id.to_string()))?;
    let event = match transition {
        NodeTransition::Bind => MapEvent::NodeBound { node_id },
        NodeTransition::Complete => MapEvent::NodeCompleted { node_id },
        NodeTransition::Block => MapEvent::NodeBlocked { node_id },
        NodeTransition::Unblock => MapEvent::NodeUnblocked { node_id },
    };
    let revision = next_revision(current)?;
    let provisional = EventBatch::new(current.id.clone(), revision, vec![event.clone()]);
    let candidate = apply_batch(Some(current), &provisional)
        .map_err(|error| rejection_from_replay(current.revision, error))?;
    let mut events = vec![event];
    if transition == NodeTransition::Complete {
        events.extend(readiness_events(&candidate));
    }
    commit(
        Some(current),
        EventBatch::new(current.id.clone(), revision, events),
    )
}

pub(crate) fn finish_end(
    current: &TaskSpaceMap,
    expected_revision: Revision,
    final_summary: String,
) -> Result<Commit, Rejection> {
    require_revision(current, expected_revision)?;
    if final_summary.trim().is_empty() {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::FinalSummaryEmpty,
            "final_summary",
        ));
    }
    let finish_ready = current
        .node(&current.finish_node_id)
        .is_some_and(|node| node.status == NodeStatus::Ready);
    if !finish_ready {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::FinishNotReady,
            current.finish_node_id.to_string(),
        ));
    }
    let unfinished: Vec<_> = current
        .nodes
        .iter()
        .filter(|(_, node)| node.role == NodeRole::Work && node.status != NodeStatus::Completed)
        .map(|(id, _)| id.to_string())
        .collect();
    if !unfinished.is_empty() {
        return Err(Rejection {
            state_commit: false,
            current_revision: current.revision,
            violations: vec![Violation {
                code: ViolationCode::UnfinishedRequiredWork,
                subjects: unfinished,
            }],
        });
    }
    let revision = next_revision(current)?;
    commit(
        Some(current),
        EventBatch::new(
            current.id.clone(),
            revision,
            vec![MapEvent::TerminalCommitted { final_summary }],
        ),
    )
}

fn readiness_events(map: &TaskSpaceMap) -> Vec<MapEvent> {
    readiness_changes(map)
        .into_iter()
        .map(|change| MapEvent::ReadinessChanged {
            node_id: change.node_id,
            from: change.from,
            to: change.to,
        })
        .collect()
}

fn commit(current: Option<&TaskSpaceMap>, events: EventBatch) -> Result<Commit, Rejection> {
    let current_revision = current.map_or(0, |map| map.revision);
    let map = apply_batch(current, &events)
        .map_err(|error| rejection_from_replay(current_revision, error))?;
    Ok(Commit { map, events })
}

fn require_revision(current: &TaskSpaceMap, expected: Revision) -> Result<(), Rejection> {
    if current.revision != expected {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::StaleRevision,
            expected.to_string(),
        ));
    }
    Ok(())
}

fn next_revision(current: &TaskSpaceMap) -> Result<Revision, Rejection> {
    current.revision.checked_add(1).ok_or_else(|| {
        Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            "revision_overflow",
        )
    })
}

fn rejection_from_replay(current_revision: Revision, error: ReplayError) -> Rejection {
    match error {
        ReplayError::InvariantViolations(violations) => {
            Rejection::many(current_revision, violations)
        }
        ReplayError::EventInvalid { code, subjects } => Rejection {
            state_commit: false,
            current_revision,
            violations: vec![Violation { code, subjects }],
        },
        other => Rejection::one(
            current_revision,
            ViolationCode::TransitionInvalid,
            format!("{other:?}"),
        ),
    }
}
