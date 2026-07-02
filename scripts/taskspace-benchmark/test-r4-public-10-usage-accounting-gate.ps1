param(
    [string]$RunRoot = ""
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
$goodReport = Join-Path $RunRoot "good-report.json"
$badReport = Join-Path $RunRoot "bad-report.json"

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-r4-public-10-tool-stress-report.ps1") `
    -PlanPath $planPath `
    -OutputPath $goodReport `
    -RequireComplete
Assert-True ($LASTEXITCODE -eq 0) "failed to build public-10 report fixture"

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

Write-Host "PASS: R4 public-10 usage accounting gate rejects ambiguous token usage"
