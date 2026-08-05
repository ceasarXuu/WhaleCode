use super::events::EventBatch;
use super::events::MapFact;
use super::events::ReplayError;
use super::events::apply_batch;
use super::invariants::Violation;
use super::invariants::ViolationCode;
use super::model::ActionRecord;
use super::model::BlockRecord;
use super::model::CompletionRecord;
use super::model::EvidenceRef;
use super::model::MapEdge;
use super::model::MapId;
use super::model::MapNode;
use super::model::ResultRef;
use super::model::Revision;
use super::model::TaskSpaceMap;
use super::model::TerminalRecord;
use super::model::node;
use super::transitions::derive_node_state;
use super::transitions::predecessors;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionInput {
    pub(crate) action: ActionRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializeMap {
    pub(crate) map_id: MapId,
    pub(crate) root: MapNode,
    pub(crate) work_nodes: Vec<MapNode>,
    pub(crate) finish: MapNode,
    pub(crate) edges: Vec<MapEdge>,
    pub(crate) actions: Vec<ActionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GraphMutation {
    pub(crate) add_work_nodes: Vec<MapNode>,
    pub(crate) add_edges: Vec<MapEdge>,
    pub(crate) remove_edges: Vec<MapEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeMutation {
    Complete {
        node_id: String,
        record: CompletionRecord,
    },
    Block {
        node_id: String,
        record: BlockRecord,
    },
    Unblock {
        node_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecuteTransaction {
    pub(crate) expected_revision: Revision,
    pub(crate) graph: GraphMutation,
    pub(crate) node_mutations: Vec<NodeMutation>,
    pub(crate) actions: Vec<ActionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResultRefInput {
    pub(crate) result_ref_id: String,
    pub(crate) is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceRefInput {
    pub(crate) evidence_ref_id: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachActionFacts {
    pub(crate) expected_revision: Revision,
    pub(crate) action_id: String,
    pub(crate) result_refs: Vec<ResultRefInput>,
    pub(crate) evidence_refs: Vec<EvidenceRefInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinishMap {
    pub(crate) expected_revision: Revision,
    pub(crate) finish_node_id: String,
    pub(crate) final_completions: Vec<FinalCompletion>,
    pub(crate) terminal: TerminalRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReopenMap {
    pub(crate) expected_revision: Revision,
    pub(crate) add_work_nodes: Vec<MapNode>,
    pub(crate) add_edges: Vec<MapEdge>,
    pub(crate) actions: Vec<ActionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinalCompletion {
    pub(crate) node_id: String,
    pub(crate) record: CompletionRecord,
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
    pub(crate) fn one(
        current_revision: Revision,
        code: ViolationCode,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            state_commit: false,
            current_revision,
            violations: vec![Violation::simple(code, vec![subject.into()])],
        }
    }

    fn from_replay(
        current: Option<&TaskSpaceMap>,
        current_revision: Revision,
        error: ReplayError,
    ) -> Self {
        let mut violations = match error {
            ReplayError::InvariantViolations(violations) => violations,
            ReplayError::InvalidFact(violation) => vec![violation],
            ReplayError::RevisionMismatch { .. } => {
                vec![Violation::simple(ViolationCode::StaleRevision, vec![])]
            }
            ReplayError::MapIdentityMismatch => {
                vec![Violation::simple(ViolationCode::MapIdentityInvalid, vec![])]
            }
            ReplayError::EmptyBatch
            | ReplayError::InitializationRequired
            | ReplayError::UnexpectedInitialization => {
                vec![Violation::simple(ViolationCode::TransitionInvalid, vec![])]
            }
        };
        for violation in &mut violations {
            let Some(node_id) = violation.node_id.as_deref() else {
                continue;
            };
            let Some(canonical) = current else {
                violation.canonical_node_present_before_transaction = Some(false);
                continue;
            };
            violation.canonical_node_present_before_transaction =
                Some(node(canonical, node_id).is_some());
            violation.canonical_state_before_transaction = derive_node_state(canonical, node_id);
            violation.canonical_unsatisfied_predecessor_ids_before_transaction =
                unsatisfied_predecessors(canonical, node_id);
        }
        Self {
            state_commit: false,
            current_revision,
            violations,
        }
    }
}

pub(crate) fn initialize(input: InitializeMap) -> Result<Commit, Rejection> {
    let mut facts = vec![MapFact::MapInitialized {
        map_id: input.map_id.clone(),
        root: input.root,
        work_nodes: input.work_nodes,
        finish: input.finish,
        edges: input.edges,
    }];
    facts.extend(
        input
            .actions
            .into_iter()
            .map(|input| MapFact::ActionRecorded {
                action: input.action,
            }),
    );
    commit(None, input.map_id, 1, facts)
}

pub(crate) fn execute(
    current: &TaskSpaceMap,
    input: ExecuteTransaction,
) -> Result<Commit, Rejection> {
    require_revision(current, input.expected_revision)?;
    let mut facts = Vec::new();
    facts.extend(
        input
            .graph
            .add_work_nodes
            .into_iter()
            .map(|node| MapFact::WorkNodeAdded { node }),
    );
    facts.extend(
        input
            .graph
            .remove_edges
            .into_iter()
            .map(|edge| MapFact::EdgeRemoved { edge }),
    );
    facts.extend(
        input
            .graph
            .add_edges
            .into_iter()
            .map(|edge| MapFact::EdgeAdded { edge }),
    );
    facts.extend(input.node_mutations.into_iter().map(node_fact));
    facts.extend(
        input
            .actions
            .into_iter()
            .map(|input| MapFact::ActionRecorded {
                action: input.action,
            }),
    );
    commit(
        Some(current),
        current.map_id.clone(),
        next_revision(current)?,
        facts,
    )
}

pub(crate) fn attach_action_facts(
    current: &TaskSpaceMap,
    input: AttachActionFacts,
) -> Result<Commit, Rejection> {
    require_revision(current, input.expected_revision)?;
    let action = current
        .action_records
        .get(&input.action_id)
        .ok_or_else(|| {
            Rejection::one(
                current.revision,
                ViolationCode::ActionRecordInvalid,
                input.action_id.clone(),
            )
        })?;
    let mut facts = Vec::new();
    facts.extend(
        input
            .result_refs
            .into_iter()
            .map(|result| MapFact::ResultAttributed {
                result_ref_id: result.result_ref_id,
                result: ResultRef {
                    node_id: action.node_id.clone(),
                    action_id: action.action_id.clone(),
                    is_error: result.is_error,
                },
            }),
    );
    facts.extend(
        input
            .evidence_refs
            .into_iter()
            .map(|evidence| MapFact::EvidenceAttributed {
                evidence_ref_id: evidence.evidence_ref_id,
                evidence: EvidenceRef {
                    node_id: action.node_id.clone(),
                    action_id: action.action_id.clone(),
                    kind: evidence.kind,
                },
            }),
    );
    commit(
        Some(current),
        current.map_id.clone(),
        next_revision(current)?,
        facts,
    )
}

pub(crate) fn finish_map(current: &TaskSpaceMap, input: FinishMap) -> Result<Commit, Rejection> {
    require_revision(current, input.expected_revision)?;
    if input.terminal.summary_ref.trim().is_empty() {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::ExactSummaryEmpty,
            input.finish_node_id,
        ));
    }
    let mut facts = input
        .final_completions
        .into_iter()
        .map(|completion| MapFact::NodeCompleted {
            node_id: completion.node_id,
            record: completion.record,
        })
        .collect::<Vec<_>>();
    facts.push(MapFact::TerminalRecorded {
        finish_node_id: input.finish_node_id,
        terminal: input.terminal,
    });
    commit(
        Some(current),
        current.map_id.clone(),
        next_revision(current)?,
        facts,
    )
}

pub(crate) fn reopen_map(current: &TaskSpaceMap, input: ReopenMap) -> Result<Commit, Rejection> {
    require_revision(current, input.expected_revision)?;
    if input.add_work_nodes.is_empty() || input.add_edges.is_empty() {
        return Err(Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            "reopen_requires_work_and_edges",
        ));
    }
    let terminal = current.terminal_record.clone().ok_or_else(|| {
        Rejection::one(
            current.revision,
            ViolationCode::TransitionInvalid,
            "active_map",
        )
    })?;
    let mut facts = vec![MapFact::MapReopened { terminal }];
    facts.extend(
        input
            .add_work_nodes
            .into_iter()
            .map(|node| MapFact::WorkNodeAdded { node }),
    );
    facts.extend(
        input
            .add_edges
            .into_iter()
            .map(|edge| MapFact::EdgeAdded { edge }),
    );
    facts.extend(
        input
            .actions
            .into_iter()
            .map(|input| MapFact::ActionRecorded {
                action: input.action,
            }),
    );
    commit(
        Some(current),
        current.map_id.clone(),
        next_revision(current)?,
        facts,
    )
}

fn node_fact(mutation: NodeMutation) -> MapFact {
    match mutation {
        NodeMutation::Complete { node_id, record } => MapFact::NodeCompleted { node_id, record },
        NodeMutation::Block { node_id, record } => MapFact::NodeBlocked { node_id, record },
        NodeMutation::Unblock { node_id } => MapFact::NodeUnblocked { node_id },
    }
}

fn commit(
    current: Option<&TaskSpaceMap>,
    map_id: MapId,
    revision: Revision,
    facts: Vec<MapFact>,
) -> Result<Commit, Rejection> {
    let events = EventBatch {
        map_id,
        revision,
        facts,
    };
    let current_revision = current.map_or(0, |map| map.revision);
    let map = apply_batch(current, &events)
        .map_err(|error| Rejection::from_replay(current, current_revision, error))?;
    Ok(Commit { map, events })
}

fn unsatisfied_predecessors(map: &TaskSpaceMap, node_id: &str) -> Vec<String> {
    predecessors(map, node_id)
        .into_iter()
        .filter(|predecessor| {
            predecessor != &map.root.node_id && !map.completion_records.contains_key(predecessor)
        })
        .collect()
}

fn require_revision(current: &TaskSpaceMap, expected_revision: Revision) -> Result<(), Rejection> {
    if current.revision == expected_revision {
        Ok(())
    } else {
        Err(Rejection::one(
            current.revision,
            ViolationCode::StaleRevision,
            expected_revision.to_string(),
        ))
    }
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
