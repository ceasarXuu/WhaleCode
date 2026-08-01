# Phase G. Non-agent gates, release fixtures, start-gate fixtures

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## G.1 Goal

Before any real E3, prove all new implementation contracts with deterministic non-agent tests and local artifacts.

## G.1.1 Current status after Phase A/B follow-up

Status: done for local non-agent gate builder; formal E3 still requires code-complete marker and explicit user approval for the formal sample set.

Already present:

```text
test-cost-instrumentation.ps1 covers several v0.0.5 summary fixtures
test-release-decision.ps1 covers v005 marker and release blocker fixtures
test-e3-start-gate.ps1 validates v005 marker identity, freshness, local evidence path, sha256, sample set, and task list identity
write-release-decision.ps1 reads v005-non-agent-gates.json and v005-code-complete.json
lib/e3-start-gate.ps1 enforces v005 marker gates
```

Now implemented:

```text
scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
canonical v005-non-agent-gates.json evidence bundle for current HEAD
per-gate local evidence files with sha256
single command that runs the required non-agent gates and writes the marker consumed by release/start gates
```

The builder has a fixture mode for fast schema/evidence validation, but the
release-like benefit must be proven with the default mode because default mode
executes the actual non-agent gates.

## G.2 Files to change

```text
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-e3-start-gate.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
scripts/taskspace-benchmark/lib/e3-start-gate.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
```

Required now:

```text
scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
scripts/taskspace-benchmark/test-v005-non-agent-gates-builder.ps1
```

## G.3 Required non-agent gates

`v005-non-agent-gates.json` must contain these gates:

```json
{
  "schema_version": 1,
  "status": "pass",
  "gates": {
    "provider_request_hook": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "runtime_budget_response": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "budget_quality_impact": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "active_context_replacement": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "state_commit_displacement": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "spawn_node_budget": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "request_phase_attribution": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "release_decision_fixture": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." },
    "start_gate_fixture": { "status": "pass", "command": "...", "exit_code": 0, "evidence_path": "...", "evidence_sha256": "..." }
  },
  "git_commit": "<current HEAD>",
  "profile_hash": "<runner profile hash>",
  "task_list_hash": "<formal P0 task list hash>",
  "source_version": "<terminal-bench source version>",
  "generated_at": "<ISO8601>"
}
```

Every gate evidence path must be a local file. `selftest://` is not acceptable.

## G.4 Build script contract

`build-v005-non-agent-gates.ps1` takes the identity that will later be used by
release/start gates:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\build-v005-non-agent-gates.ps1 `
  -RunRoot <run-root> `
  -TaskListHash <task-list-hash> `
  -ProfileHash <profile-hash> `
  -SourceVersion <source-version>
```

Default mode runs:

```text
cargo test -p codex-core provider_request_budget --lib
cargo test -p codex-core active_context_replacement --lib
cargo test -p codex-core state_commit --lib
cargo test -p codex-core budget --lib
powershell -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -File scripts\taskspace-benchmark\test-release-decision.ps1
powershell -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
```

Each gate gets a local evidence file under
`<run-root>/non-agent-evidence/<gate>.txt`, and the top-level
`v005-non-agent-gates.json` records the sha256 for that exact file.

## G.5 Release-decision negative fixtures

`test-release-decision.ps1` must include failing fixtures for:

```text
missing exact-payload-scan-events.jsonl
exact scan hash mismatch
provider request event producer not provider_lifecycle
request phase attribution all unknown
request phase attribution all model_sampling despite state_commit event
budget action without budget quality impact
BudgetQualityImpact final_classification=solved but score_eligible=false
state_commit_displacement without legacy_state_action_attempt events
spawn-node-budget unreviewed_subagent_result_count > 0
diagnostic sample_set_id terminal-bench_E3-P0_3_2 attempts release_pass
blocked_partial attempts closeable=true
```

## G.6 Start-gate fixture requirements

`test-e3-start-gate.ps1` must prove:

```text
full_e3_allowed=false when v005-non-agent-gates missing
full_e3_allowed=false when code-complete marker stale
full_e3_allowed=false when user approval sample set != terminal-bench_E3-P0_3_5
full_e3_allowed=false when task_list derivation != terminal-bench_E3-P0_3_5
full_e3_allowed=true only when all identities match and all markers are fresh/pass
```

## G.7 Acceptance

```text
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
pwsh -File scripts/taskspace-benchmark/test-v005-non-agent-gates-builder.ps1
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1
pwsh -File scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
```

## G.8 本地收益证明

Phase G 的收益是把“我跑过若干门禁”的聊天态结论，变成 release/start gate 都能校验的 typed marker：

```text
v005-non-agent-gates.json status = pass
每个 required gate status = pass
每个 gate 记录 command / exit_code / git_commit / task_list_hash / profile_hash / source_version
每个 gate evidence_path 是本地文件
每个 gate evidence_sha256 与本地文件实际 sha256 一致
start gate / release decision 继续拒绝 missing/stale/mismatched marker
```

本地已验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-v005-non-agent-gates-builder.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\build-v005-non-agent-gates.ps1 -RunRoot <target run> -TaskListHash <hash> -ProfileHash <hash> -SourceVersion <source>
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1
```
