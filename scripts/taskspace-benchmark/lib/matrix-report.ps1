function Get-TaskspaceMatrixReportData {
    param(
        [Parameter(Mandatory = $true)]$Rows,
        [Parameter(Mandatory = $true)][string[]]$RequiredLevels,
        [Parameter(Mandatory = $true)][int]$Repeats
    )
    $rowArray = @($Rows)
    $levels = @($rowArray | ForEach-Object { $_.level } | Sort-Object -Unique)
    $evidenceBlocking = New-Object System.Collections.Generic.List[string]
    foreach ($level in @($RequiredLevels)) {
        if ($levels -notcontains $level) { $evidenceBlocking.Add("missing_required_level_$level") }
    }
    foreach ($row in $rowArray) {
        if ($row.exit_code -ne 0) { $evidenceBlocking.Add("$($row.scenario): runner_exit_$($row.exit_code)") }
        if ($row.valid_pairs -lt $Repeats) { $evidenceBlocking.Add("$($row.scenario): valid_pairs_$($row.valid_pairs)_lt_$Repeats") }
        if ($row.excluded_pairs -gt 0) { $evidenceBlocking.Add("$($row.scenario): excluded_pairs_$($row.excluded_pairs)") }
        if ($row.non_e2_reports -gt 0) { $evidenceBlocking.Add("$($row.scenario): non_e2_reports_$($row.non_e2_reports)") }
    }
    $warningGaps = @($rowArray | Where-Object { [int]$_.warning_pairs -gt 0 } | ForEach-Object {
            "$($_.scenario): warning_pairs_$($_.warning_pairs)"
        })
    $utilityCostGaps = @($rowArray | Where-Object { $_.PSObject.Properties.Name -contains "utility_warning_pairs" -and [int]$_.utility_warning_pairs -gt 0 } | ForEach-Object {
            "$($_.scenario): utility_warning_pairs_$($_.utility_warning_pairs)"
        })
    [pscustomobject]@{
        rows = $rowArray
        levels = $levels
        evidence_blocking = @($evidenceBlocking.ToArray())
        warning_gaps = @($warningGaps)
        utility_cost_gaps = @($utilityCostGaps)
        e2_evidence_readiness = ($evidenceBlocking.Count -eq 0)
        e2_clean_readiness = ($evidenceBlocking.Count -eq 0 -and @($warningGaps).Count -eq 0)
        e2_utility_clean_readiness = ($evidenceBlocking.Count -eq 0 -and @($warningGaps).Count -eq 0 -and @($utilityCostGaps).Count -eq 0)
    }
}
