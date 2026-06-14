param(
    [string]$Scenario = "",
    [string]$ScenarioPath = "",
    [string]$TaskListPath = "",
    [string]$SourceVersion = "",
    [string]$OnePairSmokeRoot = "",
    [string]$RunRoot = "",
    [string]$OutputDir = "",
    [switch]$RunSelfTests,
    [switch]$AllowSkippedPathContract,
    [switch]$AllowSkippedSelfTests,
    [switch]$AllowSkippedOnePairSmoke
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
    -OnePairSmokeRoot $OnePairSmokeRoot `
    -RunSelfTests:$RunSelfTests `
    -AllowSkippedPathContract:$AllowSkippedPathContract `
    -AllowSkippedSelfTests:$AllowSkippedSelfTests `
    -AllowSkippedOnePairSmoke:$AllowSkippedOnePairSmoke

Write-Host "E3StartGate: $($gate.json_path)"
Write-Host "E3StartGateReport: $($gate.markdown_path)"
exit ([int]$gate.exit_code)
