function Get-TaskspaceEvidenceGate {
    param(
        [int]$Repeats,
        [Parameter(Mandatory = $true)]$PromptGuard,
        [Parameter(Mandatory = $true)][string]$OracleIsolationLevel,
        [string]$ProviderParamStatus = "provider-default-or-unknown",
        [bool]$InvalidPair = $false,
        [bool]$BusinessSuccess = $true,
        [bool]$AcceptedSoftIsolation = $false
    )
    $failures = New-Object System.Collections.Generic.List[string]
    if ($InvalidPair) { $failures.Add("invalid_pair") }
    if (-not $BusinessSuccess) { $failures.Add("business_success_false") }
    if ($Repeats -lt 3) { $failures.Add("repeats_lt_3") }
    if ($PromptGuard.invalid_prompt) { $failures.Add("invalid_prompt") }
    if ($PromptGuard.manual_review_required) { $failures.Add("manual_review_required") }
    if ($ProviderParamStatus -match "unknown") { $failures.Add("provider_params_unknown") }
    if ($OracleIsolationLevel -eq "soft_denylist") { $failures.Add("oracle_isolation_soft_denylist") }
    if ($OracleIsolationLevel -eq "failed") { $failures.Add("oracle_isolation_failed") }
    if ($AcceptedSoftIsolation) { $failures.Add("accepted_soft_isolation_non_e2") }
    $level = if ($failures.Count -eq 0) { "E2" } elseif (-not $PromptGuard.invalid_prompt -and -not $InvalidPair -and $BusinessSuccess -and $OracleIsolationLevel -ne "failed") { "E2-candidate" } else { "E1" }
    [pscustomobject]@{
        reported_evidence_level = $level
        evidence_gate_failures = @($failures.ToArray())
        included_in_utility_aggregate = ($level -eq "E2")
    }
}

function Compare-TaskspacePairVariables {
    param(
        [Parameter(Mandatory = $true)]$ManifestResolved,
        [Parameter(Mandatory = $true)]$LeftMetrics,
        [Parameter(Mandatory = $true)]$RightMetrics
    )
    $failures = New-Object System.Collections.Generic.List[string]
    if ($ManifestResolved.prompt_sha256_left -ne $ManifestResolved.prompt_sha256_right) { $failures.Add("prompt_checksum_mismatch") }
    if ($ManifestResolved.fixture_sha256_left -ne $ManifestResolved.fixture_sha256_right) { $failures.Add("fixture_checksum_mismatch") }
    if ($ManifestResolved.whale_sha256_left -ne $ManifestResolved.whale_sha256_right) { $failures.Add("whale_sha256_mismatch") }
    if ($ManifestResolved.model_left -ne $ManifestResolved.model_right) { $failures.Add("model_mismatch") }
    if ($ManifestResolved.timeout_seconds_left -ne $ManifestResolved.timeout_seconds_right) { $failures.Add("timeout_mismatch") }
    if ($LeftMetrics.hidden_oracle_exit_code -ne $RightMetrics.hidden_oracle_exit_code -and ($LeftMetrics.hidden_oracle_exit_code -eq 0 -or $RightMetrics.hidden_oracle_exit_code -eq 0)) {
        $failures.Add("oracle_result_mismatch")
    }
    [pscustomobject]@{
        invalid_pair = $failures.Count -gt 0
        failures = @($failures.ToArray())
    }
}

function Write-TaskspaceJson {
    param([Parameter(Mandatory = $true)]$Value, [Parameter(Mandatory = $true)][string]$Path)
    ($Value | ConvertTo-Json -Depth 30) | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Write-TaskspacePairReport {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)]$PromptGuard,
        [Parameter(Mandatory = $true)]$VariableControl,
        [Parameter(Mandatory = $true)]$EvidenceGate,
        [Parameter(Mandatory = $true)]$LeftMetrics,
        [Parameter(Mandatory = $true)]$RightMetrics,
        [Parameter(Mandatory = $true)]$Pair
    )
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Benchmark Pair Report")
    $lines.Add("")
    foreach ($row in @(
        @("scenario", $Manifest.Id),
        @("level", $Manifest.Level),
        @("requested_evidence_target", $Manifest.EvidenceTarget),
        @("reported_evidence_level", $EvidenceGate.reported_evidence_level),
        @("valid_pair", (-not $VariableControl.invalid_pair)),
        @("included_in_utility_aggregate", $EvidenceGate.included_in_utility_aggregate),
        @("left_logical_mode", $Pair.Left.LogicalMode),
        @("right_logical_mode", $Pair.Right.LogicalMode)
    )) { $lines.Add("- $($row[0]): $($row[1])") }
    $lines.Add("")
    $lines.Add("## Evidence Gate Failures")
    if (@($EvidenceGate.evidence_gate_failures).Count -eq 0) { $lines.Add("- none") } else { foreach ($failure in $EvidenceGate.evidence_gate_failures) { $lines.Add("- $failure") } }
    $lines.Add("")
    $lines.Add("## Variable Control")
    if (@($VariableControl.failures).Count -eq 0) { $lines.Add("- failures: none") } else { foreach ($failure in $VariableControl.failures) { $lines.Add("- $failure") } }
    $lines.Add("")
    $lines.Add("## Prompt Guard")
    $lines.Add("- invalid_prompt: $($PromptGuard.invalid_prompt)")
    $lines.Add("- manual_review_required: $($PromptGuard.manual_review_required)")
    $lines.Add("- hard_hits: $(@($PromptGuard.hard_hits) -join ', ')")
    $lines.Add("- context_hits: $(@($PromptGuard.context_hits) -join ', ')")
    $taskspaceMetrics = @($LeftMetrics, $RightMetrics) | Where-Object { $_.logical_mode -eq "taskspace" } | Select-Object -First 1
    if ($taskspaceMetrics) {
        $lines.Add("")
        $lines.Add("## Scenario Warnings")
        $warnings = New-Object System.Collections.Generic.List[string]
        if ($Manifest.Expected.max_taskspace_nodes -and $taskspaceMetrics.nodes -gt [int]$Manifest.Expected.max_taskspace_nodes) {
            $warnings.Add("taskspace_node_count_exceeds_expected: $($taskspaceMetrics.nodes) > $($Manifest.Expected.max_taskspace_nodes)")
        }
        if ($Manifest.Expected.max_taskspace_spawn_agent_calls -ne $null -and $taskspaceMetrics.spawn_agent_calls -gt [int]$Manifest.Expected.max_taskspace_spawn_agent_calls) {
            $warnings.Add("taskspace_spawn_agent_calls_exceeds_expected: $($taskspaceMetrics.spawn_agent_calls) > $($Manifest.Expected.max_taskspace_spawn_agent_calls)")
        }
        if ($warnings.Count -eq 0) { $lines.Add("- none") } else { foreach ($warning in $warnings) { $lines.Add("- $warning") } }
    }
    foreach ($sideMetrics in @($LeftMetrics, $RightMetrics)) {
        $lines.Add("")
        $lines.Add("## $($sideMetrics.mode) / $($sideMetrics.logical_mode)")
        foreach ($row in @(
            @("business_success", $sideMetrics.business_success),
            @("exec_exit_code", $sideMetrics.exec_exit_code),
            @("public_validation_exit_code", $sideMetrics.public_validation_exit_code),
            @("hidden_oracle_exit_code", $sideMetrics.hidden_oracle_exit_code),
            @("oracle_isolation_level", $sideMetrics.oracle_isolation_level),
            @("wall_time_ms", $sideMetrics.wall_time_ms),
            @("tool_call_count", $sideMetrics.tool_call_count),
            @("changed_paths", (@($sideMetrics.changed_paths) -join ", ")),
            @("maps", $sideMetrics.maps),
            @("nodes", $sideMetrics.nodes),
            @("edges", $sideMetrics.edges),
            @("edge_order_violations", $sideMetrics.edge_order_violations),
            @("spawn_agent_calls", $sideMetrics.spawn_agent_calls),
            @("subagent_results", $sideMetrics.subagent_results),
            @("open_leaf_nodes", $sideMetrics.open_leaf_nodes),
            @("ordinary_before_binding", $sideMetrics.ordinary_before_binding)
        )) { $lines.Add("- $($row[0]): $($row[1])") }
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Write-TaskspaceRunSummary {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Reports
    )
    $lines = @("# TaskSpace Benchmark Run Summary", "")
    foreach ($report in @($Reports)) {
        $lines += "- pair: $($report.pair_dir)"
        $lines += "  - reported_evidence_level: $($report.evidence.reported_evidence_level)"
        $lines += "  - included_in_utility_aggregate: $($report.evidence.included_in_utility_aggregate)"
        $lines += "  - pair_report: $($report.pair_report)"
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}
