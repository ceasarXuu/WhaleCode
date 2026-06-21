# Subagent VS Review: Phase A Active Budget

- Created: 2026-06-22T01:23:03.9053526+08:00
- Updated: 2026-06-22T02:32:00+08:00
- Task: Complete `docs/v0.0.5/build-R2/01-phase-a-active-budget.md` and run subagent-vs-review.
- Report path: `vs_review/2026-06-22-phase-a-active-budget-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Post-implementation code review

### Review Input

#### Objective
Complete Phase A: replace fixed TaskSpace budget constants with a typed route-aware active budget contract, wire it through provider request dispatch, node/spawn budgets, legacy budget reporting, and cost instrumentation, then validate with the specified Phase A commands.

#### Review Target
Code implementation, test strategy, and observability/logging for Phase A active budget.

#### Target Locations
- `docs/v0.0.5/build-R2/01-phase-a-active-budget.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
- Commands:
  - `cargo test -p codex-core taskspace_active_budget --locked`
  - `cargo test -p codex-core provider_request_budget --locked`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1`
  - `cargo check -p codex-cli --locked`

#### Change Introduction
The implementation adds `TaskSpaceActiveBudgetV1`, route modes, budget state/counters/violations, default route budgets, active budget activation in Experiment mode, provider request snapshots sourced from active budget, snapshot-derived client dispatch limits with per-node budget enforcement and bounded budget recovery, route-aware node/spawn trace limits, active budget tags in provider/spawn/cost artifacts, and tests for Phase A routes and provider budget behavior.

#### Risk Focus
- Active budget may be present but not actually authoritative for all Phase A budget consumers.
- Runtime counters may drift from provider/client events, especially per-node counts and restored sessions.
- Client dispatch might allow extra requests or block valid recovery/final-synthesis requests.
- Cost instrumentation may expose route fields without proving runtime-owned evidence.
- Tests may overfit to helpers rather than production call paths.
- New contract methods may exist but be disconnected from real gate paths.

#### Verification Status
- `cargo test -p codex-core taskspace_active_budget --locked`: passed, 4 tests.
- `cargo test -p codex-core provider_request_budget --locked`: passed, 9 tests.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1`: passed.
- `cargo check -p codex-cli --locked`: passed.
- Focused post-review fix validation:
  - `cargo test -p codex-core taskspace_active_budget --locked`: passed, 8 tests.
  - `cargo test -p codex-core provider_budget_pressure --locked`: passed, 5 tests.
- `pwsh` was not available in PATH; Windows PowerShell ran the same script successfully.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs/observability, and evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Phase A changes runtime state, client dispatch, counters, and existing gate behavior. | correctness, state flow, hidden edge cases |
| architecture-adversary | The change introduces a budget contract that should become the source of truth across modules. | boundaries, abstraction, long-term maintainability |
| test-validity-adversary | The task relies on targeted tests and instrumentation selftests to prove the contract. | regression coverage, overfit tests, weak assertions |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb35-b690-7c40-b0f4-e0273ad60ee3` / Ampere | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| architecture-adversary | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb35-f8a3-7e10-98db-a98c9c5e75fd` / Fermat | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb36-3e2d-77c1-82c0-68716675b291` / Godel | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Ampere

##### Summary
The implementation is not ready for Phase A closure. The typed contract exists and dispatch uses snapshot limits, but restored sessions reset budget state, spawn budget is enforced as concurrent leases rather than cumulative calls, and route-aware pressure/recovery logic still uses a fixed default constant.

##### Blocking Findings
- Restored sessions reset active budget counters, reopening provider budgets. Evidence: `restore_snapshot()` restores traces/tasks/maps but not `active_budget`, `budget_counters`, `budget_state`, or violations before `ensure_default_active_budget()`; `provider_request_budget_snapshot()` then reads zeroed counters.
- Spawn budget is not cumulative and can be bypassed by completing/releasing subagent leases. Evidence: `prepare_spawn_assignment()` derives `spawn_count_before` from current subagent leases and overwrites `budget_counters.spawn_agent_call_count` from that lease count.
- Route-aware provider pressure still uses a fixed default budget. Evidence: `provider_request_budget_pressure_active()` compares against `DEFAULT_PROVIDER_REQUEST_BUDGET_MAX`; tool blocking and forced inspect convergence use that helper.

##### Non-blocking Risks
- `set_mode_for_session()` discards the `active_budget` trace emitted by default activation.
- Per-node request accounting writes `snapshot.node_request_count + 1` for every started event in a drained batch.
- Legacy action budget is reported but not enforced.

##### Required Fixes
- Persist active budget state in snapshot schema or reconstruct counters from restored `taskspace_trace_events` during `restore_snapshot()`.
- Enforce `max_spawn_agent_calls` against cumulative `budget_counters.spawn_agent_call_count`.
- Replace provider pressure helper with a route-aware helper using active budget max.
- Ensure budget activation emits a replayable event when Experiment mode starts.

##### Missing Tests
- Snapshot round-trip preserving rollout/per-node counts.
- Spawn lifecycle cumulative budget test.
- Thin route pressure test.
- Retry/multiple-start batch counter test.
- Legacy budget enforcement or explicit scoped test proving Phase A only reports legacy actions.

##### Missing Logs / Observability
- No durable activation event on Experiment mode activation.
- No restored-budget trace for reconstructed counters.
- No violation trace for cumulative spawn overrun.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - restore, provider snapshot, spawn, and pressure helper paths.
- `third_party/codex-cli/codex-rs/core/src/client.rs` - provider retry/dispatch loop relevant to drained event batches.

#### architecture-adversary / Fermat

##### Summary
Phase A is not complete as an architecture contract. The implementation adds the right type shape and snapshot-derived client limits, but source-of-truth and gate-connection gaps allow drift and keep old fixed-budget behavior alive.

##### Blocking Findings
- Per-node provider request accounting undercounts multiple dispatch attempts in one turn. Evidence: client retry paths can produce multiple started events before runtime drains them; runtime records each started event as `snapshot.node_request_count + 1`.
- Route-aware budget is not the source of truth for existing provider pressure gates. Evidence: fixed `DEFAULT_PROVIDER_REQUEST_BUDGET_MAX` remains in pressure helper and messages.
- New runtime gate contract is mostly decorative. Evidence: `gate_provider_request_pre_dispatch`, `gate_create_node_budget`, and `gate_spawn_budget` have no production call sites; create/spawn checks are duplicated inline.

##### Non-blocking Risks
- Runtime and client budget-state threshold functions are duplicated.
- Legacy state action budget is reported but not clearly enforced.
- `route_mode` parser is unused, so route selection is not externally configurable yet.

##### Required Fixes
- Increment per-node count cumulatively within drained provider event batches.
- Replace provider pressure gates/messages with active budget max values.
- Route create_node, spawn_agent, and provider pre-dispatch decisions through typed gate methods, or remove unused facade and make real enforcement surfaces explicit.
- Emit or persist default active budget activation trace.

##### Missing Tests
- Retry/fallback test with two started events from one snapshot.
- Thin-route pressure test using max 4.
- Production-path tests proving create/spawn use route-aware gate behavior.
- Active-budget activation observability test.

##### Missing Logs / Observability
- Default active budget activation event is dropped.
- Provider budget events log snapshot node count instead of updated per-event value.
- Node/spawn blocked events lack budget state or violation id.

##### Evidence
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - active budget contract, gates, create/spawn, provider event recording, pressure helper.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs` - provider budget snapshot and event drain path.
- `third_party/codex-cli/codex-rs/core/src/client.rs` - retry and provider dispatch loop.

#### test-validity-adversary / Godel

##### Summary
Phase A is partially implemented, but claimed verification is not sufficient. Several tests validate helper methods or synthetic traces while real runtime paths drop active-budget activation evidence or do not consume all required budget dimensions.

##### Blocking Findings
- Active-budget activation trace is dropped on the normal Experiment-mode path. Evidence: `set_mode()` calls default activation, returned event is ignored, and `set_mode_for_session()` returns no events.
- Projection budget is not actually route-aware despite being in the Phase A contract. Evidence: contract includes projection fields, but projection text hardcodes `mode: default_compact` and only reports estimated tokens.
- Required route-aware gate functions are mostly unproven production behavior. Evidence: gate methods are dead-code allowed while production provider/node/spawn paths enforce elsewhere.

##### Non-blocking Risks
- Default constants remain as constants, not compatibility aliases to canonical budget function.
- PowerShell selftest uses synthetic trace events, proving parsing but not real TaskSpace emission.

##### Required Fixes
- Return and emit active-budget activation event from `set_mode_for_session()` or another session-visible path.
- Make projection consume active budget: route mode, token counters, max projection limits, and observable behavior.
- Wire typed gate methods into production paths or remove facade and test actual production enforcement.

##### Missing Tests
- Integration test for Experiment mode active-budget trace event.
- Integration test for session snapshot to provider context to active-budget artifacts.
- Thin route real `prepare_spawn_assignment()` negative test.
- Projection budget route/limit test.
- Real-trace cost instrumentation fixture.

##### Missing Logs / Observability
- No emitted activation artifact on normal mode entry.
- No projection budget event/summary for projection token counters.
- Cost instrumentation does not extract `active_budget` events directly.

##### Evidence
- `docs/v0.0.5/build-R2/01-phase-a-active-budget.md` - Phase A contract.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - activation, projection, gate methods.
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1` - synthetic instrumentation fixture.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Ampere | Restored sessions reset active budget counters. | blocking | accept | Restore path did not reconstruct counters from restored trace events. | Added trace replay reconstruction for active budget route/profile and provider/spawn/node/legacy/projection counters; restore avoids writing a new default activation trace when restored active-budget evidence exists. | Closure re-review required. |
| Ampere | Spawn budget is concurrent, not cumulative. | blocking | accept | Spawn path used active subagent lease count. | `prepare_spawn_assignment()` now gates and increments cumulative `budget_counters.spawn_agent_call_count`; lifecycle test proves released leases still count against the route budget. | Closure re-review required. |
| Ampere | Route-aware provider pressure uses fixed default budget. | blocking | accept | Pressure helper read default constant. | Provider pressure now accepts the active max from route budget snapshots; thin route test proves 3/4 requests enters pressure. | Closure re-review required. |
| Fermat | Per-node provider event batches undercount multiple starts. | blocking | accept | Runtime used `snapshot.node_request_count + 1` per started event. | Provider trace recording now increments local per-node counts cumulatively across drained started events; new test covers two started events from one snapshot. | Closure re-review required. |
| Fermat | New gate contract not connected to production paths. | blocking | accept | Gate methods existed without production call sites. | `create_node_for_main_with_kind()` and `prepare_spawn_assignment()` now call typed gate helpers; blocked traces include budget state and gate reason. Provider dispatch already consumes the runtime snapshot contract. | Closure re-review required. |
| Godel | Active-budget activation trace is dropped. | blocking | accept | `set_mode_for_session()` returned no activation events. | `set_mode_for_session()` now returns activation trace events emitted during Experiment mode activation. Existing session test asserts the returned `active_budget` event. | Closure re-review required. |
| Godel | Projection budget is not route-aware/consumed. | blocking | accept | Projection mode remained hardcoded and counters were unused. | Active projection now consumes active budget fields, reports route/max projection/input limits, updates projection counters, and emits `projection_budget` trace events. | Closure re-review required. |
| Godel | Tests overfocus helper/synthetic paths. | blocking | accept | Missing production-path and restored-counter tests. | Added tests for restored counters, production spawn cumulative blocking, thin pressure, event-batch counting, projection budget consumption, and activation event emission. | Closure re-review required. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending Round 2
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Final Conclusion

Round 1 found accepted blocking issues. Round 2 plus the focused Linnaeus re-check verified the accepted blockers are fixed. Phase A active budget closure passed with non-blocking follow-ups documented.

## Round 2: Closure Re-review

### Closure Input

#### Objective
Verify that the accepted Round 1 blocking findings are fixed without introducing new Phase A blockers.

#### Fix Summary
- Reconstructed active budget route/profile and counters from restored `taskspace_trace_events`.
- Avoided default-budget trace overwrite during snapshot restore when restored active-budget evidence exists.
- Returned `active_budget` activation events from `set_mode_for_session()`.
- Made provider pressure route-aware by passing active max request limits.
- Counted per-node provider started events cumulatively across drained batches.
- Enforced spawn budget cumulatively through `gate_spawn_budget()` in production spawn path.
- Routed create-node budget checks through `gate_create_node_budget()`.
- Made active projection consume active budget route/max fields, update projection counters, and emit `projection_budget` traces.
- Extended cost instrumentation to extract `active_budget` events and prefer runtime-owned active budget evidence.

#### Current Verification
- `cargo test -p codex-core taskspace_active_budget --locked`: passed, 8 tests.
- `cargo test -p codex-core provider_request_budget --locked`: passed, 9 tests.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1`: passed.
- `cargo check -p codex-cli --locked`: passed.

#### Re-review Focus
- Confirm restored budget replay cannot reset active route/counters.
- Confirm thin/default budget values are authoritative in provider pressure, projection, node/spawn gates, and cost artifacts.
- Confirm production spawn/create-node paths use typed gates, not disconnected helper facades.
- Confirm tests exercise production paths enough for Phase A.

### Closure Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-closure | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb4d-c5c7-71a3-9029-7030cf8827ae` / Linnaeus | spawn_agent result in current Codex thread | no | Round 2 Closure Input | main-agent reasoning and drafts | yes |
| architecture-closure | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb4d-da2a-7c63-972e-8b7c20c39e84` / Euler | spawn_agent result in current Codex thread | no | Round 2 Closure Input | main-agent reasoning and drafts | yes |
| test-observability-closure | `multi_agent_v1.spawn_agent` explorer, model `gpt-5.5`, reasoning `low` | `019eeb4d-f00a-7491-8dd2-2712fc1bf409` / Poincare | spawn_agent result in current Codex thread | no | Round 2 Closure Input | main-agent reasoning and drafts | yes |

### Closure Reviewer Outputs

#### implementation-closure / Linnaeus

##### Summary
Initial closure did not pass for implementation correctness because restored sessions reconstructed `budget_counters.rollout_model_request_count` but did not synchronize the legacy `provider_request_count` field that still feeds ordinary tool pressure gates.

##### Blocking Finding
- Restored sessions did not reconstruct the provider-pressure counter used by tool gates. Evidence: `reconstruct_budget_state_from_trace_events()` rebuilt budget counters, while `prepare_main_tool_call()` still read `self.provider_request_count`.

##### Fix Applied
- `reconstruct_budget_state_from_trace_events()` now sets `self.provider_request_count = self.budget_counters.rollout_model_request_count`.
- `taskspace_active_budget_restore_reconstructs_provider_counters` now restores a thin route at `3/4`, then asserts `prepare_main_tool_call()` blocks another inspect probe with `provider_request_budget_pressure_requires_inspect_node_transition` and `3/4 used`.
- `inspect_evidence_pressure_blocks_probe_before_provider_budget_pressure` now uses provider count `7`, staying below the default active budget provider-pressure threshold so it continues to test inspect-evidence pressure.

##### Focused Re-check
Linnaeus re-checked the applied fix and reported closure passed for the prior blocker. No remaining blocker for this focused issue.

#### architecture-closure / Euler

##### Summary
Closure passed for architecture-contract focus. No blocking issue found.

##### Non-blocking Risks
- Old default constants still exist as literal fallback constants rather than true aliases to `taskspace_active_budget_for_route`.
- Restore without prior `active_budget` trace defaults to `default_compact`; old incomplete histories cannot recover a non-default route without activation trace evidence.

#### test-observability-closure / Poincare

##### Summary
Closure passed for test-validity and observability focus. No blocking issue found.

##### Non-blocking Risks
- `request_reborn()` records activation internally but may omit live `active_budget` event emission on that path.
- The PowerShell cost selftest remains synthetic rather than an actual benchmark-run fixture.
- Downstream correlation could benefit from a stable violation id in blocked spawn/node traces, beyond `budget_gate_reason`.

### Round 2 Main Agent Triage

| Reviewer | Finding | Severity | Decision | Action Taken | Status |
|---|---|---|---|---|---|
| Linnaeus | Restored provider-pressure gate used stale `provider_request_count`. | blocking | accept | Synchronized `provider_request_count` from replayed rollout counter and added restored thin-route pressure gate regression. | fixed; focused re-check passed |
| Euler | Old default constants remain literal fallback constants. | non-blocking | defer | Confined to absent-budget fallback paths; Phase A active paths use active budget. | documented |
| Euler | Restore without active-budget trace defaults to compact route. | non-blocking | defer | Intended compatibility for old/incomplete histories. | documented |
| Poincare | `request_reborn()` live activation event is weaker than `set_mode_for_session()`. | non-blocking | defer | Persisted trace remains replayable; follow-up can improve live event emission. | documented |
| Poincare | Cost instrumentation selftest is synthetic. | non-blocking | defer | Current fixture validates runtime-shaped events; real benchmark fixture is follow-up. | documented |
| Poincare | Blocked spawn/node traces lack stable violation id. | non-blocking | defer | `budget_gate_reason` is sufficient for Phase A; stable ids can be added later if tooling needs correlation. | documented |

### Round 2 Closure Status

- Blocking findings found: yes, one focused restore-pressure issue.
- Accepted blocking findings fixed: yes.
- Blocking re-review completed: yes.
- Blocking re-review passed: yes.
- Deferred findings documented: yes.
- Allowed to proceed: yes.

