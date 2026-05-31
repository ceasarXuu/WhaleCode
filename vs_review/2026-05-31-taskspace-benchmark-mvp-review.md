# TaskSpace Benchmark MVP Review

Date: 2026-05-31

## Round 1: Implementation Review

### Review Input

#### Objective

Review the first runnable TaskSpace benchmark MVP. The MVP should run a real Whale CLI paired benchmark for `single-file-fast-fix`, comparing `standard` and `taskspace` with the same prompt, fixture, model, timeout, permission shape, public validation, and hidden oracle.

#### Review Target

- Benchmark harness implementation.
- Scenario manifest, fixture, prompt, and hidden oracle.
- Harness self-test coverage.
- Real paired run artifacts and generated report.

#### Target Locations

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `scripts/taskspace-benchmark/lib/*.ps1`
- `benchmarks/taskspace/scenarios/single-file-fast-fix/`
- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/`

#### Change Introduction

The change adds a new MVP harness instead of modifying old E1 runners. It reuses existing Whale process execution, rollout lookup, TaskSpace observability export, and graph-health helpers. The harness creates neutral `left/right` workspaces, alternates logical mode mapping by repeat, runs real `whale exec` for standard and taskspace, runs public validation and hidden oracle, records prompt guard and manifest checksums, computes evidence level, and writes `pair-report.md` plus `run-summary.md`.

#### Risk Focus

- The runner may silently use fake or partial paths instead of real Whale CLI.
- The pair may not control variables tightly enough.
- Hidden oracle may leak into agent-visible context.
- Evidence gate may mislabel `Repeats 1`, unknown provider params, or `soft_denylist` as E2.
- The prompt guard may leak taskspace/map/node/subagent concepts or overblock normal engineering language.
- The report may hide the L1 over-decomposition signal instead of surfacing it.
- The self-tests may only test mocks and not protect important evidence gates.

#### Verification Status

Commands already run by main agent:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -Model deepseek-v4-flash -TimeoutSeconds 900
git diff --check
```

Observed real-run artifacts:

- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/run-summary.md`
- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/prompt-guard.json`
- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/pair-001/pair-report.md`
- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/pair-001/left/artifacts/metrics.json`
- `target/taskspace-benchmark/single-file-fast-fix/20260531-174757-854/pair-001/right/artifacts/metrics.json`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Benchmark MVP Adversarial Reviewer | First real runnable harness touches process execution, evidence gates, hidden oracle, and reports | runner correctness, evidence credibility, isolation, test coverage |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Benchmark MVP Adversarial Reviewer | multi_agent_v1.spawn_agent | `019e7d71-3bbb-7560-873a-f64694285edb` | spawn_agent + subagent_notification | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Benchmark MVP Adversarial Reviewer

##### Summary

The MVP appears to run the real Whale CLI: the runner checks the binary, verifies `exec --help` exposes `--taskspace`, and invokes `Invoke-RealProcess` for both sides. The artifacts contain real Whale JSONL, stderr, public pytest validation, hidden oracle output, and TaskSpace observability.

##### Blocking Findings

1. Agent-visible paths leak `taskspace`, so the natural-user condition is not clean. The default run root used `target\taskspace-benchmark`, the neutral cwd check only rejected exact leaf names, and the current run proved the path appeared in model-visible rollout and JSONL artifacts.
2. Prompt guard can globally whitelist real `map/node` internal leaks when any allowed engineering phrase appears elsewhere. Benign phrases such as `Node.js source map` could suppress a separate internal `node map` phrase.

##### Non-blocking Risks

- Hidden oracle is copied under `reviewer-only\private-oracle` while Whale runs with bypass sandbox. The report correctly downgrades this to `soft_denylist`, so this is not blocking.
- `Write-TaskspaceRunSummary` hardcoded `included_in_utility_aggregate: false`, which would become wrong if a future pair reaches E2.

##### Required Fixes

- Use a neutral run root with no `taskspace`, `map`, `node`, or treatment labels in model-visible ancestor paths, and make neutral cwd checks reject substrings across the full resolved path.
- Change prompt guard allowlisting to be span/local-context based.

##### Missing Tests / Plan Gaps

- Add self-tests for model-visible cwd/path leaks, including `taskspace-benchmark`.
- Add mixed prompt guard tests for benign `Node.js source map` plus separate internal `node map`.
- Add oracle leak tests for repo-visible leaks.
- Add a test or report assertion that standard/taskspace argv differ only by `--taskspace`.
- Add a run-summary test that aggregate inclusion reflects the evidence gate instead of being hardcoded.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Benchmark MVP Adversarial Reviewer | Model-visible path leaked `taskspace` through `target\taskspace-benchmark` | blocking | accept | Natural-user benchmark should not expose private TaskSpace concepts through cwd/environment context | Default run root changed to `target\paired-bench-runs`; self-test root changed to `target\paired-bench-selftest`; neutral cwd check now scans the full resolved path for forbidden substrings | Round 2 closure |
| Benchmark MVP Adversarial Reviewer | Prompt guard allowlist was global, not span-local | blocking | accept | A benign `Node.js source map` phrase must not suppress a separate internal `node map` leak | Prompt guard now builds allowed spans and only suppresses context hits contained within an allowed span | Round 2 closure |
| Benchmark MVP Adversarial Reviewer | Hidden oracle is soft isolation under bypass sandbox | non-blocking | accept | Current MVP correctly downgrades to `soft_denylist` and excludes from E2 | Kept behavior; real report continues to show `oracle_isolation_soft_denylist` as an E2 gate failure | n/a |
| Benchmark MVP Adversarial Reviewer | Run summary hardcoded aggregate inclusion | non-blocking | accept | Future E2 report should not be contradicted by summary | Run summary now reflects `evidence.included_in_utility_aggregate` | Round 2 closure |
| Benchmark MVP Adversarial Reviewer | Missing tests for path leaks, mixed prompt, repo oracle leak, argv delta, run summary aggregate field | non-blocking | accept | These protect the exact guardrails under review | Added self-tests for all listed gaps | Round 2 closure |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes

## Round 2: Closure Review

### Review Input

#### Objective

Verify that the two accepted blocking findings from Round 1 are closed: model-visible path leaks and overly broad prompt-guard allowlisting.

#### Target Locations

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `scripts/taskspace-benchmark/lib/prompt-guard.ps1`
- `scripts/taskspace-benchmark/lib/workspace.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `target/paired-bench-runs/single-file-fast-fix/20260531-182809-669/pair-001/pair-report.md`
- `target/paired-bench-runs/single-file-fast-fix/20260531-182809-669/run-summary.md`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Benchmark MVP Closure Reviewer | Accepted blocking fixes require fresh closure review | neutral cwd, span-local prompt guard, evidence reporting |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Benchmark MVP Closure Reviewer | multi_agent_v1.spawn_agent | `019e7d96-5182-7540-9e58-459dc6e7b97c` | spawn_agent + subagent_notification | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Benchmark MVP Closure Reviewer

##### Summary

Round 2 closure passes on the two prior blocking issues. The default run root is now neutral, cwd validation rejects full-path substring leaks like `taskspace-benchmark`, prompt guard allowlisting is span-local, the run summary reflects the evidence gate, and the latest listed run reports both `standard` and `taskspace` as successful while staying `E2-candidate`, not `E2`.

##### Blocking Findings

none

##### Non-blocking Risks

- The review ledger still said blocking re-review was pending and allowed to proceed was no. This was a process gap.
- The latest pair report still reports TaskSpace over-decomposition warnings. They are surfaced correctly and do not falsify the benchmark result.

##### Required Fixes

none

##### Missing Tests / Plan Gaps

No blocking test gap found.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Benchmark MVP Closure Reviewer | No blocking findings | blocking | accept | Closure review passed the accepted blocking fixes | n/a | n/a |
| Benchmark MVP Closure Reviewer | Review ledger still showed pending/no | non-blocking | accept | Closure status should reflect completed review | Updated this report with Round 2 output and closure status | n/a |
| Benchmark MVP Closure Reviewer | TaskSpace over-decomposition warnings remain | non-blocking | accept | This is intended observability; L1 cost/regression warning is a benchmark signal, not a harness failure | Kept report behavior | n/a |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes
