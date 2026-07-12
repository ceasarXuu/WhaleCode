Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "native-cadence.ps1")
. (Join-Path $PSScriptRoot "performance-duplication.ps1")
function Get-PerformanceProperty {
    param($Object, [Parameter(Mandatory = $true)][string]$Name, $Default = $null)
    if ($null -ne $Object) {
        $property = $Object.PSObject.Properties[$Name]
        if ($null -ne $property) { return $property.Value }
    }
    $Default
}
function Read-PerformanceJson {
    param([string]$Path, [System.Collections.Generic.List[object]]$Events)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $null }
    try { Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json }
    catch {
        if ($Events) {
            $Events.Add([pscustomobject]@{ event = "artifact_parse_failed"; path = $Path; error = [string]$_.Exception.Message })
        }
        $null
    }
}
function Get-PerformanceNumber {
    param($Value)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $null }
    try { [double]$Value } catch { $null }
}
function Get-PerformanceRatio {
    param($Numerator, $Denominator)
    $num = Get-PerformanceNumber $Numerator
    $den = Get-PerformanceNumber $Denominator
    if ($null -eq $num -or $null -eq $den -or $den -eq 0) { return $null }
    [Math]::Round($num / $den, 4)
}

function Get-PerformanceWireRequestCount {
    param([string]$ArtifactDir, $CacheSummary)
    $summaryCount = Get-PerformanceNumber (Get-PerformanceProperty $CacheSummary "provider_request_count")
    if ($null -ne $summaryCount) {
        return [pscustomobject]@{ value = $summaryCount; source = "provider_cache_trace_summary" }
    }
    $wirePath = Join-Path $ArtifactDir "provider-wire-trace.jsonl"
    if (Test-Path -LiteralPath $wirePath) {
        $count = 0
        foreach ($line in [System.IO.File]::ReadLines($wirePath)) {
            if ($line -match '"event_name"\s*:\s*"provider\.chat_wire_shape_recorded"') { $count++ }
        }
        if ($count -gt 0) { return [pscustomobject]@{ value = [double]$count; source = "provider_wire_trace" } }
    }
    [pscustomobject]@{ value = $null; source = "unavailable" }
}
function Get-PerformanceActionCounts {
    param([string]$ArtifactDir, [System.Collections.Generic.List[object]]$Events)
    $shell = 0; $patch = 0; $control = 0; $other = 0
    $providerOuterCalls = 0; $nestedActions = 0
    $rolloutPath = Join-Path $ArtifactDir "rollout.jsonl"
    if (Test-Path -LiteralPath $rolloutPath) {
        foreach ($line in [System.IO.File]::ReadLines($rolloutPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try {
                $row = $line | ConvertFrom-Json
                $payload = Get-TaskspaceCanonicalResponseItem $row
                if ($null -eq $payload) { continue }
                if ([string](Get-PerformanceProperty $payload "type") -notin @("function_call", "custom_tool_call")) { continue }
                $providerOuterCalls++
                $name = [string](Get-PerformanceProperty $payload "name")
                switch ($name) {
                    "exec_command" { $shell++ }
                    "apply_patch" { $patch++ }
                    "taskspace_control" { $control++ }
                    default { $other++ }
                }
                if ($name -ne "taskspace_control") { continue }
                try {
                    $arguments = ([string](Get-PerformanceProperty $payload "arguments")) | ConvertFrom-Json
                    foreach ($nested in @((Get-PerformanceProperty $arguments "actions" @()))) {
                        $nestedActions++
                        switch ([string](Get-PerformanceProperty $nested "tool_name")) {
                            "exec_command" { $shell++ }
                            "apply_patch" { $patch++ }
                            default { $other++ }
                        }
                    }
                } catch {
                    if ($Events) {
                        $Events.Add([pscustomobject]@{ event = "control_arguments_parse_failed"; path = $rolloutPath; error = [string]$_.Exception.Message })
                    }
                }
            } catch {
                if ($Events) {
                    $Events.Add([pscustomobject]@{ event = "rollout_line_parse_failed"; path = $rolloutPath; error = [string]$_.Exception.Message })
                }
            }
        }
        return [pscustomobject]@{
            shell = $shell; patch = $patch; control = $control; other = $other
            provider_outer_tool_calls = $providerOuterCalls; nested_action_count = $nestedActions
            source = "rollout"
        }
    }
    $execPath = Join-Path $ArtifactDir "whale-exec.jsonl"
    if (Test-Path -LiteralPath $execPath) {
        foreach ($line in [System.IO.File]::ReadLines($execPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try {
                $row = $line | ConvertFrom-Json
                if ([string](Get-PerformanceProperty $row "type") -ne "item.completed") { continue }
                $item = Get-PerformanceProperty $row "item"
                switch ([string](Get-PerformanceProperty $item "type")) {
                    "command_execution" { $shell++ }
                    "file_change" { $patch++ }
                }
            } catch {
                if ($Events) {
                    $Events.Add([pscustomobject]@{ event = "exec_line_parse_failed"; path = $execPath; error = [string]$_.Exception.Message })
                }
            }
        }
        return [pscustomobject]@{
            shell = $shell; patch = $patch; control = 0; other = 0
            provider_outer_tool_calls = $null; nested_action_count = 0; source = "whale_exec"
        }
    }
    [pscustomobject]@{
        shell = $null; patch = $null; control = $null; other = $null
        provider_outer_tool_calls = $null; nested_action_count = $null; source = "unavailable"
    }
}
function Get-PerformanceMapFacts {
    param([string]$ArtifactDir, $Metrics, [System.Collections.Generic.List[string]]$Warnings)
    $graph = Read-PerformanceJson (Join-Path $ArtifactDir "graph-health.json")
    $managed = Read-PerformanceJson (Join-Path $ArtifactDir "map-management-summary.json")
    $control = Read-PerformanceJson (Join-Path $ArtifactDir "taskspace-control-usage.json")
    $observability = Read-PerformanceJson (Join-Path $ArtifactDir "observability/action-map-observability.json")
    $nodes = @()
    $edges = @()
    $taskStatus = ""
    if ($observability) {
        $nodes = @((Get-PerformanceProperty $observability "nodes" @()) | ForEach-Object {
                [pscustomobject]@{
                    id = [string](Get-PerformanceProperty $_ "id")
                    title = [string](Get-PerformanceProperty $_ "title")
                    kind = [string](Get-PerformanceProperty $_ "kind")
                    status = [string](Get-PerformanceProperty $_ "status")
                    result_count = @((Get-PerformanceProperty $_ "results" @())).Count
                }
            } | Sort-Object id)
        $edges = @((Get-PerformanceProperty $observability "edges" @()) | ForEach-Object {
                [pscustomobject]@{ from = [string](Get-PerformanceProperty $_ "from"); to = [string](Get-PerformanceProperty $_ "to") }
            })
        $tasks = @((Get-PerformanceProperty $observability "tasks" @()))
        if ($tasks.Count -gt 0) { $taskStatus = [string](Get-PerformanceProperty $tasks[0] "status") }
    }
    $mapCount = Get-PerformanceNumber (Get-PerformanceProperty $Metrics "maps")
    $nodeCount = Get-PerformanceNumber (Get-PerformanceProperty $graph "node_count" (Get-PerformanceProperty $Metrics "nodes"))
    $edgeCount = Get-PerformanceNumber (Get-PerformanceProperty $graph "edge_count" (Get-PerformanceProperty $Metrics "edges"))
    $openLeaves = Get-PerformanceNumber (Get-PerformanceProperty $Metrics "open_leaf_nodes")
    $resultCount = Get-PerformanceNumber (Get-PerformanceProperty $graph "result_count")
    $unreviewed = Get-PerformanceNumber (Get-PerformanceProperty $graph "unreviewed_result_count")
    if ($mapCount -gt 0 -and $nodeCount -gt 1 -and $edgeCount -eq 0) { $Warnings.Add("multi_node_map_without_edges") }
    if ($mapCount -gt 0 -and $openLeaves -eq 0 -and $taskStatus -eq "active") { $Warnings.Add("root_task_active_after_nodes_closed") }
    if ($unreviewed -gt 0) { $Warnings.Add("unreviewed_results_present") }
    [pscustomobject]@{
        map_count = $mapCount
        node_count = $nodeCount
        edge_count = $edgeCount
        result_count = $resultCount
        open_leaf_nodes = $openLeaves
        root_task_status = $taskStatus
        accepted_result_count = Get-PerformanceNumber (Get-PerformanceProperty $graph "accepted_result_count")
        unreviewed_result_count = $unreviewed
        retention_coverage_ratio = Get-PerformanceNumber (Get-PerformanceProperty $managed "retention_coverage_ratio")
        salience_coverage_ratio = Get-PerformanceNumber (Get-PerformanceProperty $managed "salience_coverage_ratio")
        semantic_replacement_rate = Get-PerformanceNumber (Get-PerformanceProperty $managed "semantic_replacement_rate")
        protected_miss_count = Get-PerformanceNumber (Get-PerformanceProperty $managed "protected_miss_count")
        compaction_event_count = Get-PerformanceNumber (Get-PerformanceProperty $managed "compaction_event_count")
        control_count = Get-PerformanceNumber (Get-PerformanceProperty $control "taskspace_control_count")
        control_failure_count = Get-PerformanceNumber (Get-PerformanceProperty $control "control_failure_count")
        control_protocol_failure_count = Get-PerformanceNumber (Get-PerformanceProperty $control "control_protocol_failure_count")
        control_state_failure_count = Get-PerformanceNumber (Get-PerformanceProperty $control "control_state_failure_count")
        nested_action_failure_count = Get-PerformanceNumber (Get-PerformanceProperty $control "nested_action_failure_count")
        control_actions = Get-PerformanceProperty $control "action_counts" ([pscustomobject]@{})
        runtime_event_count = Get-PerformanceNumber (Get-PerformanceProperty $control "taskspace_runtime_event_count")
        snapshot_update_count = Get-PerformanceNumber (Get-PerformanceProperty (Get-PerformanceProperty $control "runtime_event_counts") "snapshot_updated")
        nodes = $nodes
        edges = $edges
    }
}
function Get-PerformanceSideObservation {
    param([string]$MetricPath, [string]$ObservationRoot, [System.Collections.Generic.List[object]]$Events)
    $artifactDir = Split-Path -Parent $MetricPath
    $sideDir = Split-Path -Parent $artifactDir
    $pairDir = Split-Path -Parent $sideDir
    $side = Split-Path -Leaf $sideDir
    $pair = Split-Path -Leaf $pairDir
    $caseId = [System.IO.Path]::GetRelativePath($ObservationRoot, $pairDir).Replace("\", "/")
    $metrics = Read-PerformanceJson $MetricPath $Events
    if (-not $metrics) { return $null }
    $modeMap = Read-PerformanceJson (Join-Path $pairDir "logical-mode-map.json") $Events
    $mode = if ($modeMap -and $modeMap.PSObject.Properties.Name -contains $side) { [string]$modeMap.$side } else { [string](Get-PerformanceProperty $metrics "logical_mode" "unknown") }
    $repeat = if ($modeMap) { Get-PerformanceNumber (Get-PerformanceProperty $modeMap "repeat") } elseif ($pair -match '(\d+)$') { [double]$Matches[1] } else { $null }
    $cache = Read-PerformanceJson (Join-Path $artifactDir "provider-cache-trace-summary.json") $Events
    $requests = Get-PerformanceWireRequestCount $artifactDir $cache
    $actions = Get-PerformanceActionCounts $artifactDir $Events
    $cadence = Get-TaskspaceNativeCadenceFacts $artifactDir $Events
    $warnings = New-Object System.Collections.Generic.List[string]
    $taints = @((Get-PerformanceProperty $metrics "metrics_taints" @()))
    $skipped = @($taints | Where-Object { [string]$_ -match '^side_selection_skipped:' }).Count -gt 0
    if ($skipped) {
        $warnings.Add("side_not_run")
    } else {
        if ($null -eq $requests.value) { $warnings.Add("provider_request_count_unavailable") }
        if (-not $cache) { $warnings.Add("provider_cache_trace_unavailable") }
        if ([string](Get-PerformanceProperty $metrics "external_validation_status") -eq "failed") { $warnings.Add("external_validation_failed") }
    }
    $map = Get-PerformanceMapFacts $artifactDir $metrics $warnings
    $duplication = Get-PerformanceDuplicationFacts $artifactDir $Events
    $input = Get-PerformanceNumber (Get-PerformanceProperty $metrics "input_tokens")
    $cached = Get-PerformanceNumber (Get-PerformanceProperty $metrics "cached_input_tokens")
    $agentStatus = [string](Get-PerformanceProperty $metrics "agent_completion_status")
    $observationStatus = if ($skipped) { "skipped" } elseif ($agentStatus -eq "complete") { "complete" } else { "incomplete" }
    $comparisonEligible = -not $skipped -and $agentStatus -eq "complete" -and $null -ne $requests.value -and $input -gt 0
    [pscustomobject]@{
        case_id = $caseId; pair = $pair; repeat = $repeat; side = $side; logical_mode = $mode
        artifact_dir = $artifactDir
        observation_status = $observationStatus
        comparison_eligible = $comparisonEligible
        result = [pscustomobject]@{
            business_success = Get-PerformanceProperty $metrics "business_success"
            agent_completion_status = $agentStatus
            public_validation_exit_code = Get-PerformanceNumber (Get-PerformanceProperty $metrics "public_validation_exit_code")
            hidden_oracle_exit_code = Get-PerformanceNumber (Get-PerformanceProperty $metrics "hidden_oracle_exit_code")
            external_validation_status = [string](Get-PerformanceProperty $metrics "external_validation_status")
            changed_paths = @((Get-PerformanceProperty $metrics "changed_paths" @()))
        }
        actions = [pscustomobject]@{
            provider_requests = $requests.value; provider_request_source = $requests.source
            ordinary_tools = Get-PerformanceNumber (Get-PerformanceProperty $metrics "tool_call_count")
            failed_tools = Get-PerformanceNumber (Get-PerformanceProperty $metrics "failed_tool_call_count")
            provider_outer_tool_calls = $actions.provider_outer_tool_calls
            nested_actions = $actions.nested_action_count
            shell = $actions.shell; patch = $actions.patch; taskspace_control = $map.control_count
            control_failures = $map.control_failure_count
            control_protocol_failures = $map.control_protocol_failure_count
            control_state_failures = $map.control_state_failure_count
            nested_action_failures = $map.nested_action_failure_count
            provider_tool_responses = $cadence.provider_tool_response_count
            control_carrier_responses = $cadence.control_carrier_response_count
            direct_tool_mixed_responses = $cadence.direct_tool_mixed_response_count
            multi_control_carrier_responses = $cadence.multi_control_carrier_response_count
            multi_finish_carriers = $cadence.multi_finish_carrier_count
            finish_without_sibling_actions = $cadence.finish_without_sibling_action_count
            initialize_then_actions = $cadence.initialize_then_actions_count
            finish_nodes = $cadence.finish_nodes_count
            finish_then_end = $cadence.finish_then_end_count
            terminal_candidates = $cadence.terminal_candidate_count
            terminal_extra_requests = $cadence.terminal_extra_request_count
            cadence_source = $cadence.availability
            action_trace_source = $actions.source
        }
        cost = [pscustomobject]@{
            wall_time_ms = Get-PerformanceNumber (Get-PerformanceProperty $metrics "wall_time_ms")
            input_tokens = $input; cached_input_tokens = $cached
            uncached_input_tokens = Get-PerformanceNumber (Get-PerformanceProperty $metrics "uncached_input_tokens")
            output_tokens = Get-PerformanceNumber (Get-PerformanceProperty $metrics "output_tokens")
            full_cache_hit_rate = Get-PerformanceRatio $cached $input
            monetary_cost = $null; monetary_cost_status = "unavailable_no_unit_price_artifact"
        }
        cache = [pscustomobject]@{
            request_2_plus_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "request_2_plus_count")
            request_2_plus_cached_input_tokens = Get-PerformanceNumber (Get-PerformanceProperty $cache "request_2_plus_cached_input_tokens")
            request_2_plus_uncached_input_tokens = Get-PerformanceNumber (Get-PerformanceProperty $cache "request_2_plus_uncached_input_tokens")
            request_2_plus_hit_rate = Get-PerformanceNumber (Get-PerformanceProperty $cache "request_2_plus_hit_rate")
            prefix_comparison_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "prefix_comparison_count")
            prefix_preserved_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "prefix_preserved_count")
            prefix_preserved_rate = Get-PerformanceNumber (Get-PerformanceProperty $cache "prefix_preserved_rate")
            zero_cache_hit_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "zero_cache_hit_count")
            cache_warmup_candidate_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "cache_warmup_candidate_count")
            same_shape_zero_hit_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "same_shape_zero_hit_count")
            tool_choice_transition_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "tool_choice_transition_count")
            cache_shape_transition_count = Get-PerformanceNumber (Get-PerformanceProperty $cache "cache_shape_transition_count")
            trace_coverage = Get-PerformanceNumber (Get-PerformanceProperty $cache "trace_coverage")
        }
        map = $map
        duplication = $duplication
        warnings = @($warnings.ToArray())
    }
}

function Get-PerformanceModeAggregate {
    param([object[]]$Rows, [string]$Mode)
    $observed = @($Rows | Where-Object { $_.logical_mode -eq $Mode })
    $selected = @($observed | Where-Object { $_.comparison_eligible })
    if ($selected.Count -eq 0) { return $null }
    $sum = [ordered]@{}
    foreach ($field in @("provider_requests", "ordinary_tools", "failed_tools", "provider_outer_tool_calls", "nested_actions", "taskspace_control", "control_failures", "control_protocol_failures", "control_state_failures", "nested_action_failures", "provider_tool_responses", "control_carrier_responses", "direct_tool_mixed_responses", "multi_control_carrier_responses", "multi_finish_carriers", "finish_without_sibling_actions", "initialize_then_actions", "finish_nodes", "finish_then_end", "terminal_candidates", "terminal_extra_requests")) {
        $values = @($selected | ForEach-Object { Get-PerformanceNumber $_.actions.$field } | Where-Object { $null -ne $_ })
        $sum[$field] = if ($values.Count) { [double](($values | Measure-Object -Sum).Sum) } else { $null }
    }
    foreach ($field in @("wall_time_ms", "input_tokens", "cached_input_tokens", "uncached_input_tokens", "output_tokens")) {
        $values = @($selected | ForEach-Object { Get-PerformanceNumber $_.cost.$field } | Where-Object { $null -ne $_ })
        $sum[$field] = if ($values.Count) { [double](($values | Measure-Object -Sum).Sum) } else { $null }
    }
    foreach ($field in @("map_count", "node_count", "edge_count", "result_count", "open_leaf_nodes", "unreviewed_result_count", "runtime_event_count")) {
        $values = @($selected | ForEach-Object { Get-PerformanceNumber $_.map.$field } | Where-Object { $null -ne $_ })
        $sum[$field] = if ($values.Count) { [double](($values | Measure-Object -Sum).Sum) } else { $null }
    }
    $cache2 = [double](($selected | ForEach-Object { Get-PerformanceNumber $_.cache.request_2_plus_cached_input_tokens } | Where-Object { $null -ne $_ } | Measure-Object -Sum).Sum)
    $uncached2 = [double](($selected | ForEach-Object { Get-PerformanceNumber $_.cache.request_2_plus_uncached_input_tokens } | Where-Object { $null -ne $_ } | Measure-Object -Sum).Sum)
    $prefixCount = [double](($selected | ForEach-Object { Get-PerformanceNumber $_.cache.prefix_comparison_count } | Where-Object { $null -ne $_ } | Measure-Object -Sum).Sum)
    $prefixKept = [double](($selected | ForEach-Object { Get-PerformanceNumber $_.cache.prefix_preserved_count } | Where-Object { $null -ne $_ } | Measure-Object -Sum).Sum)
    [pscustomobject]@{
        logical_mode = $Mode; side_count = $selected.Count; observed_side_count = $observed.Count
        excluded_side_count = $observed.Count - $selected.Count
        solved_count = @($selected | Where-Object { $_.result.business_success -eq $true }).Count
        agent_complete_count = @($selected | Where-Object { $_.result.agent_completion_status -eq "complete" }).Count
        totals = [pscustomobject]$sum
        full_cache_hit_rate = Get-PerformanceRatio $sum.cached_input_tokens $sum.input_tokens
        request_2_plus_hit_rate = Get-PerformanceRatio $cache2 ($cache2 + $uncached2)
        prefix_preserved_rate = Get-PerformanceRatio $prefixKept $prefixCount
        prefix_preserved_count = $prefixKept; prefix_comparison_count = $prefixCount
    }
}

function Format-PerformanceValue {
    param($Value, [string]$Kind = "number")
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return "N/A" }
    if ($Kind -eq "percent") { return ("{0:N2}%" -f (([double]$Value) * 100)) }
    if ($Kind -eq "seconds") { return ("{0:N2}s" -f (([double]$Value) / 1000)) }
    if ($Kind -eq "ratio") { return ("{0:N2}x" -f ([double]$Value) ) }
    if ($Value -is [bool]) { return ([string]$Value).ToLowerInvariant() }
    if ($Value -is [ValueType]) { return ("{0:N0}" -f ([double]$Value)) }
    ([string]$Value).Replace("|", "\|").Replace("`r", " ").Replace("`n", " ")
}

function Format-PerformanceControlActions {
    param($Actions)
    if ($null -eq $Actions) { return "N/A" }
    $parts = @($Actions.PSObject.Properties | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" })
    if ($parts.Count -eq 0) { return "N/A" }
    ($parts -join ", ").Replace("|", "\|")
}

function Write-TaskspacePerformanceObservation {
    param(
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [string]$OutputDirectory = "",
        [string]$ReportBaseName = "performance-observation"
    )
    if (-not (Test-Path -LiteralPath $RunRoot)) { throw "RunRoot does not exist: $RunRoot" }
    $root = (Resolve-Path -LiteralPath $RunRoot).Path
    if ([string]::IsNullOrWhiteSpace($OutputDirectory)) { $OutputDirectory = $root }
    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $events = New-Object System.Collections.Generic.List[object]
    $events.Add([pscustomobject]@{ event = "observation_started"; run_root = $root; at = (Get-Date).ToString("o") })
    $metricFiles = @(Get-ChildItem -LiteralPath $root -Filter "metrics.json" -Recurse -File | Where-Object {
            $_.FullName -match '[\\/]pair-\d+[\\/](left|right)[\\/]artifacts[\\/]metrics\.json$'
        } | Sort-Object FullName)
    if ($metricFiles.Count -eq 0) { throw "No pair side metrics.json found under RunRoot: $root" }
    $rows = @($metricFiles | ForEach-Object { Get-PerformanceSideObservation $_.FullName $root $events } | Where-Object { $null -ne $_ } | Sort-Object case_id, logical_mode)
    $aggregates = @("standard", "taskspace", "r4" | ForEach-Object { Get-PerformanceModeAggregate $rows $_ } | Where-Object { $null -ne $_ })
    $standard = @($aggregates | Where-Object { $_.logical_mode -eq "standard" } | Select-Object -First 1)
    $taskspace = @($aggregates | Where-Object { $_.logical_mode -eq "taskspace" } | Select-Object -First 1)
    $ratios = [ordered]@{}
    if ($standard.Count -and $taskspace.Count) {
        foreach ($field in @("provider_requests", "ordinary_tools", "wall_time_ms", "input_tokens", "cached_input_tokens", "uncached_input_tokens", "output_tokens")) {
            $ratios[$field] = Get-PerformanceRatio $taskspace[0].totals.$field $standard[0].totals.$field
        }
        $ratios["request_2_plus_cache_hit_delta"] = if ($null -ne $taskspace[0].request_2_plus_hit_rate -and $null -ne $standard[0].request_2_plus_hit_rate) {
            [Math]::Round($taskspace[0].request_2_plus_hit_rate - $standard[0].request_2_plus_hit_rate, 4)
        } else { $null }
    }
    foreach ($row in $rows) {
        foreach ($warning in @($row.warnings)) {
            $events.Add([pscustomobject]@{ event = "observation_warning"; case_id = $row.case_id; pair = $row.pair; side = $row.side; logical_mode = $row.logical_mode; code = $warning })
        }
    }
    $report = [pscustomobject]@{
        schema_version = "taskspace-performance-observation-v1"
        generated_at = (Get-Date).ToString("o")
        run_root = $root
        monetary_cost_status = "unavailable_no_unit_price_artifact"
        rows = $rows
        aggregates = $aggregates
        ratios = [pscustomobject]$ratios
    }
    $jsonPath = Join-Path $OutputDirectory "$ReportBaseName.json"
    $mdPath = Join-Path $OutputDirectory "$ReportBaseName.md"
    $eventPath = Join-Path $OutputDirectory "$ReportBaseName-events.jsonl"
    $report | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace 通用性能观察")
    $lines.Add("")
    $lines.Add("- Run root: ``$root``")
    $lines.Add("- 货币成本: N/A（artifact 未冻结 provider 单价）")
    $lines.Add("")
    $lines.Add("## 结果与动作")
    $lines.Add("")
    $lines.Add("| Case | Mode | Run | Result | Agent | External | Public | Hidden | Changed | Requests | Runtime tools | Provider calls | Nested actions | Failed | Shell | Patch | Controls | Wall |")
    $lines.Add("|---|---|---|---|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $rows) {
        $result = if ($row.observation_status -eq "skipped") { "N/A" } elseif ($row.result.business_success -eq $true) { "solved" } elseif ($row.result.business_success -eq $false) { "not-solved" } else { "N/A" }
        $changed = if (@($row.result.changed_paths).Count) { @($row.result.changed_paths) -join ", " } else { "none" }
        if ($row.observation_status -eq "skipped") {
            $lines.Add("| $(Format-PerformanceValue $row.case_id) | $($row.logical_mode) | skipped | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.case_id) | $($row.logical_mode) | $($row.observation_status) | $result | $(Format-PerformanceValue $row.result.agent_completion_status) | $(Format-PerformanceValue $row.result.external_validation_status) | $(Format-PerformanceValue $row.result.public_validation_exit_code) | $(Format-PerformanceValue $row.result.hidden_oracle_exit_code) | $(Format-PerformanceValue $changed) | $(Format-PerformanceValue $row.actions.provider_requests) | $(Format-PerformanceValue $row.actions.ordinary_tools) | $(Format-PerformanceValue $row.actions.provider_outer_tool_calls) | $(Format-PerformanceValue $row.actions.nested_actions) | $(Format-PerformanceValue $row.actions.failed_tools) | $(Format-PerformanceValue $row.actions.shell) | $(Format-PerformanceValue $row.actions.patch) | $(Format-PerformanceValue $row.actions.taskspace_control) | $(Format-PerformanceValue $row.cost.wall_time_ms seconds) |")
        }
    }
    $lines.Add("")
    $lines.Add("## Schema carrier")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Tool responses | Control responses | Nested init actions | Init+actions | Finish barriers | Finish+end | Multi-finish | Direct mixed | Multi-control | Finish without sibling | Protocol failures | State failures | Nested failures | Terminal candidates | Extra final requests | Source |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    foreach ($row in $rows) {
        if ($row.observation_status -eq "skipped") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.actions.provider_tool_responses) | $(Format-PerformanceValue $row.actions.control_carrier_responses) | $(Format-PerformanceValue $row.actions.nested_actions) | $(Format-PerformanceValue $row.actions.initialize_then_actions) | $(Format-PerformanceValue $row.actions.finish_nodes) | $(Format-PerformanceValue $row.actions.finish_then_end) | $(Format-PerformanceValue $row.actions.multi_finish_carriers) | $(Format-PerformanceValue $row.actions.direct_tool_mixed_responses) | $(Format-PerformanceValue $row.actions.multi_control_carrier_responses) | $(Format-PerformanceValue $row.actions.finish_without_sibling_actions) | $(Format-PerformanceValue $row.actions.control_protocol_failures) | $(Format-PerformanceValue $row.actions.control_state_failures) | $(Format-PerformanceValue $row.actions.nested_action_failures) | $(Format-PerformanceValue $row.actions.terminal_candidates) | $(Format-PerformanceValue $row.actions.terminal_extra_requests) | $(Format-PerformanceValue $row.actions.cadence_source) |")
        }
    }
    $lines.Add("")
    $lines.Add("## 成本与缓存")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Input | Cached | Uncached | Output | Full hit | Request 2+ hit | Prefix | Zero hit | Warmup | Same-shape zero | Choice changes | Shape changes | Coverage |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $rows) {
        if ($row.observation_status -eq "skipped") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $prefix = if ($null -ne $row.cache.prefix_comparison_count) { "$(Format-PerformanceValue $row.cache.prefix_preserved_count)/$(Format-PerformanceValue $row.cache.prefix_comparison_count)" } else { "N/A" }
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.cost.input_tokens) | $(Format-PerformanceValue $row.cost.cached_input_tokens) | $(Format-PerformanceValue $row.cost.uncached_input_tokens) | $(Format-PerformanceValue $row.cost.output_tokens) | $(Format-PerformanceValue $row.cost.full_cache_hit_rate percent) | $(Format-PerformanceValue $row.cache.request_2_plus_hit_rate percent) | $prefix | $(Format-PerformanceValue $row.cache.zero_cache_hit_count) | $(Format-PerformanceValue $row.cache.cache_warmup_candidate_count) | $(Format-PerformanceValue $row.cache.same_shape_zero_hit_count) | $(Format-PerformanceValue $row.cache.tool_choice_transition_count) | $(Format-PerformanceValue $row.cache.cache_shape_transition_count) | $(Format-PerformanceValue $row.cache.trace_coverage percent) |")
        }
    }
    Add-PerformanceDuplicationMarkdown $lines $rows
    $lines.Add("")
    $lines.Add("## Map")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Maps | Nodes | Edges | Results | Open | Task | Accepted | Unreviewed | Req/node | Tools/node | Controls | Control actions |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---|")
    foreach ($row in $rows) {
        if ($row.observation_status -eq "skipped") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $reqPerNode = Get-PerformanceRatio $row.actions.provider_requests $row.map.node_count
            $toolsPerNode = Get-PerformanceRatio $row.actions.ordinary_tools $row.map.node_count
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.map.map_count) | $(Format-PerformanceValue $row.map.node_count) | $(Format-PerformanceValue $row.map.edge_count) | $(Format-PerformanceValue $row.map.result_count) | $(Format-PerformanceValue $row.map.open_leaf_nodes) | $(Format-PerformanceValue $row.map.root_task_status) | $(Format-PerformanceValue $row.map.accepted_result_count) | $(Format-PerformanceValue $row.map.unreviewed_result_count) | $(Format-PerformanceValue $reqPerNode) | $(Format-PerformanceValue $toolsPerNode) | $(Format-PerformanceValue $row.map.control_count) | $(Format-PerformanceControlActions $row.map.control_actions) |")
        }
    }
    $lines.Add("")
    $lines.Add("## Map 语义保存")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Retention | Salience | Semantic replace | Protected miss | Compaction | Runtime events | Snapshot updates |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $rows) {
        if ($row.observation_status -eq "skipped") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.map.retention_coverage_ratio percent) | $(Format-PerformanceValue $row.map.salience_coverage_ratio percent) | $(Format-PerformanceValue $row.map.semantic_replacement_rate percent) | $(Format-PerformanceValue $row.map.protected_miss_count) | $(Format-PerformanceValue $row.map.compaction_event_count) | $(Format-PerformanceValue $row.map.runtime_event_count) | $(Format-PerformanceValue $row.map.snapshot_update_count) |")
        }
    }
    $lines.Add("")
    $lines.Add("## Map 节点")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Node | Kind | Status | Results | Dependencies |")
    $lines.Add("|---:|---|---|---|---|---:|---|")
    foreach ($row in $rows) {
        foreach ($node in @($row.map.nodes)) {
            $incoming = @($row.map.edges | Where-Object { $_.to -eq $node.id } | ForEach-Object { $_.from }) -join ", "
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $node.title) | $($node.kind) | $($node.status) | $(Format-PerformanceValue $node.result_count) | $(if ($incoming) { $incoming } else { "none" }) |")
        }
    }
    $lines.Add("")
    $lines.Add("## 聚合")
    $lines.Add("")
    $lines.Add("| Mode | Solved | Requests | Tools | Wall | Input | Cached | Uncached | Output | Request 2+ hit | Prefix | Nodes | Edges |")
    $lines.Add("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($aggregate in $aggregates) {
        $lines.Add("| $($aggregate.logical_mode) | $($aggregate.solved_count)/$($aggregate.side_count) | $(Format-PerformanceValue $aggregate.totals.provider_requests) | $(Format-PerformanceValue $aggregate.totals.ordinary_tools) | $(Format-PerformanceValue $aggregate.totals.wall_time_ms seconds) | $(Format-PerformanceValue $aggregate.totals.input_tokens) | $(Format-PerformanceValue $aggregate.totals.cached_input_tokens) | $(Format-PerformanceValue $aggregate.totals.uncached_input_tokens) | $(Format-PerformanceValue $aggregate.totals.output_tokens) | $(Format-PerformanceValue $aggregate.request_2_plus_hit_rate percent) | $(Format-PerformanceValue $aggregate.prefix_preserved_rate percent) | $(Format-PerformanceValue $aggregate.totals.node_count) | $(Format-PerformanceValue $aggregate.totals.edge_count) |")
    }
    if ($ratios.Count -gt 0) {
        $lines.Add("")
        $lines.Add("## TaskSpace / Standard")
        $lines.Add("")
        $lines.Add("| Requests | Tools | Wall | Input | Cached | Uncached | Output | Request 2+ cache delta |")
        $lines.Add("|---:|---:|---:|---:|---:|---:|---:|---:|")
        $lines.Add("| $(Format-PerformanceValue $ratios.provider_requests ratio) | $(Format-PerformanceValue $ratios.ordinary_tools ratio) | $(Format-PerformanceValue $ratios.wall_time_ms ratio) | $(Format-PerformanceValue $ratios.input_tokens ratio) | $(Format-PerformanceValue $ratios.cached_input_tokens ratio) | $(Format-PerformanceValue $ratios.uncached_input_tokens ratio) | $(Format-PerformanceValue $ratios.output_tokens ratio) | $(Format-PerformanceValue $ratios.request_2_plus_cache_hit_delta percent) |")
    }
    $warningRows = @($rows | Where-Object { @($_.warnings).Count -gt 0 })
    if ($warningRows.Count -gt 0) {
        $lines.Add("")
        $lines.Add("## 机械观察")
        $lines.Add("")
        foreach ($row in $warningRows) { $lines.Add("- $($row.case_id)/$($row.side) [$($row.logical_mode)]: $(@($row.warnings) -join ', ')") }
    }
    $lines | Set-Content -LiteralPath $mdPath -Encoding UTF8
    $events.Add([pscustomobject]@{ event = "observation_completed"; row_count = $rows.Count; json_path = $jsonPath; markdown_path = $mdPath; at = (Get-Date).ToString("o") })
    @($events.ToArray()) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 12 } | Set-Content -LiteralPath $eventPath -Encoding UTF8
    [pscustomobject]@{ json_path = $jsonPath; markdown_path = $mdPath; event_log_path = $eventPath; report = $report }
}
