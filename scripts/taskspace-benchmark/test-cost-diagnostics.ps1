param([string]$RunRoot = "target\cost-diagnostics-selftest")

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

if (Test-Path -LiteralPath $RunRoot) {
    $backup = "$RunRoot.bak-$(Get-Date -Format yyyyMMddHHmmss)"
    Move-Item -LiteralPath $RunRoot -Destination $backup
}

$left = Join-Path $RunRoot "pair-001\left\artifacts"
$right = Join-Path $RunRoot "pair-001\right\artifacts"
New-Item -ItemType Directory -Path $left, $right -Force | Out-Null

[pscustomobject]@{
    logical_mode = "standard"
    model_request_count = 1
    rollout_trace_model_request_count = $null
    input_tokens = 100000
    output_tokens = 1000
    uncached_input_tokens = 20000
    avg_input_tokens_per_request = 100000
    provider_input_tokens_per_jsonl_kb = 3000
    wall_time_ms = 30000
    large_output_replay_count = 0
    projection_tokens = $null
    projection_protected_miss_count = 0
    spawn_agent_calls = 0
    runtime_state_commit_count = 0
    taskspace_runtime_event_count = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $left "metrics.json") -Encoding UTF8

[pscustomobject]@{
    logical_mode = "taskspace"
    model_request_count = 1
    rollout_trace_model_request_count = 18
    input_tokens = 400000
    output_tokens = 5000
    uncached_input_tokens = 230000
    avg_input_tokens_per_request = 400000
    provider_input_tokens_per_jsonl_kb = 21000
    wall_time_ms = 72000
    large_output_replay_count = 0
    projection_count = 10
    projection_tokens = 3500
    projection_tokens_max = 450
    projection_protected_miss_count = 0
    spawn_agent_calls = 0
    runtime_state_commit_count = 4
    taskspace_runtime_event_count = 88
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $right "metrics.json") -Encoding UTF8

[pscustomobject]@{ status = "FAIL" } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $RunRoot "suite-cost-gate.json") -Encoding UTF8

$result = & (Join-Path $PSScriptRoot "write-cost-diagnostics.ps1") -RunRoot $RunRoot
Assert-True (Test-Path -LiteralPath $result.cost_diagnostics_path) "cost-diagnostics.json was not written"
Assert-True (Test-Path -LiteralPath $result.cost_diagnostics_markdown_path) "cost-diagnostics.md was not written"
$diag = Get-Content -Raw -Encoding UTF8 -LiteralPath $result.cost_diagnostics_path | ConvertFrom-Json
Assert-True ([string]$diag.root_cause -eq "active_profile_repeats_compact_taskspace_context_across_many_model_turns") "unexpected root cause"
Assert-True (@($diag.drivers) -contains "rollout_request_count_over_partial_budget") "rollout request driver missing"
Assert-True ([double]$diag.ratios.rollout_trace_model_request_count_ratio -eq 18) "rollout request ratio incorrect"

Write-Host "TaskSpace cost diagnostics self-test: PASS"
