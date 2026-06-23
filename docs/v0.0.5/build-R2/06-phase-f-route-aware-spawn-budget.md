# Phase F. Route-aware spawn/node/subagent budget enforcement

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


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
