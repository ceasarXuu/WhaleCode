param(
    [string]$RunRoot = "",
    [string]$SnapshotReportPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target\r4-public-10-usage-accounting-gate"
}
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$planPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-public-10-tool-stress-plan.json"
$defaultSnapshotReportPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-public-10-tool-stress-report.snapshot.json"
if ([string]::IsNullOrWhiteSpace($SnapshotReportPath)) {
    $SnapshotReportPath = $defaultSnapshotReportPath
}
$goodReport = Join-Path $RunRoot "good-report.json"
$badReport = Join-Path $RunRoot "bad-report.json"

Assert-True (Test-Path -LiteralPath $SnapshotReportPath -PathType Leaf) "snapshot report not found: $SnapshotReportPath"
Copy-Item -LiteralPath $SnapshotReportPath -Destination $goodReport -Force

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "test-r4-public-10-tool-stress-plan.ps1") `
    -PlanPath $planPath `
    -ReportPath $goodReport
Assert-True ($LASTEXITCODE -eq 0) "good report did not pass public-10 gate"

$report = Get-Content -Raw -Encoding UTF8 -LiteralPath $goodReport | ConvertFrom-Json
$heterogeneousDates = @($report.rows | Where-Object { [string]$_.task_id -eq "heterogeneous-dates" })[0]
Assert-True ($null -ne $heterogeneousDates) "good report did not contain heterogeneous-dates"
Assert-True ([string]$heterogeneousDates.model_request_count_availability -eq "measured") "heterogeneous-dates model request count was not measured"
Assert-True ([string]$heterogeneousDates.taskspace_model_request_count_source -eq "rollout_trace") "heterogeneous-dates did not use rollout_trace for TaskSpace model request count"
Assert-True ([double]$heterogeneousDates.taskspace_model_request_count -gt [double]$heterogeneousDates.standard_model_request_count) "heterogeneous-dates did not expose TaskSpace request amplification"
Assert-True ([double]$heterogeneousDates.taskspace_model_request_ratio -gt 1) "heterogeneous-dates request ratio did not expose amplification"

$target = @($report.rows | Where-Object { [string]$_.run_status -eq "found" } | Select-Object -First 1)[0]
Assert-True ($null -ne $target) "good report did not contain a found row"
$target.taskspace_token_ratio = $null
$target.token_ratio_availability = "measured"
$report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $badReport -Encoding UTF8

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "test-r4-public-10-tool-stress-plan.ps1") `
        -PlanPath $planPath `
        -ReportPath $badReport *> (Join-Path $RunRoot "bad-report-gate.log")
    $badExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
Assert-True ($badExitCode -ne 0) "bad report unexpectedly passed public-10 gate"

$badLog = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $RunRoot "bad-report-gate.log")
Assert-True ($badLog -match "taskspace_token_ratio is missing") "bad report failure did not explain token ratio availability mismatch"

$badRequestReport = Join-Path $RunRoot "bad-request-report.json"
$report = Get-Content -Raw -Encoding UTF8 -LiteralPath $goodReport | ConvertFrom-Json
$requestTarget = @($report.rows | Where-Object { [string]$_.run_status -eq "found" } | Select-Object -First 1)[0]
Assert-True ($null -ne $requestTarget) "good report did not contain a found row for request-count mutation"
$requestTarget.taskspace_model_request_ratio = $null
$requestTarget.model_request_count_availability = "measured"
$report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $badRequestReport -Encoding UTF8

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "test-r4-public-10-tool-stress-plan.ps1") `
        -PlanPath $planPath `
        -ReportPath $badRequestReport *> (Join-Path $RunRoot "bad-request-report-gate.log")
    $badRequestExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
Assert-True ($badRequestExitCode -ne 0) "bad request-count report unexpectedly passed public-10 gate"

$badRequestLog = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $RunRoot "bad-request-report-gate.log")
Assert-True ($badRequestLog -match "taskspace_model_request_ratio is missing") "bad request-count report failure did not explain request ratio availability mismatch"

$fallbackRunRoot = Join-Path $RunRoot "rollout-token-fallback-runroot"
$fallbackPairDir = Join-Path $fallbackRunRoot "actual\runs\terminal_bench__heterogeneous-dates\20260703-060000-000\pair-001"
$fallbackLeftArtifacts = Join-Path $fallbackPairDir "left\artifacts"
$fallbackRightArtifacts = Join-Path $fallbackPairDir "right\artifacts"
New-Item -ItemType Directory -Force -Path $fallbackLeftArtifacts, $fallbackRightArtifacts | Out-Null
@'
- outcome_standard: agent_exec_timeout
- outcome_taskspace: agent_exec_timeout
- failure_taxonomy: timeout
'@ | Set-Content -LiteralPath (Join-Path $fallbackPairDir "pair-report.md") -Encoding UTF8
[pscustomobject]@{
    logical_mode = "standard"
    exec_timed_out = $true
    wall_time_ms = 900000
    tool_call_count = 1
    token_summary_availability = "usage_unavailable_after_timeout"
    rollout_trace_input_tokens = 100
    rollout_trace_output_tokens = 20
    rollout_trace_model_request_count = 2
    public_validation_exit_code = -1
    hidden_oracle_exit_code = -1
} | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $fallbackLeftArtifacts "metrics.json") -Encoding UTF8
[pscustomobject]@{
    logical_mode = "taskspace"
    exec_timed_out = $true
    wall_time_ms = 900000
    tool_call_count = 2
    token_summary_availability = "usage_unavailable_after_timeout"
    rollout_trace_input_tokens = 300
    rollout_trace_output_tokens = 60
    rollout_trace_model_request_count = 6
    public_validation_exit_code = -1
    hidden_oracle_exit_code = -1
} | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $fallbackRightArtifacts "metrics.json") -Encoding UTF8
$fallbackReport = Join-Path $RunRoot "rollout-token-fallback-report.json"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-r4-public-10-tool-stress-report.ps1") `
    -PlanPath $planPath `
    -RunRoots @($fallbackRunRoot) `
    -OutputPath $fallbackReport
Assert-True ($LASTEXITCODE -eq 0) "fallback report writer failed"
$fallback = Get-Content -Raw -Encoding UTF8 -LiteralPath $fallbackReport | ConvertFrom-Json
$fallbackRow = @($fallback.rows | Where-Object { [string]$_.task_id -eq "heterogeneous-dates" })[0]
Assert-True ($null -ne $fallbackRow) "fallback report did not contain heterogeneous-dates"
Assert-True ([string]$fallbackRow.token_ratio_availability -eq "recovered_from_rollout_trace") "fallback row did not mark token ratio as recovered from rollout trace"
Assert-True ([double]$fallbackRow.taskspace_token_ratio -eq 3) "fallback row did not compute token ratio from rollout trace"
Assert-True ([string]$fallbackRow.standard_usage_accounting_status -eq "recovered_from_rollout_trace") "standard fallback usage status was not recovered"
Assert-True ([string]$fallbackRow.taskspace_usage_accounting_status -eq "recovered_from_rollout_trace") "taskspace fallback usage status was not recovered"
Assert-True ([int64]$fallbackRow.standard_input_tokens -eq 100) "standard input tokens were not recovered from rollout trace"
Assert-True ([int64]$fallbackRow.taskspace_output_tokens -eq 60) "taskspace output tokens were not recovered from rollout trace"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "test-r4-public-10-tool-stress-plan.ps1") `
    -PlanPath $planPath `
    -ReportPath $fallbackReport
Assert-True ($LASTEXITCODE -eq 0) "fallback report did not pass public-10 report gate"

Write-Host "PASS: R4 public-10 usage accounting gate rejects ambiguous token usage"
