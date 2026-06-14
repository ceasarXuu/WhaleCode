$ErrorActionPreference = "Stop"

function New-TaskspaceScoringAbort {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][string]$PairReportPath,
        [Parameter(Mandatory = $true)]$AuditManifest,
        [Parameter(Mandatory = $true)][int]$Repeat,
        [Parameter(Mandatory = $true)][int]$Repeats
    )
    $firstReason = @($AuditManifest.engineering_unclean_reasons | Select-Object -First 1)[0]
    $reason = if ($firstReason) { [string]$firstReason } else { "engineering_unclean" }
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "score_validity" $reason "Engineering unclean pair in scoring mode" "" $PairReportPath
    $abortPath = Join-Path $PairDir "pair-abort.json"
    Write-TaskspaceJson ([pscustomobject]@{
            abort_scope = "sample"
            abort_phase = "score_validity"
            reason = $reason
            infra_signature = $signature
            first_failure_artifact = $PairReportPath
            skipped_repeats = if ($Repeat -lt $Repeats) { @(($Repeat + 1)..$Repeats) } else { @() }
            engineering_unclean_reasons = @($AuditManifest.engineering_unclean_reasons)
        }) $abortPath
    [pscustomobject]@{
        abort_path = $abortPath
        reason = $reason
        signature = $signature
    }
}

function Stop-TaskspaceScoringInvalidRun {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][string]$PairReportPath,
        [Parameter(Mandatory = $true)]$Evidence,
        [Parameter(Mandatory = $true)][string]$CommandLine,
        [Parameter(Mandatory = $true)][int]$Repeat,
        [Parameter(Mandatory = $true)][int]$Repeats,
        [string]$TaskListHash = "",
        [string]$SourceVersion = "",
        [string]$ProfileHash = ""
    )
    $auditManifest = [pscustomobject]@{
        engineering_unclean_reasons = if ($Evidence.PSObject.Properties.Name -contains "engineering_unclean_reasons") { @($Evidence.engineering_unclean_reasons) } else { @("engineering_unclean") }
    }
    $abort = New-TaskspaceScoringAbort $PairDir $PairReportPath $auditManifest $Repeat $Repeats
    Write-TaskspaceRunEvent $RunDir "score_validity_evaluated" @{ repeat = $Repeat; score_valid = $false; reasons = @($auditManifest.engineering_unclean_reasons) }
    Write-TaskspaceRunEvent $RunDir "scoring_run_aborted" @{ repeat = $Repeat; reasons = @($auditManifest.engineering_unclean_reasons); first_failure_artifact = $PairReportPath; pair_abort = $abort.abort_path }
    Set-TaskspaceInvalidHarnessStatus $RunDir $SampleId "score_validity" $abort.reason $abort.signature $abort.abort_path $CommandLine $Repeat $Repeat | Out-Null
    $sampleTimingPath = Write-TaskspaceSampleTiming -RunDir $RunDir -SampleId $SampleId -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
    Write-TaskspaceRuntimeBottleneckReport -TimingPath $sampleTimingPath -ScoreValid $false | Out-Null
    $abort
}
