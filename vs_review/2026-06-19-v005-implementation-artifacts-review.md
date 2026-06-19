# Subagent VS Review: v0.0.5 implementation artifacts

- Created: 2026-06-19T15:41:54.9106183+08:00
- Updated: 2026-06-19T15:52:00+08:00
- Task: 对 v0.0.5 近期工程实现执行实现级对抗性审查，确认真实 E3 前 producer-owned artifacts、runtime budget quality、active payload scan 和 release/start gates 不会误导判断。
- Report path: `vs_review/2026-06-19-v005-implementation-artifacts-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: Implementation And Test Validity

### Review Input

#### Objective
Challenge whether the current v0.0.5 implementation is ready to proceed toward the requested low-cost `terminal-bench_E3-P0_3_1` diagnostic after code completion and review. The review must not run real E3 or call real agent benchmark execution.

#### Review Target
Recent implementation around provider request budget lifecycle, runtime budget quality impact events, active provider payload scan, cost instrumentation artifact production, release-decision gates, and start-gate marker controls.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/client_tests.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `docs/v0.0.5/18-unfinished-work-engineering-design.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`

#### Change Introduction
The implementation now records provider request budget events from provider dispatch, emits paired budget quality impact trace events, scans serialized provider payloads for active TaskSpace replacement proof before hash-only fallback, extracts runtime trace events into release-required artifacts, and keeps release/start gate tests passing.

#### Risk Focus
- Runtime events may still be self-reported or script-derived rather than producer-owned.
- Active payload scan may produce false positives or scan the wrong payload.
- Budget hard stop may reduce cost by making samples unsolved while still allowing release pass.
- Request phase attribution, state_commit displacement, and spawn/node budget summaries may be too weak.
- Tests may only cover happy fixtures and miss mismatched hash, diagnostic-only, blocked_by_budget, or missing-marker cases.

#### Verification Status
- `cargo fmt` ran from `third_party/codex-cli/codex-rs` with existing nightly-only rustfmt warnings.
- `cargo test -p codex-core provider_request_budget -- --nocapture` passed.
- `cargo test -p codex-core provider_payload_scan -- --nocapture` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1 -RunRoot target\v005-cost-instrumentation-release-artifacts` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-release-decision-selftest-release-artifact-producers` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-e3-start-gate-selftest-release-artifact-producers` passed.
- Real E3 has not been run in this implementation pass.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Do not run true E3 and do not call real agent benchmark execution.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary / architecture-adversary | Recent changes span Rust provider dispatch, ActionMap runtime, session mapping, and PowerShell artifact producers. | Runtime ownership, producer truth, execution control |
| test-validity-adversary | The release decision depends on many synthetic fixtures and negative gates. | Self-deceptive tests, diagnostic/release confusion, fake pass prevention |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary / architecture-adversary | `multi_agent_v1.spawn_agent` explorer | `019eded3-e2b2-78a2-884a-a095120d293b` / Mendel | spawn result in current Codex thread | no | Round 1 Review Input adapted to implementation and architecture focus | main-agent history, reasoning, drafts, conclusions, real E3 execution | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019eded4-188b-7f43-9b7e-1dcca46d4e43` / Franklin | spawn result in current Codex thread | no | Round 1 Review Input adapted to test validity focus | main-agent history, reasoning, drafts, conclusions, real E3 execution | yes |

### Reviewer Outputs

#### implementation-adversary / architecture-adversary

##### Summary
v0.0.5 has real movement toward runtime-owned provider request and budget artifacts, but it is not release-grade. The biggest remaining issue is that exact payload scan and provider request coverage can still pass on weak derived/script artifacts rather than a complete producer-owned lifecycle contract.

##### Blocking Findings
- Provider request IDs are local debug counters, not stable lifecycle IDs. `client.rs` uses `provider-request-{n}`, which lacks session, turn, logical request, and attempt identity.
- Exact payload scan is not exact enough to prove protected item preservation or raw-output absence. It string-searches markers, treats `protected_items_present` as equivalent to active projection marker presence, and hardcodes `large_raw_output_tokens = 0`.
- Provider hook coverage can report 100% with no independent denominator. `cost-instrumentation.ps1` sets coverage to 100 whenever at least one provider event exists, and release accepts `>=99`.
- WebSocket warmup and SSE fixture paths bypass provider budget lifecycle events. Warmup uses disabled budget dispatch, and fixture returns before dispatch.
- Release still accepts instrumentation-derived `taskspace-provider-request-budget-event-v1` as provider request proof; this is better than nothing but not a separate canonical lifecycle artifact.

##### Non-blocking Risks
- Active replacement is marker-triggered; without an active projection item, old provider-visible history remains unchanged.
- All model sampling requests are labeled `model_sampling`, so recovery and validation phases are not yet distinguished.
- Release active replacement pass trusts fields derived from the weak scan chain.

##### Required Fixes
- Replace local `provider-request-{n}` with session/turn/logical-request/attempt IDs.
- Add canonical provider lifecycle events emitted at provider dispatch rather than reconstructed from ActionMap trace tags.
- Make coverage use an expected-request denominator from rollout/model usage or provider transport hooks.
- Implement exact scan over structured request input with required protected item IDs/counts and raw-output detection.
- Record warmup/startup/fixture request phases explicitly or exclude them with audited denominator logic.

##### Missing Tests
- Active marker present but protected items missing must fail.
- Large raw output replay without banned TaskSpace strings must fail.
- Two provider attempts with only one event must fail coverage.
- Unauthorized retry must preserve parent/logical request ID and attempt ID.
- WebSocket prewarm emits or is explicitly excluded from lifecycle coverage.

##### Missing Logs / Observability
- `provider_request_context_missing_reason`.
- `parent_request_id`, `attempt_seq`, `logical_request_id`.
- `payload_scan_rule_version`, protected item expected/found counts, large raw replay detected tokens.
- Separate request phases for warmup, budget recovery, validation recovery, retry/fallback.

#### test-validity-adversary

##### Summary
Release/start gate coverage is stronger than before, especially around E3 identity, suite manifest, pair evidence, stale/spoofed/mismatched markers, diagnostic comparison, and `reported_evidence_level == E3`. Two test-self-deception gaps remain.

##### Blocking Findings
- `release_pass` trusts `budget_induced_quality_impact_summary.json` instead of recomputing blocker counts from `budget-quality-impact-events.jsonl`. If events contain `blocked_by_budget` but summary says zero, release can pass.
- Provider payload scan is marker heuristic, not semantic scan. Tests only cover active marker pass and legacy marker fail, not missing protected items, raw output, forged markers, or marker in unrelated strings.

##### Non-blocking Risks
- `cost-instrumentation` matches budget actions to quality events by trace id only, without checking request/status/node consistency.
- There is no direct release negative fixture for `reported_evidence_level = diagnostic-only`, although exact `E3` matching should block it.

##### Required Fixes
- Release decision must recompute budget quality blocker counts from events and fail on summary/event mismatch.
- Provider payload scan must use real projection schema/protected item inventory and real raw-output detection.

##### Missing Tests
- Release fixture where event contains `final_classification=blocked_by_budget` but summary says zero.
- Release fixture where quality event has missing/mismatched `provider_request_budget_trace_event_id`.
- Payload scan active marker only but protected items missing.
- Payload scan active marker plus large raw output.
- Release fixture for `reported_evidence_level=diagnostic-only`.

##### Missing Logs / Observability
- Release decision should emit derived-vs-summary budget quality counts and mismatch fields.
- Payload scan artifact should log protected item IDs expected/found/missing.
- Cost instrumentation summary should include unmatched quality event count and mismatched request/status/node count.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-adversary / architecture-adversary | Provider request IDs are local counters, not stable lifecycle IDs. | blocking | accept | Current IDs are `provider-request-{n}` and do not encode turn/logical attempt identity. | Recorded as blocking implementation gap. | Replace with stable lifecycle IDs before release/E3 closure. |
| implementation-adversary / architecture-adversary | Exact payload scan is marker heuristic and does not prove protected items/raw output. | blocking | accept | Current scan equates active marker with protected item presence and hardcodes raw output tokens to 0. | Recorded as blocking implementation gap. | Implement structured scan with expected/found/missing protected item evidence and raw replay detection. |
| implementation-adversary / architecture-adversary | Provider hook coverage has no denominator. | blocking | accept | Coverage is derived from observed events only. | Recorded as blocking implementation gap. | Add denominator or fail coverage when denominator is unavailable. |
| implementation-adversary / architecture-adversary | Warmup/fixture paths bypass lifecycle events. | major | accept | Warmup and fixture paths are not part of normal scoring, but design requires explicit attribution or exclusion. | Recorded as gate-hardening item. | Emit or explicitly exclude with audited reason. |
| implementation-adversary / architecture-adversary | Release accepts instrumentation-derived provider request events as provider proof. | major | accept | Instrumentation is consuming runtime trace tags; not a separate lifecycle schema. | Recorded as producer ownership gap. | Split canonical provider lifecycle event or tighten provenance fields. |
| test-validity-adversary | Release trusts budget quality summary without recomputing from events. | blocking | accept | Event/summary mismatch can create false pass. | Not fixed in report turn. | Recompute counts in release-decision and add mismatch fixture. |
| test-validity-adversary | Payload scan tests miss marker-only false positives. | blocking | accept | Existing tests do not cover protected item missing/raw output cases. | Not fixed in report turn. | Add negative scan fixtures with semantic checks. |
| test-validity-adversary | Budget action/quality match checks only trace id. | major | accept | Request/status/node mismatch can evade summary. | Not fixed in report turn. | Add matching consistency checks in instrumentation summary. |
| test-validity-adversary | No direct diagnostic-only release negative fixture. | minor | accept | Exact `E3` match likely blocks it, but a direct regression test is cheap. | Not fixed in report turn. | Add release fixture. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - none yet; required after remediation
- Blocking re-review launch records:
  - none yet
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no for true E3; yes for blocker remediation.

## Final Conclusion

The implementation is not ready for `terminal-bench_E3-P0_3_1`. Real E3 remains blocked until accepted findings are fixed and a closure review passes.
