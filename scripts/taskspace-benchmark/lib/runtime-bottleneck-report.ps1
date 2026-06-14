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
    $class = if ($Timing.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$Timing.bottleneck_classification } else { "unknown" }
    switch ($class) {
        "agent_bound" { return [pscustomobject]@{ decision = "speedup_limited_agent_bound"; reason = "agent_execution_dominates_clean_wall_time" } }
        "validator_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "validation_or_oracle_dominates_clean_wall_time" } }
        "docker_build_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "docker_build_dominates_clean_wall_time" } }
        "docker_run_bound" { return [pscustomobject]@{ decision = "speedup_candidate_validator_or_docker"; reason = "docker_run_dominates_clean_wall_time" } }
        "queue_bound" { return [pscustomobject]@{ decision = "speedup_blocked_instrumentation"; reason = "queue_wait_requires_resource_governor_calibration" } }
        "engineering_unclean_slow" { return [pscustomobject]@{ decision = "speedup_blocked_invalid_run"; reason = "engineering_unclean_slow" } }
        default { return [pscustomobject]@{ decision = "speedup_candidate_parallelism"; reason = "timing_complete_without_single_dominant_phase" } }
    }
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
    $blockersForJson = if ($timing -and $timing.PSObject.Properties.Name -contains "runtime_optimization_blockers") { @(Get-TaskspaceRuntimeUniqueStrings $timing.runtime_optimization_blockers) } else { @() }
    $missingForJson = if ($timing -and $timing.PSObject.Properties.Name -contains "wait_attribution_missing_fields") { @(Get-TaskspaceRuntimeUniqueStrings $timing.wait_attribution_missing_fields) } else { @() }
    $jsonArtifact = [ordered]@{
        schema_version = 1
        timing_path = $TimingPath
        report_path = $OutputPath
        score_valid = $ScoreValid
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
