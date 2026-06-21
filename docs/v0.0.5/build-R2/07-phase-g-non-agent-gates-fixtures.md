# Phase G. Non-agent gates, release fixtures, start-gate fixtures

> Split from `22-v005-completion-engineering-playbook.md` to keep each execution context small and phase-cohesive.
>
> Canonical sequence: read `00-overview-and-gates.md` first, then only the phase file you are implementing.


## G.1 Goal

Before any real E3, prove all new implementation contracts with deterministic non-agent tests and local artifacts.

## G.2 Files to change

```text
scripts/taskspace-benchmark/test-cost-instrumentation.ps1
scripts/taskspace-benchmark/test-e3-start-gate.ps1
scripts/taskspace-benchmark/test-release-decision.ps1
scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
scripts/taskspace-benchmark/lib/e3-start-gate.ps1
scripts/taskspace-benchmark/write-release-decision.ps1
```

Optionally add:

```text
scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
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

## G.4 Build script pseudocode

If adding `build-v005-non-agent-gates.ps1`, implement:

```powershell
param(
  [Parameter(Mandatory=$true)][string]$RunRoot,
  [Parameter(Mandatory=$true)][string]$TaskListHash,
  [Parameter(Mandatory=$true)][string]$ProfileHash,
  [Parameter(Mandatory=$true)][string]$SourceVersion
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$head = git -C $repoRoot rev-parse HEAD
$gates = [ordered]@{}

function Invoke-Gate($Name, $Command, $EvidencePath) {
  $result = Invoke-Expression $Command
  $sha = if (Test-Path $EvidencePath) { (Get-FileHash $EvidencePath -Algorithm SHA256).Hash.ToLowerInvariant() } else { "" }
  $gates[$Name] = [pscustomobject]@{
    status = if ($LASTEXITCODE -eq 0 -and $sha) { "pass" } else { "fail" }
    producer = "build-v005-non-agent-gates.ps1"
    command = $Command
    exit_code = $LASTEXITCODE
    generated_at = (Get-Date).ToString("o")
    git_commit = $head
    profile_hash = $ProfileHash
    task_list_hash = $TaskListHash
    source_version = $SourceVersion
    evidence_path = $EvidencePath
    evidence_sha256 = $sha
  }
}

Invoke-Gate "provider_request_hook" "cargo test -p codex-core provider_request_budget --locked" "$RunRoot\evidence\provider_request_hook.txt"
Invoke-Gate "active_context_replacement" "cargo test -p codex-core active_context_replacement --locked" "$RunRoot\evidence\active_context_replacement.txt"
Invoke-Gate "release_decision_fixture" "pwsh -File scripts\taskspace-benchmark\test-release-decision.ps1" "$RunRoot\evidence\release_decision_fixture.txt"
# etc.

[pscustomobject]@{
  schema_version = 1
  status = if (@($gates.Values | Where-Object { $_.status -ne "pass" }).Count -eq 0) { "pass" } else { "fail" }
  gates = [pscustomobject]$gates
  git_commit = $head
  profile_hash = $ProfileHash
  task_list_hash = $TaskListHash
  source_version = $SourceVersion
  generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $RunRoot "v005-non-agent-gates.json")
```

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
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1
pwsh -File scripts/taskspace-benchmark/test-external-wrapper-harness.ps1
```
