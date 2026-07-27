use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;

pub(crate) use codex_protocol::taskspace::TaskSpaceActionReservation as ActionReservation;
pub(crate) use codex_protocol::taskspace::TaskSpaceBlockRecord as BlockRecord;
pub(crate) use codex_protocol::taskspace::TaskSpaceCompletionRecord as CompletionRecord;
pub(crate) use codex_protocol::taskspace::TaskSpaceEvidenceRef as EvidenceRef;
pub(crate) use codex_protocol::taskspace::TaskSpaceMapEdge as MapEdge;
pub(crate) use codex_protocol::taskspace::TaskSpaceMapId as MapId;
pub(crate) use codex_protocol::taskspace::TaskSpaceMapNode as MapNode;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeId as NodeId;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeState as NodeState;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeView as NodeView;
pub(crate) use codex_protocol::taskspace::TaskSpaceReservationId as ReservationId;
pub(crate) use codex_protocol::taskspace::TaskSpaceResultRef as ResultRef;
pub(crate) use codex_protocol::taskspace::TaskSpaceRevision as Revision;
pub(crate) use codex_protocol::taskspace::TaskSpaceTerminalRecord as TerminalRecord;

pub(crate) type TaskSpaceMap = TaskSpaceCanonicalMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NodeRole {
    TaskRoot,
    Work,
    Finish,
}

impl NodeRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TaskRoot => "task_root",
            Self::Work => "work",
            Self::Finish => "finish",
        }
    }
}

pub(crate) fn new_map(
    map_id: MapId,
    root: MapNode,
    work_nodes: Vec<MapNode>,
    finish: MapNode,
    edges: Vec<MapEdge>,
) -> TaskSpaceMap {
    TaskSpaceMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id,
        root,
        work_nodes,
        finish,
        edges,
        completion_records: Default::default(),
        block_records: Default::default(),
        action_reservations: Default::default(),
        result_refs: Default::default(),
        evidence_refs: Default::default(),
        terminal_record: None,
        revision: 1,
    }
}

pub(crate) fn node<'a>(map: &'a TaskSpaceMap, node_id: &str) -> Option<&'a MapNode> {
    if map.root.node_id == node_id {
        return Some(&map.root);
    }
    if map.finish.node_id == node_id {
        return Some(&map.finish);
    }
    map.work_nodes
        .iter()
        .find(|candidate| candidate.node_id == node_id)
}

pub(crate) fn node_role(map: &TaskSpaceMap, node_id: &str) -> Option<NodeRole> {
    if map.root.node_id == node_id {
        return Some(NodeRole::TaskRoot);
    }
    if map.finish.node_id == node_id {
        return Some(NodeRole::Finish);
    }
    map.work_nodes
        .iter()
        .any(|candidate| candidate.node_id == node_id)
        .then_some(NodeRole::Work)
}

pub(crate) fn node_ids(map: &TaskSpaceMap) -> BTreeSet<&str> {
    std::iter::once(map.root.node_id.as_str())
        .chain(
            map.work_nodes
                .iter()
                .map(|work_node| work_node.node_id.as_str()),
        )
        .chain(std::iter::once(map.finish.node_id.as_str()))
        .collect()
}

pub(crate) fn canonicalize(map: &mut TaskSpaceMap) {
    map.work_nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    map.edges.sort();
}

pub(crate) fn state_sha256(map: &TaskSpaceMap) -> Result<String, serde_json::Error> {
    let mut canonical = map.clone();
    canonicalize(&mut canonical);
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn is_complete(map: &TaskSpaceMap) -> bool {
    map.terminal_record.is_some()
}

pub(crate) fn started_node_ids(map: &TaskSpaceMap) -> BTreeSet<&str> {
    let mut started = BTreeSet::new();
    started.extend(map.completion_records.keys().map(String::as_str));
    started.extend(map.block_records.keys().map(String::as_str));
    started.extend(
        map.action_reservations
            .values()
            .map(|reservation| reservation.node_id.as_str()),
    );
    started.extend(
        map.result_refs
            .values()
            .map(|record| record.node_id.as_str()),
    );
    started.extend(
        map.evidence_refs
            .values()
            .map(|record| record.node_id.as_str()),
    );
    started
}

pub(crate) fn map_node(
    node_id: impl Into<String>,
    goal: impl Into<String>,
    source_refs: Vec<String>,
) -> TaskSpaceMapNode {
    TaskSpaceMapNode {
        node_id: node_id.into(),
        goal: goal.into(),
        source_refs,
    }
}
