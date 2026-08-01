param(
    [string]$PlanPath = "",
    [string[]]$RunRoots = @(),
    [string]$OutputPath = "",
    [switch]$RequireComplete
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($PlanPath)) {
    $PlanPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-public-10-tool-stress-plan.json"
}
if (-not $RunRoots -or $RunRoots.Count -eq 0) {
    $RunRoots = @(
        "C:\WhaleRunCache\r4-public10-20260701\actual",
        "C:\WhaleRunCache\r4-public10-20260702\actual"
    )
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "target\r4-public-10-tool-stress\r4-public-10-tool-stress-report.json"
}

function Read-JsonFile {
    param([AllowEmptyString()][string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-ObjectNumber {
    param([object]$Object, [string]$Name, [double]$Default = 0)
    if ($null -eq $Object) { return $Default }
    if (-not ($Object.PSObject.Properties.Name -contains $Name)) { return $Default }
    $value = $Object.$Name
    if ($null -eq $value -or [string]::IsNullOrWhiteSpace([string]$value)) { return $Default }
    return [double]$value
}

function Get-ObjectNullableNumber {
    param([object]$Object, [string]$Name)
    if ($null -eq $Object) { return $null }
    if (-not ($Object.PSObject.Properties.Name -contains $Name)) { return $null }
    $value = $Object.$Name
    if ($null -eq $value -or [string]::IsNullOrWhiteSpace([string]$value)) { return $null }
    try { return [double]$value } catch { return $null }
}

function Get-ObjectString {
    param([object]$Object, [string]$Name, [string]$Default = "")
    if ($null -eq $Object) { return $Default }
    if (-not ($Object.PSObject.Properties.Name -contains $Name)) { return $Default }
    if ($null -eq $Object.$Name) { return $Default }
    return [string]$Object.$Name
}

function Get-Ratio {
    param([double]$Numerator, [double]$Denominator)
    if ($Denominator -le 0) { return 0 }
    return [math]::Round(($Numerator / $Denominator), 4)
}

function Get-NullableRatio {
    param($Numerator, $Denominator)
    if ($null -eq $Numerator -or $null -eq $Denominator) { return $null }
    if ([double]$Denominator -le 0) { return $null }
    return [math]::Round(([double]$Numerator / [double]$Denominator), 4)
}

function Get-StringArray {
    param([object]$Value)
    if ($null -eq $Value) { return @() }
    @($Value | ForEach-Object { [string]$_ })
}

function Get-ReportValue {
    param([string[]]$Lines, [string]$Name, [string]$Default = "")
    $pattern = "^\s*-\s*$([regex]::Escape($Name)):\s*(.+?)\s*$"
    foreach ($line in $Lines) {
        if ($line -match $pattern) { return $Matches[1].Trim() }
    }
    return $Default
}

function Get-RunStamp {
    param([string]$Path)
    if ($Path -match "\\runs\\[^\\]+\\([^\\]+)\\pair-\d+\\pair-report\.md$") {
        return $Matches[1]
    }
    return ""
}

function Find-LatestPairReport {
    param([string]$TaskId, [string[]]$Roots)
    $reports = @()
    foreach ($root in $Roots) {
        if (-not (Test-Path -LiteralPath $root -PathType Container)) { continue }
        $reports += @(Get-ChildItem -LiteralPath $root -Recurse -Filter "pair-report.md" -File -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like "*terminal_bench__$TaskId*" })
    }
    @($reports | Sort-Object LastWriteTimeUtc -Descending)[0]
}

function Get-MetricsByLogicalMode {
    param([string]$PairDir)
    $result = @{}
    foreach ($side in @("left", "right")) {
        $metricsPath = Join-Path $PairDir "$side\artifacts\metrics.json"
        $metrics = Read-JsonFile $metricsPath
        if ($null -eq $metrics) { continue }
        $logicalMode = Get-ObjectString $metrics "logical_mode" $side
        $result[$logicalMode] = [pscustomobject]@{
            side = $side
            metrics = $metrics
            artifacts_dir = Join-Path $PairDir "$side\artifacts"
            metrics_path = $metricsPath
        }
    }
    return $result
}

function Get-CacheHitRateInfo {
    param([string]$ArtifactsDir, [object]$Metrics)
    $summary = Read-JsonFile (Join-Path $ArtifactsDir "provider-cache-trace-summary.json")
    $rate = Get-ObjectNullableNumber $summary "request_2_plus_hit_rate"
    if ($null -ne $rate) {
        return [pscustomobject]@{ Rate = [math]::Round([double]$rate, 6); Availability = "measured"; Source = "provider_cache_trace_summary" }
    }
    $cached = Get-ObjectNullableNumber $Metrics "cached_input_tokens"
    $input = Get-ObjectNullableNumber $Metrics "input_tokens"
    $derived = Get-NullableRatio $cached $input
    if ($null -ne $derived) {
        return [pscustomobject]@{ Rate = $derived; Availability = "derived_from_token_summary"; Source = "metrics_token_summary" }
    }
    $availability = if ($summary) { "cache_trace_unavailable" } else { "source_missing" }
    return [pscustomobject]@{ Rate = $null; Availability = $availability; Source = "unavailable" }
}

function Get-ModelRequestCountInfo {
    param([string]$ArtifactsDir, [object]$Metrics)
    $phaseSummary = Read-JsonFile (Join-Path $ArtifactsDir "request-phase-summary.json")
    $phaseProviderDistinctCount = Get-ObjectNullableNumber $phaseSummary "provider_request_distinct_count"
    if ($null -ne $phaseProviderDistinctCount -and [double]$phaseProviderDistinctCount -gt 0) {
        return [pscustomobject]@{ Count = [int64]$phaseProviderDistinctCount; Availability = "measured"; Source = "request_phase_summary_provider_distinct" }
    }

    $requestSummary = Read-JsonFile (Join-Path $ArtifactsDir "request-summary.json")
    $rolloutCount = Get-ObjectNullableNumber $requestSummary.rollout_trace "model_request_count"
    if ($null -ne $rolloutCount -and [double]$rolloutCount -gt 0) {
        return [pscustomobject]@{ Count = [int64]$rolloutCount; Availability = "measured"; Source = "rollout_trace" }
    }

    $providerSummary = Read-JsonFile (Join-Path $ArtifactsDir "provider-cache-trace-summary.json")
    $providerCount = Get-ObjectNullableNumber $providerSummary "provider_request_count"
    if ($null -ne $providerCount -and [double]$providerCount -gt 0) {
        return [pscustomobject]@{ Count = [int64]$providerCount; Availability = "measured"; Source = "provider_cache_trace_summary" }
    }

    $summaryCount = Get-ObjectNullableNumber $requestSummary "model_request_count"
    if ($null -ne $summaryCount -and [double]$summaryCount -gt 0) {
        return [pscustomobject]@{ Count = [int64]$summaryCount; Availability = "measured"; Source = "request_summary" }
    }

    $metricsCount = Get-ObjectNullableNumber $Metrics "model_request_count"
    if ($null -ne $metricsCount -and [double]$metricsCount -gt 0) {
        return [pscustomobject]@{ Count = [int64]$metricsCount; Availability = "measured"; Source = "metrics_token_summary" }
    }

    $availability = if ($requestSummary -or $providerSummary) { "unavailable" } else { "source_missing" }
    return [pscustomobject]@{ Count = $null; Availability = $availability; Source = "unavailable" }
}

function Get-TokenSummaryAvailability {
    param([object]$Metrics)
    $availability = Get-ObjectString $Metrics "token_summary_availability" ""
    if (-not [string]::IsNullOrWhiteSpace($availability)) { return $availability }
    $input = Get-ObjectNullableNumber $Metrics "input_tokens"
    $output = Get-ObjectNullableNumber $Metrics "output_tokens"
    if ($null -ne $input -or $null -ne $output) { return "measured_legacy" }
    return "usage_unavailable"
}

function Get-UsageAccountingStatus {
    param([object]$Metrics)
    $availability = Get-TokenSummaryAvailability $Metrics
    $input = Get-ObjectNullableNumber $Metrics "input_tokens"
    $output = Get-ObjectNullableNumber $Metrics "output_tokens"
    $timedOut = ($Metrics -and $Metrics.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$Metrics.exec_timed_out)
    if ($null -ne $input -and $null -ne $output) { return "measured" }
    if ($timedOut) { return "usage_unavailable_after_timeout" }
    if ([string]$availability -eq "source_missing") { return "usage_source_missing" }
    return "usage_unavailable"
}

function Get-TokenAccountingInfo {
    param([object]$Metrics)
    $input = Get-ObjectNullableNumber $Metrics "input_tokens"
    $output = Get-ObjectNullableNumber $Metrics "output_tokens"
    $cached = Get-ObjectNullableNumber $Metrics "cached_input_tokens"
    if ($null -ne $input -and $null -ne $output) {
        return [pscustomobject]@{ Input = [int64]$input; Output = [int64]$output; Cached = if ($null -ne $cached) { [int64]$cached } else { $null }; Status = "measured" }
    }
    $rolloutInput = Get-ObjectNullableNumber $Metrics "rollout_trace_input_tokens"
    $rolloutOutput = Get-ObjectNullableNumber $Metrics "rollout_trace_output_tokens"
    $rolloutCached = Get-ObjectNullableNumber $Metrics "rollout_trace_cached_input_tokens"
    if ($null -ne $rolloutInput -and $null -ne $rolloutOutput) {
        return [pscustomobject]@{ Input = [int64]$rolloutInput; Output = [int64]$rolloutOutput; Cached = if ($null -ne $rolloutCached) { [int64]$rolloutCached } else { $null }; Status = "recovered_from_rollout_trace" }
    }
    [pscustomobject]@{ Input = $null; Output = $null; Cached = $null; Status = Get-UsageAccountingStatus $Metrics }
}

function Get-ProjectionSummary {
    param([string]$ArtifactsDir, [object]$Metrics)
    $projection = Read-JsonFile (Join-Path $ArtifactsDir "context-projection-summary.json")
    [ordered]@{
        projection_count = [int](Get-ObjectNumber $Metrics "projection_count" (Get-ObjectNumber $projection "projection_count" 0))
        protected_miss_count = [int](Get-ObjectNumber $Metrics "projection_protected_miss_count" (Get-ObjectNumber $projection "protected_miss_count" 0))
        large_output_replay_count = [int](Get-ObjectNumber $Metrics "large_output_replay_count" 0)
        rollout_scan_mode = Get-ObjectString $Metrics "rollout_scan_mode" "unknown"
    }
}

function Get-RequestReasonSummary {
    param([string]$ArtifactsDir)
    $summary = Read-JsonFile (Join-Path $ArtifactsDir "request-reason-summary.json")
    if ($null -ne $summary) {
        return [ordered]@{
            status = Get-ObjectString $summary "request_reason_coverage_status" "unavailable"
            event_count = [int](Get-ObjectNumber $summary "request_reason_event_count" 0)
            provider_request_event_count = [int](Get-ObjectNumber $summary "provider_request_event_count" 0)
            unknown_count = [int](Get-ObjectNumber $summary "request_reason_unknown_count" 0)
            attribution_coverage = [int](Get-ObjectNumber $summary "request_reason_attribution_coverage" 0)
            repeated_same_reason_no_delta_count = [int](Get-ObjectNumber $summary "repeated_same_reason_no_delta_count" 0)
            trigger_kind_counts = if ($summary.PSObject.Properties.Name -contains "trigger_kind_counts") { $summary.trigger_kind_counts } else { [ordered]@{} }
            request_reason_delta_counts = if ($summary.PSObject.Properties.Name -contains "request_reason_delta_counts") { $summary.request_reason_delta_counts } else { [ordered]@{} }
        }
    }

    $eventsPath = Join-Path $ArtifactsDir "provider-request-events.jsonl"
    if (-not (Test-Path -LiteralPath $eventsPath -PathType Leaf)) {
        return [ordered]@{
            status = "source_missing"
            event_count = 0
            provider_request_event_count = 0
            unknown_count = $null
            attribution_coverage = $null
            repeated_same_reason_no_delta_count = 0
            trigger_kind_counts = [ordered]@{}
            request_reason_delta_counts = [ordered]@{}
        }
    }

    $eventCount = 0
    $reasonCount = 0
    $unknownCount = 0
    $repeatedNoDelta = 0
    $triggerCounts = @{}
    $deltaCounts = @{}
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $eventsPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $event = $line | ConvertFrom-Json } catch { continue }
        $eventCount++
        $trigger = Get-ObjectString $event "trigger_kind" ""
        $delta = Get-ObjectString $event "request_reason_delta" ""
        $hasReason = Test-TruthValueLike (Get-ObjectString $event "request_reason_schema_present" "") -OrText $trigger
        if ($hasReason -and -not [string]::IsNullOrWhiteSpace($trigger) -and $trigger -ne "unknown") {
            $reasonCount++
            if (-not $triggerCounts.ContainsKey($trigger)) { $triggerCounts[$trigger] = 0 }
            $triggerCounts[$trigger]++
        } else {
            $unknownCount++
        }
        if (-not [string]::IsNullOrWhiteSpace($delta)) {
            if (-not $deltaCounts.ContainsKey($delta)) { $deltaCounts[$delta] = 0 }
            $deltaCounts[$delta]++
        }
        if ($delta -eq "none" -and [int](Get-ObjectNumber $event "repeated_same_reason_count" 0) -gt 0) {
            $repeatedNoDelta++
        }
    }
    $coverage = if ($eventCount -gt 0) { [int][math]::Round(([double]$reasonCount / [double]$eventCount) * 100.0) } else { 0 }
    $status = if ($eventCount -eq 0) { "missing" } elseif ($reasonCount -eq 0) { "unavailable" } elseif ($unknownCount -eq 0) { "measured" } else { "measured_with_unknown" }
    return [ordered]@{
        status = $status
        event_count = $reasonCount
        provider_request_event_count = $eventCount
        unknown_count = $unknownCount
        attribution_coverage = $coverage
        repeated_same_reason_no_delta_count = $repeatedNoDelta
        trigger_kind_counts = Convert-HashtableToOrderedObject $triggerCounts
        request_reason_delta_counts = Convert-HashtableToOrderedObject $deltaCounts
    }
}

function Test-TruthValueLike {
    param([AllowEmptyString()][string]$Value, [AllowEmptyString()][string]$OrText = "")
    if (-not [string]::IsNullOrWhiteSpace($OrText)) { return $true }
    return $Value -ieq "true"
}

function Convert-HashtableToOrderedObject {
    param([hashtable]$Table)
    $ordered = [ordered]@{}
    foreach ($key in @($Table.Keys | Sort-Object)) { $ordered[$key] = $Table[$key] }
    return [pscustomobject]$ordered
}

function Get-ChangedPaths {
    param([object]$Metrics)
    $inventory = @($Metrics.changed_file_inventory)
    if ($inventory.Count -gt 0) {
        return @($inventory | ForEach-Object { [string]$_.path } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    }
    Get-StringArray $Metrics.changed_paths
}

function Get-ObservabilityToolCallStats {
    param([string]$ArtifactsDir, [object]$Metrics)
    $metricCount = Get-ObjectNumber $Metrics "observability_tool_call_count" -1
    $metricSource = Get-ObjectString $Metrics "observability_tool_call_availability" "metrics_field"
    if ($metricCount -ge 0) {
        return [pscustomobject]@{ Count = [int]$metricCount; Source = $metricSource }
    }

    $obsPath = Get-ObjectString $Metrics "observability_json" ""
    if ([string]::IsNullOrWhiteSpace($obsPath)) {
        $obsPath = Join-Path $ArtifactsDir "observability\action-map-observability.json"
    }
    $obs = Read-JsonFile $obsPath
    if ($null -eq $obs) {
        return [pscustomobject]@{ Count = 0; Source = "missing" }
    }

    $resultCount = 0
    foreach ($node in @($obs.nodes)) {
        foreach ($result in @($node.results)) {
            if ([string]$result.kind -eq "main_tool_call") {
                $resultCount += 1
            }
        }
    }
    if ($resultCount -gt 0) {
        return [pscustomobject]@{ Count = $resultCount; Source = "observability_results" }
    }

    $runtimeCounts = $obs.summary.runtimeEventCounts
    if ($null -eq $runtimeCounts) {
        return [pscustomobject]@{ Count = 0; Source = "unavailable" }
    }
    $count = [int]((Get-ObjectNumber $runtimeCounts "function_call" 0) + (Get-ObjectNumber $runtimeCounts "custom_tool_call" 0))
    return [pscustomobject]@{ Count = $count; Source = "observability_runtime_counts" }
}

function Get-EffectiveToolCallStats {
    param([string]$ArtifactsDir, [object]$Metrics)
    $metricsCount = Get-ObjectNumber $Metrics "tool_call_count" 0
    $rolloutCount = Get-ObjectNumber $Metrics "rollout_tool_call_count" 0
    $observability = Get-ObservabilityToolCallStats $ArtifactsDir $Metrics
    $count = [int]([math]::Max($metricsCount, [math]::Max($rolloutCount, [double]$observability.Count)))
    $source = "metrics"
    if ($metricsCount -le 0 -and [double]$observability.Count -gt 0) {
        $source = [string]$observability.Source
    }
    if ($rolloutCount -eq $count -and $rolloutCount -gt $metricsCount) {
        $source = "rollout"
    }
    if ([double]$observability.Count -eq $count -and [double]$observability.Count -gt [math]::Max($metricsCount, $rolloutCount)) {
        $source = [string]$observability.Source
    }
    return [pscustomobject]@{ Count = $count; Source = $source }
}

function New-MissingRow {
    param([object]$Plan, [object]$Sample)
    [ordered]@{
        public_benchmark = [string]$Plan.public_source.benchmark
        benchmark_version = [string]$Plan.public_source.version
        source_commit = [string]$Plan.public_source.commit
        task_id = [string]$Sample.task_id
        task_id_registry_verified = $true
        run_status = "missing"
        standard_outcome = "missing_run"
        taskspace_outcome = "missing_run"
        standard_wall_time_ms = 0
        taskspace_wall_time_ms = 0
        taskspace_wall_time_ratio = 0
        standard_tool_calls = 0
        taskspace_tool_calls = 0
        taskspace_tool_call_ratio = 0
        standard_input_tokens = $null
        standard_output_tokens = $null
        taskspace_input_tokens = $null
        taskspace_output_tokens = $null
        taskspace_token_ratio = $null
        standard_model_request_count = $null
        taskspace_model_request_count = $null
        taskspace_model_request_ratio = $null
        standard_model_request_count_source = "missing_run"
        taskspace_model_request_count_source = "missing_run"
        model_request_count_availability = "missing_run"
        standard_token_summary_availability = "missing_run"
        taskspace_token_summary_availability = "missing_run"
        standard_usage_accounting_status = "missing_run"
        taskspace_usage_accounting_status = "missing_run"
        token_ratio_availability = "missing_run"
        request_2_plus_cache_hit_rate = $null
        request_2_plus_cache_hit_rate_availability = "missing_run"
        request_reason_coverage_status = "missing_run"
        request_reason_event_count = 0
        request_reason_unknown_count = $null
        request_reason_attribution_coverage = $null
        repeated_same_reason_no_delta_count = 0
        request_reason_trigger_kind_counts = [ordered]@{}
        request_reason_delta_counts = [ordered]@{}
        tool_feedback_loss_count = 0
        tool_feedback_semantic_loss_count = 0
        tool_result_projection_count_by_reason = [ordered]@{}
        taskspace_map_attribution_missing_count = 1
        large_output_ref_count = 0
        rollout_size_bytes = 0
        changed_paths_standard = @()
        changed_paths_taskspace = @()
        validation_result = [ordered]@{ status = "missing_run" }
        failure_taxonomy = "missing_run"
        tool_call_analysis_summary = "No paired run artifact found under configured R4 public-10 run roots."
        evidence_paths = @()
    }
}

function New-RunRow {
    param([object]$Plan, [object]$Sample, [object]$PairReport)
    $pairDir = Split-Path -Parent $PairReport.FullName
    $modes = Get-MetricsByLogicalMode $pairDir
    $standard = $modes["standard"]
    $taskspace = $modes["taskspace"]
    if ($null -eq $standard -or $null -eq $taskspace) {
        $row = New-MissingRow $Plan $Sample
        $row.run_status = "invalid_pair_artifacts"
        $row.failure_taxonomy = "invalid_pair_artifacts"
        $row.evidence_paths = @($PairReport.FullName)
        return $row
    }

    $reportLines = Get-Content -Encoding UTF8 -LiteralPath $PairReport.FullName
    $standardMetrics = $standard.metrics
    $taskspaceMetrics = $taskspace.metrics
    $standardTokenInfo = Get-TokenAccountingInfo $standardMetrics
    $taskspaceTokenInfo = Get-TokenAccountingInfo $taskspaceMetrics
    $standardInputTokens = $standardTokenInfo.Input
    $standardOutputTokens = $standardTokenInfo.Output
    $taskspaceInputTokens = $taskspaceTokenInfo.Input
    $taskspaceOutputTokens = $taskspaceTokenInfo.Output
    $standardTokens = if ($null -ne $standardInputTokens -and $null -ne $standardOutputTokens) { [double]$standardInputTokens + [double]$standardOutputTokens } else { $null }
    $taskspaceTokens = if ($null -ne $taskspaceInputTokens -and $null -ne $taskspaceOutputTokens) { [double]$taskspaceInputTokens + [double]$taskspaceOutputTokens } else { $null }
    $tokenRatio = Get-NullableRatio $taskspaceTokens $standardTokens
    $tokenRatioAvailability = if ($null -ne $tokenRatio -and ([string]$standardTokenInfo.Status -eq "measured" -and [string]$taskspaceTokenInfo.Status -eq "measured")) {
        "measured"
    } elseif ($null -ne $tokenRatio -and ([string]$standardTokenInfo.Status -eq "recovered_from_rollout_trace" -or [string]$taskspaceTokenInfo.Status -eq "recovered_from_rollout_trace")) {
        "recovered_from_rollout_trace"
    } else {
        "unavailable"
    }
    $cacheHitInfo = Get-CacheHitRateInfo $taskspace.artifacts_dir $taskspaceMetrics
    $standardRequestInfo = Get-ModelRequestCountInfo $standard.artifacts_dir $standardMetrics
    $taskspaceRequestInfo = Get-ModelRequestCountInfo $taskspace.artifacts_dir $taskspaceMetrics
    $requestRatio = Get-NullableRatio $taskspaceRequestInfo.Count $standardRequestInfo.Count
    $requestAvailability = if ($null -ne $requestRatio) { "measured" } else { "unavailable" }
    $taxonomy = Get-ReportValue $reportLines "failure_taxonomy" "none"
    $graph = Read-JsonFile (Join-Path $taskspace.artifacts_dir "graph-health.json")
    $projection = Get-ProjectionSummary $taskspace.artifacts_dir $taskspaceMetrics
    $requestReason = Get-RequestReasonSummary $taskspace.artifacts_dir
    $mapMissing = if ($null -eq $graph) { 1 } else { [int](Get-ObjectNumber $graph "attribution_missing_count" 0) }
    $feedbackLoss = if ($taxonomy -match "tool_feedback_loss") { 1 } else { 0 }
    $semanticLoss = if ($taxonomy -match "semantic_loss|tool_feedback_semantic_loss") { 1 } else { 0 }
    $standardCallStats = Get-EffectiveToolCallStats $standard.artifacts_dir $standardMetrics
    $taskspaceCallStats = Get-EffectiveToolCallStats $taskspace.artifacts_dir $taskspaceMetrics
    $standardCalls = [int]$standardCallStats.Count
    $taskspaceCalls = [int]$taskspaceCallStats.Count
    $standardWall = Get-ObjectNumber $standardMetrics "wall_time_ms" 0
    $taskspaceWall = Get-ObjectNumber $taskspaceMetrics "wall_time_ms" 0
    $summary = "standard=$($standardCalls) tool calls; taskspace=$($taskspaceCalls) tool calls; " +
        "standard_model_requests=$($standardRequestInfo.Count); taskspace_model_requests=$($taskspaceRequestInfo.Count); " +
        "standard_tool_source=$($standardCallStats.Source); taskspace_tool_source=$($taskspaceCallStats.Source); " +
        "projection_count=$($projection.projection_count); protected_miss=$($projection.protected_miss_count); " +
        "feedback_loss=$feedbackLoss; semantic_loss=$semanticLoss."

    [ordered]@{
        public_benchmark = [string]$Plan.public_source.benchmark
        benchmark_version = [string]$Plan.public_source.version
        source_commit = [string]$Plan.public_source.commit
        task_id = [string]$Sample.task_id
        task_id_registry_verified = $true
        run_status = "found"
        run_stamp = Get-RunStamp $PairReport.FullName
        standard_outcome = Get-ReportValue $reportLines "outcome_standard" "unknown"
        taskspace_outcome = Get-ReportValue $reportLines "outcome_taskspace" "unknown"
        standard_wall_time_ms = [int64]$standardWall
        taskspace_wall_time_ms = [int64]$taskspaceWall
        taskspace_wall_time_ratio = Get-Ratio $taskspaceWall $standardWall
        standard_tool_calls = [int]$standardCalls
        taskspace_tool_calls = [int]$taskspaceCalls
        taskspace_tool_call_ratio = Get-Ratio $taskspaceCalls $standardCalls
        standard_input_tokens = if ($null -ne $standardInputTokens) { [int64]$standardInputTokens } else { $null }
        standard_output_tokens = if ($null -ne $standardOutputTokens) { [int64]$standardOutputTokens } else { $null }
        taskspace_input_tokens = if ($null -ne $taskspaceInputTokens) { [int64]$taskspaceInputTokens } else { $null }
        taskspace_output_tokens = if ($null -ne $taskspaceOutputTokens) { [int64]$taskspaceOutputTokens } else { $null }
        taskspace_token_ratio = $tokenRatio
        standard_model_request_count = $standardRequestInfo.Count
        taskspace_model_request_count = $taskspaceRequestInfo.Count
        taskspace_model_request_ratio = $requestRatio
        standard_model_request_count_source = [string]$standardRequestInfo.Source
        taskspace_model_request_count_source = [string]$taskspaceRequestInfo.Source
        model_request_count_availability = $requestAvailability
        standard_token_summary_availability = Get-TokenSummaryAvailability $standardMetrics
        taskspace_token_summary_availability = Get-TokenSummaryAvailability $taskspaceMetrics
        standard_usage_accounting_status = [string]$standardTokenInfo.Status
        taskspace_usage_accounting_status = [string]$taskspaceTokenInfo.Status
        token_ratio_availability = $tokenRatioAvailability
        request_2_plus_cache_hit_rate = $cacheHitInfo.Rate
        request_2_plus_cache_hit_rate_availability = [string]$cacheHitInfo.Availability
        request_reason_coverage_status = [string]$requestReason.status
        request_reason_event_count = [int]$requestReason.event_count
        request_reason_unknown_count = $requestReason.unknown_count
        request_reason_attribution_coverage = $requestReason.attribution_coverage
        repeated_same_reason_no_delta_count = [int]$requestReason.repeated_same_reason_no_delta_count
        request_reason_trigger_kind_counts = $requestReason.trigger_kind_counts
        request_reason_delta_counts = $requestReason.request_reason_delta_counts
        tool_feedback_loss_count = $feedbackLoss
        tool_feedback_semantic_loss_count = $semanticLoss
        tool_result_projection_count_by_reason = $projection
        taskspace_map_attribution_missing_count = $mapMissing
        large_output_ref_count = [int](Get-ObjectNumber $taskspaceMetrics "runtime_output_ref_created_count" 0)
        rollout_size_bytes = [int64](Get-ObjectNumber $taskspaceMetrics "rollout_bytes" 0)
        changed_paths_standard = @(Get-ChangedPaths $standardMetrics)
        changed_paths_taskspace = @(Get-ChangedPaths $taskspaceMetrics)
        validation_result = [ordered]@{
            standard_public_exit_code = [int](Get-ObjectNumber $standardMetrics "public_validation_exit_code" -1)
            taskspace_public_exit_code = [int](Get-ObjectNumber $taskspaceMetrics "public_validation_exit_code" -1)
            standard_hidden_oracle_exit_code = [int](Get-ObjectNumber $standardMetrics "hidden_oracle_exit_code" -1)
            taskspace_hidden_oracle_exit_code = [int](Get-ObjectNumber $taskspaceMetrics "hidden_oracle_exit_code" -1)
            standard_exec_timed_out = [bool]$standardMetrics.exec_timed_out
            taskspace_exec_timed_out = [bool]$taskspaceMetrics.exec_timed_out
        }
        failure_taxonomy = $taxonomy
        tool_call_analysis_summary = $summary
        evidence_paths = @(
            $PairReport.FullName,
            $standard.metrics_path,
            $taskspace.metrics_path,
            (Join-Path $taskspace.artifacts_dir "provider-cache-trace-summary.json"),
            (Join-Path $taskspace.artifacts_dir "request-reason-summary.json"),
            (Join-Path $taskspace.artifacts_dir "context-projection-summary.json"),
            (Join-Path $taskspace.artifacts_dir "graph-health.json")
        ) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
    }
}

$plan = Read-JsonFile $PlanPath
if ($null -eq $plan) { throw "plan not found: $PlanPath" }
$rows = @()
foreach ($sample in @($plan.samples)) {
    $taskId = [string]$sample.task_id
    $pairReport = Find-LatestPairReport $taskId $RunRoots
    if ($null -eq $pairReport) {
        $rows += [pscustomobject](New-MissingRow $plan $sample)
    } else {
        $rows += [pscustomobject](New-RunRow $plan $sample $pairReport)
    }
}

$completeRows = @($rows | Where-Object { [string]$_.run_status -eq "found" })
$missingRows = @($rows | Where-Object { [string]$_.run_status -ne "found" })
$report = [ordered]@{
    schema_version = 1
    artifact = "r4-public-10-tool-stress-report"
    generated_at = (Get-Date).ToString("o")
    plan_path = [System.IO.Path]::GetFullPath($PlanPath)
    run_roots = @($RunRoots)
    summary = [ordered]@{
        row_count = @($rows).Count
        complete_run_count = @($completeRows).Count
        missing_run_count = @($missingRows).Count
        missing_task_ids = @($missingRows | ForEach-Object { [string]$_.task_id })
    }
    rows = @($rows)
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null
[pscustomobject]$report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

if ($RequireComplete -and @($missingRows).Count -gt 0) {
    $missingIds = (@($missingRows | ForEach-Object { [string]$_.task_id }) -join ", ")
    Write-Error "R4 public-10 report incomplete: $(@($completeRows).Count)/$(@($rows).Count) found; missing: $missingIds"
    exit 1
}

Write-Host "R4 public-10 report written: $OutputPath"
Write-Host "complete_run_count=$(@($completeRows).Count) missing_run_count=$(@($missingRows).Count)"
