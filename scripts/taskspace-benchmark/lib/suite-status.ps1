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
