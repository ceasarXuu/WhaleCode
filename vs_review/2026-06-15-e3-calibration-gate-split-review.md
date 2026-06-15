# Subagent VS Review: E3 Calibration Gate Split

- Created: 2026-06-15T20:32:00+08:00
- Updated: 2026-06-15T20:45:00+08:00
- Report schema: adversarial-v1
- Task: Complete remaining TaskSpace E3 Harness Guardrails implementation work and prevent engineering-unclean or speed-evidence issues from producing invalid E3 conclusions.
- Report path: `vs_review/2026-06-15-e3-calibration-gate-split-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Calibration Gate Release Semantics

### Review Input

#### Objective
Ensure the TaskSpace E3 harness only allows full E3 execution when calibration evidence is clean, while preventing incomplete runtime speed attribution from being mistaken for a score-validity or engineering-clean failure.

#### Review Target
Implementation and tests for the E3 calibration/start gate split between full-E3 eligibility and speed-claim eligibility.

#### Target Locations
- `scripts/taskspace-benchmark/lib/calibration-gate.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `target/e3-official-guardrails-completion-20260615/calibration-gate-after-log-summary-one-pair.json`

#### Change Introduction
The calibration gate now treats generated timing/report evidence with `score_valid=true` and `timing_quality=complete` as sufficient for full-E3 eligibility even if speed attribution is blocked by instrumentation. Speed claims are separately disabled through `speed_claim_allowed=false`. Invalid-run and engineering-unclean source timing evidence still fails closed.

#### Risk Focus
- Speed-instrumentation blockers could accidentally permit engineering-unclean runs.
- `gate-decision.json` could route operators to the wrong next action.
- Missing or placeholder runtime report fields could pass as generated evidence.
- Tests could prove only self-referential fixtures.

#### User-Perspective Review Focus
- Operator should understand whether E3 may run, whether speed claims are allowed, and what next command class is allowed.
- Operator should not waste hours on a run that should have been stopped by gate evidence.

#### Assumptions To Attack
- `score_valid=true` plus complete timing is enough to separate engineering cleanliness from speed attribution.
- `speedup_blocked_instrumentation` never indicates engineering uncleanliness.
- Invalid runs always surface through `score_valid=false`, source timing dirty markers, or `speedup_blocked_invalid_run`.
- Calibration failures can be detected from `calibration_*` start-gate rows.

#### Adversarial Lenses
- release operations
- failure
- testing
- observability

#### Verification Status
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1` PASS.
- `.\scripts\taskspace-benchmark\test-e3-start-gate.ps1` PASS.
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1` PASS.
- `.\scripts\taskspace-benchmark\test-external-wrapper-harness.ps1` PASS.
- `git diff --check` PASS with CRLF warnings only.
- Real `log-summary` one-pair calibration artifact passes one-pair gate, disables speed claims, preserves speed blockers, and still blocks full E3 because serial/parallel artifacts are absent.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Try to disprove release safety, not confirm intent.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release-ops-adversary | The change controls whether expensive E3 runs are allowed and whether speed conclusions may be used. | operational gating, invalid results, wasted long runs |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | `multi_agent_v1.spawn_agent` | `019ecb44-d260-7520-86ed-126ad5a804e5 / Avicenna` | spawn_agent tool call | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round1-release-ops | release-ops-adversary | 1 | `019ecb44-d260-7520-86ed-126ad5a804e5 / Avicenna` | 10 minutes | completed | returned blocking findings | completed |

### Reviewer Outputs

#### round1-release-ops

##### Summary
Read-only review completed. The reviewer found two release-blocking risks: one safety risk in the calibration gate trust boundary, and one operational risk in `gate-decision.json`.

##### Blocking Findings
- Calibration gate can pass full-E3 eligibility from self-reported `score_valid=true` without independently checking engineering cleanliness.
  - Broken assumption: invalid runs always surface as `score_valid=false` or `speedup_blocked_invalid_run`.
  - Failure scenario: dirty timing has non-empty `engineering_unclean_reasons`, but runtime report claims `score_valid=true`.
  - Trigger condition: stale or fabricated report paired with dirty timing.
  - Impact: full E3 can be authorized from engineering-unclean calibration evidence.
  - Proof needed: dirty timing fixtures with report `score_valid=true` must fail.
- Failed calibration decisions route operators to `fixture_tests` instead of calibration work.
  - Broken assumption: any failed start gate means the next command category is fixture tests.
  - Failure scenario: serial/parallel calibration artifacts are missing but `gate-decision.json` says `fixture_tests`.
  - Trigger condition: calibration gate fails with missing or malformed calibration artifacts.
  - Impact: operators or automation can take the wrong next action.
  - Proof needed: failed calibration-only cases must produce `next_allowed_command_category=serial_calibration`.

##### Non-blocking Risks
- Generated-report provenance remains weak.
- Parallel smoke evidence sample binding remains thin.

##### User-Perspective Checks
- Usability: risk - calibration failures could direct the operator toward fixture tests.
- Ease of use: pass for the real one-pair artifact, which blocks full E3 because serial and parallel artifacts are missing.
- Ease of understanding: risk - calibration details did not yet preserve speed-blocker reasons.

##### Required Fixes
- Make full-E3 eligibility verify engineering cleanliness from source timing artifacts.
- Make `gate-decision.json` choose calibration-specific next categories for calibration failures.
- Preserve speed blocker diagnostics in calibration gate details.

##### Missing Tests
- Dirty timing artifact with `score_valid=true` report must fail.
- Failed calibration gate must produce the correct next command category.
- Parallel equivalence and generated-report provenance need future hardening tests.

##### Missing Logs / Observability
- Calibration gate pass details should preserve speedup decision reason and blocker lists.
- Start-gate Markdown should show `full_e3_allowed`, `speed_claim_allowed`, and next command category.

##### Evidence
- `scripts/taskspace-benchmark/lib/calibration-gate.ps1` - report trust boundary.
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` - next command category behavior.
- `target/e3-official-guardrails-completion-20260615/calibration-gate-after-log-summary-one-pair.json` - real one-pair pass and missing serial/parallel failures.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | Self-reported `score_valid=true` can mask dirty timing evidence. | Dirty source timing paired with score-valid report could authorize full E3. | blocking | accept | `calibration-gate.ps1` now checks source timing `engineering_unclean`, non-empty `engineering_unclean_reasons`, and `bottleneck_classification=engineering_unclean_slow`. | Added `Test-TaskspaceCalibrationEngineeringCleanTiming`; added dirty one-pair and serial fixtures plus isolated one-pair dirty-indicator fixtures. | Closure review Round 2. |
| release-ops-adversary | Failed calibration decisions route to fixture tests. | Missing/bad calibration artifacts produced blocked decision with wrong next category. | blocking | accept | `e3-start-gate.ps1` now detects failed `calibration_*` rows and sets `next_allowed_command_category=serial_calibration`. | Updated routing and added aggregate-only, invalid-run, and missing timing-path routing assertions. | Closure review Round 2. |
| release-ops-adversary | Speed blocker details missing from gate details. | Operator cannot understand why speed claims are disabled. | major | accept | `calibration-gate.ps1` now preserves `speedup_decision_reason`, `runtime_optimization_blockers`, and wait-attribution diagnostic fields. | Added diagnostic passthrough and tests. | Closure review Round 2. |
| release-ops-adversary | Start-gate Markdown lacks next/full/speed summary. | Operator has to open JSON to understand allowed next action. | major | accept | `e3-start-gate.ps1` attaches `gate_decision` before Markdown generation and renders the summary. | Added Markdown summary and tests. | Closure review Round 2. |
| release-ops-adversary | Generated-report provenance remains weak. | Hand-built JSON with required fields can still pass if source timing is clean. | major | defer | Current blocking risk was dirty-source masking, now fixed. Provenance needs broader report schema work. | Deferred. | Track in guardrail hardening. |
| release-ops-adversary | Parallel smoke sample binding remains thin. | A one-sample equivalence artifact could pass despite larger serial calibration. | major | defer | Existing parallel smoke semantics are unchanged by this split. | Deferred. | Track before enabling parallel E3 speed claims. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `release-ops-adversary | multi_agent_v1.spawn_agent | 019ecb4d-790b-7560-a59f-56fd5e6db2cc / Nietzsche`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes

## Round 2: Blocking Finding Closure

### Review Input

#### Objective
Verify closure of two accepted blocking findings from Round 1 for TaskSpace E3 calibration/start gate release safety.

#### Review Target
Closure implementation and tests for calibration source timing cleanliness and calibration-failure next-action routing.

#### Target Locations
- `scripts/taskspace-benchmark/lib/calibration-gate.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `vs_review/2026-06-15-e3-calibration-gate-split-review.md`
- `target/e3-official-guardrails-completion-20260615/calibration-gate-after-log-summary-one-pair.json`

#### Change Introduction
The accepted blocking findings were addressed by adding source timing engineering-clean checks and calibration-specific next command routing. Diagnostic passthrough and start-gate Markdown summary were also added.

#### Risk Focus
- Dirty timing artifacts could still bypass the gate through an untested field shape.
- Calibration failures could still route to fixture tests in some failure ordering.
- Closure tests could assert synthetic-only behavior that real artifacts do not follow.

#### User-Perspective Review Focus
- Operator should be routed to calibration work after calibration evidence fails.
- Operator should see the next/full/speed summary and why speed claims are disabled.

#### Assumptions To Attack
- Dirty timing markers cover source engineering-unclean states.
- `calibration_*` failed rows are sufficient to detect calibration failures.
- Closure tests would fail if the original blocking findings regressed.

#### Adversarial Lenses
- release operations
- failure
- testing
- observability

#### Verification Status
- Guardrail, start-gate, score-validity, external wrapper, and whitespace checks passed before Round 2.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus only on whether the accepted blocking findings are closed.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release-ops-adversary | Same operational gate semantics as Round 1; closure must prove release safety. | operational gating, invalid results, wasted long runs |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | `multi_agent_v1.spawn_agent` | `019ecb4d-790b-7560-a59f-56fd5e6db2cc / Nietzsche` | spawn_agent tool call | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round2-release-ops | release-ops-adversary | 1 | `019ecb4d-790b-7560-a59f-56fd5e6db2cc / Nietzsche` | 10 minutes | completed | no blocking counterexample found | completed |

### Reviewer Outputs

#### round2-release-ops

##### Summary
Closure verified read-only. The reviewer found no release-blocking counterexample for the accepted Round 1 findings. The calibration gate now checks source timing artifacts before trusting `score_valid=true`, and the start gate routes failed `calibration_*` rows to `serial_calibration`.

##### Blocking Findings
- none

##### Non-blocking Risks
- Dirty-indicator tests were initially combined rather than fully mutation-resistant.
- Generated-report provenance and parallel equivalence sample binding remain deferred.
- A single wait-attribution diagnostic value can serialize as a scalar in inspected JSON.

##### User-Perspective Checks
- Usability: pass - failed calibration evidence now points operators back to calibration work.
- Ease of use: pass - start-gate Markdown renders next/full/speed summary.
- Ease of understanding: pass with minor shape risk - diagnostic fields are preserved.

##### Required Fixes
- none for the accepted blocking findings

##### Missing Tests
- Non-blocking: add isolated fixtures for each dirty timing indicator.
- Non-blocking: add strict JSON-shape fixture if downstream consumers expect arrays.

##### Missing Logs / Observability
- No blocker. Diagnostic passthrough exists in code and the inspected real artifact.

##### Evidence
- `scripts/taskspace-benchmark/lib/calibration-gate.ps1` - source timing dirty checks and report/timing binding.
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1` - calibration failure routing and gate decision write order.
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` - dirty timing tests.
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1` - routing tests.
- `target/e3-official-guardrails-completion-20260615/calibration-gate-after-log-summary-one-pair.json` - real artifact blocks full E3 while preserving speed blockers.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | No blocking closure findings. | n/a | n/a | accept | Round 2 found no release-blocking counterexample. | No further blocking fix required. | n/a |
| release-ops-adversary | Dirty-indicator tests were not mutation-resistant enough. | Future edit could remove one branch and still pass combined tests. | major | accept | `test-e3-harness-guardrails.ps1` now adds isolated one-pair fixtures for `engineering_unclean=true`, non-empty `engineering_unclean_reasons`, and `bottleneck_classification=engineering_unclean_slow`. | Added isolated dirty-indicator tests and reran harness self-test. | n/a |
| release-ops-adversary | Single wait-attribution diagnostic can serialize as scalar. | Downstream consumers expecting arrays could need stricter shape. | minor | defer | Current diagnostics are operator evidence; no downstream array consumer is in scope. | Deferred to schema hardening if needed. | Track with provenance/schema follow-up. |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `release-ops-adversary | multi_agent_v1.spawn_agent | 019ecb4d-790b-7560-a59f-56fd5e6db2cc / Nietzsche`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Passed. The accepted blocking findings were fixed and passed fresh closure review. Remaining deferred items are non-blocking hardening work: generated-report provenance, parallel equivalence sample binding, and strict diagnostic array shape if future consumers require it.
