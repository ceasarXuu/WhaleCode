$ErrorActionPreference = "Stop"

function Get-TaskspaceRuntimeSpeedDecision {
    param(
        $Timing,
        [bool]$ScoreValid = $true
    )
    if (-not $Timing) {
        return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "timing_artifact_missing" }
    }
    if (-not $ScoreValid) {
        return [pscustomobject]@{ decision = "speedup_blocked_invalid_run"; reason = "score_valid_false" }
    }
    if ($Timing.PSObject.Properties.Name -contains "runtime_optimization_status" -and [string]$Timing.runtime_optimization_status -eq "blocked") {
        return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "runtime_optimization_status_blocked" }
    }
    $explicitApproval = (
        $Timing.PSObject.Properties.Name -contains "speedup_target_evidence_status" -and
        [string]$Timing.speedup_target_evidence_status -eq "approved"
    )
    $serialBaseline = (
        $Timing.PSObject.Properties.Name -contains "serial_baseline_available" -and
        [bool]$Timing.serial_baseline_available
    )
    $parallelSmoke = (
        $Timing.PSObject.Properties.Name -contains "governed_parallel_smoke_passed" -and
        [bool]$Timing.governed_parallel_smoke_passed
    )
    $scoreDrift = (
        $Timing.PSObject.Properties.Name -contains "parallel_smoke_score_drift" -and
        [bool]$Timing.parallel_smoke_score_drift
    )
    if ($explicitApproval -or ($serialBaseline -and $parallelSmoke -and -not $scoreDrift)) {
        return [pscustomobject]@{ decision = "speedup_target_approved"; reason = "serial_baseline_and_governed_parallel_smoke_passed" }
    }
    $class = if ($Timing.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$Timing.bottleneck_classification } else { "unknown" }
    switch ($class) {
        "agent_bound" { return [pscustomobject]@{ decision = "speedup_limited_agent_bound"; reason = "agent_execution_dominates_clean_wall_time" } }
        "validator_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "validation_or_oracle_dominates_clean_wall_time" } }
        "docker_build_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "docker_build_dominates_clean_wall_time" } }
        "docker_run_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "docker_run_dominates_clean_wall_time" } }
        "cleanup_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "cleanup_or_storage_overhead_requires_harness_work" } }
        "storage_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "storage_overhead_requires_harness_work" } }
        "model_queue_bound" { return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "model_queue_requires_resource_governor_calibration" } }
        "queue_bound" { return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "queue_wait_requires_resource_governor_calibration" } }
        "engineering_unclean_slow" { return [pscustomobject]@{ decision = "speedup_blocked_invalid_run"; reason = "engineering_unclean_slow" } }
        "mixed" { return [pscustomobject]@{ decision = "speedup_candidate_parallelism"; reason = "timing_complete_without_single_dominant_phase" } }
        "mixed_or_unclassified" { return [pscustomobject]@{ decision = "speedup_candidate_parallelism"; reason = "timing_complete_without_single_dominant_phase" } }
        "unknown" { return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "bottleneck_classification_unknown" } }
        default { return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "unrecognized_bottleneck_classification:$class" } }
    }
}

function Test-TaskspaceRuntimeSpeedEvidenceValid {
    param(
        $Timing,
        [bool]$ScoreValid = $true,
        [string]$TimingParseError = ""
    )
    if (-not $ScoreValid) { return $false }
    if ($TimingParseError) { return $false }
    if (-not $Timing) { return $false }
    if ($Timing.PSObject.Properties.Name -contains "timing_quality" -and [string]$Timing.timing_quality -ne "complete") { return $false }
    if (-not ($Timing.PSObject.Properties.Name -contains "timing_quality")) { return $false }
    if ($Timing.PSObject.Properties.Name -contains "runtime_optimization_status" -and [string]$Timing.runtime_optimization_status -eq "blocked") { return $false }
    if ($Timing.PSObject.Properties.Name -contains "speedup_evidence_valid") { return [bool]$Timing.speedup_evidence_valid }
    return $true
}

function Get-TaskspaceRuntimePhaseRows {
    param($Timing)
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($name in @(
            "agent_duration_ms",
            "model_request_duration_ms",
            "public_validation_duration_ms",
            "hidden_oracle_duration_ms",
            "docker_build_duration_ms",
            "docker_run_duration_ms",
            "docker_cleanup_duration_ms",
            "process_launch_wait_ms",
            "resource_wait_ms_total"
        )) {
        $value = if ($Timing -and $Timing.PSObject.Properties.Name -contains $name -and $null -ne $Timing.$name) { [int64]$Timing.$name } else { $null }
        $rows.Add([pscustomobject]@{ name = $name; duration_ms = $value })
    }
    @($rows.ToArray())
}

function Get-TaskspaceRuntimeUniqueStrings {
    param($Values)
    @($Values | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | ForEach-Object { [string]$_ } | Sort-Object -Unique)
}

function Write-TaskspaceRuntimeBottleneckReport {
    param(
        [Parameter(Mandatory = $true)][string]$TimingPath,
        [string]$OutputPath = "",
        [bool]$ScoreValid = $true
    )
    if (-not $OutputPath) { $OutputPath = Join-Path (Split-Path -Parent $TimingPath) "runtime-bottleneck.md" }
    $jsonPath = [System.IO.Path]::ChangeExtension($OutputPath, ".json")
    $timing = $null
    $parseError = ""
    if (Test-Path -LiteralPath $TimingPath) {
        try { $timing = Get-Content -Raw -Encoding UTF8 -LiteralPath $TimingPath | ConvertFrom-Json } catch { $parseError = [string]$_.Exception.Message }
    }
    $decision = if ($parseError) {
        [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "timing_parse_error" }
    } else {
        Get-TaskspaceRuntimeSpeedDecision $timing $ScoreValid
    }
    $speedupEvidenceValid = Test-TaskspaceRuntimeSpeedEvidenceValid $timing $ScoreValid $parseError
    $blockersForJson = if ($timing -and $timing.PSObject.Properties.Name -contains "runtime_optimization_blockers") { @(Get-TaskspaceRuntimeUniqueStrings $timing.runtime_optimization_blockers) } else { @() }
    $missingForJson = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_missing_fields") { @(Get-TaskspaceRuntimeUniqueStrings $timing.wait_attribution_missing_fields) } else { @() }
    $unavailableForJson = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_unavailable_fields") { $timing.wait_attribution_unavailable_fields } else { [pscustomobject]@{} }
    $jsonArtifact = [ordered]@{
        schema_version = 1
        timing_path = $TimingPath
        report_path = $OutputPath
        score_valid = $ScoreValid
        speedup_evidence_valid = $speedupEvidenceValid
        speedup_decision = [string]$decision.decision
        speedup_decision_reason = [string]$decision.reason
        timing_parse_error = $parseError
        timing_quality = if ($timing -and $timing.PSObject.Properties.Name -contains "timing_quality") { [string]$timing.timing_quality } else { "" }
        runtime_optimization_status = if ($timing -and $timing.PSObject.Properties.Name -contains "runtime_optimization_status") { [string]$timing.runtime_optimization_status } else { "" }
        bottleneck_classification = if ($timing -and $timing.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$timing.bottleneck_classification } else { "" }
        wait_attribution_status = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_status") { [string]$timing.wait_attribution_status } else { "" }
        resource_wait_attribution_mode = if ($timing -and $timing.PSObject.Properties.Name -contains "resource_wait_attribution_mode") { [string]$timing.resource_wait_attribution_mode } else { "" }
        runtime_optimization_blockers = @($blockersForJson)
        wait_attribution_missing_fields = @($missingForJson)
        wait_attribution_unavailable_fields = $unavailableForJson
        phase_durations = @(if ($timing) { Get-TaskspaceRuntimePhaseRows $timing } else { @() })
        top_spans = if ($timing -and $timing.PSObject.Properties.Name -contains "timing_breakdown" -and $timing.timing_breakdown) { @($timing.timing_breakdown.top_spans) } else { @() }
        repeated_docker_cache_keys = if ($timing -and $timing.PSObject.Properties.Name -contains "repeated_docker_cache_keys") { @(Get-TaskspaceRuntimeUniqueStrings $timing.repeated_docker_cache_keys) } else { @() }
        generated_at = (Get-Date).ToString("o")
    }
    $jsonArtifact | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Runtime Bottleneck Report")
    $lines.Add("")
    $lines.Add("- timing_path: $TimingPath")
    $lines.Add("- score_valid: $ScoreValid")
    $lines.Add("- speedup_evidence_valid: $speedupEvidenceValid")
    $lines.Add("- speedup_decision: $($decision.decision)")
    $lines.Add("- speedup_decision_reason: $($decision.reason)")
    if ($parseError) { $lines.Add("- timing_parse_error: $parseError") }
    if ($timing) {
        $lines.Add("- timing_quality: $(if ($timing.PSObject.Properties.Name -contains 'timing_quality') { $timing.timing_quality } else { '' })")
        $lines.Add("- runtime_optimization_status: $(if ($timing.PSObject.Properties.Name -contains 'runtime_optimization_status') { $timing.runtime_optimization_status } else { '' })")
        $lines.Add("- bottleneck_classification: $(if ($timing.PSObject.Properties.Name -contains 'bottleneck_classification') { $timing.bottleneck_classification } else { '' })")
        $lines.Add("- wait_attribution_status: $(if ($timing.PSObject.Properties.Name -contains 'wait_attribution_status') { $timing.wait_attribution_status } else { '' })")
        $lines.Add("- resource_wait_attribution_mode: $(if ($timing.PSObject.Properties.Name -contains 'resource_wait_attribution_mode') { $timing.resource_wait_attribution_mode } else { '' })")
        $blockers = @($blockersForJson)
        $lines.Add("- runtime_optimization_blockers: $(if ($blockers.Count -eq 0) { 'none' } else { $blockers -join ', ' })")
        $missing = @($missingForJson)
        $lines.Add("- wait_attribution_missing_fields: $(if ($missing.Count -eq 0) { 'none' } else { $missing -join ', ' })")
        $unavailablePairs = @($unavailableForJson.PSObject.Properties | ForEach-Object { "$($_.Name)=$($_.Value)" })
        $lines.Add("- wait_attribution_unavailable_fields: $(if ($unavailablePairs.Count -eq 0) { 'none' } else { $unavailablePairs -join ', ' })")
        $lines.Add("")
        $lines.Add("## Phase Durations")
        foreach ($row in @(Get-TaskspaceRuntimePhaseRows $timing)) {
            $lines.Add("- $($row.name): $(if ($null -eq $row.duration_ms) { 'null' } else { "$($row.duration_ms)ms" })")
        }
        if ($timing.PSObject.Properties.Name -contains "timing_breakdown" -and $timing.timing_breakdown) {
            $lines.Add("")
            $lines.Add("## Top Spans")
            foreach ($span in @($timing.timing_breakdown.top_spans)) {
                $lines.Add("- $($span.name): $($span.duration_ms)ms")
            }
        }
        if ($timing.PSObject.Properties.Name -contains "repeated_docker_cache_keys") {
            $keys = @($timing.repeated_docker_cache_keys)
            $lines.Add("")
            $lines.Add("## Docker Cache")
            $lines.Add("- repeated_docker_cache_keys: $(if ($keys.Count -eq 0) { 'none' } else { $keys -join ', ' })")
        }
    }
    $lines | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    $OutputPath
}

function Write-TaskspaceRuntimeCalibrationReport {
    param(
        [Parameter(Mandatory = $true)][string]$TimingPath,
        [string]$OutputPath = "",
        [bool]$ScoreValid = $true,
        [string]$CommandLine = "",
        [string]$GitCommit = "",
        [string]$ProfileHash = "",
        [string]$ParallelismPath = ""
    )
    if (-not $OutputPath) { $OutputPath = Join-Path (Split-Path -Parent $TimingPath) "runtime-calibration-report.md" }
    $jsonPath = [System.IO.Path]::ChangeExtension($OutputPath, ".json")
    $timing = $null
    $parseError = ""
    if (Test-Path -LiteralPath $TimingPath) {
        try { $timing = Get-Content -Raw -Encoding UTF8 -LiteralPath $TimingPath | ConvertFrom-Json } catch { $parseError = [string]$_.Exception.Message }
    }
    $parallelism = $null
    if (-not [string]::IsNullOrWhiteSpace($ParallelismPath) -and (Test-Path -LiteralPath $ParallelismPath)) {
        try { $parallelism = Get-Content -Raw -Encoding UTF8 -LiteralPath $ParallelismPath | ConvertFrom-Json } catch { $parallelism = $null }
    }
    $decision = if ($parseError) {
        [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "timing_parse_error" }
    } else {
        Get-TaskspaceRuntimeSpeedDecision $timing $ScoreValid
    }
    $speedupEvidenceValid = Test-TaskspaceRuntimeSpeedEvidenceValid $timing $ScoreValid $parseError
    $blockers = if ($timing -and $timing.PSObject.Properties.Name -contains "runtime_optimization_blockers") { @(Get-TaskspaceRuntimeUniqueStrings $timing.runtime_optimization_blockers) } else { @() }
    $missing = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_missing_fields") { @(Get-TaskspaceRuntimeUniqueStrings $timing.wait_attribution_missing_fields) } else { @() }
    $unavailable = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_unavailable_fields") { $timing.wait_attribution_unavailable_fields } else { [pscustomobject]@{} }
    $phaseRows = @(if ($timing) { Get-TaskspaceRuntimePhaseRows $timing } else { @() })
    $jsonArtifact = [ordered]@{
        schema_version = 1
        report_path = $OutputPath
        timing_path = $TimingPath
        parallelism_path = $ParallelismPath
        command_line = $CommandLine
        git_commit = $GitCommit
        profile_hash = $ProfileHash
        score_valid = $ScoreValid
        speedup_evidence_valid = $speedupEvidenceValid
        speedup_decision = [string]$decision.decision
        speedup_decision_reason = [string]$decision.reason
        timing_parse_error = $parseError
        timing_quality = if ($timing -and $timing.PSObject.Properties.Name -contains "timing_quality") { [string]$timing.timing_quality } else { "" }
        runtime_optimization_status = if ($timing -and $timing.PSObject.Properties.Name -contains "runtime_optimization_status") { [string]$timing.runtime_optimization_status } else { "" }
        bottleneck_classification = if ($timing -and $timing.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$timing.bottleneck_classification } else { "" }
        bottleneck_counts = if ($timing -and $timing.PSObject.Properties.Name -contains "bottleneck_counts") { $timing.bottleneck_counts } else { [pscustomobject]@{} }
        repeated_docker_cache_keys = if ($timing -and $timing.PSObject.Properties.Name -contains "repeated_docker_cache_keys") { @(Get-TaskspaceRuntimeUniqueStrings $timing.repeated_docker_cache_keys) } else { @() }
        runtime_optimization_blockers = @($blockers)
        wait_attribution_missing_fields = @($missing)
        wait_attribution_unavailable_fields = $unavailable
        phase_durations = @($phaseRows)
        parallelism = $parallelism
        generated_at = (Get-Date).ToString("o")
    }
    $jsonArtifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Runtime Calibration Report")
    $lines.Add("")
    $lines.Add("- score_valid: $ScoreValid")
    $lines.Add("- speedup_evidence_valid: $speedupEvidenceValid")
    $lines.Add("- speedup_decision: $($decision.decision)")
    $lines.Add("- speedup_decision_reason: $($decision.reason)")
    $lines.Add("- timing_path: $TimingPath")
    if ($ParallelismPath) { $lines.Add("- parallelism_path: $ParallelismPath") }
    if ($GitCommit) { $lines.Add("- git_commit: $GitCommit") }
    if ($ProfileHash) { $lines.Add("- profile_hash: $ProfileHash") }
    if ($CommandLine) { $lines.Add("- command_line: $CommandLine") }
    if ($parseError) { $lines.Add("- timing_parse_error: $parseError") }
    if ($timing) {
        $lines.Add("- timing_quality: $(if ($timing.PSObject.Properties.Name -contains 'timing_quality') { $timing.timing_quality } else { '' })")
        $lines.Add("- runtime_optimization_status: $(if ($timing.PSObject.Properties.Name -contains 'runtime_optimization_status') { $timing.runtime_optimization_status } else { '' })")
        $lines.Add("- bottleneck_classification: $(if ($timing.PSObject.Properties.Name -contains 'bottleneck_classification') { $timing.bottleneck_classification } else { '' })")
        $lines.Add("- runtime_optimization_blockers: $(if ($blockers.Count -eq 0) { 'none' } else { $blockers -join ', ' })")
        $lines.Add("- wait_attribution_missing_fields: $(if ($missing.Count -eq 0) { 'none' } else { $missing -join ', ' })")
        $unavailablePairs = @($unavailable.PSObject.Properties | ForEach-Object { "$($_.Name)=$($_.Value)" })
        $lines.Add("- wait_attribution_unavailable_fields: $(if ($unavailablePairs.Count -eq 0) { 'none' } else { $unavailablePairs -join ', ' })")
        $lines.Add("")
        $lines.Add("## Phase Shares")
        foreach ($row in @($phaseRows)) {
            $lines.Add("- $($row.name): $(if ($null -eq $row.duration_ms) { 'null' } else { "$($row.duration_ms)ms" })")
        }
        if ($timing.PSObject.Properties.Name -contains "repeated_docker_cache_keys") {
            $keys = @($timing.repeated_docker_cache_keys)
            $lines.Add("")
            $lines.Add("## Cache Status")
            $lines.Add("- repeated_docker_cache_keys: $(if ($keys.Count -eq 0) { 'none' } else { $keys -join ', ' })")
        }
    }
    if ($parallelism) {
        $lines.Add("")
        $lines.Add("## Parallelism")
        $lines.Add("- resource_governor_status: $(if ($parallelism.PSObject.Properties.Name -contains 'resource_governor_status') { $parallelism.resource_governor_status } else { '' })")
        $lines.Add("- serial_only_status: $(if ($parallelism.PSObject.Properties.Name -contains 'serial_only_status') { $parallelism.serial_only_status } else { '' })")
    }
    $lines | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    $OutputPath
}
