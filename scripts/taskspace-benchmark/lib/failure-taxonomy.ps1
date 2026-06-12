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
    if ($AuditReview -and $AuditReview.PSObject.Properties.Name -contains "completed" -and -not [bool]$AuditReview.completed) {
        Add-TaskspaceFailureClass $classes "audit_unclean"
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
    if (@($FailureClasses | Where-Object { $_ -in @("environment_noise", "validator_slow_or_flaky", "remote_asset_unavailable", "remote_asset_equivalence_unproven", "audit_unclean", "harness_materialization_failure", "validator_probe_failure", "validator_pretest_failure", "suite_circuit_breaker", "invalid_harness_run") }).Count -gt 0) {
        return "inconclusive"
    }
    $standardSuccess = Test-TaskspaceMetricSuccess $StandardMetrics
    $taskspaceSuccess = Test-TaskspaceMetricSuccess $TaskspaceMetrics
    if ($taskspaceSuccess -and -not $standardSuccess) { return "taskspace_better" }
    if ($standardSuccess -and -not $taskspaceSuccess) { return "standard_better" }
    if ($standardSuccess -and $taskspaceSuccess) { return "both_success" }
    "both_failed"
}
