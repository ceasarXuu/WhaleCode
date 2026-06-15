function Read-TaskspaceBenchmarkJsonFile {
    param([Parameter(Mandatory = $true)][string]$Path)
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-TaskspacePairRepoDir {
    param([Parameter(Mandatory = $true)][string]$SideDir)
    foreach ($relative in @("repo", "app", "terminal-bench-drive\app")) {
        $candidate = Join-Path $SideDir $relative
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    throw "Cannot locate side repo dir under $SideDir"
}

function Get-TaskspacePairEvidenceFromArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][int]$Repeats,
        [Parameter(Mandatory = $true)]$PromptGuard,
        [Parameter(Mandatory = $true)][bool]$AggregateEligible,
        [string]$AuditReviewRoot = "",
        [string]$EvidenceTarget = "E3",
        $Probe = $null
    )
    $manifestResolved = Read-TaskspaceBenchmarkJsonFile (Join-Path $PairDir "manifest.resolved.json")
    $leftMetrics = Read-TaskspaceBenchmarkJsonFile (Join-Path $PairDir "left\artifacts\metrics.json")
    $rightMetrics = Read-TaskspaceBenchmarkJsonFile (Join-Path $PairDir "right\artifacts\metrics.json")
    $externalProofPath = Join-Path $PairDir "external-e3-proof.json"
    $externalProof = if (Test-Path -LiteralPath $externalProofPath) { Read-TaskspaceBenchmarkJsonFile $externalProofPath } else { $null }
    $variableControl = Compare-TaskspacePairVariables $manifestResolved $leftMetrics $rightMetrics
    $oracleLevels = @($leftMetrics.oracle_isolation_level, $rightMetrics.oracle_isolation_level)
    if ($Probe -and $Probe.PSObject.Properties.Name -contains "oracle_isolation_level") { $oracleLevels += $Probe.oracle_isolation_level }
    $pairOracleLevel = if ($oracleLevels -contains "failed") {
        "failed"
    } elseif ($oracleLevels -contains "soft_denylist") {
        "soft_denylist"
    } elseif ($oracleLevels -contains "hard_deferred_materialization") {
        "hard_deferred_materialization"
    } else {
        "hard_sandbox"
    }
    $standardMetrics = @($leftMetrics, $rightMetrics) | Where-Object { $_.logical_mode -eq "standard" } | Select-Object -First 1
    $taskspaceMetrics = @($leftMetrics, $rightMetrics) | Where-Object { $_.logical_mode -eq "taskspace" } | Select-Object -First 1
    $sideOutcomes = [pscustomobject]@{
        standard_success = ($standardMetrics -and [bool]$standardMetrics.business_success)
        taskspace_success = ($taskspaceMetrics -and [bool]$taskspaceMetrics.business_success)
        exec_timeouts = @(@($leftMetrics, $rightMetrics) | Where-Object { $_.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$_.exec_timed_out } | ForEach-Object { "$($_.mode)/$($_.logical_mode)" })
    }
    $e3MinimumRepeats = 5
    if ($manifestResolved.e3 -and $manifestResolved.e3.PSObject.Properties.Name -contains "minimum_repeats") {
        $e3MinimumRepeats = [Math]::Max(5, [int]$manifestResolved.e3.minimum_repeats)
    }
    $claimScope = if ($manifestResolved.e3 -and $manifestResolved.e3.PSObject.Properties.Name -contains "claim_scope") { [string]$manifestResolved.e3.claim_scope } else { "" }
    $auditReview = Get-TaskspaceAuditReview $PairDir $AuditReviewRoot ([int]$manifestResolved.repeat) $claimScope
    $metricsTaints = @(@($leftMetrics, $rightMetrics) | ForEach-Object { @($_.metrics_taints) } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    $environmentFailures = @(@($leftMetrics, $rightMetrics) | ForEach-Object { @($_.validator_environment_failures) } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    $businessSuccess = [bool]($leftMetrics.business_success -or $rightMetrics.business_success)
    $evidence = Get-TaskspaceEvidenceGate $Repeats $PromptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $AggregateEligible ([string]$manifestResolved.oracle_isolation_policy) $EvidenceTarget $manifestResolved.sample_origin $manifestResolved.external_benchmark $manifestResolved.e3 ([bool]$manifestResolved.human_review_required) $auditReview.completed $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes $metricsTaints $environmentFailures
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
    $auditManifest = Write-TaskspaceAuditManifest $PairDir $manifestResolved $leftMetrics $rightMetrics $evidence $variableControl $auditReview
    $evidence | Add-Member -NotePropertyName audit_manifest_path -NotePropertyValue $auditManifest.json_path -Force
    $evidence | Add-Member -NotePropertyName failure_taxonomy -NotePropertyValue @($auditManifest.failure_taxonomy) -Force
    $evidence | Add-Member -NotePropertyName utility_direction -NotePropertyValue $auditManifest.utility_direction -Force
    $evidence | Add-Member -NotePropertyName run_score_valid -NotePropertyValue ([bool]$auditManifest.run_score_valid) -Force
    $evidence | Add-Member -NotePropertyName engineering_unclean -NotePropertyValue ([bool]$auditManifest.engineering_unclean) -Force
    $evidence | Add-Member -NotePropertyName engineering_unclean_reasons -NotePropertyValue @($auditManifest.engineering_unclean_reasons) -Force
    $evidence | Add-Member -NotePropertyName outcome_standard -NotePropertyValue ([string]$auditManifest.outcome_standard) -Force
    $evidence | Add-Member -NotePropertyName outcome_taskspace -NotePropertyValue ([string]$auditManifest.outcome_taskspace) -Force
    $evidence | Add-Member -NotePropertyName score_exclusion_reason -NotePropertyValue ([string]$auditManifest.score_exclusion_reason) -Force
    $pair = [pscustomobject]@{
        Repeat = [int]$manifestResolved.repeat
        PairDir = $PairDir
        Left = [pscustomobject]@{ LogicalMode = [string]$leftMetrics.logical_mode; RepoDir = Get-TaskspacePairRepoDir (Join-Path $PairDir "left"); ArtifactDir = Join-Path $PairDir "left\artifacts" }
        Right = [pscustomobject]@{ LogicalMode = [string]$rightMetrics.logical_mode; RepoDir = Get-TaskspacePairRepoDir (Join-Path $PairDir "right"); ArtifactDir = Join-Path $PairDir "right\artifacts" }
    }
    [pscustomobject]@{
        manifest_resolved = $manifestResolved
        left_metrics = $leftMetrics
        right_metrics = $rightMetrics
        variable_control = $variableControl
        audit_review = $auditReview
        audit_manifest = $auditManifest
        evidence = $evidence
        pair = $pair
    }
}
