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

function Get-TaskspaceTopTimingSpans {
    param([hashtable]$Durations, [int]$Count = 3)
    @($Durations.Keys | ForEach-Object {
            [pscustomobject]@{ name = [string]$_; duration_ms = [int64]$Durations[$_] }
        } | Sort-Object -Property duration_ms -Descending | Select-Object -First $Count)
}

function Get-TaskspaceTimingPercentile {
    param([int64[]]$Values, [double]$Percentile)
    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) { return $null }
    if ($sorted.Count -eq 1) { return [int64]$sorted[0] }
    $rank = [Math]::Ceiling(($Percentile / 100.0) * [double]$sorted.Count) - 1
    $index = [Math]::Max(0, [Math]::Min($sorted.Count - 1, [int]$rank))
    [int64]$sorted[$index]
}

function Get-TaskspaceRequiredWaitTimingFields {
    @(
        "model_queue_wait_ms",
        "model_retry_backoff_ms",
        "model_request_duration_ms",
        "process_launch_wait_ms",
        "docker_token_wait_ms",
        "validation_token_wait_ms",
        "disk_reservation_wait_ms",
        "cache_lock_wait_ms",
        "resource_wait_ms_total"
    )
}

function New-TaskspaceMissingWaitAttribution {
    param($Object)
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($field in @(Get-TaskspaceRequiredWaitTimingFields)) {
        $unavailable = (
            $Object -and
            $Object.PSObject.Properties.Name -contains "wait_attribution_unavailable_fields" -and
            $Object.wait_attribution_unavailable_fields -and
            $Object.wait_attribution_unavailable_fields.PSObject.Properties.Name -contains $field
        )
        if ($unavailable) { continue }
        if (-not $Object -or -not ($Object.PSObject.Properties.Name -contains $field) -or $null -eq $Object.$field) {
            $missing.Add($field)
        }
    }
    @($missing.ToArray())
}

function New-TaskspaceSerialResourceWaitAttribution {
    [pscustomobject]@{
        docker_token_wait_ms = 0
        validation_token_wait_ms = 0
        disk_reservation_wait_ms = 0
        cache_lock_wait_ms = 0
        resource_wait_ms_total = 0
        resource_wait_attribution_mode = "serial_no_resource_governor"
    }
}

function New-TaskspaceUnavailableWaitAttribution {
    param([string[]]$Fields, [string]$Reason)
    $result = [ordered]@{}
    foreach ($field in @($Fields)) {
        if (-not [string]::IsNullOrWhiteSpace($field)) { $result[$field] = $Reason }
    }
    [pscustomobject]$result
}

function Convert-TaskspaceTimingTraceTags {
    param($Tags)
    $table = [ordered]@{}
    foreach ($tag in @($Tags)) {
        $text = [string]$tag
        if ([string]::IsNullOrWhiteSpace($text)) { continue }
        $index = $text.IndexOf(":")
        if ($index -lt 0) {
            $table[$text] = $true
            continue
        }
        $key = $text.Substring(0, $index)
        $value = $text.Substring($index + 1)
        if (-not [string]::IsNullOrWhiteSpace($key)) { $table[$key] = $value }
    }
    [pscustomobject]$table
}

function Get-TaskspaceTimingIntValue {
    param($Object, [string[]]$Names)
    if ($null -eq $Object) { return $null }
    foreach ($name in @($Names)) {
        if ($Object.PSObject.Properties.Name -contains $name -and $null -ne $Object.$name -and -not [string]::IsNullOrWhiteSpace([string]$Object.$name)) {
            try { return [int64]$Object.$name } catch { return $null }
        }
    }
    $null
}

function Get-TaskspaceTimingFieldValue {
    param($Object, [string]$Name)
    if ($null -eq $Object -or -not ($Object.PSObject.Properties.Name -contains $Name)) { return $null }
    $Object.$Name
}

function Get-TaskspaceModelTimingSourcePath {
    param(
        [string]$ArtifactDir,
        [string]$FallbackJsonlPath
    )
    if (-not [string]::IsNullOrWhiteSpace($ArtifactDir)) {
        $rolloutTimingPath = Join-Path $ArtifactDir "rollout.jsonl"
        if (Test-Path -LiteralPath $rolloutTimingPath) { return $rolloutTimingPath }
    }
    $FallbackJsonlPath
}

function Get-TaskspaceModelTimingAttribution {
    param([string]$JsonlPath)
    $requestMs = [int64]0
    $eventCount = 0
    $providerRequestMs = [int64]0
    $providerEventCount = 0
    $providerRequests = @{}
    $parseErrors = 0
    if ([string]::IsNullOrWhiteSpace($JsonlPath) -or -not (Test-Path -LiteralPath $JsonlPath)) {
        return [pscustomobject]@{
            model_request_duration_ms = $null
            model_queue_wait_ms = $null
            model_retry_backoff_ms = $null
            model_timing_event_count = 0
            model_timing_source_status = "jsonl_missing"
            model_timing_source_path = $JsonlPath
            model_timing_parse_errors = 0
        }
    }
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $JsonlPath -ErrorAction SilentlyContinue)) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.Trim() -eq "artifact") { continue }
        try { $evt = $line | ConvertFrom-Json } catch { $parseErrors++; continue }
        $eventType = if ($evt.PSObject.Properties.Name -contains "type") { [string]$evt.type } else { "" }
        $payload = if ($evt.PSObject.Properties.Name -contains "payload") { $evt.payload } else { $null }
        $payloadType = if ($payload -and $payload.PSObject.Properties.Name -contains "type") { [string]$payload.type } else { "" }
        if ($eventType -eq "event_msg" -and $payload -and [string]$payload.kind -eq "provider_request_budget") {
            $tags = Convert-TaskspaceTimingTraceTags $payload.tags
            $status = [string]$tags.status
            $producer = [string]$tags.producer
            $callId = [string](Get-TaskspaceTimingFieldValue $payload "callId")
            if ([string]::IsNullOrWhiteSpace($callId)) { $callId = [string]$tags.request_id }
            if ($producer -eq "provider_lifecycle" -and -not [string]::IsNullOrWhiteSpace($callId)) {
                if (-not $providerRequests.ContainsKey($callId)) {
                    $providerRequests[$callId] = [pscustomobject]@{
                        request_id = $callId
                        logical_request_id = [string]$tags.logical_request_id
                        attempt_seq = Get-TaskspaceTimingIntValue $tags @("attempt_seq")
                        started_at_ms = $null
                        stream_opened_at_ms = $null
                        terminal_at_ms = $null
                    }
                }
                $requestState = $providerRequests[$callId]
                $startedAtMs = Get-TaskspaceTimingIntValue $tags @("started_at_ms")
                if ($null -ne $startedAtMs) { $requestState.started_at_ms = [int64]$startedAtMs }
                $createdAtMs = Get-TaskspaceTimingIntValue $payload @("createdAtMs")
                if ($status -eq "stream_opened" -and $null -ne $createdAtMs) {
                    $requestState.stream_opened_at_ms = [int64]$createdAtMs
                }
                if (@("response_completed", "response_failed", "cancelled", "failed") -contains $status) {
                    $terminalAtMs = Get-TaskspaceTimingIntValue $tags @("completed_at_ms")
                    if ($null -eq $terminalAtMs) { $terminalAtMs = $createdAtMs }
                    if ($null -ne $terminalAtMs) { $requestState.terminal_at_ms = [int64]$terminalAtMs }
                }
            }
            if (
                $producer -eq "provider_lifecycle" -and
                @("response_completed", "response_failed", "cancelled") -contains $status
            ) {
                $durationMs = Get-TaskspaceTimingIntValue $tags @("model_request_duration_ms", "latency_ms")
                if ($null -ne $durationMs) {
                    $providerRequestMs += [int64]$durationMs
                    $providerEventCount++
                }
            }
        }
        $timingMetrics = $null
        if ($eventType -eq "responsesapi.websocket_timing" -and $evt.PSObject.Properties.Name -contains "timing_metrics") {
            $timingMetrics = $evt.timing_metrics
        } elseif ($payloadType -eq "responsesapi.websocket_timing" -and $payload.PSObject.Properties.Name -contains "timing_metrics") {
            $timingMetrics = $payload.timing_metrics
        }
        if (-not $timingMetrics) { continue }
        $eventCount++
        $overhead = if ($timingMetrics.PSObject.Properties.Name -contains "responses_duration_excl_engine_and_client_tool_time_ms") { [int64]$timingMetrics.responses_duration_excl_engine_and_client_tool_time_ms } else { 0 }
        $engine = if ($timingMetrics.PSObject.Properties.Name -contains "engine_service_total_ms") { [int64]$timingMetrics.engine_service_total_ms } else { 0 }
        $requestMs += ($overhead + $engine)
    }
    $providerQueueWaitMs = [int64]0
    $providerQueueObserved = $false
    foreach ($requestState in @($providerRequests.Values)) {
        if ($null -ne $requestState.started_at_ms -and $null -ne $requestState.stream_opened_at_ms) {
            $providerQueueWaitMs += [Math]::Max([int64]0, [int64]$requestState.stream_opened_at_ms - [int64]$requestState.started_at_ms)
            $providerQueueObserved = $true
        }
    }
    $providerRetryBackoffMs = [int64]0
    $providerRetryObserved = ($providerRequests.Count -gt 0)
    $logicalGroups = @($providerRequests.Values | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.logical_request_id) -and $null -ne $_.attempt_seq } | Group-Object -Property logical_request_id)
    foreach ($group in $logicalGroups) {
        $attempts = @($group.Group | Sort-Object -Property attempt_seq)
        for ($i = 1; $i -lt $attempts.Count; $i++) {
            $previous = $attempts[$i - 1]
            $current = $attempts[$i]
            if ($null -ne $previous.terminal_at_ms -and $null -ne $current.started_at_ms) {
                $providerRetryBackoffMs += [Math]::Max([int64]0, [int64]$current.started_at_ms - [int64]$previous.terminal_at_ms)
            }
        }
    }
    if ($providerEventCount -gt 0) {
        return [pscustomobject]@{
            model_request_duration_ms = $providerRequestMs
            model_queue_wait_ms = if ($providerQueueObserved) { $providerQueueWaitMs } else { $null }
            model_retry_backoff_ms = if ($providerRetryObserved) { $providerRetryBackoffMs } else { $null }
            model_timing_event_count = $providerEventCount
            model_timing_source_status = "provider_lifecycle_timing"
            model_timing_source_path = $JsonlPath
            model_timing_parse_errors = $parseErrors
        }
    }
    [pscustomobject]@{
        model_request_duration_ms = if ($eventCount -gt 0) { $requestMs } else { $null }
        model_queue_wait_ms = $null
        model_retry_backoff_ms = if ($eventCount -gt 0) { [int64]0 } else { $null }
        model_timing_event_count = $eventCount
        model_timing_source_status = if ($eventCount -gt 0) { "responsesapi_websocket_timing" } elseif ($parseErrors -gt 0) { "jsonl_without_timing_with_parse_errors" } else { "jsonl_without_timing" }
        model_timing_source_path = $JsonlPath
        model_timing_parse_errors = $parseErrors
    }
}

function New-TaskspaceTimingDistribution {
    param([int64[]]$Values)
    [pscustomobject]@{
        count = @($Values).Count
        median_ms = Get-TaskspaceTimingPercentile $Values 50
        p95_ms = Get-TaskspaceTimingPercentile $Values 95
    }
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
        top_spans = @(Get-TaskspaceTopTimingSpans $durations 3)
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
        [string[]]$EngineeringUncleanReasons = @(),
        [string]$TaskListHash = "",
        [string]$SourceVersion = "",
        [string]$ProfileHash = ""
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
            $validationSkipped = ($validationTiming.PSObject.Properties.Name -contains "validation_skipped" -and [bool]$validationTiming.validation_skipped)
            if ($validationSkipped) {
                $spans.Add((New-TaskspaceTimingSpan "public_validation_skipped" $validationTiming.validation_started_at $validationTiming.validation_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.validation_exit_code) $false @($validationTiming.validation_skip_reason)))
                $spans.Add((New-TaskspaceTimingSpan "hidden_oracle_skipped" $validationTiming.oracle_started_at $validationTiming.oracle_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.oracle_exit_code) $false @($validationTiming.validation_skip_reason)))
            } else {
                $spans.Add((New-TaskspaceTimingSpan "public_validation" $validationTiming.validation_started_at $validationTiming.validation_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.validation_exit_code) ([int]$validationTiming.validation_exit_code -eq 124) @($validationTiming.engineering_unclean_reasons)))
                $spans.Add((New-TaskspaceTimingSpan "hidden_oracle" $validationTiming.oracle_started_at $validationTiming.oracle_finished_at $sideName ([string]$validationTiming.logical_mode) ([int]$validationTiming.oracle_exit_code) $false @()))
            }
        }
    }
    $agentMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "agent_execution" })) { $agentMs += [int64]$span.duration_ms }
    $validationMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "public_validation" })) { $validationMs += [int64]$span.duration_ms }
    $oracleMs = 0
    foreach ($span in @($spans | Where-Object { [string]$_.phase -eq "hidden_oracle" })) { $oracleMs += [int64]$span.duration_ms }
    $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0; $processLaunchWaitMs = 0; $processLaunchWaitObserved = $false; $modelRequestMs = 0; $modelRequestObserved = $false; $modelQueueWaitMs = 0; $modelQueueWaitObserved = $false; $modelRetryBackoffMs = 0; $modelRetryBackoffObserved = $false; $cacheLockWaitMs = 0; $cacheLockWaitObserved = $false
    $dockerCacheKeys = New-Object System.Collections.Generic.List[string]
    $metricValues = if ($MetricsBySide) { @($MetricsBySide.Values) } else { @() }
    foreach ($metrics in $metricValues) {
        if ($metrics.PSObject.Properties.Name -contains "process_launch_wait_ms" -and $null -ne $metrics.process_launch_wait_ms) { $processLaunchWaitMs += [int64]$metrics.process_launch_wait_ms; $processLaunchWaitObserved = $true }
        if ($metrics.PSObject.Properties.Name -contains "model_request_duration_ms" -and $null -ne $metrics.model_request_duration_ms) { $modelRequestMs += [int64]$metrics.model_request_duration_ms; $modelRequestObserved = $true }
        if ($metrics.PSObject.Properties.Name -contains "model_queue_wait_ms" -and $null -ne $metrics.model_queue_wait_ms) { $modelQueueWaitMs += [int64]$metrics.model_queue_wait_ms; $modelQueueWaitObserved = $true }
        if ($metrics.PSObject.Properties.Name -contains "model_retry_backoff_ms" -and $null -ne $metrics.model_retry_backoff_ms) { $modelRetryBackoffMs += [int64]$metrics.model_retry_backoff_ms; $modelRetryBackoffObserved = $true }
        if ($metrics.PSObject.Properties.Name -contains "docker_cache_lock_wait_ms" -and $null -ne $metrics.docker_cache_lock_wait_ms) { $cacheLockWaitMs += [int64]$metrics.docker_cache_lock_wait_ms; $cacheLockWaitObserved = $true }
        if ($metrics.PSObject.Properties.Name -contains "docker_build_duration_ms") { $dockerBuildMs += [int64]$metrics.docker_build_duration_ms }
        if ($metrics.PSObject.Properties.Name -contains "docker_run_duration_ms") { $dockerRunMs += [int64]$metrics.docker_run_duration_ms }
        if ($metrics.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { $dockerCleanupMs += [int64]$metrics.docker_cleanup_duration_ms }
        if ($metrics.PSObject.Properties.Name -contains "docker_cache_key" -and -not [string]::IsNullOrWhiteSpace([string]$metrics.docker_cache_key)) {
            $dockerCacheKeys.Add([string]$metrics.docker_cache_key)
        }
    }
    foreach ($validationTiming in @($(if ($ValidationTimingBySide) { $ValidationTimingBySide.Values } else { @() }))) {
        if ($validationTiming -and $validationTiming.PSObject.Properties.Name -contains "validation_process_launch_wait_ms" -and $null -ne $validationTiming.validation_process_launch_wait_ms) {
            $processLaunchWaitMs += [int64]$validationTiming.validation_process_launch_wait_ms
            $processLaunchWaitObserved = $true
        }
    }
    $totalDurationMs = [int64](($PairFinishedAt - $PairStartedAt).TotalMilliseconds)
    $breakdown = New-TaskspaceTimingBreakdown $totalDurationMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs $modelQueueWaitMs $EngineeringUncleanReasons
    $resourceWait = New-TaskspaceSerialResourceWaitAttribution
    $waitMissingFields = @(Get-TaskspaceRequiredWaitTimingFields | Where-Object {
            ([string]$_ -eq "model_queue_wait_ms" -and $modelQueueWaitObserved -eq $false) -or
            ([string]$_ -eq "model_retry_backoff_ms" -and $modelRetryBackoffObserved -eq $false) -or
            ([string]$_ -eq "process_launch_wait_ms" -and $processLaunchWaitObserved -eq $false) -or
            ([string]$_ -eq "model_request_duration_ms" -and $modelRequestObserved -eq $false)
        })
    $waitBlockers = @($waitMissingFields | ForEach-Object { "missing_wait_attribution:$_" })
    $artifact = [ordered]@{
        schema_version = 1
        scenario = if ($Manifest -and $Manifest.PSObject.Properties.Name -contains "Id") { [string]$Manifest.Id } else { "" }
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
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
        model_queue_wait_ms = if ($modelQueueWaitObserved) { $modelQueueWaitMs } else { $null }
        model_retry_backoff_ms = if ($modelRetryBackoffObserved) { $modelRetryBackoffMs } else { $null }
        model_request_duration_ms = if ($modelRequestObserved) { $modelRequestMs } else { $null }
        process_launch_wait_ms = if ($processLaunchWaitObserved) { $processLaunchWaitMs } else { $null }
        docker_token_wait_ms = [int64]$resourceWait.docker_token_wait_ms
        validation_token_wait_ms = [int64]$resourceWait.validation_token_wait_ms
        disk_reservation_wait_ms = [int64]$resourceWait.disk_reservation_wait_ms
        cache_lock_wait_ms = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.cache_lock_wait_ms }
        resource_wait_ms_total = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.resource_wait_ms_total }
        resource_wait_attribution_mode = if ($cacheLockWaitObserved) { "serial_with_cache_lock_observed" } else { [string]$resourceWait.resource_wait_attribution_mode }
        wait_attribution_status = if ($waitBlockers.Count -gt 0) { "missing" } else { "complete" }
        wait_attribution_missing_fields = @($waitMissingFields)
        wait_attribution_unavailable_fields = [pscustomobject]@{}
        docker_cache_keys = @($dockerCacheKeys.ToArray() | Sort-Object -Unique)
        measured_overhead_ms = $totalDurationMs - $agentMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_reason = [string]$breakdown.bottleneck_reason
        engineering_unclean_reasons = @($EngineeringUncleanReasons)
        timing_quality = "complete"
        runtime_optimization_status = "blocked"
        runtime_optimization_blockers = @($waitBlockers)
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
    $validationSkipped = ($ValidationTiming.PSObject.Properties.Name -contains "validation_skipped" -and [bool]$ValidationTiming.validation_skipped)
    $validationMs = if ($validationSkipped) { 0 } else { [int64](($ValidationTiming.validation_finished_at - $ValidationTiming.validation_started_at).TotalMilliseconds) }
    $oracleMs = if ($validationSkipped) { 0 } else { [int64](($ValidationTiming.oracle_finished_at - $ValidationTiming.oracle_started_at).TotalMilliseconds) }
    $Metrics | Add-Member -NotePropertyName public_validation_duration_ms -NotePropertyValue $validationMs -Force
    $Metrics | Add-Member -NotePropertyName hidden_oracle_duration_ms -NotePropertyValue $oracleMs -Force
    $Metrics | Add-Member -NotePropertyName public_validation_skipped -NotePropertyValue $validationSkipped -Force
    $Metrics | Add-Member -NotePropertyName public_validation_skip_reason -NotePropertyValue $(if ($validationSkipped -and $ValidationTiming.PSObject.Properties.Name -contains "validation_skip_reason") { [string]$ValidationTiming.validation_skip_reason } else { "" }) -Force
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
        [Parameter(Mandatory = $true)][string]$SampleId,
        [string]$TaskListHash = "",
        [string]$SourceVersion = "",
        [string]$ProfileHash = ""
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
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0; $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0; $processLaunchWaitMs = 0; $processLaunchWaitObserved = $false; $modelRequestMs = 0; $modelRequestObserved = $false; $modelQueueWaitMs = 0; $modelQueueWaitObserved = $false; $modelRetryBackoffMs = 0; $modelRetryBackoffObserved = $false; $cacheLockWaitMs = 0; $cacheLockWaitObserved = $false
    $bottleneckCounts = @{}
    $cacheKeyCounts = @{}
    $childRuntimeBlockers = New-Object System.Collections.Generic.List[string]
    $waitMissingFields = New-Object System.Collections.Generic.List[string]
    $phaseValues = @{
        total = New-Object System.Collections.Generic.List[int64]
        agent = New-Object System.Collections.Generic.List[int64]
        public_validation = New-Object System.Collections.Generic.List[int64]
        hidden_oracle = New-Object System.Collections.Generic.List[int64]
        docker_build = New-Object System.Collections.Generic.List[int64]
        docker_run = New-Object System.Collections.Generic.List[int64]
        docker_cleanup = New-Object System.Collections.Generic.List[int64]
    }
    foreach ($pair in $pairs) {
        $totalMs += [int64]$pair.total_duration_ms
        $agentMs += [int64]$pair.agent_duration_ms
        $validationMs += [int64]$pair.public_validation_duration_ms
        $oracleMs += [int64]$pair.hidden_oracle_duration_ms
        $overheadMs += [int64]$pair.measured_overhead_ms
        if ($pair.PSObject.Properties.Name -contains "process_launch_wait_ms" -and $null -ne $pair.process_launch_wait_ms) { $processLaunchWaitMs += [int64]$pair.process_launch_wait_ms; $processLaunchWaitObserved = $true }
        if ($pair.PSObject.Properties.Name -contains "model_request_duration_ms" -and $null -ne $pair.model_request_duration_ms) { $modelRequestMs += [int64]$pair.model_request_duration_ms; $modelRequestObserved = $true }
        if ($pair.PSObject.Properties.Name -contains "model_queue_wait_ms" -and $null -ne $pair.model_queue_wait_ms) { $modelQueueWaitMs += [int64]$pair.model_queue_wait_ms; $modelQueueWaitObserved = $true }
        if ($pair.PSObject.Properties.Name -contains "model_retry_backoff_ms" -and $null -ne $pair.model_retry_backoff_ms) { $modelRetryBackoffMs += [int64]$pair.model_retry_backoff_ms; $modelRetryBackoffObserved = $true }
        if ($pair.PSObject.Properties.Name -contains "cache_lock_wait_ms" -and $null -ne $pair.cache_lock_wait_ms) { $cacheLockWaitMs += [int64]$pair.cache_lock_wait_ms; $cacheLockWaitObserved = $true }
        $pairDockerBuildMs = if ($pair.PSObject.Properties.Name -contains "docker_build_duration_ms") { [int64]$pair.docker_build_duration_ms } else { 0 }
        $pairDockerRunMs = if ($pair.PSObject.Properties.Name -contains "docker_run_duration_ms") { [int64]$pair.docker_run_duration_ms } else { 0 }
        $pairDockerCleanupMs = if ($pair.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { [int64]$pair.docker_cleanup_duration_ms } else { 0 }
        $dockerBuildMs += $pairDockerBuildMs; $dockerRunMs += $pairDockerRunMs; $dockerCleanupMs += $pairDockerCleanupMs
        $phaseValues.total.Add([int64]$pair.total_duration_ms); $phaseValues.agent.Add([int64]$pair.agent_duration_ms)
        $phaseValues.public_validation.Add([int64]$pair.public_validation_duration_ms); $phaseValues.hidden_oracle.Add([int64]$pair.hidden_oracle_duration_ms)
        $phaseValues.docker_build.Add($pairDockerBuildMs); $phaseValues.docker_run.Add($pairDockerRunMs); $phaseValues.docker_cleanup.Add($pairDockerCleanupMs)
        $class = if ($pair.PSObject.Properties.Name -contains "bottleneck_classification") { [string]$pair.bottleneck_classification } else { "unknown" }
        if (-not $bottleneckCounts.ContainsKey($class)) { $bottleneckCounts[$class] = 0 }
        $bottleneckCounts[$class]++
        if ($pair.PSObject.Properties.Name -contains "docker_cache_keys") {
            foreach ($key in @($pair.docker_cache_keys | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })) {
                if (-not $cacheKeyCounts.ContainsKey([string]$key)) { $cacheKeyCounts[[string]$key] = 0 }
                $cacheKeyCounts[[string]$key]++
            }
        }
        foreach ($missingField in @(New-TaskspaceMissingWaitAttribution $pair)) {
            if (-not $waitMissingFields.Contains($missingField)) { $waitMissingFields.Add($missingField) }
        }
        if ($pair.PSObject.Properties.Name -contains "runtime_optimization_blockers") {
            foreach ($blocker in @($pair.runtime_optimization_blockers | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })) {
                $childRuntimeBlockers.Add("child_pair:$blocker")
            }
        }
    }
    $aggregateUncleanReasons = if ($bottleneckCounts.ContainsKey("engineering_unclean_slow")) { @("child_engineering_unclean_slow") } else { @() }
    $breakdown = New-TaskspaceTimingBreakdown $totalMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs $modelQueueWaitMs $aggregateUncleanReasons
    $resourceWait = New-TaskspaceSerialResourceWaitAttribution
    if (-not $modelQueueWaitObserved -and -not $waitMissingFields.Contains("model_queue_wait_ms")) { $waitMissingFields.Add("model_queue_wait_ms") }
    if (-not $modelRetryBackoffObserved -and -not $waitMissingFields.Contains("model_retry_backoff_ms")) { $waitMissingFields.Add("model_retry_backoff_ms") }
    $waitBlockers = @($waitMissingFields.ToArray() | ForEach-Object { "missing_wait_attribution:$_" })
    $timingBlocked = (@($missingPairTimingDirs).Count -gt 0 -or $parseErrors.Count -gt 0 -or $waitBlockers.Count -gt 0 -or $childRuntimeBlockers.Count -gt 0)
    $artifact = [ordered]@{
        schema_version = 1
        sample_id = $SampleId
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
        run_dir = $RunDir
        pair_count = @($pairs).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        model_queue_wait_ms = if ($modelQueueWaitObserved) { $modelQueueWaitMs } else { $null }
        model_retry_backoff_ms = if ($modelRetryBackoffObserved) { $modelRetryBackoffMs } else { $null }
        model_request_duration_ms = if ($modelRequestObserved) { $modelRequestMs } else { $null }
        process_launch_wait_ms = if ($processLaunchWaitObserved) { $processLaunchWaitMs } else { $null }
        docker_token_wait_ms = [int64]$resourceWait.docker_token_wait_ms
        validation_token_wait_ms = [int64]$resourceWait.validation_token_wait_ms
        disk_reservation_wait_ms = [int64]$resourceWait.disk_reservation_wait_ms
        cache_lock_wait_ms = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.cache_lock_wait_ms }
        resource_wait_ms_total = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.resource_wait_ms_total }
        resource_wait_attribution_mode = if ($cacheLockWaitObserved) { "serial_with_cache_lock_observed" } else { [string]$resourceWait.resource_wait_attribution_mode }
        wait_attribution_status = if ($waitBlockers.Count -gt 0) { "missing" } else { "complete" }
        wait_attribution_missing_fields = @($waitMissingFields.ToArray())
        wait_attribution_unavailable_fields = [pscustomobject]@{}
        measured_overhead_ms = $overheadMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_counts = Convert-TaskspaceTimingHashtable $bottleneckCounts
        phase_distributions = [ordered]@{
            total = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.total.ToArray())
            agent = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.agent.ToArray())
            public_validation = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.public_validation.ToArray())
            hidden_oracle = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.hidden_oracle.ToArray())
            docker_build = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_build.ToArray())
            docker_run = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_run.ToArray())
            docker_cleanup = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_cleanup.ToArray())
        }
        docker_cache_key_counts = Convert-TaskspaceTimingHashtable $cacheKeyCounts
        repeated_docker_cache_keys = @($cacheKeyCounts.Keys | Where-Object { [int]$cacheKeyCounts[$_] -gt 1 } | Sort-Object)
        missing_pair_timing_count = @($missingPairTimingDirs).Count
        missing_pair_timing_dirs = @($missingPairTimingDirs)
        timing_parse_error_count = $parseErrors.Count
        timing_parse_error_paths = @($parseErrors.ToArray())
        timing_quality = if ($timingBlocked) { "incomplete" } else { "complete" }
        runtime_optimization_status = if ($timingBlocked) { "blocked" } else { "ready" }
        runtime_optimization_blockers = @(@($missingPairTimingDirs | ForEach-Object { "missing_pair_timing:$_" }) + @($parseErrors.ToArray() | ForEach-Object { "malformed_pair_timing:$_" }) + $waitBlockers + @($childRuntimeBlockers.ToArray()))
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
        [Parameter(Mandatory = $true)]$SampleStatuses,
        [string]$TaskListHash = "",
        [string]$SourceVersion = "",
        [string]$ProfileHash = ""
    )
    $sampleTimingFiles = @(Get-ChildItem -LiteralPath $SuiteRoot -Filter "sample-timing.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $sampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $SuiteRoot "samples") -Directory -ErrorAction SilentlyContinue | Sort-Object FullName)
    $statusSampleDirs = @($SampleStatuses | ForEach-Object {
            if ($_.PSObject.Properties.Name -contains "sample_root" -and -not [string]::IsNullOrWhiteSpace([string]$_.sample_root)) {
                [string]$_.sample_root
            }
        })
    $expectedSampleDirs = @(@($sampleDirs | ForEach-Object { $_.FullName }) + $statusSampleDirs | Sort-Object -Unique)
    $sampleTimingPaths = @($sampleTimingFiles | ForEach-Object { [System.IO.Path]::GetFullPath($_.FullName) })
    $missingSampleTimingDirs = @($expectedSampleDirs | Where-Object {
            $sampleRoot = [System.IO.Path]::GetFullPath([string]$_).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
            $prefix = $sampleRoot + [System.IO.Path]::DirectorySeparatorChar
            @($sampleTimingPaths | Where-Object { $_ -eq (Join-Path $sampleRoot "sample-timing.json") -or $_.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase) }).Count -eq 0
        })
    $parseErrors = New-Object System.Collections.Generic.List[string]
    $samples = @()
    foreach ($file in $sampleTimingFiles) {
        try { $samples += (Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json) } catch { $parseErrors.Add($file.FullName) }
    }
    $totalMs = 0; $agentMs = 0; $validationMs = 0; $oracleMs = 0; $overheadMs = 0; $dockerBuildMs = 0; $dockerRunMs = 0; $dockerCleanupMs = 0; $processLaunchWaitMs = 0; $processLaunchWaitObserved = $false; $modelRequestMs = 0; $modelRequestObserved = $false; $modelQueueWaitMs = 0; $modelQueueWaitObserved = $false; $modelRetryBackoffMs = 0; $modelRetryBackoffObserved = $false; $cacheLockWaitMs = 0; $cacheLockWaitObserved = $false
    $bottleneckCounts = @{}
    $cacheKeyCounts = @{}
    $childRuntimeBlockers = New-Object System.Collections.Generic.List[string]
    $waitMissingFields = New-Object System.Collections.Generic.List[string]
    $phaseValues = @{
        total = New-Object System.Collections.Generic.List[int64]
        agent = New-Object System.Collections.Generic.List[int64]
        public_validation = New-Object System.Collections.Generic.List[int64]
        hidden_oracle = New-Object System.Collections.Generic.List[int64]
        docker_build = New-Object System.Collections.Generic.List[int64]
        docker_run = New-Object System.Collections.Generic.List[int64]
        docker_cleanup = New-Object System.Collections.Generic.List[int64]
    }
    foreach ($sample in $samples) {
        $totalMs += [int64]$sample.total_pair_duration_ms
        $agentMs += [int64]$sample.agent_duration_ms
        $validationMs += [int64]$sample.public_validation_duration_ms
        $oracleMs += [int64]$sample.hidden_oracle_duration_ms
        $overheadMs += [int64]$sample.measured_overhead_ms
        if ($sample.PSObject.Properties.Name -contains "process_launch_wait_ms" -and $null -ne $sample.process_launch_wait_ms) { $processLaunchWaitMs += [int64]$sample.process_launch_wait_ms; $processLaunchWaitObserved = $true }
        if ($sample.PSObject.Properties.Name -contains "model_request_duration_ms" -and $null -ne $sample.model_request_duration_ms) { $modelRequestMs += [int64]$sample.model_request_duration_ms; $modelRequestObserved = $true }
        if ($sample.PSObject.Properties.Name -contains "model_queue_wait_ms" -and $null -ne $sample.model_queue_wait_ms) { $modelQueueWaitMs += [int64]$sample.model_queue_wait_ms; $modelQueueWaitObserved = $true }
        if ($sample.PSObject.Properties.Name -contains "model_retry_backoff_ms" -and $null -ne $sample.model_retry_backoff_ms) { $modelRetryBackoffMs += [int64]$sample.model_retry_backoff_ms; $modelRetryBackoffObserved = $true }
        if ($sample.PSObject.Properties.Name -contains "cache_lock_wait_ms" -and $null -ne $sample.cache_lock_wait_ms) { $cacheLockWaitMs += [int64]$sample.cache_lock_wait_ms; $cacheLockWaitObserved = $true }
        $sampleDockerBuildMs = if ($sample.PSObject.Properties.Name -contains "docker_build_duration_ms") { [int64]$sample.docker_build_duration_ms } else { 0 }
        $sampleDockerRunMs = if ($sample.PSObject.Properties.Name -contains "docker_run_duration_ms") { [int64]$sample.docker_run_duration_ms } else { 0 }
        $sampleDockerCleanupMs = if ($sample.PSObject.Properties.Name -contains "docker_cleanup_duration_ms") { [int64]$sample.docker_cleanup_duration_ms } else { 0 }
        $dockerBuildMs += $sampleDockerBuildMs; $dockerRunMs += $sampleDockerRunMs; $dockerCleanupMs += $sampleDockerCleanupMs
        $phaseValues.total.Add([int64]$sample.total_pair_duration_ms); $phaseValues.agent.Add([int64]$sample.agent_duration_ms)
        $phaseValues.public_validation.Add([int64]$sample.public_validation_duration_ms); $phaseValues.hidden_oracle.Add([int64]$sample.hidden_oracle_duration_ms)
        $phaseValues.docker_build.Add($sampleDockerBuildMs); $phaseValues.docker_run.Add($sampleDockerRunMs); $phaseValues.docker_cleanup.Add($sampleDockerCleanupMs)
        if ($sample.PSObject.Properties.Name -contains "bottleneck_counts") {
            foreach ($prop in @($sample.bottleneck_counts.PSObject.Properties)) {
                if (-not $bottleneckCounts.ContainsKey($prop.Name)) { $bottleneckCounts[$prop.Name] = 0 }
                $bottleneckCounts[$prop.Name] += [int]$prop.Value
            }
        }
        if ($sample.PSObject.Properties.Name -contains "docker_cache_key_counts") {
            foreach ($prop in @($sample.docker_cache_key_counts.PSObject.Properties)) {
                if (-not $cacheKeyCounts.ContainsKey($prop.Name)) { $cacheKeyCounts[$prop.Name] = 0 }
                $cacheKeyCounts[$prop.Name] += [int]$prop.Value
            }
        }
        foreach ($missingField in @(New-TaskspaceMissingWaitAttribution $sample)) {
            if (-not $waitMissingFields.Contains($missingField)) { $waitMissingFields.Add($missingField) }
        }
        if ($sample.PSObject.Properties.Name -contains "runtime_optimization_blockers") {
            foreach ($blocker in @($sample.runtime_optimization_blockers | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })) {
                $childRuntimeBlockers.Add("child_sample:$blocker")
            }
        }
    }
    $aggregateUncleanReasons = if ($bottleneckCounts.ContainsKey("engineering_unclean_slow")) { @("child_engineering_unclean_slow") } else { @() }
    $breakdown = New-TaskspaceTimingBreakdown $totalMs $agentMs $validationMs $oracleMs $dockerBuildMs $dockerRunMs $dockerCleanupMs $modelQueueWaitMs $aggregateUncleanReasons
    $resourceWait = New-TaskspaceSerialResourceWaitAttribution
    if (-not $modelQueueWaitObserved -and -not $waitMissingFields.Contains("model_queue_wait_ms")) { $waitMissingFields.Add("model_queue_wait_ms") }
    if (-not $modelRetryBackoffObserved -and -not $waitMissingFields.Contains("model_retry_backoff_ms")) { $waitMissingFields.Add("model_retry_backoff_ms") }
    $waitBlockers = @($waitMissingFields.ToArray() | ForEach-Object { "missing_wait_attribution:$_" })
    $timingBlocked = (@($missingSampleTimingDirs).Count -gt 0 -or $parseErrors.Count -gt 0 -or $waitBlockers.Count -gt 0 -or $childRuntimeBlockers.Count -gt 0)
    $artifact = [ordered]@{
        schema_version = 1
        suite_root = $SuiteRoot
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
        sample_count = @($SampleStatuses).Count
        timing_sample_count = @($samples).Count
        total_pair_duration_ms = $totalMs
        agent_duration_ms = $agentMs
        public_validation_duration_ms = $validationMs
        hidden_oracle_duration_ms = $oracleMs
        docker_build_duration_ms = $dockerBuildMs
        docker_run_duration_ms = $dockerRunMs
        docker_cleanup_duration_ms = $dockerCleanupMs
        model_queue_wait_ms = if ($modelQueueWaitObserved) { $modelQueueWaitMs } else { $null }
        model_retry_backoff_ms = if ($modelRetryBackoffObserved) { $modelRetryBackoffMs } else { $null }
        model_request_duration_ms = if ($modelRequestObserved) { $modelRequestMs } else { $null }
        process_launch_wait_ms = if ($processLaunchWaitObserved) { $processLaunchWaitMs } else { $null }
        docker_token_wait_ms = [int64]$resourceWait.docker_token_wait_ms
        validation_token_wait_ms = [int64]$resourceWait.validation_token_wait_ms
        disk_reservation_wait_ms = [int64]$resourceWait.disk_reservation_wait_ms
        cache_lock_wait_ms = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.cache_lock_wait_ms }
        resource_wait_ms_total = if ($cacheLockWaitObserved) { $cacheLockWaitMs } else { [int64]$resourceWait.resource_wait_ms_total }
        resource_wait_attribution_mode = if ($cacheLockWaitObserved) { "serial_with_cache_lock_observed" } else { [string]$resourceWait.resource_wait_attribution_mode }
        wait_attribution_status = if ($waitBlockers.Count -gt 0) { "missing" } else { "complete" }
        wait_attribution_missing_fields = @($waitMissingFields.ToArray())
        wait_attribution_unavailable_fields = [pscustomobject]@{}
        measured_overhead_ms = $overheadMs
        timing_breakdown = $breakdown
        bottleneck_classification = [string]$breakdown.bottleneck_classification
        bottleneck_counts = Convert-TaskspaceTimingHashtable $bottleneckCounts
        phase_distributions = [ordered]@{
            total = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.total.ToArray())
            agent = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.agent.ToArray())
            public_validation = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.public_validation.ToArray())
            hidden_oracle = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.hidden_oracle.ToArray())
            docker_build = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_build.ToArray())
            docker_run = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_run.ToArray())
            docker_cleanup = New-TaskspaceTimingDistribution ([int64[]]$phaseValues.docker_cleanup.ToArray())
        }
        docker_cache_key_counts = Convert-TaskspaceTimingHashtable $cacheKeyCounts
        repeated_docker_cache_keys = @($cacheKeyCounts.Keys | Where-Object { [int]$cacheKeyCounts[$_] -gt 1 } | Sort-Object)
        missing_sample_timing_count = @($missingSampleTimingDirs).Count
        missing_sample_timing_dirs = @($missingSampleTimingDirs)
        timing_parse_error_count = $parseErrors.Count
        timing_parse_error_paths = @($parseErrors.ToArray())
        timing_quality = if ($timingBlocked) { "incomplete" } else { "complete" }
        runtime_optimization_status = if ($timingBlocked) { "blocked" } else { "ready" }
        runtime_optimization_blockers = @(@($missingSampleTimingDirs | ForEach-Object { "missing_sample_timing:$_" }) + @($parseErrors.ToArray() | ForEach-Object { "malformed_sample_timing:$_" }) + $waitBlockers + @($childRuntimeBlockers.ToArray()))
        sample_timing_paths = @($sampleTimingFiles | ForEach-Object { $_.FullName })
        generated_at = (Get-Date).ToString("o")
    }
    $path = Join-Path $SuiteRoot "suite-timing.json"
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $path -Encoding UTF8
    $path
}
