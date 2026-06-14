# Subagent VS Review: E3 Runtime Speedup Plan

- Created: 2026-06-14T23:00:00+08:00
- Updated: 2026-06-14T23:20:00+08:00
- Report schema: adversarial-v1
- Task: Add detailed runtime bottleneck and speedup planning to the TaskSpace E3 guardrails plan.
- Report path: `vs_review/2026-06-14-e3-runtime-speedup-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Runtime Plan Adversarial Review

### Review Input

#### Objective
Review whether the E3 runtime-speedup additions are concrete, internally consistent, and safe for score validity.

#### Review Target
Documentation / engineering implementation plan.

#### Target Locations
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The plan adds a runtime bottleneck model for 15-task E3, timing artifact requirements, speedup phases S0-S5, audit-required handling, disk/Docker timing, resource-governed parallelism, and calibration requirements.

#### Risk Focus
- Hidden contradictions between hard engineering-unclean execution rules and audit-required score-pending rules.
- Validator timeout semantics weakening the user's hard execution constraint.
- Parallelism making wall-time and score-comparison evidence invalid.
- Speedup claims exceeding evidence.

#### User-Perspective Review Focus
- Whether a future engineer can implement the plan without hidden context.
- Whether a future operator can tell invalid harness, audit-pending, and score-bearing outcomes apart.

#### Assumptions To Attack
- Timing artifacts can prove bottlenecks.
- Pure audit pending is not an engineering failure.
- Parallel runs can remain score-comparable.
- The plan does not overpromise runtime reductions.

#### Adversarial Lenses
- requirements
- concurrency
- failure
- testing
- observability
- documentation

#### Verification Status
- `git diff --check` passed before review with only CRLF warnings.
- No runtime tests were needed because this change is documentation-only.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target file directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 180s | none | 2 | accepted blocking findings require closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary | The target is an engineering plan that must be executable by future agents and engineers. | hidden contradictions, missing validation, unclear workflow |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | `multi_agent_v1.spawn_agent` | `019ec689-3f47-72a1-8f53-599bad768595` | spawn_agent result and subagent notification | false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| gauss-runtime-plan-review | documentation-skill-adversary | 1 | `019ec689-3f47-72a1-8f53-599bad768595` | 180s | completed | returned within wait | completed |

### Reviewer Outputs

#### gauss-runtime-plan-review

##### Summary
The runtime-speedup section is stronger than a generic plan, but it had semantic contradictions and one parallel timing flaw that could let engineers implement a faster-looking harness without proving comparable scoring.

##### Blocking Findings
- Audit-required semantics conflict with the earlier hard scoring contract.
  - Broken assumption: pure incomplete audit has one authoritative status.
  - Failure scenario: implementers cannot tell whether incomplete audit exits `3`, sets `score_valid=false`, sets `run_validity=audit_required`, or continues all pairs.
  - Trigger condition: a run produces complete pair artifacts but no completed human audit.
  - Impact: full E3 could abort as invalid harness or publish invalid score language inconsistently.
  - Proof needed: a state machine and fixtures for pure audit-required versus malformed audit.
- Validator timeout semantics conflict and can weaken the hard execution constraint.
  - Broken assumption: post-`tests_started` validator timeout is clearly classified.
  - Failure scenario: timeout after tests start is treated as benchmark outcome even though the execution constraint allows only agent execution timeout as an unexpected condition.
  - Trigger condition: public validation exits `124` after lifecycle marker `tests_started`.
  - Impact: engineering failure can pollute agent score.
  - Proof needed: explicit timeout classification and fixtures for pretest versus post-start timeout.
- Parallel timing attribution is under-specified for overlapped work.
  - Broken assumption: summed child durations can reconcile directly to wall time under parallelism.
  - Failure scenario: inclusive child durations exceed wall time, producing incorrect bottleneck class or speedup claim.
  - Trigger condition: `MaxParallelSamples`, pair parallelism, or validation parallelism overlaps spans.
  - Impact: speedup claims and largest-phase analysis can become mathematically wrong.
  - Proof needed: interval spans, critical path, exclusive wall attribution, and resource-wait tests.

##### Non-blocking Risks
- The planned first safe parallel smoke command omitted planned parallel flags.
- `score_valid equivalent` was too weak for parallel acceptance.
- `>=30%` should be calibration-backed, not a hard pass/fail target before bottleneck evidence.

##### User-Perspective Checks
- Usability: risk - contradictory status terms would make the plan hard to execute.
- Ease of use: risk - parallel smoke command looked serial despite its heading.
- Ease of understanding: risk - speedup target wording could be read as a promise.

##### Required Fixes
- Define authoritative `engineering_unclean`, `audit_required`, `score_ready`, and `score_valid` semantics.
- Keep public validation timeout as engineering-unclean.
- Add parallel timing span and overlap accounting requirements.

##### Missing Tests
- Pure `audit_required` fixture with expected exit/status, score readiness, resumability, and no score language.
- Pretest timeout versus post-`tests_started` validator timeout fixtures.
- Parallel overlap timing fixture.
- Parallel smoke assertion for observed `MaxParallelSamples=2`.

##### Missing Logs / Observability
- Parallel timing needs resource wait and critical path artifacts.

##### Evidence
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md` - reviewer cited conflicting audit, timeout, and parallel timing sections.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| gauss | Audit-required semantics conflict | Pure missing audit could be treated as engineering-unclean or score-pending depending on section | blocking | accept | The plan contained old lines treating `e3_human_review_not_completed` as engineering-unclean and new lines treating it as audit-required. | Updated the plan to define `score_ready`, `score_block_reason`, `audit_required_count`, pure audit-pending `run_validity=valid`, and malformed/hash-mismatched audit as `engineering_unclean`. | Round 2 closure review |
| gauss | Validator timeout semantics conflict | Post-`tests_started` validator timeout could become benchmark outcome | blocking | accept | User constraint allows only agent execution timeout as allowed unexpected condition. | Updated timeout mapping so any `public_validation_exit_code=124` remains engineering-unclean; lifecycle markers only refine reason. | Round 2 closure review |
| gauss | Parallel timing attribution under-specified | Inclusive child durations could be used as wall-time evidence | blocking | accept | Parallel spans can overlap and summed child durations can exceed wall time. | Added interval span fields, inclusive duration, exclusive wall attribution, critical path, and resource wait requirements plus overlap fixture acceptance. | Round 2 closure review |
| gauss | Parallel smoke command looked serial | Planned smoke omitted intended flags | major | accept | The command lacked `-MaxParallelSamples`. | Changed heading to post-R3 target command and added `-MaxParallelSamples 2`, `-MaxDockerConcurrency 1`, and `-MaxModelConcurrency 1`; added observed parallelism proof requirement. | Round 2 closure review |
| gauss | Parallel equivalence too weak | Score validity alone may hide inclusion/outcome/audit/proof drift | major | accept | Score validity is a coarse aggregate. | Strengthened acceptance to require score readiness, validity, inclusion/exclusion rows, hard outcome classification, and audit/proof parity. | Round 2 closure review |
| gauss | `>=30%` could overpromise | Speedup target may be impossible for agent-bound runs | major | accept | Plan should be evidence-backed. | Reworded `>=30%` as calibration-backed and conditional on observed bottleneck class. | Round 2 closure review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: pending closure review
- Blocking re-review completed: no
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - See Round 2 Reviewer Launch Records.
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: closure review pending
- Allowed to proceed: no

## Round 2: Closure Review

### Review Input

#### Objective
Verify whether the three blocking findings from Round 1 are closed.

#### Review Target
Documentation / engineering implementation plan after fixes.

#### Target Locations
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The plan was updated to split pure audit pending from engineering unclean, keep validator timeout as engineering unclean, add parallel timing overlap accounting, strengthen parallel acceptance, and make speed targets calibration-backed.

#### Risk Focus
- Whether the Round 1 contradictions remain.
- Whether closure changes introduced new ambiguity.

#### User-Perspective Review Focus
- Whether a future engineer can implement the corrected status and timing rules without hidden context.

#### Assumptions To Attack
- The state split is now authoritative.
- Timeout semantics now match the hard execution constraint.
- Parallel timing now has enough data to prove speedups.

#### Adversarial Lenses
- requirements
- concurrency
- testing
- observability
- documentation

#### Verification Status
- Closure review launched after document edits.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 180s | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary | Same artifact and closure of accepted blocking findings. | contradiction closure and implementation clarity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | `multi_agent_v1.spawn_agent` | `019ec68f-b3fe-7b12-b522-2c59b87861b5` | spawn_agent result | false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| mencius-closure-review | documentation-skill-adversary | 1 | `019ec68f-b3fe-7b12-b522-2c59b87861b5` | 180s | completed | returned within wait | completed |

### Reviewer Outputs

#### mencius-closure-review

##### Summary
All prior blocking findings are closed in the current document.

##### Blocking Findings
- none.

##### Non-blocking Risks
- Earlier sections still contained broad wording like "failure after `tests_started` remains a normal benchmark result" without restating "except timeout".
  - Broken assumption: later hard clean-execution addendum is enough to prevent misreading.
  - Failure scenario: future implementer reads the early section only and treats validator timeout as benchmark outcome.
  - Trigger condition: implementation work starts from the early guardrail policy table.
  - Impact: timeout classification can drift from the hard execution constraint.
  - Proof needed: early sections explicitly distinguish assertion failure from validator timeout.

##### User-Perspective Checks
- Usability: pass - audit pending, engineering unclean, and score readiness are now separable.
- Ease of use: pass - planned parallel smoke now contains the intended flags and states they are post-R3.
- Ease of understanding: minor residual risk fixed by main-agent follow-up.

##### Required Fixes
- none blocking.

##### Missing Tests
- none beyond the fixture coverage already added to the plan.

##### Missing Logs / Observability
- none beyond the parallel timing fields already added to the plan.

##### Evidence
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md` - audit semantics closed.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md` - validator timeout semantics closed.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md` - parallel timing attribution closed.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| mencius | Prior blocking findings are closed | n/a | n/a | accept | Closure reviewer confirmed the three Round 1 blocking findings are closed. | Kept the closure result and marked review passed. | n/a |
| mencius | Earlier broad `tests_started` wording could still mislead | Future implementer may treat validator timeout as benchmark outcome if reading early sections only | major | accept | The plan had older broad phrases before the hard addendum. | Updated early non-goals, sentinel policy, tests, risks, and success criteria to say assertion failures after `tests_started` are benchmark outcomes, but validator timeouts remain engineering-unclean. | n/a |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - `019ec68f-b3fe-7b12-b522-2c59b87861b5`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Passed. Round 1 blocking findings were accepted, fixed in the plan, and closed by a fresh internal subagent review. One non-blocking residual wording risk was accepted and fixed after closure review.
