# taskspace_control Schema Source

```rust
StartTask {
        task_title: String,
        #[serde(default)]
        task_objective: String,
        #[serde(default)]
        node_kind: String,
        node_title: String,
        node_context_summary: String,
        #[serde(default)]
        bind_current: bool,
    },
    RouteTask {
        task_id: String,
    },
    CreateNode {
        kind: String,
        title: String,
        context_summary: String,
        #[serde(default)]
        dependency_node_ids: Vec<String>,
        #[serde(default)]
        bind_current: bool,
    },
    BindNode {
        node_id: String,
    },
    FinishNode {
        node_id: String,
        result_summary: String,
        #[serde(default)]
        next_node_id: Option<String>,
        #[serde(default)]
        next_node_kind: Option<String>,
        #[serde(default)]
        next_node_title: Option<String>,
        #[serde(default)]
        next_node_context_summary: Option<String>,
        #[serde(default)]
        next_dependency_node_ids: Vec<String>,
    },
    BlockNode {
        node_id: String,
        blocker_summary: String,
    },
    RecordOutputContract {
        output_contract_id: String,
        kind: String,
        description: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    RecordFactSource {
        fact_source_id: String,
        provenance: String,
        description: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    RecordFact {
        claim_id: String,
        statement: String,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
    },
    MarkResultValidity {
        result_id: String,
        validity: String,
        validity_reason: String,
        #[serde(default)]
        claims: Vec<TaskSpaceCognitiveClaimArgs>,
        #[serde(default)]
        evidence_refs: Vec<TaskSpaceEvidenceRefArgs>,
        #[serde(default)]
        changed_artifacts: Vec<String>,
        #[serde(default)]
        validator_refs: Vec<String>,
        #[serde(default)]
        remaining_uncertainty: Vec<String>,
    },
```
