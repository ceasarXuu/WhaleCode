use super::cognitive::EvidenceRef;

pub(crate) const PROBLEM_STATE_LEDGER_VERSION: &str = "taskspace-problem-ledger-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemStateLedger {
    pub(crate) objective: String,
    pub(crate) success_criteria: Vec<ProblemSuccessCriterion>,
    pub(crate) known_facts: Vec<ProblemLedgerFact>,
    pub(crate) open_questions: Vec<ProblemOpenQuestion>,
    pub(crate) hypotheses: Vec<ProblemHypothesis>,
    pub(crate) decisions: Vec<ProblemDecision>,
    pub(crate) risks: Vec<ProblemRisk>,
    pub(crate) blockers: Vec<ProblemBlocker>,
    pub(crate) next_best_action: Option<ProblemNextBestAction>,
    pub(crate) updated_at_ms: i64,
    pub(crate) schema_incomplete: bool,
}

impl ProblemStateLedger {
    pub(crate) fn new(
        objective: impl Into<String>,
        success_criteria: Vec<ProblemSuccessCriterion>,
        updated_at_ms: i64,
    ) -> Self {
        Self {
            objective: objective.into(),
            success_criteria,
            known_facts: Vec::new(),
            open_questions: Vec::new(),
            hypotheses: Vec::new(),
            decisions: Vec::new(),
            risks: Vec::new(),
            blockers: Vec::new(),
            next_best_action: None,
            updated_at_ms,
            schema_incomplete: false,
        }
    }

    pub(crate) fn legacy_from_objective(objective: impl Into<String>, updated_at_ms: i64) -> Self {
        Self {
            objective: objective.into(),
            updated_at_ms,
            schema_incomplete: true,
            ..Self::default()
        }
    }

    pub(crate) fn upsert_success_criterion(&mut self, record: ProblemSuccessCriterion, now: i64) {
        let id = record.id.clone();
        upsert_by_id(&mut self.success_criteria, &id, record);
        self.updated_at_ms = now;
    }

    pub(crate) fn upsert_known_fact(&mut self, record: ProblemLedgerFact, now: i64) {
        let id = record.id.clone();
        upsert_by_id(&mut self.known_facts, &id, record);
        self.updated_at_ms = now;
    }

    pub(crate) fn upsert_open_question(&mut self, record: ProblemOpenQuestion, now: i64) {
        let id = record.id.clone();
        upsert_by_id(&mut self.open_questions, &id, record);
        self.updated_at_ms = now;
    }

    pub(crate) fn close_open_question(
        &mut self,
        question_id: &str,
        resolution: String,
        closed_by_result_id: Option<String>,
        evidence_refs: Vec<EvidenceRef>,
        now: i64,
    ) -> bool {
        let Some(question) = self
            .open_questions
            .iter_mut()
            .find(|question| question.id == question_id)
        else {
            return false;
        };
        question.status = "closed".to_string();
        question.resolution = Some(resolution);
        question.closed_by_result_id = closed_by_result_id;
        question.evidence_refs = evidence_refs;
        self.updated_at_ms = now;
        true
    }

    pub(crate) fn upsert_decision(&mut self, record: ProblemDecision, now: i64) {
        let id = record.id.clone();
        upsert_by_id(&mut self.decisions, &id, record);
        self.updated_at_ms = now;
    }

    pub(crate) fn set_next_best_action(&mut self, action: ProblemNextBestAction, now: i64) {
        self.next_best_action = Some(action);
        self.updated_at_ms = now;
    }
}

impl Default for ProblemStateLedger {
    fn default() -> Self {
        Self {
            objective: String::new(),
            success_criteria: Vec::new(),
            known_facts: Vec::new(),
            open_questions: Vec::new(),
            hypotheses: Vec::new(),
            decisions: Vec::new(),
            risks: Vec::new(),
            blockers: Vec::new(),
            next_best_action: None,
            updated_at_ms: 0,
            schema_incomplete: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemSuccessCriterion {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemLedgerFact {
    pub(crate) id: String,
    pub(crate) statement: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemOpenQuestion {
    pub(crate) id: String,
    pub(crate) question: String,
    pub(crate) reason: String,
    pub(crate) blocking: bool,
    pub(crate) status: String,
    pub(crate) opened_by_node_id: Option<String>,
    pub(crate) closed_by_result_id: Option<String>,
    pub(crate) resolution: Option<String>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemHypothesis {
    pub(crate) id: String,
    pub(crate) statement: String,
    pub(crate) confidence: String,
    pub(crate) status: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
    pub(crate) falsification_check: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemDecision {
    pub(crate) id: String,
    pub(crate) decision_kind: String,
    pub(crate) decision: String,
    pub(crate) rationale: String,
    pub(crate) depends_on_results: Vec<String>,
    pub(crate) depends_on_facts: Vec<String>,
    pub(crate) resolves_questions: Vec<String>,
    pub(crate) supports_criteria: Vec<String>,
    pub(crate) risks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemRisk {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) mitigation: Option<String>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemBlocker {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProblemNextBestAction {
    pub(crate) node_id: Option<String>,
    pub(crate) action_summary: String,
    pub(crate) reason: String,
    pub(crate) expected_artifact: Option<String>,
    pub(crate) blocked_by: Vec<String>,
}

fn upsert_by_id<T>(records: &mut Vec<T>, id: &str, record: T)
where
    T: HasLedgerId,
{
    if let Some(existing) = records
        .iter_mut()
        .find(|existing| existing.ledger_id() == id)
    {
        *existing = record;
    } else {
        records.push(record);
    }
}

trait HasLedgerId {
    fn ledger_id(&self) -> &str;
}

impl HasLedgerId for ProblemSuccessCriterion {
    fn ledger_id(&self) -> &str {
        &self.id
    }
}

impl HasLedgerId for ProblemLedgerFact {
    fn ledger_id(&self) -> &str {
        &self.id
    }
}

impl HasLedgerId for ProblemOpenQuestion {
    fn ledger_id(&self) -> &str {
        &self.id
    }
}

impl HasLedgerId for ProblemDecision {
    fn ledger_id(&self) -> &str {
        &self.id
    }
}
