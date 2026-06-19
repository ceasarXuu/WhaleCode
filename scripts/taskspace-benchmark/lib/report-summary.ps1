function Test-TaskspaceReportedEvidenceLevelIsFormalE3 {
    param($Evidence)
    if (-not $Evidence) { return $false }
    [string]$Evidence.reported_evidence_level -eq "E3"
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
        $reportTarget = if ($report.PSObject.Properties.Name -contains "evidence_target") { [string]$report.evidence_target } else { "" }
        if ($reportTarget -eq "E3" -or (Test-TaskspaceReportedEvidenceLevelIsFormalE3 $report.evidence)) {
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
    $validE3 = @($all | Where-Object { $_.evidence.included_in_e3_aggregate -and (Test-TaskspaceReportedEvidenceLevelIsFormalE3 $_.evidence) })
    $included = @($all | Where-Object { $_.evidence.included_in_utility_aggregate -or $_.evidence.included_in_e3_aggregate })
    $e3Rows = @($all | Where-Object { Test-TaskspaceReportedEvidenceLevelIsFormalE3 $_.evidence })
    $partialRows = @($all | Where-Object { [string]$_.evidence.reported_evidence_level -like "*candidate" })
    $environmentRows = @($all | Where-Object {
            (@($_.evidence.evidence_gate_failures) + @($_.evidence.e3_gate_failures)) -match "environment|remote_asset|docker_"
        })
    $auditReadyRows = @($e3Rows | Where-Object { $_.evidence.human_review_completed -and @($_.evidence.e3_gate_failures).Count -eq 0 })
    $reviewCompleted = @($e3Rows | Where-Object { $_.evidence.human_review_completed })
    $validE3Reviewed = @($validE3 | Where-Object { $_.evidence.human_review_completed })
    $reviewDisagreements = @($e3Rows | Where-Object { $_.evidence.human_review_disagreement })
    $decisionCounts = @{}
    foreach ($row in $validE3Reviewed) {
        $decision = [string]$row.evidence.human_review_decision
        if ([string]::IsNullOrWhiteSpace($decision)) { $decision = "missing" }
        if (-not $decisionCounts.ContainsKey($decision)) { $decisionCounts[$decision] = 0 }
        $decisionCounts[$decision]++
    }
    $lines = @(
        "# TaskSpace Benchmark Aggregate Report",
        "",
        "- configured_pairs: $($all.Count)",
        "- eligible_pairs: $($all.Count - $environmentRows.Count)",
        "- environment_failed_pairs: $($environmentRows.Count)",
        "- partial_pairs: $($partialRows.Count)",
        "- e3_candidate_pairs: $(@($all | Where-Object { [string]$_.evidence.reported_evidence_level -eq 'E3-candidate' }).Count)",
        "- audit_ready_pairs: $($auditReadyRows.Count)",
        "- e3_included_pairs: $($validE3.Count)",
        "- all_pairs: $($all.Count)",
        "- valid_utility_pairs: $($valid.Count)"
    )
    if ($e3Rows.Count -gt 0) {
        $decisionSummary = @($decisionCounts.Keys | Sort-Object | ForEach-Object { "$_=$($decisionCounts[$_])" }) -join "; "
        $lines += "- valid_e3_pairs: $($validE3.Count)"
        $lines += "- e3_human_review_completed_pairs: $($reviewCompleted.Count)"
        $lines += "- e3_human_review_disagreement_pairs: $($reviewDisagreements.Count)"
        $lines += "- e3_human_review_decisions: $decisionSummary"
        $lines += "- e3_taskspace_better_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_taskspace_better' }).Count)"
        $lines += "- e3_standard_better_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_standard_better' }).Count)"
        $lines += "- e3_no_clear_delta_pairs: $(@($validE3Reviewed | Where-Object { [string]$_.evidence.human_review_decision -eq 'include_no_clear_delta' }).Count)"
        $lines += "- e3_taskspace_benefit_note: only include_taskspace_better counts as directional TaskSpace benefit evidence"
    }
    $lines += "- excluded_pairs: $($all.Count - $included.Count)"
    foreach ($report in $all) {
        $lines += ""
        $lines += "## Pair $($report.repeat)"
        $lines += "- pair_report: $($report.pair_report)"
        $lines += "- reported_evidence_level: $($report.evidence.reported_evidence_level)"
        $lines += "- included_in_utility_aggregate: $($report.evidence.included_in_utility_aggregate)"
        $hasE3Diagnostics = (Test-TaskspaceReportedEvidenceLevelIsFormalE3 $report.evidence) -or @($report.evidence.e3_gate_failures).Count -gt 0
        if ($hasE3Diagnostics) {
            $lines += "- included_in_e3_aggregate: $($report.evidence.included_in_e3_aggregate)"
            $lines += "- human_review_completed: $($report.evidence.human_review_completed)"
            $lines += "- human_review_decision: $($report.evidence.human_review_decision)"
            $lines += "- human_review_disagreement: $($report.evidence.human_review_disagreement)"
        }
        $lines += "- evidence_gate_failures: $(if (@($report.evidence.evidence_gate_failures).Count -eq 0) { 'none' } else { @($report.evidence.evidence_gate_failures) -join ', ' })"
        if ($hasE3Diagnostics) {
            $lines += "- e3_gate_failures: $(if (@($report.evidence.e3_gate_failures).Count -eq 0) { 'none' } else { @($report.evidence.e3_gate_failures) -join ', ' })"
        }
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}
