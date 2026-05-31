# TaskSpace Benchmark E2 Closure Review

Date: 2026-05-31

## Round 1: E2 Gate Review

### Review Input

#### Objective

Review the E2 closure changes for the TaskSpace benchmark harness. The user challenged that stopping at E2-candidate was insufficient. The harness now attempts to satisfy E2 for `single-file-fast-fix` with three real paired runs, explicit CLI config, enabled aggregate, neutral temp run root, and delayed hidden-oracle materialization.

#### Review Target

- E2 gate implementation.
- Oracle isolation changes.
- Aggregate report behavior.
- Latest E2 real-run evidence.

#### Target Locations

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/lib/workspace.ps1`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065\aggregate-report.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065\run-summary.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065\pair-001\pair-report.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065\pair-002\pair-report.md`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065\pair-003\pair-report.md`

#### Change Introduction

The runner now defaults to `full-auto`, records explicit CLI config override `model_reasoning_effort="max"`, supports `-EnableAggregate`, writes aggregate reports, runs three repeats, alternates left/right logical mode, moves default run output to `%TEMP%\whale-paired-bench-runs`, and delays private oracle/canary materialization until after both agents complete. The isolation probe tries to read the planned canary path before materialization; E2 accepts `hard_deferred_materialization` and rejects `soft_denylist`/`failed`.

#### Risk Focus

- E2 may be a label trick rather than a real gate.
- Delayed materialization may be weaker than hard sandbox and should be challenged.
- Source oracle may still be discoverable from the repository or run path.
- `provider_param_status = explicit-cli-config` may be too coarse.
- Aggregate may include pairs that should be excluded.
- Three repeats may not all be real Whale runs.

#### Verification Status

Commands run by main agent:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -Model deepseek-v4-flash -TimeoutSeconds 900 -EnableAggregate
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 3 -Model deepseek-v4-flash -TimeoutSeconds 900 -EnableAggregate
git diff --check
```

Observed latest E2 run:

- Run dir: `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-185645-065`
- Aggregate: `valid_utility_pairs: 3`, `excluded_pairs: 0`
- Each pair reports `reported_evidence_level: E2`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| E2 Benchmark Adversarial Reviewer | E2 proof claims need fresh adversarial review | evidence gate, isolation, aggregate, real-run credibility |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| E2 Benchmark Adversarial Reviewer | multi_agent_v1.spawn_agent | `019e7db2-a3db-7481-86f9-bd572b0962f7` | spawn_agent + subagent_notification | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### E2 Benchmark Adversarial Reviewer

##### Summary

E2 closure is not fully credible yet. The paired run artifacts show three real Whale runs, both modes successful, gate failures reported as `none`, and aggregate counting 3/3 E2 pairs. The blocking gaps are oracle isolation and provider observability.

##### Blocking Findings

1. Oracle isolation is not hard enough for E2 because the source oracle remains readable outside the temp pair repo. The private run oracle is deferred, but the source oracle remains in the project workspace and actual Whale runs have root read permission.
2. The E2 gate collapses `hard_deferred_materialization` into `hard_sandbox`, hiding the weaker isolation class.
3. Provider parameter evidence is overclaimed because `provider_param_status = explicit-cli-config` only proves one override was passed.

##### Non-blocking Risks

- Pair 002 and 003 violate expected TaskSpace behavior limits but still pass E2. Since business success is true, this is not blocking E2 evidence, but weakens interpretation.
- Aggregate behavior is acceptable for the reviewed artifact.

##### Required Fixes

- Make the source hidden oracle inaccessible to Whale runs, not merely unmentioned.
- Preserve `hard_deferred_materialization` as its own gate input and require an explicit E2 policy decision.
- Replace `provider_param_status = explicit-cli-config` with a structured checklist of required provider/runtime params.

##### Missing Tests / Plan Gaps

- No test proves the source oracle cannot be read by an agent with benchmark permissions.
- No regression test covers `hard_deferred_materialization` as a distinct gate value.
- No test rejects partial provider config as explicit.
- No aggregate regression test mixes E2 and non-E2 pairs.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| E2 Benchmark Adversarial Reviewer | Source hidden oracle remained readable in workspace | blocking | accept | Hidden oracle source in repo weakens E2 isolation under root-read permissions | Scenario manifest no longer references a hidden oracle source file; tracked `private-oracle/oracle.py` is now a non-secret placeholder; harness generates private oracle only after both Whale runs finish | Round 2 closure |
| E2 Benchmark Adversarial Reviewer | `hard_deferred_materialization` collapsed into `hard_sandbox` | blocking | accept | Distinct isolation classes must remain visible to the gate | Runner preserves `hard_deferred_materialization`; evidence gate includes `oracle_isolation_policy`; E2 requires explicit `deferred_materialization_allowed` policy | Round 2 closure |
| E2 Benchmark Adversarial Reviewer | Provider params overclaimed by one override | blocking | accept | E2 should not pass on an opaque string | Replaced string status with structured checklist: required model, model_reasoning_effort, sandbox_mode; missing fields fail E2 | Round 2 closure |
| E2 Benchmark Adversarial Reviewer | TaskSpace over-decomposition warnings remain | non-blocking | accept | Warnings are benchmark signal, not E2 gate failure | Kept warnings visible in pair report | n/a |
| E2 Benchmark Adversarial Reviewer | Missing tests for provider/deferred isolation/mixed aggregate | non-blocking | accept | These protect the E2 gate from label drift | Added self-tests for strict deferred policy failure, partial provider config failure, and mixed aggregate E2/non-E2 counting | Round 2 closure |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes

## Round 2: E2 Closure Review

### Review Input

#### Objective

Verify closure for Round 1 E2 blocking findings: source oracle readability, `hard_deferred_materialization` gate semantics, structured provider parameter evidence, and mixed aggregate tests.

#### Target Locations

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/lib/workspace.ps1`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/scenario-manifest.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `benchmarks/taskspace/scenarios/single-file-fast-fix/scenario.json`
- `benchmarks/taskspace/scenarios/single-file-fast-fix/private-oracle/oracle.py`
- `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-190812-881\aggregate-report.md`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| E2 Closure Reviewer | Accepted E2 blocking fixes require fresh closure review | hidden oracle isolation, provider evidence, aggregate E2 gate |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| E2 Closure Reviewer | multi_agent_v1.spawn_agent | `019e7dbe-60c7-7e03-8042-962c379b6d02` | spawn_agent + subagent_notification | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### E2 Closure Reviewer

##### Summary

E2 closure Round 2 blocking items appear closed. No issue was found that makes the current E2 evidence untrustworthy or indicates evidence fabrication. The latest run directory is `C:\Users\77585\AppData\Local\Temp\whale-paired-bench-runs\single-file-fast-fix\20260531-190812-881`; aggregate reports 3 pairs, 3 valid utility pairs, 0 excluded pairs, and each pair is `E2` with `evidence_gate_failures: none`.

##### Blocking Findings

none

##### Non-blocking Risks

- The private oracle directory itself is created before the Whale runs, but the secret files are not written until after both Whale runs and the isolation probe.
- Pair reports still show TaskSpace over-decomposition warnings in some runs, but this is not an E2 evidence blocker because business success, hidden oracle, variable control, and gate failures remain clean.

##### Required Fixes

none

##### Missing Tests / Plan Gaps

No blocking gap. Prior gaps now have self-test coverage:

- partial provider config rejected.
- deferred materialization remains distinct under strict policy.
- mixed E2/non-E2 aggregate counting.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| E2 Closure Reviewer | No blocking findings | blocking | accept | Closure review passed the accepted E2 fixes | n/a | n/a |
| E2 Closure Reviewer | Private oracle directory exists before runs, but secret files are delayed | non-blocking | accept | Empty directory is not a secret; canary/oracle are written only after both Whale runs and probe | Keep current deferred materialization policy explicit in reports | n/a |
| E2 Closure Reviewer | TaskSpace over-decomposition warnings remain | non-blocking | accept | Warnings are visible benchmark signal and do not affect E2 evidence gate | Keep warnings visible | Future benchmark interpretation |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes
