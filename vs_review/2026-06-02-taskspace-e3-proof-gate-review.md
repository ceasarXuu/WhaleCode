# Subagent VS Review: TaskSpace E3 Proof Gate

- Created: 2026-06-02T23:51:35+08:00
- Updated: 2026-06-03T00:08:00+08:00
- Report schema: adversarial-v1
- Task: harden TaskSpace E3 external benchmark evidence so external results cannot be promoted from manifest assertions alone.
- Report path: `vs_review/2026-06-02-taskspace-e3-proof-gate-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Proof Gate And External Runner Review

### Review Input

#### Objective
Ensure external E3 benchmark results are gated by runtime proof, validator-source isolation proof, repeated runs, aggregate generation, and artifact audit review rather than by static manifest claims.

#### Review Target
Implementation, tests, and documentation for the TaskSpace E3 external benchmark proof chain.

#### Target Locations
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/audit-report.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-external.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/lib/workspace.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `benchmarks/taskspace/external/README.md`

#### Change Introduction
The change adds per-pair external proof artifacts, makes external E3 gate decisions use runtime proof instead of manifest-only fidelity booleans, emits audit-review templates with artifact hashes, adds a strict E3 external runner entrypoint, and adds Docker validator runtime markers for Terminal-Bench.

#### Risk Focus
- False E3 promotion from manifest declarations or stale artifacts.
- Validator source or hidden tests becoming readable by the agent.
- Docker `/app` runtime proof conflating environment fidelity with business validation success.
- Audit templates creating false confidence or stale hashes.
- Windows/WSL/Docker path and readonly mount proof gaps.
- PowerShell argument forwarding and wrapper behavior.

#### Assumptions To Attack
- Runtime proof generated after execution can still prove the agent never saw validator source.
- Token/path scans are enough to detect validator source leakage.
- E3 external entrypoint consistently enforces repeats, aggregate generation, and source pinning.
- Terminal-Bench post-hoc Docker validation remains downgraded until official/equivalent runner proof exists.
- New tests catch false-positive E3 promotion and wrapper parameter regressions.

#### Adversarial Lenses
- requirements
- data and isolation
- failure
- maintenance
- testing
- observability

#### Verification Status
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `scripts\taskspace-benchmark\run-taskspace-e3-external.ps1 -PlanOnly` passed on Terminal-Bench `hello-world`.
- Real diagnostic Terminal-Bench pair ran with `run-taskspace-external-benchmark.ps1 -Repeats 1 -EnableAggregate -AllowDiagnosticNonTargetResult`.
- Diagnostic output generated `external-runtime-proof.json`, `external-isolation-proof.json`, `external-e3-proof.json`, `audit-review.suggested.json`, and a pair report that remained `E2-candidate`.
- Docker residual check with `D:\whale-docker\bin\docker.cmd` found no labeled containers or images.
- `git diff --check` passed.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Attack the proof and gate assumptions rather than confirming the implementation.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | 10 minutes once if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| E3 evidence gate reviewer | The main risk is false E3 promotion or misleading evidence boundaries. | evidence gate, audit chain, proof semantics |
| Windows/Docker boundary reviewer | The implementation depends on Windows paths, WSL Docker, `/app` aliasing, and container marker parsing. | Docker runtime, path conversion, source isolation, PowerShell wrappers |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| E3 evidence gate reviewer | `multi_agent_v1.spawn_agent` | `019e8908-078a-7902-8c2b-22c8fe729a10` | spawn_agent result | false | Round 1 Review Input plus target paths | main-agent history, reasoning, drafts, conclusions, full diff unless read from repo | yes |
| Windows/Docker boundary reviewer | `multi_agent_v1.spawn_agent` | `019e8908-4b56-7bf0-a018-31451f3c2672` | spawn_agent result | false | Round 1 Review Input plus target paths | main-agent history, reasoning, drafts, conclusions, full diff unless read from repo | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| evidence-gate | E3 evidence gate reviewer | 1 | `019e8908-078a-7902-8c2b-22c8fe729a10` | 15 minutes | completed | returned structured findings | completed |
| docker-boundary | Windows/Docker boundary reviewer | 1 | `019e8908-4b56-7bf0-a018-31451f3c2672` | 15 minutes | completed | returned structured findings | completed |

### Reviewer Outputs

#### evidence-gate

##### Summary
The current Terminal-Bench adapter was not promoting itself to E3, but the first implementation was not safe enough for future E3 promotion because proof and audit semantics were too weak.

##### Blocking Findings
- Source-isolation proof could pass when validator source was missing and was only post-hoc.
- Runtime fidelity was mostly log-string proof plus manifest assertion.
- Business success aggregation used one-sided success as generic `business_success`.
- Audit completion was too shallow for E3 trust.

##### Non-blocking Risks
- `SourceVersion` was only checked for non-empty text.
- Pair reports mixed declared fidelity and proof fields.
- `-AllowCandidateResult` is diagnostic and must not be used as an E3 pass path.

##### Required Fixes
- Fail closed when validator source is blank or missing and compare validator source tree hash.
- Add stronger wrapper/runtime proof fields.
- Split side outcomes and require audit decision consistency.
- Require typed external audit attestations.
- Verify pinned source revision when possible.

##### Missing Tests
- Missing validator source must not prove isolation.
- Runtime marker spoofing must not prove runtime.
- One-sided outcome must not satisfy `include_no_clear_delta`.
- External audit without proof attestations must fail.

##### Missing Logs / Observability
- Validator source tree hash in proof.
- Runtime hash/nonce/provenance fields.
- Explicit diagnostic/candidate distinction.

##### Evidence
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/audit-report.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`

#### docker-boundary

##### Summary
The Docker/runtime boundary was still too weak for official-equivalent claims: marker parsing, source isolation, and `/app` alias validation needed stricter semantics or explicit downgrade.

##### Blocking Findings
- Docker/runtime proof was marker-only and could pass independently from validator business result.
- Validator-source isolation was inferred from absence of path tokens and ignored the adapter's declared false source-isolation claim.
- `/app` alias validation only tested direct PowerShell, not every possible Whale command backend.

##### Non-blocking Risks
- Docker cleanup was best-effort and not reported.
- Backend selection logs lacked enough diagnostic detail.
- The strict E3 wrapper lacked a pass-through runner hook for stubbed forwarding tests.

##### Required Fixes
- Add nonce/hash/inspect proof markers and structured cleanup markers.
- Do not promote `agent_cannot_read_validator_source` from token scans alone.
- Keep Terminal-Bench post-hoc `/app` validation downgraded until official/equivalent runner proof exists.
- Add a runner hook to the strict E3 wrapper.

##### Missing Tests
- Missing wrapper marker should fail proof.
- Missing validator source should fail proof.
- One-sided outcomes must be checked against audit decision.
- External audit without attestations must fail.

##### Missing Logs / Observability
- Docker server/version and wrapper provenance.
- Cleanup exit rows.
- Validator-source hash and declared-vs-proven separation.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| evidence-gate | Missing validator source could prove isolation | Absence of a path was treated as proof of isolation. | blocking | accept | Missing source is not isolation proof. | `e3-proof.ps1` now requires source existence and tree SHA match before physical outside-repo proof. | closure review |
| evidence-gate | Runtime proof was too string-based | A future manifest flip plus weak logs could over-promote. | blocking | accept | Runtime rows now require wrapper marker, nonce, wrapper hash, entry hash, `/app`, Docker inspect marker, and readonly mount marker. | Added wrapper/entry hash, nonce, Docker version/inspect, cleanup markers; proof parser now uses anchored line markers. | closure review |
| evidence-gate | One-sided success could satisfy generic E3 business success | `left OR right` hid which mode passed. | blocking | accept | E3 utility decisions must match standard/taskspace outcomes. | Runner now passes `standard_success` and `taskspace_success`; gate rejects inconsistent include decisions. | closure review |
| evidence-gate | Audit too shallow | A reviewer could set decision without reviewing proof. | blocking | accept | External audit now requires proof artifacts, typed attestations, and decision rationale. | Added external audit attestation checks and suggested template fields. | closure review |
| evidence-gate | SourceVersion only non-empty | Pin was not verified when possible. | non-blocking | accept | Terminal-Bench task dirs are usually in a git checkout. | Adapter now checks 40-hex `SourceVersion` against git HEAD when available. | n/a |
| evidence-gate | Declared and proven fields blurred | Reports mixed manifest claims with proof results. | non-blocking | accept | Misread reports weaken audit quality. | Pair report labels manifest fields as `declared_*` and proof fields as `proof_*`. | n/a |
| docker-boundary | Marker-only runtime proof | Markers alone were insufficient. | blocking | accept | Added nonce/hash/inspect provenance and anchored parsing. | Same runtime proof hardening as above. | closure review |
| docker-boundary | Token-scan isolation over-claimed source isolation | Physical placement was treated as agent-inaccessible source. | blocking | accept | Current adapter declares source isolation false; proof must not override that. | Combined proof now keeps `agent_cannot_read_validator_source=false` unless physical proof and explicit adapter claim are both true. | closure review |
| docker-boundary | `/app` alias not fully backend-equivalent | Direct PowerShell smoke does not prove every shell backend. | blocking | accept as E3 limitation | The current adapter remains non-E3 and reports official equivalence false. Real diagnostic still validates the current Whale path for `hello-world`, but not official equivalence. | Kept `official_runner_or_equivalent=false`; no E3 promotion. Broader backend matrix remains future work. | closure review validates no false promotion |
| docker-boundary | Strict E3 wrapper lacked runner hook | Stubbed forwarding tests were harder. | non-blocking | accept | Useful for future regression tests. | Added `-RunnerPath` pass-through to strict E3 wrapper. | n/a |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - Round 2 pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: pending closure review
- Allowed to proceed: no

## Final Conclusion

Pending closure review.

## Round 2: Blocking Closure Review

### Review Input

#### Objective
Verify closure of accepted blocking findings from Round 1 without widening the scope beyond E3 proof/gate safety.

#### Review Target
Fixes to external proof semantics, audit requirements, side outcome consistency, Terminal-Bench Docker markers, and strict E3 wrapper behavior.

#### Target Locations
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/audit-report.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-external.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/test-e3-proof-harness.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `benchmarks/taskspace/external/README.md`

#### Change Introduction
Round 1 fixes added validator source existence/hash checks, stronger runtime proof markers, declared-vs-proven source isolation separation, side outcome consistency, external audit attestations, SourceVersion git HEAD checks, and a strict E3 runner hook.

#### Risk Focus
- Accepted blocking findings not actually closed.
- New false E3 promotion path.
- Runtime proof or audit proof becoming stale, spoofable, or missing.

#### Assumptions To Attack
- Missing validator source now fails closed.
- Manifest-only source isolation cannot promote E3.
- Runtime proof requires wrapper provenance.
- External audit cannot pass without proof artifacts and attestations.
- Side outcome consistency is enforced for E3 include decisions.
- Terminal-Bench remains E3-ineligible.

#### Adversarial Lenses
- failure
- data and isolation
- testing
- observability

#### Verification Status
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-proof-harness.ps1` passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed.
- `scripts\taskspace-benchmark\run-taskspace-e3-external.ps1 -PlanOnly` passed on Terminal-Bench `hello-world`.
- Real diagnostic Terminal-Bench pair generated proof artifacts and remained `E2-candidate`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Focus on scoped blocking closure and new high-risk regressions.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | 10 minutes once if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Closure reviewer | Required because Round 1 had accepted blocking findings. | proof gate closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Closure reviewer | `multi_agent_v1.spawn_agent` | `019e8917-f076-7a60-826d-0a653147eb12` | spawn_agent result | false | Round 2 Review Input plus target paths | main-agent history, reasoning, drafts, conclusions, full diff unless read from repo | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| closure | Closure reviewer | 1 | `019e8917-f076-7a60-826d-0a653147eb12` | 15 minutes | completed | returned one scoped blocking finding | completed |

### Reviewer Outputs

#### closure

##### Summary
Closure was mostly implemented, but the gate still allowed E3 promotion when `SideOutcomes` was omitted.

##### Blocking Findings
- Side outcome consistency was conditional, not fail-closed.

##### Non-blocking Risks
- `runtime_proven` and `validator_mount_proven` are split fields; combined E3 fidelity uses both.
- `-AllowCandidateResult` is explicit diagnostic allowance and should not be used as E3 pass evidence.

##### Required Fixes
- Add an E3 failure when `SideOutcomes` is null or lacks `standard_success` / `taskspace_success`.
- Update E3 ready tests to pass explicit side outcomes and add a negative test.

##### Missing Tests
- Missing side outcomes must fail E3.
- Validator source hash mismatch should fail source-isolation proof.
- Missing readonly marker should keep official/equivalent proof false.

##### Missing Logs / Observability
- No blocking observability gap found.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| closure | Side outcomes missing could still promote E3 | Gate only checked side-outcome consistency when the object existed. | blocking | accept | E3 utility decisions need explicit standard/taskspace outcome facts. | `Get-TaskspaceEvidenceGate` now emits `e3_side_outcomes_missing`; E3 ready tests pass explicit side outcomes; missing side outcomes negative test added. | validated by harness |
| closure | Validator hash mismatch negative test missing | Missing hash mismatch coverage could hide future regression. | non-blocking | accept | Reviewer asked for coverage. | Added `test-e3-proof-harness.ps1` hash mismatch negative case. | n/a |
| closure | Missing readonly marker negative test missing | Missing readonly coverage could hide future runtime proof regression. | non-blocking | accept | Reviewer asked for coverage. | Added `test-e3-proof-harness.ps1` readonly marker negative case. | n/a |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Blocked reason: n/a
- Allowed to proceed: yes
