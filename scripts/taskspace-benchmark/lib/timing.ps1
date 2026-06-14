$ErrorActionPreference = "Stop"

function New-TaskspaceTimingSpan {
    param(
        [Parameter(Mandatory = $true)][string]$Phase,
        [datetime]$StartedAt,
        [datetime]$FinishedAt,
        [string]$Side = "",
        [string]$LogicalMode = "",
        [int]$ExitCode = 0,
        [bool]$TimedOut = $false,
        [string[]]$EngineeringUncleanReasons = @()
    )
    [pscustomobject]@{
        phase = $Phase
        side = $Side
        logical_mode = $LogicalMode
        started_at = $StartedAt.ToString("o")
        finished_at = $FinishedAt.ToString("o")
        duration_ms = [int64](($FinishedAt - $StartedAt).TotalMilliseconds)
        exit_code = $ExitCode
        timed_out = $TimedOut
        engineering_unclean_reasons = @($EngineeringUncleanReasons)
    }
}

function Write-TaskspacePairTiming {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][int]$Repeat,
        [datetime]$PairStartedAt,
        [datetime]$PairFinishedAt,
        $Manifest,
        $Pair,
        $MetricsBySide,
        $ValidationTimingBySide,
        [string[]]$EngineeringUncleanReasons = @()
    )
    $spans = New-Object System.Collections.Generic.List[object]
    $spans.Add((New-TaskspaceTimingSpan "pair_total" $PairStartedAt $PairFinishedAt "" "" 0 $false $EngineeringUncleanReasons))
    foreach ($sideName in @("left", "right")) {
        $metrics = if ($MetricsBySide -and $MetricsBySide.ContainsKey($sideName)) { $MetricsBySide[$sideName] } else { $null }
        if ($metrics) {
            $duration = [int64]$metrics.wall_time_ms
            $finished = $PairStartedAt.AddMilliseconds($duration)
            $spans.Add((New-TaskspaceTimingSpan "agent_execution" $PairStartedAt $finished $sideName ([string]$metrics.logical_mode) ([int]$metrics.exec_exit_code) ([bool]$metrics.exec_timed_out) @($metrics.validator_environment_failures)))
        }
        $validationTiming = if ($ValidationTimingBySide -and $ValidationTimingBySide.ContainsKey($sideName)) { $ValidationTimingBySide[$sideName] } else { $null }
        if ($validationTiming) {
            $spans.Add((New-TaskspaceTimingSpan "public_validation" $validationTiming.validation_started_at $validationTiming.validation_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.validation_exit_code) ([int]$validationTiming.validation_exit_code -eq 124) @($validationTiming.engineering_unclean_reasons)))
            $spans.Add((New-TaskspaceTimingSpan "hidden_oracle" $validationTiming.oracle_started_at $validationTiming.oracle_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.oracle_exit_code) $false @()))
        }
    }
    $agentMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "agent_execution" })) { $agentMs += [int64]$span.duration_ms }
    $validationMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "public_validation" })) { $validationMs += [int64]$span.duration_ms }
    $oracleMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "hidden_oracle" })) { $oracleMs += [int64]$span.duration_ms }
    $artifact = [ordered]@{
        schema_version = 1
        scenario = if ($Manifest -and $Manifest.PSObject.Properties.Name -contains "Id") { [string]$Manifest.Id } else { "" }
        pair_id = "pair-{0:000}" -f $Repeat
        pair_dir = $PairDir
        started_at = $PairStartedAt.ToString("o")
        finished_at = $PairFinishedAt.ToString("o")
        total_duration_ms = [int64](($PairFinishedAt - $PairStartedAt).TotalMilliseconds)
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        measured_overhead_ms = [int64](($PairFinishedAt - $PairStartedAt).TotalMilliseconds) - $agentMs
        engineering_unclean_reasons = @($EngineeringUncleanReasons)
        spans = @($spans.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
    $path = Join-Path $PairDir "pair-timing.json"
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}
