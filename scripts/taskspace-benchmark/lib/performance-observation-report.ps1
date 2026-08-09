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
    $invalidModeMapRows = @($rows | Where-Object { @($_.warnings) -contains "logical_mode_map_invalid" })
    $comparisonScopeValid = $invalidModeMapRows.Count -eq 0
    if (-not $comparisonScopeValid) {
        $events.Add([pscustomobject]@{
                event = "performance_comparison_scope_invalid"
                code = "logical_mode_map_invalid"
                invalid_side_count = $invalidModeMapRows.Count
            })
    }
    $aggregates = @()
    if ($comparisonScopeValid) {
        $aggregates = @("standard", "taskspace", "r4" | ForEach-Object { Get-PerformanceModeAggregate $rows $_ } | Where-Object { $null -ne $_ })
    }
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
    foreach ($row in $invalidModeMapRows) {
        foreach ($sectionName in @("result", "actions", "cost", "cache", "section_cost", "map", "patch", "duplication")) {
            $section = $row.$sectionName
            if ($null -eq $section) { continue }
            foreach ($property in @($section.PSObject.Properties)) { $property.Value = $null }
        }
    }
    $report = [pscustomobject]@{
        schema_version = "taskspace-performance-observation-v1"
        generated_at = (Get-Date).ToString("o")
        run_root = $root
        monetary_cost_status = "unavailable_no_unit_price_artifact"
        comparison_scope_status = if ($comparisonScopeValid) { "measured" } else { "unavailable" }
        comparison_scope_findings = if ($comparisonScopeValid) { @() } else { @("logical_mode_map_invalid") }
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
        $result = if ($row.observation_status -in @("skipped", "invalid")) { "N/A" } elseif ($row.result.business_success -eq $true) { "solved" } elseif ($row.result.business_success -eq $false) { "not-solved" } else { "N/A" }
        $changed = if (@($row.result.changed_paths).Count) { @($row.result.changed_paths) -join ", " } else { "none" }
        if ($row.observation_status -in @("skipped", "invalid")) {
            $lines.Add("| $(Format-PerformanceValue $row.case_id) | $($row.logical_mode) | $($row.observation_status) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.case_id) | $($row.logical_mode) | $($row.observation_status) | $result | $(Format-PerformanceValue $row.result.agent_completion_status) | $(Format-PerformanceValue $row.result.external_validation_status) | $(Format-PerformanceValue $row.result.public_validation_exit_code) | $(Format-PerformanceValue $row.result.hidden_oracle_exit_code) | $(Format-PerformanceValue $changed) | $(Format-PerformanceValue $row.actions.provider_requests) | $(Format-PerformanceValue $row.actions.ordinary_tools) | $(Format-PerformanceValue $row.actions.provider_outer_tool_calls) | $(Format-PerformanceValue $row.actions.nested_actions) | $(Format-PerformanceValue $row.actions.failed_tools) | $(Format-PerformanceValue $row.actions.shell) | $(Format-PerformanceValue $row.actions.patch) | $(Format-PerformanceValue $row.actions.taskspace_control) | $(Format-PerformanceValue $row.cost.wall_time_ms seconds) |")
        }
    }
    $lines.Add("")
    $lines.Add("## TaskSpace sequence")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Tool responses | Control responses | Mixed responses | Multi-control | Manifests | Paired | Violations | Orphan siblings | Declared actions | Owned siblings | Init pairs | Execute pairs | Reopen pairs | Finish Map | Final Work close | Standalone control | Protocol failures | State failures | Parse errors | Source |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    foreach ($row in $rows) {
        if ($row.observation_status -in @("skipped", "invalid") -or
            $row.actions.action_protocol -eq "taskspace_exec") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.actions.provider_tool_responses) | $(Format-PerformanceValue $row.actions.control_responses) | $(Format-PerformanceValue $row.actions.mixed_control_action_responses) | $(Format-PerformanceValue $row.actions.multi_control_responses) | $(Format-PerformanceValue $row.actions.action_manifest_count) | $(Format-PerformanceValue $row.actions.action_manifest_pairs) | $(Format-PerformanceValue $row.actions.action_manifest_violations) | $(Format-PerformanceValue $row.actions.orphan_siblings) | $(Format-PerformanceValue $row.actions.cadence_declared_actions) | $(Format-PerformanceValue $row.actions.cadence_owned_siblings) | $(Format-PerformanceValue $row.actions.initialize_and_execute_pairs) | $(Format-PerformanceValue $row.actions.execute_pairs) | $(Format-PerformanceValue $row.actions.reopen_pairs) | $(Format-PerformanceValue $row.actions.finish_maps) | $(Format-PerformanceValue $row.actions.finish_map_final_work) | $(Format-PerformanceValue $row.actions.standalone_control_responses) | $(Format-PerformanceValue $row.actions.control_protocol_failures) | $(Format-PerformanceValue $row.actions.control_state_failures) | $(Format-PerformanceValue $row.actions.cadence_parse_errors) | $(Format-PerformanceValue $row.actions.cadence_source) |")
        }
    }
    $lines.Add("")
    $lines.Add("## TaskSpace Exec")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Capability | Exec | Map ops | Client | Hosted | Node bindings | Client results | Hosted results | Failed | Trace events | Request links | Outer links | Status |")
    $lines.Add("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    foreach ($row in $rows) {
        if ($row.observation_status -in @("skipped", "invalid") -or
            $row.actions.action_protocol -ne "taskspace_exec") {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.actions.capability_identity) | $(Format-PerformanceValue $row.actions.taskspace_exec) | $(Format-PerformanceValue $row.actions.map_operations) | $(Format-PerformanceValue $row.actions.client_actions) | $(Format-PerformanceValue $row.actions.hosted_bindings) | $(Format-PerformanceValue $row.actions.node_bindings) | $(Format-PerformanceValue $row.actions.client_results) | $(Format-PerformanceValue $row.actions.hosted_results) | $(Format-PerformanceValue $row.actions.failed_tools) | $(Format-PerformanceValue $row.actions.exec_trace_events) | $(Format-PerformanceValue $row.actions.correlated_requests) | $(Format-PerformanceValue $row.actions.correlated_outer_calls) | $(Format-PerformanceValue $row.actions.exec_observation_status) |")
        }
    }
    $lines.Add("")
    $validDisplayRows = @($rows | Where-Object { $_.observation_status -ne "invalid" })
    Add-TaskspacePatchObservationMarkdown $lines $validDisplayRows
    $lines.Add("## 成本与缓存")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Input | Cached | Uncached | Output | Full hit | Request 2+ hit | Prefix | Zero hit | Warmup | Same-shape zero | Choice changes | Shape changes | Coverage |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $rows) {
        if ($row.observation_status -in @("skipped", "invalid")) {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $prefix = if ($null -ne $row.cache.prefix_comparison_count) { "$(Format-PerformanceValue $row.cache.prefix_preserved_count)/$(Format-PerformanceValue $row.cache.prefix_comparison_count)" } else { "N/A" }
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.cost.input_tokens) | $(Format-PerformanceValue $row.cost.cached_input_tokens) | $(Format-PerformanceValue $row.cost.uncached_input_tokens) | $(Format-PerformanceValue $row.cost.output_tokens) | $(Format-PerformanceValue $row.cost.full_cache_hit_rate percent) | $(Format-PerformanceValue $row.cache.request_2_plus_hit_rate percent) | $prefix | $(Format-PerformanceValue $row.cache.zero_cache_hit_count) | $(Format-PerformanceValue $row.cache.cache_warmup_candidate_count) | $(Format-PerformanceValue $row.cache.same_shape_zero_hit_count) | $(Format-PerformanceValue $row.cache.tool_choice_transition_count) | $(Format-PerformanceValue $row.cache.cache_shape_transition_count) | $(Format-PerformanceValue $row.cache.trace_coverage percent) |")
        }
    }
    Add-PerformanceSectionCostMarkdown $lines $validDisplayRows $aggregates
    Add-PerformanceDuplicationMarkdown $lines $validDisplayRows
    $lines.Add("")
    $lines.Add("## Map")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Maps | Nodes | Edges | Results | Open | Task | Accepted | Unreviewed | Req/node | Tools/node | Controls | Control actions |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---|")
    foreach ($row in $rows) {
        if ($row.observation_status -in @("skipped", "invalid")) {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $reqPerNode = Get-PerformanceRatio $row.actions.provider_requests $row.map.node_count
            $toolsPerNode = Get-PerformanceRatio $row.actions.ordinary_tools $row.map.node_count
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.map.map_count) | $(Format-PerformanceValue $row.map.node_count) | $(Format-PerformanceValue $row.map.edge_count) | $(Format-PerformanceValue $row.map.result_count) | $(Format-PerformanceValue $row.map.open_leaf_nodes) | $(Format-PerformanceValue $row.map.root_task_status) | $(Format-PerformanceValue $row.map.accepted_result_count) | $(Format-PerformanceValue $row.map.unreviewed_result_count) | $(Format-PerformanceValue $reqPerNode) | $(Format-PerformanceValue $toolsPerNode) | $(Format-PerformanceValue $row.map.control_count) | $(Format-PerformanceControlActions $row.map.control_actions) |")
        }
    }
    $lines.Add("")
    $lines.Add("## Map 显式读取")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Requests | Completed | Failed | Repeated revision | Lag samples | Lag mean | Lag max | Stale errors |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $rows) {
        if ($row.observation_status -in @("skipped", "invalid")) {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.map.read_map_request_count) | $(Format-PerformanceValue $row.map.read_map_completion_count) | $(Format-PerformanceValue $row.map.read_map_failure_count) | $(Format-PerformanceValue $row.map.read_map_repeated_revision_count) | $(Format-PerformanceValue $row.map.read_map_revision_lag_sample_count) | $(Format-PerformanceValue $row.map.read_map_revision_lag_mean) | $(Format-PerformanceValue $row.map.read_map_revision_lag_max) | $(Format-PerformanceValue $row.map.read_map_stale_revision_error_count) |")
        }
    }
    $lines.Add("")
    $lines.Add("## Map 节点")
    $lines.Add("")
    $lines.Add("| Repeat | Mode | Node | Kind | Status | Results | Dependencies |")
    $lines.Add("|---:|---|---|---|---|---:|---|")
    foreach ($row in @($rows | Where-Object { $_.observation_status -ne "invalid" })) {
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
