# v0.0.5 Completion Engineering Playbook: Module Checklist and Closeout

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## 4.1 `core/src/action_map/runtime.rs`

Required work:

```text
[ ] Add TaskSpaceRouteMode
[ ] Add TaskSpaceActiveBudgetV1 as advisory complexity profile
[ ] Add TaskSpaceBudgetCounters
[ ] Add TaskSpaceBudgetState
[ ] Add TaskSpaceProviderRequestPhase
[ ] Add BudgetQualityImpactV1
[ ] Add LegacyStateActionAttemptV1
[ ] Replace fixed budget constants with route-aware advisory profile
[ ] Extend provider_request_budget_snapshot
[ ] Add next_provider_request_phase
[ ] Add record_budget_quality_impact
[ ] Add record_legacy_state_action_attempt
[ ] Fix state_commit_displacement denominator
[ ] Add spawn/node profile observability and subagent result review quality gates
```

## 4.2 `core/src/client.rs`

Required work:

```text
[ ] Replace count/max-only ProviderRequestBudgetContext with ProviderRequestBudgetLimits
[ ] Record rollout request profile hints without blocking dispatch
[ ] Record per-node request profile hints without forcing recovery
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
[ ] Remove hard-stop BudgetQualityImpact producer; keep legacy hard-stop regression detection
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
[ ] Parse route-aware spawn/node/subagent profile fields
[ ] Fail summaries when required producer-owned evidence is missing
```

## 4.6 `scripts/taskspace-benchmark/write-release-decision.ps1`

Required work:

```text
[ ] Block release if exact payload scan is missing or hash/request mismatch
[ ] Block release if active replacement proof is hash-only without exact scan
[ ] Block DeepSeek release-like claims if TaskSpace provider-cache-trace-summary.json is missing
[ ] Block DeepSeek release-like claims if request_2_plus_hit_rate < 0.95
[ ] Block DeepSeek release-like claims if trace_coverage < 0.99
[ ] Block DeepSeek release-like claims if cache_usage_missing_count > 0
[ ] Block DeepSeek release-like claims if native_tools_schema_hot_path_count > 0
[ ] Block DeepSeek release-like claims if tool_free_action_contract_count == 0
[ ] Block release if BudgetQualityImpact has validation skip or score-ineligible solved
[ ] Block release if state_commit_displacement denominator lacks legacy attempts
[ ] Block release if spawn/node profile trace has blocked budget events or unreviewed subagent results
[ ] Block release-like claims if open_leaf_nodes > 0
[ ] Require runtime bottleneck evidence when agent_walltime_ratio exceeds the configured threshold
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
