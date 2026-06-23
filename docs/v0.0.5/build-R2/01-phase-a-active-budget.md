# Phase A. TaskSpaceActiveBudgetV1 and route-aware budget state

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


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
            max_rollout_model_requests: 8,
            max_model_requests_per_node: 3,
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
fn thin_route_budget_uses_eight_requests_and_no_spawn() { /* activate Thin; assert limits */ }

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

## A.11 Plan update after DeepSeek cache repair

The cache repair changes Phase A from a provider-blocked design task into a
measurable regression and benefit gate. Phase A should not be considered
complete merely because the B-tier sample solves the task.

Updated Phase A acceptance:

```text
DeepSeek provider preflight passes before any B/C diagnostic
current whale binary is rebuilt from the current codex source commit
TaskSpace B-tier business_success=true
TaskSpace public and hidden validation exit codes are 0
TaskSpace provider-cache-trace-summary.json passes the hard cache gate
budget-quality-summary records active_budget_source=runtime
budget-quality-summary records blocked_by_budget_samples_count=0
request-phase-summary records provider_request_hook_coverage >= 99%
request-phase-summary records request_phase_attribution_coverage >= 95%
```

Current B-tier smoke evidence:

```text
run = target/phase-a-benefit-B-rerun40/single-file-fast-fix/20260624-012521-098
outcome_taskspace = solved
request_2_plus_hit_rate = 0.989997
provider_request_hook_coverage = 100
request_phase_attribution_coverage = 100
blocked_by_budget_samples_count = 0
```

Open items produced by that same smoke:

```text
open_leaf_nodes = 1
taskspace_wall_time_ratio = 2.72
runtime_bottleneck_classification = agent_bound
```

Action: keep Phase A focused on typed active budget and request/node budget
evidence. Route the open leaf follow-up into Phase F/H graph hygiene, and route
the walltime follow-up into Phase H runtime bottleneck analysis before C/E3.
