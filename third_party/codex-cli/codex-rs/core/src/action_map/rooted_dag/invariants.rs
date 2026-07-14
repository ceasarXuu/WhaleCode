use super::model::NodeId;
use super::model::NodeRole;
use super::model::NodeStatus;
use super::model::TaskSpaceMap;
use petgraph::Direction;
use petgraph::algo::is_cyclic_directed;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::Dfs;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ViolationCode {
    RootMissing,
    MultipleRoots,
    FinishMissing,
    MultipleFinishes,
    RootIdMismatch,
    FinishIdMismatch,
    EdgeEndpointMissing,
    DuplicateEdge,
    SelfLoop,
    CycleDetected,
    NonRootZeroIndegree,
    NonFinishZeroOutdegree,
    NodeUnreachableFromRoot,
    FinishUnreachableFromNode,
    RoleStatusInvalid,
    TransitionInvalid,
    StaleRevision,
    FinishNotReady,
    UnfinishedRequiredWork,
    FinalSummaryEmpty,
    LegacySchemaUnsupported,
}

impl ViolationCode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::RootMissing => "root_missing",
            Self::MultipleRoots => "multiple_roots",
            Self::FinishMissing => "finish_missing",
            Self::MultipleFinishes => "multiple_finishes",
            Self::RootIdMismatch => "root_id_mismatch",
            Self::FinishIdMismatch => "finish_id_mismatch",
            Self::EdgeEndpointMissing => "edge_endpoint_missing",
            Self::DuplicateEdge => "duplicate_edge",
            Self::SelfLoop => "self_loop",
            Self::CycleDetected => "cycle_detected",
            Self::NonRootZeroIndegree => "non_root_zero_indegree",
            Self::NonFinishZeroOutdegree => "non_finish_zero_outdegree",
            Self::NodeUnreachableFromRoot => "node_unreachable_from_root",
            Self::FinishUnreachableFromNode => "finish_unreachable_from_node",
            Self::RoleStatusInvalid => "role_status_invalid",
            Self::TransitionInvalid => "transition_invalid",
            Self::StaleRevision => "stale_revision",
            Self::FinishNotReady => "finish_not_ready",
            Self::UnfinishedRequiredWork => "unfinished_required_work",
            Self::FinalSummaryEmpty => "final_summary_empty",
            Self::LegacySchemaUnsupported => "legacy_schema_unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Violation {
    pub(super) code: ViolationCode,
    pub(super) subjects: Vec<String>,
}

type Violations = BTreeMap<ViolationCode, BTreeSet<String>>;

pub(super) fn validate(map: &TaskSpaceMap) -> Vec<Violation> {
    let mut found = Violations::new();
    validate_roles(map, &mut found);
    let graph = validate_edges(map, &mut found);
    validate_degrees(map, &graph, &mut found);
    validate_reachability(map, &graph, &mut found);
    validate_terminal_coherence(map, &mut found);
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

fn validate_roles(map: &TaskSpaceMap, found: &mut Violations) {
    let roots: Vec<_> = map
        .nodes
        .iter()
        .filter(|(_, node)| node.role == NodeRole::TaskRoot)
        .collect();
    let finishes: Vec<_> = map
        .nodes
        .iter()
        .filter(|(_, node)| node.role == NodeRole::Finish)
        .collect();
    match roots.as_slice() {
        [] => add_empty(found, ViolationCode::RootMissing),
        [(id, _)] if *id != &map.root_node_id => {
            add(found, ViolationCode::RootIdMismatch, id.to_string());
        }
        [..] if roots.len() > 1 => {
            for (id, _) in roots {
                add(found, ViolationCode::MultipleRoots, id.to_string());
            }
        }
        _ => {}
    }
    match finishes.as_slice() {
        [] => add_empty(found, ViolationCode::FinishMissing),
        [(id, _)] if *id != &map.finish_node_id => {
            add(found, ViolationCode::FinishIdMismatch, id.to_string());
        }
        [..] if finishes.len() > 1 => {
            for (id, _) in finishes {
                add(found, ViolationCode::MultipleFinishes, id.to_string());
            }
        }
        _ => {}
    }
    for (id, node) in &map.nodes {
        if !node.status_allowed() {
            add(found, ViolationCode::RoleStatusInvalid, id.to_string());
        }
    }
}

fn validate_edges<'a>(map: &'a TaskSpaceMap, found: &mut Violations) -> DiGraphMap<&'a NodeId, ()> {
    let mut graph = DiGraphMap::new();
    for id in map.nodes.keys() {
        graph.add_node(id);
    }
    let mut seen = BTreeSet::new();
    for edge in &map.edges {
        let subject = format!("{}->{}", edge.from, edge.to);
        if !seen.insert((&edge.from, &edge.to)) {
            add(found, ViolationCode::DuplicateEdge, subject);
            continue;
        }
        if edge.from == edge.to {
            add(found, ViolationCode::SelfLoop, subject.clone());
        }
        if !map.nodes.contains_key(&edge.from) || !map.nodes.contains_key(&edge.to) {
            add(found, ViolationCode::EdgeEndpointMissing, subject);
            continue;
        }
        graph.add_edge(&edge.from, &edge.to, ());
    }
    if is_cyclic_directed(&graph) {
        add_empty(found, ViolationCode::CycleDetected);
    }
    graph
}

fn validate_degrees(map: &TaskSpaceMap, graph: &DiGraphMap<&NodeId, ()>, found: &mut Violations) {
    for id in map.nodes.keys() {
        let indegree = graph.neighbors_directed(id, Direction::Incoming).count();
        let outdegree = graph.neighbors_directed(id, Direction::Outgoing).count();
        if id != &map.root_node_id && indegree == 0 {
            add(found, ViolationCode::NonRootZeroIndegree, id.to_string());
        }
        if id != &map.finish_node_id && outdegree == 0 {
            add(found, ViolationCode::NonFinishZeroOutdegree, id.to_string());
        }
    }
}

fn validate_reachability(
    map: &TaskSpaceMap,
    graph: &DiGraphMap<&NodeId, ()>,
    found: &mut Violations,
) {
    if graph.contains_node(&map.root_node_id) {
        let mut dfs = Dfs::new(graph, &map.root_node_id);
        let mut reachable = BTreeSet::new();
        while let Some(id) = dfs.next(graph) {
            reachable.insert(id);
        }
        for id in map.nodes.keys().filter(|id| !reachable.contains(id)) {
            add(
                found,
                ViolationCode::NodeUnreachableFromRoot,
                id.to_string(),
            );
        }
    }
    if graph.contains_node(&map.finish_node_id) {
        let mut reversed = DiGraphMap::<&NodeId, ()>::new();
        for (from, to, _) in graph.all_edges() {
            reversed.add_edge(to, from, ());
        }
        let mut dfs = Dfs::new(&reversed, &map.finish_node_id);
        let mut can_reach_finish = BTreeSet::new();
        while let Some(id) = dfs.next(&reversed) {
            can_reach_finish.insert(id);
        }
        for id in map.nodes.keys().filter(|id| !can_reach_finish.contains(id)) {
            add(
                found,
                ViolationCode::FinishUnreachableFromNode,
                id.to_string(),
            );
        }
    }
}

fn validate_terminal_coherence(map: &TaskSpaceMap, found: &mut Violations) {
    let root_closed = map
        .node(&map.root_node_id)
        .is_some_and(|node| node.status == NodeStatus::Closed);
    let finish_closed = map
        .node(&map.finish_node_id)
        .is_some_and(|node| node.status == NodeStatus::Closed);
    if root_closed != finish_closed
        || (root_closed && map.terminal_summary_ref.is_none())
        || (!root_closed && map.terminal_summary_ref.is_some())
    {
        add_empty(found, ViolationCode::TransitionInvalid);
    }
}
