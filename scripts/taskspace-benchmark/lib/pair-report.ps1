function Get-TaskspaceEvidenceGate {
    param(
        [int]$Repeats,
        [Parameter(Mandatory = $true)]$PromptGuard,
        [Parameter(Mandatory = $true)][string]$OracleIsolationLevel,
        $ProviderParamStatus = "provider-default-or-unknown",
        [bool]$InvalidPair = $false,
        [bool]$BusinessSuccess = $true,
        [bool]$AcceptedSoftIsolation = $false,
        [bool]$AggregateEligible = $true,
        [string]$OracleIsolationPolicy = "deferred_materialization_allowed",
        [string]$EvidenceTarget = "E2",
        $SampleOrigin = $null,
        $ExternalBenchmark = $null,
        $E3Config = $null,
        [bool]$HumanReviewRequired = $false,
        [bool]$HumanReviewCompleted = $false,
        [int]$E3MinimumRepeats = 5,
        [string]$HumanReviewDecision = "",
        [bool]$HumanReviewDisagreement = $false
    )
    $failures = New-Object System.Collections.Generic.List[string]
    $e3Failures = New-Object System.Collections.Generic.List[string]
    $effectiveE3MinimumRepeats = [Math]::Max(5, $E3MinimumRepeats)
    if ($InvalidPair) { $failures.Add("invalid_pair") }
    if (-not $BusinessSuccess) { $failures.Add("business_success_false") }
    if ($Repeats -lt 3) { $failures.Add("repeats_lt_3") }
    if ($PromptGuard.invalid_prompt) { $failures.Add("invalid_prompt") }
    if ($PromptGuard.manual_review_required) { $failures.Add("manual_review_required") }
    $providerComplete = $false
    if ($ProviderParamStatus -is [string]) {
        $providerComplete = $ProviderParamStatus -notmatch "unknown"
    } else {
        $providerComplete = [bool]$ProviderParamStatus.complete
    }
    if (-not $providerComplete) { $failures.Add("provider_params_incomplete") }
    if ($OracleIsolationLevel -eq "soft_denylist") { $failures.Add("oracle_isolation_soft_denylist") }
    if ($OracleIsolationLevel -eq "failed") { $failures.Add("oracle_isolation_failed") }
    if ($OracleIsolationLevel -eq "hard_deferred_materialization" -and $OracleIsolationPolicy -ne "deferred_materialization_allowed") {
        $failures.Add("oracle_isolation_deferred_not_allowed")
    }
    if ($AcceptedSoftIsolation) { $failures.Add("accepted_soft_isolation_non_e2") }
    if (-not $AggregateEligible) { $failures.Add("aggregate_not_enabled") }
    $target = ([string]$EvidenceTarget).ToUpperInvariant()
    if ($target -eq "E3") {
        if ($Repeats -lt $effectiveE3MinimumRepeats) { $e3Failures.Add("e3_repeats_lt_$effectiveE3MinimumRepeats") }
        $originType = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "type") { [string]$SampleOrigin.type } else { "" }
        if ($originType -notin @("historical_whale_failure", "external_benchmark")) {
            $e3Failures.Add("e3_sample_origin_missing_or_invalid")
        }
        $claimScope = if ($null -ne $E3Config -and $E3Config.PSObject.Properties.Name -contains "claim_scope") { [string]$E3Config.claim_scope } else { "" }
        if ([string]::IsNullOrWhiteSpace($claimScope)) { $e3Failures.Add("e3_claim_scope_missing") }
        $originSource = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "source") { [string]$SampleOrigin.source } else { "" }
        if ([string]::IsNullOrWhiteSpace($originSource)) { $e3Failures.Add("e3_sample_source_missing") }
        $promptSha = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "original_prompt_sha256") { [string]$SampleOrigin.original_prompt_sha256 } else { "" }
        if ([string]::IsNullOrWhiteSpace($promptSha)) { $e3Failures.Add("e3_original_prompt_sha_missing") }
        if ($originType -eq "historical_whale_failure") {
            $sanitized = ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "sanitized" -and [bool]$SampleOrigin.sanitized)
            $privacyReviewed = ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "privacy_review_completed" -and [bool]$SampleOrigin.privacy_review_completed)
            $sanitizationSummary = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "sanitization_summary") { [string]$SampleOrigin.sanitization_summary } else { "" }
            $privacyRiskSummary = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "privacy_risk_summary") { [string]$SampleOrigin.privacy_risk_summary } else { "" }
            if (-not $sanitized) { $e3Failures.Add("e3_historical_sample_not_sanitized") }
            if (-not $privacyReviewed) { $e3Failures.Add("e3_privacy_review_not_completed") }
            if ([string]::IsNullOrWhiteSpace($sanitizationSummary)) { $e3Failures.Add("e3_sanitization_summary_missing") }
            if ([string]::IsNullOrWhiteSpace($privacyRiskSummary)) { $e3Failures.Add("e3_privacy_risk_summary_missing") }
        }
        if ($originType -eq "external_benchmark") {
            $sampleId = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "sample_id") { [string]$SampleOrigin.sample_id } else { "" }
            $sourceVersion = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "source_version") { [string]$SampleOrigin.source_version } else { "" }
            $sourceUrl = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "source_url") { [string]$SampleOrigin.source_url } else { "" }
            $license = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "license") { [string]$SampleOrigin.license } else { "" }
            $dataPolicy = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "data_policy") { [string]$SampleOrigin.data_policy } else { "" }
            $validatorSha = if ($null -ne $SampleOrigin -and $SampleOrigin.PSObject.Properties.Name -contains "original_validator_sha256") { [string]$SampleOrigin.original_validator_sha256 } else { "" }
            $benchmarkName = if ($null -ne $ExternalBenchmark -and $ExternalBenchmark.PSObject.Properties.Name -contains "name") { [string]$ExternalBenchmark.name } else { "" }
            $adapterVersion = if ($null -ne $ExternalBenchmark -and $ExternalBenchmark.PSObject.Properties.Name -contains "adapter_version") { [string]$ExternalBenchmark.adapter_version } else { "" }
            if ([string]::IsNullOrWhiteSpace($sampleId)) { $e3Failures.Add("e3_external_sample_id_missing") }
            if ([string]::IsNullOrWhiteSpace($sourceVersion)) { $e3Failures.Add("e3_external_source_version_missing") }
            if ([string]::IsNullOrWhiteSpace($sourceUrl)) { $e3Failures.Add("e3_external_source_url_missing") }
            if ([string]::IsNullOrWhiteSpace($license)) { $e3Failures.Add("e3_external_license_missing") }
            if ([string]::IsNullOrWhiteSpace($dataPolicy)) { $e3Failures.Add("e3_external_data_policy_missing") }
            if ([string]::IsNullOrWhiteSpace($validatorSha)) { $e3Failures.Add("e3_external_validator_sha_missing") }
            if ([string]::IsNullOrWhiteSpace($benchmarkName)) { $e3Failures.Add("e3_external_benchmark_name_missing") }
            if ([string]::IsNullOrWhiteSpace($adapterVersion)) { $e3Failures.Add("e3_external_adapter_version_missing") }
        }
        if (-not $HumanReviewRequired) { $e3Failures.Add("e3_human_review_not_required") }
        if (-not $HumanReviewCompleted) { $e3Failures.Add("e3_human_review_not_completed") }
        $includeReviewDecisions = @(
            "include_taskspace_better",
            "include_standard_better",
            "include_no_clear_delta"
        )
        $excludeReviewDecisions = @(
            "exclude_harness_failure",
            "exclude_invalid_prompt",
            "exclude_validator_unclear",
            "exclude_privacy_or_sample_risk"
        )
        $validReviewDecisions = @($includeReviewDecisions + $excludeReviewDecisions)
        if ($HumanReviewCompleted -and $validReviewDecisions -notcontains $HumanReviewDecision) {
            $e3Failures.Add("e3_human_review_decision_missing_or_invalid")
        }
        if ($HumanReviewCompleted -and $excludeReviewDecisions -contains $HumanReviewDecision) {
            $e3Failures.Add("e3_human_review_excluded_pair")
        }
    }
    $level = if ($target -eq "E3" -and $failures.Count -eq 0 -and $e3Failures.Count -eq 0) {
        "E3"
    } elseif ($target -eq "E3" -and $failures.Count -eq 0) {
        "E3-candidate"
    } elseif ($failures.Count -eq 0) {
        "E2"
    } elseif (-not $PromptGuard.invalid_prompt -and -not $InvalidPair -and $BusinessSuccess -and $OracleIsolationLevel -ne "failed") {
        "E2-candidate"
    } else {
        "E1"
    }
    [pscustomobject]@{
        reported_evidence_level = $level
        evidence_gate_failures = @($failures.ToArray())
        e3_gate_failures = @($e3Failures.ToArray())
        included_in_utility_aggregate = ($level -eq "E2")
        included_in_e3_aggregate = ($level -eq "E3")
        e3_minimum_repeats = $effectiveE3MinimumRepeats
        human_review_completed = $HumanReviewCompleted
        human_review_decision = $HumanReviewDecision
        human_review_disagreement = $HumanReviewDisagreement
        oracle_isolation_policy = $OracleIsolationPolicy
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
        [Parameter(Mandatory = $true)]$Pair,
        $IsolationProbe = $null
    )
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Benchmark Pair Report")
    $lines.Add("")
    foreach ($row in @(
        @("scenario", $Manifest.Id),
        @("level", $Manifest.Level),
        @("requested_evidence_target", $Manifest.EvidenceTarget),
        @("reported_evidence_level", $EvidenceGate.reported_evidence_level),
        @("oracle_isolation_policy", $EvidenceGate.oracle_isolation_policy),
        @("valid_pair", (-not $VariableControl.invalid_pair)),
        @("included_in_utility_aggregate", $EvidenceGate.included_in_utility_aggregate),
        @("left_logical_mode", $Pair.Left.LogicalMode),
        @("right_logical_mode", $Pair.Right.LogicalMode)
    )) { $lines.Add("- $($row[0]): $($row[1])") }
    if ([string]$Manifest.EvidenceTarget -eq "E3") {
        $lines.Add("- included_in_e3_aggregate: $($EvidenceGate.included_in_e3_aggregate)")
    }
    $lines.Add("")
    $lines.Add("## Evidence Gate Failures")
    if (@($EvidenceGate.evidence_gate_failures).Count -eq 0) { $lines.Add("- none") } else { foreach ($failure in $EvidenceGate.evidence_gate_failures) { $lines.Add("- $failure") } }
    $lines.Add("")
    if ([string]$Manifest.EvidenceTarget -eq "E3" -or @($EvidenceGate.e3_gate_failures).Count -gt 0) {
        $originType = if ($null -ne $Manifest.SampleOrigin -and $Manifest.SampleOrigin.PSObject.Properties.Name -contains "type") { [string]$Manifest.SampleOrigin.type } else { "" }
        $claimScope = if ($null -ne $Manifest.E3 -and $Manifest.E3.PSObject.Properties.Name -contains "claim_scope") { [string]$Manifest.E3.claim_scope } else { "" }
        $lines.Add("## E3 Gate")
        $lines.Add("- sample_origin_type: $originType")
        $lines.Add("- human_review_required: $($Manifest.HumanReviewRequired)")
        $lines.Add("- human_review_completed: $($EvidenceGate.human_review_completed)")
        $lines.Add("- human_review_decision: $($EvidenceGate.human_review_decision)")
        $lines.Add("- human_review_disagreement: $($EvidenceGate.human_review_disagreement)")
        $lines.Add("- e3_minimum_repeats: $($EvidenceGate.e3_minimum_repeats)")
        $lines.Add("- claim_scope: $claimScope")
        if (@($EvidenceGate.e3_gate_failures).Count -eq 0) { $lines.Add("- failures: none") } else { foreach ($failure in $EvidenceGate.e3_gate_failures) { $lines.Add("- $failure") } }
        if ($EvidenceGate.PSObject.Properties.Name -contains "audit_review_source_path") {
            $lines.Add("- audit_review_source_path: $($EvidenceGate.audit_review_source_path)")
            $auditFailures = @($EvidenceGate.audit_review_failures)
            if ($auditFailures.Count -eq 0) { $lines.Add("- audit_review_failures: none") } else { $lines.Add("- audit_review_failures: $($auditFailures -join ', ')") }
        }
        $lines.Add("")
    }
    $lines.Add("## Variable Control")
    if (@($VariableControl.failures).Count -eq 0) { $lines.Add("- failures: none") } else { foreach ($failure in $VariableControl.failures) { $lines.Add("- $failure") } }
    $lines.Add("")
    $lines.Add("## Prompt Guard")
    $lines.Add("- invalid_prompt: $($PromptGuard.invalid_prompt)")
    $lines.Add("- manual_review_required: $($PromptGuard.manual_review_required)")
    $lines.Add("- hard_hits: $(@($PromptGuard.hard_hits) -join ', ')")
    $lines.Add("- context_hits: $(@($PromptGuard.context_hits) -join ', ')")
    if ($IsolationProbe) {
        $lines.Add("")
        $lines.Add("## Oracle Isolation Probe")
        $lines.Add("- oracle_isolation_level: $($IsolationProbe.oracle_isolation_level)")
        $lines.Add("- canary_leaked: $($IsolationProbe.canary_leaked)")
        $lines.Add("- canary_materialized_during_probe: $($IsolationProbe.canary_materialized_during_probe)")
        $lines.Add("- path_mentioned: $($IsolationProbe.path_mentioned)")
        $lines.Add("- jsonl: $($IsolationProbe.jsonl_path)")
    }
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
    $standardMetrics = @($LeftMetrics, $RightMetrics) | Where-Object { $_.logical_mode -eq "standard" } | Select-Object -First 1
    if ($standardMetrics -and $taskspaceMetrics) {
        $toolRatio = if ([int]$standardMetrics.tool_call_count -gt 0) {
            [math]::Round(([double]$taskspaceMetrics.tool_call_count / [double]$standardMetrics.tool_call_count), 2)
        } else { 0 }
        $timeRatio = if ([int64]$standardMetrics.wall_time_ms -gt 0) {
            [math]::Round(([double]$taskspaceMetrics.wall_time_ms / [double]$standardMetrics.wall_time_ms), 2)
        } else { 0 }
        $toolWarn = $Manifest.Thresholds.taskspace_tool_call_ratio_warn -and $toolRatio -gt [double]$Manifest.Thresholds.taskspace_tool_call_ratio_warn
        $timeWarn = $Manifest.Thresholds.taskspace_wall_time_ratio_warn -and $timeRatio -gt [double]$Manifest.Thresholds.taskspace_wall_time_ratio_warn
        $outcome = if ($taskspaceMetrics.business_success -and -not $standardMetrics.business_success) {
            "taskspace_better"
        } elseif ($standardMetrics.business_success -and -not $taskspaceMetrics.business_success) {
            "taskspace_worse"
        } elseif ($standardMetrics.business_success -and $taskspaceMetrics.business_success -and -not ($toolWarn -or $timeWarn)) {
            "both_success_cost_within_budget"
        } elseif ($standardMetrics.business_success -and $taskspaceMetrics.business_success) {
            "both_success_taskspace_cost_higher"
        } else {
            "both_failed_or_inconclusive"
        }
        $lines.Add("")
        $lines.Add("## Utility Assessment")
        $lines.Add("- outcome: $outcome")
        $lines.Add("- taskspace_tool_call_ratio: $toolRatio")
        $lines.Add("- taskspace_wall_time_ratio: $timeRatio")
        $lines.Add("- taskspace_tool_call_ratio_warn: $toolWarn")
        $lines.Add("- taskspace_wall_time_ratio_warn: $timeWarn")
        $lines.Add("- note: evidence level proves paired comparability; utility outcome is reported separately to avoid overstating benefit.")
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
        if ([string]$report.evidence.reported_evidence_level -like "E3*") {
            $lines += "  - included_in_e3_aggregate: $($report.evidence.included_in_e3_aggregate)"
        }
        $lines += "  - pair_report: $($report.pair_report)"
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Write-TaskspaceAggregateReport {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Reports
    )
    $all = @($Reports)
    $valid = @($all | Where-Object { $_.evidence.included_in_utility_aggregate })
    $validE3 = @($all | Where-Object { $_.evidence.included_in_e3_aggregate })
    $included = @($all | Where-Object { $_.evidence.included_in_utility_aggregate -or $_.evidence.included_in_e3_aggregate })
    $hasE3Rows = @($all | Where-Object { [string]$_.evidence.reported_evidence_level -like "E3*" }).Count -gt 0
    $e3Rows = @($all | Where-Object { [string]$_.evidence.reported_evidence_level -like "E3*" })
    $reviewCompleted = @($e3Rows | Where-Object { $_.evidence.human_review_completed })
    $reviewDisagreements = @($e3Rows | Where-Object { $_.evidence.human_review_disagreement })
    $decisionCounts = @{}
    foreach ($row in $reviewCompleted) {
        $decision = [string]$row.evidence.human_review_decision
        if ([string]::IsNullOrWhiteSpace($decision)) { $decision = "missing" }
        if (-not $decisionCounts.ContainsKey($decision)) { $decisionCounts[$decision] = 0 }
        $decisionCounts[$decision]++
    }
    $taskspaceBetterPairs = @($reviewCompleted | Where-Object { [string]$_.evidence.human_review_decision -eq "include_taskspace_better" }).Count
    $standardBetterPairs = @($reviewCompleted | Where-Object { [string]$_.evidence.human_review_decision -eq "include_standard_better" }).Count
    $noClearDeltaPairs = @($reviewCompleted | Where-Object { [string]$_.evidence.human_review_decision -eq "include_no_clear_delta" }).Count
    $lines = @(
        "# TaskSpace Benchmark Aggregate Report",
        "",
        "- all_pairs: $($all.Count)",
        "- valid_utility_pairs: $($valid.Count)"
    )
    if ($hasE3Rows) {
        $decisionSummary = if ($decisionCounts.Count -eq 0) {
            ""
        } else {
            @($decisionCounts.Keys | Sort-Object | ForEach-Object { "$_=$($decisionCounts[$_])" }) -join "; "
        }
        $lines += "- valid_e3_pairs: $($validE3.Count)"
        $lines += "- e3_human_review_completed_pairs: $($reviewCompleted.Count)"
        $lines += "- e3_human_review_disagreement_pairs: $($reviewDisagreements.Count)"
        $lines += "- e3_human_review_decisions: $decisionSummary"
        $lines += "- e3_taskspace_better_pairs: $taskspaceBetterPairs"
        $lines += "- e3_standard_better_pairs: $standardBetterPairs"
        $lines += "- e3_no_clear_delta_pairs: $noClearDeltaPairs"
        $lines += "- e3_taskspace_benefit_note: only include_taskspace_better counts as directional TaskSpace benefit evidence"
    }
    $lines += "- excluded_pairs: $($all.Count - $included.Count)"
    foreach ($report in $all) {
        $lines += ""
        $lines += "## Pair $($report.repeat)"
        $lines += "- pair_report: $($report.pair_report)"
        $lines += "- reported_evidence_level: $($report.evidence.reported_evidence_level)"
        $lines += "- included_in_utility_aggregate: $($report.evidence.included_in_utility_aggregate)"
        if ([string]$report.evidence.reported_evidence_level -like "E3*") {
            $lines += "- included_in_e3_aggregate: $($report.evidence.included_in_e3_aggregate)"
            $lines += "- human_review_completed: $($report.evidence.human_review_completed)"
            $lines += "- human_review_decision: $($report.evidence.human_review_decision)"
            $lines += "- human_review_disagreement: $($report.evidence.human_review_disagreement)"
        }
        if (@($report.evidence.evidence_gate_failures).Count -eq 0) {
            $lines += "- evidence_gate_failures: none"
        } else {
            $lines += "- evidence_gate_failures: $(@($report.evidence.evidence_gate_failures) -join ', ')"
        }
        if ([string]$report.evidence.reported_evidence_level -like "E3*") {
            if (@($report.evidence.e3_gate_failures).Count -eq 0) {
                $lines += "- e3_gate_failures: none"
            } else {
                $lines += "- e3_gate_failures: $(@($report.evidence.e3_gate_failures) -join ', ')"
            }
        }
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}
