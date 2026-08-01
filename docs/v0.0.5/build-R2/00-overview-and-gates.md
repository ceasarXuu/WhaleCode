# v0.0.5 Completion Engineering Playbook: Overview and Gates

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


- Created: 2026-06-21
- Last reviewed: 2026-06-26
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
spawn_node_budget_gate_pass = true  # 仅表示 advisory profile trace 存在、没有 profile blocked event、没有未审查 subagent result
```

Engineering re-entry before formal E3 requires:

```text
DeepSeek provider preflight succeeds for the selected model
current whale binary is rebuilt from the current codex source commit
terminal-bench_E3-P0_1_1 targeted diagnostic
model_request_count_ratio <= 2.5x
avg_input_per_request_ratio <= 1.25x
agent_walltime_ratio is measured from the pair report
blocked_by_budget_samples_count = 0 for release-like claims；这是旧 hard-stop 行为的回归计数
B-tier smoke may close only when business_success=true and the TaskSpace-side cache gate passes
open_leaf_nodes = 0 for release-like claims; otherwise record graph hygiene follow-up
agent_walltime_ratio <= 2.5x or a runtime bottleneck report exists with owner/follow-up
```

DeepSeek official TaskSpace runs also require the cache blocker gates in:

```text
docs/v0.0.5/缓存命中问题修复/README.md
```

The cache blocker has live provider evidence, but every DeepSeek official
TaskSpace benchmark still must archive the TaskSpace-side
`provider-cache-trace-summary.json`. The hard cache target is:

```text
request_2_plus_hit_rate >= 0.95
trace_coverage >= 0.99
cache_usage_missing_count == 0
native_tools_schema_hot_path_count == 0
tool_free_action_contract_count > 0
```

## 1.1 B-tier acceptance after cache repair

B-tier smoke is now a useful Phase A regression signal, but it is not utility
aggregate proof when the only exclusion is `repeats_lt_3`.

Required evidence for a B-tier pass:

```text
pair-report.md shows TaskSpace business_success=true and outcome_taskspace=solved
TaskSpace public_validation_exit_code = 0
TaskSpace hidden_oracle_exit_code = 0
TaskSpace provider-cache-trace-summary.json passes the hard cache target above
request-phase-summary shows provider_request_hook_coverage >= 99%
budget-quality-summary shows blocked_by_budget_samples_count = 0
spawn-node-budget-summary 可以显示 within_budget_status=over_profile_hint；只要 status=pass 就不是 release blocker
```

Current B evidence after the DeepSeek cache fix:

```text
run = target/phase-a-benefit-B-rerun40/single-file-fast-fix/20260624-012521-098
utility_direction = both_success
outcome_taskspace = solved
request_2_plus_hit_rate = 0.989997
trace_coverage = 1
cache_usage_missing_count = 0
native_tools_schema_hot_path_count = 0
tool_free_action_contract_count = 9
provider_request_hook_coverage = 100
request_phase_attribution_coverage = 100
blocked_by_budget_samples_count = 0
open_leaf_nodes = 1
taskspace_wall_time_ratio = 2.72
```

Interpretation: cache and business-success blockers are cleared for this smoke,
but `open_leaf_nodes=1` and `taskspace_wall_time_ratio=2.72` remain follow-up
gates before C/E3 or release-like claims.

## 1.2 Current status after Phase A/B and follow-up fixes

As of 2026-06-26, the post-A/B implementation state is:

```text
Phase A advisory profile semantics are implemented.
Phase B request phase attribution is implemented for core semantic producers.
Phase C producer-owned exact payload scan proof is implemented locally.
TaskSpace action-contract taskspace_control ABI canonicalization is fixed.
codex-core full library gate is green on the current commit.
```

Validated evidence:

```text
cargo test -p codex-core --lib --quiet
  1985 passed; 0 failed; 4 ignored

cargo check -p codex-cli --locked
  passed

taskspace_control action-contract ABI:
  action_name / command / control_action / control_type aliases are canonicalized before native handler execution
  taskspace_control_count now includes native tool calls and taskspace-action-v1 lifecycle controls

Phase C exact payload scan:
  exact_payload_scan runtime trace is produced by provider_payload_scanner
  active replacement artifacts no longer synthesize scan rows from provider budget booleans
  release decision fails hash-only, synthetic-producer, mismatched-hash, missing-provider-join, and missing-protected-items fixtures
```

This changes the remaining work shape. The old B-tier `missing field action`
failure is no longer a Phase B attribution blocker; it is now a repaired
action-contract ABI issue. A new post-ABI B-tier smoke is still required before
claiming business-success readiness.

## 2. Current alpha implementation baseline

The alpha branch now has these implementation states:

| Area | Existing implementation | Remaining problem |
|---|---|---|
| Provider request budget / profile | Phase A advisory-only dispatch and spawn/node profile gates are implemented | `budget_recovery` still exists as a compatibility attribution path; it must not become a profile hard-stop |
| Request phase attribution | `TaskSpaceProviderRequestPhase`, pending provider phase state, `ProviderRequestAttribution::from_snapshot`, `phase_counts`, and `phase_token_summary` exist | Some enum variants remain reserved/deferred; post-ABI B-tier rerun is still needed |
| TaskSpace action contract ABI | `taskspace-action-v1` lifecycle controls are canonicalized before native `taskspace_control`; benchmark usage splits native vs action-contract controls | Needs post-ABI B-tier business validation |
| Active context replacement / payload scan | Provider request events carry payload hash and scan booleans; `exact_payload_scan` runtime trace is emitted with producer `provider_payload_scanner`; release fixtures require request_id/hash join | Needs post-ABI B-tier evidence on a real run, but the Phase C producer-owned gate is implemented locally |
| Budget quality impact | Advisory-only profile semantics are reflected in budget quality summaries and can be packaged by `build-v005-non-agent-gates.ps1` | Needs formal sample-set identity and user approval before real E3 |
| `state_commit` displacement | `legacy_state_action_attempt` and `state_commit_displacement` runtime traces exist; release fixtures require a real legacy-attempt denominator; Phase G packages this as canonical non-agent evidence | Needs formal sample-set identity and user approval before real E3 |
| Spawn/node profile and subagent quality | Spawn/node profile traces are advisory; runtime enforces subagent plan and unreviewed-result gates; `spawn-node-budget-summary.json` now reports subagent review debt and release fixtures block unreviewed subagent results | Needs post-ABI B-tier evidence on a real run |
| Release/start gates | Release decision and E3 start gate know v0.0.5 markers; `build-v005-non-agent-gates.ps1` now builds current-HEAD local evidence with sha256 | Formal E3 still requires current code-complete marker and explicit user approval marker |

## 3. Implementation phases

Complete the work in this order. Do not run real E3 until Phase G is green.

```text
Phase A  Done: advisory active complexity profile and profile-hint observability
Phase B  Done with caveats: request phase attribution and context propagation
Phase C  Done locally: exact provider payload scan proof with producer-owned scan event and release fixtures
Phase D  Done for scope: advisory profile quality impact, full-field artifacts, and hard-stop regression detection
Phase E  Done for scope: legacy state action displacement denominator now uses real attempt events
Phase F  Done for scope: route-aware spawn/node hints remain advisory; release artifacts now block unreviewed subagent result debt
Phase G  Done for scope: canonical non-agent gate builder writes current-HEAD evidence bundle consumed by release/start gates
Phase H  Blocked after B-tier diagnostic: business/cache/fanout pass, but active context replacement, open_leaf_nodes, walltime, and timing attribution block targeted/formal E3
```
