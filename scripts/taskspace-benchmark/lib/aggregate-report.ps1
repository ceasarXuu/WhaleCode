function Add-TaskspaceCount {
    param([hashtable]$Table, [string]$Key)
    if ([string]::IsNullOrWhiteSpace($Key)) { return }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = 0 }
    $Table[$Key]++
}

function Convert-TaskspaceHashtableToObject {
    param([hashtable]$Table)
    $ordered = [ordered]@{}
    foreach ($key in @($Table.Keys | Sort-Object)) { $ordered[$key] = $Table[$key] }
    [pscustomobject]$ordered
}

function Get-TaskspaceEvidenceArray {
    param($Evidence, [Parameter(Mandatory = $true)][string]$Name)
    if ($Evidence -and $Evidence.PSObject.Properties.Name -contains $Name) { return @($Evidence.$Name) }
    @()
}

function Get-TaskspaceRowEngineeringUncleanReasons {
    param($Row)
    $reasons = New-Object System.Collections.Generic.List[string]
    if (-not $Row -or -not $Row.evidence) { return @() }
    foreach ($reason in @(Get-TaskspaceEvidenceArray $Row.evidence "engineering_unclean_reasons")) {
        if (-not [string]::IsNullOrWhiteSpace([string]$reason) -and -not $reasons.Contains([string]$reason)) { $reasons.Add([string]$reason) }
    }
    foreach ($failure in @((Get-TaskspaceEvidenceArray $Row.evidence "evidence_gate_failures") + (Get-TaskspaceEvidenceArray $Row.evidence "e3_gate_failures"))) {
        $text = [string]$failure
        if ($text -match "public_validation_timeout|docker|validator|proof|eligible|fidelity|path_unresolvable|uv_cache|source|materialization|disk_space|report") {
            if (-not $reasons.Contains($text)) { $reasons.Add($text) }
        }
    }
    foreach ($class in @(Get-TaskspaceEvidenceArray $Row.evidence "failure_taxonomy")) {
        $text = [string]$class
        if ($text -in @("engineering_unclean", "environment_noise", "validator_slow_or_flaky", "harness_materialization_failure", "validator_probe_failure", "validator_pretest_failure", "suite_circuit_breaker", "invalid_harness_run", "audit_invalid")) {
            if (-not $reasons.Contains($text)) { $reasons.Add($text) }
        }
    }
    @($reasons.ToArray())
}

function Get-TaskspaceAggregateTimingArtifact {
    param([Parameter(Mandatory = $true)][string]$ReportPath)
    $dir = Split-Path -Parent $ReportPath
    foreach ($name in @("suite-timing.json", "sample-timing.json")) {
        $candidate = Join-Path $dir $name
        if (Test-Path -LiteralPath $candidate) {
            try {
                $json = Get-Content -Raw -Encoding UTF8 -LiteralPath $candidate | ConvertFrom-Json
                return [pscustomobject]@{ path = $candidate; json = $json }
            } catch {
                return [pscustomobject]@{ path = $candidate; json = $null; parse_error = [string]$_.Exception.Message }
            }
        }
    }
    [pscustomobject]@{ path = ""; json = $null }
}

function Get-TaskspaceRuntimeBottleneckArtifact {
    param([Parameter(Mandatory = $true)][string]$ReportPath)
    $dir = Split-Path -Parent $ReportPath
    $candidate = Join-Path $dir "runtime-bottleneck.json"
    if (Test-Path -LiteralPath $candidate) {
        try {
            $json = Get-Content -Raw -Encoding UTF8 -LiteralPath $candidate | ConvertFrom-Json
            return [pscustomobject]@{ path = $candidate; json = $json }
        } catch {
            return [pscustomobject]@{ path = $candidate; json = $null; parse_error = [string]$_.Exception.Message }
        }
    }
    [pscustomobject]@{ path = ""; json = $null }
}

function Write-TaskspaceAggregateJsonArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Aggregate,
        [Parameter(Mandatory = $true)]$PairIndex,
        [Parameter(Mandatory = $true)]$FailureSummary,
        [Parameter(Mandatory = $true)]$GraphSummary
    )
    $dir = Split-Path -Parent $Path
    $aggregateJson = Join-Path $dir "aggregate.json"
    $pairIndexJson = Join-Path $dir "pair-index.json"
    $failureJson = Join-Path $dir "failure-taxonomy-summary.json"
    $graphJson = Join-Path $dir "graph-health-summary.json"
    $Aggregate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $aggregateJson -Encoding UTF8
    @($PairIndex) | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $pairIndexJson -Encoding UTF8
    $FailureSummary | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $failureJson -Encoding UTF8
    $GraphSummary | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $graphJson -Encoding UTF8
}

function Write-TaskspaceAggregateReport {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Reports
    )
    $all = @($Reports)
    $validUtility = @($all | Where-Object { $_.evidence.included_in_utility_aggregate })
    $validE3 = @($all | Where-Object { $_.evidence.included_in_e3_aggregate })
    $included = @($all | Where-Object { $_.evidence.included_in_utility_aggregate -or $_.evidence.included_in_e3_aggregate })
    $e3Rows = @($all | Where-Object { [string]$_.evidence.reported_evidence_level -like "E3*" })
    $partialRows = @($all | Where-Object { [string]$_.evidence.reported_evidence_level -like "*candidate" })
    $environmentRows = @($all | Where-Object {
            (@($_.evidence.evidence_gate_failures) + @($_.evidence.e3_gate_failures)) -match "environment|remote_asset|docker_|public_validation_timeout"
        })
    $invalidHarnessRows = @($all | Where-Object {
            (@($_.evidence.failure_taxonomy) + @($_.evidence.evidence_gate_failures) + @($_.evidence.e3_gate_failures)) -match "invalid_harness|harness_materialization|validator_probe|validator_pretest|suite_circuit_breaker|path_unresolvable|uv_cache|validator_source|no_tests_started"
        })
    $engineeringUncleanRows = @($all | Where-Object {
            ($_.evidence.PSObject.Properties.Name -contains "engineering_unclean" -and [bool]$_.evidence.engineering_unclean) -or
            (@(Get-TaskspaceRowEngineeringUncleanReasons $_).Count -gt 0)
        })
    $auditRequiredRows = @($all | Where-Object {
            ($_.evidence.PSObject.Properties.Name -contains "audit_required" -and [bool]$_.evidence.audit_required) -or
            (@($_.evidence.e3_gate_failures) -contains "e3_human_review_not_completed") -or
            (@($_.evidence.evidence_gate_failures) -contains "audit_review_missing")
        })
    $auditReadyRows = @($e3Rows | Where-Object { $_.evidence.human_review_completed -and @($_.evidence.e3_gate_failures).Count -eq 0 })
    $reviewCompleted = @($e3Rows | Where-Object { $_.evidence.human_review_completed })
    $validE3Reviewed = @($validE3 | Where-Object { $_.evidence.human_review_completed })
    $reviewDisagreements = @($e3Rows | Where-Object { $_.evidence.human_review_disagreement })
    $decisionCounts = @{}
    $failureCounts = @{}
    $engineeringUncleanCounts = @{}
    $directionCounts = @{}
    $excludedByReason = @{}
    $pairIndex = New-Object System.Collections.Generic.List[object]
    foreach ($row in $all) {
        $decision = [string]$row.evidence.human_review_decision
        if ($row.evidence.human_review_completed) { Add-TaskspaceCount $decisionCounts ($(if ($decision) { $decision } else { "missing" })) }
        foreach ($class in @($row.evidence.failure_taxonomy)) { Add-TaskspaceCount $failureCounts ([string]$class) }
        foreach ($reason in @(Get-TaskspaceRowEngineeringUncleanReasons $row)) { Add-TaskspaceCount $engineeringUncleanCounts ([string]$reason) }
        Add-TaskspaceCount $directionCounts ([string]$row.evidence.utility_direction)
        $gateFailures = @(@($row.evidence.evidence_gate_failures) + @($row.evidence.e3_gate_failures))
        if (-not ($row.evidence.included_in_utility_aggregate -or $row.evidence.included_in_e3_aggregate)) {
            if ($gateFailures.Count -eq 0) { Add-TaskspaceCount $excludedByReason "not_included_by_gate" }
            else { foreach ($failure in $gateFailures) { Add-TaskspaceCount $excludedByReason ([string]$failure) } }
        }
        $pairIndex.Add([pscustomobject]@{
                repeat = $row.repeat
                pair_dir = $row.pair_dir
                pair_report = $row.pair_report
                audit_manifest = if ($row.evidence.PSObject.Properties.Name -contains "audit_manifest_path") { [string]$row.evidence.audit_manifest_path } else { "" }
                reported_evidence_level = [string]$row.evidence.reported_evidence_level
                included_in_utility_aggregate = [bool]$row.evidence.included_in_utility_aggregate
                included_in_e3_aggregate = [bool]$row.evidence.included_in_e3_aggregate
                utility_direction = if ($row.evidence.PSObject.Properties.Name -contains "utility_direction") { [string]$row.evidence.utility_direction } else { "" }
                failure_taxonomy = if ($row.evidence.PSObject.Properties.Name -contains "failure_taxonomy") { @($row.evidence.failure_taxonomy) } else { @() }
                run_score_ready = if ($row.evidence.PSObject.Properties.Name -contains "run_score_ready") { [bool]$row.evidence.run_score_ready } else { (@(Get-TaskspaceRowEngineeringUncleanReasons $row).Count -eq 0 -and -not ([bool]($row.evidence.PSObject.Properties.Name -contains "audit_required" -and [bool]$row.evidence.audit_required))) }
                run_score_valid = if ($row.evidence.PSObject.Properties.Name -contains "run_score_valid") { [bool]$row.evidence.run_score_valid } else { (@(Get-TaskspaceRowEngineeringUncleanReasons $row).Count -eq 0) }
                audit_required = if ($row.evidence.PSObject.Properties.Name -contains "audit_required") { [bool]$row.evidence.audit_required } else { $false }
                engineering_unclean = if ($row.evidence.PSObject.Properties.Name -contains "engineering_unclean") { [bool]$row.evidence.engineering_unclean } else { (@(Get-TaskspaceRowEngineeringUncleanReasons $row).Count -gt 0) }
                engineering_unclean_reasons = @(Get-TaskspaceRowEngineeringUncleanReasons $row)
                outcome_standard = if ($row.evidence.PSObject.Properties.Name -contains "outcome_standard") { [string]$row.evidence.outcome_standard } else { "" }
                outcome_taskspace = if ($row.evidence.PSObject.Properties.Name -contains "outcome_taskspace") { [string]$row.evidence.outcome_taskspace } else { "" }
                evidence_gate_failures = @($row.evidence.evidence_gate_failures)
                e3_gate_failures = @($row.evidence.e3_gate_failures)
            })
    }
    $graphWarningCounts = @{}
    $graphHealthPaths = @()
    foreach ($row in $all) {
        if ([string]::IsNullOrWhiteSpace([string]$row.pair_dir)) { continue }
        foreach ($side in @("left", "right")) {
            $candidate = Join-Path $row.pair_dir "$side\artifacts\graph-health.json"
            if (Test-Path -LiteralPath $candidate) {
                $graphHealthPaths += $candidate
                try {
                    $graph = Get-Content -Raw -Encoding UTF8 -LiteralPath $candidate | ConvertFrom-Json
                    foreach ($warning in @($graph.warnings)) { Add-TaskspaceCount $graphWarningCounts ([string]$warning) }
                } catch {
                    Add-TaskspaceCount $graphWarningCounts "graph_health_parse_error"
                }
            }
        }
    }
    $scoreReady = ($engineeringUncleanRows.Count -eq 0 -and $auditRequiredRows.Count -eq 0)
    $scoreValid = $scoreReady
    $scoreBlockReason = if ($engineeringUncleanRows.Count -gt 0) { "engineering_unclean" } elseif ($auditRequiredRows.Count -gt 0) { "audit_required" } else { "" }
    $diagnosticComparisonEnabled = ($invalidHarnessRows.Count -eq 0 -and $scoreReady)
    $agentExecTimeoutCount = @($all | Where-Object {
            (@($_.evidence.outcome_standard, $_.evidence.outcome_taskspace) -contains "agent_exec_timeout") -and
            -not ($_.evidence.PSObject.Properties.Name -contains "engineering_unclean" -and [bool]$_.evidence.engineering_unclean)
        }).Count
    $cleanComparablePairCount = @($all | Where-Object { @(Get-TaskspaceRowEngineeringUncleanReasons $_).Count -eq 0 -and -not ($_.evidence.PSObject.Properties.Name -contains "audit_required" -and [bool]$_.evidence.audit_required) }).Count
    $timingArtifact = Get-TaskspaceAggregateTimingArtifact $Path
    $runtimeBottleneckArtifact = Get-TaskspaceRuntimeBottleneckArtifact $Path
    $timingSummary = [ordered]@{
        timing_path = [string]$timingArtifact.path
        runtime_bottleneck_path = [string]$runtimeBottleneckArtifact.path
        bottleneck_classification = if ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$timingArtifact.json.bottleneck_classification } else { "" }
        speedup_decision = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "speedup_decision") { [string]$runtimeBottleneckArtifact.json.speedup_decision } else { "" }
        speedup_decision_reason = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "speedup_decision_reason") { [string]$runtimeBottleneckArtifact.json.speedup_decision_reason } else { "" }
        runtime_optimization_status = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "runtime_optimization_status") { [string]$runtimeBottleneckArtifact.json.runtime_optimization_status } elseif ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "runtime_optimization_status") { [string]$timingArtifact.json.runtime_optimization_status } else { "" }
        wait_attribution_status = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "wait_attribution_status") { [string]$runtimeBottleneckArtifact.json.wait_attribution_status } elseif ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "wait_attribution_status") { [string]$timingArtifact.json.wait_attribution_status } else { "" }
        wait_attribution_missing_fields = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "wait_attribution_missing_fields") { @($runtimeBottleneckArtifact.json.wait_attribution_missing_fields) } elseif ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "wait_attribution_missing_fields") { @($timingArtifact.json.wait_attribution_missing_fields) } else { @() }
        wait_attribution_unavailable_fields = if ($runtimeBottleneckArtifact.json -and $runtimeBottleneckArtifact.json.PSObject.Properties.Name -contains "wait_attribution_unavailable_fields") { $runtimeBottleneckArtifact.json.wait_attribution_unavailable_fields } elseif ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "wait_attribution_unavailable_fields") { $timingArtifact.json.wait_attribution_unavailable_fields } else { [pscustomobject]@{} }
        top_spans = if ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "timing_breakdown") { @($timingArtifact.json.timing_breakdown.top_spans) } else { @() }
        phase_distributions = if ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "phase_distributions") { $timingArtifact.json.phase_distributions } else { $null }
        repeated_docker_cache_keys = if ($timingArtifact.json -and $timingArtifact.json.PSObject.Properties.Name -contains "repeated_docker_cache_keys") { @($timingArtifact.json.repeated_docker_cache_keys) } else { @() }
    }
    $aggregate = [ordered]@{
        aggregate_version = "taskspace-0.0.4-phase1-aggregate-v1"
        run_validity = if ($engineeringUncleanRows.Count -gt 0) { "invalid_harness" } else { "valid" }
        score_ready = $scoreReady
        score_valid = $scoreValid
        score_block_reason = $scoreBlockReason
        score_invalid_reason = if ($scoreBlockReason -eq "engineering_unclean") { "engineering_unclean" } else { "" }
        score_fields_enabled = $scoreReady
        engineering_unclean_count = $engineeringUncleanRows.Count
        engineering_unclean_reasons = Convert-TaskspaceHashtableToObject $engineeringUncleanCounts
        audit_required_count = $auditRequiredRows.Count
        agent_exec_timeout_count = $agentExecTimeoutCount
        clean_comparable_pair_count = $cleanComparablePairCount
        score_bearing_outcomes = @("solved", "wrong", "agent_exec_timeout")
        diagnostic_comparison_enabled = $diagnosticComparisonEnabled
        invalid_run_reason = if ($engineeringUncleanRows.Count -gt 0) { "engineering_unclean" } else { "" }
        abort_scope = if ($engineeringUncleanRows.Count -gt 0) { "sample" } else { "none" }
        abort_phase = if ($engineeringUncleanRows.Count -gt 0) { "report_gate" } else { "" }
        abort_signature = ""
        first_failure_artifact = if ($engineeringUncleanRows.Count -gt 0) { [string]$engineeringUncleanRows[0].pair_report } elseif ($invalidHarnessRows.Count -gt 0) { [string]$invalidHarnessRows[0].pair_report } else { "" }
        configured_pairs = $all.Count
        eligible_pairs = $all.Count - $environmentRows.Count
        environment_failed_pairs = $environmentRows.Count
        partial_pairs = $partialRows.Count
        e3_candidate_pairs = $e3Rows.Count
        audit_ready_pairs = $auditReadyRows.Count
        e3_included_pairs = $validE3.Count
        all_pairs = $all.Count
        valid_utility_pairs = $validUtility.Count
        valid_e3_pairs = $validE3.Count
        excluded_pairs = $all.Count - $included.Count
        taskspace_better = if ($scoreValid -and $diagnosticComparisonEnabled -and $directionCounts.ContainsKey("taskspace_better")) { $directionCounts["taskspace_better"] } else { $null }
        standard_better = if ($scoreValid -and $diagnosticComparisonEnabled -and $directionCounts.ContainsKey("standard_better")) { $directionCounts["standard_better"] } else { $null }
        pass_rate_delta = if ($scoreValid) { 0 } else { $null }
        diagnostic_pass_rate_delta = if ($scoreValid -and $diagnosticComparisonEnabled) { 0 } else { $null }
        both_success = if ($directionCounts.ContainsKey("both_success")) { $directionCounts["both_success"] } else { 0 }
        both_failed = if ($directionCounts.ContainsKey("both_failed")) { $directionCounts["both_failed"] } else { 0 }
        inconclusive = if ($directionCounts.ContainsKey("inconclusive")) { $directionCounts["inconclusive"] } else { 0 }
        excluded_by_reason = Convert-TaskspaceHashtableToObject $excludedByReason
        failure_taxonomy_summary = Convert-TaskspaceHashtableToObject $failureCounts
        graph_health_summary = [ordered]@{
            graph_health_files = @($graphHealthPaths).Count
            warnings = Convert-TaskspaceHashtableToObject $graphWarningCounts
        }
        timing_summary = $timingSummary
    }
    $failureSummary = [ordered]@{
        failure_taxonomy_summary = $aggregate.failure_taxonomy_summary
        excluded_by_reason = $aggregate.excluded_by_reason
    }
    $graphSummary = $aggregate.graph_health_summary
    Write-TaskspaceAggregateJsonArtifacts $Path $aggregate @($pairIndex.ToArray()) $failureSummary $graphSummary
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Benchmark Aggregate Report")
    $lines.Add("")
    $lines.Add("- run_validity: $($aggregate["run_validity"])")
    $lines.Add("- score_ready: $($aggregate["score_ready"])")
    $lines.Add("- score_valid: $($aggregate["score_valid"])")
    $lines.Add("- score_block_reason: $($aggregate["score_block_reason"])")
    $lines.Add("- score_fields_enabled: $($aggregate["score_fields_enabled"])")
    $lines.Add("- diagnostic_comparison_enabled: $($aggregate["diagnostic_comparison_enabled"])")
    if (-not $scoreReady) {
        $lines.Add("- score_invalid_reason: $($aggregate["score_invalid_reason"])")
        $lines.Add("- engineering_unclean_count: $($aggregate["engineering_unclean_count"])")
        $lines.Add("- audit_required_count: $($aggregate["audit_required_count"])")
        $lines.Add("- agent_exec_timeout_count: $($aggregate["agent_exec_timeout_count"])")
        $lines.Add("- clean_comparable_pair_count: $($aggregate["clean_comparable_pair_count"])")
        $lines.Add("- invalid_run_reason: $($aggregate["invalid_run_reason"])")
        $lines.Add("- first_failure_artifact: $($aggregate["first_failure_artifact"])")
        $lines.Add("- diagnostic_note: $(if ($scoreBlockReason -eq 'audit_required') { 'score fields disabled because E3 human review is pending' } else { 'score fields disabled because engineering clean execution is not established' })")
    }
    $summaryKeys = @("configured_pairs", "eligible_pairs", "environment_failed_pairs", "partial_pairs", "e3_candidate_pairs", "audit_ready_pairs", "e3_included_pairs", "all_pairs", "valid_utility_pairs", "valid_e3_pairs", "excluded_pairs")
    if ($diagnosticComparisonEnabled) {
        $summaryKeys += @("taskspace_better", "standard_better", "both_success", "both_failed", "inconclusive")
    } else {
        $summaryKeys += @("both_success", "both_failed", "inconclusive")
    }
    foreach ($key in $summaryKeys) {
        $lines.Add("- ${key}: $($aggregate[$key])")
    }
    if ($e3Rows.Count -gt 0 -and $reviewCompleted.Count -gt 0) {
        $decisionSummary = @($decisionCounts.Keys | Sort-Object | ForEach-Object { "$_=$($decisionCounts[$_])" }) -join "; "
        $lines.Add("- e3_human_review_completed_pairs: $($reviewCompleted.Count)")
        $lines.Add("- e3_human_review_disagreement_pairs: $($reviewDisagreements.Count)")
        $lines.Add("- e3_human_review_decisions: $decisionSummary")
        $lines.Add("- e3_taskspace_better_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_taskspace_better' }).Count)")
        $lines.Add("- e3_standard_better_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_standard_better' }).Count)")
        $lines.Add("- e3_no_clear_delta_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_no_clear_delta' }).Count)")
        $lines.Add("- e3_taskspace_benefit_note: only include_taskspace_better counts as directional TaskSpace benefit evidence")
    }
    $lines.Add("")
    $lines.Add("## Failure Taxonomy Summary")
    if ($failureCounts.Count -eq 0) { $lines.Add("- none") } else { foreach ($key in @($failureCounts.Keys | Sort-Object)) { $lines.Add("- ${key}: $($failureCounts[$key])") } }
    $lines.Add("")
    $lines.Add("## Graph Health Summary")
    $lines.Add("- graph_health_files: $(@($graphHealthPaths).Count)")
    if ($graphWarningCounts.Count -eq 0) { $lines.Add("- warnings: none") } else { foreach ($key in @($graphWarningCounts.Keys | Sort-Object)) { $lines.Add("- ${key}: $($graphWarningCounts[$key])") } }
    $lines.Add("")
    $lines.Add("## Timing Summary")
    $lines.Add("- timing_path: $($timingSummary["timing_path"])")
    $lines.Add("- runtime_bottleneck_path: $($timingSummary["runtime_bottleneck_path"])")
    $lines.Add("- bottleneck_classification: $($timingSummary["bottleneck_classification"])")
    $lines.Add("- speedup_decision: $($timingSummary["speedup_decision"])")
    $lines.Add("- speedup_decision_reason: $($timingSummary["speedup_decision_reason"])")
    $lines.Add("- runtime_optimization_status: $($timingSummary["runtime_optimization_status"])")
    $lines.Add("- wait_attribution_status: $($timingSummary["wait_attribution_status"])")
    $lines.Add("- wait_attribution_missing_fields: $(if (@($timingSummary["wait_attribution_missing_fields"]).Count -eq 0) { 'none' } else { @($timingSummary["wait_attribution_missing_fields"]) -join ', ' })")
    $unavailableWaitPairs = @($timingSummary["wait_attribution_unavailable_fields"].PSObject.Properties | ForEach-Object { "$($_.Name)=$($_.Value)" })
    $lines.Add("- wait_attribution_unavailable_fields: $(if ($unavailableWaitPairs.Count -eq 0) { 'none' } else { $unavailableWaitPairs -join ', ' })")
    if (@($timingSummary["top_spans"]).Count -eq 0) {
        $lines.Add("- top_spans: none")
    } else {
        foreach ($span in @($timingSummary["top_spans"])) { $lines.Add("- top_span: $($span.name)=$($span.duration_ms)ms") }
    }
    if ($timingSummary["phase_distributions"]) {
        foreach ($phase in @($timingSummary["phase_distributions"].PSObject.Properties.Name | Sort-Object)) {
            $dist = $timingSummary["phase_distributions"].$phase
            $lines.Add("- ${phase}_median_ms: $($dist.median_ms)")
            $lines.Add("- ${phase}_p95_ms: $($dist.p95_ms)")
        }
    }
    if (@($timingSummary["repeated_docker_cache_keys"]).Count -gt 0) {
        $lines.Add("- repeated_docker_cache_keys: $(@($timingSummary["repeated_docker_cache_keys"]) -join ', ')")
    } else {
        $lines.Add("- repeated_docker_cache_keys: none")
    }
    foreach ($report in $all) {
        $lines.Add("")
        $lines.Add("## Pair $($report.repeat)")
        $lines.Add("- pair_report: $($report.pair_report)")
        $lines.Add("- audit_manifest: $(if ($report.evidence.PSObject.Properties.Name -contains 'audit_manifest_path') { $report.evidence.audit_manifest_path } else { '' })")
        $lines.Add("- reported_evidence_level: $($report.evidence.reported_evidence_level)")
        $lines.Add("- included_in_utility_aggregate: $($report.evidence.included_in_utility_aggregate)")
        if ([string]$report.evidence.reported_evidence_level -like "E3*") {
            $lines.Add("- included_in_e3_aggregate: $($report.evidence.included_in_e3_aggregate)")
            $lines.Add("- human_review_completed: $($report.evidence.human_review_completed)")
            $lines.Add("- human_review_decision: $($report.evidence.human_review_decision)")
            $lines.Add("- human_review_disagreement: $($report.evidence.human_review_disagreement)")
        }
        $lines.Add("- utility_direction: $(if ($report.evidence.PSObject.Properties.Name -contains 'utility_direction') { $report.evidence.utility_direction } else { '' })")
        $lines.Add("- failure_taxonomy: $(if ($report.evidence.PSObject.Properties.Name -contains 'failure_taxonomy' -and @($report.evidence.failure_taxonomy).Count -gt 0) { @($report.evidence.failure_taxonomy) -join ', ' } else { 'none' })")
        $lines.Add("- evidence_gate_failures: $(if (@($report.evidence.evidence_gate_failures).Count -eq 0) { 'none' } else { @($report.evidence.evidence_gate_failures) -join ', ' })")
        if ([string]$report.evidence.reported_evidence_level -like "E3*") {
            $lines.Add("- e3_gate_failures: $(if (@($report.evidence.e3_gate_failures).Count -eq 0) { 'none' } else { @($report.evidence.e3_gate_failures) -join ', ' })")
        }
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}
