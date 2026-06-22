# v0.0.5 Completion Engineering Playbook: Overview and Gates

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


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

DeepSeek official TaskSpace runs also require the cache blocker gates in:

```text
docs/v0.0.5/缓存命中问题修复/README.md
```

Until that project closes, DeepSeek TaskSpace benchmark output is diagnostic-only for cost-sensitive v0.0.5 experiments. The hard cache target is:

```text
steady_state_provider_cache_hit_rate_for_requests_2_plus >= 0.95
taskspace_uncached_input_tokens <= 1.2x standard_uncached_input_tokens on comparable diagnostic samples
cache trace coverage >= 99%
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
