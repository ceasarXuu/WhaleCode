# Subagent VS Review: E3 Speedup Plan

- Created: 2026-06-15T03:20:00+08:00
- Updated: 2026-06-15T03:42:00+08:00
- Report schema: adversarial-v1
- Task: add the multi-hour E3 runtime bottleneck and speedup work into the guardrails implementation plan with concrete engineering phases and acceptance standards
- Report path: `vs_review/2026-06-15-e3-speedup-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Plan Executability Review

### Review Input

#### Objective

Review whether the added E3 speedup plan is concrete enough to guide engineering implementation and conservative enough to prevent another multi-hour invalid E3 run.

#### Review Target

Documentation and execution plan.

#### Target Locations

- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md`

#### Change Introduction

The implementation plan now includes Section 16.14, a detailed performance diagnosis and speedup execution plan covering timing budget buckets, early-abort timing closure, invalid-run waste elimination, validator/Docker cost reduction, governed sample-level parallelism, speed-claim decision rules, concrete implementation order, and an adversarial review checklist. The completion audit now records this section as planned but not implemented.

#### Risk Focus

- The plan may still be too abstract for a fresh engineer to implement without hidden context.
- The plan may accidentally authorize speed claims or full E3 before clean calibration evidence exists.
- The plan may miss a hard gate that prevents Docker/validator/audit/report failures from being counted as model outcomes.
- The plan may define speed buckets without tying them to actual artifacts and tests strongly enough.
- The plan may not make the immediate next repair step unambiguous.

#### User-Perspective Review Focus

- A realistic operator should be able to tell what command category is allowed next.
- A fresh engineer should be able to identify the first implementation task and the evidence required to close it.
- The plan should not let a reader confuse planned speedup work with completed guardrail implementation.

#### Assumptions To Attack

- Section 16.14 is executable without reading the conversation history.
- `gate-decision.json` is sufficient as the sole authorization artifact.
- Early-abort timing closure is clearly the first engineering task.
- Speed claims are blocked until serial and parallel calibration artifacts exist.
- The completion audit prevents status inflation.

#### Adversarial Lenses

- documentation
- release-ops
- observability
- test-validity
- maintenance
- comprehension

#### Verification Status

- The change is documentation-only in this round.
- No full E3 or calibration run was executed for this documentation change.
- Existing uncommitted code changes are present in the working tree and are not part of this review target.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on falsifying the plan's executability, gate safety, and anti-self-deception properties.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary | The target is a plan that future engineers and agents must execute from documentation without hidden chat context. | fresh-session executability, hidden assumptions, ambiguous gates |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | multi_agent_v1.spawn_agent | 019ec78f-1b28-74e0-a0a1-379483081659 | spawn_agent tool result | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-output-1 | documentation-skill-adversary | 1 | 019ec78f-1b28-74e0-a0a1-379483081659 | 10 minutes | completed | reviewer completed | completed |

### Reviewer Outputs

#### reviewer-output-1

##### Summary

The speedup plan is directionally conservative and mostly concrete, but the reviewer found blocking documentation issues. The plan did not yet make the full-E3 authorization source unambiguous, and the completion audit action list undercut the stated "early-abort timing first" sequence.

##### Blocking Findings

- Gate authorization is internally inconsistent and unsafe as written.
  - Broken assumption: `gate-decision.json` is sufficient as the sole authorization artifact.
  - Failure scenario: a stale or hand-built `gate-decision.json` says `full_e3_allowed=true`, but the linked calibration gate, review, or parallel comparator artifacts are absent or from a different run.
  - Trigger condition: an operator follows "only authorization" without validating calibration/review/provenance artifacts.
  - Impact: full E3 can start from stale or incomplete evidence, recreating a multi-hour invalid run.
  - Proof needed: provenance-bearing gate schema plus negative fixtures for missing/stale/mismatched referenced artifacts.
- The audit does not make early-abort timing closure clearly first.
  - Broken assumption: a fresh engineer will execute Section 16.14 in order.
  - Failure scenario: the audit action list starts with Docker cache or comparator work while the known pre-scheduling timing gap remains open.
  - Trigger condition: an engineer follows the audit action list top-down.
  - Impact: work can drift away from the immediate blocker that prevents early invalid run diagnosis.
  - Proof needed: audit action list reordered so early-abort timing and reconstruction sample rows are actions 1-2.

##### Non-blocking Risks

- `prompt/config hashes when available` weakens comparator coverage.
- "latest accepted serial calibration" is required for `time_saved_estimate_ms`, but Section 16.14 did not define discovery or acceptance provenance locally.
- The review report was pending at review time, so the audit should not be treated as review-closed yet.

##### User-Perspective Checks

- Usability: risk - a cautious operator can see full E3 is blocked, but may not know whether `gate-decision.json` alone is authoritative.
- Ease of use: risk - a fresh engineer can identify Section 16.14's intended sequence, but not reliably from the audit action list.
- Ease of understanding: risk - audit and plan wording were inconsistent on gate authorization.

##### Required Fixes

- Make `gate-decision.json` either a true provenance-bearing authorization manifest or stop calling it the sole authorization artifact.
- Add required fields for `schema_version`, `generated_at`, `run_root`, command category, source command, calibration gate path/hash/status, serial baseline artifact path/hash, parallel smoke artifact path/hash, comparator status, review status, and identity hashes.
- Reorder audit "Next Engineering Actions" so early-abort timing closure and reconstruction sample rows are actions 1-2, or split earlier items into a separate non-blocking backlog.
- Define how to locate and accept the "latest accepted serial calibration."

##### Missing Tests

- A negative fixture where `gate-decision.json.full_e3_allowed=true` but `calibration-gate.json` is missing/stale/mismatched must block full E3.
- A stale-run-root or mismatched task-list hash fixture for gate decisions.
- A disk/start-gate abort fixture proving `suite-timing.json` and reconstruction sample rows exist on exit `3`.
- A comparator fixture where prompt/config/proof hashes are absent must block parallel acceptance, not silently pass.

##### Missing Logs / Observability

- Add a structured `gate_decision_evaluated` event with allowed category, blocking reasons, and artifact hashes.
- Add `early_abort_timing_artifact_written` on every exit `3` path.
- Add `calibration_artifact_accepted` / `calibration_artifact_rejected` with identity hashes and artifact paths.
- Add `review_gate_evaluated` before full E3 authorization.

##### Evidence

- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3227` - earlier gate schema was too small for sole authorization.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3554` - earlier wording called final `gate-decision.json` the only authorization.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:39` - earlier next-action ordering did not put early-abort timing first.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | Gate authorization is internally inconsistent and unsafe as written. | A stale or hand-built `gate-decision.json` could authorize full E3 without referenced calibration/review artifacts. | blocking | accept | The previous schema only carried high-level booleans and identity hashes. | Updated Section 16.13.8 to define `gate-decision.json` as a provenance-bearing authorization manifest with schema version, generation time, run root, source command, calibration gate path/hash/status, serial baseline path/hash/status, parallel smoke path/hash/comparator status, review gate path/hash/status, predecessor artifact hashes, and mandatory hash verification before full E3. | Round 2 closure review required. |
| documentation-skill-adversary | The audit does not make early-abort timing closure clearly first. | A fresh engineer could follow the audit action list and work on Docker/cache/comparator gaps before the known early-abort timing blocker. | blocking | accept | The audit action list placed Docker/cache and comparator items before Section 16.14 execution order. | Reordered `Next Engineering Actions` so early-abort timing artifact closure and reconstruction sample-row correctness are actions 1-2; moved Docker/cache/comparator/legacy importer work after those, with an explicit backlog note. | Round 2 closure review required. |
| documentation-skill-adversary | `prompt/config hashes when available` weakens comparator coverage. | Optional hash wording can allow parallel acceptance with missing required identity fields. | non-blocking | accept | The audit already records comparator coverage as incomplete. | Kept as follow-up in audit backlog: absence of required hashes must block parallel acceptance rather than silently pass. | Implement in comparator closure step. |
| documentation-skill-adversary | Latest accepted serial calibration lacks local provenance definition. | `time_saved_estimate_ms` could cite an ambiguous or stale baseline. | non-blocking | accept | Provenance-bearing gate now requires serial baseline path/hash/status; detailed discovery implementation still belongs in calibration tooling. | Added serial baseline path/hash/timing quality to gate schema. | Add negative stale-baseline fixture during gate implementation. |
| documentation-skill-adversary | Missing tests for stale/mismatched gate, early abort, and comparator absent hashes. | Plan could pass documentation review but implementation can still regress. | non-blocking | accept | These are test requirements for implementation phases. | Covered in the updated gate schema and audit next actions; implementation remains open. | Add tests during implementation. |
| documentation-skill-adversary | Missing observability events for gate and calibration decisions. | Operators may not diagnose why a command was allowed or blocked. | non-blocking | accept | This is an implementation logging requirement. | Accepted as implementation follow-up. | Add structured events during gate implementation. |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: closure review pending
- Allowed to proceed: no

## Final Conclusion

Round 1 found accepted blocking issues. The plan and audit were updated, and a closure review is required before this review can pass.

## Round 2: Blocking Closure Review

### Review Input

#### Objective

Verify whether the two accepted Round 1 blocking findings are closed.

#### Review Target

Documentation closure review for the E3 speedup plan and completion audit.

#### Target Locations

- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md`
- `vs_review/2026-06-15-e3-speedup-plan-review.md`

#### Change Introduction

Section 16.13.8 was updated to define `gate-decision.json` as a provenance-bearing authorization manifest with required referenced artifact paths, hashes, statuses, and verification rules. The audit `Next Engineering Actions` were reordered so early-abort timing closure and runtime reconstruction sample-row correctness are actions 1-2.

#### Risk Focus

- Full E3 could still be authorized by a stale or hand-built gate decision.
- Audit action order could still let Docker/cache/comparator/legacy work jump ahead of the known early-abort timing blocker.

#### User-Perspective Review Focus

- A fresh operator should know that full E3 requires verified provenance, not just booleans.
- A fresh engineer should know the first two implementation tasks.

#### Assumptions To Attack

- The accepted blocking findings are now actually closed.
- The new wording is not self-contradictory.

#### Adversarial Lenses

- documentation
- release-ops
- observability
- comprehension

#### Verification Status

- Documentation changes have been applied.
- No code implementation or full E3 run is part of this closure review.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary | Closure of documentation executability and authorization-gate findings. | fresh-session executability, hidden assumptions, ambiguous gates |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | multi_agent_v1.spawn_agent | 019ec794-42bb-7650-b119-3fdf5f18585d | spawn_agent tool result | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| reviewer-output-2 | documentation-skill-adversary | 1 | 019ec794-42bb-7650-b119-3fdf5f18585d | 10 minutes | completed | reviewer completed | completed |

### Reviewer Outputs

#### reviewer-output-2

##### Summary

The two accepted Round 1 blocking findings are closed for the requested documentation closure scope.

##### Blocking Findings

- none

##### Non-blocking Risks

- Implementation proof is still pending: the docs require stale/missing/hash-mismatched gate fixtures, early-abort timing fixtures, and comparator coverage, but this review only checked whether the documentation now closes the two accepted blockers.
- The completion audit still marks the broader implementation as partial/not complete, which is correct and not a closure blocker.

##### User-Perspective Checks

- Usability: pass - a fresh operator is now told that a bare or hand-written `full_e3_allowed=true` is not authorization.
- Ease of use: pass - a fresh engineer now sees early-abort timing closure and runtime reconstruction sample-row correctness as actions 1-2 before Docker/cache/comparator/legacy work.
- Ease of understanding: pass - the closure wording separates documentation closure from pending implementation proof.

##### Required Fixes

- none

##### Missing Tests

- stale/missing/mismatched `gate-decision.json` referenced artifact fixture
- stale run-root/task-list mismatch fixture
- disk/start-gate abort fixture proving `suite-timing.json` and reconstruction sample rows on exit `3`

##### Missing Logs / Observability

- `gate_decision_evaluated`
- `early_abort_timing_artifact_written`
- `calibration_artifact_accepted/rejected`
- `review_gate_evaluated`

##### Evidence

- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3219` - `gate-decision.json` is valid only with provenance for every artifact that justified the decision.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3221` - bare/hand-written booleans, stale gates, and missing/hash-mismatched artifacts do not authorize the next E3 step.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3243` - schema requires calibration gate path/hash/status.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3248` - schema requires serial baseline path/hash/status/timing quality.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3254` - schema requires parallel smoke path/hash/score/comparator status.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3260` - schema requires review gate path/hash/status.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3276` - runner must verify paths and recompute hashes before accepting `full_e3`.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3292` - full E3 is forbidden unless gate, calibration, serial/parallel hashes, review gate, and identity checks pass.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:39` - backlog must not jump ahead of the first two actions.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:41` - action 1 is early-abort timing artifact closure.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:42` - action 2 is runtime reconstruction sample-row correctness.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:43` - Docker/validator/proof/audit/report fast-fail work starts only at action 3.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:52` - Docker cache, comparator coverage, and legacy importer are explicitly backlog after actions 1-2.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | Round 1 blocking findings closed for documentation closure scope. | n/a | n/a | accept | Closure reviewer found no blocking findings for the two closure questions. | Mark review passed for documentation scope. | Implementation proof remains pending in the plan and audit. |
| documentation-skill-adversary | Implementation proof still pending. | Documentation closure does not prove code, tests, logs, or full E3 runtime behavior. | non-blocking | accept | This is already represented in the completion audit as partial/not complete. | Keep implementation tasks open. | Implement stale gate fixtures, early-abort timing fixtures, comparator coverage, and structured gate logs during engineering work. |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - documentation-skill-adversary / 019ec794-42bb-7650-b119-3fdf5f18585d
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: n/a
- Allowed to proceed: yes
