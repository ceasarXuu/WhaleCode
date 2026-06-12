# Subagent VS Review: TaskSpace Phase 5 Preimplementation

- Created: 2026-06-12T17:32:54+08:00
- Updated: 2026-06-12T17:38:20+08:00
- Report schema: adversarial-v1
- Task: Continue TaskSpace v0.0.4 Phase 5 implementation planning and execution.
- Report path: `vs_review/2026-06-12-taskspace-phase5-preimplementation-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Phase 5 Contract And ROI Review

### Review Input

#### Objective
Challenge whether TaskSpace v0.0.4 Phase 5 can be implemented as a bounded, auditable subagent evidence contract with viewer v2 and thin-mode report-only warnings, without over-role-izing the system or creating self-deceptive ROI metrics.

#### Review Target
Preimplementation architecture and validation review for Phase 5.

#### Target Locations
- `docs/plans/2026-06-11-taskspace-0.0.4-engineering-implementation-design.md`
- `docs/plans/taskspace_0_0_4_design_docs/09-subagent-contract-and-roi.md`
- `docs/plans/taskspace_0_0_4_design_docs/10-graph-health-and-viewer.md`
- `docs/plans/taskspace_0_0_4_design_docs/12-benchmark-and-release-plan.md`
- `third_party/codex-cli/codex-rs/core/src/action_map/`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/test-action-map-graph-health.ps1`

#### Change Introduction
Phase 5 is intended to add `record_subagent_plan`, inject bounded subagent assignment context, require main-agent validity/adoption decisions for subagent results, compute subagent decision yield in graph health, emit thin-mode recommendations without behavior switching, and expose minimal viewer v2 health/adoption/ledger UI.

#### Risk Focus
- The subagent contract may become a role taxonomy or prompt-only convention instead of a runtime-verifiable artifact contract.
- Subagent ROI metrics may be gameable if they count spawn/result volume rather than adopted evidence that supports decisions.
- Thin mode may accidentally become a behavior switch instead of report-only guidance.
- Viewer v2 may display fields without preserving refresh/selection/transform state or without enough evidence for E3 judgment.
- Runtime gates may reject useful parallel work or allow unbounded subagent work because readiness, lease, plan, and adoption states are not aligned.

#### User-Perspective Review Focus
- A future maintainer should be able to inspect one TaskSpace run and tell whether a subagent contributed evidence or noise.
- A CLI/viewer user should see warnings that explain over-fragmentation or thin-mode mismatch without needing hidden context.
- A future implementing agent should be able to map the docs to concrete code changes and validation commands.

#### Assumptions To Attack
- Existing action-map snapshots contain enough structure to compute subagent ROI without weakening runtime contracts.
- `record_subagent_plan` can be added without duplicating node assignment, lease, or result-adoption concepts.
- Thin-mode report-only classification can be represented in artifacts without changing task behavior.
- Viewer v2 can be minimal and still satisfy auditability.
- Regression tests can prove unused subagent results, viewer refresh state, and thin-mode report-only warnings.

#### Adversarial Lenses
- architecture
- state
- failure
- maintenance
- testing
- observability
- user comprehension

#### Verification Status
- Phase 1-4 are already present on branch `whalecode-alpha`.
- Current HEAD: `24abcc89b Close TaskSpace finish contract review findings`.
- Worktree was clean before this review report was created.
- Phase 5 implementation has not yet been completed in this round.
- Known validation targets for later execution: Rust action-map runtime tests, graph-health PowerShell tests, and viewer smoke/regression checks.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Attack the Phase 5 plan and likely implementation shape; do not assume the current plan is correct.
- Focus on concrete blockers, major risks, missing tests, missing logs, and user-facing comprehension failures.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension up to 10 minutes if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | Phase 5 touches action-map runtime contracts, graph-health artifacts, viewer state, and long-term subagent abstraction boundaries. | module boundaries, state contracts, maintainability, testability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | multi_agent_v1.spawn_agent | 019ebb2e-3f04-7103-985f-867584536d1d | spawn_agent result in current Codex thread | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| architecture-adversary-r1 | architecture-adversary | 1 | 019ebb2e-3f04-7103-985f-867584536d1d | 15 minutes max, completed early | completed | reviewer returned formal output | completed |

### Reviewer Outputs

#### architecture-adversary-r1

##### Summary
Phase 5 is implementable, but not yet bounded enough as a preimplementation contract. The current design correctly avoids new subagent roles and already has runtime result validity/adoption primitives, but `record_subagent_plan` is still under-specified as an auditable runtime artifact. If implemented literally from the docs, Phase 5 can still become prompt-only orchestration plus gameable ROI counters.

##### Blocking Findings
- Missing first-class SubagentPlan lifecycle and join keys.
  - Broken assumption: `record_subagent_plan` can be added without duplicating assignment/lease/result concepts.
  - Failure scenario: implementation stores plan text in prompt/context, then `prepare_spawn_assignment` creates a lease and `record_child_result` records a result, but no durable `plan_id` links plan -> lease -> child thread -> result -> adoption.
  - Trigger condition: a maintainer inspects one run with multiple ready nodes or repeated spawns on similar node titles.
  - Impact: cannot prove whether a subagent fulfilled a bounded assignment or produced unrelated noise; ROI and viewer summaries become reconstructive guesses.
  - Proof needed: runtime schema with stable plan IDs and explicit foreign keys, plus tests proving spawn without prior plan is rejected and one plan can be traced through lease, `source_thread_id`, result, validity, and adoption.
  - Evidence: `09-subagent-contract-and-roi.md:19-32` lists fields but no plan ID/status/timestamps; `runtime.rs:3536-3630` creates lease/assignment without any plan lookup; `protocol.rs:2108-2182` exposes maps/nodes/leases/results but no subagent plan collection.
- ROI metric can still be self-deceptive.
  - Broken assumption: existing action-map snapshots contain enough structure to compute subagent ROI safely.
  - Failure scenario: a subagent result is marked accepted and adopted by a node or stale reference, then counted as "adopted" even though no decision/fact/hypothesis actually changed the main path.
  - Trigger condition: graph health computes `adopted_subagent_results` or `subagent_decision_yield` from spawn/result counts or generic adoption state.
  - Impact: noisy delegation can look productive; Phase 5 would repeat the E3 failure mode it is meant to expose.
  - Proof needed: metric must count only subagent-sourced accepted results that are adopted by active decision/fact/hypothesis records and support a current decision; node-only adoption must not count toward decision yield.
  - Evidence: ROI is defined as `decisions_supported_by_subagent_results / spawn_count` in `09-subagent-contract-and-roi.md:61-71` and `10-graph-health-and-viewer.md:31-42`; current adoption state treats any adoption refs, including nodes, as `accepted_adopted` in `cognitive.rs:141-181`; `adopt_result_for_main` only requires one adoption reference in `runtime.rs:2431-2451`.

##### Non-blocking Risks
- Thin mode is a label without a classifier contract. The docs say report-only and no behavior switch, but they do not define classifier inputs, output schema, or where the recommendation lives.
  - Broken assumption: thin mode can be added as an informal report label.
  - Failure scenario: implementers either skip it or accidentally use it as behavior control because the artifact boundary is absent.
  - Trigger condition: Phase 5 adds thin-mode warning logic without a stable schema.
  - Impact: report-only promise becomes unverifiable.
  - Proof needed: classifier input/output schema and tests that spawn behavior is unchanged.
- Viewer v2 is already preserving refresh UI state reasonably well, but graph-health/audit readiness are absent from the snapshot contract.
  - Broken assumption: adding UI panels alone satisfies auditability.
  - Failure scenario: viewer shows adoption details but cannot explain graph-health warning provenance or audit readiness.
  - Trigger condition: viewer v2 derives state from incomplete snapshots.
  - Impact: users still cannot judge subagent value from one run.
  - Proof needed: snapshot-visible graph-health/adoption/ROI fields or a deterministic viewer-side derivation with tests.
- Existing graph-health script is topology-only and cannot validate Phase 5 ROI/thin-mode claims yet.
  - Broken assumption: current graph-health infrastructure is close enough.
  - Failure scenario: tests pass while subagent ROI and thin-mode warnings are missing.
  - Trigger condition: Phase 5 relies on existing graph-health test coverage.
  - Impact: self-deceptive validation.
  - Proof needed: explicit warnings and tests for unused subagent results, node-only adoption, invalid/questioned results, and report-only thin-mode mismatch.

##### User-Perspective Checks
- Usability: risk - a maintainer cannot yet inspect one run and answer "what did the subagent contribute?" without manually joining lease/result/adoption and inferring intent.
- Ease of use: risk - a viewer user can see validity/adoption details today, but not a clear "subagent evidence vs noise" summary or thin-mode mismatch explanation.
- Ease of understanding: risk - a future implementing agent can map the broad files to change, but the docs need a concrete `SubagentPlanV1` schema and exact counting rules before coding.

##### Required Fixes
- Define `SubagentPlanV1` as a runtime artifact with `plan_id`, `task_id`, `map_id`, `parent_node_id`, planned scope fields, status, `lease_id`, child thread ID, result IDs, and timestamps.
- Make `prepare_spawn_assignment` require an unused valid plan for the selected ready node, then bind the lease to `plan_id`.
- Define graph-health ROI rules so `subagent_decision_yield` counts adopted, accepted, subagent-sourced results supporting current decisions only.
- Define thin-mode recommendation as report-only artifact data, not prompt behavior: classifier inputs, output enum, warning code, and proof that spawn remains allowed.

##### Missing Tests
- Spawn without `record_subagent_plan` fails.
- Plan for node A cannot be consumed by spawn on node B.
- Unused accepted subagent result emits `subagent_no_adoption`.
- Node-only adoption does not increase `subagent_decision_yield`.
- Questioned/invalid subagent results never count as yield.
- Thin-mode recommendation with spawn emits warning but does not block spawn or alter task behavior.
- Viewer refresh retains expanded details and graph transform after graph-health/adoption panels are added.

##### Missing Logs / Observability
- Trace event for `record_subagent_plan`.
- Trace event or snapshot field linking `plan_id -> lease_id -> child_thread_id -> result_id`.
- Graph-health report source identity: snapshot/run ID, generated timestamp, schema version.
- Warning payloads with human-readable reason and exact evidence refs for thin-mode mismatch and subagent no-adoption.

##### Evidence
- `docs/plans/2026-06-11-taskspace-0.0.4-engineering-implementation-design.md:674-691`
- `docs/plans/taskspace_0_0_4_design_docs/09-subagent-contract-and-roi.md:19-32`
- `docs/plans/taskspace_0_0_4_design_docs/09-subagent-contract-and-roi.md:61-81`
- `docs/plans/taskspace_0_0_4_design_docs/10-graph-health-and-viewer.md:14-42`
- `docs/plans/taskspace_0_0_4_design_docs/10-graph-health-and-viewer.md:59-76`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3536-3630`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:3732-3811`
- `third_party/codex-cli/codex-rs/core/src/action_map/cognitive.rs:141-181`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs:2108-2182`
- `scripts/action-map-graph-health-lib.ps1:1-18`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs:331-470`

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| architecture-adversary | Missing first-class SubagentPlan lifecycle and join keys. | A prompt/context-only plan cannot durably link plan -> lease -> child thread -> result -> adoption. | blocking | accept | Phase 5 objective requires auditable spawn/result/adoption chain; reviewer cited current snapshot and assignment paths lacking a plan collection or join key. | Phase 5 implementation must introduce `SubagentPlanV1` as a first-class runtime/snapshot artifact, not a prompt-only field. | Implement schema, spawn gate, plan binding, prompt injection, trace/snapshot evidence, and runtime tests before marking Phase 5 complete. |
| architecture-adversary | ROI metric can still be self-deceptive. | Generic accepted/adopted state can count node-only or stale adoption without proof that subagent evidence supported a current decision. | blocking | accept | Phase 5 goal is ROI-observable bounded evidence, so graph health must distinguish decision-supported subagent evidence from generic adoption. | Phase 5 graph-health rules must count decision yield only from accepted subagent-sourced results adopted into current decision/fact/hypothesis evidence, excluding node-only/invalid/questioned results. | Add graph-health script/tests for unused result, node-only adoption, questioned/invalid exclusion, and decision-supported yield. |
| architecture-adversary | Thin mode is a label without a classifier contract. | Report-only thin mode may become unverifiable or accidentally behavior-changing without stable schema. | major | accept | The Phase 5 plan says report-only, but the reviewer identified missing classifier inputs/output location. | Treat thin-mode recommendation as explicit report artifact data with warning code and test that spawn behavior remains unchanged. | Add schema fields and tests during Phase 5 implementation. |
| architecture-adversary | Viewer v2 graph-health/audit readiness absent from snapshot contract. | UI panels cannot prove auditability if warning provenance and graph-health data are not available or deterministic. | major | accept | Viewer v2 deliverable requires health/adoption/ledger UI; reviewer found current snapshot lacks graph-health field. | Viewer v2 implementation must either expose graph-health/ROI data in snapshot or derive it deterministically with tests. | Add viewer update and smoke/regression checks after runtime/schema decisions. |
| architecture-adversary | Existing graph-health script is topology-only. | Phase 5 could pass existing tests while ROI/thin-mode warnings are absent. | major | accept | Reviewer cited current script output omissions and test coverage boundaries. | Extend graph-health library and tests to include schema version/source identity, subagent ROI, warnings, and thin-mode report-only fields. | Run PowerShell graph-health tests after implementation. |
| architecture-adversary | Required fix: Define `SubagentPlanV1`. | Missing runtime artifact lifecycle. | blocking | accept | Same as first blocking finding. | Added as Phase 5 implementation requirement in this report. | Implement before closure review. |
| architecture-adversary | Required fix: Make `prepare_spawn_assignment` require an unused valid plan. | Spawn can proceed without bounded plan. | blocking | accept | Same as first blocking finding. | Added as Phase 5 implementation requirement in this report. | Implement before closure review. |
| architecture-adversary | Required fix: Define graph-health ROI rules. | Yield metric can be self-deceptive. | blocking | accept | Same as second blocking finding. | Added as Phase 5 implementation requirement in this report. | Implement before closure review. |
| architecture-adversary | Required fix: Define thin-mode recommendation as report-only artifact data. | Thin-mode contract can drift into behavior. | major | accept | Same as thin-mode risk. | Added as Phase 5 implementation requirement in this report. | Implement and validate report-only behavior. |
| architecture-adversary | Missing tests list. | Without these tests, Phase 5 validation can be self-deceptive. | major | accept | Missing tests correspond directly to accepted blocking/major findings. | Added tests to Phase 5 acceptance checklist in this report. | Add Rust, PowerShell, and viewer regression tests during implementation. |
| architecture-adversary | Missing logs/observability list. | Without trace and warning payloads, failures cannot be diagnosed after the session. | major | accept | Missing logs correspond to AGENTS logging principle and Phase 5 auditability goal. | Added logging/observability requirements to Phase 5 acceptance checklist in this report. | Add trace/snapshot fields and graph-health warning evidence refs during implementation. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - required after Phase 5 fixes are implemented
- Blocking re-review launch records:
  - required after Phase 5 fixes are implemented
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: accepted blocking findings are preimplementation requirements and must be implemented plus re-reviewed before Phase 5 is complete
- Allowed to proceed: yes, for implementation only; no, for claiming Phase 5 completion

## Final Conclusion

Phase 5 may proceed into implementation, but it cannot be marked complete until the accepted blocking findings are fixed, validated, and re-reviewed by a fresh internal subagent closure round.

## Round 2: Phase 5 Implementation Closure Review

### Review Input

#### Objective
Review whether the implemented TaskSpace v0.0.4 Phase 5 changes close the accepted Round 1 blockers without introducing over-role-izing, self-deceptive ROI, or thin-mode behavior switches.

#### Review Target
Post-implementation closure review for SubagentPlanV1, graph-health ROI, viewer v2, observability export, and thin-mode report-only behavior.

#### Target Locations
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/action-map-observability-lib.ps1`
- `scripts/export-action-map-observability.ps1`
- `scripts/taskspace-benchmark/lib/graph-health.ps1`
- `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1`
- `scripts/test-action-map-graph-health.ps1`
- `scripts/test-action-map-observability-lib.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`

#### Change Introduction
Implementation added first-class `SubagentPlanV1` runtime/snapshot state, `taskspace_control(action=record_subagent_plan)`, spawn gating on an unused plan for the selected node, plan binding through lease/child/result/snapshot, graph-health v2 ROI and thin-mode warnings, observability export of `subagentPlans` and `subagentPlanId`, benchmark ROI updates, and viewer v2 subagent ROI/adoption panels.

#### Verification Status Sent To Reviewer
- `cargo check -p codex-core`: PASS
- `cargo test -p codex-core action_map::runtime::tests -- --nocapture`: PASS, 158 tests
- `cargo test -p codex-protocol action_map -- --nocapture`: PASS, 8 tests
- `cargo test -p codex-tui action_map_viewer -- --nocapture`: PASS, 2 tests
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-observability-lib.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| closure-adversary | multi_agent_v1.spawn_agent | 019ebb6b-15a8-7ad0-a90a-7fc43787043d | spawn_agent result in current Codex thread | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions beyond navigation packet | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| closure-adversary-r2 | closure-adversary | 1 | 019ebb6b-15a8-7ad0-a90a-7fc43787043d | 15 minutes max, completed early | completed | reviewer returned blocking closure findings | completed |

### Reviewer Outputs

#### closure-adversary-r2

##### Summary
Read-only closure review completed. The Rust SubagentPlan lifecycle is mostly present: plan record gates spawn, lease/result/snapshot carry `subagent_plan_id`, and legacy serde defaults avoid obvious phantom plan creation. The closure was not clean because ROI/reporting still had decision-yield correctness gaps and tests missed the real export shape.

##### Blocking Findings
- Benchmark and viewer ROI can count stale decision adoption as current decision yield.
  - Counterexample: accepted subagent result has `adoptedByDecisions=["deleted-decision"]`, but current ledger has no such decision and no `dependsOnResults` edge to the result.
  - Trigger: old snapshot/export or ledger rewrite leaves stale adoption refs.
  - Impact: `subagent_decision_yield` becomes positive and `subagent_no_decision_yield` is suppressed even though no current decision is supported.
  - Proof needed: tests where `adoptedByDecisions` references a missing or currently unrelated decision must yield 0 and warn.
  - Evidence: `scripts/taskspace-benchmark/lib/graph-health.ps1`, `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`.
- Graph-health v2 misses real exported result join keys.
  - Counterexample: exported node result has `resultId=result-1`, accepted, `subagentPlanId=subagent-plan-1`; current decision has `dependsOnResults=["result-1"]`.
  - Trigger: running v2 graph health on observability export rather than raw snapshot-shaped rows.
  - Impact: false negative ROI and noisy closure reports for valid subagent evidence.
  - Proof needed: integration test using exported observability shape, not only synthetic `id` rows.
  - Evidence: `scripts/action-map-observability-lib.ps1`, `scripts/export-action-map-observability.ps1`, `scripts/action-map-graph-health-lib.ps1`.

##### Non-blocking Risks
- `subagent_plan_links_lease_child_result_and_snapshot` does not restore the snapshot despite the test name.
- Viewer tests are still mostly string checks and do not execute ROI logic.
- Lease/result runtime events do not carry `subagent_plan_id`; snapshot export carries the join key, but event-only observability remains weaker.

##### Required Fixes
- Require `adoptedByDecisions` refs to match current ledger decisions, preferably also checking that the decision depends on the result.
- Teach v2 graph health to accept `resultId`/`result_id` everywhere it compares result IDs.
- Add tests for stale decision refs, exported observability shape, and viewer ROI guardrails.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| closure-adversary | Benchmark and viewer ROI can count stale decision adoption as current decision yield. | blocking | accept | ROI must represent current decision support, not stale adoption refs. | Updated graph-health v2, benchmark graph-health, and viewer ROI logic so `adoptedByDecisions` only counts when it references a current decision that also depends on the result. Added stale decision counterexamples to graph-health and benchmark tests. | Fresh closure re-review required. |
| closure-adversary | Graph-health v2 misses real exported result join keys. | blocking | accept | Observability export stores node results as `resultId`; v2 must support exported and snapshot-shaped result IDs. | Updated graph-health v2 and benchmark result-id helpers to support `id`, `resultId`, and `result_id`. Added exported observability-shape graph-health test. | Fresh closure re-review required. |
| closure-adversary | Snapshot restore roundtrip test gap. | non-blocking | defer | Existing runtime tests cover snapshot export and serde defaults; restore roundtrip is useful but not required for Phase 5 closure because serde restore path already maps `subagent_plans` and existing restore tests cover snapshot migration patterns. | No code change in this round. | Track as future hardening if snapshot restoration regressions appear. |
| closure-adversary | Viewer ROI execution test gap. | non-blocking | accept | Viewer logic was corrected, and static guardrails now assert the exact current-decision and exported-result-id expressions are present. Full JS execution harness is not currently part of the viewer test stack. | Added viewer static assertions for `resultId/result_id` and current decision matching. | Consider a JS DOM harness later if viewer logic grows. |
| closure-adversary | Event-only observability weaker than snapshot export. | non-blocking | defer | Phase 5 exit criterion can judge contribution from snapshot/export artifacts; event-only joins are a future observability improvement. | No runtime event payload change in this round. | Keep as future observability hardening. |

### Post-Fix Validation

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS
- `cargo test -p codex-tui action_map_viewer -- --nocapture`: PASS

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: no
- Blocking re-review passed: no
- Required next action: start a new fresh internal subagent closure review focused on the Round 2 fixes.

## Round 3: Focused ROI Closure Re-review

### Review Input

#### Objective
Verify whether the accepted Round 2 ROI and exported-result-id blockers are closed.

#### Review Target
Focused read-only re-review of graph-health v2, benchmark graph-health, and viewer ROI decision-yield logic.

#### Target Locations
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/test-action-map-graph-health.ps1`
- `scripts/taskspace-benchmark/lib/graph-health.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `vs_review/2026-06-12-taskspace-phase5-preimplementation-review.md`

#### Verification Status Sent To Reviewer
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS
- `cargo test -p codex-tui action_map_viewer -- --nocapture`: PASS

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| focused-roi-closure-adversary | multi_agent_v1.spawn_agent | 019ebb75-eb9d-7c32-a7ac-10c0fe789737 | spawn_agent result in current Codex thread | fork_context=false | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions beyond navigation packet | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| focused-roi-closure-adversary-r3 | focused-roi-closure-adversary | 1 | 019ebb75-eb9d-7c32-a7ac-10c0fe789737 | 15 minutes max, completed early | completed | reviewer returned blocking closure findings | completed |

### Reviewer Outputs

#### focused-roi-closure-adversary-r3

##### Summary
Read-only re-review completed. The Round 2 blockers were not fully closed. Benchmark and viewer logic looked corrected for the stale-decision counterexample, but `scripts/action-map-graph-health-lib.ps1` still had two blocking gaps.

##### Blocking Findings
- Stale `adoptedByDecisions` can still count as current decision yield in v2 graph health.
  - Counterexample: stale ref plus current unrelated decision dependency produced `Count=1`, `Yield=1`, no `subagent_no_decision_yield`.
  - Required fix: if `adoptedByDecisions` is present, do not use the fallback dependency-only path.
- Exported `resultId` rows undercount when more than one exported result supports decisions.
  - Counterexample: two accepted exported `resultId` rows yielded `Count=1`, `Yield=0.5`; expected `Count=2`, `Yield=1`.
  - Required fix: add decision-supporting result IDs to the set using normalized IDs, not only `id`/`Id`.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| focused-roi-closure-adversary | Stale `adoptedByDecisions` can still count through fallback. | blocking | accept | Explicit stale decision refs must invalidate decision support unless they match a current decision that depends on the result. | Updated `Test-TaskspaceResultSupportsDecision` to return false after explicit unmatched `adoptedByDecisions`; updated benchmark logic similarly. Strengthened graph-health test so current `decision-1.dependsOnResults=["result-1"]` still yields 0 when adoption cites `deleted-decision`. | Fresh closure re-review required. |
| focused-roi-closure-adversary | Exported `resultId` rows collapse in decision-support set. | blocking | accept | The set key must use the same normalized result ID that comparisons use. | Added `Get-TaskspaceResultId` and use it for both decision checks and decision-supporting result set/evidence refs. Strengthened exported-shape test to two `resultId` rows and two current decisions; expected count 2 and yield 1. | Fresh closure re-review required. |

### Post-Fix Validation

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-observability-lib.ps1`: PASS

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: no
- Blocking re-review passed: no
- Required next action: start a new fresh internal subagent closure review focused on Round 3 fixes.

## Round 4: Final Focused ROI Closure Review

### Review Input

#### Objective
Verify whether Round 3 accepted blocking findings are now closed in graph-health v2 and benchmark/viewer ROI.

#### Review Target
Final focused read-only re-review of graph-health v2 normalized result IDs and explicit stale decision adoption handling.

#### Target Locations
- `scripts/action-map-graph-health-lib.ps1`
- `scripts/test-action-map-graph-health.ps1`
- `scripts/taskspace-benchmark/lib/graph-health.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `third_party/codex-cli/codex-rs/tui/src/app/action_map_viewer.rs`
- `vs_review/2026-06-12-taskspace-phase5-preimplementation-review.md`

#### Verification Status Sent To Reviewer
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-observability-lib.ps1`: PASS

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| final-roi-closure-adversary | multi_agent_v1.spawn_agent | 019ebb7c-4f6b-7282-a38c-d40aec97c3b2 | spawn_agent result in current Codex thread | fork_context=false | Round 4 Review Input | main-agent history, reasoning, drafts, conclusions beyond navigation packet | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| final-roi-closure-adversary-r4 | final-roi-closure-adversary | 1 | 019ebb7c-4f6b-7282-a38c-d40aec97c3b2 | 15 minutes max, completed early | completed | reviewer found no blocking closure findings | completed |

### Reviewer Outputs

#### final-roi-closure-adversary-r4

##### Summary
Round 3 accepted blocking findings appear closed for graph-health v2. No remaining blocking findings were found.

##### Blocking Findings
None.

##### Evidence
- `scripts/action-map-graph-health-lib.ps1`: `Test-TaskspaceResultSupportsDecision` normalizes result IDs through `Get-TaskspaceResultId`, requires explicit `adoptedByDecisions` to match a current decision that depends on the result, and returns false after unmatched explicit decision refs.
- `scripts/action-map-graph-health-lib.ps1`: decision-yield set keys use `Get-TaskspaceResultId`.
- `scripts/action-map-graph-health-lib.ps1`: helper covers `id`, `Id`, `resultId`, `result_id`.
- `scripts/test-action-map-graph-health.ps1`: stale adopted-decision test has current `decision-1` depending on `result-1`, expects yield count `0`, and expects `subagent_no_decision_yield`.
- `scripts/test-action-map-graph-health.ps1`: exported-shape test uses two `resultId` rows and expects count `2`, yield `1`.

##### Non-blocking Risks
- Benchmark stale-decision test covered stale explicit adoption, but its current decision did not reproduce the exact fallback counterexample.
- Viewer ROI logic looks corrected, but coverage is still static/string-level rather than executing the embedded JS ROI function.
- Minor viewer warning evidence used `r.id` for no-adoption warnings, so `resultId`-only rows could show blank evidence.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| final-roi-closure-adversary | No blocking findings. | n/a | accept | Fresh closure reviewer found Round 3 blockers closed. | Marked closure passed. | None. |
| final-roi-closure-adversary | Benchmark exact fallback counterexample missing. | non-blocking | accept | Low-cost test hardening. | Updated benchmark stale-decision test so current `decision-1.dependsOnResults=["result-1"]` while adoption cites `deleted-decision`. Re-ran benchmark harness: PASS. | None. |
| final-roi-closure-adversary | Viewer no-adoption warning could show blank evidence for resultId-only rows. | non-blocking | accept | Low-cost user-facing evidence polish. | Updated viewer warning evidence to use `r.id || r.resultId || r.result_id`. Re-ran viewer test: PASS. | None. |
| final-roi-closure-adversary | Viewer ROI still string-level tested only. | non-blocking | defer | A JS execution harness is out of scope for Phase 5 closure; static guardrails and source review cover the changed expressions. | No additional harness added. | Revisit if viewer panel becomes release-critical. |

### Post-Fix Validation

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1`: PASS
- `cargo test -p codex-tui action_map_viewer -- --nocapture`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-action-map-graph-health.ps1`: PASS

### Final Closure Status

- Blocking findings found in closure rounds: yes
- Accepted blocking findings fixed: yes
- Fresh blocking re-review completed after fixes: yes
- Blocking re-review passed: yes
- Remaining blocking findings: none
- Deferred findings: viewer executable JS ROI harness, non-blocking
- Phase 5 allowed to close: yes
