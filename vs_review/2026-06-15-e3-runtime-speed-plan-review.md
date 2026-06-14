# Subagent VS Review: E3 Runtime Speed Plan

- Created: 2026-06-15T03:05:00+08:00
- Updated: 2026-06-15T03:05:00+08:00
- Report schema: adversarial-v1
- Task: add the 15-task E3 runtime bottleneck and speedup execution plan to the v0.0.4 E3 guardrails plan.
- Report path: `vs_review/2026-06-15-e3-runtime-speed-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Runtime plan falsification

### Review Input

#### Objective
Challenge whether the new runtime bottleneck and speedup plan is executable enough to prevent another multi-hour invalid E3 run and to answer whether E3 can be materially sped up without changing the v0.0.4 scoring profile.

#### Review Target
Documentation and operational execution plan.

#### Target Locations
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md`

#### Change Introduction
The plan now includes Section 16.13, a detailed runtime bottleneck repair plan with hypotheses, phases R0-R5, validation gates, and hard stop rules. The completion audit now records this as documented but not implemented.

#### Risk Focus
- The plan may still allow another full E3 run before timing identity, engineering cleanliness, and speed evidence are ready.
- The plan may claim speedup paths without specifying enough artifacts or pass/fail criteria.
- The phase order may be wrong: parallelism or cache work could happen before basic instrumentation and invalid-run fast-fail are proven.
- The plan may not distinguish comparable v0.0.4 harness speedup from a new agent profile.
- The plan may miss operational stop conditions for disk, Docker, validator, or stale calibration artifacts.

#### User-Perspective Review Focus
- A future operator should understand exactly when to stop, what artifact to inspect, and what command category comes next.
- The plan should not require hidden conversation context to know whether a full E3 rerun is allowed.

#### Assumptions To Attack
- Section 16.13 is concrete enough for implementation tickets.
- The plan has measurable acceptance criteria for each phase.
- The audit status prevents "documented" from being mistaken for "implemented".
- The hard stop rules cover the failure classes observed in the previous E3 attempts.
- The plan can answer "can we significantly speed this up?" without running invalid full suites.

#### Adversarial Lenses
- release operations
- observability
- testing
- failure handling
- maintainability
- comprehension

#### Verification Status
- `git diff --check` passed with only Windows line-ending warnings.
- Section presence was checked with `Select-String`.
- No runtime E3 rerun was performed for this documentation change.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on blocking or major plan defects, not style.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release-ops-adversary | The highest risk is operational sequencing: avoiding another invalid multi-hour E3 and gating full reruns/speed claims correctly. | release/runbook gates, stop conditions, evidence readiness |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | `multi_agent_v1.spawn_agent` | `019ec765-634f-7b11-b1ee-45ed95921f20` | spawn_agent result nickname `Confucius` | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round1-release-ops | release-ops-adversary | 1 | `019ec765-634f-7b11-b1ee-45ed95921f20` | 10 minutes | completed | reviewer returned findings | completed |

### Reviewer Outputs

#### round1-release-ops

##### Summary
Section 16.13 is directionally strong, but release-ops gaps can still allow an expensive weakly authorized E3 path or speed claim from non-comparable evidence.

##### Blocking Findings
- Older run instructions still authorize a larger E3 run too early.
  - Broken assumption: Section 16.13 fully prevents another full or large invalid run before timing identity, cleanliness, and speed evidence are ready.
  - Failure scenario: An operator follows the earlier one-pair-smoke block and runs `SampleLimit 3 -RepeatCount 5 -ScoringMode` after one smoke, before R0-R5 completion.
  - Trigger condition: One-pair smoke passes or appears clean, then operator uses the older command block.
  - Impact: Substantial runtime can be spent and evidence can conflict with the stricter R5 gate.
  - Proof needed: Supersede the older command block so every multi-sample scoring run goes through Section 16.13/R5 with `calibration-gate.json status=pass`, `full_e3_allowed=true`, and matching identity.
- Direct-helper bypass remains a known operational escape hatch.
  - Broken assumption: The audit status prevents partial implementation from being mistaken for permission to run scoring E3.
  - Failure scenario: A future operator uses a direct helper or dev policy path instead of the canonical suite/start-gate path.
  - Trigger condition: The audit says start gate is implemented while noting direct helper policy and identity gaps.
  - Impact: A score-bearing run could start without real task-list/profile identity flowing through the gate.
  - Proof needed: Document direct helper as non-scoring/dev-only until it enforces the same gate, or require it to refuse scoring mode unless calibration identity is verified.
- Representative three-task serial calibration is underspecified.
  - Broken assumption: The plan has measurable acceptance criteria for each phase.
  - Failure scenario: Operators choose different representative subsets and use one to authorize a speed claim or full run.
  - Trigger condition: R5 requires a representative three-task serial calibration without a deterministic selection artifact.
  - Impact: Speed decisions can be cherry-picked or biased by task-family overhead differences.
  - Proof needed: Add `calibration-selection.json` with selected sample IDs, task-family rationale, source task-list hash, subset hash, and deterministic selection rule.

##### Non-blocking Risks
- R0 reconstruction output location is not constrained enough to guarantee historical artifacts are not rewritten.
- Hard stop rules are not command-facing; they do not name the next allowed command category.
- The audit next actions duplicate the governed parallel smoke step and may confuse sequencing.

##### User-Perspective Checks
- Full E3 allowance is clear in Section 16.13 R5, but not across the whole document because older text still exists.
- Comparable v0.0.4 speedup is distinguished from a new profile.
- Audit mostly prevents documented from being mistaken for implemented, but the direct-helper gap weakens this.

##### Required Fixes
- Supersede the older one-pair-smoke-only multi-sample command.
- Add a command-level gate table.
- Define `calibration-selection.json`.
- Document direct helper scoring mode as non-scoring-only unless it enforces the same identity gate.

##### Missing Tests
- Negative direct-helper scoring test without calibration identity.
- Stale calibration artifact mismatch fixture.
- Older one-pair-smoke-only path cannot authorize multi-sample scoring.
- Deterministic representative task-selection test.

##### Missing Logs / Observability
- `calibration-selection.json`.
- `gate-decision.json` with `next_allowed_command_category`.
- Stale-artifact fields including created time, git commit, task-list hash, profile hash, planned run root, and validity window.
- Separate R0 reconstruction output root.

##### Evidence
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:859` - older multi-sample command area.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3156` - R5 full E3 gate.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:17` - direct helper gap.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:27` - documented, not implemented status.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | Older run instructions authorize large E3 too early | Older command could be followed after one-pair smoke only. | blocking | accept | The plan contains stricter Section 16.13 R5 gates but older command text can still mislead operators. | Supersede older command block and add command-gate decision table. | Closure review Round 2 |
| release-ops-adversary | Direct-helper bypass remains operational escape hatch | Direct helper could be mistaken for score-bearing path without identity gate. | blocking | accept | Completion audit currently calls start gate implemented while direct helper remains a gap. | Strengthen audit and plan: direct helpers are non-scoring/dev-only until same identity gate is enforced. | Closure review Round 2 |
| release-ops-adversary | Representative three-task calibration underspecified | Different subsets can authorize biased speed evidence. | blocking | accept | R5 says representative three-task calibration but did not define selection artifact. | Add deterministic `calibration-selection.json` artifact and acceptance fields. | Closure review Round 2 |
| release-ops-adversary | R0 output root not constrained | Reconstruction could rewrite historical artifacts. | major | accept | R0 says read-only but output location was not explicit enough. | Add separate reconstruction output root requirement. | Closure review Round 2 |
| release-ops-adversary | Hard stop rules not command-facing | Operator may know to stop but not what command class is allowed next. | major | accept | Hard stop table lacked next-command category. | Add gate-decision artifact and allowed-command table. | Closure review Round 2 |
| release-ops-adversary | Audit duplicate parallel smoke step | Sequencing can be confusing. | minor | accept | Next actions duplicated the governed sample-parallel smoke. | Remove duplicate and make sequence single ordered list. | Closure review Round 2 |

### Closure Status

- Blocking findings found: pending
- Accepted blocking findings fixed: pending
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: pending
- Deferred findings documented: pending
- Blocked reason: pending
- Allowed to proceed: pending

## Round 2: Blocking Closure Review

### Review Input

#### Objective
Verify that the accepted Round 1 blocking findings are closed after documentation updates.

#### Review Target
Documentation and operational execution plan closure.

#### Target Locations
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md`
- `vs_review/2026-06-15-e3-runtime-speed-plan-review.md`

#### Change Introduction
The plan now marks the old one-pair-smoke-only multi-sample command as obsolete, adds `gate-decision.json`, adds `calibration-selection.json`, constrains direct helper entrypoints as non-scoring/dev-only until they enforce the same identity gate, constrains R0 reconstruction output, and updates the audit next actions.

#### Risk Focus
- The old command might still authorize multi-sample scoring too early.
- The direct-helper route might still be usable as score-bearing evidence.
- The representative three-task calibration might still be ambiguous or cherry-pickable.
- The new command-level gate might not be clear enough for operators.

#### User-Perspective Review Focus
- A future operator should know which command category is allowed next and when full E3 is forbidden.

#### Assumptions To Attack
- Round 1 blocking findings are actually fixed, not just restated.
- `gate-decision.json` and `calibration-selection.json` are concrete enough to implement.
- The audit no longer implies full completion or broad start-gate coverage.

#### Adversarial Lenses
- release operations
- failure handling
- observability
- testing
- comprehension

#### Verification Status
- `git diff --check` passed with only Windows line-ending warnings.
- `Select-String` verified new artifact names and constraints are present.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers where possible.
- Focus only on whether Round 1 blocking findings are closed or still blocking.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 10 minutes | none | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release-ops-adversary | Same operational sequencing risk as Round 1; this is a closure review. | release/runbook gates, stop conditions, evidence readiness |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | `multi_agent_v1.spawn_agent` | `019ec769-bcb7-7431-92f0-efb06e1a07d0` | spawn_agent result nickname `Boyle` | fork_context=false | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| round2-release-ops | release-ops-adversary | 1 | `019ec769-bcb7-7431-92f0-efb06e1a07d0` | 10 minutes | completed | reviewer returned closure result | completed |

### Reviewer Outputs

#### round2-release-ops

##### Summary
Round 2 closure review found the three accepted Round 1 blocking findings closed. No remaining blocking authorization path was found that lets an operator treat one-pair smoke, direct helpers, or an ambiguous three-task subset as score-bearing full E3 evidence.

##### Blocking Findings
- none

##### Non-blocking Risks
- The old multi-sample command is still physically present, but it is clearly fenced as non-release/non-scoring unless R5 has already passed.
- The completion audit had broad shorthand saying full E3 needs `calibration-gate.json status=pass`; the stronger full condition appears later with `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, and identity checks.

##### User-Perspective Checks
- Usability: pass - allowed command category is explicit through `gate-decision.json`.
- Ease of use: pass - full 15-task E3 is explicitly forbidden unless `next_allowed_command_category=full_e3`, `full_e3_allowed=true`, speed-claim state is correct, and task-list/source/profile identity matches.
- Ease of understanding: pass - direct helpers are non-scoring/dev-only until they enforce the same identity gate; representative three-task calibration requires deterministic `calibration-selection.json`.

##### Required Fixes
- none

##### Missing Tests
- No new blocking test gap for documentation closure. Future implementation tests remain tracked in the plan.

##### Missing Logs / Observability
- No blocking observability gap for documentation closure. The plan now requires `gate-decision.json`, `calibration-selection.json`, separate R0 reconstruction output, and task-list/source/profile identity hashes.

##### Evidence
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:859` - old command superseded and fenced.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3017` - direct helper constrained as non-scoring/dev-only until identity gate parity.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3180` - `calibration-selection.json` fields defined.
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:3217` - `gate-decision.json` and command categories defined.
- `docs/plans/2026-06-15-e3-guardrails-completion-audit.md:31` - audit keeps implementation partial and blocks full E3/speed claims.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | Round 1 blocking closure | Old command, direct helper, and representative calibration could have remained unsafe. | blocking | accept | Round 2 found no remaining blocking authorization path. | Closure accepted. The audit no-go shorthand was also tightened to include `gate-decision.json`, `full_e3_allowed=true`, and identity checks. | none |
| release-ops-adversary | Old command still physically present | Operator must read warning around old command. | minor | accept | The command is fenced as non-release/non-scoring unless R5 passes. Keeping it as dev/debug reference is acceptable. | No further action. | none |
| release-ops-adversary | Audit shorthand weaker than final gate | Earlier no-go bullet could be read too broadly. | minor | accept | Tightened no-go condition to match command gate and identity requirements. | Updated completion audit. | none |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - release-ops-adversary via `multi_agent_v1.spawn_agent`, session `019ec769-bcb7-7431-92f0-efb06e1a07d0`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Passed for documentation closure. Section 16.13 is now a usable engineering plan for runtime bottleneck diagnosis and guarded speedup work, but implementation remains partial and full E3 execution remains blocked until the documented gates are implemented and satisfied.
