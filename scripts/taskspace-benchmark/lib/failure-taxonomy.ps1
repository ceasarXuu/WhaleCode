function Test-TaskspaceMetricSuccess {
    param($Metrics)
    ($Metrics -and $Metrics.PSObject.Properties.Name -contains "business_success" -and [bool]$Metrics.business_success)
}

function Get-TaskspaceMetricArray {
    param($Metrics, [Parameter(Mandatory = $true)][string]$Name)
    if ($Metrics -and $Metrics.PSObject.Properties.Name -contains $Name) { return @($Metrics.$Name) }
    @()
}

function Add-TaskspaceFailureClass {
    param(
        [Parameter(Mandatory = $true)]$Classes,
        [Parameter(Mandatory = $true)][string]$Class
    )
    if (-not [string]::IsNullOrWhiteSpace($Class) -and -not $Classes.Contains($Class)) {
        $Classes.Add($Class)
    }
}

function Get-TaskspaceMetricBool {
    param($Metrics, [Parameter(Mandatory = $true)][string]$Name)
    ($Metrics -and $Metrics.PSObject.Properties.Name -contains $Name -and [bool]$Metrics.$Name)
}

function Test-TaskspaceCleanAgentTimeoutValidationSkip {
    param($Metrics)
    if (-not $Metrics) { return $false }
    if (-not (Get-TaskspaceMetricBool $Metrics "exec_timed_out")) { return $false }
    if (-not (Get-TaskspaceMetricBool $Metrics "public_validation_skipped")) { return $false }
    $skipReason = if ($Metrics.PSObject.Properties.Name -contains "public_validation_skip_reason") { [string]$Metrics.public_validation_skip_reason } else { "" }
    $probeStatus = if ($Metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_status") { [string]$Metrics.pre_agent_validator_probe_status } else { "" }
    $probeHash = if ($Metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_hash") { [string]$Metrics.pre_agent_validator_probe_hash } else { "" }
    ($skipReason -eq "agent_exec_timeout" -and $probeStatus -eq "passed" -and $probeHash -match '^[0-9a-f]{64}$')
}

function Test-TaskspaceExternalValidationCompleted {
    param($Metrics)
    if (-not $Metrics) { return $false }
    if (-not (Get-TaskspaceMetricBool $Metrics "tests_started_seen")) { return $false }
    if (-not (Get-TaskspaceMetricBool $Metrics "tests_completed_seen")) { return $false }
    if ($Metrics.PSObject.Properties.Name -contains "validation_lifecycle_stage" -and [string]$Metrics.validation_lifecycle_stage -ne "tests_completed") { return $false }
    $true
}

function Test-TaskspaceAuditPending {
    param(
        $Evidence = $null,
        $AuditReview = $null,
        $ManifestResolved = $null
    )
    $reviewRequired = $false
    if ($ManifestResolved -and $ManifestResolved.PSObject.Properties.Name -contains "human_review_required" -and [bool]$ManifestResolved.human_review_required) {
        $reviewRequired = $true
    }
    if ($Evidence) {
        $gateFailures = @(@($Evidence.evidence_gate_failures) + @($Evidence.e3_gate_failures))
        if (@($gateFailures | Where-Object { [string]$_ -eq "e3_human_review_not_completed" -or [string]$_ -eq "audit_review_missing" }).Count -gt 0) {
            $reviewRequired = $true
        }
    }
    if (-not $reviewRequired) { return $false }
    if ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "completed" -and [bool]$AuditReview.completed) { return $false }
    $true
}

function Test-TaskspaceAuditReviewInvalid {
    param($AuditReview = $null)
    if (-not $AuditReview -or -not ($AuditReview.PSObject.Properties.Name -contains "failures")) { return $false }
    $failures = @(@($AuditReview.failures) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($failures.Count -eq 0) { return $false }
    @($failures | Where-Object { [string]$_ -notin @("audit_review_missing", "e3_human_review_not_completed") }).Count -gt 0
}

function Get-TaskspaceEngineeringUncleanReasons {
    param(
        $Metrics,
        $Evidence = $null,
        $AuditReview = $null,
        $VariableControl = $null
    )
    $reasons = New-Object System.Collections.Generic.List[string]
    if ($Metrics) {
        if (Get-TaskspaceMetricBool $Metrics "taskspace_profile_hard_stop_seen") {
            Add-TaskspaceFailureClass $reasons "taskspace_profile_hard_stop"
        }
        if (Get-TaskspaceMetricBool $Metrics "public_validation_skipped") {
            $skipReason = if ($Metrics.PSObject.Properties.Name -contains "public_validation_skip_reason") { [string]$Metrics.public_validation_skip_reason } else { "" }
            $probeStatus = if ($Metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_status") { [string]$Metrics.pre_agent_validator_probe_status } else { "" }
            $probeHash = if ($Metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_hash") { [string]$Metrics.pre_agent_validator_probe_hash } else { "" }
            if ($skipReason -ne "agent_exec_timeout" -or $probeStatus -ne "passed" -or $probeHash -notmatch '^[0-9a-f]{64}$') {
                Add-TaskspaceFailureClass $reasons "pre_agent_validator_probe_missing_or_failed"
            }
        }
        if ($Metrics.PSObject.Properties.Name -contains "public_validation_exit_code" -and [int]$Metrics.public_validation_exit_code -eq 124) {
            Add-TaskspaceFailureClass $reasons "public_validation_timeout"
        }
        foreach ($failure in @(Get-TaskspaceMetricArray $Metrics "validator_environment_failures")) {
            $text = [string]$failure
            if ([string]::IsNullOrWhiteSpace($text)) { continue }
            if ($text -eq "docker_run_failure" -and (Get-TaskspaceMetricBool $Metrics "tests_started_seen") -and (Get-TaskspaceMetricBool $Metrics "tests_completed_seen")) { continue }
            Add-TaskspaceFailureClass $reasons $text
        }
        foreach ($taint in @(Get-TaskspaceMetricArray $Metrics "metrics_taints")) {
            $text = [string]$taint
            if ([string]::IsNullOrWhiteSpace($text)) { continue }
            Add-TaskspaceFailureClass $reasons $text
        }
        if ($Metrics.PSObject.Properties.Name -contains "active_sentinel_warning_count" -and [int]$Metrics.active_sentinel_warning_count -gt 0) {
            $cleanAgentTimeoutSkip = Test-TaskspaceCleanAgentTimeoutValidationSkip $Metrics
            $externalValidationCompleted = Test-TaskspaceExternalValidationCompleted $Metrics
            foreach ($sentinelType in @(Get-TaskspaceMetricArray $Metrics "active_sentinel_warning_types")) {
                $text = [string]$sentinelType
                if ([string]::IsNullOrWhiteSpace($text)) { $text = "unknown" }
                if (($cleanAgentTimeoutSkip -or $externalValidationCompleted) -and $text -eq "validator_failure") { continue }
                Add-TaskspaceFailureClass $reasons "active_sentinel_warning:$text"
            }
            if (-not $cleanAgentTimeoutSkip -and -not $externalValidationCompleted -and @(Get-TaskspaceMetricArray $Metrics "active_sentinel_warning_types").Count -eq 0) {
                Add-TaskspaceFailureClass $reasons "active_sentinel_warning"
            }
        }
        if (Get-TaskspaceMetricBool $Metrics "pretest_failure") {
            $signature = if ($Metrics.PSObject.Properties.Name -contains "infra_signature" -and $Metrics.infra_signature) { [string]$Metrics.infra_signature.stable_code } else { "validator_pretest_failure" }
            Add-TaskspaceFailureClass $reasons $signature
        }
        if ($Metrics.PSObject.Properties.Name -contains "validation_lifecycle_stage" -and [string]$Metrics.validation_lifecycle_stage -eq "unknown" -and -not (Get-TaskspaceMetricBool $Metrics "tests_started_seen") -and -not (Get-TaskspaceMetricBool $Metrics "exec_timed_out")) {
            Add-TaskspaceFailureClass $reasons "no_tests_started_marker"
        }
    }
    if ($Evidence) {
        foreach ($failure in @(@($Evidence.evidence_gate_failures) + @($Evidence.e3_gate_failures))) {
            $text = [string]$failure
            if ($text -in @("audit_review_missing", "e3_human_review_not_completed")) { continue }
            if ($text -match "public_validation_timeout|docker|validator|proof|eligible|fidelity|audit_review_invalid|audit_hash_mismatch|audit_review_malformed|path_unresolvable|uv_cache|source|materialization|disk_space|report") {
                Add-TaskspaceFailureClass $reasons $text
            }
        }
    }
    if ($VariableControl -and $VariableControl.PSObject.Properties.Name -contains "invalid_pair" -and [bool]$VariableControl.invalid_pair) {
        Add-TaskspaceFailureClass $reasons "audit_unclean"
    }
    if (Test-TaskspaceAuditReviewInvalid $AuditReview) {
        Add-TaskspaceFailureClass $reasons "e3_audit_review_invalid"
    }
    @($reasons.ToArray())
}

function Get-TaskspaceAgentOutcome {
    param(
        $Metrics,
        [string[]]$EngineeringUncleanReasons = @()
    )
    if (@($EngineeringUncleanReasons).Count -gt 0) { return "engineering_unclean" }
    if (Get-TaskspaceMetricBool $Metrics "exec_timed_out") { return "agent_exec_timeout" }
    if (Get-TaskspaceMetricBool $Metrics "sampling_interrupted") { return "runtime_interrupted" }
    if ($Metrics -and $Metrics.PSObject.Properties.Name -contains "agent_completion_status" -and [string]$Metrics.agent_completion_status -ne "complete") { return "agent_incomplete" }
    if (Test-TaskspaceMetricSuccess $Metrics) { return "solved" }
    "wrong"
}

function Test-TaskspaceEngineeringUnclean {
    param([string[]]$Reasons = @())
    @($Reasons | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0
}

function Get-TaskspaceFailureTaxonomy {
    param(
        $StandardMetrics,
        $TaskspaceMetrics,
        $Evidence = $null,
        $AuditReview = $null,
        $VariableControl = $null
    )
    $classes = New-Object System.Collections.Generic.List[string]
    $standardSuccess = Test-TaskspaceMetricSuccess $StandardMetrics
    $taskspaceSuccess = Test-TaskspaceMetricSuccess $TaskspaceMetrics
    $standardValidationTimeout = ($StandardMetrics -and [int]$StandardMetrics.public_validation_exit_code -eq 124)
    $taskspaceValidationTimeout = ($TaskspaceMetrics -and [int]$TaskspaceMetrics.public_validation_exit_code -eq 124)
    $standardExecTimeout = ($StandardMetrics -and $StandardMetrics.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$StandardMetrics.exec_timed_out)
    $taskspaceExecTimeout = ($TaskspaceMetrics -and $TaskspaceMetrics.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$TaskspaceMetrics.exec_timed_out)
    $standardEnvironmentFailures = @(Get-TaskspaceMetricArray $StandardMetrics "validator_environment_failures")
    $taskspaceEnvironmentFailures = @(Get-TaskspaceMetricArray $TaskspaceMetrics "validator_environment_failures")
    $hardReasons = @(@(
        @(Get-TaskspaceEngineeringUncleanReasons $StandardMetrics $Evidence $AuditReview $VariableControl) +
        @(Get-TaskspaceEngineeringUncleanReasons $TaskspaceMetrics $Evidence $AuditReview $VariableControl)
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    if ($hardReasons.Count -gt 0) {
        Add-TaskspaceFailureClass $classes "engineering_unclean"
    }
    $standardAgentFailureClassifiable = -not ($standardValidationTimeout -or $standardExecTimeout -or $standardEnvironmentFailures.Count -gt 0)
    $taskspaceAgentFailureClassifiable = -not ($taskspaceValidationTimeout -or $taskspaceExecTimeout -or $taskspaceEnvironmentFailures.Count -gt 0)
    $standardChanged = @(Get-TaskspaceMetricArray $StandardMetrics "changed_paths")
    $taskspaceChanged = @(Get-TaskspaceMetricArray $TaskspaceMetrics "changed_paths")
    $environmentFailures = @(
        @($standardEnvironmentFailures) +
        @($taskspaceEnvironmentFailures)
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique
    foreach ($failure in $environmentFailures) {
        if ([string]$failure -match "validator_probe_failed") { Add-TaskspaceFailureClass $classes "validator_probe_failure" }
        elseif ([string]$failure -match "path_unresolvable|relative_materialized_path|uv_cache_missing|validator_source_missing|runtime_manifest_missing") { Add-TaskspaceFailureClass $classes "harness_materialization_failure" }
        elseif ([string]$failure -match "no_tests_started_marker") { Add-TaskspaceFailureClass $classes "validator_pretest_failure" }
        elseif ([string]$failure -match "suite_circuit_breaker|suite_repeated_infra_signature") { Add-TaskspaceFailureClass $classes "suite_circuit_breaker" }
        elseif ([string]$failure -match "invalid_harness") { Add-TaskspaceFailureClass $classes "invalid_harness_run" }
        elseif ([string]$failure -match "remote_asset") { Add-TaskspaceFailureClass $classes "remote_asset_equivalence_unproven" }
        elseif ([string]$failure -match "docker") { Add-TaskspaceFailureClass $classes "environment_noise" }
        elseif ([string]$failure -match "public_validation_timeout") { Add-TaskspaceFailureClass $classes "validator_slow_or_flaky" }
        else { Add-TaskspaceFailureClass $classes "environment_noise" }
    }
    if ($standardValidationTimeout -and $taskspaceValidationTimeout) {
        Add-TaskspaceFailureClass $classes "validator_slow_or_flaky"
    }
    if (($taskspaceExecTimeout -or $taskspaceValidationTimeout) -and -not ($standardExecTimeout -or $standardValidationTimeout) -and $standardSuccess) {
        Add-TaskspaceFailureClass $classes "taskspace_overhead_timeout"
    }
    if ($taskspaceAgentFailureClassifiable -and -not $taskspaceSuccess -and $taskspaceChanged.Count -eq 0) {
        Add-TaskspaceFailureClass $classes "agent_no_patch"
    }
    if ($standardAgentFailureClassifiable -and -not $standardSuccess -and $standardChanged.Count -eq 0) {
        Add-TaskspaceFailureClass $classes "agent_no_patch"
    }
    if ($taskspaceAgentFailureClassifiable -and -not $taskspaceSuccess -and $taskspaceChanged.Count -gt 0) {
        Add-TaskspaceFailureClass $classes "agent_patch_wrong"
    }
    if ($standardAgentFailureClassifiable -and -not $standardSuccess -and $standardChanged.Count -gt 0) {
        Add-TaskspaceFailureClass $classes "agent_patch_wrong"
    }
    $graphWarnings = @()
    if ($TaskspaceMetrics -and $TaskspaceMetrics.PSObject.Properties.Name -contains "graph_health_warnings") {
        $graphWarnings = @($TaskspaceMetrics.graph_health_warnings)
    }
    if ($graphWarnings -contains "node_inflation_high") {
        Add-TaskspaceFailureClass $classes "node_overfragmentation"
    }
    if ($graphWarnings -contains "subagent_no_adoption" -or $graphWarnings -contains "subagent_no_decision_yield") {
        Add-TaskspaceFailureClass $classes "subagent_noise_or_unused"
    }
    if ($graphWarnings -contains "synthesis_not_ready") {
        Add-TaskspaceFailureClass $classes "result_not_synthesized"
    }
    if ($VariableControl -and $VariableControl.PSObject.Properties.Name -contains "invalid_pair" -and [bool]$VariableControl.invalid_pair) {
        Add-TaskspaceFailureClass $classes "audit_unclean"
    }
    if (Test-TaskspaceAuditPending $Evidence $AuditReview) {
        Add-TaskspaceFailureClass $classes "audit_unclean"
    }
    if (Test-TaskspaceAuditReviewInvalid $AuditReview) {
        Add-TaskspaceFailureClass $classes "audit_invalid"
    }
    if ($Evidence) {
        $gateFailures = @(@($Evidence.evidence_gate_failures) + @($Evidence.e3_gate_failures))
        if (@($gateFailures | Where-Object { [string]$_ -match "audit|unclean" }).Count -gt 0) {
            Add-TaskspaceFailureClass $classes "audit_unclean"
        }
        if (@($gateFailures | Where-Object { [string]$_ -match "remote_asset" }).Count -gt 0) {
            Add-TaskspaceFailureClass $classes "remote_asset_equivalence_unproven"
        }
        if (@($gateFailures | Where-Object { [string]$_ -match "invalid_harness|validator_probe|pretest|path_unresolvable|uv_cache|validator_source" }).Count -gt 0) {
            Add-TaskspaceFailureClass $classes "invalid_harness_run"
        }
        if (@($gateFailures | Where-Object { [string]$_ -match "environment|docker" }).Count -gt 0) {
            Add-TaskspaceFailureClass $classes "environment_noise"
        }
    }
    if ($classes.Count -eq 0 -and -not ($standardSuccess -and $taskspaceSuccess)) {
        Add-TaskspaceFailureClass $classes "unknown"
    }
    @($classes.ToArray())
}

function Get-TaskspaceUtilityDirection {
    param($StandardMetrics, $TaskspaceMetrics, [string[]]$FailureClasses = @())
    if (@($FailureClasses | Where-Object { $_ -in @("engineering_unclean", "environment_noise", "validator_slow_or_flaky", "remote_asset_unavailable", "remote_asset_equivalence_unproven", "audit_unclean", "harness_materialization_failure", "validator_probe_failure", "validator_pretest_failure", "suite_circuit_breaker", "invalid_harness_run") }).Count -gt 0) {
        return "score_disabled"
    }
    $standardSuccess = Test-TaskspaceMetricSuccess $StandardMetrics
    $taskspaceSuccess = Test-TaskspaceMetricSuccess $TaskspaceMetrics
    if ($taskspaceSuccess -and -not $standardSuccess) { return "taskspace_better" }
    if ($standardSuccess -and -not $taskspaceSuccess) { return "standard_better" }
    if ($standardSuccess -and $taskspaceSuccess) { return "both_success" }
    "both_failed"
}
