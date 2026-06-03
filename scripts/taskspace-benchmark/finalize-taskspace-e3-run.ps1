param(
    [Parameter(Mandatory = $true)][string]$RunDir,
    [switch]$EnableAggregate
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $repoRoot "scripts\action-map-graph-health-lib.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\audit-report.ps1")
. (Join-Path $repoRoot "scripts\taskspace-benchmark\lib\pair-report.ps1")

function Read-JsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-PairRepoDir {
    param([Parameter(Mandatory = $true)][string]$SideDir)
    foreach ($relative in @("repo", "terminal-bench-drive\app")) {
        $candidate = Join-Path $SideDir $relative
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    throw "Cannot locate side repo dir under $SideDir"
}

$run = (Resolve-Path -LiteralPath $RunDir).Path
$promptGuardPath = Join-Path $run "prompt-guard.json"
if (-not (Test-Path -LiteralPath $promptGuardPath)) { throw "prompt-guard.json not found under run dir: $run" }
$promptGuard = Read-JsonFile $promptGuardPath
$pairDirs = @(Get-ChildItem -LiteralPath $run -Directory -Filter "pair-*" | Sort-Object Name)
if ($pairDirs.Count -eq 0) { throw "No pair directories found under run dir: $run" }

$reports = New-Object System.Collections.Generic.List[object]
foreach ($pairDirItem in $pairDirs) {
    $pairDir = $pairDirItem.FullName
    $manifestResolved = Read-JsonFile (Join-Path $pairDir "manifest.resolved.json")
    $leftMetrics = Read-JsonFile (Join-Path $pairDir "left\artifacts\metrics.json")
    $rightMetrics = Read-JsonFile (Join-Path $pairDir "right\artifacts\metrics.json")
    $externalProofPath = Join-Path $pairDir "external-e3-proof.json"
    $externalProof = if (Test-Path -LiteralPath $externalProofPath) { Read-JsonFile $externalProofPath } else { $null }
    $repeat = [int]$manifestResolved.repeat
    $e3MinimumRepeats = 5
    if ($manifestResolved.e3.PSObject.Properties.Name -contains "minimum_repeats") {
        $e3MinimumRepeats = [Math]::Max(5, [int]$manifestResolved.e3.minimum_repeats)
    }
    $oracleLevels = @($leftMetrics.oracle_isolation_level, $rightMetrics.oracle_isolation_level)
    $pairOracleLevel = if ($oracleLevels -contains "failed") {
        "failed"
    } elseif ($oracleLevels -contains "soft_denylist") {
        "soft_denylist"
    } elseif ($oracleLevels -contains "hard_deferred_materialization") {
        "hard_deferred_materialization"
    } else {
        "hard_sandbox"
    }
    $businessSuccess = [bool]($leftMetrics.business_success -or $rightMetrics.business_success)
    $variableControl = Compare-TaskspacePairVariables $manifestResolved $leftMetrics $rightMetrics
    $standardMetrics = @($leftMetrics, $rightMetrics) | Where-Object { $_.logical_mode -eq "standard" } | Select-Object -First 1
    $taskspaceMetrics = @($leftMetrics, $rightMetrics) | Where-Object { $_.logical_mode -eq "taskspace" } | Select-Object -First 1
    $sideOutcomes = [pscustomobject]@{
        standard_success = ($standardMetrics -and [bool]$standardMetrics.business_success)
        taskspace_success = ($taskspaceMetrics -and [bool]$taskspaceMetrics.business_success)
        exec_timeouts = @(@($leftMetrics, $rightMetrics) | Where-Object { $_.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$_.exec_timed_out } | ForEach-Object { "$($_.mode)/$($_.logical_mode)" })
    }
    $claimScope = if ($manifestResolved.e3.PSObject.Properties.Name -contains "claim_scope") { [string]$manifestResolved.e3.claim_scope } else { "" }
    $auditReview = Get-TaskspaceAuditReview $pairDir "" $repeat $claimScope
    $evidence = Get-TaskspaceEvidenceGate $pairDirs.Count $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $true ([string]$manifestResolved.oracle_isolation_policy) "E3" $manifestResolved.sample_origin $manifestResolved.external_benchmark $manifestResolved.e3 ([bool]$manifestResolved.human_review_required) $auditReview.completed $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes
    $evidence | Add-Member -NotePropertyName audit_review_source_path -NotePropertyValue $auditReview.source_path -Force
    $evidence | Add-Member -NotePropertyName audit_review_failures -NotePropertyValue @($auditReview.failures) -Force
    if ($externalProof) {
        $evidence | Add-Member -NotePropertyName external_runtime_proof_path -NotePropertyValue $externalProof.runtime_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_runner_equivalence_proof_path -NotePropertyValue $externalProof.runner_equivalence_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_isolation_proof_path -NotePropertyValue $externalProof.isolation_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_combined_proof_path -NotePropertyValue $externalProof.combined_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_proof_official_runner_or_equivalent -NotePropertyValue $externalProof.validator_fidelity.official_runner_or_equivalent -Force
        $evidence | Add-Member -NotePropertyName external_proof_agent_cannot_read_validator_source -NotePropertyValue $externalProof.validator_fidelity.agent_cannot_read_validator_source -Force
        $evidence | Add-Member -NotePropertyName external_proof_validator_e3_eligible -NotePropertyValue $externalProof.validator_fidelity.e3_eligible -Force
    }
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
    $pair = [pscustomobject]@{
        Repeat = $repeat
        PairDir = $pairDir
        Left = [pscustomobject]@{ LogicalMode = [string]$leftMetrics.logical_mode; RepoDir = Get-PairRepoDir (Join-Path $pairDir "left"); ArtifactDir = Join-Path $pairDir "left\artifacts" }
        Right = [pscustomobject]@{ LogicalMode = [string]$rightMetrics.logical_mode; RepoDir = Get-PairRepoDir (Join-Path $pairDir "right"); ArtifactDir = Join-Path $pairDir "right\artifacts" }
    }
    $pairReportPath = Join-Path $pairDir "pair-report.md"
    Write-TaskspacePairReport $pairReportPath $manifestForReport $promptGuard $variableControl $evidence $leftMetrics $rightMetrics $pair
    $postWriteAudit = Get-TaskspaceAuditReview $pairDir "" $repeat $claimScope
    if (-not $postWriteAudit.completed) {
        Write-Host "AuditReviewPostWriteFailure: pair-$('{0:000}' -f $repeat) $(@($postWriteAudit.failures) -join ', ')"
        $evidence = Get-TaskspaceEvidenceGate $pairDirs.Count $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $true ([string]$manifestResolved.oracle_isolation_policy) "E3" $manifestResolved.sample_origin $manifestResolved.external_benchmark $manifestResolved.e3 ([bool]$manifestResolved.human_review_required) $false $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes
        $evidence | Add-Member -NotePropertyName audit_review_source_path -NotePropertyValue $postWriteAudit.source_path -Force
        $evidence | Add-Member -NotePropertyName audit_review_failures -NotePropertyValue @($postWriteAudit.failures) -Force
        Write-TaskspacePairReport $pairReportPath $manifestForReport $promptGuard $variableControl $evidence $leftMetrics $rightMetrics $pair
    }
    $reports.Add([pscustomobject]@{ repeat = $repeat; pair_dir = $pairDir; pair_report = $pairReportPath; evidence_target = "E3"; evidence = $evidence })
}

$summaryPath = Join-Path $run "run-summary.md"
Write-TaskspaceRunSummary -Path $summaryPath -Reports @($reports.ToArray())
if ($EnableAggregate) { Write-TaskspaceAggregateReport -Path (Join-Path $run "aggregate-report.md") -Reports @($reports.ToArray()) }
Write-Host "RunSummary: $summaryPath"
$failed = @(Get-TaskspaceFailedReports $reports "E3")
if ($failed.Count -gt 0) { exit 1 }
