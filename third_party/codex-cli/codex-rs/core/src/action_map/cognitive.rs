pub(crate) const COGNITIVE_SCHEMA_VERSION: &str = "taskspace-cognitive-v1";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TaskCognitiveState {
    pub(crate) success_criteria: Vec<String>,
    pub(crate) fact_sources: Vec<FactSource>,
    pub(crate) output_contracts: Vec<OutputContract>,
    pub(crate) facts: Vec<CognitiveClaim>,
    pub(crate) assumptions: Vec<CognitiveClaim>,
    pub(crate) risk_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceRef {
    pub(crate) result_id: Option<String>,
    pub(crate) claim_id: Option<String>,
    pub(crate) fact_source_id: Option<String>,
    pub(crate) trace_event_id: Option<String>,
    pub(crate) artifact_ref: Option<String>,
    pub(crate) validator_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FactSource {
    pub(crate) id: String,
    pub(crate) provenance: DataProvenance,
    pub(crate) description: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataProvenance {
    ObservedFromEnvironment,
    ProvidedByUser,
    GeneratedForTestOnly,
    Inferred,
    Unknown,
}

impl DataProvenance {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ObservedFromEnvironment => "observed_from_environment",
            Self::ProvidedByUser => "provided_by_user",
            Self::GeneratedForTestOnly => "generated_for_test_only",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "observed_from_environment" => Some(Self::ObservedFromEnvironment),
            "provided_by_user" => Some(Self::ProvidedByUser),
            "generated_for_test_only" => Some(Self::GeneratedForTestOnly),
            "inferred" => Some(Self::Inferred),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputContract {
    pub(crate) id: String,
    pub(crate) kind: OutputContractKind,
    pub(crate) description: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputContractKind {
    Artifact,
    Format,
    Encoding,
    Schema,
    Validator,
    NonGoal,
}

impl OutputContractKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Format => "format",
            Self::Encoding => "encoding",
            Self::Schema => "schema",
            Self::Validator => "validator",
            Self::NonGoal => "non_goal",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "artifact" => Some(Self::Artifact),
            "format" => Some(Self::Format),
            "encoding" => Some(Self::Encoding),
            "schema" => Some(Self::Schema),
            "validator" => Some(Self::Validator),
            "non_goal" => Some(Self::NonGoal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CognitiveClaim {
    pub(crate) id: String,
    pub(crate) statement: String,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeResultEvidencePackage {
    pub(crate) claims: Vec<CognitiveClaim>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
    pub(crate) changed_artifacts: Vec<String>,
    pub(crate) validator_refs: Vec<String>,
    pub(crate) remaining_uncertainty: Vec<String>,
    pub(crate) validity: ResultValidity,
    pub(crate) validity_reason: String,
    pub(crate) adoption: NodeResultAdoption,
}

impl Default for NodeResultEvidencePackage {
    fn default() -> Self {
        Self {
            claims: Vec::new(),
            evidence_refs: Vec::new(),
            changed_artifacts: Vec::new(),
            validator_refs: Vec::new(),
            remaining_uncertainty: Vec::new(),
            validity: ResultValidity::Unreviewed,
            validity_reason: String::new(),
            adoption: NodeResultAdoption::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NodeResultAdoption {
    pub(crate) adoption_state: ResultAdoptionState,
    pub(crate) adopted_by_facts: Vec<String>,
    pub(crate) adopted_by_hypotheses: Vec<String>,
    pub(crate) adopted_by_decisions: Vec<String>,
    pub(crate) adopted_by_criteria: Vec<String>,
    pub(crate) adopted_by_nodes: Vec<String>,
}

impl NodeResultAdoption {
    pub(crate) fn refresh_state(&mut self, validity: ResultValidity) {
        self.adoption_state = match validity {
            ResultValidity::Unreviewed => ResultAdoptionState::None,
            ResultValidity::Accepted if self.has_refs() => ResultAdoptionState::AcceptedAdopted,
            ResultValidity::Accepted => ResultAdoptionState::AcceptedUnused,
            ResultValidity::Questioned => ResultAdoptionState::Questioned,
            ResultValidity::Invalid => ResultAdoptionState::Invalid,
        };
    }

    pub(crate) fn merge_refs(
        &mut self,
        facts: Vec<String>,
        hypotheses: Vec<String>,
        decisions: Vec<String>,
        criteria: Vec<String>,
        nodes: Vec<String>,
    ) {
        merge_unique(&mut self.adopted_by_facts, facts);
        merge_unique(&mut self.adopted_by_hypotheses, hypotheses);
        merge_unique(&mut self.adopted_by_decisions, decisions);
        merge_unique(&mut self.adopted_by_criteria, criteria);
        merge_unique(&mut self.adopted_by_nodes, nodes);
    }

    fn has_refs(&self) -> bool {
        !self.adopted_by_facts.is_empty()
            || !self.adopted_by_hypotheses.is_empty()
            || !self.adopted_by_decisions.is_empty()
            || !self.adopted_by_criteria.is_empty()
            || !self.adopted_by_nodes.is_empty()
    }
}

fn merge_unique(target: &mut Vec<String>, refs: Vec<String>) {
    for value in refs {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultAdoptionState {
    None,
    AcceptedUnused,
    AcceptedAdopted,
    Questioned,
    Invalid,
}

impl Default for ResultAdoptionState {
    fn default() -> Self {
        Self::None
    }
}

impl ResultAdoptionState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AcceptedUnused => "accepted_unused",
            Self::AcceptedAdopted => "accepted_adopted",
            Self::Questioned => "questioned",
            Self::Invalid => "invalid",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "accepted_unused" => Some(Self::AcceptedUnused),
            "accepted_adopted" => Some(Self::AcceptedAdopted),
            "questioned" => Some(Self::Questioned),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultValidity {
    Unreviewed,
    Accepted,
    Questioned,
    Invalid,
}

impl ResultValidity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Accepted => "accepted",
            Self::Questioned => "questioned",
            Self::Invalid => "invalid",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "unreviewed" => Some(Self::Unreviewed),
            "accepted" => Some(Self::Accepted),
            "questioned" => Some(Self::Questioned),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}
