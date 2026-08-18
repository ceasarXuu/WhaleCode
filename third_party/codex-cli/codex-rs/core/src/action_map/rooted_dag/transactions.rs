use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapId;
use super::model::MapNode;
use super::model::NodeAction;
use super::model::NodeState;
use super::model::Revision;
use super::model::TaskSpaceMap;
use super::model::canonicalize;
use super::model::is_complete;
use super::model::new_map;
use super::model::node;
use super::model::node_mut;
use super::transitions::normalize_readiness;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializeMap {
    pub(crate) map_id: MapId,
    pub(crate) root: MapNode,
    pub(crate) work_nodes: Vec<MapNode>,
    pub(crate) finish: MapNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NodePatch {
    pub(crate) node_id: String,
    pub(crate) goal: Option<String>,
    pub(crate) state: Option<NodeState>,
    pub(crate) content: Option<String>,
    pub(crate) parents: Option<Vec<String>>,
    pub(crate) append_actions: Vec<NodeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecuteTransaction {
    pub(crate) request_revision: Revision,
    pub(crate) add_work_nodes: Vec<MapNode>,
    pub(crate) patches: Vec<NodePatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinishMap {
    pub(crate) request_revision: Revision,
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReopenMap {
    pub(crate) request_revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) map: TaskSpaceMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rejection {
    pub(crate) state_commit: bool,
    pub(crate) current_revision: Revision,
    pub(crate) violations: Vec<Violation>,
}

impl Rejection {
    pub(crate) fn one(
        current_revision: Revision,
        code: ViolationCode,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            state_commit: false,
            current_revision,
            violations: vec![Violation::one(code, subject)],
        }
    }
}

pub(crate) fn initialize(input: InitializeMap) -> Result<Commit, Rejection> {
    let mut map = new_map(input.map_id, input.root, input.work_nodes, input.finish);
    map.root.state = NodeState::InFlight;
    for node in &mut map.work_nodes {
        node.state = NodeState::Waiting;
    }
    map.finish.state = NodeState::Waiting;
    normalize_readiness(&mut map);
    validate_candidate(map)
}

pub(crate) fn execute(
    current: &TaskSpaceMap,
    mut input: ExecuteTransaction,
) -> Result<Commit, Rejection> {
    require_revision(current, input.request_revision)?;
    if is_complete(current) {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::MapAlreadyFinished,
            current.map_id.clone(),
        ));
    }
    let added_node_ids = input
        .add_work_nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(patch) = input
        .patches
        .iter()
        .find(|patch| patch.state.is_some() && added_node_ids.contains(patch.node_id.as_str()))
    {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            patch.node_id.clone(),
        ));
    }
    let mut candidate = current.clone();
    for node in &mut input.add_work_nodes {
        node.state = NodeState::Waiting;
    }
    candidate.work_nodes.extend(input.add_work_nodes);
    for patch in input.patches {
        apply_patch(&mut candidate, patch)?;
        normalize_readiness(&mut candidate);
    }
    normalize_readiness(&mut candidate);
    candidate.revision = next_revision(current)?;
    canonicalize(&mut candidate);
    validate_candidate(candidate)
}

pub(crate) fn finish_map(current: &TaskSpaceMap, input: FinishMap) -> Result<Commit, Rejection> {
    require_revision(current, input.request_revision)?;
    if is_complete(current) {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::MapAlreadyFinished,
            current.map_id.clone(),
        ));
    }
    if input.content.trim().is_empty() {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            current.finish.node_id.clone(),
        ));
    }
    let mut candidate = current.clone();
    normalize_readiness(&mut candidate);
    if candidate.finish.state != NodeState::Ready {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::FinishNotReady,
            candidate.finish.node_id.clone(),
        ));
    }
    candidate.root.state = NodeState::Completed;
    candidate.finish.state = NodeState::Completed;
    candidate.finish.content = input.content;
    candidate.revision = next_revision(current)?;
    validate_candidate(candidate)
}

pub(crate) fn reopen_map(current: &TaskSpaceMap, input: ReopenMap) -> Result<Commit, Rejection> {
    require_revision(current, input.request_revision)?;
    if !is_complete(current) {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::MapNotFinished,
            current.map_id.clone(),
        ));
    }
    let mut candidate = current.clone();
    candidate.root.state = NodeState::InFlight;
    candidate.finish.state = NodeState::Waiting;
    candidate.revision = next_revision(current)?;
    normalize_readiness(&mut candidate);
    validate_candidate(candidate)
}

fn apply_patch(map: &mut TaskSpaceMap, patch: NodePatch) -> Result<(), Rejection> {
    let current = node(map, &patch.node_id).cloned().ok_or_else(|| {
        Rejection::one(
            map.revision,
            ViolationCode::NodeIdentityInvalid,
            patch.node_id.clone(),
        )
    })?;
    let target_state = patch.state.unwrap_or(current.state);
    if !allowed_transition(current.state, target_state) {
        return Err(Rejection::one(
            map.revision,
            ViolationCode::TransitionInvalid,
            patch.node_id,
        ));
    }
    if current.node_id == map.root.node_id || current.node_id == map.finish.node_id {
        if patch.state.is_some() {
            return Err(Rejection::one(
                map.revision,
                ViolationCode::TransitionInvalid,
                current.node_id,
            ));
        }
    }
    let target = node_mut(map, &current.node_id).expect("node existed before mutable lookup");
    if let Some(goal) = patch.goal {
        target.goal = goal;
    }
    target.state = target_state;
    if let Some(content) = patch.content {
        target.content = content;
    }
    if let Some(parents) = patch.parents {
        target.parents = parents;
    }
    target.actions.extend(patch.append_actions);
    Ok(())
}

fn allowed_transition(from: NodeState, to: NodeState) -> bool {
    from == to
        || matches!(
            (from, to),
            (NodeState::Ready, NodeState::InFlight | NodeState::Completed)
                | (NodeState::InFlight, NodeState::Completed)
        )
}

fn require_revision(current: &TaskSpaceMap, request_revision: Revision) -> Result<(), Rejection> {
    if current.revision != request_revision {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::StaleRevision,
            request_revision.to_string(),
        ));
    }
    Ok(())
}

fn next_revision(current: &TaskSpaceMap) -> Result<Revision, Rejection> {
    current.revision.checked_add(1).ok_or_else(|| {
        Rejection::one(
            current.revision,
            ViolationCode::RevisionInvalid,
            current.revision.to_string(),
        )
    })
}

fn validate_candidate(map: TaskSpaceMap) -> Result<Commit, Rejection> {
    let violations = validate(&map);
    if violations.is_empty() {
        Ok(Commit { map })
    } else {
        Err(Rejection {
            state_commit: false,
            current_revision: map.revision,
            violations,
        })
    }
}
