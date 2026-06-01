# Subagent VS Review: TaskSpace E2 Optimization

- Created: 2026-06-01T05:22:36+08:00
- Updated: 2026-06-01T05:47:00+08:00
- Task: Optimize TaskSpace so E2-level paired real-agent tests can pass cleanly without hidden mechanism gaps.
- Report path: `vs_review/2026-06-01-taskspace-e2-optimization-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Post-implementation adversarial review

### Review Input

#### Objective

Challenge whether the TaskSpace E2 optimization is a real mechanism improvement rather than benchmark overfitting or prompt-only masking.

#### Review Target

Implementation, prompt/runtime policy, test evidence, and design documentation for the latest TaskSpace E2 clean-readiness optimization.

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/basemap.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs`
- `docs/testing/2026-06-01-taskspace-clean-e2-optimization-plan.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-044420-323\e2-matrix-report.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-045538-470\e2-matrix-report.md`

#### Change Introduction

The optimization tightens TaskSpace behavior around minimum sufficient maps, node completion evidence, ready-node-only subagent assignment, inspect-node budget, pre-fix diagnostic test placement, and product-doc/test/implementation reconciliation.

#### Risk Focus

- The added guidance may be too prompt-only and not sufficiently enforced by runtime.
- The product contract guidance may overfit one benchmark failure instead of generalizing.
- Ready-node-only spawn may reduce useful multi-agent handoff patterns.
- Evidence gates may be too brittle or too narrow for real coding tasks.
- Passing E2 matrix evidence may still miss unhealthy map growth, stale public tests, or low-value subagent results.

#### Verification Status

- `rustup run stable cargo test -p codex-core --lib action_map --locked --jobs 2` passed: 101 passed, 0 failed.
- L2 targeted matrix passed: `e2_evidence_readiness=True`, `e2_clean_readiness=True`, 3/3 valid pairs.
- Full L1/L2/L3 matrix passed: `e2_evidence_readiness=True`, `e2_clean_readiness=True`, 9/9 valid pairs, 0 warning pairs.
- Harness self-tests passed for `single-file-fast-fix`, `multi-file-order-pipeline`, and `subscription-billing-repair`.
- Installed `whale.exe` hash used by matrix: `4e5d47b606f8cf00f51fecd90a9f35fe32400ff9e6ded94ee87c536b9913be7f`.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on blocking risks that should prevent commit, plus non-blocking follow-ups.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace mechanism critic | Needs to challenge runtime/prompt boundaries and E2 evidence quality | mechanism correctness, benchmark overfit, validation gaps |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace mechanism critic | `multi_agent_v1.spawn_agent` explorer | `019e7feb-567d-7841-a0d1-0d720ec27127` / Avicenna | spawn_agent tool result | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### TaskSpace mechanism critic

##### Summary

The reviewer found real runtime improvements in ready-node-only spawn and main-agent typed completion gates, but did not consider the mechanism fully closed because subagent completion could bypass the same gate and tool success was inferred from text.

##### Blocking Findings

- Subagent completion could bypass typed evidence gates: `record_child_result` completed nodes from `AgentStatus::Completed` without checking whether implementation or validation evidence existed.
- Evidence gate success detection used text matching on result body, so a failed tool output containing `success: true` could satisfy completion.
- Product document/test/implementation reconciliation remains mostly prompt-level and was introduced after a specific L2 benchmark failure, so it needs to be treated as methodology evidence rather than a fully objective runtime gate.

##### Non-blocking Risks

- Ready-node-only spawn is a deliberate simplification but reduces mid-node handoff flexibility.
- Allowing build evidence to satisfy smoke/regression nodes may be too broad for some scenarios.
- The matrix report can show clean readiness while some pairs still have higher TaskSpace cost.
- Subagent result quantity is visible, but result adoption/quality is not yet measured.

##### Required Fixes

- Apply typed completion evidence to subagent-owned nodes.
- Store tool success as structured metadata rather than reading it from result text.
- Record product-rule conflict handling as explicit node rationale and broaden future benchmarks beyond this one conflict shape.

##### Missing Tests

- Subagent implementation node cannot complete without edit evidence.
- Subagent smoke/regression node cannot complete without successful test/build evidence.
- Failed tool output containing `success: true` cannot satisfy completion evidence.
- Future product-conflict benchmark variants should cover non-README specs and stale-docs counterexamples.

##### Missing Logs / Observability

- Pair-level cost warnings need to remain visible even when matrix-level clean readiness passes.
- Subagent result adoption and independent evidence value are not yet measured.

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `record_child_result`, `validate_completion_evidence`, and `node_has_successful_action`.
- `third_party/codex-cli/codex-rs/core/src/action_map/contracts.rs` - implement/test nodes can be subagent-owned.
- `C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-045538-470\e2-matrix-report.md` - full matrix passed but reports aggregate utility separately.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| TaskSpace mechanism critic | Subagent completion bypasses typed evidence gates | blocking | accept | Subagent-owned implement/test nodes use the same node kinds and should not have weaker completion rules. | `record_child_result` now converts completion without required evidence into a blocker; added subagent implement/smoke tests. | Closure review Round 2 |
| TaskSpace mechanism critic | Tool success inferred from result body text | blocking | accept | Text preview is untrusted evidence; completion must use structured metadata. | Added `tool_success: Option<bool>` to `NodeResult` and snapshots; completion evidence now checks `tool_success == Some(true)`. | Closure review Round 2 |
| TaskSpace mechanism critic | Product contract alignment is prompt-level and benchmark-shaped | blocking | reject as blocker, accept as bounded risk | Runtime cannot objectively adjudicate semantic product authority without becoming a brittle oracle. The intended first-line mechanism is inspect-node methodology plus node result rationale. Current E2 claim is scoped to readiness/clean matrix, not broad utility proof. | Documented the limitation and kept the rule generic: reconcile docs/tests/implementation and record rationale. | Broaden in future E3 benchmark variants |
| TaskSpace mechanism critic | Ready-node-only spawn reduces mid-node handoff | non-blocking | accept | This is a deliberate product simplification matching node ownership rules. | Already documented in optimization plan. | Revisit only if real workflows need handoff |
| TaskSpace mechanism critic | Build evidence may be too broad for regression nodes | non-blocking | defer | Some repos only have build/typecheck validation; stricter per-project validation belongs in future node contracts. | No code change in this round. | Track with richer benchmark scenarios |
| TaskSpace mechanism critic | Matrix clean readiness can hide pair-level cost drag | non-blocking | accept | Clean readiness is mechanism-readiness, not utility superiority. | Final report will state cost drag separately; no code change. | E3 utility benchmark should treat this as primary metric |
| TaskSpace mechanism critic | Subagent result quality/adoption is not measured | non-blocking | accept | Counts alone do not prove value. | No code change in E2 closure. | Add adoption/value metrics in E3 design |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - pending
- Blocking re-review launch records:
  - Round 2 launch record pending
- Rejected findings backed by evidence: pending
- Deferred findings documented: pending
- Allowed to proceed: pending

## Final Conclusion

Pending.

## Round 2: Accepted blocking closure review

### Review Input

#### Objective

Verify that Round 1 accepted blocking findings are closed without relying on main-agent context.

#### Review Target

The runtime evidence-gate hardening for subagent completion and structured tool success.

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `docs/testing/2026-06-01-taskspace-clean-e2-optimization-plan.md`
- `vs_review/2026-06-01-taskspace-e2-optimization-review.md`

#### Change Introduction

Round 1 accepted two blocking issues. The implementation now stores tool success as structured result metadata and applies typed completion evidence validation to subagent-owned nodes.

#### Risk Focus

- Subagent completion may still bypass evidence gates.
- Completion evidence may still rely on result body text.
- Snapshot restore may lose structured success metadata.
- Tests may not cover the accepted blocking paths.

#### Verification Status

- `rustup run stable cargo test -p codex-core --lib action_map --locked --jobs 2` passed: 104 passed, 0 failed.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Report only closure blockers and residual risks.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace closure critic | Needs to verify accepted blocking fixes only | closure correctness, regression risk |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace closure critic | `multi_agent_v1.spawn_agent` explorer | `019e7ffb-2cc2-7f20-a663-8cc1c7b8371c` / Banach | spawn_agent tool result | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### TaskSpace closure critic

##### Summary

Round 1 two accepted blocking issues are closed. Subagent completed/map-update results now call the same typed completion evidence gate, and completion evidence uses `tool_success: Some(true)` rather than body text matching.

##### Closure Blocking Findings

- none

##### Remaining Non-blocking Risks

- Round 2 output was pending before this update; now recorded here.
- Build evidence still satisfies smoke/regression nodes. This is a known product boundary risk, not one of the accepted closure blockers.

##### Missing Tests

- Core closure paths are covered: subagent implementation without edit evidence blocks; subagent smoke without successful validation blocks; failed output containing `success: true` cannot fake success.
- Reviewer suggested adding a snapshot round-trip test for `tool_success` preservation.

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `record_child_result` calls `validate_completion_evidence_for` before completing subagent-owned nodes.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - `node_has_successful_action` checks `tool_success == Some(true)`.
- `third_party/codex-cli/codex-rs/core/src/action_map/map.rs` - `NodeResult` has `tool_success: Option<bool>`.
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs` - `ActionMapSnapshotResult` has `tool_success: Option<bool>` with default deserialization.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` - tests cover failed preview text, subagent implement evidence, and subagent smoke evidence.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| TaskSpace closure critic | No closure blockers | blocking | accept | Fresh reviewer found both accepted blockers closed. | No further blocker fix needed. | n/a |
| TaskSpace closure critic | Add snapshot round-trip test for `tool_success` | non-blocking | accept | Context restore is part of TaskSpace runtime reliability and the test is cheap. | Added `snapshot_restore_preserves_tool_success_for_completion_evidence`. | Re-run action_map tests |
| TaskSpace closure critic | Build evidence may be too broad for smoke/regression | non-blocking | defer | This needs future node-contract granularity, not a closure fix. | No code change in this round. | E3 benchmark design |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Allowed to proceed: yes

## Final Conclusion

Passed. Round 1 accepted blockers were fixed, Round 2 found no closure blockers, and the remaining risks are explicitly scoped as future E3/product-contract work rather than E2 mechanism blockers.

## Round 3: E2 clean semantics and serial inspect closure review

### Review Input

#### Objective

Verify that the two later accepted blocking findings are closed without relying on main-agent context.

#### Review Target

The E2 matrix reporting semantics, subagent-result metrics, and serial inspect delegation guard.

#### Target Locations

- `scripts/taskspace-benchmark/run-taskspace-e2-matrix.ps1`
- `scripts/taskspace-benchmark/lib/matrix-report.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `docs/testing/2026-06-01-taskspace-clean-e2-optimization-plan.md`
- `vs_review/2026-06-01-taskspace-e2-optimization-review.md`

#### Change Introduction

Round 3 responds to two accepted blocking findings from the latest review:

- `e2_clean_readiness` previously looked like a clean utility claim even when pair-level cost warnings existed.
- The serial inspect delegation guard only covered a completed-narrow-inspect sequence and could still allow main-running-inspect plus one ready inspect to become a half-parallel handoff.

The implementation now separates mechanism clean readiness from utility cost clean readiness, fixes `subagent_results` counting to use real subagent lease threads, rejects one-ready-inspect spawn while the main agent already holds a running inspect node, and keeps maintenance-barrier recovery as an explicit exception.

#### Risk Focus

- The revised reporting could still mislead readers about E2 utility.
- The inspect guard could still be benchmark-shaped or could break legitimate initial ready-node assignment.
- The new guard could block maintenance-barrier recovery.
- Tests might cover only the happy path.

#### Verification Status

- `rustup run stable cargo test -p codex-core --lib action_map --locked --jobs 2` passed: 109 passed, 0 failed.
- Benchmark harness self-tests passed for `single-file-fast-fix`, `multi-file-order-pipeline`, and `subscription-billing-repair`.
- Local Whale build 18 installed with SHA256 `24F0BFE16185473BC9EE3D3AD8F22E3D11AF1CE061537DC8B30196F5DF7E19BF`.
- Full real E2 matrix: `C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-171111-912\e2-matrix-report.md`.
- Latest matrix result: `e2_evidence_readiness=True`, `e2_clean_readiness=True`, `e2_utility_clean_readiness=False`, L1/L2/L3 9/9 valid pairs, mechanism warning gaps none, L1 utility cost gaps visible.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Report closure blockers, non-blocking residual risks, and missing tests.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace closure critic | Needs to verify accepted reporting and inspect-guard blockers | reporting semantics, guard correctness, regression risk |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace closure critic | `multi_agent_v1.spawn_agent` explorer | `019e828d-e8d7-7d02-bf6a-f3f5aa807d42` / Raman | spawn_agent tool result | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### TaskSpace closure critic

##### Summary

Closure result: both accepted blocking findings are closed in the current worktree. No new code-level blocking issue should prevent commit.

`e2_clean_readiness=True` is now explicitly scoped to mechanism cleanliness, while utility cost drag is separated into `e2_utility_clean_readiness=False` plus `Utility Cost Gaps`. The latest matrix report makes this visible.

##### Closure Blocking Findings

- none

##### Remaining Non-blocking Risks

- The review report needed this final closure round recorded before commit.
- Maintenance-barrier recovery is tested for the base case, but not for the combined edge case where an earlier completed narrow inspect also exists.

##### Missing Tests

No blocking missing tests.

Covered:

- running-main-inspect plus one ready inspect rejection
- completed narrow inspect plus serial follow-up rejection
- two-track inspect group assignment
- initial single ready inspect spawn remains allowed
- maintenance barrier allows subagent on different ready node
- utility-clean separation in harness self-test

##### Evidence

- `scripts/taskspace-benchmark/lib/matrix-report.ps1` separates `utility_cost_gaps` and `e2_utility_clean_readiness`.
- `scripts/taskspace-benchmark/run-taskspace-e2-matrix.ps1` counts utility warnings separately and writes readiness scope.
- Latest real report shows `e2_clean_readiness=True`, `e2_utility_clean_readiness=False`, and L1 utility gap: `C:\Users\77585\AppData\Local\Temp\whale-paired-matrix-runs\20260601-171111-912\e2-matrix-report.md`.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` guard logic checks ready plus active inspect capacity, running main inspect, and completed narrow inspect.
- Installed Whale hash matches build 18: `24F0BFE16185473BC9EE3D3AD8F22E3D11AF1CE061537DC8B30196F5DF7E19BF`.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| TaskSpace closure critic | No closure blockers | blocking | accept | Fresh reviewer found both later accepted blockers closed. | No further code fix needed. | n/a |
| TaskSpace closure critic | Record closure round before commit | non-blocking/process | accept | Project review-trail rule requires the report to include launch records and reviewer output. | Added Round 3 to this report. | Commit with related changes |
| TaskSpace closure critic | Combined completed-narrow-inspect plus maintenance-barrier recovery edge lacks explicit test | non-blocking | defer | Base maintenance-barrier recovery and serial narrow-inspect rejection are covered independently. The combined edge is rare and not needed for E2 closure. | No code change in this round. | Add if a future regression appears |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Allowed to proceed: yes

## Current Final Conclusion

Passed for E2 mechanism readiness. The implementation now has runtime-level node completion evidence gates, structured `tool_success`, subagent completion hardening, stricter serial inspect delegation rules, and honest matrix reporting that separates E2 mechanism clean from utility cost clean. The latest full matrix proves `e2_evidence_readiness=True` and `e2_clean_readiness=True`; it also honestly reports `e2_utility_clean_readiness=False` due L1 wall-time drag, so this remains an E2 mechanism pass rather than an E3 utility-superiority claim.
