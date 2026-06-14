$ErrorActionPreference = "Stop"

function Get-TaskspaceComparableSampleMap {
    param($SuiteHealth)
    $map = @{}
    foreach ($status in @($SuiteHealth.sample_statuses)) {
        $sampleId = if ($status.PSObject.Properties.Name -contains "sample_id") { [string]$status.sample_id } else { "" }
        if ([string]::IsNullOrWhiteSpace($sampleId)) { continue }
        $map[$sampleId] = $status
    }
    $map
}

function Get-TaskspaceComparableField {
    param($Object, [string]$Field)
    if ($Object -and $Object.PSObject.Properties.Name -contains $Field) { return $Object.$Field }
    $null
}

function Compare-TaskspaceScalarField {
    param(
        [Parameter(Mandatory = $true)]$Drifts,
        [Parameter(Mandatory = $true)][string]$Scope,
        [Parameter(Mandatory = $true)][string]$Field,
        $SerialObject,
        $ParallelObject
    )
    $serialValue = Get-TaskspaceComparableField $SerialObject $Field
    $parallelValue = Get-TaskspaceComparableField $ParallelObject $Field
    if ([string]$serialValue -ne [string]$parallelValue) {
        $Drifts.Add([pscustomobject]@{
                scope = $Scope
                field = $Field
                serial = $serialValue
                parallel = $parallelValue
            })
    }
}

function Compare-TaskspaceSuiteScoreEquivalence {
    param(
        [Parameter(Mandatory = $true)]$SerialSuiteHealth,
        [Parameter(Mandatory = $true)]$ParallelSuiteHealth
    )
    $drifts = New-Object System.Collections.Generic.List[object]
    foreach ($field in @(
            "status",
            "suite_score_valid",
            "completed_child_processes",
            "score_valid_child_runs",
            "score_invalid_child_runs",
            "first_score_invalid_run",
            "invalid_harness_sample_count",
            "remaining_samples_skipped",
            "remaining_pairs_skipped"
        )) {
        Compare-TaskspaceScalarField $drifts "suite" $field $SerialSuiteHealth $ParallelSuiteHealth
    }

    $serialMap = Get-TaskspaceComparableSampleMap $SerialSuiteHealth
    $parallelMap = Get-TaskspaceComparableSampleMap $ParallelSuiteHealth
    $serialIds = @($serialMap.Keys | Sort-Object)
    $parallelIds = @($parallelMap.Keys | Sort-Object)
    if (($serialIds -join "`n") -ne ($parallelIds -join "`n")) {
        $drifts.Add([pscustomobject]@{
                scope = "suite"
                field = "sample_id_set"
                serial = @($serialIds)
                parallel = @($parallelIds)
            })
    }

    foreach ($sampleId in @($serialIds)) {
        if (-not $parallelMap.ContainsKey($sampleId)) { continue }
        $serialStatus = $serialMap[$sampleId]
        $parallelStatus = $parallelMap[$sampleId]
        foreach ($field in @(
                "run_validity",
                "phase",
                "exit_code",
                "attempted_pairs",
                "completed_pairs",
                "abort_scope",
                "abort_phase",
                "abort_signature",
                "abort_reason",
                "skipped_reason",
                "score_ready",
                "score_valid",
                "score_block_reason",
                "score_invalid_reason",
                "audit_required",
                "audit_status",
                "proof_status",
                "profile_hash",
                "prompt_hash",
                "config_hash",
                "included_in_e3_aggregate",
                "included_in_utility_aggregate",
                "outcome_standard",
                "outcome_taskspace"
            )) {
            Compare-TaskspaceScalarField $drifts "sample:$sampleId" $field $serialStatus $parallelStatus
        }
    }

    [pscustomobject]@{
        schema_version = 1
        comparable = ($drifts.Count -eq 0)
        parallel_smoke_score_drift = ($drifts.Count -gt 0)
        drift_count = $drifts.Count
        drifts = @($drifts.ToArray())
        compared_sample_ids = @($serialIds)
    }
}

function Write-TaskspaceSuiteScoreEquivalence {
    param(
        [Parameter(Mandatory = $true)][string]$SerialSuiteHealthPath,
        [Parameter(Mandatory = $true)][string]$ParallelSuiteHealthPath,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [string]$TaskListHash = "",
        [string]$SourceVersion = "",
        [string]$ProfileHash = ""
    )
    $serial = Get-Content -Raw -Encoding UTF8 -LiteralPath $SerialSuiteHealthPath | ConvertFrom-Json
    $parallel = Get-Content -Raw -Encoding UTF8 -LiteralPath $ParallelSuiteHealthPath | ConvertFrom-Json
    $result = Compare-TaskspaceSuiteScoreEquivalence $serial $parallel
    $result | Add-Member -NotePropertyName task_list_hash -NotePropertyValue $TaskListHash -Force
    $result | Add-Member -NotePropertyName source_version -NotePropertyValue $SourceVersion -Force
    $result | Add-Member -NotePropertyName profile_hash -NotePropertyValue $ProfileHash -Force
    $result | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    $result
}
