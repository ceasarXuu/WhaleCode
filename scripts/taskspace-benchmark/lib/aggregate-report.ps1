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
    $auditReadyRows = @($e3Rows | Where-Object { $_.evidence.human_review_completed -and @($_.evidence.e3_gate_failures).Count -eq 0 })
    $reviewCompleted = @($e3Rows | Where-Object { $_.evidence.human_review_completed })
    $validE3Reviewed = @($validE3 | Where-Object { $_.evidence.human_review_completed })
    $reviewDisagreements = @($e3Rows | Where-Object { $_.evidence.human_review_disagreement })
    $decisionCounts = @{}
    $failureCounts = @{}
    $directionCounts = @{}
    $excludedByReason = @{}
    $pairIndex = New-Object System.Collections.Generic.List[object]
    foreach ($row in $all) {
        $decision = [string]$row.evidence.human_review_decision
        if ($row.evidence.human_review_completed) { Add-TaskspaceCount $decisionCounts ($(if ($decision) { $decision } else { "missing" })) }
        foreach ($class in @($row.evidence.failure_taxonomy)) { Add-TaskspaceCount $failureCounts ([string]$class) }
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
    $diagnosticComparisonEnabled = ($invalidHarnessRows.Count -eq 0)
    $aggregate = [ordered]@{
        aggregate_version = "taskspace-0.0.4-phase1-aggregate-v1"
        run_validity = if ($diagnosticComparisonEnabled) { "valid" } else { "invalid_harness" }
        diagnostic_comparison_enabled = $diagnosticComparisonEnabled
        invalid_run_reason = if ($diagnosticComparisonEnabled) { "" } else { "invalid_harness_failure_detected" }
        abort_scope = if ($diagnosticComparisonEnabled) { "none" } else { "sample" }
        abort_phase = if ($diagnosticComparisonEnabled) { "" } else { "report_gate" }
        abort_signature = ""
        first_failure_artifact = if ($invalidHarnessRows.Count -gt 0) { [string]$invalidHarnessRows[0].pair_report } else { "" }
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
        taskspace_better = if ($diagnosticComparisonEnabled -and $directionCounts.ContainsKey("taskspace_better")) { $directionCounts["taskspace_better"] } else { 0 }
        standard_better = if ($diagnosticComparisonEnabled -and $directionCounts.ContainsKey("standard_better")) { $directionCounts["standard_better"] } else { 0 }
        both_success = if ($directionCounts.ContainsKey("both_success")) { $directionCounts["both_success"] } else { 0 }
        both_failed = if ($directionCounts.ContainsKey("both_failed")) { $directionCounts["both_failed"] } else { 0 }
        inconclusive = if ($directionCounts.ContainsKey("inconclusive")) { $directionCounts["inconclusive"] } else { 0 }
        excluded_by_reason = Convert-TaskspaceHashtableToObject $excludedByReason
        failure_taxonomy_summary = Convert-TaskspaceHashtableToObject $failureCounts
        graph_health_summary = [ordered]@{
            graph_health_files = @($graphHealthPaths).Count
            warnings = Convert-TaskspaceHashtableToObject $graphWarningCounts
        }
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
    $lines.Add("- diagnostic_comparison_enabled: $($aggregate["diagnostic_comparison_enabled"])")
    if (-not $diagnosticComparisonEnabled) {
        $lines.Add("- invalid_run_reason: $($aggregate["invalid_run_reason"])")
        $lines.Add("- first_failure_artifact: $($aggregate["first_failure_artifact"])")
        $lines.Add("- diagnostic_note: comparison disabled because harness validity is not established")
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
    if ($e3Rows.Count -gt 0 -and $diagnosticComparisonEnabled) {
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
