param(
    [string]$Scenario = "",
    [string]$ScenarioPath = "",
    [string]$TaskListPath = "",
    [string]$SourceVersion = "",
    [string]$ExpectedTaskListHash = "",
    [string]$ExpectedProfileHash = "",
    [ValidateSet("", "deepswe", "terminal-bench")]
    [string]$Benchmark = "",
    [int]$Repeats = 0,
    [string]$OnePairSmokeRoot = "",
    [string]$SerialCalibrationRoot = "",
    [string]$ParallelEquivalencePath = "",
    [string]$V005NonAgentGatesPath = "",
    [string]$V005CodeCompleteMarkerPath = "",
    [string]$V005UserApprovalMarkerPath = "",
    [string]$ExpectedSampleSetId = "terminal-bench_E3-P0_3_5",
    [string]$RunRoot = "",
    [string]$OutputDir = "",
    [switch]$RunSelfTests,
    [switch]$AllowSkippedPathContract,
    [switch]$AllowSkippedSelfTests,
    [switch]$AllowSkippedOnePairSmoke,
    [switch]$AllowSkippedCalibrationGate
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot ("target\e3-start-gate\{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
}

$gate = Invoke-TaskspaceE3StartGate `
    -RepoRoot $repoRoot `
    -BenchmarkRoot $PSScriptRoot `
    -OutputDir $OutputDir `
    -Scenario $Scenario `
    -ScenarioPath $ScenarioPath `
    -RunRoot $RunRoot `
    -TaskListPath $TaskListPath `
    -SourceVersion $SourceVersion `
    -ExpectedTaskListHash $ExpectedTaskListHash `
    -ExpectedProfileHash $ExpectedProfileHash `
    -Benchmark $Benchmark `
    -Repeats $Repeats `
    -OnePairSmokeRoot $OnePairSmokeRoot `
    -SerialCalibrationRoot $SerialCalibrationRoot `
    -ParallelEquivalencePath $ParallelEquivalencePath `
    -V005NonAgentGatesPath $V005NonAgentGatesPath `
    -V005CodeCompleteMarkerPath $V005CodeCompleteMarkerPath `
    -V005UserApprovalMarkerPath $V005UserApprovalMarkerPath `
    -ExpectedSampleSetId $ExpectedSampleSetId `
    -RunSelfTests:$RunSelfTests `
    -AllowSkippedPathContract:$AllowSkippedPathContract `
    -AllowSkippedSelfTests:$AllowSkippedSelfTests `
    -AllowSkippedOnePairSmoke:$AllowSkippedOnePairSmoke `
    -AllowSkippedCalibrationGate:$AllowSkippedCalibrationGate

Write-Host "E3StartGate: $($gate.json_path)"
Write-Host "E3StartGateReport: $($gate.markdown_path)"
Write-Host "GateDecision: $($gate.gate_decision_path)"
exit ([int]$gate.exit_code)
