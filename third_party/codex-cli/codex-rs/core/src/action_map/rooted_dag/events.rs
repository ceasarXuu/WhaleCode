use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::MapEdge;
use super::model::MapId;
use super::model::MapNode;
use super::model::NodeId;
use super::model::NodeStatus;
use super::model::Revision;
use super::model::TaskSpaceMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum MapEvent {
    MapInitialized {
        map: TaskSpaceMap,
    },
    GraphMutationCommitted {
        add_nodes: BTreeMap<NodeId, MapNode>,
        add_edges: Vec<MapEdge>,
        remove_edges: Vec<MapEdge>,
    },
    NodeBound {
        node_id: NodeId,
    },
    NodeBlocked {
        node_id: NodeId,
    },
    NodeCompleted {
        node_id: NodeId,
    },
    NodeUnblocked {
        node_id: NodeId,
    },
    ReadinessChanged {
        node_id: NodeId,
        from: NodeStatus,
        to: NodeStatus,
    },
    TerminalCommitted {
        final_summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MapEventRecord {
    pub(super) event_id: String,
    pub(super) map_id: MapId,
    pub(super) revision: Revision,
    pub(super) sequence: u32,
    pub(super) event: MapEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct EventBatch {
    pub(super) map_id: MapId,
    pub(super) revision: Revision,
    pub(super) records: Vec<MapEventRecord>,
}

impl EventBatch {
    pub(super) fn new(map_id: MapId, revision: Revision, events: Vec<MapEvent>) -> Self {
        let records = events
            .into_iter()
            .enumerate()
            .map(|(sequence, event)| MapEventRecord {
                event_id: event_id(&map_id, revision, sequence),
                map_id: map_id.clone(),
                revision,
                sequence: u32::try_from(sequence).expect("event batch sequence fits u32"),
                event,
            })
            .collect();
        Self {
            map_id,
            revision,
            records,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplayError {
    InitializationRequired,
    DuplicateInitialization,
    EmptyBatch,
    MapIdMismatch,
    RevisionOutOfOrder {
        expected: Revision,
        actual: Revision,
    },
    SequenceOutOfOrder {
        expected: u32,
        actual: u32,
    },
    EventIdMismatch,
    EventInvalid {
        code: ViolationCode,
        subjects: Vec<String>,
    },
    InvariantViolations(Vec<Violation>),
}

pub(super) fn apply_batch(
    current: Option<&TaskSpaceMap>,
    batch: &EventBatch,
) -> Result<TaskSpaceMap, ReplayError> {
    if batch.records.is_empty() {
        return Err(ReplayError::EmptyBatch);
    }
    let expected_revision = match current {
        Some(map) => map
            .revision
            .checked_add(1)
            .ok_or_else(|| ReplayError::EventInvalid {
                code: ViolationCode::TransitionInvalid,
                subjects: vec!["revision_overflow".to_string()],
            })?,
        None => 1,
    };
    if batch.revision != expected_revision {
        return Err(ReplayError::RevisionOutOfOrder {
            expected: expected_revision,
            actual: batch.revision,
        });
    }
    if current.is_some_and(|map| map.id != batch.map_id) {
        return Err(ReplayError::MapIdMismatch);
    }
    let mut candidate = current.cloned();
    for (index, record) in batch.records.iter().enumerate() {
        let expected_sequence = u32::try_from(index).expect("event batch sequence fits u32");
        if record.sequence != expected_sequence {
            return Err(ReplayError::SequenceOutOfOrder {
                expected: expected_sequence,
                actual: record.sequence,
            });
        }
        if record.map_id != batch.map_id || record.revision != batch.revision {
            return Err(ReplayError::MapIdMismatch);
        }
        if record.event_id != event_id(&batch.map_id, batch.revision, index) {
            return Err(ReplayError::EventIdMismatch);
        }
        apply_record(&mut candidate, record)?;
    }
    let mut candidate = candidate.ok_or(ReplayError::InitializationRequired)?;
    candidate.revision = batch.revision;
    candidate.canonicalize();
    let violations = validate(&candidate);
    if !violations.is_empty() {
        return Err(ReplayError::InvariantViolations(violations));
    }
    Ok(candidate)
}

fn event_id(map_id: &MapId, revision: Revision, sequence: usize) -> String {
    format!(
        "event:{}:{}:{revision}:{sequence}",
        map_id.as_str().len(),
        map_id
    )
}

pub(super) fn replay_batches(batches: &[EventBatch]) -> Result<TaskSpaceMap, ReplayError> {
    let mut current = None;
    for batch in batches {
        current = Some(apply_batch(current.as_ref(), batch)?);
    }
    current.ok_or(ReplayError::InitializationRequired)
}

fn apply_record(
    candidate: &mut Option<TaskSpaceMap>,
    record: &MapEventRecord,
) -> Result<(), ReplayError> {
    match &record.event {
        MapEvent::MapInitialized { map } => {
            if candidate.is_some() {
                return Err(ReplayError::DuplicateInitialization);
            }
            if map.id != record.map_id || map.revision != 0 {
                return Err(ReplayError::MapIdMismatch);
            }
            *candidate = Some(map.clone());
        }
        event => apply_existing_event(
            candidate
                .as_mut()
                .ok_or(ReplayError::InitializationRequired)?,
            event,
            &record.event_id,
        )?,
    }
    Ok(())
}

fn apply_existing_event(
    map: &mut TaskSpaceMap,
    event: &MapEvent,
    event_id: &str,
) -> Result<(), ReplayError> {
    match event {
        MapEvent::MapInitialized { .. } => return Err(ReplayError::DuplicateInitialization),
        MapEvent::GraphMutationCommitted {
            add_nodes,
            add_edges,
            remove_edges,
        } => apply_graph_mutation(map, add_nodes, add_edges, remove_edges)?,
        MapEvent::NodeBound { node_id } => {
            if map.current_binding.is_some() {
                return invalid(ViolationCode::TransitionInvalid, node_id);
            }
            set_status(map, node_id, NodeStatus::Ready, NodeStatus::Running)?;
            map.current_binding = Some(node_id.clone());
        }
        MapEvent::NodeBlocked { node_id } => {
            require_binding(map, node_id)?;
            set_status(map, node_id, NodeStatus::Running, NodeStatus::Blocked)?;
            map.current_binding = None;
        }
        MapEvent::NodeCompleted { node_id } => {
            require_binding(map, node_id)?;
            set_status(map, node_id, NodeStatus::Running, NodeStatus::Completed)?;
            map.current_binding = None;
        }
        MapEvent::NodeUnblocked { node_id } => {
            set_status(map, node_id, NodeStatus::Blocked, NodeStatus::Ready)?;
        }
        MapEvent::ReadinessChanged { node_id, from, to } => {
            set_status(map, node_id, *from, *to)?;
        }
        MapEvent::TerminalCommitted { final_summary } => {
            if final_summary.trim().is_empty() {
                return invalid_text(ViolationCode::FinalSummaryEmpty, "final_summary");
            }
            set_status(
                map,
                &map.finish_node_id.clone(),
                NodeStatus::Ready,
                NodeStatus::Closed,
            )?;
            set_status(
                map,
                &map.root_node_id.clone(),
                NodeStatus::Open,
                NodeStatus::Closed,
            )?;
            map.terminal_summary_ref = Some(event_id.to_string());
        }
    }
    Ok(())
}

fn apply_graph_mutation(
    map: &mut TaskSpaceMap,
    add_nodes: &BTreeMap<NodeId, MapNode>,
    add_edges: &[MapEdge],
    remove_edges: &[MapEdge],
) -> Result<(), ReplayError> {
    for edge in remove_edges {
        let Some(index) = map.edges.iter().position(|current| current == edge) else {
            return invalid_text(
                ViolationCode::TransitionInvalid,
                format!("{}->{}", edge.from, edge.to),
            );
        };
        map.edges.remove(index);
    }
    for (id, node) in add_nodes {
        if map.nodes.insert(id.clone(), node.clone()).is_some() {
            return invalid(ViolationCode::TransitionInvalid, id);
        }
    }
    map.edges.extend(add_edges.iter().cloned());
    Ok(())
}

fn require_binding(map: &TaskSpaceMap, node_id: &NodeId) -> Result<(), ReplayError> {
    if map.current_binding.as_ref() != Some(node_id) {
        return invalid(ViolationCode::TransitionInvalid, node_id);
    }
    Ok(())
}

fn set_status(
    map: &mut TaskSpaceMap,
    node_id: &NodeId,
    expected: NodeStatus,
    target: NodeStatus,
) -> Result<(), ReplayError> {
    let Some(node) = map.nodes.get_mut(node_id) else {
        return invalid(ViolationCode::TransitionInvalid, node_id);
    };
    if node.status != expected {
        return invalid(ViolationCode::TransitionInvalid, node_id);
    }
    node.status = target;
    Ok(())
}

fn invalid<T>(code: ViolationCode, subject: &NodeId) -> Result<T, ReplayError> {
    invalid_text(code, subject.to_string())
}

fn invalid_text<T>(code: ViolationCode, subject: impl Into<String>) -> Result<T, ReplayError> {
    Err(ReplayError::EventInvalid {
        code,
        subjects: vec![subject.into()],
    })
}
