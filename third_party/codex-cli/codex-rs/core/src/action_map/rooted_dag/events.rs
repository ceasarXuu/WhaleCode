use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::invariants::validate;
use super::model::ActionReservation;
use super::model::BlockRecord;
use super::model::CompletionRecord;
use super::model::EvidenceRef;
use super::model::MapEdge;
use super::model::MapId;
use super::model::MapNode;
use super::model::NodeRole;
use super::model::NodeState;
use super::model::ReservationId;
use super::model::ResultRef;
use super::model::Revision;
use super::model::TaskSpaceMap;
use super::model::TerminalRecord;
use super::model::canonicalize;
use super::model::new_map;
use super::model::node;
use super::model::node_role;
use super::transitions::derive_node_state;
use super::transitions::downstream_started_nodes;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "fact")]
pub(crate) enum MapFact {
    MapInitialized {
        map_id: MapId,
        root: MapNode,
        work_nodes: Vec<MapNode>,
        finish: MapNode,
        edges: Vec<MapEdge>,
    },
    WorkNodeAdded {
        node: MapNode,
    },
    EdgeAdded {
        edge: MapEdge,
    },
    EdgeRemoved {
        edge: MapEdge,
    },
    NodeCompleted {
        node_id: String,
        record: CompletionRecord,
    },
    NodeBlocked {
        node_id: String,
        record: BlockRecord,
    },
    NodeUnblocked {
        node_id: String,
    },
    NodeReworked {
        node_id: String,
    },
    ActionReserved {
        reservation_id: ReservationId,
        reservation: ActionReservation,
    },
    ResultAttributed {
        result_ref_id: String,
        result: ResultRef,
    },
    EvidenceAttributed {
        evidence_ref_id: String,
        evidence: EvidenceRef,
    },
    ActionReleased {
        reservation_id: ReservationId,
    },
    TerminalRecorded {
        finish_node_id: String,
        terminal: TerminalRecord,
        root_completion: CompletionRecord,
        finish_completion: CompletionRecord,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EventBatch {
    pub(crate) map_id: MapId,
    pub(crate) revision: Revision,
    pub(crate) facts: Vec<MapFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReplayError {
    EmptyBatch,
    MapIdentityMismatch,
    RevisionMismatch {
        expected: Revision,
        actual: Revision,
    },
    InitializationRequired,
    UnexpectedInitialization,
    InvalidFact {
        code: ViolationCode,
        subjects: Vec<String>,
    },
    InvariantViolations(Vec<Violation>),
}

impl ReplayError {
    fn invalid(code: ViolationCode, subject: impl Into<String>) -> Self {
        Self::InvalidFact {
            code,
            subjects: vec![subject.into()],
        }
    }
}

pub(crate) fn apply_batch(
    current: Option<&TaskSpaceMap>,
    batch: &EventBatch,
) -> Result<TaskSpaceMap, ReplayError> {
    if batch.facts.is_empty() {
        return Err(ReplayError::EmptyBatch);
    }
    let expected_revision = current
        .map(|map| map.revision.checked_add(1).unwrap_or(u64::MAX))
        .unwrap_or(1);
    if batch.revision != expected_revision {
        return Err(ReplayError::RevisionMismatch {
            expected: expected_revision,
            actual: batch.revision,
        });
    }
    if current.is_some_and(|map| map.map_id != batch.map_id) {
        return Err(ReplayError::MapIdentityMismatch);
    }

    let mut candidate = current.cloned();
    for fact in &batch.facts {
        apply_fact(&mut candidate, fact)?;
    }
    let mut candidate = candidate.ok_or(ReplayError::InitializationRequired)?;
    if candidate.map_id != batch.map_id {
        return Err(ReplayError::MapIdentityMismatch);
    }
    candidate.revision = batch.revision;
    canonicalize(&mut candidate);
    let violations = validate(&candidate);
    if !violations.is_empty() {
        return Err(ReplayError::InvariantViolations(violations));
    }
    Ok(candidate)
}

pub(crate) fn replay_batches(batches: &[EventBatch]) -> Result<TaskSpaceMap, ReplayError> {
    let mut current = None;
    for batch in batches {
        current = Some(apply_batch(current.as_ref(), batch)?);
    }
    current.ok_or(ReplayError::InitializationRequired)
}

fn apply_fact(candidate: &mut Option<TaskSpaceMap>, fact: &MapFact) -> Result<(), ReplayError> {
    match fact {
        MapFact::MapInitialized {
            map_id,
            root,
            work_nodes,
            finish,
            edges,
        } => {
            if candidate.is_some() {
                return Err(ReplayError::UnexpectedInitialization);
            }
            *candidate = Some(new_map(
                map_id.clone(),
                root.clone(),
                work_nodes.clone(),
                finish.clone(),
                edges.clone(),
            ));
            Ok(())
        }
        _ => apply_existing_fact(
            candidate
                .as_mut()
                .ok_or(ReplayError::InitializationRequired)?,
            fact,
        ),
    }
}

fn apply_existing_fact(map: &mut TaskSpaceMap, fact: &MapFact) -> Result<(), ReplayError> {
    if map.terminal_record.is_some() {
        return Err(ReplayError::invalid(
            ViolationCode::TransitionInvalid,
            "terminal_map",
        ));
    }
    match fact {
        MapFact::MapInitialized { .. } => Err(ReplayError::UnexpectedInitialization),
        MapFact::WorkNodeAdded { node: work_node } => {
            if node(map, &work_node.node_id).is_some() {
                return Err(ReplayError::invalid(
                    ViolationCode::DuplicateNode,
                    work_node.node_id.clone(),
                ));
            }
            map.work_nodes.push(work_node.clone());
            Ok(())
        }
        MapFact::EdgeAdded { edge } => {
            require_unstarted_target(map, edge)?;
            map.edges.push(edge.clone());
            Ok(())
        }
        MapFact::EdgeRemoved { edge } => {
            require_unstarted_target(map, edge)?;
            let index = map
                .edges
                .iter()
                .position(|candidate| candidate == edge)
                .ok_or_else(|| {
                    ReplayError::invalid(
                        ViolationCode::TransitionInvalid,
                        format!("{}->{}", edge.from, edge.to),
                    )
                })?;
            map.edges.remove(index);
            Ok(())
        }
        MapFact::NodeCompleted { node_id, record } => {
            require_work_state(map, node_id, NodeState::Ready)?;
            if map
                .completion_records
                .insert(node_id.clone(), record.clone())
                .is_some()
            {
                return Err(ReplayError::invalid(
                    ViolationCode::TransitionInvalid,
                    node_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::NodeBlocked { node_id, record } => {
            require_work_state(map, node_id, NodeState::Ready)?;
            if map
                .block_records
                .insert(node_id.clone(), record.clone())
                .is_some()
            {
                return Err(ReplayError::invalid(
                    ViolationCode::TransitionInvalid,
                    node_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::NodeUnblocked { node_id } => {
            if map.block_records.remove(node_id).is_none() {
                return Err(ReplayError::invalid(
                    ViolationCode::TransitionInvalid,
                    node_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::NodeReworked { node_id } => {
            if node_role(map, node_id) != Some(NodeRole::Work)
                || map.completion_records.remove(node_id).is_none()
            {
                return Err(ReplayError::invalid(
                    ViolationCode::TransitionInvalid,
                    node_id.clone(),
                ));
            }
            let conflicts = downstream_started_nodes(map, node_id);
            if !conflicts.is_empty() {
                return Err(ReplayError::InvalidFact {
                    code: ViolationCode::ExecutionCausalityConflict,
                    subjects: conflicts,
                });
            }
            Ok(())
        }
        MapFact::ActionReserved {
            reservation_id,
            reservation,
        } => reserve_action(map, reservation_id, reservation),
        MapFact::ResultAttributed {
            result_ref_id,
            result,
        } => {
            require_matching_reservation(
                map,
                &result.reservation_id,
                &result.action_id,
                &result.node_id,
            )?;
            if map
                .result_refs
                .insert(result_ref_id.clone(), result.clone())
                .is_some()
            {
                return Err(ReplayError::invalid(
                    ViolationCode::FactReferenceInvalid,
                    result_ref_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::EvidenceAttributed {
            evidence_ref_id,
            evidence,
        } => {
            require_matching_reservation(
                map,
                &evidence.reservation_id,
                &evidence.action_id,
                &evidence.node_id,
            )?;
            if map
                .evidence_refs
                .insert(evidence_ref_id.clone(), evidence.clone())
                .is_some()
            {
                return Err(ReplayError::invalid(
                    ViolationCode::FactReferenceInvalid,
                    evidence_ref_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::ActionReleased { reservation_id } => {
            if map.action_reservations.remove(reservation_id).is_none() {
                return Err(ReplayError::invalid(
                    ViolationCode::ReservationInvalid,
                    reservation_id.clone(),
                ));
            }
            Ok(())
        }
        MapFact::TerminalRecorded {
            finish_node_id,
            terminal,
            root_completion,
            finish_completion,
        } => finish(
            map,
            finish_node_id,
            terminal,
            root_completion,
            finish_completion,
        ),
    }
}

fn require_unstarted_target(map: &TaskSpaceMap, edge: &MapEdge) -> Result<(), ReplayError> {
    if super::model::started_node_ids(map).contains(edge.to.as_str()) {
        return Err(ReplayError::invalid(
            ViolationCode::ExecutionCausalityConflict,
            format!("{}->{}", edge.from, edge.to),
        ));
    }
    Ok(())
}

fn require_work_state(
    map: &TaskSpaceMap,
    node_id: &str,
    expected: NodeState,
) -> Result<(), ReplayError> {
    if node_role(map, node_id) != Some(NodeRole::Work)
        || derive_node_state(map, node_id) != Some(expected)
    {
        return Err(ReplayError::invalid(
            ViolationCode::TransitionInvalid,
            node_id,
        ));
    }
    Ok(())
}

fn reserve_action(
    map: &mut TaskSpaceMap,
    reservation_id: &str,
    reservation: &ActionReservation,
) -> Result<(), ReplayError> {
    let state = derive_node_state(map, &reservation.node_id);
    if node_role(map, &reservation.node_id) != Some(NodeRole::Work)
        || !matches!(state, Some(NodeState::Ready | NodeState::InFlight))
        || map.action_reservations.contains_key(reservation_id)
        || map
            .action_reservations
            .values()
            .any(|current| current.action_id == reservation.action_id)
    {
        return Err(ReplayError::invalid(
            ViolationCode::ReservationInvalid,
            reservation_id,
        ));
    }
    map.action_reservations
        .insert(reservation_id.to_string(), reservation.clone());
    Ok(())
}

fn require_matching_reservation(
    map: &TaskSpaceMap,
    reservation_id: &str,
    action_id: &str,
    node_id: &str,
) -> Result<(), ReplayError> {
    let matches = map
        .action_reservations
        .get(reservation_id)
        .is_some_and(|reservation| {
            reservation.action_id == action_id && reservation.node_id == node_id
        });
    if matches {
        Ok(())
    } else {
        Err(ReplayError::invalid(
            ViolationCode::ReservationInvalid,
            reservation_id,
        ))
    }
}

fn finish(
    map: &mut TaskSpaceMap,
    finish_node_id: &str,
    terminal: &TerminalRecord,
    root_completion: &CompletionRecord,
    finish_completion: &CompletionRecord,
) -> Result<(), ReplayError> {
    if finish_node_id != map.finish.node_id
        || derive_node_state(map, finish_node_id) != Some(NodeState::Ready)
        || !map.action_reservations.is_empty()
        || terminal.summary_ref.trim().is_empty()
        || root_completion.action_id != terminal.action_id
        || finish_completion.action_id != terminal.action_id
    {
        return Err(ReplayError::invalid(
            ViolationCode::FinishNotReady,
            finish_node_id,
        ));
    }
    map.completion_records
        .insert(map.root.node_id.clone(), root_completion.clone());
    map.completion_records
        .insert(map.finish.node_id.clone(), finish_completion.clone());
    map.terminal_record = Some(terminal.clone());
    Ok(())
}
