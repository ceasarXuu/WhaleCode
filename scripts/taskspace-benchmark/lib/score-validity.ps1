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
