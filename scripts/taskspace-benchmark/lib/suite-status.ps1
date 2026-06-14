$ErrorActionPreference = "Stop"

function Test-TaskspaceSuiteChildStatusComplete {
    param($Status, [int]$Repeats)
    if ($null -eq $Status) { return $false }
    $names = @($Status.PSObject.Properties.Name)
    if (-not ($names -contains "run_validity") -or [string]$Status.run_validity -ne "valid") { return $false }
    if (-not ($names -contains "phase")) { return $false }
    if (@("completed", "audit_required", "finalize") -notcontains [string]$Status.phase) { return $false }
    if (-not ($names -contains "attempted_pairs") -or -not ($names -contains "completed_pairs")) { return $false }
    return ([int]$Status.attempted_pairs -ge $Repeats -and [int]$Status.completed_pairs -ge $Repeats)
}

function New-TaskspaceSuiteChildFailureStatus {
    param($Status, [string]$SampleId, [string]$TaskDir, [int]$ChildExit, [string]$StatusPath, [string]$SampleRoot)
    [pscustomobject]@{
        sample_id = if ($Status -and $Status.PSObject.Properties.Name -contains "sample_id") { [string]$Status.sample_id } else { $SampleId }
        task_dir = $TaskDir
        run_validity = "invalid_harness"
        exit_code = $ChildExit
        abort_scope = "sample"
        abort_phase = "child_process"
        abort_signature = "harness_materialization_failure/child_process_failed"
        abort_reason = "child_exit_$ChildExit"
        attempted_pairs = if ($Status -and $Status.PSObject.Properties.Name -contains "attempted_pairs") { $Status.attempted_pairs } else { 0 }
        completed_pairs = if ($Status -and $Status.PSObject.Properties.Name -contains "completed_pairs") { $Status.completed_pairs } else { 0 }
        first_failure_artifact = if ($StatusPath) { $StatusPath } else { $SampleRoot }
        sample_root = $SampleRoot
    }
}

function Get-TaskspaceSuiteScoreValiditySummary {
    param($SampleStatuses, [int]$Repeats)
    $statuses = @($SampleStatuses)
    $completed = @($statuses | Where-Object { Test-TaskspaceSuiteChildStatusComplete $_ $Repeats })
    $valid = @($statuses | Where-Object { $_.PSObject.Properties.Name -contains "run_validity" -and [string]$_.run_validity -eq "valid" })
    $invalid = @($statuses | Where-Object { $_.PSObject.Properties.Name -contains "run_validity" -and [string]$_.run_validity -eq "invalid_harness" })
    $firstInvalid = @($invalid | Select-Object -First 1)[0]
    [pscustomobject]@{
        completed_child_processes = $completed.Count
        score_valid_child_runs = $valid.Count
        score_invalid_child_runs = $invalid.Count
        first_score_invalid_run = if ($firstInvalid) {
            if ($firstInvalid.PSObject.Properties.Name -contains "sample_id") { [string]$firstInvalid.sample_id } else { "" }
        } else { "" }
        suite_score_valid = ($invalid.Count -eq 0)
    }
}

function Get-TaskspaceSuiteRemainingSkippedPairs {
    param([Parameter(Mandatory = $true)][string]$SuiteRoot)
    $skippedPairs = 0
    foreach ($abortFile in @(Get-ChildItem -LiteralPath $SuiteRoot -Filter "pair-abort.json" -Recurse -ErrorAction SilentlyContinue)) {
        try {
            $abort = Get-Content -Raw -Encoding UTF8 -LiteralPath $abortFile.FullName | ConvertFrom-Json
            $skippedPairs += @($abort.skipped_repeats).Count
        } catch {}
    }
    $skippedPairs
}

function Get-TaskspaceSuiteExpectedTimeSaved {
    param([Parameter(Mandatory = $true)][string]$SuiteRoot, $SampleStatuses, [int]$Repeats)
    $sampleTimingFiles = @(Get-ChildItem -LiteralPath $SuiteRoot -Filter "sample-timing.json" -Recurse -ErrorAction SilentlyContinue)
    $totalMs = [int64]0
    $pairCount = 0
    foreach ($file in $sampleTimingFiles) {
        try {
            $timing = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json
            if ($timing.PSObject.Properties.Name -contains "pair_count" -and [int]$timing.pair_count -gt 0) {
                $pairCount += [int]$timing.pair_count
                $totalMs += [int64]$timing.total_pair_duration_ms
            }
        } catch {}
    }
    $skippedPairs = Get-TaskspaceSuiteRemainingSkippedPairs $SuiteRoot
    $skippedSamples = @($SampleStatuses | Where-Object { $_.PSObject.Properties.Name -contains "skipped_reason" -and -not [string]::IsNullOrWhiteSpace([string]$_.skipped_reason) }).Count
    $skippedPairEquivalent = $skippedPairs + ($skippedSamples * $Repeats)
    if ($pairCount -le 0 -or $skippedPairEquivalent -le 0) {
        return [pscustomobject]@{
            expected_time_saved_minutes = $null
            skipped_pair_equivalent_count = $skippedPairEquivalent
            expected_time_saved_basis = if ($skippedPairEquivalent -le 0) { "no_skipped_work" } else { "no_serial_baseline" }
        }
    }
    $averagePairMs = [double]$totalMs / [double]$pairCount
    [pscustomobject]@{
        expected_time_saved_minutes = [Math]::Round((($averagePairMs * $skippedPairEquivalent) / 60000.0), 2)
        skipped_pair_equivalent_count = $skippedPairEquivalent
        expected_time_saved_basis = "observed_average_pair_duration_ms=$([Math]::Round($averagePairMs, 0))"
    }
}
