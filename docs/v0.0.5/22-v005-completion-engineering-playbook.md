# v0.0.5 Completion Engineering Playbook

- Created: 2026-06-21
- Branch target: `whalecode-alpha`
- Status: engineering execution guide; not a release approval document
- Canonical design dependency: `18-unfinished-work-engineering-design.md`
- Scope: close the remaining v0.0.5 implementation gaps found in the alpha branch static audit

## 0. Purpose

This document turns the v0.0.5 alpha audit into concrete engineering work. It is intentionally implementation-oriented. Every item below names the code location, data shape, producer, consumer, tests, and acceptance gate.

This file does not authorize real E3. Real `terminal-bench_E3-P0_3_5` remains blocked until code-complete, non-agent gates, user approval, and E3 start gate all pass.

## 1. Closeout target

v0.0.5 can close only when all of the following are true on formal P0 evidence:

```text
sample_set_id = terminal-bench_E3-P0_3_5
TaskSpace solved >= Standard solved - 1
TaskSpace direct input+output ratio <= 2.0x Standard
TaskSpace agent walltime ratio <= 2.0x Standard
TaskSpace model_request_count ratio <= 2.0x Standard
provider_request_hook_coverage >= 99%
request_phase_attribution_coverage >= 95%
unknown_request_phase_ratio <= 5%
active_context_replacement_confirmed = true
budget_quality_impact_gate_pass = true
state_commit_displacement_gate_pass = true
spawn_node_budget_gate_pass = true
```

Engineering re-entry before formal E3 requires:

```text
terminal-bench_E3-P0_1_1 targeted diagnostic
model_request_count_ratio <= 2.5x
avg_input_per_request_ratio <= 1.25x
agent_walltime_ratio <= 2.5x
blocked_by_budget_samples_count = 0 for release-like claims
```

## 2. Current alpha implementation baseline

The alpha branch already has partial implementations in these areas:

| Area | Existing implementation | Remaining problem |
|---|---|---|
| Provider request budget | `ProviderRequestBudgetContext` in `core/src/client.rs`; request dispatch checks before HTTP/WebSocket request | Fixed max request count; not route-aware; phase attribution is too coarse |
| Active context replacement | `prepare_provider_visible_prompt_items` and `compose_provider_visible_history` in `core/src/session/turn.rs` | Marker-based; exact payload proof is mostly derived from budget events |
| Budget quality impact | `budget_quality_impact` trace generated from provider budget events in `action_map/runtime.rs` | Quality fields are mostly static; validator state is not actually joined |
| `state_commit` | `taskspace_control(action=state_commit)` handler and `state_commit_for_main` runtime method | Displacement denominator counts commit sections, not real legacy action attempts |
| Spawn/node budget | Node count and spawn/node budget trace events exist | Budget is fixed, not route/profile-aware; subagent result adoption budget is incomplete |
| Release decision | `write-release-decision.ps1` checks many new artifacts | Some artifacts are synthesized from summaries instead of canonical producer-owned facts |

## 3. Implementation phases

Complete the work in this order. Do not run real E3 until Phase G is green.

```text
Phase A  TaskSpaceActiveBudgetV1 and route-aware budget state
Phase B  Request phase attribution and context propagation
Phase C  Exact provider payload scan proof
Phase D  BudgetQualityImpactV1 with validator/quality semantics
Phase E  Legacy state action displacement denominator
Phase F  Route-aware spawn/node/subagent budget enforcement
Phase G  Non-agent gates, release-decision fixtures, start-gate fixtures
Phase H  Targeted diagnostic and formal E3 readiness
```

---

# Phase A. TaskSpaceActiveBudgetV1 and route-aware budget state

## A.1 Goal

Replace the current fixed constants with a typed active budget contract. Budget must be selected by route/profile and consumed by provider request, node, spawn, legacy action, projection, and quality gates.

## A.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/action_map/mod.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
third_party/codex-cli/codex-rs/core/src/client.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
```

## A.3 New Rust types

Add these types in `core/src/action_map/runtime.rs`. Re-export only the snapshot/input types through `action_map/mod.rs` when needed by `session`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceRouteMode {
    Thin,
    VerificationFirst,
    DefaultCompact,
    SubagentAssisted,
    Deep,
}

impl TaskSpaceRouteMode {
    pub(crate) fn as_str(self) -> &'static str { /* exact mapping */ }
    pub(crate) fn from_str(value: &str) -> Option<Self> { /* strict parser */ }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpaceBudgetState {
    Normal,
    Warned,
    CompactCheckpointRequired,
    ThinDowngraded,
    HardStopped,
}

impl TaskSpaceBudgetState {
    pub(crate) fn as_str(self) -> &'static str { /* exact mapping */ }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceActiveBudgetV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) profile_name: String,
    pub(crate) route_mode: TaskSpaceRouteMode,
    pub(crate) max_rollout_model_requests: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) max_spawn_agent_calls: usize,
    pub(crate) max_subagent_results: usize,
    pub(crate) max_nodes: usize,
    pub(crate) max_open_leaf_nodes: usize,
    pub(crate) max_legacy_state_actions: usize,
    pub(crate) max_projection_tokens: usize,
    pub(crate) max_avg_input_tokens_per_request: usize,
    pub(crate) post_budget_grace_requests: usize,
    pub(crate) budget_response_policy: TaskSpaceBudgetResponsePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceBudgetCounters {
    pub(crate) rollout_model_request_count: usize,
    pub(crate) model_request_count_by_node: std::collections::HashMap<String, usize>,
    pub(crate) spawn_agent_call_count: usize,
    pub(crate) subagent_result_count: usize,
    pub(crate) node_count: usize,
    pub(crate) open_leaf_node_count: usize,
    pub(crate) legacy_state_action_attempt_count: usize,
    pub(crate) legacy_state_action_displaced_count: usize,
    pub(crate) legacy_state_action_allowed_count: usize,
    pub(crate) state_commit_count: usize,
    pub(crate) projection_tokens_last: usize,
    pub(crate) projection_tokens_max: usize,
    pub(crate) post_budget_grace_request_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceBudgetViolation {
    pub(crate) violation_id: String,
    pub(crate) counter_name: String,
    pub(crate) counter_value: usize,
    pub(crate) counter_limit: usize,
    pub(crate) state_before: TaskSpaceBudgetState,
    pub(crate) state_after: TaskSpaceBudgetState,
    pub(crate) action_taken: String,
    pub(crate) created_at_ms: i64,
}
```

`TaskSpaceBudgetResponsePolicy` can start as a small enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceBudgetResponsePolicy {
    WarnCompactThinHardStop,
}
```

## A.4 Budget defaults

Implement one canonical function:

```rust
pub(crate) fn taskspace_active_budget_for_route(
    profile_name: &str,
    route_mode: TaskSpaceRouteMode,
) -> TaskSpaceActiveBudgetV1 {
    match route_mode {
        TaskSpaceRouteMode::Thin => TaskSpaceActiveBudgetV1 {
            profile_name: profile_name.to_string(),
            route_mode,
            max_rollout_model_requests: 4,
            max_model_requests_per_node: 2,
            max_spawn_agent_calls: 0,
            max_subagent_results: 0,
            max_nodes: 4,
            max_open_leaf_nodes: 2,
            max_legacy_state_actions: 0,
            max_projection_tokens: 12_000,
            max_avg_input_tokens_per_request: 12_000,
            post_budget_grace_requests: 1,
            ..default_budget_common()
        },
        TaskSpaceRouteMode::VerificationFirst => { /* requests <= 6, spawn = 0, nodes <= 5 */ }
        TaskSpaceRouteMode::DefaultCompact => { /* requests <= 10, spawn <= 2, nodes <= 8 */ }
        TaskSpaceRouteMode::SubagentAssisted => { /* requests <= 14, spawn <= 3, nodes <= 10 */ }
        TaskSpaceRouteMode::Deep => { /* requests <= 20, spawn <= 4, nodes <= 14 */ }
    }
}
```

Do not keep `DEFAULT_PROVIDER_REQUEST_BUDGET_MAX`, `DEFAULT_ACTIVE_SPAWN_AGENT_BUDGET_MAX`, and `DEFAULT_ACTIVE_NODE_BUDGET_MAX` as the main source of truth. They may remain as compatibility aliases only if they call this function.

## A.5 Runtime state changes

Extend `ActionMapRuntimeState`:

```rust
pub(crate) struct ActionMapRuntimeState {
    // existing fields
    active_budget: Option<TaskSpaceActiveBudgetV1>,
    budget_counters: TaskSpaceBudgetCounters,
    budget_state: TaskSpaceBudgetState,
    budget_violations: Vec<TaskSpaceBudgetViolation>,
}
```

Initialize in `Default`:

```rust
active_budget: None,
budget_counters: TaskSpaceBudgetCounters::default(),
budget_state: TaskSpaceBudgetState::Normal,
budget_violations: Vec::new(),
```

## A.6 New runtime functions

Add these methods to `ActionMapRuntimeState`:

```rust
pub(crate) fn activate_active_budget_for_route(
    &mut self,
    profile_name: &str,
    route_mode: TaskSpaceRouteMode,
) -> Vec<MapRuntimeEvent>;

pub(crate) fn active_budget(&self) -> Option<&TaskSpaceActiveBudgetV1>;

pub(crate) fn budget_counters(&self) -> &TaskSpaceBudgetCounters;

pub(crate) fn update_budget_state_for_counter(
    &mut self,
    counter_name: &str,
    counter_value: usize,
    counter_limit: usize,
    action_context: &str,
) -> Option<MapRuntimeEvent>;

pub(crate) fn gate_provider_request_pre_dispatch(
    &mut self,
    snapshot: &ActionMapProviderRequestBudgetSnapshot,
) -> TaskSpaceBudgetGateDecision;

pub(crate) fn gate_create_node_budget(
    &mut self,
    map_id: &str,
    candidate_node_kind: NodeKind,
) -> TaskSpaceBudgetGateDecision;

pub(crate) fn gate_spawn_budget(
    &mut self,
    map_id: &str,
    parent_node_id: &str,
) -> TaskSpaceBudgetGateDecision;
```

Use a common gate decision type:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceBudgetGateDecision {
    pub(crate) allowed: bool,
    pub(crate) budget_state: TaskSpaceBudgetState,
    pub(crate) reason: String,
    pub(crate) blocking_items: Vec<String>,
    pub(crate) next_valid_actions: Vec<String>,
    pub(crate) recovery_request_phase: Option<TaskSpaceProviderRequestPhase>,
    pub(crate) quality_impact_required: bool,
}
```

## A.7 Provider request snapshot contract

Extend `ActionMapProviderRequestBudgetSnapshot` in `runtime.rs`:

```rust
pub(crate) struct ActionMapProviderRequestBudgetSnapshot {
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: String,
    pub(crate) node_id: Option<String>,
    pub(crate) route_mode: Option<String>,
    pub(crate) profile_name: Option<String>,
    pub(crate) request_phase: Option<String>,
    pub(crate) provider_request_context_missing_reason: Option<String>,
    pub(crate) request_count: usize,
    pub(crate) max_requests: usize,
    pub(crate) node_request_count: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) post_budget_grace_requests: usize,
    pub(crate) budget_state: String,
}
```

`provider_request_budget_snapshot()` must read from `active_budget`, not from constants.

Pseudo implementation:

```rust
pub(crate) fn provider_request_budget_snapshot(&self) -> Option<ActionMapProviderRequestBudgetSnapshot> {
    if self.mode != MapRuntimeMode::Experiment { return None; }
    let budget = self.active_budget.as_ref()?;
    let map_id = self.active_map_id.clone()?;
    let node_id = self.current_main_node_id.clone();
    let phase = self.next_provider_request_phase(&map_id, node_id.as_deref());
    let node_request_count = node_id
        .as_ref()
        .and_then(|id| self.budget_counters.model_request_count_by_node.get(id).copied())
        .unwrap_or(0);

    Some(ActionMapProviderRequestBudgetSnapshot {
        task_id: self.maps.get(&map_id).and_then(|m| m.task_id.clone()),
        map_id,
        node_id,
        route_mode: Some(budget.route_mode.as_str().to_string()),
        profile_name: Some(budget.profile_name.clone()),
        request_phase: Some(phase.as_str().to_string()),
        provider_request_context_missing_reason: phase.missing_reason(),
        request_count: self.budget_counters.rollout_model_request_count,
        max_requests: budget.max_rollout_model_requests,
        node_request_count,
        max_model_requests_per_node: budget.max_model_requests_per_node,
        post_budget_grace_requests: budget.post_budget_grace_requests,
        budget_state: self.budget_state.as_str().to_string(),
    })
}
```

## A.8 Client budget dispatch change

`ProviderRequestBudgetContext::enabled_with_attribution(...)` currently receives only count and max. Replace with a snapshot-derived config:

```rust
pub(crate) struct ProviderRequestBudgetLimits {
    pub(crate) request_count: usize,
    pub(crate) max_requests: usize,
    pub(crate) node_request_count: usize,
    pub(crate) max_model_requests_per_node: usize,
    pub(crate) post_budget_grace_requests: usize,
    pub(crate) budget_state: String,
}
```

New signature:

```rust
pub(crate) fn enabled_with_attribution(
    limits: ProviderRequestBudgetLimits,
    attribution: ProviderRequestAttribution,
) -> Self
```

`before_dispatch()` must enforce both rollout and per-node limits:

```rust
if before >= max_requests {
    return blocked("provider_request_budget_exhausted");
}
if node_before >= max_model_requests_per_node && request_phase != Some("budget_recovery") {
    return blocked("provider_node_request_budget_exhausted");
}
```

## A.9 Tests

Add Rust tests in `core/src/action_map/runtime.rs` tests or a dedicated module:

```rust
#[test]
fn thin_route_budget_uses_four_requests_and_no_spawn() { /* activate Thin; assert limits */ }

#[test]
fn default_compact_budget_uses_ten_requests_and_two_spawn() { /* activate DefaultCompact */ }

#[test]
fn provider_request_snapshot_uses_active_budget_not_constants() { /* assert max_requests */ }

#[test]
fn node_request_budget_blocks_before_rollout_budget() { /* node limit exhausted */ }
```

Add client tests in `core/src/client_tests.rs`:

```rust
#[test]
fn provider_request_budget_blocks_per_node_limit() { /* enabled context with node max = 1 */ }

#[test]
fn provider_request_budget_allows_budget_recovery_grace_once() { /* phase budget_recovery */ }
```

## A.10 Acceptance

```text
cargo test -p codex-core taskspace_active_budget
cargo test -p codex-core provider_request_budget
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
```

Artifacts must show:

```json
{
  "active_budget_source": "runtime",
  "route_mode": "thin|verification_first|default_compact|subagent_assisted|deep",
  "max_rollout_model_requests": "route-specific",
  "max_model_requests_per_node": "route-specific"
}
```

---

# Phase B. Request phase attribution and context propagation

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

---

# Phase C. Exact provider payload scan proof

## C.1 Goal

Release proof must be tied to the exact provider-visible payload. It must not be inferred only from `projection-events.jsonl` or from booleans synthesized by `cost-instrumentation.ps1`.

## C.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/client.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## C.3 New client-side structs

In `client.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ExactPayloadScanEventV1 {
    pub(crate) schema_version: &'static str,
    pub(crate) scan_event_id: String,
    pub(crate) request_id: String,
    pub(crate) provider_payload_sha256: String,
    pub(crate) scanner_version: String,
    pub(crate) matcher_version: String,
    pub(crate) checked_byte_ranges: Vec<(usize, usize)>,
    pub(crate) negative_checks_performed: Vec<String>,
    pub(crate) active_projection_present: bool,
    pub(crate) legacy_taskspace_history_present: bool,
    pub(crate) raw_taskspace_control_history_tokens: usize,
    pub(crate) completed_stale_node_history_tokens: usize,
    pub(crate) rejected_subagent_body_tokens: usize,
    pub(crate) large_raw_output_tokens: usize,
    pub(crate) protected_items_present: bool,
    pub(crate) passed: bool,
    pub(crate) failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderPayloadEvidenceV1 {
    pub(crate) sha256: String,
    pub(crate) bytes: usize,
    pub(crate) artifact_path: Option<String>,
    pub(crate) exact_scan: ExactPayloadScanEventV1,
}
```

Replace `provider_payload_digest` with:

```rust
fn provider_payload_evidence<T: serde::Serialize>(
    request_id: &str,
    payload: &T,
    capture_policy: ProviderPayloadCapturePolicy,
) -> Option<ProviderPayloadEvidenceV1>
```

## C.4 Scanner implementation

Pseudo:

```rust
fn scan_provider_payload_text(request_id: &str, sha256: &str, text: &str) -> ExactPayloadScanEventV1 {
    let active_projection_present = text.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER);
    let legacy_taskspace_history_present = contains_any(text, &[
        TASKSPACE_SHADOW_PROJECTION_MARKER,
        "TaskSpace Bootstrap",
        "TaskSpace ContextProjectionV1 shadow update",
        "TaskSpace mode is now active",
        "taskspace_control("
    ]);
    let raw_taskspace_control_history_tokens = count_token_estimate_for_blocks(text, "taskspace_control");
    let completed_stale_node_history_tokens = count_token_estimate_for_blocks(text, "completed stale node");
    let rejected_subagent_body_tokens = count_token_estimate_for_blocks(text, "rejected subagent");
    let large_raw_output_tokens = estimate_large_raw_output_tokens(text);
    let protected_items_present = active_projection_block_contains_protected_items(text);

    let mut failure_reasons = Vec::new();
    if !active_projection_present { failure_reasons.push("active_projection_missing".into()); }
    if legacy_taskspace_history_present { failure_reasons.push("legacy_taskspace_history_present".into()); }
    if large_raw_output_tokens > 0 { failure_reasons.push("large_raw_output_present".into()); }
    if !protected_items_present { failure_reasons.push("protected_items_missing".into()); }

    ExactPayloadScanEventV1 {
        schema_version: "taskspace-exact-payload-scan-event-v1",
        scan_event_id: format!("scan:{request_id}:{sha256}"),
        request_id: request_id.to_string(),
        provider_payload_sha256: sha256.to_string(),
        scanner_version: "v005-exact-scan-2".to_string(),
        matcher_version: "v005-marker-and-structural-negative-checks-2".to_string(),
        checked_byte_ranges: vec![(0, text.len())],
        negative_checks_performed: vec![
            "legacy_taskspace_history".into(),
            "raw_taskspace_control_history".into(),
            "completed_stale_node_history".into(),
            "rejected_subagent_body".into(),
            "large_raw_output".into(),
        ],
        active_projection_present,
        legacy_taskspace_history_present,
        raw_taskspace_control_history_tokens,
        completed_stale_node_history_tokens,
        rejected_subagent_body_tokens,
        large_raw_output_tokens,
        protected_items_present,
        passed: failure_reasons.is_empty(),
        failure_reasons,
    }
}
```

## C.5 Event propagation

`ProviderRequestBudgetEvent` must add:

```rust
pub(crate) exact_payload_scan_event_id: Option<String>,
pub(crate) provider_payload_artifact: Option<String>,
pub(crate) raw_taskspace_control_history_tokens: Option<usize>,
pub(crate) completed_stale_node_history_tokens: Option<usize>,
pub(crate) rejected_subagent_body_tokens: Option<usize>,
```

`record_provider_payload` must push both:

```text
provider_request_budget status=payload_captured
exact_payload_scan event with same request_id + provider_payload_sha256
```

The scan event must be created before redaction/hash-only fallback. If payload artifact capture is disabled for privacy, scan event is still mandatory.

## C.6 Artifact generation rule

Update `New-TaskspaceActiveReplacementArtifacts`:

Current unacceptable pattern:

```powershell
# Do not synthesize scan events from budget event booleans alone.
$scanId = "scan-$($event.trace_event_id)"
$passed = [bool]$event.exact_payload_scan_passed -and ...
```

Required pattern:

```powershell
$exactPayloadScanEvents = Get-TaskspaceTraceEvents $ObservabilityJsonPath @("exact_payload_scan")
$providerRequestEvents = Get-TaskspaceTraceEvents $ObservabilityJsonPath @("provider_request_budget")

foreach ($scan in $exactPayloadScanEvents) {
    $matchingProvider = $providerRequestEvents | Where-Object {
        $_.request_id -eq $scan.request_id -and
        $_.provider_payload_sha256 -eq $scan.provider_payload_sha256
    }
    if (-not $matchingProvider) { mark failure }
}
```

## C.7 Release gate rule

`write-release-decision.ps1` must fail when:

```text
active-context-replacement-report.json is present but exact-payload-scan-events.jsonl is empty
scan_event_id does not join provider request event by request_id and payload hash
scan event was synthesized by cost instrumentation instead of producer=provider_lifecycle or producer=provider_payload_scanner
provider_payload_sha256 is empty
legacy_taskspace_history_present=true
large_raw_output_tokens>0
protected_items_present=false
```

## C.8 Tests

Rust tests in `client_tests.rs`:

```rust
#[test]
fn exact_payload_scan_event_id_matches_request_and_payload_hash() {}

#[test]
fn exact_payload_scan_fails_when_shadow_projection_present() {}

#[test]
fn exact_payload_scan_fails_when_large_raw_output_present() {}

#[test]
fn exact_payload_scan_passes_active_projection_with_protected_items() {}
```

PowerShell fixtures:

```text
release-decision fails when exact scan is synthesized without provider event
release-decision fails when scan hash mismatches provider request hash
release-decision passes when scan joins provider request by request_id/hash
```

## C.9 Acceptance

```text
active_context_replacement_gate_pass = true
exact_payload_scan_gate_pass = true
exact_payload_scan_matching_provider_event_count > 0
legacy_taskspace_history_present = false
large_raw_output_tokens = 0
protected_items_present = true
```

---

# Phase D. BudgetQualityImpactV1 with validator/quality semantics

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

---

# Phase E. Legacy state action displacement denominator

## E.1 Goal

`state_commit_displacement` must measure real legacy actions attempted / displaced / allowed. It must not use `state_commit` section count as the denominator.

## E.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## E.3 Runtime tracking structs

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyStateActionAttemptV1 {
    pub(crate) action: String,
    pub(crate) task_id: Option<String>,
    pub(crate) map_id: Option<String>,
    pub(crate) node_id: Option<String>,
    pub(crate) displaced: bool,
    pub(crate) allowed: bool,
    pub(crate) reason: String,
    pub(crate) created_at_ms: i64,
}
```

Add field:

```rust
legacy_state_action_attempts: Vec<LegacyStateActionAttemptV1>,
```

## E.4 Handler refactor

Current handler calls a pure reject function before executing actions. Replace:

```rust
reject_legacy_state_action_for_active_profile(&args)?;
```

with:

```rust
if let Some(action) = legacy_state_action_name(&args) {
    session
        .record_action_map_legacy_state_action_attempt(
            &turn,
            action,
            /*displaced*/ true,
            /*allowed*/ false,
            "active_profile_requires_state_commit",
        )
        .await;
    return Err(FunctionCallError::RespondToModel(format!(
        "TaskSpace active profile blocks legacy state action `{action}`. Use taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1) to batch state changes; start_task initial_* fields remain allowed for new-task setup."
    )));
}
```

Add:

```rust
fn legacy_state_action_name(args: &TaskSpaceControlArgs) -> Option<&'static str> {
    match args {
        TaskSpaceControlArgs::RecordOutputContract { .. } => Some("record_output_contract"),
        TaskSpaceControlArgs::RecordFactSource { .. } => Some("record_fact_source"),
        TaskSpaceControlArgs::RecordFact { .. } => Some("record_fact"),
        TaskSpaceControlArgs::RecordSuccessCriteria { .. } => Some("record_success_criteria"),
        TaskSpaceControlArgs::RecordOpenQuestion { .. } => Some("record_open_question"),
        TaskSpaceControlArgs::CloseOpenQuestion { .. } => Some("close_open_question"),
        TaskSpaceControlArgs::RecordDecision { .. } => Some("record_decision"),
        TaskSpaceControlArgs::RecordNextBestAction { .. } => Some("record_next_best_action"),
        TaskSpaceControlArgs::MarkResultValidity { .. } => Some("mark_result_validity"),
        TaskSpaceControlArgs::AdoptResult { .. } => Some("adopt_result"),
        _ => None,
    }
}
```

## E.5 Session wrapper

Add to `Session` in `core/src/session/mod.rs`:

```rust
pub(crate) async fn record_action_map_legacy_state_action_attempt(
    &self,
    turn_context: &TurnContext,
    action: &str,
    displaced: bool,
    allowed: bool,
    reason: &str,
) {
    let events = {
        let mut state = self.state.lock().await;
        state.action_map_runtime.record_legacy_state_action_attempt(
            action,
            displaced,
            allowed,
            reason,
        )
    };
    self.emit_map_runtime_events(turn_context, events).await;
}
```

## E.6 Runtime method

```rust
pub(crate) fn record_legacy_state_action_attempt(
    &mut self,
    action: &str,
    displaced: bool,
    allowed: bool,
    reason: &str,
) -> Vec<MapRuntimeEvent> {
    if self.mode != MapRuntimeMode::Experiment { return Vec::new(); }
    let attempt = LegacyStateActionAttemptV1 { ... };
    self.budget_counters.legacy_state_action_attempt_count += 1;
    if displaced { self.budget_counters.legacy_state_action_displaced_count += 1; }
    if allowed { self.budget_counters.legacy_state_action_allowed_count += 1; }
    self.legacy_state_action_attempts.push(attempt);
    vec![self.record_runtime_budget_trace_event(
        "legacy_state_action_attempt",
        ...,
        vec![
            "schema:taskspace-legacy-state-action-attempt-v1".to_string(),
            "producer:runtime".to_string(),
            format!("action:{action}"),
            format!("displaced:{displaced}"),
            format!("allowed:{allowed}"),
            format!("reason:{reason}"),
        ],
    )]
}
```

## E.7 State commit displacement event

In `state_commit_for_main`, replace:

```rust
let legacy_state_action_attempt_count = outcome.accepted_sections.len() + outcome.rejected_sections.len();
let legacy_state_action_displaced_count = outcome.accepted_sections.len();
```

with:

```rust
let legacy_state_action_attempt_count = self.budget_counters.legacy_state_action_attempt_count;
let legacy_state_action_displaced_count = self.budget_counters.legacy_state_action_displaced_count;
let legacy_state_action_count = self.budget_counters.legacy_state_action_allowed_count;
```

Also emit `state_commit_section_count` separately so compression can be measured without corrupting displacement denominator.

## E.8 Script changes

`New-TaskspaceStateCommitDisplacementSummary` must read both:

```text
legacy_state_action_attempt events
state_commit_displacement events
```

Summary rules:

```powershell
$attemptCount = count legacy_state_action_attempt where producer=runtime
$displacedCount = count attempts where displaced=true
$allowedCount = count attempts where allowed=true
$stateCommitCount = count state_commit_displacement where status in accepted,partial
$status = if ($stateCommitCount -gt 0 -and $attemptCount -gt 0 -and $displacedCount -ge $attemptCount -and $allowedCount -le $legacyBudget) { "pass" } else { "fail" }
```

## E.9 Tests

Rust:

```rust
#[test]
fn legacy_state_action_rejection_records_attempt_and_displacement() {}

#[test]
fn state_commit_displacement_denominator_uses_legacy_attempts_not_sections() {}

#[test]
fn state_commit_sections_do_not_increment_legacy_attempt_count() {}
```

PowerShell:

```text
state_commit_displacement fails if no legacy_state_action_attempt event exists
state_commit_displacement fails if displacement denominator is state_commit section count only
state_commit_displacement passes when legacy attempt is blocked and state_commit accepted
```

## E.10 Acceptance

```text
legacy_state_action_attempt_count > 0 on displacement fixture
legacy_state_action_displaced_count >= legacy_state_action_attempt_count
legacy_state_action_count <= route budget
state_commit_count > 0
state_commit_section_count is reported separately
```

---

# Phase F. Route-aware spawn/node/subagent budget enforcement

## F.1 Goal

Fanout must be limited by route/profile budget and by result adoption. Spawn is allowed only when it has a decision target, bounded scope, and adoption path.

## F.2 Files to change

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/tools/src/agent_tool.rs
third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs
scripts/taskspace-benchmark/lib/cost-instrumentation.ps1
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
```

## F.3 Gate functions

Add these methods:

```rust
pub(crate) fn gate_record_subagent_plan(
    &mut self,
    parent_node_id: &str,
    plan: &ActionMapSubagentPlanInput,
) -> Result<Vec<MapRuntimeEvent>, ActionMapGateError>;

pub(crate) fn gate_spawn_agent_call(
    &mut self,
    parent_node_id: &str,
    plan_id: Option<&str>,
) -> Result<Vec<MapRuntimeEvent>, ActionMapGateError>;

pub(crate) fn record_subagent_result_decision(
    &mut self,
    result_id: &str,
    decision: SubagentResultDecision,
) -> Vec<MapRuntimeEvent>;
```

Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubagentResultDecision {
    Adopt,
    Reject,
    Defer,
}
```

## F.4 Record subagent plan validation

Before accepting `record_subagent_plan`, require:

```text
why_parallelizable non-empty
expected_artifact non-empty
acceptance_check non-empty
max_scope non-empty
supports_questions or tests_hypotheses or depends_on_results non-empty
decision_target present
```

If missing, block with:

```text
reason = subagent_plan_missing_decision_target
next_valid_actions = [
  "finish current inspect node",
  "continue main-agent serial work",
  "record_subagent_plan with decision_target and bounded scope"
]
```

## F.5 Spawn budget enforcement

Pseudo:

```rust
fn gate_spawn_agent_call(&mut self, parent_node_id: &str, plan_id: Option<&str>) -> Result<Vec<MapRuntimeEvent>, ActionMapGateError> {
    let budget = self.active_budget.as_ref().ok_or_else(no_budget)?;
    if budget.max_spawn_agent_calls == 0 {
        return Err(block_spawn("route_disallows_spawn"));
    }
    if self.budget_counters.spawn_agent_call_count >= budget.max_spawn_agent_calls {
        return Err(block_spawn("spawn_budget_exhausted"));
    }
    let plan = require_plan_with_decision_target(plan_id)?;
    if has_no_yield_debt_for_plan_class(plan) {
        return Err(block_spawn("same_class_spawn_disabled_after_no_yield"));
    }
    self.budget_counters.spawn_agent_call_count += 1;
    emit_spawn_node_budget_event("spawn", "allowed")
}
```

## F.6 Subagent result adoption

Add tracking:

```rust
pub(crate) struct PendingSubagentResultReview {
    pub(crate) result_id: String,
    pub(crate) plan_id: String,
    pub(crate) created_at_provider_request_count: usize,
    pub(crate) deadline_provider_request_count: usize,
    pub(crate) decision: Option<SubagentResultDecision>,
}
```

Rules:

```text
Every subagent result must be adopt/reject/defer within N=2 main provider requests.
If deadline passes, block new spawn and require decision.
If two no-yield results for same plan class occur, disable same-class spawn for the run.
unreviewed_subagent_result_count must be 0 for release.
```

## F.7 Cost artifact

`spawn-node-budget-summary.json` must include:

```json
{
  "schema_version": "taskspace-spawn-node-budget-summary-v1",
  "status": "pass|fail",
  "route_mode": "thin",
  "spawn_agent_call_count": 0,
  "max_spawn_agent_calls": 0,
  "subagent_result_count": 0,
  "max_subagent_results": 0,
  "unreviewed_subagent_result_count": 0,
  "subagent_no_decision_yield_count": 0,
  "same_class_spawn_disabled_count": 0,
  "node_count": 3,
  "max_nodes": 4,
  "open_leaf_node_count": 1,
  "max_open_leaf_nodes": 2
}
```

## F.8 Tests

Rust:

```rust
#[test]
fn thin_route_blocks_spawn_even_with_subagent_plan() {}

#[test]
fn default_route_allows_spawn_with_decision_target() {}

#[test]
fn spawn_without_decision_target_is_blocked() {}

#[test]
fn unreviewed_subagent_result_blocks_new_spawn_after_deadline() {}

#[test]
fn node_budget_uses_route_budget_not_default_constant() {}
```

PowerShell:

```text
spawn-node-budget summary fails when unreviewed_subagent_result_count > 0
spawn-node-budget summary fails when spawn_count > route budget
spawn-node-budget summary passes for thin route with spawn=0 and node_count<=4
```

## F.9 Acceptance

```text
spawn_agent_call_count <= max_spawn_agent_calls
subagent_result_count <= max_subagent_results
unreviewed_subagent_result_count = 0
subagent_no_decision_yield_count = 0
node_count <= max_nodes
open_leaf_node_count <= max_open_leaf_nodes
```

---

# Phase G. Non-agent gates, release fixtures, start-gate fixtures

## G.1 Goal

Before any real E3, prove all new implementation contracts with deterministic non-agent tests and local artifacts.

## G.2 Files to change

```text
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-e3-start-gate.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
scripts/taskspace-benchmark/lib/e3-start-gate.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
```

Optionally add:

```text
scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
```

## G.3 Required non-agent gates

`v005-non-agent-gates.json` must contain these gates:

```json
{
  "schema_version": 1,
  "status": "pass",
  "gates": {
    "provider_request_hook": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "runtime_budget_response": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "budget_quality_impact": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "active_context_replacement": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "state_commit_displacement": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "spawn_node_budget": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "request_phase_attribution": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "release_decision_fixture": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "start_gate_fixture": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." }
  },
  "git_commit": "<current HEAD>",
  "profile_hash": "<runner profile hash>",
  "task_list_hash": "<formal P0 task list hash>",
  "source_version": "<terminal-bench source version>",
  "generated_at": "<ISO8601>"
}
```

Every gate evidence path must be a local file. `selftest://` is not acceptable.

## G.4 Build script pseudocode

If adding `build-v005-non-agent-gates.ps1`, implement:

```powershell
param(
  [Parameter(Mandatory=$true)][string]$RunRoot,
  [Parameter(Mandatory=$true)][string]$TaskListHash,
  [Parameter(Mandatory=$true)][string]$ProfileHash,
  [Parameter(Mandatory=$true)][string]$SourceVersion
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$head = git -C $repoRoot rev-parse HEAD
$gates = [ordered]@{}

function Invoke-Gate($Name, $Command, $EvidencePath) {
  $result = Invoke-Expression $Command
  $sha = if (Test-Path $EvidencePath) { (Get-FileHash $EvidencePath -Algorithm SHA256).Hash.ToLowerInvariant() } else { "" }
  $gates[$Name] = [pscustomobject]@{
    status = if ($LASTEXITCODE -eq 0 -and $sha) { "pass" } else { "fail" }
    producer = "build-v005-non-agent-gates.ps1"
    command = $Command
    exit_code = $LASTEXITCODE
    generated_at = (Get-Date).ToString("o")
    git_commit = $head
    profile_hash = $ProfileHash
    task_list_hash = $TaskListHash
    source_version = $SourceVersion
    evidence_path = $EvidencePath
    evidence_sha256 = $sha
  }
}

Invoke-Gate "provider_request_hook" "cargo test -p codex-core provider_request_budget --locked" "$RunRoot\evidence\provider_request_hook.txt"
Invoke-Gate "active_context_replacement" "cargo test -p codex-core active_context_replacement --locked" "$RunRoot\evidence\active_context_replacement.txt"
Invoke-Gate "release_decision_fixture" "pwsh -File scripts\taskspace-benchmark\test-release-decision.ps1" "$RunRoot\evidence\release_decision_fixture.txt"
# etc.

[pscustomobject]@{
  schema_version = 1
  status = if (@($gates.Values | Where-Object { $_.status -ne "pass" }).Count -eq 0) { "pass" } else { "fail" }
  gates = [pscustomobject]$gates
  git_commit = $head
  profile_hash = $ProfileHash
  task_list_hash = $TaskListHash
  source_version = $SourceVersion
  generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $RunRoot "v005-non-agent-gates.json")
```

## G.5 Release-decision negative fixtures

`test-release-decision.ps1` must include failing fixtures for:

```text
missing exact-payload-scan-events.jsonl
exact scan hash mismatch
provider request event producer not provider_lifecycle
request phase attribution all unknown
request phase attribution all model_sampling despite state_commit event
budget action without budget quality impact
BudgetQualityImpact final_classification=solved but score_eligible=false
state_commit_displacement without legacy_state_action_attempt events
spawn-node-budget unreviewed_subagent_result_count > 0
diagnostic sample_set_id terminal-bench_E3-P0_3_2 attempts release_pass
blocked_partial attempts closeable=true
```

## G.6 Start-gate fixture requirements

`test-e3-start-gate.ps1` must prove:

```text
full_e3_allowed=false when v005-non-agent-gates missing
full_e3_allowed=false when code-complete marker stale
full_e3_allowed=false when user approval sample set != terminal-bench_E3-P0_3_5
full_e3_allowed=false when task_list derivation != terminal-bench_E3-P0_3_5
full_e3_allowed=true only when all identities match and all markers are fresh/pass
```

## G.7 Acceptance

```text
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1
pwsh -File scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
```

---

# Phase H. Targeted diagnostic and formal E3 readiness

## H.1 Diagnostic sequence

Only after Phases A-G are green:

```powershell
# Non-agent gates first.
pwsh -File scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1 `
  -RunRoot <run-root> `
  -TaskListHash <formal-task-list-hash> `
  -ProfileHash <profile-hash> `
  -SourceVersion <source-version>

# Then one targeted diagnostic, not release proof.
pwsh -File scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1 `
  -SampleSet terminal-bench_E3-P0_1_1 `
  -EvidenceTarget diagnostic-only `
  -Profile taskspace-v005-active
```

The targeted diagnostic must show:

```text
request count is no longer 30x-190x Standard
spawn count stays within route budget
active payload scan passes
request phase summary has meaningful phase distribution
budget quality impact summary has no silent validation skip
```

## H.2 Formal E3 start gate

Only after targeted diagnostic is acceptable:

```powershell
pwsh -File scripts/taskspace-benchmark/lib/e3-start-gate.ps1 `
  -ExpectedSampleSetId terminal-bench_E3-P0_3_5 `
  -V005NonAgentGatesPath <run-root>\v005-non-agent-gates.json `
  -V005CodeCompletePath <run-root>\v005-code-complete.json `
  -V005UserApprovalPath <run-root>\v005-user-approval.json
```

Formal E3 may run only if:

```text
start-gate/gate-decision.json full_e3_allowed = true
start-gate/gate-decision.json v005_markers_passed = true
start-gate/gate-decision.json calibration_gate_passed = true
```

## H.3 Formal E3 command category

The formal command must produce:

```text
run-status.json evidence_target = E3
run-status.json sample_set_id = terminal-bench_E3-P0_3_5
run-status.json repeats_per_sample >= 5
pair_completed reported_evidence_level = E3 for every counted pair
formal pair ledger = exactly 3 samples x 5 repeats
```

`terminal-bench_E3-P0_1_1`, `_3_1`, and `_3_2` must never produce `release_pass`.

---

# 4. Module-by-module change checklist

## 4.1 `core/src/action_map/runtime.rs`

Required work:

```text
[ ] Add TaskSpaceRouteMode
[ ] Add TaskSpaceActiveBudgetV1
[ ] Add TaskSpaceBudgetCounters
[ ] Add TaskSpaceBudgetState
[ ] Add TaskSpaceProviderRequestPhase
[ ] Add BudgetQualityImpactV1
[ ] Add LegacyStateActionAttemptV1
[ ] Replace fixed budget constants with route-aware active_budget
[ ] Extend provider_request_budget_snapshot
[ ] Add next_provider_request_phase
[ ] Add record_budget_quality_impact
[ ] Add record_legacy_state_action_attempt
[ ] Fix state_commit_displacement denominator
[ ] Add spawn/node/subagent result review budget gates
```

## 4.2 `core/src/client.rs`

Required work:

```text
[ ] Replace count/max-only ProviderRequestBudgetContext with ProviderRequestBudgetLimits
[ ] Enforce rollout request budget
[ ] Enforce per-node request budget
[ ] Generate ExactPayloadScanEventV1 before redaction/hash-only fallback
[ ] Add exact_payload_scan_event_id to ProviderRequestBudgetEvent
[ ] Preserve request_id/logical_request_id/attempt across retry/fallback
[ ] Ensure terminal event is generated for completed/error/cancelled/blocked
```

## 4.3 `core/src/session/turn.rs`

Required work:

```text
[ ] Build ProviderRequestAttribution from full snapshot
[ ] Preserve missing context reason
[ ] Add hard-stop BudgetQualityImpact producer
[ ] Add no-action recovery exhausted BudgetQualityImpact producer
[ ] Ensure active context replacement emits proof context before provider request
[ ] Add tests for active replacement leak cases
```

## 4.4 `core/src/tools/handlers/taskspace_control.rs`

Required work:

```text
[ ] Replace pure legacy reject with runtime-recorded legacy attempt
[ ] Set next provider phase after state_commit
[ ] Set next provider phase after record_subagent_plan
[ ] Ensure state_commit errors still produce displacement/rejection evidence
```

## 4.5 `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`

Required work:

```text
[ ] Parse exact_payload_scan runtime events instead of synthesizing scan events from budget booleans
[ ] Add phase_counts and phase_token_summary
[ ] Parse BudgetQualityImpactV1 full fields
[ ] Parse legacy_state_action_attempt events
[ ] Parse route-aware spawn/node/subagent budget fields
[ ] Fail summaries when required producer-owned evidence is missing
```

## 4.6 `scripts/taskspace-benchmark/write-release-decision.ps1`

Required work:

```text
[ ] Block release if exact payload scan is missing or hash/request mismatch
[ ] Block release if active replacement proof is hash-only without exact scan
[ ] Block release if BudgetQualityImpact has validation skip or score-ineligible solved
[ ] Block release if state_commit_displacement denominator lacks legacy attempts
[ ] Block release if spawn/node budget has unreviewed subagent results
[ ] Block release if diagnostic sample set attempts release_pass
[ ] Keep blocked_partial closeable=false
```

---

# 5. Definition of done

The implementation is code-complete only when all are true:

```text
cargo check -p codex-cli --locked passes
cargo test -p codex-core provider_request_budget passes
cargo test -p codex-core active_context_replacement passes
cargo test -p codex-core state_commit passes
cargo test -p codex-core spawn_node_budget passes
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1 passes
v005-non-agent-gates.json exists and status=pass
v005-code-complete.json exists and status=pass
v005-user-approval.json exists and explicitly approves terminal-bench_E3-P0_3_5
start-gate/gate-decision.json full_e3_allowed=true
```

Then, and only then, run formal `terminal-bench_E3-P0_3_5`.

---

# 6. Recommended PR split

Use this split to keep reviews bounded:

1. `v005-budget-contract`: Phase A runtime/client budget contract and tests.
2. `v005-request-phase`: Phase B phase attribution and cost summary fixtures.
3. `v005-payload-proof`: Phase C exact payload scan proof and release gate updates.
4. `v005-quality-impact`: Phase D quality impact events and scoring blockers.
5. `v005-state-commit-displacement`: Phase E legacy denominator fix.
6. `v005-spawn-budget`: Phase F route-aware spawn/node/subagent result budget.
7. `v005-gates`: Phase G non-agent gates, release-decision fixtures, start-gate fixtures.
8. `v005-diagnostic`: Phase H targeted diagnostic evidence only; no release close.

Do not combine Phase H with implementation PRs. Diagnostic evidence must be produced from a clean code-complete commit.
