use super::model::NodeState;
use super::model::TaskSpaceMap;
use super::model::is_complete;
use super::model::node_ids;
use super::model::nodes;
use super::transitions::predecessors_satisfied;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use petgraph::algo::is_cyclic_directed;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::Dfs;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ViolationCode {
    SchemaVersionInvalid,
    MapIdentityInvalid,
    RevisionInvalid,
    NodeIdentityInvalid,
    WorkNodeRequired,
    DuplicateNode,
    NodeGoalEmpty,
    RootHasParent,
    NonRootNoParent,
    ParentDuplicate,
    ParentEndpointMissing,
    SelfLoop,
    CycleDetected,
    NonFinishNoChild,
    FinishHasChild,
    NodeUnreachableFromRoot,
    FinishUnreachableFromNode,
    NodeStateInvalid,
    ActionInvalid,
    ActionConflict,
    StaleRevision,
    TransitionInvalid,
    FinishNotReady,
    MapAlreadyFinished,
    MapNotFinished,
}

impl ViolationCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionInvalid => "schema_version_invalid",
            Self::MapIdentityInvalid => "map_identity_invalid",
            Self::RevisionInvalid => "revision_invalid",
            Self::NodeIdentityInvalid => "node_identity_invalid",
            Self::WorkNodeRequired => "work_node_required",
            Self::DuplicateNode => "duplicate_node",
            Self::NodeGoalEmpty => "node_goal_empty",
            Self::RootHasParent => "root_has_parent",
            Self::NonRootNoParent => "non_root_no_parent",
            Self::ParentDuplicate => "parent_duplicate",
            Self::ParentEndpointMissing => "parent_endpoint_missing",
            Self::SelfLoop => "self_loop",
            Self::CycleDetected => "cycle_detected",
            Self::NonFinishNoChild => "non_finish_no_child",
            Self::FinishHasChild => "finish_has_child",
            Self::NodeUnreachableFromRoot => "node_unreachable_from_root",
            Self::FinishUnreachableFromNode => "finish_unreachable_from_node",
            Self::NodeStateInvalid => "node_state_invalid",
            Self::ActionInvalid => "action_invalid",
            Self::ActionConflict => "action_conflict",
            Self::StaleRevision => "stale_revision",
            Self::TransitionInvalid => "transition_invalid",
            Self::FinishNotReady => "finish_not_ready",
            Self::MapAlreadyFinished => "map_already_finished",
            Self::MapNotFinished => "map_not_finished",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Violation {
    pub(crate) code: ViolationCode,
    pub(crate) subjects: Vec<String>,
}

impl Violation {
    pub(crate) fn one(code: ViolationCode, subject: impl Into<String>) -> Self {
        Self {
            code,
            subjects: vec![subject.into()],
        }
    }
}

type Violations = BTreeMap<ViolationCode, BTreeSet<String>>;

pub(crate) fn validate(map: &TaskSpaceMap) -> Vec<Violation> {
    let mut found = Violations::new();
    validate_identity(map, &mut found);
    let graph = validate_parents(map, &mut found);
    validate_shape(map, &graph, &mut found);
    validate_reachability(map, &graph, &mut found);
    validate_states(map, &mut found);
    validate_actions(map, &mut found);
    found
        .into_iter()
        .map(|(code, subjects)| Violation {
            code,
            subjects: subjects.into_iter().collect(),
        })
        .collect()
}

fn add(found: &mut Violations, code: ViolationCode, subject: impl Into<String>) {
    found.entry(code).or_default().insert(subject.into());
}

fn add_empty(found: &mut Violations, code: ViolationCode) {
    found.entry(code).or_default();
}

fn validate_identity(map: &TaskSpaceMap, found: &mut Violations) {
    if map.schema_version != TASKSPACE_CANONICAL_SCHEMA_VERSION {
        add(
            found,
            ViolationCode::SchemaVersionInvalid,
            map.schema_version.clone(),
        );
    }
    if map.map_id.trim().is_empty() {
        add_empty(found, ViolationCode::MapIdentityInvalid);
    }
    if map.revision == 0 {
        add_empty(found, ViolationCode::RevisionInvalid);
    }
    if map.work_nodes.is_empty() {
        add_empty(found, ViolationCode::WorkNodeRequired);
    }
    let mut seen = BTreeSet::new();
    for (_, node) in nodes(map) {
        if node.node_id.trim().is_empty() {
            add_empty(found, ViolationCode::NodeIdentityInvalid);
        } else if !seen.insert(node.node_id.as_str()) {
            add(found, ViolationCode::DuplicateNode, node.node_id.clone());
        }
        if node.goal.trim().is_empty() {
            add(found, ViolationCode::NodeGoalEmpty, node.node_id.clone());
        }
    }
}

fn validate_parents<'a>(map: &'a TaskSpaceMap, found: &mut Violations) -> DiGraphMap<&'a str, ()> {
    let ids = node_ids(map);
    let mut graph = DiGraphMap::new();
    for id in &ids {
        graph.add_node(*id);
    }
    for (_, node) in nodes(map) {
        if node.node_id == map.root.node_id && !node.parents.is_empty() {
            add(found, ViolationCode::RootHasParent, node.node_id.clone());
        }
        if node.node_id != map.root.node_id && node.parents.is_empty() {
            add(found, ViolationCode::NonRootNoParent, node.node_id.clone());
        }
        let mut seen = BTreeSet::new();
        for parent in &node.parents {
            let relation = format!("{parent}->{}", node.node_id);
            if !seen.insert(parent.as_str()) {
                add(found, ViolationCode::ParentDuplicate, relation.clone());
            }
            if parent == &node.node_id {
                add(found, ViolationCode::SelfLoop, relation.clone());
            }
            if !ids.contains(parent.as_str()) {
                add(found, ViolationCode::ParentEndpointMissing, relation);
                continue;
            }
            graph.add_edge(parent.as_str(), node.node_id.as_str(), ());
        }
    }
    if is_cyclic_directed(&graph) {
        add_empty(found, ViolationCode::CycleDetected);
    }
    graph
}

fn validate_shape(map: &TaskSpaceMap, graph: &DiGraphMap<&str, ()>, found: &mut Violations) {
    for id in node_ids(map) {
        let children = graph
            .neighbors_directed(id, petgraph::Direction::Outgoing)
            .count();
        if id == map.finish.node_id && children != 0 {
            add(found, ViolationCode::FinishHasChild, id);
        } else if id != map.finish.node_id && children == 0 {
            add(found, ViolationCode::NonFinishNoChild, id);
        }
    }
}

fn validate_reachability(map: &TaskSpaceMap, graph: &DiGraphMap<&str, ()>, found: &mut Violations) {
    let mut from_root = Dfs::new(graph, map.root.node_id.as_str());
    let mut root_reachable = BTreeSet::new();
    while let Some(id) = from_root.next(graph) {
        root_reachable.insert(id);
    }
    for id in node_ids(map) {
        if !root_reachable.contains(id) {
            add(found, ViolationCode::NodeUnreachableFromRoot, id);
        }
    }

    let mut reversed: DiGraphMap<&str, ()> = DiGraphMap::new();
    for (from, to, ()) in graph.all_edges() {
        reversed.add_edge(to, from, ());
    }
    let mut to_finish = Dfs::new(&reversed, map.finish.node_id.as_str());
    let mut finish_reachable = BTreeSet::new();
    while let Some(id) = to_finish.next(&reversed) {
        finish_reachable.insert(id);
    }
    for id in node_ids(map) {
        if !finish_reachable.contains(id) {
            add(found, ViolationCode::FinishUnreachableFromNode, id);
        }
    }
}

fn validate_states(map: &TaskSpaceMap, found: &mut Violations) {
    let complete = is_complete(map);
    if (map.root.state == NodeState::Completed) != (map.finish.state == NodeState::Completed) {
        add_empty(found, ViolationCode::NodeStateInvalid);
    }
    if !complete && map.root.state != NodeState::InFlight {
        add(
            found,
            ViolationCode::NodeStateInvalid,
            map.root.node_id.clone(),
        );
    }
    for (_, node) in nodes(map) {
        if node.node_id == map.root.node_id {
            continue;
        }
        let ready = predecessors_satisfied(map, &node.node_id);
        if matches!(
            node.state,
            NodeState::Ready | NodeState::InFlight | NodeState::Completed
        ) && !ready
        {
            add(found, ViolationCode::NodeStateInvalid, node.node_id.clone());
        }
        if node.state == NodeState::Waiting && ready {
            add(found, ViolationCode::NodeStateInvalid, node.node_id.clone());
        }
    }
}

fn validate_actions(map: &TaskSpaceMap, found: &mut Violations) {
    let mut identities = BTreeMap::new();
    for (role, node) in nodes(map) {
        if role != super::model::NodeRole::Work && !node.actions.is_empty() {
            add(found, ViolationCode::ActionInvalid, node.node_id.clone());
        }
        let mut local = BTreeSet::new();
        for action in &node.actions {
            if action.action_id.trim().is_empty() || action.tool_name.trim().is_empty() {
                add(found, ViolationCode::ActionInvalid, node.node_id.clone());
                continue;
            }
            if !local.insert(action.action_id.as_str()) {
                add(
                    found,
                    ViolationCode::ActionInvalid,
                    action.action_id.clone(),
                );
            }
            match identities.get(action.action_id.as_str()) {
                Some(existing) if *existing != (action.tool_name.as_str(), action.outcome) => {
                    add(
                        found,
                        ViolationCode::ActionConflict,
                        action.action_id.clone(),
                    );
                }
                None => {
                    identities.insert(
                        action.action_id.as_str(),
                        (action.tool_name.as_str(), action.outcome),
                    );
                }
                _ => {}
            }
        }
    }
}
