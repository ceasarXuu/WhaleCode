param(
    [Parameter(Mandatory = $true)][string]$RunDir,
    [switch]$EnableAggregate,
    [switch]$AllowInvalidHarnessFinalize
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $repoRoot "scripts\action-map-graph-health-lib.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\audit-report.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\graph-health.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\failure-taxonomy.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\audit-manifest.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\pair-artifact-classifier.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\pair-report.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\report-summary.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\aggregate-report.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\timing.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\cost-instrumentation.ps1")

function Read-JsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-PairRepoDir {
    param([Parameter(Mandatory = $true)][string]$SideDir)
    foreach ($relative in @("repo", "app", "terminal-bench-drive\app")) {
        $candidate = Join-Path $SideDir $relative
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    throw "Cannot locate side repo dir under $SideDir"
}

$run = (Resolve-Path -LiteralPath $RunDir).Path
$sampleStatusPath = Join-Path $run "sample-status.json"
if ((Test-Path -LiteralPath $sampleStatusPath) -and -not $AllowInvalidHarnessFinalize) {
    $sampleStatus = Read-JsonFile $sampleStatusPath
    if ($sampleStatus.PSObject.Properties.Name -contains "run_validity" -and [string]$sampleStatus.run_validity -eq "invalid_harness") {
        Write-Error "Run is invalid_harness; use -AllowInvalidHarnessFinalize only for forensic report rebuilds: $sampleStatusPath"
        exit 3
    }
}
$promptGuardPath = Join-Path $run "prompt-guard.json"
if (-not (Test-Path -LiteralPath $promptGuardPath)) { throw "prompt-guard.json not found under run dir: $run" }
$promptGuard = Read-JsonFile $promptGuardPath
$pairDirs = @(Get-ChildItem -LiteralPath $run -Directory -Filter "pair-*" | Sort-Object Name)
if ($pairDirs.Count -eq 0) { throw "No pair directories found under run dir: $run" }

$reports = New-Object System.Collections.Generic.List[object]
$sampleId = ""
foreach ($pairDirItem in $pairDirs) {
    $pairDir = $pairDirItem.FullName
    $classified = Get-TaskspacePairEvidenceFromArtifacts $pairDir $pairDirs.Count $promptGuard $true "" "E3"
    $manifestResolved = $classified.manifest_resolved
    if ([string]::IsNullOrWhiteSpace($sampleId)) { $sampleId = [string]$manifestResolved.scenario }
    $repeat = [int]$manifestResolved.repeat
    $evidence = $classified.evidence
    $manifestForReport = [pscustomobject]@{
        Id = [string]$manifestResolved.scenario
        Level = "L3"
        EvidenceTarget = "E3"
        SampleOrigin = $manifestResolved.sample_origin
        ExternalBenchmark = $manifestResolved.external_benchmark
        E3 = $manifestResolved.e3
        HumanReviewRequired = [bool]$manifestResolved.human_review_required
        Expected = [pscustomobject]@{}
        Thresholds = [pscustomobject]@{}
    }
    $pairReportPath = Join-Path $pairDir "pair-report.md"
    Write-TaskspacePairReport $pairReportPath $manifestForReport $promptGuard $classified.variable_control $evidence $classified.left_metrics $classified.right_metrics $classified.pair
    $reports.Add([pscustomobject]@{ repeat = $repeat; pair_dir = $pairDir; pair_report = $pairReportPath; evidence_target = "E3"; evidence = $evidence })
}

$summaryPath = Join-Path $run "run-summary.md"
Write-TaskspaceRunSummary -Path $summaryPath -Reports @($reports.ToArray())
if ([string]::IsNullOrWhiteSpace($sampleId)) { $sampleId = Split-Path -Leaf $run }
$sampleTimingPath = Write-TaskspaceSampleTiming $run $sampleId
$costAggregate = Write-TaskspaceCostAggregateArtifacts -RootDir $run -Scope "sample"
if ($EnableAggregate) { Write-TaskspaceAggregateReport -Path (Join-Path $run "aggregate-report.md") -Reports @($reports.ToArray()) }
[pscustomobject]@{
    schema_version = 1
    rerender_mode = "artifact_only"
    validation_rerun_allowed = $false
    hidden_oracle_rerun_allowed = $false
    pair_count = $reports.Count
    sample_timing_path = $sampleTimingPath
    suite_cost_gate_path = [string]$costAggregate.suite_cost_gate_path
    generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $run "finalize-health.json") -Encoding UTF8
Write-Host "RunSummary: $summaryPath"
$failed = @(Get-TaskspaceFailedReports $reports "E3")
if ($failed.Count -gt 0) { exit 1 }
