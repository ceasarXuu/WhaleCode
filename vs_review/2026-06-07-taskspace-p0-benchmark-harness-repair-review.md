# Subagent VS Review: TaskSpace P0 Benchmark Harness Repair Plan

- Created: 2026-06-07T05:24:00+08:00
- Updated: 2026-06-07T05:48:00+08:00
- Report schema: adversarial-v1
- Task: review the repair plan for the failed Terminal-Bench P0 TaskSpace benchmark attempt.
- Report path: `vs_review/2026-06-07-taskspace-p0-benchmark-harness-repair-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed for implementation planning

## Round 1: Pre-implementation Plan Review

### Review Input

#### Objective

Find weaknesses in the proposed repair plan before implementation, especially places where the benchmark harness could still produce self-deceptive E3 utility evidence.

#### Review Target

Design and engineering plan for repairing the P0 benchmark harness after the partial run at `D:\whalecode-alpha\target\benchp0-20260607-014707`.

#### Target Locations

- `docs/testing/2026-06-07-taskspace-p0-benchmark-harness-repair-plan.md`
- `coe/2026-06-07-05-18-taskspace-p0-benchmark-harness.md`
- `scripts/taskspace-benchmark/`
- `target/benchp0-20260607-014707`

#### Change Introduction

The plan proposes separating sample preflight, agent execution, validator execution, metrics extraction, audit, and final aggregation. It adds contextual prompt guard behavior, remote asset preflight/cache, non-fatal metrics extraction, explicit run status files, and stable pair classification categories.

#### Risk Focus

- Relaxing prompt guard may let TaskSpace-friendly prompts through.
- Remote asset cache may break official runner equivalence.
- Non-fatal metrics may hide real filesystem pollution.
- Audit automation may make E3 conclusions look cleaner than they are.
- Windows/WSL/Docker failures may remain hard to resume or classify.

#### Assumptions To Attack

- External benchmark domain terms can be safely separated from internal TaskSpace leakage.
- Cached remote assets can be proven equivalent to pinned upstream assets.
- Metrics extraction can be best-effort without corrupting pair classification.
- A run state machine can resume without duplicating or losing pairs.
- The existing PowerShell harness can support this without a parallel runner.

#### Adversarial Lenses

- testing
- observability
- release operations
- failure recovery
- data integrity
- benchmark validity

#### Verification Status

- No repair code has been implemented.
- Evidence comes from the 2026-06-07 P0 partial run and current harness scripts.
- Known unverified area: exact cache injection implementation and official-equivalence proof update.

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers or log fragments when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| test-validity-adversary | The plan is about benchmark validity and E3 evidence cleanliness. | self-deceptive tests, weak gates, invalid utility claims |
| release-ops-and-observability-adversary | The failures cross Windows, PowerShell, WSL, Docker, cache, file locks, and run recovery. | operational robustness, logging, resume, environment classification |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9ec7-239e-7682-a1a5-544b9c040dce` | spawn_agent return nickname `Lovelace` | fork_context=false | Round 1 Review Input plus explicit task prompt | main-agent history, reasoning, drafts beyond named files, write permission | yes |
| release-ops-and-observability-adversary | `multi_agent_v1.spawn_agent` explorer | `019e9ec7-7a72-77e2-b7fc-b7fea6da91da` | spawn_agent return nickname `Bernoulli` | fork_context=false | Round 1 Review Input plus explicit task prompt | main-agent history, reasoning, drafts beyond named files, write permission | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| test-validity-round-1 | test-validity-adversary | 1 | `019e9ec7-239e-7682-a1a5-544b9c040dce` | completed | completed | blocking design findings returned | integrate |
| release-ops-round-1 | release-ops-and-observability-adversary | 1 | `019e9ec7-7a72-77e2-b7fc-b7fea6da91da` | completed | completed | blocking design findings returned | integrate |

### Reviewer Outputs

#### Reviewer 1: test-validity-adversary

Verdict: the plan direction is correct, but it still had benchmark self-deception risks.

Blocking findings:

1. Prompt guard allowlists lacked provenance constraints. A domain allowlist for `multi-agent` must apply only to pinned upstream task spans, not adapter wrappers, generated prompts, or user-provided control text.
2. Remote asset caching could break Terminal-Bench equivalence. Dockerfile rewrites or local `COPY` injection must default to E3-ineligible unless content-addressed equivalence proof exists.
3. Metrics best-effort handling cannot treat critical fixture/hash failures as harmless warnings. Critical files such as `oewn.sqlite` need taints and E3 blocking until re-readable and hashed.
4. Audit automation was too ambiguous. A writer can produce an audit draft, but E3 inclusion requires explicit reviewer completion and attestation.

Required tests:

- malicious prompt that mixes benchmark domain text with `/taskspace`, `spawn_agent`, or `bind_node`
- over-broad allowlist regex
- cache checksum mismatch
- Dockerfile rewrite without equivalence proof
- locked critical artifact taint/exclusion
- generated audit file cannot enter E3 aggregate by itself

Required logs:

- prompt-guard source spans and allowlist source
- remote asset manifest and transform proof
- metrics warnings and taints
- aggregate denominator splits

#### Reviewer 2: release-ops-and-observability-adversary

Verdict: the plan identifies the right failure classes, but the operational contract was not strong enough to make P0 reruns resumable and diagnosable.

Blocking findings:

1. `run-status.json` and `sample-status.json` were underspecified for recovery. They need schema version, atomic writes, phase transition log, per-pair cursor, last successful artifact, process/host/owner fields, heartbeat, stale-lock policy, argv/env snapshot, and idempotency tokens.
2. Remote cache trust boundary was weak. Cache keys must be tied to source revision, URL, content hash, and post-injection materialized tree hash.
3. Docker failures were still too coarse. The adapter must emit structured build/run/inspect/cleanup result JSON and separate daemon/backend failure, image pull failure, apt failure, remote asset fetch failure, proxy failure, and path conversion failure.
4. Windows/PowerShell artifact scanning was not hardened. Recursive scans can hit volatile `.tbench-testing` paths, CLIXML stderr, CRLF warnings, Chinese exceptions, and lock races. The plan needs explicit scan boundaries, warnings, and stderr layering.

Required tests:

- resume from every phase
- negative cache cases
- Docker build classification fixtures
- PowerShell scan regression with locked/volatile files
- partial metrics still emit pair/sample status

Required logs:

- append-only `events.jsonl`
- `preflight.remote-assets.json`
- `docker-build-result.json`
- raw metrics warning fields, including `FullyQualifiedErrorId`
- resume decision log

### Main Agent Response

Accepted all blocking findings.

Plan updates made in `docs/testing/2026-06-07-taskspace-p0-benchmark-harness-repair-plan.md`:

1. Added prompt-source provenance fields and constrained allowlists to upstream benchmark task spans only.
2. Strengthened remote asset cache requirements with required checksums, source lines, transform diff, source revision, and post-injection tree hash.
3. Added E3 blocking taints for critical artifact hash failures.
4. Expanded run/sample status schemas with atomic writes, append-only events, lock heartbeat, stale-lock reclaim policy, argv/env snapshot, pair cursor, and idempotency token.
5. Split audit draft generation from audit completion and aggregate inclusion.
6. Added structured Docker failure categories and required `docker-build-result.json`.
7. Added aggregate denominator semantics.
8. Expanded tests for prompt provenance, cache negatives, Docker classification, locked critical artifacts, resume phases, and audit boundary.

Rejected findings:

- None.

Deferred findings:

- A general cross-benchmark cache service is deferred. The immediate repair keeps scope on the Terminal-Bench adapter and existing PowerShell benchmark harness.
- Benchmark run visualization is deferred. The repair prioritizes machine-readable artifacts because E3 cleanliness depends on auditability first.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed in plan: yes
- Blocking re-review completed: not required for planning document; required after repair implementation changes
- Blocking re-review passed: not applicable yet
- Blocking re-review round links:
  - none
- Blocking re-review launch records:
  - none
- Rejected findings backed by evidence: none rejected
- Deferred findings documented: yes
- Blocked reason: none for implementation planning; P0 rerun remains blocked until implementation and smoke tests complete
- Allowed to proceed: yes, for implementation; no, for claiming E3 utility evidence

## Final Conclusion

The repair plan passed pre-implementation adversarial review after accepting and integrating all blocking findings. The project should not rerun P0 as E3 evidence until the implementation satisfies the strengthened contracts for prompt provenance, remote asset equivalence, critical metrics taints, resumable run state, structured Docker classification, and explicit audit completion.
