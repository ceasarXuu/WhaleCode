# Phase B. Request phase attribution and context propagation

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.
>
> 2026-06-26 review: Phase B attribution is implemented and codex-core full gate is green. The rerun45 `taskspace_control` failure was later fixed in the action-contract ABI layer; a post-ABI B-tier rerun is still required for business-success evidence.


## B.1 Goal

Every provider request must have a meaningful phase. `model_sampling` for everything except final synthesis is not sufficient.

## B.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## B.3 New enum

Add in `runtime.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceProviderRequestPhase {
    Startup,
    ProjectionUpdate,
    StateCommit,
    LegacyStateAction,
    OrdinaryToolRecovery,
    SubagentSpawn,
    SubagentResultProcessing,
    ValidationRecovery,
    FinalSynthesis,
    BudgetRecovery,
    ModelSampling,
    Unknown,
}

impl TaskSpaceProviderRequestPhase {
    pub(crate) fn as_str(self) -> &'static str { /* strict mapping */ }
}
```

## B.4 Runtime context state

Add to `ActionMapRuntimeState`:

```rust
pending_provider_request_phase: Option<TaskSpaceProviderRequestPhase>,
pending_provider_request_context_reason: Option<String>,
last_provider_request_context: Option<TaskSpaceProviderRequestContextV1>,
```

Define:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceProviderRequestContextV1 {
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) route_mode: Option<String>,
    pub(crate) request_phase: TaskSpaceProviderRequestPhase,
    pub(crate) context_selection_reason: String,
    pub(crate) missing_reason: Option<String>,
}
```

## B.5 Phase selection rules

Implement:

```rust
pub(crate) fn next_provider_request_phase(
    &self,
    map_id: &str,
    node_id: Option<&str>,
) -> TaskSpaceProviderRequestPhase {
    if self.budget_state == TaskSpaceBudgetState::CompactCheckpointRequired {
        return TaskSpaceProviderRequestPhase::BudgetRecovery;
    }
    if let Some(phase) = self.pending_provider_request_phase {
        return phase;
    }
    let Some(node) = node_id.and_then(|id| self.maps.get(map_id).and_then(|m| m.nodes.get(id))) else {
        return TaskSpaceProviderRequestPhase::Unknown;
    };
    match node.kind {
        NodeKind::FinalSynthesis => TaskSpaceProviderRequestPhase::FinalSynthesis,
        NodeKind::SmokeTest | NodeKind::RegressionTest => TaskSpaceProviderRequestPhase::ValidationRecovery,
        _ => TaskSpaceProviderRequestPhase::ModelSampling,
    }
}
```

Compatibility note after Phase A: `budget_recovery` is allowed only as a
request-phase attribution or legacy diagnostic marker. It must not be used to
block dispatch, hide tools, force final response, or make release fail merely
because a profile hint was exceeded.

## B.6 Phase producers

Set `pending_provider_request_phase` at the actual semantic transition points:

| Producer | Location | Phase to set |
|---|---|---|
| Active projection replacement generated | `build_developer_context` path before prompt composition | `ProjectionUpdate` |
| `taskspace_control(state_commit)` returns | `taskspace_control.rs` handler after successful state commit | `StateCommit` |
| legacy state action rejected | `taskspace_control.rs` handler before error | `LegacyStateAction` |
| `record_subagent_plan` succeeds | `runtime.rs` / handler | `SubagentSpawn` |
| subagent result recorded | `runtime.rs` child result path | `SubagentResultProcessing` |
| validation node blocked or failed | `mark_result_validity_for_main` / completion validation error path | `ValidationRecovery` |
| budget gate emits compact/final recovery | budget gate path | `BudgetRecovery` |

Add helper:

```rust
pub(crate) fn set_next_provider_request_phase(
    &mut self,
    phase: TaskSpaceProviderRequestPhase,
    reason: impl Into<String>,
) -> Vec<MapRuntimeEvent>;
```

Each call must emit a trace event:

```text
kind = provider_request_context_selected
tags:
  schema:taskspace-provider-request-context-v1
  producer:runtime
  request_phase:<phase>
  context_selection_reason:<reason>
```

## B.7 Session wiring

In `session/turn.rs`, replace the current `ProviderRequestAttribution` construction:

```rust
ProviderRequestAttribution {
    request_scope_id: Some(turn_context.sub_id.to_string()),
    task_id: snapshot.task_id.as_ref().map(|id| id.to_string()),
    map_id: Some(snapshot.map_id.to_string()),
    node_id: snapshot.node_id.as_ref().map(|id| id.to_string()),
    request_phase: snapshot.request_phase.clone(),
}
```

with:

```rust
ProviderRequestAttribution::from_snapshot(&snapshot, &turn_context.sub_id)
```

Define `from_snapshot` in `client.rs` or a shared module so phase/missing reason are copied consistently.

## B.8 Cost instrumentation change

`New-TaskspaceProviderRequestArtifacts` must output per-phase counts, not only coverage:

```json
{
  "schema_version": "taskspace-request-phase-summary-v1",
  "provider_request_hook_coverage": 100,
  "provider_request_terminal_coverage": 100,
  "request_phase_attribution_coverage": 97,
  "unknown_request_phase_ratio": 3,
  "phase_counts": {
    "model_sampling": 5,
    "state_commit": 2,
    "validation_recovery": 1,
    "final_synthesis": 1
  },
  "phase_token_summary": {
    "state_commit": { "input_tokens": 1234, "output_tokens": 321 }
  }
}
```

## B.9 Tests

Rust tests:

```rust
#[test]
fn state_commit_sets_next_provider_request_phase_state_commit() {}

#[test]
fn final_synthesis_node_sets_final_synthesis_phase() {}

#[test]
fn unknown_phase_records_missing_context_reason() {}

#[test]
fn budget_recovery_overrides_pending_model_sampling_phase() {}
```

PowerShell fixtures:

```text
request-phase-summary fails when all non-final requests are model_sampling
request-phase-summary fails when unknown_request_phase_ratio > 5
request-phase-summary passes when state_commit / validation_recovery / final_synthesis are present
```

## B.10 Acceptance

```text
provider_request_hook_coverage >= 99
provider_request_terminal_coverage >= 99
request_phase_attribution_coverage >= 95
unknown_request_phase_ratio <= 5
phase_counts includes at least two non-model_sampling phases on synthetic fixture
```

## B.11 Implementation status

Status: implemented and revalidated locally on 2026-06-26.

Implemented scope:

```text
TaskSpaceProviderRequestPhase enum and strict as_str mapping
pending_provider_request_phase / pending_provider_request_context_reason / last_provider_request_context runtime state
provider_request_context_selected runtime trace event
next_provider_request_phase selection:
  compact checkpoint -> budget_recovery
  pending semantic phase -> pending phase
  missing current node -> unknown + current_main_node_missing
  final synthesis node -> final_synthesis
  smoke/regression nodes -> validation_recovery
  default -> model_sampling
state_commit accepted -> pending state_commit
record_subagent_plan accepted -> pending subagent_spawn
non-accepted result validity -> pending validation_recovery
ProviderRequestAttribution::from_snapshot
request-phase-summary phase_counts / phase_token_summary
request-phase-summary phase_diversity_gate_pass for synthetic fixtures
```

Deferred producers:

```text
projection_update
legacy_state_action
ordinary_tool_recovery
subagent_result_processing
```

These enum variants are reserved but not yet emitted by a semantic transition
producer in this phase. They should be wired only when their source transition
has a clear runtime event boundary.

Local validation:

```text
cargo test -p codex-core request_phase --lib
  4 passed

cargo test -p codex-core provider_request_budget --lib
  10 passed

pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
  cost instrumentation selftest passed

pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1
  Release decision self-test: PASS
  RunRoot: target/release-decision-selftest/run-20260624-202641-438

cargo build -p codex-cli --bin whale --locked
  finished dev build
```

Synthetic request-phase fixture evidence:

```text
phase_counts.model_sampling = 1
phase_counts.validation_recovery = 1
phase_counts.state_commit = 1
non_model_sampling_distinct_phase_count = 2
phase_diversity_gate_pass = true
phase_token_summary.state_commit.input_tokens = 7
phase_token_summary.validation_recovery.cached_input_tokens = 1
```

B-tier diagnostic rerun:

```text
run = target/phase-b-complete-B-rerun45/single-file-fast-fix/20260624-203208-963
valid_pair = True
included_in_utility_aggregate = False
outcome_taskspace = wrong
business_success = False
exec_exit_code = 1
public_validation_exit_code = 1
hidden_oracle_exit_code = 1
failure_taxonomy = agent_patch_wrong
provider_request_hook_coverage = 100
provider_request_terminal_coverage = 100
request_phase_attribution_coverage = 100
unknown_request_phase_ratio = 0
phase_counts.model_sampling = 24
phase_counts.budget_recovery = 5
phase_diversity_gate_pass = false
request_2_plus_hit_rate = 0.991148
trace_coverage = 1
tool_free_action_contract_count = 7
taskspace_control_count = 0
state_commit_count = 0
open_leaf_nodes = 1
```

Interpretation as of 2026-06-24: Phase B instrumentation gates were satisfied on
the real B-tier run, but the B-tier business gate failed because the agent did
not successfully patch `src/tax_calc.py`. The trace showed malformed
`taskspace_control` attempts with missing `action`, so the run never exercised
the new semantic phase producers beyond `budget_recovery`.

2026-06-26 update:

```text
root cause = action-contract taskspace_control ABI drift
fix commit = aef9f7a31 Fix TaskSpace action contract control ABI
full gate commit = 557fe3304 Fix codex-core TaskSpace gate regressions
taskspace_control_count now includes native controls and taskspace-action-v1 lifecycle controls
action_contract_taskspace_control_count is reported separately
cargo test -p codex-core --lib --quiet passed
```

Phase B therefore remains green for attribution, but the old rerun45 artifact is
no longer sufficient business evidence. The next B-tier smoke must be produced
after the ABI fix and must verify:

```text
business_success = true
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
request_phase_attribution_coverage >= 95
unknown_request_phase_ratio <= 5
taskspace_control_count > 0 when lifecycle transitions are required
state_commit_count > 0 when state_commit is the expected workflow path
blocked_by_budget_samples_count = 0
```
