param([string]$RunRoot = "")

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/run-state.ps1")

$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}
function New-SideMetrics([string]$Side, [string]$Mode, [bool]$Success, [int]$OracleExit = 0) {
    [pscustomobject]@{
        mode = $Side
        logical_mode = $Mode
        business_success = $Success
        exec_exit_code = 0
        exec_timed_out = $false
        public_validation_exit_code = 0
        hidden_oracle_exit_code = $OracleExit
    }
}

$passing = @(
    New-SideMetrics "left" "standard" $true
    New-SideMetrics "right" "taskspace" $true
)
$disabled = Get-TaskspacePairStopDecision $false $passing
$enabledPassing = Get-TaskspacePairStopDecision $true $passing
Assert-True (-not [bool]$disabled.stop -and -not [bool]$enabledPassing.stop) "passing pair triggered the stop condition"

$failedBusiness = Get-TaskspacePairStopDecision $true @(
    New-SideMetrics "left" "standard" $true
    New-SideMetrics "right" "taskspace" $false
)
Assert-True ([bool]$failedBusiness.stop -and [string]$failedBusiness.code -eq "any_side_failure") "failed TaskSpace side did not stop the batch"
Assert-True ([string]$failedBusiness.failed_sides[0].logical_mode -eq "taskspace" -and @($failedBusiness.failed_sides[0].reasons) -contains "business_success_false") "failed side identity or reason was lost"

$failedOracle = Get-TaskspacePairStopDecision $true @(
    New-SideMetrics "left" "standard" $true 1
)
Assert-True ([bool]$failedOracle.stop -and @($failedOracle.failed_sides[0].reasons) -contains "hidden_oracle_exit_code_nonzero") "oracle failure did not stop the batch"

$runnerText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot "run-taskspace-benchmark-pairs.ps1")
$entryText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot "run-taskspace-benchmark.ps1")
Assert-True ($entryText -match '\[switch\]\$StopOnAnySideFailure') "runner does not expose the executable stop contract"
Assert-True ($runnerText -match 'Get-TaskspacePairStopDecision' -and $runnerText -match 'run_stop_condition_triggered' -and $runnerText -match '(?m)^\s*break\s*$') "pair loop does not enforce and record the stop decision"
Assert-True ($entryText -match '\$completedPairCount = @\(\$pairReports\.ToArray\(\)\)\.Count') "finalization does not use the actual completed pair count"
Assert-True ($entryText -match 'phase -eq "stopped"' -and $entryText -match 'cannot be resumed without -ForceRerun') "stopped run can be resumed accidentally"

if (-not $RunRoot) { $RunRoot = Join-Path ([IO.Path]::GetTempPath()) "taskspace-stop-$([guid]::NewGuid().ToString('N'))" }
Initialize-TaskspaceBenchmarkRunState $RunRoot "fixture" 3 "E2" "fixture" | Out-Null
Set-TaskspaceBenchmarkRunPhase $RunRoot "stopped" 1 1 $false | Out-Null
$runStatus = Read-TaskspaceRunStatus $RunRoot
$sampleStatus = Set-TaskspaceSampleStatus $RunRoot "fixture" "stopped" 1 1
Assert-True ([int]$runStatus.completed_pairs -eq 1 -and [int]$runStatus.exit_code -eq 1 -and -not [bool]$runStatus.resume_allowed -and [bool]$runStatus.force_rerun_required) "stopped run status was reported as a normal completion"
Assert-True ([int]$sampleStatus.completed_pairs -eq 1 -and [int]$sampleStatus.exit_code -eq 1 -and -not [bool]$sampleStatus.resume_allowed -and [string]$sampleStatus.abort_phase -eq "stop_condition") "stopped sample status was reported as a normal completion"
Remove-Item -LiteralPath $RunRoot -Recurse -Force -ErrorAction SilentlyContinue

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "run stop condition selftest passed"
