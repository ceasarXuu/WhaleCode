# v0.0.5 Completion Engineering Playbook: Module Checklist and Closeout

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.

Status legend:

```text
[x] implemented and covered by current local gates
[~] partially implemented or implemented in a shape that does not yet satisfy the final contract
[ ] not implemented / still a blocker
```


## 4.1 `core/src/action_map/runtime.rs`

Required work:

```text
[x] Add TaskSpaceRouteMode
[x] Add TaskSpaceActiveBudgetV1 as advisory complexity profile
[x] Add TaskSpaceBudgetCounters
[x] Add TaskSpaceBudgetState
[x] Add TaskSpaceProviderRequestPhase
[~] Add BudgetQualityImpactV1 / equivalent budget_quality_impact trace semantics
[ ] Add LegacyStateActionAttemptV1 as an independent producer
[x] Replace fixed budget constants with route-aware advisory profile
[x] Extend provider_request_budget_snapshot
[x] Add next_provider_request_phase
[~] Add record_budget_quality_impact; keep as advisory quality/regression signal
[ ] Add record_legacy_state_action_attempt
[ ] Fix state_commit_displacement denominator
[~] Add spawn/node profile observability and subagent result review quality gates
```

## 4.2 `core/src/client.rs`

Required work:

```text
[~] Replace count/max-only ProviderRequestBudgetContext; current shape still carries profile max fields but dispatch is advisory
[x] Record rollout request profile hints without blocking dispatch
[x] Record per-node request profile hints without forcing recovery
[ ] Generate producer-owned ExactPayloadScanEventV1 before redaction/hash-only fallback
[ ] Add exact_payload_scan_event_id to ProviderRequestBudgetEvent from producer-owned scan
[x] Preserve request_id/logical_request_id/attempt across retry/fallback
[x] Ensure terminal event is generated for completed/error/cancelled/blocked
```

## 4.3 `core/src/session/turn.rs`

Required work:

```text
[x] Build ProviderRequestAttribution from full snapshot
[x] Preserve missing context reason
[x] Remove hard-stop BudgetQualityImpact producer; keep legacy hard-stop regression detection
[~] Add no-action recovery exhausted BudgetQualityImpact producer
[~] Ensure active context replacement emits proof context before provider request
[~] Add tests for active replacement leak cases
[x] Canonicalize taskspace-action-v1 taskspace_control action aliases before native handler execution
```

## 4.4 `core/src/tools/handlers/taskspace_control.rs`

Required work:

```text
[ ] Replace pure legacy reject with runtime-recorded legacy attempt
[x] Set next provider phase after state_commit
[x] Set next provider phase after record_subagent_plan
[~] Ensure state_commit errors still produce displacement/rejection evidence
[x] Normalize taskspace_control action aliases at native handler boundary
```

## 4.5 `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`

Required work:

```text
[ ] Parse exact_payload_scan runtime events instead of synthesizing scan events from budget booleans
[x] Add phase_counts and phase_token_summary
[~] Parse BudgetQualityImpactV1 full fields
[ ] Parse legacy_state_action_attempt events
[~] Parse route-aware spawn/node/subagent profile fields
[~] Fail summaries when required producer-owned evidence is missing
[x] Split taskspace_control usage into native and action-contract lifecycle counts
```

## 4.6 `scripts/taskspace-benchmark/write-release-decision.ps1`

Required work:

```text
[~] Block release if exact payload scan is missing or hash/request mismatch
[~] Block release if active replacement proof is hash-only without exact scan
[ ] Block DeepSeek release-like claims if TaskSpace provider-cache-trace-summary.json is missing
[ ] Block DeepSeek release-like claims if request_2_plus_hit_rate < 0.95
[ ] Block DeepSeek release-like claims if trace_coverage < 0.99
[ ] Block DeepSeek release-like claims if cache_usage_missing_count > 0
[ ] Block DeepSeek release-like claims if native_tools_schema_hot_path_count > 0
[ ] Block DeepSeek release-like claims if tool_free_action_contract_count == 0
[x] Block release if BudgetQualityImpact has validation skip or score-ineligible solved
[ ] Block release if state_commit_displacement denominator lacks legacy attempts
[~] Block release if spawn/node profile trace has blocked budget events or unreviewed subagent results
[x] Block release-like claims if open_leaf_nodes > 0
[x] Require runtime bottleneck evidence when agent_walltime_ratio exceeds the configured threshold
[x] Block release if diagnostic sample set attempts release_pass
[x] Keep blocked_partial closeable=false
[~] Validate v005-non-agent-gates.json and v005-code-complete.json markers
```

---

# 5. Definition of done

The implementation is code-complete only when all are true:

Current status as of 2026-06-26: not code-complete.

Currently green local gates:

```text
cargo test -p codex-core --lib --quiet
  1985 passed; 0 failed; 4 ignored
cargo check -p codex-cli --locked
  passed
taskspace_action_contract / taskspace_control focused gates
  passed in the action-contract ABI repair run
```

Current code-complete blockers:

```text
Phase C producer-owned exact payload scan event is missing
Phase E independent legacy state action attempt denominator is missing
Phase G canonical v005-non-agent-gates.json builder is missing
post-ABI B-tier smoke business/cache/open-leaf/walltime evidence is missing
```

```text
cargo check -p codex-cli --locked passes
cargo test -p codex-core provider_request_budget passes
cargo test -p codex-core active_context_replacement passes
cargo test -p codex-core state_commit passes
cargo test -p codex-core budget --lib passes, including advisory spawn/node profile assertions
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-deepseek-cache-verifier.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1 passes
B-tier smoke business and cache gates pass with provider-cache-trace-summary.json archived
open_leaf_nodes = 0 before release-like closeout
agent_walltime_ratio <= configured threshold or a runtime bottleneck report blocks formal E3
v005-non-agent-gates.json exists and status=pass
v005-code-complete.json exists and status=pass
v005-user-approval.json exists and explicitly approves terminal-bench_E3-P0_3_5
start-gate/gate-decision.json full_e3_allowed=true
```

Then, and only then, run formal `terminal-bench_E3-P0_3_5`.

---

# 6. Recommended PR split

Use this split to keep reviews bounded:

1. `v005-profile-advisory-contract`: Phase A runtime/client advisory profile contract and tests.
2. `v005-request-phase`: Phase B phase attribution and cost summary fixtures.
3. `v005-payload-proof`: Phase C exact payload scan proof and release gate updates.
4. `v005-quality-impact`: Phase D quality impact events and scoring blockers.
5. `v005-state-commit-displacement`: Phase E legacy denominator fix.
6. `v005-spawn-profile`: Phase F route-aware spawn/node profile observability and subagent result quality gates.
7. `v005-gates`: Phase G non-agent gates, release-decision fixtures, start-gate fixtures.
8. `v005-diagnostic`: Phase H targeted diagnostic evidence only; no release close.

Do not combine Phase H with implementation PRs. Diagnostic evidence must be produced from a clean code-complete commit.
