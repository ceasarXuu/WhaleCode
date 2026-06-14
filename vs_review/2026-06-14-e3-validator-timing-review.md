# Subagent VS Review: E3 Validator Timing And Runtime Fields

- Created: 2026-06-14T18:53:39+08:00
- Updated: 2026-06-14T19:31:00+08:00
- Task: Execute the TaskSpace E3 Harness Guardrails Implementation Plan.
- Report path: `vs_review/2026-06-14-e3-validator-timing-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Validator timing implementation review

### Review Input

#### Objective
Validate whether the latest E3 validator timing and runtime-reduction support is correct enough to proceed under the TaskSpace E3 harness guardrails plan.

#### Review Target
Code implementation, test strategy, observability, and plan alignment for the latest validator timing phase fields.

#### Target Locations
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/lib/timing.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/test-e3-score-validity.ps1`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The implementation adds Docker build/run/inspect/cleanup phase duration capture to the generated terminal-bench validator, propagates these fields into benchmark metrics, adds a `validation_timeout_phase` classification, and introduces a `ValidationPretestTimeoutSeconds` runner parameter. The plan document section 15.5 describes validator and Docker overhead reduction, including pretest timeout split and Docker phase timing.

#### Risk Focus
- Whether `ValidationPretestTimeoutSeconds` actually enforces the pretest timeout split described by the plan, or only affects probe execution.
- Whether timeout phase classification is reliable when validator output is buffered, missing, truncated, or killed.
- Whether generated PowerShell validator code is syntactically valid and records Docker phases correctly on build/run/inspect failures.
- Whether cleanup timing is correctly aggregated without double counting or null-cast surprises.
- Whether the added tests prove real runner/adapter behavior rather than only helper-level fields.
- Whether docs or artifacts overclaim runtime reduction readiness.

#### Verification Status
- Latest known passing tests before this review: `test-e3-score-validity.ps1`, `test-terminal-bench-adapter-harness.ps1`, `test-terminal-bench-uv-cache-harness.ps1`, `test-e3-harness-guardrails.ps1`, `test-e3-start-gate.ps1`, `test-harness.ps1`, and `git diff --check`.
- Known unverified area: no fresh adversarial review has been done for the validator timing commit; true full public-validation pretest/test timeout split may not be implemented.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return findings in this structure: summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs or observability, evidence.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | The change spans generated validator code, timing aggregation, and runner parameters. | Correctness, edge cases, generated PowerShell behavior |
| test-validity-adversary | The current proof may be helper-level and not exercise actual runner/adapter semantics. | Self-deceptive tests, missing black-box coverage |
| observability-adversary | The goal is to diagnose multi-hour E3 bottlenecks and prevent wasted runs. | Logs, metrics, phase attribution, actionable artifacts |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | multi_agent_v1.spawn_agent | 019ec5c5-4883-7bf1-8b69-a259a67a5d93 / Archimedes | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | multi_agent_v1.spawn_agent | 019ec5c5-c4f9-7be0-9ff5-854854803c93 / Anscombe | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| observability-adversary | multi_agent_v1.spawn_agent | 019ec5c6-1a70-7a82-baef-1e3d90f764c5 / Halley | spawn_agent result in current Codex thread | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary / Archimedes

##### Summary
Not ready to proceed as runtime-reduction support. Docker phase duration capture is plausible, but the true pretest/test timeout split is not implemented, timeout phase classification is unreliable on killed validators, cleanup is unbounded, and several observability fields are placeholders or absent.

##### Blocking Findings
- `ValidationPretestTimeoutSeconds` only limits `-ProbeOnly`, not real public validation.
- `validation_timeout_phase` can misclassify a tests-started timeout as `pretest` because timeout kills can lose buffered stdout/stderr markers.
- Generated validator cleanup directly runs Docker `rm` and `rmi` without its own timeout.

##### Non-blocking Risks
- `docker_observed_duration_ms` undercounts by using first and last phase finish timestamps.
- `validator_probe_duration_ms` is always null.
- Planned `tests_started_at` and `tests_completed_at` fields are not emitted.

##### Required Fixes
- Implement real two-stage validation timeout.
- Make marker capture durable during timeout.
- Add bounded cleanup execution or delegate cleanup to a bounded runner path.
- Fix `docker_observed_duration_ms` semantics.

##### Missing Tests
- Synthetic pre-marker timeout, post-`tests_started` timeout, cleanup hang/failure, generated validator parse/runtime checks, and `docker_observed_duration_ms` coverage.

##### Missing Logs / Observability
- Probe duration, tests-started/completed timestamps, and cleanup timeout classification.

##### Evidence
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:242`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:362`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:364`
- `scripts/action-map-real-user-e2e-lib.ps1:54`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1:26`
- `scripts/taskspace-benchmark/lib/timing.ps1:94`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:504`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:1151`

#### test-validity-adversary / Anscombe

##### Summary
Not ready to proceed. Current tests prove helper assignment only, not real runner/adapter behavior for score-bearing validation timeout splitting or reliable timeout classification.

##### Blocking Findings
- `ValidationPretestTimeoutSeconds` gates only the probe path, not score-bearing validation.
- `validation_timeout_phase` is unreliable for real killed validators because stdout/stderr are written only after normal exit in the old process helper.
- Added tests hand-construct metrics and fake Docker results; they do not exercise generated validator output, process timeout capture, or runner timeout splitting.

##### Non-blocking Risks
- Docker phase capture has no parser/integration test for freshly materialized generated PowerShell.
- Cleanup timing from timeout cleanup can be absent.
- `docker_observed_duration_ms` excludes the first phase duration.

##### Required Fixes
- Implement actual two-stage validation timeout.
- Persist stdout/stderr incrementally or otherwise durably during execution.
- Add `ValidationTestTimeoutSeconds` or make naming honest.
- Make timeout classification fail-closed if marker visibility is untrusted.

##### Missing Tests
- Generated or runner-level pretest timeout, post-marker timeout, adapter parse/materialization, and proof that `ValidationPretestTimeoutSeconds` affects score-bearing validation.

##### Missing Logs / Observability
- `tests_started_at`, `tests_completed_at`, pair report surfacing, cleanup command duration and timeout fields.

##### Evidence
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:242`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:362`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:364`
- `scripts/action-map-real-user-e2e-lib.ps1:54`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1:23`
- `scripts/taskspace-benchmark/test-e3-score-validity.ps1:161`
- `scripts/taskspace-benchmark/test-e3-score-validity.ps1:177`

#### observability-adversary / Halley

##### Summary
Not ready to proceed. Target scripts and extracted generated validator parse cleanly, but the two core guardrails are missing: real pretest timeout splitting and reliable timeout phase classification.

##### Blocking Findings
- `ValidationPretestTimeoutSeconds` only gates `-ProbeOnly`.
- `validation_timeout_phase` cannot reliably classify real timeouts because killed validators can lose stdout markers.
- Generated validator cleanup is not bounded; runner cleanup is bounded only after the validator process returns or is killed.

##### Non-blocking Risks
- `docker_observed_duration_ms` is misleading.
- `validator_probe_duration_ms` is always null.
- Docs show planned parallel flags that the suite runner does not accept yet.

##### Required Fixes
- Enforce true two-stage validation timeout with durable `tests_started` signal.
- Bound generated-validator cleanup or remove cleanup from generated validator and rely on bounded runner cleanup.
- Correct `docker_observed_duration_ms`.

##### Missing Tests
- Pre-marker timeout, post-marker timeout, generated-validator cleanup hang, and generated validator phase artifact smoke.

##### Missing Logs / Observability
- Durable timeout phase marker, tests timestamps, probe duration, and cleanup timeout duration/classification.

##### Evidence
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:242`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:362`
- `scripts/action-map-real-user-e2e-lib.ps1:47`
- `scripts/action-map-real-user-e2e-lib.ps1:54`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:503`
- `scripts/taskspace-benchmark/lib/timing.ps1:113`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:1391`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-adversary | `ValidationPretestTimeoutSeconds` only gates probe | blocking | accept | Public validation previously passed only `$effectiveValidationTimeout` | Added `ValidationTestTimeoutSeconds`; score-bearing validation now calls `Invoke-TaskspaceValidationCommand` with pretest and test budgets | Closure review required |
| implementation-adversary | Timeout phase can lose marker on kill | blocking | accept | Old helper wrote stdout/stderr only after normal process exit | Replaced validation execution with file-backed polling watchdog; writes `taskspace_validation_timeout_phase` and durable tests timestamps | Closure review required |
| implementation-adversary | Generated validator cleanup is unbounded | blocking | accept | Generated validator directly called Docker cleanup in `finally` | Removed generated cleanup; cleanup is deferred to bounded runner cleanup path | Closure review required |
| implementation-adversary | `docker_observed_duration_ms` undercounts | non-blocking | accept | It used first/last finish timestamp | Changed calculation to earliest `started_at` and latest `finished_at` | Covered by score-validity test path |
| implementation-adversary | Missing probe/tests timing fields | non-blocking | accept | `validator_probe_duration_ms` was null; tests timestamps absent | Added probe duration from runner and lifecycle timestamp markers | Closure review required |
| test-validity-adversary | Tests were helper-level only | blocking | accept | Existing test hand-constructed metrics | Added runner-level tests for pretest fail-fast and post-marker timeout classification in `test-oracle-runner-harness.ps1` | Closure review required |
| observability-adversary | Docs showed unimplemented parallel flags | non-blocking | accept | Suite runner does not accept those flags | Updated plan to mark them future Phase R3 contract, not current CLI | Closure review required |

Validation evidence after fixes:
- `.\scripts\taskspace-benchmark\test-oracle-runner-harness.ps1` passed at 2026-06-14T19:21+08:00.
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1` passed at 2026-06-14T19:22+08:00.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: pending closure review
- Blocking re-review completed: no
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no, closure review required

## Round 1 Interim Conclusion

Pending closure review.

## Round 2: Closure review for accepted blocking findings

### Review Input

#### Objective
Verify whether accepted blocking findings from Round 1 are closed.

#### Review Target
Closure of implementation, tests, cleanup, and observability fixes for validator timing.

#### Target Locations
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/lib/harness-health.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/timing.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/test-oracle-runner-harness.ps1`
- `scripts/taskspace-benchmark/test-e3-score-validity.ps1`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md`

#### Change Introduction
The main agent accepted all Round 1 blocking findings and implemented runner-level pretest/test timeout splitting, file-backed lifecycle marker polling, bounded process-tree termination, runner-delegated Terminal-Bench cleanup, cleanup duration artifacts, probe duration, corrected Docker observed duration, and runner-level timeout tests.

#### Risk Focus
- Public validation still not passing pretest/test budgets.
- Process tree not killed, causing false fail-fast.
- Marker polling not durable enough to classify killed post-`tests_started` validators.
- `cmd.exe` quoting regressions.
- Cleanup still unbounded.
- Tests proving only helper-level behavior.
- Docs overclaiming unimplemented Docker cache or parallelism.

#### Verification Status
- `.\scripts\taskspace-benchmark\test-oracle-runner-harness.ps1` passed.
- `.\scripts\taskspace-benchmark\test-e3-score-validity.ps1` passed.
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1` passed.
- `.\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1` passed.
- `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1` passed.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Return findings in this structure: summary, blocking findings, non-blocking risks, required fixes, missing tests, missing logs or observability, evidence, closure verdict.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation/test closure adversary | Closure spans implementation semantics and tests; one focused reviewer is enough for accepted-finding closure. | False closure, runner behavior, test strength |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation/test closure adversary | multi_agent_v1.spawn_agent | 019ec5e1-4842-75c0-ba7b-43b3c10f9f58 / Gibbs | spawn_agent result in current Codex thread | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation/test closure adversary / Gibbs

##### Summary
Read-only closure review completed. No remaining blocking false closure was found for the four accepted Round 1 blocking findings. The verdict is based on source/report inspection and the main agent's reported local passes.

##### Blocking Findings
- None.

##### Non-blocking Risks
- `docker_observed_duration_ms` was under-tested.
- Process-tree kill uses `taskkill /T /F`; tests prove a simple sleeping PowerShell process times out but do not prove child/grandchild cleanup.
- Cleanup observability is adequate for closure but coarse: aggregate duration and exit codes, not per-command duration/timed-out fields.

##### Required Fixes
- No blocking fixes required before closing accepted Round 1 blockers.

##### Missing Tests
- Direct assertion for `docker_observed_duration_ms` with generated-style phases.
- Validation timeout test with spawned child process cleanup.
- Fake Docker cleanup command that hangs and proves bounded duration plus classification.

##### Missing Logs / Observability
- Consider per-cleanup-command `duration_ms`, `timed_out`, and stderr snippet in `validation-cleanup-result.json`.

##### Evidence
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:369`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1:129`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:143`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1:73`
- `scripts/taskspace-benchmark/lib/harness-health.ps1:271`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1:302`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1:503`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1:206`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1:371`
- `scripts/taskspace-benchmark/test-oracle-runner-harness.ps1:48`
- `scripts/taskspace-benchmark/test-oracle-runner-harness.ps1:68`
- `docs/plans/2026-06-13-taskspace-e3-harness-guardrails-implementation-plan.md:1174`

##### Closure Verdict
Accepted Round 1 blocking findings are closed. Proceed, with non-blocking test and observability gaps tracked.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Gibbs | No remaining blocking false closure | blocking | accept | Closure reviewer found no blockers | Proceed with final validation | n/a |
| Gibbs | `docker_observed_duration_ms` under-tested | non-blocking | accept | Cheap to cover directly | Added generated-style phase fixture and assertion in `test-e3-score-validity.ps1`; reran and passed | n/a |
| Gibbs | Process-tree grandchild cleanup not directly tested | non-blocking | defer | Accepted blockers are closed; broader process tree fixture can be added without blocking current repair | Track as future hardening | Add child/grandchild timeout fixture before relying on broader process families |
| Gibbs | Cleanup observability is coarse | non-blocking | defer | Current runner cleanup is bounded and writes aggregate duration/classification; per-command durations are an observability improvement | Track as future hardening | Add per-command cleanup timing fields |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `019ec5e1-4842-75c0-ba7b-43b3c10f9f58 / Gibbs`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes

## Final Conclusion

Passed. The accepted Round 1 blocking findings are closed after implementation, local validation, and fresh closure review.
