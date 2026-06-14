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

function Add-TaskspaceMetricTimingFields {
    param($Metrics, $ValidationTiming)
    if (-not $Metrics -or -not $ValidationTiming) { return $Metrics }
    $validationMs = [int64](($ValidationTiming.validation_finished_at - $ValidationTiming.validation_started_at).TotalMilliseconds)
    $oracleMs = [int64](($ValidationTiming.oracle_finished_at - $ValidationTiming.oracle_started_at).TotalMilliseconds)
    $Metrics | Add-Member -NotePropertyName public_validation_duration_ms -NotePropertyValue $validationMs -Force
    $Metrics | Add-Member -NotePropertyName hidden_oracle_duration_ms -NotePropertyValue $oracleMs -Force
    $Metrics | Add-Member -NotePropertyName validator_probe_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_observed_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_build_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_run_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_inspect_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_cleanup_duration_ms -NotePropertyValue $null -Force
    $timeoutPhase = if ([int]$Metrics.public_validation_exit_code -eq 124) {
        if ($Metrics.PSObject.Properties.Name -contains "tests_started_seen" -and [bool]$Metrics.tests_started_seen) { "tests" } else { "pretest" }
    } else { "" }
    $Metrics | Add-Member -NotePropertyName validation_timeout_phase -NotePropertyValue $timeoutPhase -Force
    if ($Metrics.PSObject.Properties.Name -contains "docker_build_result_path" -and $Metrics.docker_build_result_path -and (Test-Path -LiteralPath $Metrics.docker_build_result_path)) {
        try {
            $docker = Get-Content -Raw -Encoding UTF8 -LiteralPath $Metrics.docker_build_result_path | ConvertFrom-Json
            $phases = @($docker.phases | Where-Object { $_.timestamp })
            foreach ($phase in @($docker.phases)) {
                if (-not ($phase.PSObject.Properties.Name -contains "duration_ms")) { continue }
                $duration = [int64]$phase.duration_ms
                switch ([string]$phase.phase) {
                    "build" { $Metrics.docker_build_duration_ms = $duration }
                    "run" { $Metrics.docker_run_duration_ms = $duration }
                    "inspect" { $Metrics.docker_inspect_duration_ms = $duration }
                    "cleanup_container" { $Metrics.docker_cleanup_duration_ms = [int64]$Metrics.docker_cleanup_duration_ms + $duration }
                    "cleanup_image" { $Metrics.docker_cleanup_duration_ms = [int64]$Metrics.docker_cleanup_duration_ms + $duration }
                }
            }
            if ($phases.Count -ge 2) {
                $first = [datetime]::Parse([string]$phases[0].timestamp)
                $last = [datetime]::Parse([string]$phases[-1].timestamp)
                $Metrics.docker_observed_duration_ms = [int64](($last - $first).TotalMilliseconds)
            }
        } catch {
            $Metrics.docker_observed_duration_ms = $null
        }
    }
    $Metrics
}

function Write-TaskspaceSampleTiming {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$SampleId
    )
    $pairTimingFiles = @(Get-ChildItem -LiteralPath $RunDir -Filter "pair-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $pairDirs = @(Get-ChildItem -LiteralPath $RunDir -Directory -Filter "pair-*" -ErrorAction SilentlyContinue | Sort-Object FullName)
    $pairTimingParents = @($pairTimingFiles | ForEach-Object { (Split-Path -Parent $_.FullName) })
    $missingPairTimingDirs = @($pairDirs | Where-Object { $pairTimingParents -notcontains $_.FullName } | ForEach-Object { $_.FullName })
    $parseErrors = New-Object System.Collections.Generic.List[string]
    $pairs = @()
    foreach ($file in $pairTimingFiles) {
        try { $pairs += (Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json) } catch { $parseErrors.Add($file.FullName) }
    }
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0
    foreach ($pair in $pairs) {
        $totalMs += [int64]$pair.total_duration_ms
        $agentMs += [int64]$pair.agent_duration_ms
        $validationMs += [int64]$pair.public_validation_duration_ms
        $oracleMs += [int64]$pair.hidden_oracle_duration_ms
        $overheadMs += [int64]$pair.measured_overhead_ms
    }
    $artifact = [ordered]@{
        schema_version = 1
        sample_id = $SampleId
        run_dir = $RunDir
        pair_count = @($pairs).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        measured_overhead_ms = $overheadMs
        missing_pair_timing_count = @($missingPairTimingDirs).Count
        missing_pair_timing_dirs = @($missingPairTimingDirs)
        timing_parse_error_count = $parseErrors.Count
        timing_parse_error_paths = @($parseErrors.ToArray())
        pair_timing_paths = @($pairTimingFiles | ForEach-Object { $_.FullName })
        generated_at = (Get-Date).ToString("o")
    }
    $path = Join-Path $RunDir "sample-timing.json"
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}

function Write-TaskspaceSuiteTiming {
    param(
        [Parameter(Mandatory = $true)][string]$SuiteRoot,
        [Parameter(Mandatory = $true)]$SampleStatuses
    )
    $sampleTimingFiles = @(Get-ChildItem -LiteralPath $SuiteRoot -Filter "sample-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $sampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $SuiteRoot "samples") -Directory -ErrorAction SilentlyContinue | Sort-Object FullName)
    $statusSampleDirs = @($SampleStatuses | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains "sample_root" -and -not [string]::IsNullOrWhiteSpace([string]$_.sample_root)) {
                [string]$_.sample_root
            }
        })
    $expectedSampleDirs = @(@($sampleDirs | ForEach-Object { $_.FullName }) + $statusSampleDirs | Sort-Object -Unique)
    $sampleTimingParents = @($sampleTimingFiles | ForEach-Object { (Split-Path -Parent $_.FullName) })
    $missingSampleTimingDirs = @($expectedSampleDirs | Where-Object { $sampleTimingParents -notcontains $_ })
    $parseErrors = New-Object System.Collections.Generic.List[string]
    $samples = @()
    foreach ($file in $sampleTimingFiles) {
        try { $samples += (Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json) } catch { $parseErrors.Add($file.FullName) }
    }
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0
    foreach ($sample in $samples) {
        $totalMs += [int64]$sample.total_pair_duration_ms
        $agentMs += [int64]$sample.agent_duration_ms
        $validationMs += [int64]$sample.public_validation_duration_ms
        $oracleMs += [int64]$sample.hidden_oracle_duration_ms
        $overheadMs += [int64]$sample.measured_overhead_ms
    }
    $artifact = [ordered]@{
        schema_version = 1
        suite_root = $SuiteRoot
        sample_count = @($SampleStatuses).Count
        timing_sample_count = @($samples).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        measured_overhead_ms = $overheadMs
        missing_sample_timing_count = @($missingSampleTimingDirs).Count
        missing_sample_timing_dirs = @($missingSampleTimingDirs)
        timing_parse_error_count = $parseErrors.Count
        timing_parse_error_paths = @($parseErrors.ToArray())
        sample_timing_paths = @($sampleTimingFiles | ForEach-Object { $_.FullName })
        generated_at = (Get-Date).ToString("o")
    }
    $path = Join-Path $SuiteRoot "suite-timing.json"
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}
