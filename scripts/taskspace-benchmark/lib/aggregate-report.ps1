function Write-TaskspaceAggregateReport {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$PairReports
    )
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace Benchmark Aggregate Report")
    $lines.Add("")
    $lines.Add("This report is reserved for MVP+1 and later utility aggregate runs.")
    $lines.Add("")
    $lines.Add("- configured_pairs: $(@($PairReports).Count)")
    $lines.Add("- eligible_pairs: 0")
    $lines.Add("- environment_failed_pairs: 0")
    $lines.Add("- partial_pairs: 0")
    $lines.Add("- e3_candidate_pairs: 0")
    $lines.Add("- audit_ready_pairs: 0")
    $lines.Add("- e3_included_pairs: 0")
    $lines.Add("- all_pairs: $(@($PairReports).Count)")
    $lines.Add("- valid_utility_pairs: 0")
    $lines.Add("- excluded_pairs: $(@($PairReports).Count)")
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}
