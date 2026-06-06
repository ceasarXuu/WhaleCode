# Subagent VS Review: TaskSpace P0 Benchmark Harness Implementation

- Created: 2026-06-07T05:48:00+08:00
- Updated: 2026-06-07T06:20:00+08:00
- Report schema: adversarial-v1
- Task: review the implementation of the P0 benchmark harness repair.
- Status: closure review pending

## Round 1: Implementation Review

### Review Input

Objective: challenge the uncommitted implementation against `docs/testing/2026-06-07-taskspace-p0-benchmark-harness-repair-plan.md`.

Target locations:

- `scripts/taskspace-benchmark/lib/prompt-guard.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/run-state.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/adapters/external-benchmark-common.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `coe/2026-06-07-05-18-taskspace-p0-benchmark-harness.md`

Risk focus:

- prompt allowlist provenance safety
- remote asset preflight/cache/equivalence proof
- metrics taint propagation into evidence gate
- run-state observability and resumability
- Docker result classification
- test coverage for the repaired paths

Reviewer instructions: fresh read-only internal subagent, no inherited main-agent context, inspect files directly, cite evidence paths/lines where possible.

### Reviewer Launch Records

| Reviewer | Mechanism | Session / Job ID | Context Forked | Read-only |
|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9ee4-94cb-7100-8fc2-30de617b1529` | false | yes |

### Reviewer Output

Summary: the first implementation covered only part of the repair contract. It moved `multi-agent` out of hard fail, propagated critical metrics taints, and stopped unproven remote URL samples before Whale execution, but it did not fully satisfy the plan.

Blocking findings:

1. Prompt allowlist provenance was incomplete and natural-language control prompts such as `spawn subagents and bind node` were not rejected.
2. Remote asset preflight only discovered URLs and always marked them unproven. It lacked expected checksum, cache verification, injection, transform diff, and post-injection tree hash.
3. Run-state was not recoverable or fully auditable. It lacked sample list, env snapshot, lock/heartbeat/stale policy, resume decision, and audit/finalize boundary.
4. Docker result classification was written as a side artifact only and did not enter metrics, evidence gate, or aggregate denominator.

Non-blocking risks:

- Metrics critical artifact detection was hardcoded and did not include all plan metadata.
- Aggregate denominator split was pair-level and string-match based.
- COE said repair implemented before the blocking implementation gaps were closed.

Missing tests/logs:

- actual `recover-accuracy-log` prompt test
- cached asset injection and checksum mismatch tests
- resume phase tests
- Docker taxonomy tests
- audit draft cannot enter aggregate without explicit completion
- event tests for validator result, metrics taint, audit, resume, final inclusion/exclusion

### Main Agent Response

Accepted all blocking findings.

Implemented responses:

1. Prompt guard now rejects natural-language `spawn subagents` / `bind node`, and source-hit details include line, byte, raw sha, and adapted sha fields. Terminal-Bench adapter now writes prompt source spans with upstream and adapter wrapper provenance.
2. Terminal-Bench remote asset handling now extracts nearby expected SHA-256, verifies local cache content, rewrites Dockerfile `curl -o <path> <url>` into `COPY .wra/... <path>` when proof is available, records transform diff, and records post-injection tree hash. Unproven assets remain preflight-ineligible.
3. Run-state now records sample list, env snapshot, lock owner, heartbeat, stale policy, resume decision, phase_started_at, and audit status. E3 aggregate final readiness remains false when audit is pending.
4. Docker validator wrapper writes `docker-build-result.json`; metrics parse it into `validator_environment_failures`; evidence gate receives those failures and aggregate denominator can see environment/Docker exclusions.
5. Tests were expanded for natural prompt control rejection, prompt spans, cached remote asset injection, locked critical metrics taints, run-state fields, and Docker result parsing.

Validation:

- `test-harness.ps1`: PASS
- `test-oracle-runner-harness.ps1`: PASS
- `test-terminal-bench-uv-cache-harness.ps1`: PASS
- `test-e3-proof-harness.ps1`: PASS
- remote asset preflight smoke: expected exit 2 translated to test success
- real one-pair `single-file-fast-fix` smoke: exit 0, run/sample status complete, pair report generated

Rejected findings: none.

Deferred findings:

- Full phase-by-phase resume implementation remains partial. Current status artifacts contain resume metadata and events, but automatic resume/skip orchestration is not complete. This should remain a known limitation before claiming full P0 orchestration maturity.
- Full Docker taxonomy still starts from structured phase classifications. Fine-grained inference such as base-image vs apt vs proxy vs remote-asset fetch requires output parsing in a later hardening pass.

## Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: partial to substantial
- Blocking closure review completed: pending
- Allowed to claim full clean P0/E3 readiness: no

## Round 2: Blocking Closure Review

### Closure Review Input

Objective: verify whether the accepted Round 1 blocking findings have been sufficiently addressed after closure hardening.

Target locations:

- `scripts/taskspace-benchmark/lib/prompt-guard.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-remote-assets.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-equivalence.ps1`
- `scripts/taskspace-benchmark/lib/run-state.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`

Validation already run:

- `test-harness.ps1`: PASS
- `test-e3-proof-harness.ps1`: PASS
- remote asset preflight smoke: expected ineligible path
- real one-pair smoke: completed and wrote pair report

### Closure Reviewer Launch Records

| Reviewer | Mechanism | Session / Job ID | Context Forked | Read-only | Status |
|---|---|---|---|---|---|
| closure-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9ef8-4d93-7e43-9ca0-26a59132b3d7` | false | yes | completed |

### Closure Reviewer Output

Summary: prompt guard provenance/control rejection, remote asset preflight/cache/injection proof, and Docker classification into metrics/evidence/aggregate were substantially closed. The remaining blocking issue was run-state/resume: the implementation recorded state fields but did not yet consume existing status/artifacts to resume work.

Blocking findings:

1. Run-state recoverability was still incomplete. The runner always created a new run and did not expose `-ResumeLatest`, `RunId`, or `Force` semantics; tests only checked fields existed.

Non-blocking risks:

- Remote asset checksum binding is heuristic and should later be tied to explicit URL/destination/hash declarations.
- Docker taxonomy is still coarse and can be further split into base image, apt, proxy, and remote-asset fetch cases.
- The review artifact needed this closure result written back to the repo.

Closure verdict before main-agent response: partial, not closed.

### Closure Main Agent Response

Accepted the run-state blocker.

Implemented response:

1. Added real resume entrypoints to `run-taskspace-benchmark.ps1`: `-ResumeLatest`, `-RunId`, and `-ForceRerun`.
2. Added run-state readers and latest-run lookup to `lib/run-state.ps1`.
3. Added stale-lock detection and `resume_requested` / `stale_lock_reclaimed` / `pair_skipped_completed` events.
4. Runner now reuses an existing run dir and skips completed pairs when `-ResumeLatest` or `-RunId` is used without `-ForceRerun`.
5. Added resume-related unit assertions in `test-harness.ps1`.
6. Ran a real two-step resume smoke: first normal one-pair run, then `-ResumeLatest`; the second invocation reused the same run dir and wrote `pair_skipped_completed` instead of rerunning the pair.

Validation after response:

- `test-harness.ps1`: PASS
- `test-terminal-bench-adapter-harness.ps1`: PASS
- `test-external-wrapper-harness.ps1`: PASS
- `test-e3-proof-harness.ps1`: PASS
- remote preflight smoke: expected ineligible path
- real resume smoke: same run dir reused, `pair_skipped_completed` event recorded

Remaining limitations:

- Full interrupted phase resume matrix is still future hardening. Current implementation closes completed-pair recovery and stale-lock observability, but does not yet resume from every internal phase with exact cursor semantics.

### Final Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes for completed-pair resume and stale-lock observability
- Blocking closure review completed: yes
- Allowed to claim full clean P0/E3 readiness: no, full P0 rerun still required
- Allowed to proceed with another P0 attempt: yes, with the stated limitation that phase-by-phase resume is not fully proven
