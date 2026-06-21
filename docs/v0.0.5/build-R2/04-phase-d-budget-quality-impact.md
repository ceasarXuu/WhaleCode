# Phase D. BudgetQualityImpactV1 with validator/quality semantics

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## D.1 Goal

A budget action must not fake cost reduction by skipping required correctness work. Every budget-induced stop, downgrade, no-spawn, create-node block, validation skip, or final abort must carry quality impact evidence.

## D.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## D.3 Runtime struct

Add in `runtime.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BudgetQualityImpactV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) sample_id: Option<String>,
    pub(crate) request_id: Option<String>,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) budget_action: String,
    pub(crate) budget_state_before: String,
    pub(crate) budget_state_after: String,
    pub(crate) counter_name: String,
    pub(crate) counter_value: usize,
    pub(crate) counter_limit: usize,
    pub(crate) validator_status_before: String,
    pub(crate) validator_status_after: String,
    pub(crate) missing_evidence_count: usize,
    pub(crate) protected_item_miss_count: usize,
    pub(crate) solve_risk: String,
    pub(crate) bounded_recovery_allowed: bool,
    pub(crate) bounded_recovery_used: bool,
    pub(crate) route_escalation_allowed: bool,
    pub(crate) route_escalation_used: bool,
    pub(crate) manual_override_allowed: bool,
    pub(crate) manual_override_used: bool,
    pub(crate) final_classification: String,
    pub(crate) score_eligible: bool,
    pub(crate) reason: String,
}
```

## D.4 Validator state collector

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceValidationState {
    pub(crate) validator_status: String,
    pub(crate) missing_evidence_count: usize,
    pub(crate) protected_item_miss_count: usize,
    pub(crate) satisfied_criteria_count: usize,
    pub(crate) open_criteria_count: usize,
    pub(crate) blocking_open_question_count: usize,
    pub(crate) accepted_validator_result_count: usize,
}

impl ActionMapRuntimeState {
    fn validation_state_for_active_task(&self) -> TaskSpaceValidationState { /* inspect ledger */ }
}
```

Pseudo:

```rust
fn validation_state_for_active_task(&self) -> TaskSpaceValidationState {
    let Some(task_id) = self.active_task_id.as_ref() else { return missing("no_active_task"); };
    let Some(task) = self.tasks.get(task_id) else { return missing("active_task_missing"); };
    let open_criteria = task.problem_ledger.success_criteria.iter()
        .filter(|c| !matches!(c.status.as_str(), "satisfied" | "waived"))
        .count();
    let blocking_questions = task.problem_ledger.open_questions.iter()
        .filter(|q| q.blocking && q.status == "open")
        .count();
    let accepted_validator_results = self.active_map()
        .map(|map| count_accepted_validator_results(map))
        .unwrap_or(0);

    TaskSpaceValidationState {
        validator_status: if open_criteria == 0 && blocking_questions == 0 { "clean" } else { "incomplete" }.to_string(),
        missing_evidence_count: open_criteria + blocking_questions,
        protected_item_miss_count: 0,
        satisfied_criteria_count: ...,
        open_criteria_count: open_criteria,
        blocking_open_question_count: blocking_questions,
        accepted_validator_result_count: accepted_validator_results,
    }
}
```

## D.5 Quality impact producer

Replace ad-hoc quality tags in `record_provider_request_budget_events` with a helper:

```rust
fn record_budget_quality_impact(
    &mut self,
    budget_action: &str,
    request_id: Option<String>,
    counter_name: &str,
    counter_value: usize,
    counter_limit: usize,
    state_before: TaskSpaceBudgetState,
    state_after: TaskSpaceBudgetState,
    reason: impl Into<String>,
) -> MapRuntimeEvent
```

Pseudo:

```rust
let before = self.validation_state_for_active_task();
let final_classification = match (budget_action, before.validator_status.as_str()) {
    ("hard_stop", "clean") => "score_eligible",
    ("early_final", "clean") => "score_eligible",
    ("validation_skip", _) => "validation_skip",
    (_, "incomplete") => "blocked_by_budget",
    _ => "accepted_risk",
};
let score_eligible = final_classification == "score_eligible";

BudgetQualityImpactV1 {
    validator_status_before: before.validator_status.clone(),
    validator_status_after: before.validator_status,
    missing_evidence_count: before.missing_evidence_count,
    protected_item_miss_count: before.protected_item_miss_count,
    solve_risk: if score_eligible { "none" } else { "possible_solve_loss" }.into(),
    bounded_recovery_allowed: !score_eligible,
    bounded_recovery_used: false,
    final_classification: final_classification.into(),
    score_eligible,
    ...
}
```

## D.6 Required producers

Call `record_budget_quality_impact` from these places:

| Budget action | Producer location |
|---|---|
| provider request blocked | `record_provider_request_budget_events` when `status=blocked` |
| compact checkpoint required | provider budget transition to `compact_checkpoint_required` |
| thin downgrade | transition to `thin_downgraded` |
| node budget block | `create_node_for_main_with_kind` over budget |
| spawn budget block | spawn gate over budget |
| final response hard stop | `session/turn.rs` when `response_actionability.is_hard_stop()` |
| no-action recovery exhausted | `session/turn.rs` second non-action response |
| validation skip | any code path that completes a run without validation node evidence |

## D.7 Cost instrumentation

`New-TaskspaceBudgetArtifacts` must parse all fields, not fixed defaults. Add output fields:

```json
{
  "validator_status_before": "incomplete",
  "validator_status_after": "incomplete",
  "missing_evidence_count": 2,
  "protected_item_miss_count": 0,
  "solve_risk": "possible_solve_loss",
  "bounded_recovery_allowed": true,
  "bounded_recovery_used": false,
  "route_escalation_allowed": false,
  "route_escalation_used": false,
  "manual_override_used": false,
  "final_classification": "blocked_by_budget",
  "score_eligible": false
}
```

Summary must count distinct samples, not only events:

```json
{
  "budget_quality_impact_logged_for_every_budget_action": true,
  "budget_induced_validation_skip_count": 0,
  "budget_induced_score_ineligible_solved_count": 0,
  "blocked_by_budget_samples_count": 0,
  "manual_override_used_count": 0,
  "missing_evidence_event_count": 0
}
```

## D.8 Tests

Rust:

```rust
#[test]
fn hard_stop_with_incomplete_validation_is_not_score_eligible() {}

#[test]
fn early_final_with_clean_validation_is_score_eligible() {}

#[test]
fn validation_skip_is_release_blocker() {}

#[test]
fn node_budget_block_records_quality_impact() {}
```

PowerShell:

```text
budget quality summary fails if budget action lacks quality event
budget quality summary fails if blocked_by_budget sample is reported as solved
budget quality summary passes if hard stop occurs after clean validation and no missing evidence
```

## D.9 Acceptance

```text
budget_quality_impact_logged_for_every_budget_action = true
budget_quality_impact_missing_count = 0
budget_induced_validation_skip_count = 0
budget_induced_score_ineligible_solved_count = 0
blocked_by_budget_samples_count = 0 for release_pass
manual_override_used_count = 0 for release_pass
```
