use super::model::NodeRole;
use super::model::TaskSpaceMap;
use super::model::node_ids;
use super::model::node_role;
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
pub(crate) enum ViolationCode {
    SchemaVersionInvalid,
    MapIdentityInvalid,
    RevisionInvalid,
    NodeIdentityInvalid,
    NodeGoalEmpty,
    DuplicateNode,
    EdgeEndpointMissing,
    DuplicateEdge,
    SelfLoop,
    CycleDetected,
    NonRootZeroIndegree,
    NonFinishZeroOutdegree,
    NodeUnreachableFromRoot,
    FinishUnreachableFromNode,
    RecordNodeInvalid,
    FactConflict,
    FactReferenceInvalid,
    ReservationInvalid,
    TransitionInvalid,
    ExecutionCausalityConflict,
    StaleRevision,
    FinishNotReady,
    UnfinishedRequiredWork,
    FinalSummaryEmpty,
    TerminalRecordInvalid,
}

impl ViolationCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionInvalid => "schema_version_invalid",
            Self::MapIdentityInvalid => "map_identity_invalid",
            Self::RevisionInvalid => "revision_invalid",
            Self::NodeIdentityInvalid => "node_identity_invalid",
            Self::NodeGoalEmpty => "node_goal_empty",
            Self::DuplicateNode => "duplicate_node",
            Self::EdgeEndpointMissing => "edge_endpoint_missing",
            Self::DuplicateEdge => "duplicate_edge",
            Self::SelfLoop => "self_loop",
            Self::CycleDetected => "cycle_detected",
            Self::NonRootZeroIndegree => "non_root_zero_indegree",
            Self::NonFinishZeroOutdegree => "non_finish_zero_outdegree",
            Self::NodeUnreachableFromRoot => "node_unreachable_from_root",
            Self::FinishUnreachableFromNode => "finish_unreachable_from_node",
            Self::RecordNodeInvalid => "record_node_invalid",
            Self::FactConflict => "fact_conflict",
            Self::FactReferenceInvalid => "fact_reference_invalid",
            Self::ReservationInvalid => "reservation_invalid",
            Self::TransitionInvalid => "transition_invalid",
            Self::ExecutionCausalityConflict => "execution_causality_conflict",
            Self::StaleRevision => "stale_revision",
            Self::FinishNotReady => "finish_not_ready",
            Self::UnfinishedRequiredWork => "unfinished_required_work",
            Self::FinalSummaryEmpty => "final_summary_empty",
            Self::TerminalRecordInvalid => "terminal_record_invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Violation {
    pub(crate) code: ViolationCode,
    pub(crate) subjects: Vec<String>,
}

type Violations = BTreeMap<ViolationCode, BTreeSet<String>>;

pub(crate) fn validate(map: &TaskSpaceMap) -> Vec<Violation> {
    let mut found = Violations::new();
    validate_identity(map, &mut found);
    let graph = validate_edges(map, &mut found);
    validate_degrees(map, &graph, &mut found);
    validate_reachability(map, &graph, &mut found);
    validate_facts(map, &mut found);
    validate_terminal(map, &mut found);
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
    if map.schema_version != codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION {
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
    let mut seen = BTreeSet::new();
    for node in std::iter::once(&map.root)
        .chain(map.work_nodes.iter())
        .chain(std::iter::once(&map.finish))
    {
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

fn validate_edges<'a>(map: &'a TaskSpaceMap, found: &mut Violations) -> DiGraphMap<&'a str, ()> {
    let ids = node_ids(map);
    let mut graph = DiGraphMap::new();
    for id in &ids {
        graph.add_node(*id);
    }
    let mut seen = BTreeSet::new();
    for edge in &map.edges {
        let subject = format!("{}->{}", edge.from, edge.to);
        if !seen.insert((edge.from.as_str(), edge.to.as_str())) {
            add(found, ViolationCode::DuplicateEdge, subject.clone());
        }
        if edge.from == edge.to {
            add(found, ViolationCode::SelfLoop, subject.clone());
        }
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            add(found, ViolationCode::EdgeEndpointMissing, subject);
            continue;
        }
        graph.add_edge(edge.from.as_str(), edge.to.as_str(), ());
    }
    if is_cyclic_directed(&graph) {
        add_empty(found, ViolationCode::CycleDetected);
    }
    graph
}

fn validate_degrees(map: &TaskSpaceMap, graph: &DiGraphMap<&str, ()>, found: &mut Violations) {
    for id in node_ids(map) {
        let indegree = graph.neighbors_directed(id, Direction::Incoming).count();
        let outdegree = graph.neighbors_directed(id, Direction::Outgoing).count();
        if id != map.root.node_id && indegree == 0 {
            add(found, ViolationCode::NonRootZeroIndegree, id);
        }
        if id != map.finish.node_id && outdegree == 0 {
            add(found, ViolationCode::NonFinishZeroOutdegree, id);
        }
    }
}

fn validate_reachability(map: &TaskSpaceMap, graph: &DiGraphMap<&str, ()>, found: &mut Violations) {
    if graph.contains_node(map.root.node_id.as_str()) {
        let mut dfs = Dfs::new(graph, map.root.node_id.as_str());
        let mut reachable = BTreeSet::new();
        while let Some(id) = dfs.next(graph) {
            reachable.insert(id);
        }
        for id in node_ids(map)
            .into_iter()
            .filter(|id| !reachable.contains(id))
        {
            add(found, ViolationCode::NodeUnreachableFromRoot, id);
        }
    }
    if graph.contains_node(map.finish.node_id.as_str()) {
        let mut reversed: DiGraphMap<&str, ()> = DiGraphMap::new();
        for (from, to, ()) in graph.all_edges() {
            reversed.add_edge(to, from, ());
        }
        let mut dfs = Dfs::new(&reversed, map.finish.node_id.as_str());
        let mut reachable = BTreeSet::new();
        while let Some(id) = dfs.next(&reversed) {
            reachable.insert(id);
        }
        for id in node_ids(map)
            .into_iter()
            .filter(|id| !reachable.contains(id))
        {
            add(found, ViolationCode::FinishUnreachableFromNode, id);
        }
    }
}

fn validate_facts(map: &TaskSpaceMap, found: &mut Violations) {
    let ids = node_ids(map);
    for node_id in map
        .completion_records
        .keys()
        .chain(map.block_records.keys())
    {
        if !ids.contains(node_id.as_str()) {
            add(found, ViolationCode::RecordNodeInvalid, node_id.clone());
        }
    }
    for node_id in map.completion_records.keys() {
        if map.block_records.contains_key(node_id)
            || map
                .action_reservations
                .values()
                .any(|reservation| reservation.node_id == *node_id)
        {
            add(found, ViolationCode::FactConflict, node_id.clone());
        }
    }
    for node_id in map.block_records.keys() {
        if node_role(map, node_id) != Some(NodeRole::Work) {
            add(found, ViolationCode::RecordNodeInvalid, node_id.clone());
        }
    }
    validate_reservations(map, found);
    validate_references(map, found);
}

fn validate_reservations(map: &TaskSpaceMap, found: &mut Violations) {
    let mut action_ids = BTreeSet::new();
    for (reservation_id, reservation) in &map.action_reservations {
        if reservation_id.trim().is_empty()
            || reservation.action_id.trim().is_empty()
            || reservation.tool_name.trim().is_empty()
            || node_role(map, &reservation.node_id) != Some(NodeRole::Work)
            || map.completion_records.contains_key(&reservation.node_id)
            || map.block_records.contains_key(&reservation.node_id)
        {
            add(
                found,
                ViolationCode::ReservationInvalid,
                reservation_id.clone(),
            );
        }
        if !action_ids.insert(reservation.action_id.as_str()) {
            add(
                found,
                ViolationCode::ReservationInvalid,
                reservation.action_id.clone(),
            );
        }
    }
}

fn validate_references(map: &TaskSpaceMap, found: &mut Violations) {
    for (ref_id, result) in &map.result_refs {
        if ref_id.trim().is_empty()
            || node_role(map, &result.node_id) != Some(NodeRole::Work)
            || result.action_id.trim().is_empty()
            || result.reservation_id.trim().is_empty()
        {
            add(found, ViolationCode::FactReferenceInvalid, ref_id.clone());
        }
    }
    for (ref_id, evidence) in &map.evidence_refs {
        if ref_id.trim().is_empty()
            || node_role(map, &evidence.node_id) != Some(NodeRole::Work)
            || evidence.action_id.trim().is_empty()
            || evidence.reservation_id.trim().is_empty()
            || evidence.kind.trim().is_empty()
        {
            add(found, ViolationCode::FactReferenceInvalid, ref_id.clone());
        }
    }
    for (node_id, completion) in &map.completion_records {
        for ref_id in &completion.result_ref_ids {
            if !map
                .result_refs
                .get(ref_id)
                .is_some_and(|record| record.node_id == *node_id)
            {
                add(found, ViolationCode::FactReferenceInvalid, ref_id.clone());
            }
        }
        for ref_id in &completion.evidence_ref_ids {
            if !map
                .evidence_refs
                .get(ref_id)
                .is_some_and(|record| record.node_id == *node_id)
            {
                add(found, ViolationCode::FactReferenceInvalid, ref_id.clone());
            }
        }
    }
}

fn validate_terminal(map: &TaskSpaceMap, found: &mut Violations) {
    let root_complete = map.completion_records.contains_key(&map.root.node_id);
    let finish_complete = map.completion_records.contains_key(&map.finish.node_id);
    match &map.terminal_record {
        None if root_complete || finish_complete => {
            add_empty(found, ViolationCode::TerminalRecordInvalid);
        }
        Some(terminal) => {
            if terminal.action_id.trim().is_empty()
                || terminal.summary_ref.trim().is_empty()
                || !root_complete
                || !finish_complete
                || !map.action_reservations.is_empty()
                || !map.block_records.is_empty()
            {
                add_empty(found, ViolationCode::TerminalRecordInvalid);
            }
            for work_node in &map.work_nodes {
                if !map.completion_records.contains_key(&work_node.node_id) {
                    add(
                        found,
                        ViolationCode::UnfinishedRequiredWork,
                        work_node.node_id.clone(),
                    );
                }
            }
        }
        None => {}
    }
}
