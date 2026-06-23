# Phase B. Request phase attribution and context propagation

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


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
