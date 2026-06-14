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

function Get-TaskspaceTimingPercent {
    param([int64]$Value, [int64]$Total)
    if ($Total -le 0) { return 0.0 }
    [Math]::Round((100.0 * [double]$Value / [double]$Total), 2)
}

function Convert-TaskspaceTimingHashtable {
    param([hashtable]$Table)
    $ordered = [ordered]@{}
    foreach ($key in @($Table.Keys | Sort-Object)) { $ordered[$key] = $Table[$key] }
    [pscustomobject]$ordered
}

function Get-TaskspaceLargestTimingSpan {
    param([hashtable]$Durations)
    $largestName = ""
    $largestMs = [int64]0
    foreach ($key in @($Durations.Keys)) {
        $value = [int64]$Durations[$key]
        if ($value -gt $largestMs) { $largestName = [string]$key; $largestMs = $value }
    }
    [pscustomobject]@{ name = $largestName; duration_ms = $largestMs }
}

function Get-TaskspaceTimingBottleneck {
    param(
        [int64]$TotalMs,
        [int64]$AgentMs,
        [int64]$ValidationMs,
        [int64]$OracleMs,
        [int64]$DockerBuildMs,
        [int64]$CleanupMs,
        [int64]$QueueWaitMs = 0,
        [string[]]$EngineeringUncleanReasons = @()
    )
    if (@($EngineeringUncleanReasons | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0) {
        return [pscustomobject]@{ classification = "engineering_unclean_slow"; reason = "engineering_unclean_reason_present" }
    }
    $validatorSubtotal = $ValidationMs + $OracleMs
    if ((Get-TaskspaceTimingPercent $AgentMs $TotalMs) -ge 70) { return [pscustomobject]@{ classification = "agent_bound"; reason = "agent_subtotal_ge_70_percent" } }
    if ((Get-TaskspaceTimingPercent $validatorSubtotal $TotalMs) -ge 30) { return [pscustomobject]@{ classification = "validator_bound"; reason = "validation_oracle_subtotal_ge_30_percent" } }
    if ((Get-TaskspaceTimingPercent $DockerBuildMs $TotalMs) -ge 15) { return [pscustomobject]@{ classification = "docker_build_bound"; reason = "docker_build_subtotal_ge_15_percent" } }
    if ((Get-TaskspaceTimingPercent $CleanupMs $TotalMs) -ge 5) { return [pscustomobject]@{ classification = "cleanup_bound"; reason = "cleanup_subtotal_ge_5_percent" } }
    if ((Get-TaskspaceTimingPercent $QueueWaitMs $TotalMs) -ge 10) { return [pscustomobject]@{ classification = "queue_bound"; reason = "queue_wait_subtotal_ge_10_percent" } }
    [pscustomobject]@{ classification = "mixed_or_unclassified"; reason = "no_threshold_met" }
}

function New-TaskspaceTimingBreakdown {
    param(
        [int64]$TotalMs,
        [int64]$AgentMs,
        [int64]$ValidationMs,
        [int64]$OracleMs,
        [int64]$DockerBuildMs,
        [int64]$DockerRunMs,
        [int64]$DockerCleanupMs,
        [int64]$QueueWaitMs = 0,
        [string[]]$EngineeringUncleanReasons = @()
    )
    $durations = @{
        agent = $AgentMs
        public_validation = $ValidationMs
        hidden_oracle = $OracleMs
        docker_build = $DockerBuildMs
        docker_run = $DockerRunMs
        docker_cleanup = $DockerCleanupMs
        queue_wait = $QueueWaitMs
    }
    $bottleneck = Get-TaskspaceTimingBottleneck $TotalMs $AgentMs $ValidationMs $OracleMs $DockerBuildMs $DockerCleanupMs $QueueWaitMs $EngineeringUncleanReasons
    [pscustomobject]@{
        total_duration_ms = $TotalMs
        subtotal_percentages = [ordered]@{
            agent = Get-TaskspaceTimingPercent $AgentMs $TotalMs
            public_validation = Get-TaskspaceTimingPercent $ValidationMs $TotalMs
            hidden_oracle = Get-TaskspaceTimingPercent $OracleMs $TotalMs
            validator_and_oracle = Get-TaskspaceTimingPercent ($ValidationMs + $OracleMs) $TotalMs
            docker_build = Get-TaskspaceTimingPercent $DockerBuildMs $TotalMs
            docker_run = Get-TaskspaceTimingPercent $DockerRunMs $TotalMs
            docker_cleanup = Get-TaskspaceTimingPercent $DockerCleanupMs $TotalMs
            queue_wait = Get-TaskspaceTimingPercent $QueueWaitMs $TotalMs
        }
        largest_span = Get-TaskspaceLargestTimingSpan $durations
        bottleneck_classification = [string]$bottleneck.classification
        bottleneck_reason = [string]$bottleneck.reason
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
    $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0
    $metricValues = if ($MetricsBySide) { @($MetricsBySide.Values) } else { @() }
    foreach ($metrics in $metricValues) {
        if ($metrics.PSObject.Properties.Name -contains "docker_build_duration_ms") { $dockerBuildMs += [int64]$metrics.docker_build_duration_ms }
        if ($metrics.PSObject.Properties.Name -contains "docker_run_duration_ms") { $dockerRunMs += [int64]$metrics.docker_run_duration_ms }
        if ($metrics.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { $dockerCleanupMs += [int64]$metrics.docker_cleanup_duration_ms }
    }
    $totalDurationMs = [int64](($PairFinishedAt - $PairStartedAt).TotalMilliseconds)
    $breakdown = New-TaskspaceTimingBreakdown $totalDurationMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs 0 $EngineeringUncleanReasons
    $artifact = [ordered]@{
        schema_version = 1
        scenario = if ($Manifest -and $Manifest.PSObject.Properties.Name -contains "Id") { [string]$Manifest.Id } else { "" }
        pair_id = "pair-{0:000}" -f $Repeat
        pair_dir = $PairDir
        started_at = $PairStartedAt.ToString("o")
        finished_at = $PairFinishedAt.ToString("o")
        total_duration_ms = $totalDurationMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        measured_overhead_ms = $totalDurationMs - $agentMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_reason = [string]$breakdown.bottleneck_reason
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
    $probeMs = if ($ValidationTiming.PSObject.Properties.Name -contains "probe_duration_ms") { $ValidationTiming.probe_duration_ms } else { $null }
    $Metrics | Add-Member -NotePropertyName validator_probe_duration_ms -NotePropertyValue $probeMs -Force
    $Metrics | Add-Member -NotePropertyName docker_observed_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_build_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_run_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_inspect_duration_ms -NotePropertyValue $null -Force
    $Metrics | Add-Member -NotePropertyName docker_cleanup_duration_ms -NotePropertyValue $null -Force
    $timeoutPhase = if ($Metrics.PSObject.Properties.Name -contains "validation_timeout_phase" -and -not [string]::IsNullOrWhiteSpace([string]$Metrics.validation_timeout_phase)) {
        [string]$Metrics.validation_timeout_phase
    } elseif ([int]$Metrics.public_validation_exit_code -eq 124) {
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
            if ($phases.Count -ge 1) {
                $first = [datetime]::Parse([string]$phases[0].started_at)
                $last = [datetime]::Parse([string]$phases[-1].finished_at)
                $Metrics.docker_observed_duration_ms = [int64](($last - $first).TotalMilliseconds)
            }
        } catch {
            $Metrics.docker_observed_duration_ms = $null
        }
    }
    if ($Metrics.PSObject.Properties.Name -contains "validation_cleanup_result_path" -and $Metrics.validation_cleanup_result_path -and (Test-Path -LiteralPath $Metrics.validation_cleanup_result_path)) {
        try {
            $cleanup = Get-Content -Raw -Encoding UTF8 -LiteralPath $Metrics.validation_cleanup_result_path | ConvertFrom-Json
            if ($cleanup.PSObject.Properties.Name -contains "duration_ms") {
                $Metrics.docker_cleanup_duration_ms = [int64]$Metrics.docker_cleanup_duration_ms + [int64]$cleanup.duration_ms
            }
        } catch {}
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
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0; $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0
    $bottleneckCounts = @{}
    foreach ($pair in $pairs) {
        $totalMs += [int64]$pair.total_duration_ms
        $agentMs += [int64]$pair.agent_duration_ms
        $validationMs += [int64]$pair.public_validation_duration_ms
        $oracleMs += [int64]$pair.hidden_oracle_duration_ms
        $overheadMs += [int64]$pair.measured_overhead_ms
        if ($pair.PSObject.Properties.Name -contains "docker_build_duration_ms") { $dockerBuildMs += [int64]$pair.docker_build_duration_ms }
        if ($pair.PSObject.Properties.Name -contains "docker_run_duration_ms") { $dockerRunMs += [int64]$pair.docker_run_duration_ms }
        if ($pair.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { $dockerCleanupMs += [int64]$pair.docker_cleanup_duration_ms }
        $class = if ($pair.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$pair.bottleneck_classification } else { "unknown" }
        if (-not $bottleneckCounts.ContainsKey($class)) { $bottleneckCounts[$class] = 0 }
        $bottleneckCounts[$class]++
    }
    $breakdown = New-TaskspaceTimingBreakdown $totalMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs
    $artifact = [ordered]@{
        schema_version = 1
        sample_id = $SampleId
        run_dir = $RunDir
        pair_count = @($pairs).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        measured_overhead_ms = $overheadMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_counts = Convert-TaskspaceTimingHashtable $bottleneckCounts
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
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0; $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0
    $bottleneckCounts = @{}
    foreach ($sample in $samples) {
        $totalMs += [int64]$sample.total_pair_duration_ms
        $agentMs += [int64]$sample.agent_duration_ms
        $validationMs += [int64]$sample.public_validation_duration_ms
        $oracleMs += [int64]$sample.hidden_oracle_duration_ms
        $overheadMs += [int64]$sample.measured_overhead_ms
        if ($sample.PSObject.Properties.Name -contains "docker_build_duration_ms") { $dockerBuildMs += [int64]$sample.docker_build_duration_ms }
        if ($sample.PSObject.Properties.Name -contains "docker_run_duration_ms") { $dockerRunMs += [int64]$sample.docker_run_duration_ms }
        if ($sample.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { $dockerCleanupMs += [int64]$sample.docker_cleanup_duration_ms }
        if ($sample.PSObject.Properties.Name -contains "bottleneck_counts") {
            foreach ($prop in @($sample.bottleneck_counts.PSObject.Properties)) {
                if (-not $bottleneckCounts.ContainsKey($prop.Name)) { $bottleneckCounts[$prop.Name] = 0 }
                $bottleneckCounts[$prop.Name] += [int]$prop.Value
            }
        }
    }
    $breakdown = New-TaskspaceTimingBreakdown $totalMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs
    $artifact = [ordered]@{
        schema_version = 1
        suite_root = $SuiteRoot
        sample_count = @($SampleStatuses).Count
        timing_sample_count = @($samples).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        measured_overhead_ms = $overheadMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_counts = Convert-TaskspaceTimingHashtable $bottleneckCounts
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
