param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\release-decision-selftest" }
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { [void]$script:failures.Add($Message) }
}

function Write-Json($Value, [string]$Path) {
    $dir = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function New-FixtureRun([string]$Name, [string]$CostStatus, [bool]$ScoreValid, [int]$RoutingMistakes) {
    $dir = Join-Path $RunRoot $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Write-Json ([pscustomobject]@{
            status = $CostStatus
            ratios = [pscustomobject]@{
                direct_input_output_ratio = 1.5
                walltime_ratio = 1.2
                model_request_count_ratio = 1
            }
        }) (Join-Path $dir "suite-cost-gate.json")
    Write-Json ([pscustomobject]@{
            run_validity = "valid"
            score_valid = $ScoreValid
            both_success = 1
            both_failed = 0
            excluded_pairs = 0
            excluded_by_reason = [pscustomobject]@{}
        }) (Join-Path $dir "aggregate.json")
    Write-Json ([pscustomobject]@{
            taskspace_projection_count = 1
            missing_taskspace_projection_count = 0
            taskspace_projection_protected_miss_count = 0
        }) (Join-Path $dir "context-projection-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            protected_miss_count = 0
        }) (Join-Path $dir "suite-map-management-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            routing_mistake_count = $RoutingMistakes
            recommended_mode = "thin"
            router_status = "report_only"
            verification_first_expected_format_count = 0
        }) (Join-Path $dir "suite-routing-summary.json")
    Write-Json ([pscustomobject]@{
            logical_mode = "taskspace"
            large_output_replay_count = 0
            runtime_output_ref_created_count = 1
        }) (Join-Path $dir "pair-001\right\artifacts\metrics.json")
    $dir
}

$passDir = New-FixtureRun "pass" "PASS" $true 0
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $passDir *> $null
Assert-True ($LASTEXITCODE -eq 0) "PASS fixture did not exit 0"
$passDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $passDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$passDecision.decision -eq "PASS") "PASS fixture did not write PASS decision"

$failDir = New-FixtureRun "fail" "FAIL" $true 1
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $failDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "FAIL fixture did not exit 1"
$failDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $failDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$failDecision.decision -eq "FAIL") "FAIL fixture did not write FAIL decision"
Assert-True (@($failDecision.blockers) -contains "cost_gate_failed") "FAIL fixture did not report cost blocker"
Assert-True (@($failDecision.blockers) -contains "routing_gate_failed") "FAIL fixture did not report routing blocker"

if ($failures.Count -gt 0) {
    Write-Error ("Release decision self-test failed: " + (@($failures.ToArray()) -join "; "))
    exit 1
}
Write-Host "Release decision self-test: PASS"
Write-Host "RunRoot: $RunRoot"
