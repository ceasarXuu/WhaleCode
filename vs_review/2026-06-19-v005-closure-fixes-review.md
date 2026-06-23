# Subagent VS Review: v0.0.5 closure fixes

- Created: 2026-06-19
- Updated: 2026-06-19
- Task: Verify closure fixes for accepted v0.0.5 continuation-scheme review blockers before running a low-cost diagnostic E3-candidate.
- Report path: `vs_review/2026-06-19-v005-closure-fixes-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: closure review

### Review Input

#### Objective

Verify whether the accepted blocking findings from `vs_review/2026-06-19-v005-continuation-scheme-review.md` are closed enough to permit a low-cost diagnostic `terminal-bench_E3-P0_3_1` run. This does not authorize formal E3 release proof.

#### Review Target

Implementation and test closure for:

- provider request attribution fallback removal
- active provider-visible context replacement
- runtime-owned `state_commit_displacement` denominator
- spawn/node budget enforcement split
- suite runner attestation receipt-chain join
- release/start gate anti-spoofing and marker identity checks

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/write-release-decision.ps1`
- `scripts/taskspace-benchmark/test-cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/test-e3-start-gate.ps1`
- `scripts/taskspace-benchmark/test-release-decision.ps1`
- `docs/v0.0.5/implementation-log.md`
- `docs/experiments/taskspace-evidence-levels-and-samples.md`

#### Change Introduction

The implementation now removes ActionMap ready-node fallback for provider request attribution, records runtime-owned budget/displacement evidence, splits spawn/node budget release and negative-fixture semantics, hardens release/start gates, and records the changes in the v0.0.5 implementation log.

#### Risk Focus

- Provider request attribution must not infer node/phase from later ActionMap state.
- Active replacement must remove raw TaskSpace/control/large output replay while preserving protected inputs.
- State commit displacement must require a denominator.
- Spawn/node budget must distinguish successful over-budget enforcement from within-budget release cleanliness.
- Runner attestation must join the suite receipt chain, not just pass string checks.
- Release decision must reject non-agent marker identity mismatch and weak code-complete markers.

#### Verification Status

Main agent reported these commands passed before review:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1 -RunRoot target\v005-runtime-budget-cost-selftest-4
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-start-gate-hardening-selftest-2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-release-hardening-selftest-2
cargo test -p codex-core state_commit -- --nocapture
cargo test -p codex-core provider_request_budget -- --nocapture
cargo test -p codex-core active_context_replacement -- --nocapture
git diff --check
```

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | Verify runtime/session/script implementation closure | correctness, state flow, evidence semantics |
| test-validity-adversary | Verify anti-spoofing and E3 reporting gates | test validity, release gate integrity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf80-3fb8-7d03-bb3c-fb6d3bfebfd2` | spawn/wait/close tool transcript | no | Round 1 implementation closure packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |
| test-validity-adversary | `multi_agent_v1.spawn_agent` explorer | `019edf80-80fa-7c10-a9b6-7481f834da94` | spawn/wait/close tool transcript | no | Round 1 test-validity closure packet | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### implementation-adversary

##### Summary

Closure review passes. The target commits close the prior accepted blocking findings enough to allow a low-cost diagnostic E3-candidate. This still does not authorize formal E3.

##### Blocking Findings

- None.

##### Non-blocking Risks

- Active provider-visible composer still relies partly on string markers; future marker drift could require stronger typed contracts.
- Attestation chain is local artifact integrity, not a cryptographic trust boundary.

##### Required Fixes

- None.

##### Missing Tests

- No blocking test gap. Existing tests cover provider fallback removal, active replacement, weak code-complete marker, non-agent identity mismatch, receipt chain failures, and spawn/node split semantics.

##### Missing Logs / Observability

- No blocking observability gap.

##### Evidence

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1555` - provider snapshot uses explicit current main node and records missing reason.
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:10161` - test covers no fallback to ready node.
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1373` - active replacement composer keeps protected items and removes legacy/control/large replay.
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:292` - state commit denominator is consumed from runtime events.
- `scripts/taskspace-benchmark/write-release-decision.ps1:720` - release gate requires state commit denominator.
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:372` - spawn/node budget splits within-budget and over-budget enforcement status.
- `scripts/taskspace-benchmark/write-release-decision.ps1:568` - attestation joins receipt chain by nonce/hash/process/command hash.

#### test-validity-adversary

##### Summary

Closure review passes. Current gates are sufficient to prevent the reviewed E3 misrun/misreport paths. The next run may be a low-cost diagnostic E3-candidate, not a formal E3 release proof.

##### Blocking Findings

- None.

##### Non-blocking Risks

- The attestation chain can be recomputed by an actor with write access to all local artifacts; do not describe it as a cryptographic trust boundary.
- Formal E3 conclusions still require real external-audit/provenance interpretation beyond local consistency.

##### Required Fixes

- None.

##### Missing Tests

- No blocking missing tests. Existing tests cover forged attestation, damaged receipt chain, incomplete sample coverage, non-agent identity mismatch, weak code-complete marker, and suite no-scheduling when `full_e3_allowed=false`.

##### Missing Logs / Observability

- No blocking gap. Release decision exposes attestation receipt-chain pass, event count, formal sample coverage, sample counts, derived sample set, and marker pass status.

##### Evidence

- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:106` - `full_e3_allowed` requires top-level pass, calibration pass, and v0.0.5 markers.
- `scripts/taskspace-benchmark/lib/e3-start-gate.ps1:539` - blocked marker creates top-level `blocked_for_full_e3`.
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1:357` - suite runner stops before scheduling when `full_e3_allowed=false`.
- `scripts/taskspace-benchmark/write-release-decision.ps1:595` - formal E3 identity requires sample set, repeats, pair count, start gate, markers, provenance, and receipt.
- `scripts/taskspace-benchmark/write-release-decision.ps1:727` - release decision revalidates all v0.0.5 non-agent gates.
- `scripts/taskspace-benchmark/test-release-decision.ps1:724` - test covers non-agent identity mismatch.
- `scripts/taskspace-benchmark/test-release-decision.ps1:734` - test covers weak code-complete marker.
- `docs/experiments/taskspace-evidence-levels-and-samples.md:25` - E3-candidate is not final E3 score or release proof.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-adversary | No blocking findings | n/a | accept | Reviewer found closure sufficient for diagnostic candidate | Recorded report | Proceed to low-cost diagnostic run |
| implementation-adversary | String-marker based active composer can drift | minor | defer | Non-blocking for current diagnostic; typed contracts can be strengthened later | Recorded risk | Track for later hardening |
| implementation-adversary | Attestation chain is not cryptographic trust boundary | minor | accept | Accurate limitation | Recorded risk | Avoid overclaiming in result report |
| test-validity-adversary | No blocking findings | n/a | accept | Reviewer found release/start gates sufficient for reviewed risks | Recorded report | Proceed to low-cost diagnostic run |
| test-validity-adversary | Formal E3 still needs full official evidence | minor | accept | Diagnostic run is not release proof | Recorded risk | Label next run as diagnostic/E3-candidate only |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: yes
- Allowed to proceed: yes, for low-cost diagnostic E3-candidate only

## Final Conclusion

Closure review passed. It is acceptable to run the requested low-cost `terminal-bench_E3-P0_3_1` diagnostic candidate. The run must not be reported as formal E3 release proof.
