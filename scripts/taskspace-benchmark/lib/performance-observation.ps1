Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "native-cadence.ps1")
. (Join-Path $PSScriptRoot "patch-observability.ps1")
. (Join-Path $PSScriptRoot "performance-duplication.ps1")
. (Join-Path $PSScriptRoot "performance-section-cost.ps1")
. (Join-Path $PSScriptRoot "r7-integer-facts.ps1")
. (Join-Path $PSScriptRoot "performance-token-identity.ps1")
. (Join-Path $PSScriptRoot "performance-count-identity.ps1")
. (Join-Path $PSScriptRoot "logical-mode-map.ps1")
. (Join-Path $PSScriptRoot "taskspace-exec-observation.ps1")
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
        if ($null -ne $Events) {
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
function Get-PerformanceCount {
    param($Value)
    ConvertTo-R7NonnegativeInt64Fact $Value
}
function Get-PerformanceRatio {
    param($Numerator, $Denominator)
    $num = Get-PerformanceNumber $Numerator
    $den = Get-PerformanceNumber $Denominator
    if ($null -eq $num -or $null -eq $den -or $den -eq 0) { return $null }
    [Math]::Round($num / $den, 4)
}
function Get-PerformanceWireRequestCount {
    param([string]$ArtifactDir)
    $facts = Read-PerformanceJson (Join-Path $ArtifactDir "request-facts.json")
    if ($facts -and [string]$facts.schema_version -eq "whalecode-request-facts-v1") {
        $boundaryMeasured = [string]$facts.availability.boundary -eq "measured"
        $completionMeasured = [string]$facts.availability.completion -eq "measured"
        $usageMeasured = [string]$facts.availability.usage -eq "measured"
        $usage = Get-PerformanceProperty (Get-PerformanceProperty $facts "summary") "usage"
        $value = if ($boundaryMeasured) { Get-PerformanceCount $facts.summary.boundary_request_count } else { $null }
        return [pscustomobject]@{
            value = $value
            source = if ($boundaryMeasured) { "request_facts_boundary" } else { "request_facts_unavailable" }
            logical = Get-PerformanceCount $facts.summary.logical_request_count
            attempts = Get-PerformanceCount $facts.summary.local_attempt_count
            boundary = if ($boundaryMeasured) { Get-PerformanceCount $facts.summary.boundary_request_count } else { $null }
            completed = if ($completionMeasured) { Get-PerformanceCount $facts.summary.completed_response_count } else { $null }
            failed_or_cancelled = Get-PerformanceCount $facts.summary.failed_or_cancelled_attempt_count
            usage_measured = $usageMeasured
            input_tokens = if ($usageMeasured) { Get-PerformanceCount (Get-PerformanceProperty $usage "input_tokens") } else { $null }
            cached_input_tokens = if ($usageMeasured) { Get-PerformanceCount (Get-PerformanceProperty $usage "cached_input_tokens") } else { $null }
            uncached_input_tokens = if ($usageMeasured) { Get-PerformanceCount (Get-PerformanceProperty $usage "uncached_input_tokens") } else { $null }
            output_tokens = if ($usageMeasured) { Get-PerformanceCount (Get-PerformanceProperty $usage "output_tokens") } else { $null }
        }
    }
    [pscustomobject]@{
        value = $null; source = "unavailable"; logical = $null; attempts = $null
        boundary = $null; completed = $null; failed_or_cancelled = $null
        usage_measured = $false; input_tokens = $null; cached_input_tokens = $null
        uncached_input_tokens = $null; output_tokens = $null
    }
}
function Get-PerformanceMapFacts {
    param([string]$ArtifactDir, $Metrics, [System.Collections.Generic.List[string]]$Warnings)
    $graph = Read-PerformanceJson (Join-Path $ArtifactDir "graph-health.json")
    $control = Read-PerformanceJson (Join-Path $ArtifactDir "taskspace-control-usage.json")
    $observability = Read-PerformanceJson (Join-Path $ArtifactDir "observability/action-map-observability.json")
    $nodes = @()
    $edges = @()
    $taskStatus = ""
    $mapStoreAvailability = ""
    if ($observability) {
        $source = Get-PerformanceProperty $observability "source"
        $mapStore = Get-PerformanceProperty $source "mapStore"
        $mapStoreAvailability = [string](Get-PerformanceProperty $mapStore "availability" "")
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
    $mapCount = if ($mapStoreAvailability -eq "measured") {
        [int64]@((Get-PerformanceProperty $observability "maps" @())).Count
    } else {
        Get-PerformanceCount (Get-PerformanceProperty $Metrics "maps")
    }
    $nodeCount = if ($mapStoreAvailability -eq "measured") {
        [int64]$nodes.Count
    } else {
        Get-PerformanceCount (Get-PerformanceProperty $graph "node_count" (Get-PerformanceProperty $Metrics "nodes"))
    }
    $edgeCount = if ($mapStoreAvailability -eq "measured") {
        [int64]$edges.Count
    } else {
        Get-PerformanceCount (Get-PerformanceProperty $graph "edge_count" (Get-PerformanceProperty $Metrics "edges"))
    }
    $openLeaves = Get-PerformanceCount (Get-PerformanceProperty $Metrics "open_leaf_nodes")
    $resultCount = Get-PerformanceCount (Get-PerformanceProperty $graph "result_count")
    $unreviewed = Get-PerformanceCount (Get-PerformanceProperty $graph "unreviewed_result_count")
    if ($mapCount -gt 0 -and $nodeCount -gt 1 -and $edgeCount -eq 0) { $Warnings.Add("multi_node_map_without_edges") }
    if ($mapCount -gt 0 -and $openLeaves -eq 0 -and $taskStatus -eq "active") { $Warnings.Add("root_task_active_after_nodes_closed") }
    if ($unreviewed -gt 0) { $Warnings.Add("unreviewed_results_present") }
    [pscustomobject]@{
        map_count = $mapCount
        map_store_availability = $mapStoreAvailability
        node_count = $nodeCount
        edge_count = $edgeCount
        result_count = $resultCount
        open_leaf_nodes = $openLeaves
        root_task_status = $taskStatus
        accepted_result_count = Get-PerformanceCount (Get-PerformanceProperty $graph "accepted_result_count")
        unreviewed_result_count = $unreviewed
        control_count = Get-PerformanceCount (Get-PerformanceProperty $control "taskspace_control_count")
        action_manifest_count = Get-PerformanceCount (Get-PerformanceProperty $control "action_manifest_count")
        declared_action_count = Get-PerformanceCount (Get-PerformanceProperty $control "declared_action_count")
        initialize_and_execute_count = Get-PerformanceCount (Get-PerformanceProperty $control "initialize_and_execute_count")
        committed_initialize_and_execute_count = Get-PerformanceCount (Get-PerformanceProperty $control "committed_initialize_and_execute_count")
        failed_initialize_and_execute_count = Get-PerformanceCount (Get-PerformanceProperty $control "failed_initialize_and_execute_count")
        sequence_preflight_rejected_call_count = Get-PerformanceCount (Get-PerformanceProperty $control "sequence_preflight_rejected_call_count")
        control_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_failure_count")
        control_preflight_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_preflight_failure_count")
        control_handler_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_handler_failure_count")
        control_protocol_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_protocol_failure_count")
        control_state_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_state_failure_count")
        control_argument_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_argument_failure_count")
        control_resource_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "control_resource_failure_count")
        nested_action_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "nested_action_failure_count")
        ordinary_gate_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "ordinary_gate_failure_count")
        committed_control_count = Get-PerformanceCount (Get-PerformanceProperty $control "committed_control_count")
        graph_revision_commit_count = Get-PerformanceCount (Get-PerformanceProperty $control "graph_revision_commit_count")
        read_map_request_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_request_count")
        read_map_completion_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_completion_count")
        read_map_failure_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_failure_count")
        read_map_repeated_revision_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_repeated_revision_count")
        read_map_revision_lag_sample_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_revision_lag_sample_count")
        read_map_revision_lag_mean = Get-PerformanceNumber (Get-PerformanceProperty $control "read_map_revision_lag_mean")
        read_map_revision_lag_max = Get-PerformanceNumber (Get-PerformanceProperty $control "read_map_revision_lag_max")
        read_map_stale_revision_error_count = Get-PerformanceCount (Get-PerformanceProperty $control "read_map_stale_revision_error_count")
        control_actions = Get-PerformanceProperty $control "action_counts" ([pscustomobject]@{})
        runtime_event_count = Get-PerformanceCount (Get-PerformanceProperty $control "taskspace_runtime_event_count")
        snapshot_update_count = Get-PerformanceCount (Get-PerformanceProperty (Get-PerformanceProperty $control "runtime_event_counts") "snapshot_updated")
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
    $modeMapResult = Read-TaskspaceLogicalModeMap (Join-Path $pairDir "logical-mode-map.json")
    $modeMap = $modeMapResult.map
    $modeMapValid = [bool]$modeMapResult.valid
    $mode = if ($modeMapValid) { [string]$modeMap.$side } else { "unknown" }
    $repeat = if ($modeMapValid) { [int64]$modeMap.repeat } else { $null }
    $cache = Read-PerformanceJson (Join-Path $artifactDir "provider-cache-trace-summary.json") $Events
    $requests = Get-PerformanceWireRequestCount $artifactDir
    $actions = Get-PerformanceActionCounts $artifactDir $Events
    $cadence = Get-TaskspaceNativeCadenceFacts $artifactDir $Events
    $patchObservation = Get-TaskspacePatchObservability $artifactDir $Events
    $warnings = New-Object System.Collections.Generic.List[string]
    if ([string]$actions.protocol -eq "taskspace_exec" -and [string]$actions.availability -ne "measured") {
        $warnings.Add("taskspace_exec_observation_incomparable")
        foreach ($finding in @($actions.findings)) { $warnings.Add("taskspace_exec:$finding") }
    }
    $taints = @((Get-PerformanceProperty $metrics "metrics_taints" @()))
    $skipped = @($taints | Where-Object { [string]$_ -match '^side_selection_skipped:' }).Count -gt 0
    if (-not $modeMapValid) {
        $warnings.Add("logical_mode_map_invalid")
    } elseif ($skipped) {
        $warnings.Add("side_not_run")
    } else {
        if ($null -eq $requests.value) { $warnings.Add("provider_request_count_unavailable") }
        if (-not $cache) { $warnings.Add("provider_cache_trace_unavailable") }
        if ([string](Get-PerformanceProperty $metrics "external_validation_status") -eq "failed") { $warnings.Add("external_validation_failed") }
    }
    if (-not $modeMapValid -and $null -ne $Events) {
        $Events.Add([pscustomobject]@{
                event = "performance_logical_mode_map_invalid"
                code = "logical_mode_map_invalid"
                case_id = $caseId
                pair = $pair
                repeat = $repeat
                side = $side
                logical_mode = "unknown"
                metric_path = $MetricPath
                artifact_dir = $artifactDir
            })
    }
    $map = Get-PerformanceMapFacts $artifactDir $metrics $warnings
    $duplication = Get-PerformanceDuplicationFacts $artifactDir $Events
    $tokenSource = if ([bool]$requests.usage_measured) { $requests } else { $metrics }
    $tokenIdentity = Get-PerformanceTokenIdentity `
        $tokenSource `
        $cache `
        $requests.value `
        $skipped
    $countIdentity = Get-PerformanceCountIdentity `
        $metrics $actions $map $cadence $patchObservation $cache $mode $skipped
    if (-not $tokenIdentity.valid) {
        $warnings.Add("performance_token_identity_invalid")
        if ($null -ne $Events) {
            $Events.Add([pscustomobject]@{
                    event = "performance_token_identity_invalid"
                    code = "token_identity_invalid"
                    case_id = $caseId
                    pair = $pair
                    repeat = $repeat
                    side = $side
                    logical_mode = $mode
                    metric_path = $MetricPath
                    artifact_dir = $artifactDir
                    invalid_fields = @($tokenIdentity.invalid_fields)
                })
        }
    }
    if (-not $countIdentity.valid) {
        $warnings.Add("performance_count_identity_invalid")
        if ($null -ne $Events) {
            $Events.Add([pscustomobject]@{
                    event = "performance_count_identity_invalid"
                    code = "count_identity_invalid"
                    case_id = $caseId
                    pair = $pair
                    repeat = $repeat
                    side = $side
                    logical_mode = $mode
                    metric_path = $MetricPath
                    artifact_dir = $artifactDir
                    invalid_fields = @($countIdentity.invalid_fields)
                })
        }
    }
    $agentStatus = [string](Get-PerformanceProperty $metrics "agent_completion_status")
    $observationStatus = if (-not $modeMapValid -or -not $tokenIdentity.valid -or -not $countIdentity.valid) {
        "invalid"
    } elseif ($skipped) {
        "skipped"
    } elseif ($agentStatus -eq "complete") {
        "complete"
    } else {
        "incomplete"
    }
    $comparisonEligible = -not $skipped -and $modeMapValid -and $tokenIdentity.valid -and
        $countIdentity.valid -and
        $agentStatus -eq "complete"
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
            provider_logical_requests = $requests.logical
            provider_local_attempts = $requests.attempts
            provider_boundary_requests = $requests.boundary
            provider_completed_responses = $requests.completed
            provider_failed_or_cancelled_attempts = $requests.failed_or_cancelled
            ordinary_tools = if ([string]$actions.protocol -eq "taskspace_exec") {
                [int64]$actions.client_action_count + [int64]$actions.hosted_binding_count
            } else { Get-PerformanceCount (Get-PerformanceProperty $metrics "tool_call_count") }
            failed_tools = if ([string]$actions.protocol -eq "taskspace_exec") {
                $actions.failed_action_count
            } else { Get-PerformanceCount (Get-PerformanceProperty $metrics "failed_tool_call_count") }
            action_protocol = $actions.protocol
            exec_observation_status = Get-PerformanceProperty $actions "availability"
            provider_outer_tool_calls = $actions.provider_outer_tool_calls
            nested_actions = $actions.nested_action_count
            shell = $actions.shell; patch = $actions.patch; taskspace_control = $map.control_count
            taskspace_exec = Get-PerformanceCount (Get-PerformanceProperty $actions "exec_count")
            map_operations = Get-PerformanceCount (Get-PerformanceProperty $actions "map_operation_count")
            client_actions = Get-PerformanceCount (Get-PerformanceProperty $actions "client_action_count")
            hosted_bindings = Get-PerformanceCount (Get-PerformanceProperty $actions "hosted_binding_count")
            node_bindings = Get-PerformanceCount (Get-PerformanceProperty $actions "node_binding_count")
            client_results = Get-PerformanceCount (Get-PerformanceProperty $actions "client_result_count")
            hosted_results = Get-PerformanceCount (Get-PerformanceProperty $actions "hosted_result_count")
            exec_trace_events = Get-PerformanceCount (Get-PerformanceProperty $actions "trace_event_count")
            correlated_requests = Get-PerformanceCount (Get-PerformanceProperty $actions "correlated_request_count")
            correlated_outer_calls = Get-PerformanceCount (Get-PerformanceProperty $actions "correlated_outer_call_count")
            capability_identity = Get-PerformanceProperty $actions "capability_identity"
            wire_capability_identity = Get-PerformanceProperty $actions "wire_capability_identity"
            exec_findings = @((Get-PerformanceProperty $actions "findings" @()))
            action_manifests = $map.action_manifest_count
            declared_actions = $map.declared_action_count
            initialize_and_execute = $map.initialize_and_execute_count
            committed_initialize_and_execute = $map.committed_initialize_and_execute_count
            failed_initialize_and_execute = $map.failed_initialize_and_execute_count
            sequence_preflight_rejected_calls = $map.sequence_preflight_rejected_call_count
            control_failures = $map.control_failure_count
            control_preflight_failures = $map.control_preflight_failure_count
            control_handler_failures = $map.control_handler_failure_count
            control_protocol_failures = $map.control_protocol_failure_count
            control_state_failures = $map.control_state_failure_count
            control_argument_failures = $map.control_argument_failure_count
            control_resource_failures = $map.control_resource_failure_count
            nested_action_failures = $map.nested_action_failure_count
            ordinary_gate_failures = $map.ordinary_gate_failure_count
            committed_controls = $map.committed_control_count
            graph_revision_commits = $map.graph_revision_commit_count
            provider_tool_responses = $cadence.provider_tool_response_count
            control_responses = $cadence.control_response_count
            mixed_control_action_responses = $cadence.mixed_control_action_response_count
            multi_control_responses = $cadence.multi_control_response_count
            action_manifest_count = $cadence.action_manifest_count
            action_manifest_pairs = $cadence.action_manifest_pair_count
            action_manifest_violations = $cadence.action_manifest_violation_count
            orphan_siblings = $cadence.orphan_sibling_count
            cadence_declared_actions = $cadence.declared_action_count
            cadence_owned_siblings = $cadence.owned_sibling_count
            initialize_and_execute_pairs = $cadence.initialize_and_execute_pair_count
            execute_pairs = $cadence.execute_pair_count
            reopen_pairs = $cadence.reopen_pair_count
            finish_maps = $cadence.finish_map_count
            finish_map_final_work = $cadence.finish_map_final_work_count
            standalone_control_responses = $cadence.standalone_control_response_count
            terminal_candidates = $cadence.terminal_candidate_count
            terminal_extra_requests = $cadence.terminal_extra_request_count
            cadence_parse_errors = $cadence.control_argument_parse_error_count
            cadence_source = $cadence.availability
            action_trace_source = $actions.source
        }
        cost = [pscustomobject]@{
            wall_time_ms = Get-PerformanceNumber (Get-PerformanceProperty $metrics "wall_time_ms")
            input_tokens = $tokenIdentity.input_tokens
            cached_input_tokens = $tokenIdentity.cached_input_tokens
            uncached_input_tokens = $tokenIdentity.uncached_input_tokens
            output_tokens = $tokenIdentity.output_tokens
            full_cache_hit_rate = Get-PerformanceRatio `
                $tokenIdentity.cached_input_tokens `
                $tokenIdentity.input_tokens
            monetary_cost = $null; monetary_cost_status = "unavailable_no_unit_price_artifact"
        }
        cache = [pscustomobject]@{
            request_2_plus_count = $tokenIdentity.request_2_plus_count
            request_2_plus_cached_input_tokens =
                $tokenIdentity.request_2_plus_cached_input_tokens
            request_2_plus_uncached_input_tokens =
                $tokenIdentity.request_2_plus_uncached_input_tokens
            request_2_plus_hit_rate = Get-PerformanceNumber (Get-PerformanceProperty $cache "request_2_plus_hit_rate")
            prefix_comparison_count = Get-PerformanceCount (Get-PerformanceProperty $cache "prefix_comparison_count")
            prefix_preserved_count = Get-PerformanceCount (Get-PerformanceProperty $cache "prefix_preserved_count")
            prefix_preserved_rate = Get-PerformanceNumber (Get-PerformanceProperty $cache "prefix_preserved_rate")
            zero_cache_hit_count = Get-PerformanceCount (Get-PerformanceProperty $cache "zero_cache_hit_count")
            cache_warmup_candidate_count = Get-PerformanceCount (Get-PerformanceProperty $cache "cache_warmup_candidate_count")
            same_shape_zero_hit_count = Get-PerformanceCount (Get-PerformanceProperty $cache "same_shape_zero_hit_count")
            tool_choice_transition_count = Get-PerformanceCount (Get-PerformanceProperty $cache "tool_choice_transition_count")
            cache_shape_transition_count = Get-PerformanceCount (Get-PerformanceProperty $cache "cache_shape_transition_count")
            trace_coverage = Get-PerformanceNumber (Get-PerformanceProperty $cache "trace_coverage")
        }
        section_cost = Get-PerformanceSectionCostFacts $cache
        map = $map
        patch = $patchObservation
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
    foreach ($field in @("provider_requests", "provider_logical_requests", "provider_local_attempts", "provider_boundary_requests", "provider_completed_responses", "provider_failed_or_cancelled_attempts", "ordinary_tools", "failed_tools", "provider_outer_tool_calls", "nested_actions", "taskspace_exec", "map_operations", "client_actions", "hosted_bindings", "node_bindings", "client_results", "hosted_results", "exec_trace_events", "correlated_requests", "correlated_outer_calls", "taskspace_control", "action_manifests", "declared_actions", "initialize_and_execute", "committed_initialize_and_execute", "failed_initialize_and_execute", "sequence_preflight_rejected_calls", "control_failures", "control_protocol_failures", "control_state_failures", "nested_action_failures", "provider_tool_responses", "control_responses", "mixed_control_action_responses", "multi_control_responses", "action_manifest_count", "action_manifest_pairs", "action_manifest_violations", "orphan_siblings", "cadence_declared_actions", "cadence_owned_siblings", "initialize_and_execute_pairs", "execute_pairs", "reopen_pairs", "finish_maps", "finish_map_final_work", "standalone_control_responses", "terminal_candidates", "terminal_extra_requests", "cadence_parse_errors")) {
        $values = @($selected | ForEach-Object { $_.actions.$field })
        $sum[$field] = Get-PerformanceOptionalExactInt64Sum $values $field
    }
    foreach ($field in @("wall_time_ms")) {
        $values = @($selected | ForEach-Object { Get-PerformanceNumber $_.cost.$field } | Where-Object { $null -ne $_ })
        $sum[$field] = if ($values.Count) { [double](($values | Measure-Object -Sum).Sum) } else { $null }
    }
    foreach ($field in @("input_tokens", "cached_input_tokens", "uncached_input_tokens", "output_tokens")) {
        $sum[$field] = Get-PerformanceExactInt64Sum `
            @($selected | ForEach-Object { $_.cost.$field }) `
            $field
    }
    foreach ($field in @("map_count", "node_count", "edge_count", "result_count", "open_leaf_nodes", "unreviewed_result_count", "runtime_event_count")) {
        $values = @($selected | ForEach-Object { $_.map.$field })
        $sum[$field] = Get-PerformanceOptionalExactInt64Sum $values $field
    }
    Add-TaskspacePatchAggregateFields $sum $selected
    $cache2 = Get-PerformanceExactInt64Sum `
        @($selected | ForEach-Object { $_.cache.request_2_plus_cached_input_tokens }) `
        "request_2_plus_cached_input_tokens"
    $uncached2 = Get-PerformanceExactInt64Sum `
        @($selected | ForEach-Object { $_.cache.request_2_plus_uncached_input_tokens }) `
        "request_2_plus_uncached_input_tokens"
    $prefixCounts = @($selected | ForEach-Object { $_.cache.prefix_comparison_count })
    $prefixKepts = @($selected | ForEach-Object { $_.cache.prefix_preserved_count })
    $prefixCount = Get-PerformanceOptionalExactInt64Sum `
        $prefixCounts "prefix_comparison_count"
    $prefixKept = Get-PerformanceOptionalExactInt64Sum `
        $prefixKepts "prefix_preserved_count"
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
        section_cost = Get-PerformanceModeSectionCostAggregate $selected
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
. (Join-Path $PSScriptRoot "performance-observation-report.ps1")
