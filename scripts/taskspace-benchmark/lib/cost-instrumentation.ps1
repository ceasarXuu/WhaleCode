if (-not (Get-Command Get-TaskspaceCanonicalResponseItem -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "canonical-rollout.ps1")
}

function Add-TaskspaceCostCount {
    param([hashtable]$Table, [string]$Key)
    if ([string]::IsNullOrWhiteSpace($Key)) { $Key = "unknown" }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = 0 }
    $Table[$Key]++
}

function Convert-TaskspaceCostTable {
    param([hashtable]$Table)
    $ordered = [ordered]@{}
    foreach ($key in @($Table.Keys | Sort-Object)) { $ordered[$key] = $Table[$key] }
    [pscustomobject]$ordered
}

function ConvertFrom-TaskspaceCostCountObject {
    param($Counts)
    $table = @{}
    if ($null -eq $Counts) { return $table }
    foreach ($property in @($Counts.PSObject.Properties)) {
        try { $table[[string]$property.Name] = [int]$property.Value } catch {}
    }
    $table
}

function Get-TaskspaceCostJsonlRows {
    param([string]$Path)
    $rows = New-Object System.Collections.Generic.List[object]
    $parseErrors = 0
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ rows = @(); parse_errors = 0; source_status = "missing" }
    }
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $rows.Add(($line | ConvertFrom-Json))
        } catch {
            $parseErrors++
        }
    }
    [pscustomobject]@{ rows = @($rows.ToArray()); parse_errors = $parseErrors; source_status = "read" }
}

function Get-TaskspaceCostProperty {
    param($Value, [string[]]$Names)
    if ($null -eq $Value) { return $null }
    foreach ($name in $Names) {
        if ($Value.PSObject.Properties.Name -contains $name) { return $Value.$name }
    }
    $null
}

function Convert-TaskspaceTraceTags {
    param($Event)
    $table = [ordered]@{}
    if ($null -eq $Event) { return [pscustomobject]$table }
    $tags = Get-TaskspaceCostProperty $Event @("tags")
    if ($null -eq $tags -and $null -ne $Event.details) {
        $tags = Get-TaskspaceCostProperty $Event.details @("tags")
    }
    foreach ($tag in @($tags)) {
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

function Get-TaskspaceTraceField {
    param($Event, [string[]]$Names)
    $value = Get-TaskspaceCostProperty $Event $Names
    if ($null -ne $value) { return $value }
    if ($null -ne $Event.details) { return Get-TaskspaceCostProperty $Event.details $Names }
    $null
}

function Get-TaskspaceTraceEvents {
    param(
        [string]$ObservabilityJsonPath,
        [string[]]$Kinds,
        [AllowEmptyString()][string]$RolloutJsonlPath = ""
    )
    $events = New-Object System.Collections.Generic.List[object]
    $seen = @{}
    try {
        if (-not [string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -and (Test-Path -LiteralPath $ObservabilityJsonPath)) {
            $obs = (Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath) | ConvertFrom-Json
            foreach ($event in @($obs.timeline)) {
                $kind = [string](Get-TaskspaceTraceField $event @("kind"))
                if ($Kinds -notcontains $kind) { continue }
                $traceId = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
                $dedupeKey = if ([string]::IsNullOrWhiteSpace($traceId)) { "obs:${kind}:$($events.Count)" } else { "${kind}:$traceId" }
                if ($seen.ContainsKey($dedupeKey)) { continue }
                $seen[$dedupeKey] = $true
                $events.Add($event)
            }
        }
    } catch {}
    try {
        if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath) -and (Test-Path -LiteralPath $RolloutJsonlPath)) {
            foreach ($line in [System.IO.File]::ReadLines($RolloutJsonlPath)) {
                if ([string]::IsNullOrWhiteSpace($line)) { continue }
                $row = $null
                try { $row = $line | ConvertFrom-Json } catch { continue }
                if ([string]$row.type -ne "event_msg" -or $null -eq $row.payload) { continue }
                $payload = $row.payload
                $snapshot = Get-TaskspaceCostProperty $payload @("snapshot")
                foreach ($traceEvent in @((Get-TaskspaceCostProperty $snapshot @("traceEvents", "trace_events")))) {
                    $traceKind = [string](Get-TaskspaceTraceField $traceEvent @("kind"))
                    if ($Kinds -notcontains $traceKind) { continue }
                    $traceId = [string](Get-TaskspaceTraceField $traceEvent @("traceEventId", "trace_event_id", "id"))
                    $dedupeKey = if ([string]::IsNullOrWhiteSpace($traceId)) { "rollout-snapshot:${traceKind}:$($events.Count)" } else { "${traceKind}:$traceId" }
                    if ($seen.ContainsKey($dedupeKey)) { continue }
                    $seen[$dedupeKey] = $true
                    $events.Add($traceEvent)
                }
                $kind = [string]$payload.kind
                if ($Kinds -notcontains $kind) { continue }
                $traceId = [string]$payload.traceEventId
                $dedupeKey = if ([string]::IsNullOrWhiteSpace($traceId)) { "rollout:${kind}:$($events.Count)" } else { "${kind}:$traceId" }
                if ($seen.ContainsKey($dedupeKey)) { continue }
                $seen[$dedupeKey] = $true
                $events.Add([pscustomobject]@{
                    id = $traceId
                    trace_event_id = $traceId
                    kind = $kind
                    task_id = [string]$payload.taskId
                    map_id = [string]$payload.mapId
                    node_id = [string]$payload.nodeId
                    result_id = [string]$payload.resultId
                    call_id = [string]$payload.callId
                    action_class = [string]$payload.actionClass
                    tool_success = $payload.toolSuccess
                    tags = @($payload.tags)
                    artifact_refs = @($payload.artifactRefs)
                    created_at_ms = $payload.createdAtMs
                })
            }
        }
    } catch {}
    @($events.ToArray())
}

function Convert-TaskspaceTraceInt {
    param($Value, [int]$Default = 0)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $Default }
    try { return [int]$Value } catch { return $Default }
}

function Convert-TaskspaceTraceNullableInt {
    param($Value)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $null }
    try { return [int64]$Value } catch { return $null }
}

function Convert-TaskspaceTraceBool {
    param($Value, [bool]$Default = $false)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $Default }
    $text = [string]$Value
    if ($text -ieq "true") { return $true }
    if ($text -ieq "false") { return $false }
    return $Default
}

function Get-TaskspaceSubagentReviewDebt {
    param([AllowEmptyString()][string]$ObservabilityJsonPath = "")
    $sourceStatus = "missing"
    $latestSnapshot = $null
    if (-not [string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -and (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        try {
            $obs = (Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath) | ConvertFrom-Json
            $sourceStatus = "read"
            foreach ($event in @($obs.timeline)) {
                $kind = [string](Get-TaskspaceTraceField $event @("kind"))
                $details = Get-TaskspaceCostProperty $event @("details")
                $detailType = [string](Get-TaskspaceCostProperty $details @("type"))
                if ($kind -ne "snapshot_updated" -and $detailType -ne "snapshot_updated") { continue }
                $snapshot = Get-TaskspaceCostProperty $details @("snapshot")
                if ($null -ne $snapshot) { $latestSnapshot = $snapshot }
            }
        } catch {
            $sourceStatus = "parse_error"
        }
    }
    $results = New-Object System.Collections.Generic.List[object]
    if ($null -ne $latestSnapshot) {
        foreach ($map in @($latestSnapshot.maps)) {
            $mapId = [string](Get-TaskspaceCostProperty $map @("id", "mapId", "map_id"))
            foreach ($result in @($map.results)) {
                $subagentPlanId = [string](Get-TaskspaceCostProperty $result @("subagentPlanId", "subagent_plan_id"))
                if ([string]::IsNullOrWhiteSpace($subagentPlanId)) { continue }
                $evidencePackage = Get-TaskspaceCostProperty $result @("evidencePackage", "evidence_package")
                $validity = [string](Get-TaskspaceCostProperty $evidencePackage @("validity"))
                if ([string]::IsNullOrWhiteSpace($validity)) { $validity = "unreviewed" }
                $results.Add([pscustomobject]@{
                    map_id = $mapId
                    node_id = [string](Get-TaskspaceCostProperty $result @("nodeId", "node_id"))
                    result_id = [string](Get-TaskspaceCostProperty $result @("id", "resultId", "result_id"))
                    subagent_plan_id = $subagentPlanId
                    kind = [string](Get-TaskspaceCostProperty $result @("kind"))
                    validity = $validity
                })
            }
        }
    }
    $resultRows = @($results.ToArray())
    $unreviewed = @($resultRows | Where-Object { [string]$_.validity -eq "unreviewed" })
    $reviewed = @($resultRows | Where-Object { [string]$_.validity -ne "unreviewed" })
    $reviewDebtStatus = if ($sourceStatus -ne "read") {
        "not_measured"
    } elseif ($null -eq $latestSnapshot) {
        "not_measured"
    } elseif ($unreviewed.Count -gt 0) {
        "unreviewed_subagent_results"
    } else {
        "no_unreviewed_subagent_results"
    }
    [pscustomobject]@{
        source_status = $sourceStatus
        review_debt_status = $reviewDebtStatus
        subagent_result_count = [int]$resultRows.Count
        reviewed_subagent_result_count = [int]$reviewed.Count
        unreviewed_subagent_result_count = [int]$unreviewed.Count
        subagent_results = @($resultRows)
        unreviewed_subagent_results = @($unreviewed)
    }
}

function New-TaskspaceBudgetArtifacts {
    param([string]$ObservabilityJsonPath, [AllowEmptyString()][string]$RolloutJsonlPath = "")
    $activeBudgetEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("active_budget") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        if ([string]$tags.producer -ne "runtime") { continue }
        $activeBudgetEvents.Add([pscustomobject]@{
            schema_version = "taskspace-active-budget-v1"
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            active_budget_source = [string]$tags.active_budget_source
            profile_name = [string]$tags.profile_name
            route_mode = [string]$tags.route_mode
            max_rollout_model_requests = Convert-TaskspaceTraceInt $tags.max_rollout_model_requests
            max_model_requests_per_node = Convert-TaskspaceTraceInt $tags.max_model_requests_per_node
            max_spawn_agent_calls = Convert-TaskspaceTraceInt $tags.max_spawn_agent_calls
            max_nodes = Convert-TaskspaceTraceInt $tags.max_nodes
            max_projection_tokens = Convert-TaskspaceTraceInt $tags.max_projection_tokens
        })
    }
    $budgetEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("provider_request_budget") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        $budgetEvents.Add([pscustomobject]@{
            schema_version = "taskspace-budget-event-v1"
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            request_id = [string](Get-TaskspaceTraceField $event @("call_id"))
            transport = [string]$tags.transport
            status = [string]$tags.status
            request_phase = [string]$tags.request_phase
            request_reason_schema_present = ([string]$tags.schema -eq "taskspace-provider-request-reason-v1" -or -not [string]::IsNullOrWhiteSpace([string]$tags.trigger_kind))
            node_kind = [string]$tags.node_kind
            trigger_kind = [string]$tags.trigger_kind
            response_actionability_previous = [string]$tags.response_actionability_previous
            previous_response_recovery_action = [string]$tags.previous_response_recovery_action
            previous_response_trace_event_id = [string]$tags.previous_response_trace_event_id
            latest_tool_result_refs = [string]$tags.latest_tool_result_refs
            model_visible_feedback_refs = [string]$tags.model_visible_feedback_refs
            adoption_blockers = [string]$tags.adoption_blockers
            projection_bundle_hash = [string]$tags.projection_bundle_hash
            request_reason_delta = [string]$tags.request_reason_delta
            repeated_same_reason_count = Convert-TaskspaceTraceInt $tags.repeated_same_reason_count
            reason_confidence = [string]$tags.reason_confidence
            hard_stop_stage = [string]$tags.hard_stop_stage
            hard_stop_reason = [string]$tags.hard_stop_reason
            producer = [string]$tags.producer
            request_count_before = Convert-TaskspaceTraceInt $tags.request_count_before
            request_count_after = Convert-TaskspaceTraceInt $tags.request_count_after
            max_requests = Convert-TaskspaceTraceInt $tags.max_requests
            active_budget_source = [string]$tags.active_budget_source
            route_mode = [string]$tags.route_mode
            profile_name = [string]$tags.profile_name
            node_request_count = Convert-TaskspaceTraceInt $tags.node_request_count
            max_model_requests_per_node = Convert-TaskspaceTraceInt $tags.max_model_requests_per_node
            post_budget_grace_requests = Convert-TaskspaceTraceInt $tags.post_budget_grace_requests
            runtime_budget_state = [string]$tags.runtime_budget_state
            budget_response_action_taken = Convert-TaskspaceTraceBool $tags.budget_response_action_taken $false
            input_tokens = Get-TaskspaceUsageNumber $tags @("input_tokens")
            cached_input_tokens = Get-TaskspaceUsageNumber $tags @("cached_input_tokens")
            output_tokens = Get-TaskspaceUsageNumber $tags @("output_tokens")
            reasoning_output_tokens = Get-TaskspaceUsageNumber $tags @("reasoning_output_tokens")
            total_tokens = Get-TaskspaceUsageNumber $tags @("total_tokens")
            started_at_ms = Convert-TaskspaceTraceNullableInt $tags.started_at_ms
            completed_at_ms = Convert-TaskspaceTraceNullableInt $tags.completed_at_ms
            latency_ms = Convert-TaskspaceTraceNullableInt $tags.latency_ms
            model_request_duration_ms = Convert-TaskspaceTraceNullableInt $tags.model_request_duration_ms
            provider_payload_sha256 = [string]$tags.provider_payload_sha256
            provider_payload_bytes = Convert-TaskspaceTraceInt $tags.provider_payload_bytes
            provider_wire_api = [string]$tags.provider_wire_api
            tools_count = Convert-TaskspaceTraceInt $tags.tools_count
            tools_present = Convert-TaskspaceTraceBool $tags.tools_present $false
            request_shape_classifier = [string]$tags.request_shape_classifier
            messages_hash = [string]$tags.messages_hash
            stable_prefix_hash = [string]$tags.stable_prefix_hash
            dynamic_suffix_hash = [string]$tags.dynamic_suffix_hash
            exact_payload_scan_event_id = [string]$tags.exact_payload_scan_event_id
            exact_payload_scan_passed = Convert-TaskspaceTraceBool $tags.exact_payload_scan_passed $false
            active_projection_present = Convert-TaskspaceTraceBool $tags.active_projection_present $false
            active_projection_count = Convert-TaskspaceTraceInt $tags.active_projection_count
            large_raw_output_tokens = Convert-TaskspaceTraceInt $tags.large_raw_output_tokens
            runtime_boundary_forbidden_markers = [string]$tags.runtime_boundary_forbidden_markers
            protected_items_present = Convert-TaskspaceTraceBool $tags.protected_items_present $false
            replacement_confirmed = Convert-TaskspaceTraceBool $tags.replacement_confirmed $false
        })
    }
    $qualityEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("budget_quality_impact") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        $qualityEvents.Add([pscustomobject]@{
            schema_version = "taskspace-budget-quality-impact-v1"
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            provider_request_budget_trace_event_id = [string]$tags.provider_request_budget_trace_event_id
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            request_id = [string](Get-TaskspaceTraceField $event @("call_id"))
            budget_action = [string]$tags.budget_action
            provider_request_status = [string]$tags.provider_request_status
            counter_name = [string]$tags.counter_name
            counter_value = Convert-TaskspaceTraceInt $tags.counter_value
            counter_limit = Convert-TaskspaceTraceInt $tags.counter_limit
            active_budget_source = [string]$tags.active_budget_source
            route_mode = [string]$tags.route_mode
            budget_state_before = [string]$tags.budget_state_before
            budget_state_after = [string]$tags.budget_state_after
            budget_transition_reason = [string]$tags.budget_transition_reason
            request_phase = [string]$tags.request_phase
            logical_request_id = [string]$tags.logical_request_id
            attempt_seq = Convert-TaskspaceTraceInt $tags.attempt_seq
            score_eligible = Convert-TaskspaceTraceBool $tags.score_eligible $false
            budget_induced_validation_skip = Convert-TaskspaceTraceBool $tags.budget_induced_validation_skip $false
            manual_override_used = Convert-TaskspaceTraceBool $tags.manual_override_used $false
            bounded_recovery_used = Convert-TaskspaceTraceBool $tags.bounded_recovery_used $false
            final_classification = [string]$tags.final_classification
        })
    }
    $qualityByProviderTrace = @{}
    foreach ($quality in @($qualityEvents.ToArray())) {
        if (-not [string]::IsNullOrWhiteSpace([string]$quality.provider_request_budget_trace_event_id)) {
            $qualityByProviderTrace[[string]$quality.provider_request_budget_trace_event_id] = $true
        }
    }
    $budgetActions = @($budgetEvents.ToArray() | Where-Object { [bool]$_.budget_response_action_taken })
    $missing = 0
    foreach ($action in $budgetActions) {
        if (-not $qualityByProviderTrace.ContainsKey([string]$action.trace_event_id)) { $missing++ }
    }
    $summary = [pscustomobject]@{
        schema_version = "taskspace-budget-quality-impact-summary-v1"
        budget_event_count = [int]$budgetEvents.Count
        active_budget_source = if ($activeBudgetEvents.Count -gt 0) { [string]$activeBudgetEvents[0].active_budget_source } elseif ($budgetEvents.Count -gt 0) { [string](Get-TaskspaceCostProperty $budgetEvents[0] @("active_budget_source")) } else { "" }
        route_mode = if ($activeBudgetEvents.Count -gt 0) { [string]$activeBudgetEvents[0].route_mode } elseif ($budgetEvents.Count -gt 0) { [string](Get-TaskspaceCostProperty $budgetEvents[0] @("route_mode")) } else { "" }
        max_rollout_model_requests = if ($activeBudgetEvents.Count -gt 0) { [int]$activeBudgetEvents[0].max_rollout_model_requests } elseif ($budgetEvents.Count -gt 0) { [int](@($budgetEvents.ToArray() | Measure-Object -Property max_requests -Maximum).Maximum) } else { 0 }
        max_model_requests_per_node = if ($activeBudgetEvents.Count -gt 0) { [int]$activeBudgetEvents[0].max_model_requests_per_node } elseif ($budgetEvents.Count -gt 0) { [int](@($budgetEvents.ToArray() | Measure-Object -Property max_model_requests_per_node -Maximum).Maximum) } else { 0 }
        budget_quality_impact_event_count = [int]$qualityEvents.Count
        budget_action_count = [int]$budgetActions.Count
        budget_quality_impact_logged_for_every_budget_action = ($missing -eq 0)
        budget_quality_impact_missing_count = [int]$missing
        budget_induced_validation_skip_count = [int](@($qualityEvents.ToArray() | Where-Object { [bool]$_.budget_induced_validation_skip -or [string]$_.final_classification -eq "validation_skip" }).Count)
        budget_induced_score_ineligible_solved_count = [int](@($qualityEvents.ToArray() | Where-Object { -not [bool]$_.score_eligible -and [string]$_.final_classification -eq "solved" }).Count)
        blocked_by_budget_samples_count = [int](@($qualityEvents.ToArray() | Where-Object { [string]$_.final_classification -eq "blocked_by_budget" }).Count)
        manual_override_used_count = [int](@($qualityEvents.ToArray() | Where-Object { [bool]$_.manual_override_used }).Count)
    }
    [pscustomobject]@{
        budget_events = @($budgetEvents.ToArray())
        active_budget_events = @($activeBudgetEvents.ToArray())
        budget_quality_impact_events = @($qualityEvents.ToArray())
        budget_quality_impact_summary = $summary
    }
}

function New-TaskspaceExactPayloadScanEvents {
    param([string]$ObservabilityJsonPath, [AllowEmptyString()][string]$RolloutJsonlPath = "")
    $scanEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("exact_payload_scan") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        $scanEvents.Add([pscustomobject]@{
            schema_version = "taskspace-exact-payload-scan-event-v1"
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            request_id = [string](Get-TaskspaceTraceField $event @("call_id"))
            producer = [string]$tags.producer
            scan_event_id = [string]$tags.scan_event_id
            provider_request_budget_trace_event_id = [string]$tags.provider_request_budget_trace_event_id
            provider_payload_sha256 = [string]$tags.provider_payload_sha256
            provider_payload_bytes = Convert-TaskspaceTraceInt $tags.provider_payload_bytes
            scanner_version = [string]$tags.scanner_version
            matcher_version = [string]$tags.matcher_version
            checked_byte_ranges = [string]$tags.checked_byte_ranges
            negative_checks_performed = [string]$tags.negative_checks_performed
            projection_required = Convert-TaskspaceTraceBool $tags.projection_required $true
            active_projection_present = Convert-TaskspaceTraceBool $tags.active_projection_present $false
            active_projection_count = Convert-TaskspaceTraceInt $tags.active_projection_count
            large_raw_output_tokens = Convert-TaskspaceTraceInt $tags.large_raw_output_tokens
            runtime_boundary_forbidden_markers = [string]$tags.runtime_boundary_forbidden_markers
            protected_items_present = Convert-TaskspaceTraceBool $tags.protected_items_present $false
            replacement_confirmed = Convert-TaskspaceTraceBool $tags.replacement_confirmed $false
            passed = Convert-TaskspaceTraceBool $tags.passed $false
            failure_reasons = [string]$tags.failure_reasons
        })
    }
    @($scanEvents.ToArray())
}

function New-TaskspaceActiveReplacementArtifacts {
    param([object[]]$BudgetEvents, [object[]]$ExactPayloadScanEvents)
    $providerByJoin = @{}
    foreach ($event in @($BudgetEvents)) {
        $requestId = [string]$event.request_id
        $payloadHash = [string]$event.provider_payload_sha256
        if ([string]::IsNullOrWhiteSpace($requestId) -or [string]::IsNullOrWhiteSpace($payloadHash)) { continue }
        $providerByJoin["$requestId|$payloadHash"] = $true
    }
    $scanEvents = @($ExactPayloadScanEvents | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_.scan_event_id) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.request_id) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.provider_payload_sha256) -and
        ([string]$_.producer -eq "provider_payload_scanner" -or [string]$_.producer -eq "provider_lifecycle")
    } | ForEach-Object {
        $joinKey = "$([string]$_.request_id)|$([string]$_.provider_payload_sha256)"
        $_ | Add-Member -NotePropertyName matching_provider_event -NotePropertyValue ([bool]$providerByJoin.ContainsKey($joinKey)) -Force
        $_
    })
    $selected = @($scanEvents | Where-Object { [bool]$_.passed -and [bool]$_.matching_provider_event -and [bool]$_.projection_required } | Select-Object -First 1)
    if ($selected.Count -eq 0) {
        $selected = @($scanEvents | Select-Object -First 1)
    }
    $first = if ($selected.Count -gt 0) { $selected[0] } else { $null }
    $matchingScanEvents = @($scanEvents |
        Where-Object { [bool]$_.matching_provider_event } |
        Group-Object -Property scan_event_id |
        ForEach-Object { $_.Group | Select-Object -First 1 })
    $failedMatchingScanEvents = @($matchingScanEvents | Where-Object { -not [bool]$_.passed })
    $projectionScanEvents = @($matchingScanEvents | Where-Object { [bool]$_.projection_required })
    $unconfirmedMatchingScanEvents = @($projectionScanEvents | Where-Object { -not [bool]$_.replacement_confirmed })
    $projectionUniquenessViolations = @($projectionScanEvents | Where-Object { [int]$_.active_projection_count -gt 1 })
    $projectionCountMaximum = if ($projectionScanEvents.Count -gt 0) {
        [int](($projectionScanEvents | Measure-Object -Property active_projection_count -Maximum).Maximum)
    } else { 0 }
    $allMatchingPayloadScansPassed = ($matchingScanEvents.Count -gt 0 -and $failedMatchingScanEvents.Count -eq 0)
    $replacementConfirmed = ($allMatchingPayloadScansPassed -and $unconfirmedMatchingScanEvents.Count -eq 0 -and $projectionUniquenessViolations.Count -eq 0)
    $report = [pscustomobject]@{
        schema_version = "taskspace-active-context-replacement-report-v1"
        provider_payload_available = ($null -ne $first -and -not [string]::IsNullOrWhiteSpace([string]$first.provider_payload_sha256))
        request_id = if ($null -ne $first) { [string]$first.request_id } else { "" }
        provider_payload_sha256 = if ($null -ne $first) { [string]$first.provider_payload_sha256 } else { "" }
        exact_payload_scan_passed = [bool]$allMatchingPayloadScansPassed
        exact_payload_scan_event_id = if ($null -ne $first) { [string]$first.scan_event_id } else { "" }
        exact_payload_scan_producer = if ($null -ne $first) { [string]$first.producer } else { "" }
        exact_payload_scan_matching_provider_event = if ($null -ne $first) { [bool]$first.matching_provider_event } else { $false }
        active_projection_count = if ($null -ne $first) { [int]$first.active_projection_count } else { 0 }
        active_projection_count_max = $projectionCountMaximum
        active_projection_uniqueness_violation_count = $projectionUniquenessViolations.Count
        matching_payload_scan_count = $matchingScanEvents.Count
        failed_matching_payload_scan_count = $failedMatchingScanEvents.Count
        replacement_confirmed = [bool]$replacementConfirmed
        large_raw_output_tokens = if ($null -ne $first) { [int]$first.large_raw_output_tokens } else { 0 }
        runtime_boundary_forbidden_markers = if ($null -ne $first) { [string]$first.runtime_boundary_forbidden_markers } else { "" }
        protected_items_present = if ($null -ne $first) { [bool]$first.protected_items_present } else { $false }
    }
    [pscustomobject]@{
        exact_payload_scan_events = @($scanEvents)
        active_context_replacement_report = $report
    }
}

function New-TaskspaceProviderRequestArtifacts {
    param([object[]]$BudgetEvents, $RequestSummary = $null)
    $events = New-Object System.Collections.Generic.List[object]
    $phaseCounts = @{}
    $phaseTokens = @{}
    $triggerCounts = @{}
    $deltaCounts = @{}
    $phaseKnown = 0
    $reasonSchemaCount = 0
    $reasonKnown = 0
    $unknownReason = 0
    $repeatedNoDeltaCount = 0
    $terminalStatuses = @("response_completed", "response_failed", "cancelled", "blocked")
    foreach ($event in @($BudgetEvents)) {
        if ([string]::IsNullOrWhiteSpace([string]$event.request_id)) { continue }
        $phase = [string]$event.request_phase
        if ([string]::IsNullOrWhiteSpace($phase)) { $phase = "unknown" }
        if ($phase -ne "unknown") { $phaseKnown++ }
        $triggerKind = [string]$event.trigger_kind
        $deltaKind = [string]$event.request_reason_delta
        $hasReasonSchema = [bool]$event.request_reason_schema_present -or -not [string]::IsNullOrWhiteSpace($triggerKind)
        if ($hasReasonSchema) { $reasonSchemaCount++ }
        if ($hasReasonSchema -and -not [string]::IsNullOrWhiteSpace($triggerKind) -and $triggerKind -ne "unknown") {
            $reasonKnown++
            Add-TaskspaceCostCount $triggerCounts $triggerKind
        } else {
            $unknownReason++
        }
        if (-not [string]::IsNullOrWhiteSpace($deltaKind)) {
            Add-TaskspaceCostCount $deltaCounts $deltaKind
        }
        if ($deltaKind -eq "none" -and [int]$event.repeated_same_reason_count -gt 0) {
            $repeatedNoDeltaCount++
        }
        Add-TaskspaceCostCount $phaseCounts $phase
        if (-not $phaseTokens.ContainsKey($phase)) {
            $phaseTokens[$phase] = [ordered]@{
                request_count = 0
                terminal_request_count = 0
                input_tokens = [int64]0
                cached_input_tokens = [int64]0
                output_tokens = [int64]0
                reasoning_output_tokens = [int64]0
                total_tokens = [int64]0
            }
        }
        $phaseTokens[$phase].request_count++
        if ([string]$event.status -in $terminalStatuses) { $phaseTokens[$phase].terminal_request_count++ }
        $inputTokens = Convert-TaskspaceTraceInt $event.input_tokens
        $cachedInputTokens = Convert-TaskspaceTraceInt $event.cached_input_tokens
        $outputTokens = Convert-TaskspaceTraceInt $event.output_tokens
        $reasoningOutputTokens = Convert-TaskspaceTraceInt $event.reasoning_output_tokens
        $totalTokens = Convert-TaskspaceTraceInt $event.total_tokens
        $phaseTokens[$phase].input_tokens += [int64]$inputTokens
        $phaseTokens[$phase].cached_input_tokens += [int64]$cachedInputTokens
        $phaseTokens[$phase].output_tokens += [int64]$outputTokens
        $phaseTokens[$phase].reasoning_output_tokens += [int64]$reasoningOutputTokens
        $phaseTokens[$phase].total_tokens += [int64]$totalTokens
        $events.Add([pscustomobject]@{
            schema_version = "taskspace-provider-request-budget-event-v1"
            request_id = [string]$event.request_id
            logical_request_id = [string]$event.logical_request_id
            parent_request_id = [string]$event.parent_request_id
            attempt_seq = Convert-TaskspaceTraceInt $event.attempt_seq
            request_phase = if ([string]::IsNullOrWhiteSpace($phase)) { "unknown" } else { $phase }
            request_reason_schema_present = [bool]$event.request_reason_schema_present
            node_kind = [string]$event.node_kind
            trigger_kind = [string]$event.trigger_kind
            response_actionability_previous = [string]$event.response_actionability_previous
            previous_response_recovery_action = [string]$event.previous_response_recovery_action
            previous_response_trace_event_id = [string]$event.previous_response_trace_event_id
            latest_tool_result_refs = [string]$event.latest_tool_result_refs
            model_visible_feedback_refs = [string]$event.model_visible_feedback_refs
            adoption_blockers = [string]$event.adoption_blockers
            projection_bundle_hash = [string]$event.projection_bundle_hash
            request_reason_delta = [string]$event.request_reason_delta
            repeated_same_reason_count = Convert-TaskspaceTraceInt $event.repeated_same_reason_count
            reason_confidence = [string]$event.reason_confidence
            hard_stop_stage = [string]$event.hard_stop_stage
            hard_stop_reason = [string]$event.hard_stop_reason
            producer = [string]$event.producer
            task_id = [string]$event.task_id
            map_id = [string]$event.map_id
            node_id = [string]$event.node_id
            status = [string]$event.status
            transport = [string]$event.transport
            trace_event_id = [string]$event.trace_event_id
            started_at_ms = Convert-TaskspaceTraceNullableInt $event.started_at_ms
            completed_at_ms = Convert-TaskspaceTraceNullableInt $event.completed_at_ms
            latency_ms = Convert-TaskspaceTraceNullableInt $event.latency_ms
            model_request_duration_ms = Convert-TaskspaceTraceNullableInt $event.model_request_duration_ms
            input_tokens = $inputTokens
            cached_input_tokens = $cachedInputTokens
            output_tokens = $outputTokens
            reasoning_output_tokens = $reasoningOutputTokens
            total_tokens = $totalTokens
            provider_payload_sha256 = [string]$event.provider_payload_sha256
            provider_payload_bytes = [int]$event.provider_payload_bytes
            exact_payload_scan_event_id = [string]$event.exact_payload_scan_event_id
            provider_wire_api = [string]$event.provider_wire_api
            tools_count = [int]$event.tools_count
            tools_present = [bool]$event.tools_present
            request_shape_classifier = [string]$event.request_shape_classifier
            messages_hash = [string]$event.messages_hash
            stable_prefix_hash = [string]$event.stable_prefix_hash
            dynamic_suffix_hash = [string]$event.dynamic_suffix_hash
        })
    }
    $count = [int]$events.Count
    $distinctRequestCount = @($events.ToArray() | Select-Object -ExpandProperty request_id -Unique).Count
    $terminalRequestCount = @($events.ToArray() | Where-Object { [string]$_.status -in $terminalStatuses } | Select-Object -ExpandProperty request_id -Unique).Count
    $expectedRequestCount = Convert-TaskspaceTraceInt (Get-TaskspaceCostProperty $RequestSummary @("model_request_count"))
    if ($expectedRequestCount -lt $distinctRequestCount) { $expectedRequestCount = $distinctRequestCount }
    $coverage = if ($count -gt 0) { [int][Math]::Round(([double]$phaseKnown / [double]$count) * 100.0) } else { 0 }
    $unknownRatio = if ($count -gt 0) { [int][Math]::Round((([double]$count - [double]$phaseKnown) / [double]$count) * 100.0) } else { 100 }
    $reasonCoverage = if ($count -gt 0) { [int][Math]::Round(([double]$reasonKnown / [double]$count) * 100.0) } else { 0 }
    $reasonUnknownRatio = if ($count -gt 0) { [int][Math]::Round(([double]$unknownReason / [double]$count) * 100.0) } else { 100 }
    $reasonCoverageStatus = if ($count -eq 0) {
        "missing"
    } elseif ($reasonSchemaCount -eq 0) {
        "unavailable"
    } elseif ($unknownReason -eq 0) {
        "measured"
    } else {
        "measured_with_unknown"
    }
    $hookCoverage = if ($expectedRequestCount -gt 0) { [int][Math]::Min(100, [Math]::Round(([double]$distinctRequestCount / [double]$expectedRequestCount) * 100.0)) } else { 0 }
    $terminalCoverage = if ($expectedRequestCount -gt 0) { [int][Math]::Min(100, [Math]::Round(([double]$terminalRequestCount / [double]$expectedRequestCount) * 100.0)) } else { 0 }
    $phaseTokenSummary = [ordered]@{}
    foreach ($key in @($phaseTokens.Keys | Sort-Object)) { $phaseTokenSummary[$key] = [pscustomobject]$phaseTokens[$key] }
    $nonModelSamplingPhaseNames = @($phaseCounts.Keys | Where-Object { $_ -ne "model_sampling" -and $_ -ne "final_synthesis" -and $_ -ne "unknown" } | Sort-Object)
    $nonModelSamplingPhaseCount = [int](@($nonModelSamplingPhaseNames | ForEach-Object { [int]$phaseCounts[$_] } | Measure-Object -Sum).Sum)
    $phaseDiversityGatePass = $nonModelSamplingPhaseNames.Count -ge 2
    [pscustomobject]@{
        provider_request_events = @($events.ToArray())
        request_phase_summary = [pscustomobject]@{
            schema_version = "taskspace-request-phase-summary-v1"
            provider_request_hook_coverage = $hookCoverage
            provider_request_terminal_coverage = $terminalCoverage
            request_phase_attribution_coverage = $coverage
            unknown_request_phase_ratio = $unknownRatio
            provider_request_event_count = $count
            provider_request_distinct_count = [int]$distinctRequestCount
            provider_request_terminal_count = [int]$terminalRequestCount
            expected_model_request_count = [int]$expectedRequestCount
            phase_counts = Convert-TaskspaceCostTable $phaseCounts
            phase_token_summary = [pscustomobject]$phaseTokenSummary
            non_model_sampling_phase_count = [int]$nonModelSamplingPhaseCount
            non_model_sampling_distinct_phase_count = [int]$nonModelSamplingPhaseNames.Count
            phase_diversity_gate_pass = [bool]$phaseDiversityGatePass
        }
        request_reason_summary = [pscustomobject]@{
            schema_version = "taskspace-request-reason-summary-v1"
            request_reason_coverage_status = $reasonCoverageStatus
            request_reason_event_count = [int]$reasonSchemaCount
            provider_request_event_count = $count
            request_reason_attribution_coverage = $reasonCoverage
            unknown_request_reason_ratio = $reasonUnknownRatio
            request_reason_unknown_count = [int]$unknownReason
            repeated_same_reason_no_delta_count = [int]$repeatedNoDeltaCount
            trigger_kind_counts = Convert-TaskspaceCostTable $triggerCounts
            request_reason_delta_counts = Convert-TaskspaceCostTable $deltaCounts
        }
    }
}

function New-TaskspaceProviderWireCacheTraceArtifacts {
    param([Parameter(Mandatory = $true)][string]$TracePath)
    $shapes = @{}
    $terminals = @{}
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $TracePath -ErrorAction SilentlyContinue)) {
        if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
        try { $event = $line | ConvertFrom-Json } catch { continue }
        if ([string]$event.schema_version -ne "provider-chat-wire-trace-v2") { continue }
        $requestId = [string]$event.request_id
        if ([string]::IsNullOrWhiteSpace($requestId)) { continue }
        if ([string]$event.status -eq "payload_captured") {
            $shapes[$requestId] = $event
        } elseif ([string]$event.event_name -eq "provider.chat_wire_request_terminal") {
            $terminals[$requestId] = $event
        }
    }
    $events = New-Object System.Collections.Generic.List[object]
    $shapeCounts = @{}
    $firstDiffPathCounts = @{}
    $missingUsage = 0
    $request2PlusHit = [int64]0
    $request2PlusMiss = [int64]0
    $request2PlusCount = 0
    $prefixComparisonCount = 0
    $prefixPreservedCount = 0
    $zeroCacheHitCount = 0
    $cacheWarmupCandidateCount = 0
    $sameShapeZeroHitCount = 0
    $toolChoiceTransitionCount = 0
    $cacheShapeTransitionCount = 0
    $seenCacheShapes = @{}
    $previousCacheShapeHash = ""
    foreach ($shape in @($shapes.Values | Sort-Object -Property request_index)) {
        $requestId = [string]$shape.request_id
        $terminal = if ($terminals.ContainsKey($requestId)) { $terminals[$requestId] } else { $null }
        $inputTokens = if ($null -ne $terminal) { Get-TaskspaceCostProperty $terminal @("input_tokens") } else { $null }
        $cachedTokens = if ($null -ne $terminal) { Get-TaskspaceCostProperty $terminal @("cached_input_tokens") } else { $null }
        $uncachedTokens = $null
        if ($null -ne $inputTokens -and $null -ne $cachedTokens) {
            $uncachedTokens = [Math]::Max(0, [int64]$inputTokens - [int64]$cachedTokens)
        } else {
            $missingUsage++
        }
        $hitRate = if ($null -ne $cachedTokens -and $null -ne $uncachedTokens -and ([double]$cachedTokens + [double]$uncachedTokens) -gt 0) {
            [Math]::Round([double]$cachedTokens / ([double]$cachedTokens + [double]$uncachedTokens), 6)
        } else { $null }
        $toolsCount = Convert-TaskspaceTraceInt $shape.tools_count
        $classifier = if ($toolsCount -gt 0) { "native_tools_schema_hot_path" } else { "tool_free_action_contract" }
        Add-TaskspaceCostCount $shapeCounts $classifier
        $requestIndex = Convert-TaskspaceTraceInt $shape.request_index
        $cacheShapeHash = [string]$shape.cache_shape_hash
        $sameCacheShapeSeenBefore = -not [string]::IsNullOrWhiteSpace($cacheShapeHash) -and $seenCacheShapes.ContainsKey($cacheShapeHash)
        $cacheHitClass = if ($null -eq $cachedTokens -or $null -eq $uncachedTokens) {
            "unavailable"
        } elseif ([int64]$cachedTokens -eq 0) {
            "zero"
        } elseif ([int64]$uncachedTokens -eq 0) {
            "full"
        } else {
            "partial"
        }
        $cacheWarmupCandidate = $cacheHitClass -eq "zero" -and -not $sameCacheShapeSeenBefore
        $sameShapeZeroHit = $cacheHitClass -eq "zero" -and $sameCacheShapeSeenBefore
        if ($cacheHitClass -eq "zero") { $zeroCacheHitCount++ }
        if ($cacheWarmupCandidate) { $cacheWarmupCandidateCount++ }
        if ($sameShapeZeroHit) { $sameShapeZeroHitCount++ }
        if ([bool]$shape.tool_choice_changed) { $toolChoiceTransitionCount++ }
        if ($requestIndex -ge 2 -and $cacheShapeHash -ne $previousCacheShapeHash) {
            $cacheShapeTransitionCount++
        }
        if ($requestIndex -ge 2 -and $null -ne $cachedTokens -and $null -ne $uncachedTokens) {
            $request2PlusHit += [int64]$cachedTokens
            $request2PlusMiss += [int64]$uncachedTokens
            $request2PlusCount++
        }
        if ($requestIndex -ge 2) {
            $prefixComparisonCount++
            if ([bool]$shape.prefix_preserved) { $prefixPreservedCount++ }
            $firstDiffPath = [string]$shape.first_diff_path
            if (-not [string]::IsNullOrWhiteSpace($firstDiffPath)) {
                Add-TaskspaceCostCount $firstDiffPathCounts $firstDiffPath
            }
        }
        $events.Add([pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceV3"
            request_id = $requestId
            logical_request_id = $requestId
            model_request_index = $requestIndex
            attempt_seq = 1
            request_phase = "transport_observed"
            task_id = ""
            map_id = ""
            node_id = ""
            provider_wire_api = [string]$shape.provider_wire_api
            transport = "responses_http"
            tools_count = $toolsCount
            tools_present = ($toolsCount -gt 0)
            request_shape_classifier = $classifier
            stable_prefix_hash = $cacheShapeHash
            dynamic_suffix_hash = ""
            messages_hash = [string]$shape.messages_hash
            tools_hash = [string]$shape.tools_hash
            cache_shape_hash = $cacheShapeHash
            tool_choice_kind = [string]$shape.tool_choice_kind
            tool_choice_name = [string]$shape.tool_choice_name
            provider_payload_sha256 = [string]$shape.provider_payload_sha256
            pre_wire_payload_sha256 = [string]$shape.pre_wire_payload_sha256
            provider_payload_bytes = Convert-TaskspaceTraceInt $shape.provider_payload_bytes
            epoch_id = [string]$shape.epoch_id
            previous_request_id = [string]$shape.previous_request_id
            message_count = Convert-TaskspaceTraceInt $shape.message_count
            message_shapes = @($shape.message_shapes)
            lcp_message_count = Convert-TaskspaceTraceInt $shape.lcp_message_count
            lcp_message_bytes = Convert-TaskspaceTraceInt $shape.lcp_message_bytes
            message_prefix_preserved = if ($requestIndex -ge 2) { [bool]$shape.message_prefix_preserved } else { $null }
            tool_choice_preserved = if ($requestIndex -ge 2) { [bool]$shape.tool_choice_preserved } else { $null }
            tool_choice_changed = if ($requestIndex -ge 2) { [bool]$shape.tool_choice_changed } else { $null }
            prefix_preserved = if ($requestIndex -ge 2) { [bool]$shape.prefix_preserved } else { $null }
            first_diff_index = Get-TaskspaceCostProperty $shape @("first_diff_index")
            first_diff_path = [string]$shape.first_diff_path
            input_tokens = $inputTokens
            cached_input_tokens = $cachedTokens
            uncached_input_tokens = $uncachedTokens
            hit_rate = $hitRate
            cache_hit_class = $cacheHitClass
            same_cache_shape_seen_before = [bool]$sameCacheShapeSeenBefore
            cache_warmup_candidate = [bool]$cacheWarmupCandidate
            same_shape_zero_hit = [bool]$sameShapeZeroHit
            status = if ($null -ne $terminal) { [string]$terminal.status } else { "terminal_missing" }
        })
        if (-not [string]::IsNullOrWhiteSpace($cacheShapeHash)) {
            $seenCacheShapes[$cacheShapeHash] = $true
        }
        $previousCacheShapeHash = $cacheShapeHash
    }
    $count = [int]$events.Count
    $covered = @($events.ToArray() | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_.provider_payload_sha256) -and
        [string]$_.status -ne "terminal_missing"
    }).Count
    $request2PlusDenominator = [double]$request2PlusHit + [double]$request2PlusMiss
    [pscustomobject]@{
        provider_cache_trace_events = @($events.ToArray())
        provider_cache_trace_summary = [pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceSummaryV3"
            source = "provider_final_wire_trace"
            provider_request_count = $count
            trace_coverage = if ($count -gt 0) { [Math]::Round([double]$covered / [double]$count, 6) } else { 0.0 }
            cache_usage_missing_count = [int]$missingUsage
            request_shape_counts = Convert-TaskspaceCostTable $shapeCounts
            native_tools_schema_hot_path_count = if ($shapeCounts.ContainsKey("native_tools_schema_hot_path")) { [int]$shapeCounts["native_tools_schema_hot_path"] } else { 0 }
            tool_free_action_contract_count = if ($shapeCounts.ContainsKey("tool_free_action_contract")) { [int]$shapeCounts["tool_free_action_contract"] } else { 0 }
            unknown_or_unclassified_count = 0
            request_2_plus_count = [int]$request2PlusCount
            request_2_plus_cached_input_tokens = [int64]$request2PlusHit
            request_2_plus_uncached_input_tokens = [int64]$request2PlusMiss
            request_2_plus_hit_rate = if ($request2PlusDenominator -gt 0) { [Math]::Round([double]$request2PlusHit / $request2PlusDenominator, 6) } else { $null }
            prefix_comparison_count = [int]$prefixComparisonCount
            prefix_preserved_count = [int]$prefixPreservedCount
            prefix_preserved_rate = if ($prefixComparisonCount -gt 0) { [Math]::Round([double]$prefixPreservedCount / [double]$prefixComparisonCount, 6) } else { $null }
            first_diff_path_counts = Convert-TaskspaceCostTable $firstDiffPathCounts
            zero_cache_hit_count = [int]$zeroCacheHitCount
            cache_warmup_candidate_count = [int]$cacheWarmupCandidateCount
            same_shape_zero_hit_count = [int]$sameShapeZeroHitCount
            tool_choice_transition_count = [int]$toolChoiceTransitionCount
            cache_shape_transition_count = [int]$cacheShapeTransitionCount
        }
    }
}

function New-TaskspaceProviderCacheTraceArtifacts {
    param([object[]]$BudgetEvents, [AllowEmptyString()][string]$ProviderWireTracePath = "")
    if (-not [string]::IsNullOrWhiteSpace($ProviderWireTracePath) -and
        (Test-Path -LiteralPath $ProviderWireTracePath) -and
        (Get-Item -LiteralPath $ProviderWireTracePath).Length -gt 0) {
        return New-TaskspaceProviderWireCacheTraceArtifacts $ProviderWireTracePath
    }
    $terminalStatuses = @("response_completed", "response_failed", "cancelled")
    $events = New-Object System.Collections.Generic.List[object]
    $shapeCounts = @{}
    $missingUsage = 0
    $request2PlusHit = [int64]0
    $request2PlusMiss = [int64]0
    $request2PlusCount = 0
    foreach ($event in @($BudgetEvents)) {
        if ([string]$event.status -notin $terminalStatuses) { continue }
        if ([string]::IsNullOrWhiteSpace([string]$event.request_id)) { continue }
        $inputTokens = Get-TaskspaceCostProperty $event @("input_tokens")
        $cachedTokens = Get-TaskspaceCostProperty $event @("cached_input_tokens")
        $uncachedTokens = $null
        if ($null -ne $inputTokens -and $null -ne $cachedTokens) {
            $uncachedTokens = [Math]::Max(0, [int64]$inputTokens - [int64]$cachedTokens)
        } else {
            $missingUsage++
        }
        $hitRate = if ($null -ne $cachedTokens -and $null -ne $uncachedTokens -and ([double]$cachedTokens + [double]$uncachedTokens) -gt 0) {
            [Math]::Round([double]$cachedTokens / ([double]$cachedTokens + [double]$uncachedTokens), 6)
        } else {
            $null
        }
        $shape = [string]$event.request_shape_classifier
        if ([string]::IsNullOrWhiteSpace($shape)) {
            $shape = if ([bool]$event.tools_present -or [int]$event.tools_count -gt 0) { "native_tools_schema_hot_path" } else { "unknown_or_unclassified" }
        }
        Add-TaskspaceCostCount $shapeCounts $shape
        $attemptSeq = Convert-TaskspaceTraceInt $event.attempt_seq
        $modelRequestIndex = Convert-TaskspaceTraceInt $event.request_count_after
        if ($modelRequestIndex -ge 2 -and $null -ne $cachedTokens -and $null -ne $uncachedTokens) {
            $request2PlusHit += [int64]$cachedTokens
            $request2PlusMiss += [int64]$uncachedTokens
            $request2PlusCount++
        }
        $events.Add([pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceV1"
            request_id = [string]$event.request_id
            logical_request_id = [string]$event.logical_request_id
            model_request_index = $modelRequestIndex
            attempt_seq = $attemptSeq
            request_phase = [string]$event.request_phase
            task_id = [string]$event.task_id
            map_id = [string]$event.map_id
            node_id = [string]$event.node_id
            provider_wire_api = [string]$event.provider_wire_api
            transport = [string]$event.transport
            tools_count = [int]$event.tools_count
            tools_present = [bool]$event.tools_present
            request_shape_classifier = $shape
            stable_prefix_hash = [string]$event.stable_prefix_hash
            dynamic_suffix_hash = [string]$event.dynamic_suffix_hash
            messages_hash = [string]$event.messages_hash
            provider_payload_sha256 = [string]$event.provider_payload_sha256
            input_tokens = $inputTokens
            cached_input_tokens = $cachedTokens
            uncached_input_tokens = $uncachedTokens
            hit_rate = $hitRate
            status = [string]$event.status
        })
    }
    $count = [int]$events.Count
    $covered = @($events.ToArray() | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_.request_shape_classifier) -and
        -not [string]::IsNullOrWhiteSpace([string]$_.provider_payload_sha256)
    }).Count
    $request2PlusDenominator = [double]$request2PlusHit + [double]$request2PlusMiss
    [pscustomobject]@{
        provider_cache_trace_events = @($events.ToArray())
        provider_cache_trace_summary = [pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceSummaryV1"
            provider_request_count = $count
            trace_coverage = if ($count -gt 0) { [Math]::Round([double]$covered / [double]$count, 6) } else { 0.0 }
            cache_usage_missing_count = [int]$missingUsage
            request_shape_counts = Convert-TaskspaceCostTable $shapeCounts
            native_tools_schema_hot_path_count = if ($shapeCounts.ContainsKey("native_tools_schema_hot_path")) { [int]$shapeCounts["native_tools_schema_hot_path"] } else { 0 }
            tool_free_action_contract_count = if ($shapeCounts.ContainsKey("tool_free_action_contract")) { [int]$shapeCounts["tool_free_action_contract"] } else { 0 }
            unknown_or_unclassified_count = if ($shapeCounts.ContainsKey("unknown_or_unclassified")) { [int]$shapeCounts["unknown_or_unclassified"] } else { 0 }
            request_2_plus_count = [int]$request2PlusCount
            request_2_plus_cached_input_tokens = [int64]$request2PlusHit
            request_2_plus_uncached_input_tokens = [int64]$request2PlusMiss
            request_2_plus_hit_rate = if ($request2PlusDenominator -gt 0) { [Math]::Round([double]$request2PlusHit / $request2PlusDenominator, 6) } else { $null }
        }
    }
}

function Test-TaskspaceProviderCacheTraceTaskspaceArtifact {
    param([Parameter(Mandatory = $true)][string]$SummaryPath)
    $artifactDir = Split-Path -Parent $SummaryPath
    $metricsPath = Join-Path $artifactDir "metrics.json"
    $metric = $null
    if (Test-Path -LiteralPath $metricsPath) {
        try { $metric = Get-Content -Raw -Encoding UTF8 -LiteralPath $metricsPath | ConvertFrom-Json } catch { $metric = $null }
    }
    if ($metric -and $metric.PSObject.Properties.Name -contains "logical_mode") {
        return ([string]$metric.logical_mode -eq "taskspace")
    }
    return ($SummaryPath -match '(?i)[\\/]+right[\\/]+artifacts[\\/]+provider-cache-trace-summary\.json$')
}

function Add-TaskspaceProviderCacheShapeCounts {
    param($ShapeCounts, $RequestShapeCounts)
    if (-not $RequestShapeCounts) { return }
    foreach ($property in @($RequestShapeCounts.PSObject.Properties)) {
        $name = [string]$property.Name
        if ([string]::IsNullOrWhiteSpace($name)) { continue }
        if (-not $ShapeCounts.ContainsKey($name)) { $ShapeCounts[$name] = 0 }
        $ShapeCounts[$name] = [int]$ShapeCounts[$name] + [int]$property.Value
    }
}

function New-TaskspaceProviderCacheTraceAggregateArtifacts {
    param([Parameter(Mandatory = $true)][string]$RootDir)
    $rootSummaryPath = Join-Path $RootDir "provider-cache-trace-summary.json"
    $rootTracePath = Join-Path $RootDir "provider-cache-trace.jsonl"
    $summaryFiles = @(Get-ChildItem -LiteralPath $RootDir -Filter "provider-cache-trace-summary.json" -Recurse -ErrorAction SilentlyContinue |
        Where-Object {
            [System.IO.Path]::GetFullPath($_.FullName) -ne [System.IO.Path]::GetFullPath($rootSummaryPath) -and
            (Test-TaskspaceProviderCacheTraceTaskspaceArtifact $_.FullName)
        } |
        Sort-Object FullName)
    $shapeCounts = @{}
    $providerRequestCount = 0
    $coveredCount = 0
    $missingUsage = 0
    $request2PlusCount = 0
    $request2PlusHit = [int64]0
    $request2PlusMiss = [int64]0
    $toolFreeCount = 0
    $nativeToolsCount = 0
    $unknownCount = 0
    $zeroCacheHitCount = 0
    $cacheWarmupCandidateCount = 0
    $sameShapeZeroHitCount = 0
    $toolChoiceTransitionCount = 0
    $cacheShapeTransitionCount = 0
    $eventLines = New-Object System.Collections.Generic.List[string]
    foreach ($file in $summaryFiles) {
        try { $summary = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json } catch { continue }
        $count = if ($summary.PSObject.Properties.Name -contains "provider_request_count") { [int]$summary.provider_request_count } else { 0 }
        $providerRequestCount += $count
        $coverage = if ($summary.PSObject.Properties.Name -contains "trace_coverage") { [double]$summary.trace_coverage } else { 0.0 }
        $coveredCount += [int][Math]::Round($coverage * [double]$count)
        if ($summary.PSObject.Properties.Name -contains "cache_usage_missing_count") { $missingUsage += [int]$summary.cache_usage_missing_count }
        if ($summary.PSObject.Properties.Name -contains "request_2_plus_count") { $request2PlusCount += [int]$summary.request_2_plus_count }
        if ($summary.PSObject.Properties.Name -contains "request_2_plus_cached_input_tokens") { $request2PlusHit += [int64]$summary.request_2_plus_cached_input_tokens }
        if ($summary.PSObject.Properties.Name -contains "request_2_plus_uncached_input_tokens") { $request2PlusMiss += [int64]$summary.request_2_plus_uncached_input_tokens }
        if ($summary.PSObject.Properties.Name -contains "native_tools_schema_hot_path_count") { $nativeToolsCount += [int]$summary.native_tools_schema_hot_path_count }
        if ($summary.PSObject.Properties.Name -contains "tool_free_action_contract_count") { $toolFreeCount += [int]$summary.tool_free_action_contract_count }
        if ($summary.PSObject.Properties.Name -contains "unknown_or_unclassified_count") { $unknownCount += [int]$summary.unknown_or_unclassified_count }
        if ($summary.PSObject.Properties.Name -contains "zero_cache_hit_count") { $zeroCacheHitCount += [int]$summary.zero_cache_hit_count }
        if ($summary.PSObject.Properties.Name -contains "cache_warmup_candidate_count") { $cacheWarmupCandidateCount += [int]$summary.cache_warmup_candidate_count }
        if ($summary.PSObject.Properties.Name -contains "same_shape_zero_hit_count") { $sameShapeZeroHitCount += [int]$summary.same_shape_zero_hit_count }
        if ($summary.PSObject.Properties.Name -contains "tool_choice_transition_count") { $toolChoiceTransitionCount += [int]$summary.tool_choice_transition_count }
        if ($summary.PSObject.Properties.Name -contains "cache_shape_transition_count") { $cacheShapeTransitionCount += [int]$summary.cache_shape_transition_count }
        if ($summary.PSObject.Properties.Name -contains "request_shape_counts") {
            Add-TaskspaceProviderCacheShapeCounts $shapeCounts $summary.request_shape_counts
        }
        $tracePath = Join-Path (Split-Path -Parent $file.FullName) "provider-cache-trace.jsonl"
        if (Test-Path -LiteralPath $tracePath) {
            foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $tracePath)) {
                if (-not [string]::IsNullOrWhiteSpace($line)) { [void]$eventLines.Add($line) }
            }
        }
    }
    $denominator = [double]$request2PlusHit + [double]$request2PlusMiss
    [pscustomobject]@{
        provider_cache_trace_events = @($eventLines.ToArray())
        provider_cache_trace_summary = [pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceSummaryV3"
            provider_request_count = [int]$providerRequestCount
            trace_coverage = if ($providerRequestCount -gt 0) { [Math]::Round([double]$coveredCount / [double]$providerRequestCount, 6) } else { 0.0 }
            cache_usage_missing_count = [int]$missingUsage
            request_shape_counts = Convert-TaskspaceCostTable $shapeCounts
            native_tools_schema_hot_path_count = [int]$nativeToolsCount
            tool_free_action_contract_count = [int]$toolFreeCount
            unknown_or_unclassified_count = [int]$unknownCount
            request_2_plus_count = [int]$request2PlusCount
            request_2_plus_cached_input_tokens = [int64]$request2PlusHit
            request_2_plus_uncached_input_tokens = [int64]$request2PlusMiss
            request_2_plus_hit_rate = if ($denominator -gt 0) { [Math]::Round([double]$request2PlusHit / $denominator, 6) } else { $null }
            zero_cache_hit_count = [int]$zeroCacheHitCount
            cache_warmup_candidate_count = [int]$cacheWarmupCandidateCount
            same_shape_zero_hit_count = [int]$sameShapeZeroHitCount
            tool_choice_transition_count = [int]$toolChoiceTransitionCount
            cache_shape_transition_count = [int]$cacheShapeTransitionCount
        }
    }
}

function New-TaskspaceStateCommitDisplacementSummary {
    param([string]$ObservabilityJsonPath)
    $events = New-Object System.Collections.Generic.List[object]
    $attemptEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("legacy_state_action_attempt"))) {
        $tags = Convert-TaskspaceTraceTags $event
        if ([string]$tags.producer -ne "runtime") { continue }
        $attemptEvents.Add([pscustomobject]@{
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            action = [string]$tags.action
            displaced = Convert-TaskspaceTraceBool $tags.displaced $false
            allowed = Convert-TaskspaceTraceBool $tags.allowed $false
            reason = [string]$tags.reason
            producer = [string]$tags.producer
        })
    }
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("state_commit_displacement"))) {
        $tags = Convert-TaskspaceTraceTags $event
        if ([string]$tags.producer -ne "runtime") { continue }
        $events.Add([pscustomobject]@{
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            commit_id = [string]$tags.commit_id
            status = [string]$tags.status
            accepted_section_count = Convert-TaskspaceTraceInt $tags.accepted_section_count
            rejected_section_count = Convert-TaskspaceTraceInt $tags.rejected_section_count
            state_commit_section_count = Convert-TaskspaceTraceInt $tags.state_commit_section_count
            state_commit_count = Convert-TaskspaceTraceInt $tags.state_commit_count
            model_visible_state_commit_count = Convert-TaskspaceTraceInt $tags.model_visible_state_commit_count
            runtime_synthesized_state_commit_count = Convert-TaskspaceTraceInt $tags.runtime_synthesized_state_commit_count
            legacy_state_action_attempt_count = Convert-TaskspaceTraceInt $tags.legacy_state_action_attempt_count
            legacy_state_action_displaced_count = Convert-TaskspaceTraceInt $tags.legacy_state_action_displaced_count
            legacy_state_action_count = Convert-TaskspaceTraceInt $tags.legacy_state_action_count
            legacy_state_action_budget = Convert-TaskspaceTraceInt $tags.legacy_state_action_budget
            producer = [string]$tags.producer
        })
    }
    $acceptedStateCommitEvents = @($events.ToArray() | Where-Object { [string]$_.status -eq "accepted" -or [string]$_.status -eq "partial" })
    $stateCommitCount = [int](@($acceptedStateCommitEvents | Measure-Object -Property state_commit_count -Sum).Sum)
    $stateCommitSectionCount = [int](@($acceptedStateCommitEvents | Measure-Object -Property state_commit_section_count -Sum).Sum)
    $modelVisibleStateCommitCount = [int](@($acceptedStateCommitEvents | Measure-Object -Property model_visible_state_commit_count -Sum).Sum)
    $runtimeSynthesizedStateCommitCount = [int](@($acceptedStateCommitEvents | Measure-Object -Property runtime_synthesized_state_commit_count -Sum).Sum)
    $legacyStateActionAttemptCount = [int]$attemptEvents.Count
    $legacyStateActionDisplacedCount = [int](@($attemptEvents.ToArray() | Where-Object { [bool]$_.displaced }).Count)
    $legacyStateActionCount = [int](@($attemptEvents.ToArray() | Where-Object { [bool]$_.allowed }).Count)
    $legacyStateActionBudget = if ($events.Count -gt 0) { [int](@($events.ToArray() | Measure-Object -Property legacy_state_action_budget -Maximum).Maximum) } else { 0 }
    $sourceStatus = if ($events.Count -gt 0) { "runtime" } else { "missing_runtime" }
    $hasDisplacementDenominator = $legacyStateActionAttemptCount -gt 0
    $displacementRate = if ($legacyStateActionAttemptCount -gt 0) {
        [Math]::Round(($legacyStateActionDisplacedCount / [double]$legacyStateActionAttemptCount), 4)
    } else {
        [double]0
    }
    [pscustomobject]@{
        schema_version = "taskspace-state-commit-displacement-v1"
        status = if ($sourceStatus -eq "runtime" -and $stateCommitCount -gt 0 -and $hasDisplacementDenominator -and $legacyStateActionDisplacedCount -ge $legacyStateActionAttemptCount -and $legacyStateActionCount -le $legacyStateActionBudget) { "pass" } else { "fail" }
        source_status = $sourceStatus
        producer = "runtime"
        has_displacement_denominator = [bool]$hasDisplacementDenominator
        model_visible_state_commit_count = [int]$modelVisibleStateCommitCount
        runtime_synthesized_state_commit_count = [int]$runtimeSynthesizedStateCommitCount
        legacy_state_action_attempt_count = [int]$legacyStateActionAttemptCount
        legacy_state_action_displaced_count = [int]$legacyStateActionDisplacedCount
        legacy_state_action_displacement_rate = [double]$displacementRate
        legacy_state_action_count = [int]$legacyStateActionCount
        legacy_state_action_budget = [int]$legacyStateActionBudget
        state_commit_count = [int]$stateCommitCount
        state_commit_section_count = [int]$stateCommitSectionCount
        runtime_state_commit_count = [int]$stateCommitCount
        taskspace_control_count = [int]$stateCommitCount
        runtime_event_count = [int]$events.Count
        runtime_events = @($events.ToArray())
        legacy_state_action_attempt_event_count = [int]$attemptEvents.Count
        legacy_state_action_attempt_events = @($attemptEvents.ToArray())
    }
}

function New-TaskspaceSpawnNodeBudgetSummary {
    param([string]$ObservabilityJsonPath, [AllowEmptyString()][string]$RolloutJsonlPath = "")
    $events = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("spawn_node_budget") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        if ([string]$tags.producer -ne "runtime") { continue }
        $events.Add([pscustomobject]@{
            trace_event_id = [string](Get-TaskspaceTraceField $event @("trace_event_id", "id"))
            task_id = [string](Get-TaskspaceTraceField $event @("task_id"))
            map_id = [string](Get-TaskspaceTraceField $event @("map_id"))
            node_id = [string](Get-TaskspaceTraceField $event @("node_id"))
            budget_kind = [string]$tags.budget_kind
            action = [string]$tags.action
            status = [string]$tags.status
            active_budget_source = [string]$tags.active_budget_source
            route_mode = [string]$tags.route_mode
            profile_name = [string]$tags.profile_name
            spawn_agent_call_count_after = Convert-TaskspaceTraceInt $tags.spawn_agent_call_count_after
            max_spawn_agent_calls = Convert-TaskspaceTraceInt $tags.max_spawn_agent_calls
            node_count = Convert-TaskspaceTraceInt $tags.node_count
            node_count_after = Convert-TaskspaceTraceInt $tags.node_count_after
            max_nodes = Convert-TaskspaceTraceInt $tags.max_nodes
            budget_response_action_taken = Convert-TaskspaceTraceBool $tags.budget_response_action_taken
            producer = [string]$tags.producer
        })
    }
    $runtimeEvents = @($events.ToArray())
    $spawnEvents = @($runtimeEvents | Where-Object { [string]$_.budget_kind -eq "spawn" })
    $nodeEvents = @($runtimeEvents | Where-Object { [string]$_.budget_kind -eq "node" })
    $blockedEvents = @($runtimeEvents | Where-Object { [string]$_.status -eq "blocked" })
    $invalidBlockedEvents = @($blockedEvents | Where-Object { -not [bool]$_.budget_response_action_taken })
    $spawnCount = if ($spawnEvents.Count -gt 0) { [int](@($spawnEvents | Measure-Object -Property spawn_agent_call_count_after -Maximum).Maximum) } else { 0 }
    $maxSpawnAgentCalls = if ($spawnEvents.Count -gt 0) { [int](@($spawnEvents | Measure-Object -Property max_spawn_agent_calls -Maximum).Maximum) } else { 0 }
    $nodeCountFromCreate = if ($nodeEvents.Count -gt 0) { [int](@($nodeEvents | Measure-Object -Property node_count_after -Maximum).Maximum) } else { 0 }
    $nodeCountFromSpawn = if ($spawnEvents.Count -gt 0) { [int](@($spawnEvents | Measure-Object -Property node_count -Maximum).Maximum) } else { 0 }
    $nodeCount = [Math]::Max($nodeCountFromCreate, $nodeCountFromSpawn)
    $maxNodes = if ($runtimeEvents.Count -gt 0) { [int](@($runtimeEvents | Measure-Object -Property max_nodes -Maximum).Maximum) } else { 0 }
    $overProfileHint = ($runtimeEvents.Count -gt 0 -and (($maxSpawnAgentCalls -ge 0 -and $spawnCount -gt $maxSpawnAgentCalls) -or ($maxNodes -ge 0 -and $nodeCount -gt $maxNodes)))
    $sourceStatus = if ($runtimeEvents.Count -gt 0) { "runtime" } else { "missing_runtime" }
    $reviewDebt = Get-TaskspaceSubagentReviewDebt $ObservabilityJsonPath
    [pscustomobject]@{
        schema_version = "taskspace-spawn-node-budget-summary-v1"
        status = if ($sourceStatus -eq "runtime" -and $blockedEvents.Count -eq 0 -and [int]$reviewDebt.unreviewed_subagent_result_count -eq 0) { "pass" } else { "fail" }
        within_budget_status = if ($sourceStatus -ne "runtime") { "missing_runtime" } elseif ($overProfileHint) { "over_profile_hint" } else { "within_profile_hint" }
        over_budget_enforcement_status = if ($blockedEvents.Count -eq 0) { "advisory_only" } else { "blocked_event_observed" }
        subagent_review_debt_status = [string]$reviewDebt.review_debt_status
        subagent_review_source_status = [string]$reviewDebt.source_status
        source_status = $sourceStatus
        producer = "runtime"
        active_budget_source = if ($runtimeEvents.Count -gt 0) { [string](Get-TaskspaceCostProperty $runtimeEvents[0] @("active_budget_source")) } else { "" }
        route_mode = if ($runtimeEvents.Count -gt 0) { [string](Get-TaskspaceCostProperty $runtimeEvents[0] @("route_mode")) } else { "" }
        spawn_agent_call_count = [int]$spawnCount
        max_spawn_agent_calls = [int]$maxSpawnAgentCalls
        node_count = [int]$nodeCount
        max_nodes = [int]$maxNodes
        over_profile_hint = [bool]$overProfileHint
        subagent_result_count = [int]$reviewDebt.subagent_result_count
        reviewed_subagent_result_count = [int]$reviewDebt.reviewed_subagent_result_count
        unreviewed_subagent_result_count = [int]$reviewDebt.unreviewed_subagent_result_count
        runtime_event_count = [int]$runtimeEvents.Count
        blocked_budget_event_count = [int]$blockedEvents.Count
        invalid_blocked_budget_event_count = [int]$invalidBlockedEvents.Count
        unreviewed_subagent_results = @($reviewDebt.unreviewed_subagent_results)
        runtime_events = @($runtimeEvents)
    }
}

function Get-TaskspaceTokenUsageObjects {
    param($Value)
    $found = New-Object System.Collections.Generic.List[object]
    function Visit-UsageValue($Current) {
        if ($null -eq $Current) { return }
        if ($Current -is [string] -or $Current -is [ValueType]) { return }
        $names = @($Current.PSObject.Properties.Name)
        $hasInput = @("input_tokens", "prompt_tokens") | Where-Object { $names -contains $_ }
        $hasOutput = @("output_tokens", "completion_tokens") | Where-Object { $names -contains $_ }
        if ($names -contains "usage" -and $null -ne $Current.usage) {
            Visit-UsageValue $Current.usage
        }
        if ($hasInput.Count -gt 0 -or $hasOutput.Count -gt 0) {
            $found.Add($Current)
            return
        }
        foreach ($prop in @($Current.PSObject.Properties)) {
            if ($prop.Name -eq "usage") { continue }
            if ($prop.Value -is [System.Collections.IEnumerable] -and -not ($prop.Value -is [string])) {
                foreach ($item in @($prop.Value)) { Visit-UsageValue $item }
            } else {
                Visit-UsageValue $prop.Value
            }
        }
    }
    Visit-UsageValue $Value
    @($found.ToArray())
}

function Get-TaskspaceUsageNumber {
    param($Usage, [string[]]$Names)
    $value = Get-TaskspaceCostProperty $Usage $Names
    if ($null -eq $value -or [string]::IsNullOrWhiteSpace([string]$value)) { return $null }
    try { return [int64]$value } catch { return $null }
}

function Get-TaskspaceCachedInputTokens {
    param($Usage)
    $direct = Get-TaskspaceUsageNumber $Usage @("cached_input_tokens", "cached_prompt_tokens")
    if ($null -ne $direct) { return $direct }
    $details = Get-TaskspaceCostProperty $Usage @("input_tokens_details", "prompt_tokens_details")
    if ($details) { return Get-TaskspaceUsageNumber $details @("cached_tokens") }
    $null
}

function Get-TaskspacePercentile {
    param([object[]]$Values, [double]$Percentile)
    $numbers = @($Values | Where-Object { $null -ne $_ } | ForEach-Object { [int64]$_ } | Sort-Object)
    if ($numbers.Count -eq 0) { return $null }
    if ($numbers.Count -eq 1) { return [int64]$numbers[0] }
    $rank = [Math]::Ceiling(([double]$Percentile / 100.0) * [double]$numbers.Count)
    $index = [Math]::Max(0, [Math]::Min($numbers.Count - 1, [int]$rank - 1))
    [int64]$numbers[$index]
}

function New-TaskspaceTokenSummary {
    param([string]$JsonlPath)
    $parsed = Get-TaskspaceCostJsonlRows $JsonlPath
    $inputTotal = [int64]0
    $outputTotal = [int64]0
    $cachedTotal = [int64]0
    $inputValues = New-Object System.Collections.Generic.List[Int64]
    $outputValues = New-Object System.Collections.Generic.List[Int64]
    $cachedValues = New-Object System.Collections.Generic.List[Int64]
    $usageCount = 0
    $missingInput = 0
    $missingOutput = 0
    $missingCached = 0
    foreach ($row in @($parsed.rows)) {
        foreach ($usage in @(Get-TaskspaceTokenUsageObjects $row)) {
            $usageCount++
            $input = Get-TaskspaceUsageNumber $usage @("input_tokens", "prompt_tokens")
            $output = Get-TaskspaceUsageNumber $usage @("output_tokens", "completion_tokens")
            $cached = Get-TaskspaceCachedInputTokens $usage
            if ($null -eq $input) { $missingInput++ } else { $inputTotal += $input; $inputValues.Add([int64]$input) }
            if ($null -eq $output) { $missingOutput++ } else { $outputTotal += $output; $outputValues.Add([int64]$output) }
            if ($null -eq $cached) { $missingCached++ } else { $cachedTotal += $cached; $cachedValues.Add([int64]$cached) }
        }
    }
    $status = if ($parsed.source_status -ne "read") { "source_missing" } elseif ($usageCount -eq 0) { "usage_unavailable" } elseif ($missingInput -gt 0 -or $missingOutput -gt 0) { "partial" } else { "measured" }
    $inputArray = @($inputValues.ToArray())
    $outputArray = @($outputValues.ToArray())
    $cachedArray = @($cachedValues.ToArray())
    [pscustomobject]@{
        schema_version = "taskspace-token-summary-v1"
        source_path = $JsonlPath
        source_status = [string]$parsed.source_status
        parse_errors = [int]$parsed.parse_errors
        parse_status = if ($parsed.parse_errors -gt 0) { "partial" } else { "ok" }
        availability = $status
        model_request_count = if ($usageCount -gt 0) { [int]$usageCount } else { $null }
        input_tokens = if ($usageCount -gt 0 -and $missingInput -lt $usageCount) { $inputTotal } else { $null }
        output_tokens = if ($usageCount -gt 0 -and $missingOutput -lt $usageCount) { $outputTotal } else { $null }
        cached_input_tokens = if ($usageCount -gt 0 -and $missingCached -lt $usageCount) { $cachedTotal } else { $null }
        uncached_input_tokens = if ($usageCount -gt 0 -and $missingInput -lt $usageCount -and $missingCached -lt $usageCount) { [Math]::Max(0, $inputTotal - $cachedTotal) } else { $null }
        request_distribution = [pscustomobject]@{
            max_input_tokens = if ($inputArray.Count -gt 0) { [int64]($inputArray | Measure-Object -Maximum).Maximum } else { $null }
            p95_input_tokens = Get-TaskspacePercentile $inputArray 95
            first_input_tokens = if ($inputArray.Count -gt 0) { [int64]$inputArray[0] } else { $null }
            last_input_tokens = if ($inputArray.Count -gt 0) { [int64]$inputArray[$inputArray.Count - 1] } else { $null }
            max_output_tokens = if ($outputArray.Count -gt 0) { [int64]($outputArray | Measure-Object -Maximum).Maximum } else { $null }
            p95_output_tokens = Get-TaskspacePercentile $outputArray 95
            first_output_tokens = if ($outputArray.Count -gt 0) { [int64]$outputArray[0] } else { $null }
            last_output_tokens = if ($outputArray.Count -gt 0) { [int64]$outputArray[$outputArray.Count - 1] } else { $null }
            max_cached_input_tokens = if ($cachedArray.Count -gt 0) { [int64]($cachedArray | Measure-Object -Maximum).Maximum } else { $null }
            p95_cached_input_tokens = Get-TaskspacePercentile $cachedArray 95
        }
        missing_usage_fields = [pscustomobject]@{
            input_tokens = [int]$missingInput
            output_tokens = [int]$missingOutput
            cached_input_tokens = [int]$missingCached
        }
    }
}

function New-TaskspaceRolloutRequestTraceSummary {
    param([AllowEmptyString()][string]$RolloutJsonlPath = "")
    $sourceStatus = if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath) -and (Test-Path -LiteralPath $RolloutJsonlPath -PathType Leaf)) { "read" } else { "missing" }
    $parseErrors = 0
    $inputValues = New-Object System.Collections.Generic.List[Int64]
    $outputValues = New-Object System.Collections.Generic.List[Int64]
    $cachedValues = New-Object System.Collections.Generic.List[Int64]
    if ($sourceStatus -eq "read") {
        foreach ($line in [System.IO.File]::ReadLines($RolloutJsonlPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try { $row = $line | ConvertFrom-Json } catch { $parseErrors++; continue }
            if (-not ($row.PSObject.Properties.Name -contains "type" -and [string]$row.type -eq "event_msg")) { continue }
            if (-not ($row.PSObject.Properties.Name -contains "payload" -and $null -ne $row.payload)) { continue }
            if (-not ($row.payload.PSObject.Properties.Name -contains "type" -and [string]$row.payload.type -eq "token_count")) { continue }
            $info = Get-TaskspaceCostProperty $row.payload @("info")
            if (-not $info) { continue }
            $last = Get-TaskspaceCostProperty $info @("last_token_usage")
            if (-not $last) { continue }
            $input = Get-TaskspaceUsageNumber $last @("input_tokens", "prompt_tokens")
            $output = Get-TaskspaceUsageNumber $last @("output_tokens", "completion_tokens")
            $cached = Get-TaskspaceCachedInputTokens $last
            if ($null -ne $input) { $inputValues.Add([int64]$input) }
            if ($null -ne $output) { $outputValues.Add([int64]$output) }
            if ($null -ne $cached) { $cachedValues.Add([int64]$cached) }
        }
    }
    $inputArray = @($inputValues.ToArray())
    $outputArray = @($outputValues.ToArray())
    $cachedArray = @($cachedValues.ToArray())
    [pscustomobject]@{
        source_path = $RolloutJsonlPath
        source_status = $sourceStatus
        parse_errors = [int]$parseErrors
        availability = if ($sourceStatus -ne "read") { "source_missing" } elseif ($inputArray.Count -eq 0 -and $outputArray.Count -eq 0) { "usage_unavailable" } else { "measured" }
        model_request_count = if ($inputArray.Count -gt 0 -or $outputArray.Count -gt 0) { [Math]::Max($inputArray.Count, $outputArray.Count) } else { $null }
        input_tokens = if ($inputArray.Count -gt 0) { ($inputArray | Measure-Object -Sum).Sum } else { $null }
        output_tokens = if ($outputArray.Count -gt 0) { ($outputArray | Measure-Object -Sum).Sum } else { $null }
        cached_input_tokens = if ($cachedArray.Count -gt 0) { ($cachedArray | Measure-Object -Sum).Sum } else { $null }
        max_input_tokens_per_request = if ($inputArray.Count -gt 0) { [int64]($inputArray | Measure-Object -Maximum).Maximum } else { $null }
        p95_input_tokens_per_request = Get-TaskspacePercentile $inputArray 95
        first_input_tokens_per_request = if ($inputArray.Count -gt 0) { [int64]$inputArray[0] } else { $null }
        last_input_tokens_per_request = if ($inputArray.Count -gt 0) { [int64]$inputArray[$inputArray.Count - 1] } else { $null }
        max_output_tokens_per_request = if ($outputArray.Count -gt 0) { [int64]($outputArray | Measure-Object -Maximum).Maximum } else { $null }
        p95_output_tokens_per_request = Get-TaskspacePercentile $outputArray 95
        first_output_tokens_per_request = if ($outputArray.Count -gt 0) { [int64]$outputArray[0] } else { $null }
        last_output_tokens_per_request = if ($outputArray.Count -gt 0) { [int64]$outputArray[$outputArray.Count - 1] } else { $null }
        max_cached_input_tokens_per_request = if ($cachedArray.Count -gt 0) { [int64]($cachedArray | Measure-Object -Maximum).Maximum } else { $null }
        p95_cached_input_tokens_per_request = Get-TaskspacePercentile $cachedArray 95
    }
}

function New-TaskspaceRequestSummary {
    param([string]$JsonlPath, $TokenSummary, [AllowEmptyString()][string]$RolloutJsonlPath = "")
    $rolloutTrace = New-TaskspaceRolloutRequestTraceSummary $RolloutJsonlPath
    $requestTraceMeasured = [string]$rolloutTrace.availability -eq "measured" -and
        $null -ne $rolloutTrace.model_request_count -and
        [int]$rolloutTrace.model_request_count -gt 0
    $requestCount = if ($requestTraceMeasured) { [int]$rolloutTrace.model_request_count } else { $null }
    [pscustomobject]@{
        schema_version = "taskspace-request-summary-v1"
        source_path = $JsonlPath
        rollout_source_path = $RolloutJsonlPath
        availability = if ($requestTraceMeasured) { "measured" } else { "request_trace_unavailable" }
        model_request_count_source = if ($requestTraceMeasured) { "rollout_trace" } else { "unavailable" }
        model_request_count = $requestCount
        token_usage_record_count = $TokenSummary.model_request_count
        avg_input_tokens_per_request = if ($requestTraceMeasured -and $null -ne $rolloutTrace.input_tokens) { [Math]::Round([double]$rolloutTrace.input_tokens / [double]$requestCount, 4) } else { $null }
        avg_output_tokens_per_request = if ($requestTraceMeasured -and $null -ne $rolloutTrace.output_tokens) { [Math]::Round([double]$rolloutTrace.output_tokens / [double]$requestCount, 4) } else { $null }
        max_input_tokens_per_request = $rolloutTrace.max_input_tokens_per_request
        p95_input_tokens_per_request = $rolloutTrace.p95_input_tokens_per_request
        first_input_tokens_per_request = $rolloutTrace.first_input_tokens_per_request
        last_input_tokens_per_request = $rolloutTrace.last_input_tokens_per_request
        max_output_tokens_per_request = $rolloutTrace.max_output_tokens_per_request
        p95_output_tokens_per_request = $rolloutTrace.p95_output_tokens_per_request
        first_output_tokens_per_request = $rolloutTrace.first_output_tokens_per_request
        last_output_tokens_per_request = $rolloutTrace.last_output_tokens_per_request
        rollout_trace = $rolloutTrace
        parse_status = [string]$TokenSummary.parse_status
        parse_errors = [int]$TokenSummary.parse_errors
    }
}

function New-TaskspaceProviderInputVisibilitySummary {
    param([string]$JsonlPath, $TokenSummary)
    $jsonlBytes = $null
    if (-not [string]::IsNullOrWhiteSpace($JsonlPath) -and (Test-Path -LiteralPath $JsonlPath)) {
        $jsonlBytes = [int64](Get-Item -LiteralPath $JsonlPath).Length
    }
    $jsonlKb = if ($null -ne $jsonlBytes -and [int64]$jsonlBytes -gt 0) {
        [double]$jsonlBytes / 1024.0
    } else {
        $null
    }
    $inputTokens = if ($TokenSummary -and $TokenSummary.PSObject.Properties.Name -contains "input_tokens") { $TokenSummary.input_tokens } else { $null }
    $outputTokens = if ($TokenSummary -and $TokenSummary.PSObject.Properties.Name -contains "output_tokens") { $TokenSummary.output_tokens } else { $null }
    [pscustomobject]@{
        schema_version = "taskspace-provider-input-visibility-v1"
        source_path = $JsonlPath
        jsonl_bytes = $jsonlBytes
        input_tokens = $inputTokens
        output_tokens = $outputTokens
        provider_input_tokens_per_jsonl_kb = Get-TaskspaceCostRatio $inputTokens $jsonlKb
        provider_total_tokens_per_jsonl_kb = if ($null -ne $inputTokens -and $null -ne $outputTokens) {
            Get-TaskspaceCostRatio ([double]$inputTokens + [double]$outputTokens) $jsonlKb
        } else {
            $null
        }
        visibility_note = "Compares provider usage input tokens with local whale-exec JSONL byte size; high ratios indicate hidden provider-side request components or accounting outside the visible transcript."
    }
}

function New-TaskspaceControlUsageSummary {
    param(
        [string]$JsonlPath,
        [string]$ObservabilityJsonPath = "",
        [AllowEmptyString()][string]$RolloutJsonlPath = ""
    )
    $parsed = Get-TaskspaceCostJsonlRows $JsonlPath
    $actions = @{}
    $total = 0
    $stateCommit = 0
    $nativeTotal = 0
    $actionContractTotal = 0
    function Add-ControlUsage([AllowEmptyString()][string]$Action, [string]$Source) {
        $script:taskspaceCostControlTotal++
        if ($Source -eq "action_contract") {
            $script:taskspaceCostActionContractControlTotal++
        } else {
            $script:taskspaceCostNativeControlTotal++
        }
        if ($Action -eq "state_commit") { $script:taskspaceCostStateCommit++ }
        Add-TaskspaceCostCount $script:taskspaceCostActions $Action
    }
    function Visit-ControlValue($Current) {
        if ($null -eq $Current) { return }
        if ($Current -is [string]) {
            $text = $Current.Trim()
            if ($text.StartsWith("{") -and $text.Contains("taskspace-action-v1")) {
                try {
                    $actionContract = $text | ConvertFrom-Json
                    if ([string](Get-TaskspaceCostProperty $actionContract @("schema_version")) -eq "taskspace-action-v1" -and [string](Get-TaskspaceCostProperty $actionContract @("action")) -eq "taskspace_control") {
                        $args = Get-TaskspaceCostProperty $actionContract @("args")
                        $innerAction = [string](Get-TaskspaceCostProperty $args @("action", "control_action", "control_type", "action_name", "command"))
                        Add-ControlUsage $innerAction "action_contract"
                    }
                } catch {}
            }
            return
        }
        if ($Current -is [ValueType]) { return }
        $names = @($Current.PSObject.Properties.Name)
        $nameValue = Get-TaskspaceCostProperty $Current @("name", "tool")
        if ([string]$nameValue -eq "taskspace_control") {
            $action = ""
            $arguments = Get-TaskspaceCostProperty $Current @("arguments", "args")
            if ($arguments) {
                if ($arguments -is [string]) {
                    try {
                        $parsedArgs = $arguments | ConvertFrom-Json
                        $action = [string](Get-TaskspaceCostProperty $parsedArgs @("action"))
                    } catch {}
                } else {
                    $action = [string](Get-TaskspaceCostProperty $arguments @("action"))
                }
            }
            if ([string]::IsNullOrWhiteSpace($action)) {
                $action = [string](Get-TaskspaceCostProperty $Current @("action"))
            }
            Add-ControlUsage $action "native"
        }
        foreach ($prop in @($Current.PSObject.Properties)) {
            if ($prop.Value -is [System.Collections.IEnumerable] -and -not ($prop.Value -is [string])) {
                foreach ($item in @($prop.Value)) { Visit-ControlValue $item }
            } else {
                Visit-ControlValue $prop.Value
            }
        }
    }
    $script:taskspaceCostActions = $actions
    $script:taskspaceCostControlTotal = 0
    $script:taskspaceCostStateCommit = 0
    $script:taskspaceCostNativeControlTotal = 0
    $script:taskspaceCostActionContractControlTotal = 0
    foreach ($row in @($parsed.rows)) { Visit-ControlValue $row }
    $total = $script:taskspaceCostControlTotal
    $stateCommit = $script:taskspaceCostStateCommit
    $nativeTotal = $script:taskspaceCostNativeControlTotal
    $actionContractTotal = $script:taskspaceCostActionContractControlTotal
    $execNativeTotal = [int]$nativeTotal
    $execActionContractTotal = [int]$actionContractTotal
    $execTotal = [int]$total
    Remove-Variable -Name taskspaceCostActions -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostControlTotal -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostStateCommit -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostNativeControlTotal -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostActionContractControlTotal -Scope Script -ErrorAction SilentlyContinue
    $rolloutSourceStatus = if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath) -and (Test-Path -LiteralPath $RolloutJsonlPath -PathType Leaf)) { "read" } else { "missing" }
    $rolloutActions = @{}
    $rolloutNativeCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutActionContractCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutControlFailureCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutControlProtocolFailureCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutControlStateFailureCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutNestedActionFailureCallIds = [System.Collections.Generic.HashSet[string]]::new()
    $rolloutResponseItemCount = 0
    if ($rolloutSourceStatus -eq "read") {
        foreach ($line in [System.IO.File]::ReadLines($RolloutJsonlPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try { $row = $line | ConvertFrom-Json } catch { continue }
            $payload = Get-TaskspaceCanonicalResponseItem $row
            if ($null -eq $payload) { continue }
            $rolloutResponseItemCount++
            $payloadType = [string](Get-TaskspaceCostProperty $payload @("type"))
            if ($payloadType -in @("function_call_output", "custom_tool_call_output")) {
                $callId = [string](Get-TaskspaceCostProperty $payload @("call_id"))
                if (-not [string]::IsNullOrWhiteSpace($callId) -and
                    ($rolloutNativeCallIds.Contains($callId) -or $rolloutActionContractCallIds.Contains($callId))) {
                    $output = [string](Get-TaskspaceCostProperty $payload @("output"))
                    $controlFailed = $false
                    $failureClass = ""
                    if (-not [string]::IsNullOrWhiteSpace($output)) {
                        try {
                            $batch = $output | ConvertFrom-Json
                            $schemaVersion = [string](Get-TaskspaceCostProperty $batch @("schema_version"))
                            $controlFailed = $schemaVersion -eq "TaskSpaceControlResultV1" -and
                                $batch.PSObject.Properties.Name -contains "success" -and
                                [bool](Get-TaskspaceCostProperty $batch @("success")) -eq $false
                            if ($controlFailed) {
                                $status = [string](Get-TaskspaceCostProperty $batch @("status"))
                                $error = Get-TaskspaceCostProperty $batch @("error")
                                $errorClass = [string](Get-TaskspaceCostProperty $error @("class"))
                                $failureClass = if ($status -eq "partial") {
                                    "nested_action"
                                } elseif ($status -eq "state_machine_failed" -or $errorClass -eq "state_machine") {
                                    "state"
                                } else {
                                    "protocol"
                                }
                            }
                        } catch {}
                    }
                    if ($controlFailed) {
                        [void]$rolloutControlFailureCallIds.Add($callId)
                        if ($failureClass -eq "state") {
                            [void]$rolloutControlStateFailureCallIds.Add($callId)
                        } elseif ($failureClass -eq "nested_action") {
                            [void]$rolloutNestedActionFailureCallIds.Add($callId)
                        } else {
                            [void]$rolloutControlProtocolFailureCallIds.Add($callId)
                        }
                    }
                }
                continue
            }
            if ($payloadType -notin @("function_call", "custom_tool_call")) { continue }
            if ([string](Get-TaskspaceCostProperty $payload @("name")) -ne "taskspace_control") { continue }
            $callId = [string](Get-TaskspaceCostProperty $payload @("call_id"))
            if ([string]::IsNullOrWhiteSpace($callId)) { continue }
            $isActionContract = $callId.StartsWith("taskspace-action-contract-")
            $added = if ($isActionContract) {
                $rolloutActionContractCallIds.Add($callId)
            } else {
                $rolloutNativeCallIds.Add($callId)
            }
            if (-not $added) { continue }
            $action = ""
            $arguments = Get-TaskspaceCostProperty $payload @("arguments", "args")
            if ($arguments -is [string]) {
                try {
                    $parsedArguments = $arguments | ConvertFrom-Json
                    $action = [string](Get-TaskspaceCostProperty $parsedArguments @("action"))
                } catch {}
            } elseif ($null -ne $arguments) {
                $action = [string](Get-TaskspaceCostProperty $arguments @("action"))
            }
            Add-TaskspaceCostCount $rolloutActions $action
        }
    }
    $rolloutNativeTotal = [int]$rolloutNativeCallIds.Count
    $rolloutActionContractTotal = [int]$rolloutActionContractCallIds.Count
    $rolloutTotal = $rolloutNativeTotal + $rolloutActionContractTotal
    $rolloutControlTelemetryMeasured = ($rolloutSourceStatus -eq "read" -and $rolloutResponseItemCount -gt 0)
    $controlCountSource = "unavailable"
    if ($rolloutControlTelemetryMeasured) {
        $nativeTotal = $rolloutNativeTotal
        $actionContractTotal = $rolloutActionContractTotal
        $total = $rolloutTotal
        $actions = $rolloutActions
        $controlCountSource = "rollout_trace"
    } elseif ($parsed.source_status -eq "read") {
        $controlCountSource = "whale_exec_jsonl"
    }
    $stateCommit = if ($actions.ContainsKey("state_commit")) { [int]$actions["state_commit"] } else { 0 }
    $controlCountSourceMismatch = (
        $rolloutControlTelemetryMeasured -and
        $parsed.source_status -eq "read" -and
        $execTotal -gt 0 -and
        $rolloutTotal -gt 0 -and
        ($execTotal -ne $rolloutTotal -or
            $execNativeTotal -ne $rolloutNativeTotal -or
            $execActionContractTotal -ne $rolloutActionContractTotal)
    )
    $runtimeEventCounts = @{}
    $runtimeEventTotal = 0
    $runtimeStateCommit = 0
    $runtimeOutputRefCreated = 0
    $runtimeOutputRefSliceRead = 0
    $runtimeSourceStatus = "missing"
    if (-not [string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -and (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        try {
            $obsText = Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath
            $obs = $obsText | ConvertFrom-Json
            $runtimeSourceStatus = "read"
            foreach ($event in @($obs.timeline | Where-Object { [string]$_.kind -notlike "tool:*" })) {
                $kind = [string]$event.kind
                Add-TaskspaceCostCount $runtimeEventCounts $kind
                $runtimeEventTotal++
                if ($kind -eq "output_ref.created") { $runtimeOutputRefCreated++ }
                if ($kind -eq "output_ref.slice_read") { $runtimeOutputRefSliceRead++ }
                $updateKind = [string](Get-TaskspaceCostProperty $event @("updateKind"))
                if ([string]::IsNullOrWhiteSpace($updateKind) -and $null -ne $event.details) {
                    $updateKind = [string](Get-TaskspaceCostProperty $event.details @("updateKind"))
                }
                if ($updateKind -like "state_commit*") { $runtimeStateCommit++ }
            }
            $exportMode = ""
            if ($obs.PSObject.Properties.Name -contains "source" -and $obs.source -and
                $obs.source.PSObject.Properties.Name -contains "exportPolicy" -and $obs.source.exportPolicy) {
                $exportMode = [string]$obs.source.exportPolicy.rollout_export_mode
            }
            if ($exportMode -eq "summary_only_large_rollout" -and
                $obs.PSObject.Properties.Name -contains "summary" -and $obs.summary -and
                $obs.summary.PSObject.Properties.Name -contains "runtimeEventCounts") {
                $runtimeEventCounts = ConvertFrom-TaskspaceCostCountObject $obs.summary.runtimeEventCounts
                $runtimeEventTotal = 0
                $runtimeStateCommit = 0
                $runtimeOutputRefCreated = 0
                $runtimeOutputRefSliceRead = 0
                foreach ($key in @($runtimeEventCounts.Keys)) {
                    $count = [int]$runtimeEventCounts[$key]
                    $runtimeEventTotal += $count
                    if ($key -eq "output_ref.created") { $runtimeOutputRefCreated += $count }
                    if ($key -eq "output_ref.slice_read") { $runtimeOutputRefSliceRead += $count }
                    if ($key -like "state_commit*") { $runtimeStateCommit += $count }
                }
                $runtimeSourceStatus = "summary_only_large_rollout"
            }
            $observedCreatedRefs = New-Object 'System.Collections.Generic.HashSet[string]'
            $observedSliceRefs = New-Object 'System.Collections.Generic.HashSet[string]'
            foreach ($match in [regex]::Matches($obsText, "(?s)OutputReferenceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                [void]$observedCreatedRefs.Add([string]$match.Groups[1].Value)
            }
            foreach ($match in [regex]::Matches($obsText, "(?s)OutputSliceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                [void]$observedSliceRefs.Add([string]$match.Groups[1].Value)
            }
            $runtimeOutputRefCreated = [Math]::Max([int]$runtimeOutputRefCreated, [int]$observedCreatedRefs.Count)
            $runtimeOutputRefSliceRead = [Math]::Max([int]$runtimeOutputRefSliceRead, [int]$observedSliceRefs.Count)
        } catch {
            $runtimeSourceStatus = "parse_error"
        }
    }
    [pscustomobject]@{
        schema_version = "taskspace-control-usage-v2"
        source_path = $JsonlPath
        observability_source_path = $ObservabilityJsonPath
        rollout_source_path = $RolloutJsonlPath
        source_status = [string]$parsed.source_status
        rollout_source_status = $rolloutSourceStatus
        rollout_response_item_count = [int]$rolloutResponseItemCount
        rollout_control_telemetry_measured = [bool]$rolloutControlTelemetryMeasured
        observability_source_status = $runtimeSourceStatus
        parse_errors = [int]$parsed.parse_errors
        availability = if ($parsed.source_status -eq "read" -or $rolloutSourceStatus -eq "read") { "measured" } else { "source_missing" }
        taskspace_control_count_source = $controlCountSource
        taskspace_control_count_source_mismatch = [bool]$controlCountSourceMismatch
        whale_exec_taskspace_control_count = $execTotal
        rollout_taskspace_control_count = $rolloutTotal
        whale_exec_native_taskspace_control_count = $execNativeTotal
        rollout_native_taskspace_control_count = $rolloutNativeTotal
        whale_exec_action_contract_taskspace_control_count = $execActionContractTotal
        rollout_action_contract_taskspace_control_count = $rolloutActionContractTotal
        taskspace_control_count = [int]$total
        native_taskspace_control_count = [int]$nativeTotal
        action_contract_taskspace_control_count = [int]$actionContractTotal
        control_failure_count = [int]$rolloutControlFailureCallIds.Count
        control_protocol_failure_count = [int]$rolloutControlProtocolFailureCallIds.Count
        control_state_failure_count = [int]$rolloutControlStateFailureCallIds.Count
        nested_action_failure_count = [int]$rolloutNestedActionFailureCallIds.Count
        state_commit_count = [int]$stateCommit
        runtime_state_commit_count = [int]$runtimeStateCommit
        runtime_output_ref_created_count = [int]$runtimeOutputRefCreated
        runtime_output_ref_slice_read_count = [int]$runtimeOutputRefSliceRead
        taskspace_runtime_event_count = [int]$runtimeEventTotal
        action_counts = Convert-TaskspaceCostTable $actions
        runtime_event_counts = Convert-TaskspaceCostTable $runtimeEventCounts
    }
}

function New-TaskspaceReplaySummary {
    param([string]$ArtifactDir)
    $largest = [int64]0
    $largeReplay = 0
    $checked = 0
    $outputReferenceCount = 0
    $outputSliceCount = 0
    $truncationMarkerCount = 0
    $rawLargeMarkerCount = 0
    foreach ($name in @("taskspace.graph.final.json", "taskspace.graph.timeout.json", "graph-health.json")) {
        $path = Join-Path $ArtifactDir $name
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $checked++
        $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        if ($text.Length -gt $largest) { $largest = [int64]$text.Length }
        $outputReferenceCount += ([regex]::Matches($text, "OutputReferenceV1:")).Count
        $outputSliceCount += ([regex]::Matches($text, "OutputSliceV1:")).Count
        if ($text -match "\[\.{3} telemetry preview truncated \.\{3}\]" -or $text -match "\[\.\.\. telemetry preview truncated \.\.\.\]") {
            $truncationMarkerCount++
            $largeReplay++
        }
        $markerMatches = ([regex]::Matches($text, "middle-secret-marker")).Count
        if ($markerMatches -ge 300) {
            $rawLargeMarkerCount++
            $largeReplay++
        }
    }
    [pscustomobject]@{
        schema_version = "taskspace-replay-summary-v1"
        availability = if ($checked -gt 0) { "heuristic" } else { "source_missing" }
        checked_artifact_count = [int]$checked
        largest_tool_output_bytes = $largest
        large_output_replay_count = [int]$largeReplay
        output_reference_count = [int]$outputReferenceCount
        output_slice_count = [int]$outputSliceCount
        truncation_marker_count = [int]$truncationMarkerCount
        raw_large_marker_count = [int]$rawLargeMarkerCount
        raw_output_in_prompt_violation = ([int]$largeReplay -gt 0)
    }
}

function New-TaskspaceOutputRefEvents {
    param(
        [AllowEmptyString()][string]$ObservabilityJsonPath = "",
        [AllowEmptyString()][string]$ArtifactDir = ""
    )
    $events = New-Object System.Collections.Generic.List[object]
    $seen = New-Object 'System.Collections.Generic.HashSet[string]'
    if (-not [string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -and (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        try {
            $obsText = Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath
            $obs = $obsText | ConvertFrom-Json
            foreach ($event in @($obs.timeline)) {
                $kind = [string]$event.kind
                if ($kind -notin @("output_ref.created", "output_ref.slice_read")) { continue }
                $ref = [string](Get-TaskspaceCostProperty $event @("artifactRef", "artifact_ref", "outputRef", "output_ref"))
                if ([string]::IsNullOrWhiteSpace($ref) -and $null -ne $event.details) {
                    $ref = [string](Get-TaskspaceCostProperty $event.details @("artifactRef", "artifact_ref", "outputRef", "output_ref"))
                }
                $dedupe = "$kind|$ref|$($events.Count)"
                if (-not $seen.Add($dedupe)) { continue }
                $events.Add([pscustomobject]@{
                    schema_version = "taskspace-output-ref-event-v1"
                    source = "observability_timeline"
                    kind = $kind
                    artifact_ref = $ref
                    call_id = [string](Get-TaskspaceCostProperty $event @("callId", "call_id"))
                    timestamp_ms = Get-TaskspaceCostProperty $event @("timestampMs", "timestamp_ms", "createdAtMs", "created_at_ms")
                })
            }
            foreach ($match in [regex]::Matches($obsText, "(?s)OutputReferenceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                $ref = [string]$match.Groups[1].Value
                $dedupe = "output_ref.created|$ref|fallback"
                if ($seen.Add($dedupe)) {
                    $events.Add([pscustomobject]@{
                        schema_version = "taskspace-output-ref-event-v1"
                        source = "observability_text_fallback"
                        kind = "output_ref.created"
                        artifact_ref = $ref
                        call_id = ""
                        timestamp_ms = $null
                    })
                }
            }
            foreach ($match in [regex]::Matches($obsText, "(?s)OutputSliceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                $ref = [string]$match.Groups[1].Value
                $dedupe = "output_ref.slice_read|$ref|fallback"
                if ($seen.Add($dedupe)) {
                    $events.Add([pscustomobject]@{
                        schema_version = "taskspace-output-ref-event-v1"
                        source = "observability_text_fallback"
                        kind = "output_ref.slice_read"
                        artifact_ref = $ref
                        call_id = ""
                        timestamp_ms = $null
                    })
                }
            }
        } catch {
            $events.Add([pscustomobject]@{
                schema_version = "taskspace-output-ref-event-v1"
                source = "observability_parse_error"
                kind = "output_ref.parse_error"
                artifact_ref = ""
                call_id = ""
                timestamp_ms = $null
            })
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ArtifactDir)) {
        foreach ($name in @("taskspace.graph.final.json", "taskspace.graph.timeout.json", "graph-health.json")) {
            $path = Join-Path $ArtifactDir $name
            if (-not (Test-Path -LiteralPath $path)) { continue }
            $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
            foreach ($match in [regex]::Matches($text, "(?s)OutputReferenceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                $ref = [string]$match.Groups[1].Value
                $dedupe = "output_ref.created|$ref|artifact"
                if ($seen.Add($dedupe)) {
                    $events.Add([pscustomobject]@{
                        schema_version = "taskspace-output-ref-event-v1"
                        source = $name
                        kind = "output_ref.created"
                        artifact_ref = $ref
                        call_id = ""
                        timestamp_ms = $null
                    })
                }
            }
            foreach ($match in [regex]::Matches($text, "(?s)OutputSliceV1:.*?artifact_ref:\s*(output-ref://sha256/[a-fA-F0-9]{64})")) {
                $ref = [string]$match.Groups[1].Value
                $dedupe = "output_ref.slice_read|$ref|artifact"
                if ($seen.Add($dedupe)) {
                    $events.Add([pscustomobject]@{
                        schema_version = "taskspace-output-ref-event-v1"
                        source = $name
                        kind = "output_ref.slice_read"
                        artifact_ref = $ref
                        call_id = ""
                        timestamp_ms = $null
                    })
                }
            }
        }
    }
    @($events.ToArray())
}

function Get-TaskspaceContextProjectionBlocks {
    param(
        [AllowEmptyString()][string]$JsonlPath = "",
        [AllowEmptyString()][string]$ObservabilityJsonPath = "",
        [AllowEmptyString()][string]$RolloutJsonlPath = ""
    )
    $texts = New-Object System.Collections.Generic.List[string]
    foreach ($path in @($JsonlPath, $ObservabilityJsonPath)) {
        if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path)) { continue }
        $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        $unescaped = $raw -replace "\\r\\n", "`n" -replace "\\n", "`n"
        $texts.Add($unescaped)
    }
    if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath) -and (Test-Path -LiteralPath $RolloutJsonlPath -PathType Leaf)) {
        foreach ($line in [System.IO.File]::ReadLines($RolloutJsonlPath)) {
            if ($line -notmatch 'ContextProjectionV1') { continue }
            $texts.Add(($line -replace "\\r\\n", "`n" -replace "\\n", "`n"))
        }
    }
    $blocks = New-Object System.Collections.Generic.List[string]
    foreach ($text in @($texts.ToArray())) {
        foreach ($match in [regex]::Matches($text, "(?s)ContextProjectionV1 active replacement:.*?ContextProjectionV1 active replacement end\.|ContextProjectionV1 shadow \(not active replacement\):.*?- estimated_tokens:\s*\d+")) {
            $block = [string]$match.Value
            if (-not [string]::IsNullOrWhiteSpace($block)) { $blocks.Add($block) }
        }
    }
    @($blocks.ToArray() | Select-Object -Unique)
}

function New-TaskspaceContextProjectionEvent {
    param([Parameter(Mandatory = $true)][string]$Block)
    $projectionKind = if ($Block -match "ContextProjectionV1 active replacement:") { "active_replacement" } elseif ($Block -match "ContextProjectionV1 shadow \(not active replacement\):") { "shadow" } else { "unknown" }
    $requiredSections = if ($projectionKind -eq "active_replacement") {
        @("task_id", "map_id", "current_node", "map_nodes", "current_node_recent_events", "result_refs_available")
    } else {
        @("success_criteria", "current_node", "blockers", "decisions", "facts", "relevant_results", "verified_input_evidence", "fact_source_coverage", "result_refs_available")
    }
    $missing = @($requiredSections | Where-Object { $Block -notmatch "(?m)^\s*(?:-\s*)?$([regex]::Escape($_)):" })
    $projectionId = ""
    $taskId = ""
    $mode = ""
    $estimatedTokens = $null
    $projectionMatch = [regex]::Match($Block, "(?m)^-\s*projection_id:\s*(.+?)\s*$")
    if ($projectionMatch.Success) { $projectionId = $projectionMatch.Groups[1].Value.Trim() }
    $taskMatch = [regex]::Match($Block, "(?m)^-\s*task_id:\s*(.+?)\s*$")
    if ($taskMatch.Success) { $taskId = $taskMatch.Groups[1].Value.Trim() }
    $modeMatch = [regex]::Match($Block, "(?m)^-\s*mode:\s*(.+?)\s*$")
    if ($modeMatch.Success) { $mode = $modeMatch.Groups[1].Value.Trim() }
    $tokenMatch = [regex]::Match($Block, "(?m)^-\s*estimated_tokens:\s*(\d+)\s*$")
    if ($tokenMatch.Success) { $estimatedTokens = [int64]$tokenMatch.Groups[1].Value }
    [pscustomobject]@{
        schema_version = "taskspace-context-projection-event-v1"
        projection_id = $projectionId
        task_id = $taskId
        mode = if ([string]::IsNullOrWhiteSpace($mode)) { "unknown" } else { $mode }
        projection_kind = $projectionKind
        estimated_tokens = $estimatedTokens
        protected_miss_count = [int]$missing.Count
        protected_missing_sections = @($missing)
        source = "model_visible_context"
    }
}

function New-TaskspaceContextProjectionTraceEvents {
    param(
        [AllowEmptyString()][string]$ObservabilityJsonPath = "",
        [AllowEmptyString()][string]$RolloutJsonlPath = ""
    )
    $events = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("projection_budget") $RolloutJsonlPath)) {
        $tags = Convert-TaskspaceTraceTags $event
        $taskId = [string](Get-TaskspaceTraceField $event @("taskId", "task_id"))
        $mapId = [string](Get-TaskspaceTraceField $event @("mapId", "map_id"))
        $traceId = [string](Get-TaskspaceTraceField $event @("traceEventId", "trace_event_id", "id"))
        $events.Add([pscustomobject]@{
            schema_version = "taskspace-context-projection-event-v1"
            projection_id = if ([string]::IsNullOrWhiteSpace($taskId) -or [string]::IsNullOrWhiteSpace($mapId)) { "" } else { "projection-taskspace-$taskId-$mapId" }
            task_id = $taskId
            map_id = $mapId
            mode = "taskspace"
            projection_kind = "epoch_snapshot"
            estimated_tokens = Convert-TaskspaceTraceNullableInt $tags.projection_tokens
            protected_miss_count = 0
            protected_missing_sections = @()
            source = "runtime_trace"
            trace_event_id = $traceId
        })
    }
    @($events.ToArray())
}

function New-TaskspaceContextProjectionSummary {
    param(
        [AllowEmptyString()][string]$JsonlPath = "",
        [AllowEmptyString()][string]$ObservabilityJsonPath = "",
        [AllowEmptyString()][string]$RolloutJsonlPath = ""
    )
    $sourcePresent = $false
    foreach ($path in @($JsonlPath, $ObservabilityJsonPath, $RolloutJsonlPath)) {
        if (-not [string]::IsNullOrWhiteSpace($path) -and (Test-Path -LiteralPath $path)) { $sourcePresent = $true }
    }
    $blockEvents = @(Get-TaskspaceContextProjectionBlocks $JsonlPath $ObservabilityJsonPath $RolloutJsonlPath | ForEach-Object {
            New-TaskspaceContextProjectionEvent $_
        })
    $events = @($blockEvents) + @(New-TaskspaceContextProjectionTraceEvents $ObservabilityJsonPath $RolloutJsonlPath)
    $tokenValues = @($events | Where-Object { $null -ne $_.estimated_tokens } | ForEach-Object { [int64]$_.estimated_tokens })
    $tokenTotal = [int64]0
    foreach ($value in $tokenValues) { $tokenTotal += [int64]$value }
    $protectedMiss = 0
    foreach ($event in $events) { $protectedMiss += [int]$event.protected_miss_count }
    $activeProjectionCount = @($events | Where-Object { [string]$_.projection_kind -in @("active_replacement", "epoch_snapshot") }).Count
    $shadowProjectionCount = @($events | Where-Object { [string]$_.projection_kind -eq "shadow" }).Count
    [pscustomobject]@{
        schema_version = "taskspace-context-projection-summary-v1"
        source_path = $JsonlPath
        observability_source_path = $ObservabilityJsonPath
        rollout_source_path = $RolloutJsonlPath
        availability = if ($events.Count -gt 0) { "measured" } elseif ($sourcePresent) { "projection_unavailable" } else { "source_missing" }
        projection_count = [int]$events.Count
        projection_tokens_total = if ($tokenValues.Count -gt 0) { $tokenTotal } else { $null }
        projection_tokens_max = if ($tokenValues.Count -gt 0) { ($tokenValues | Measure-Object -Maximum).Maximum } else { $null }
        projection_tokens_avg = if ($tokenValues.Count -gt 0) { [Math]::Round([double]$tokenTotal / [double]$tokenValues.Count, 4) } else { $null }
        protected_miss_count = [int]$protectedMiss
        active_projection_count = [int]$activeProjectionCount
        shadow_projection_count = [int]$shadowProjectionCount
        events = @($events)
    }
}

function Get-TaskspaceCostRolloutScanPolicy {
    param([AllowEmptyString()][string]$RolloutJsonlPath = "")
    $thresholdBytes = 33554432
    if ($env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES) {
        try { $thresholdBytes = [Math]::Max(1048576, [int64]$env:TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES) } catch { }
    }
    $bytes = $null
    if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath) -and (Test-Path -LiteralPath $RolloutJsonlPath)) {
        try { $bytes = [int64](Get-Item -LiteralPath $RolloutJsonlPath).Length } catch { $bytes = $null }
    }
    $large = ($null -ne $bytes -and [int64]$bytes -gt [int64]$thresholdBytes)
    [pscustomobject]@{
        schema_version = "taskspace-cost-scan-policy-v2"
        rollout_source_path = $RolloutJsonlPath
        rollout_effective_scan_path = $RolloutJsonlPath
        rollout_bytes = $bytes
        rollout_scan_max_bytes = [int64]$thresholdBytes
        rollout_scan_mode = if ([string]::IsNullOrWhiteSpace($RolloutJsonlPath) -or -not (Test-Path -LiteralPath $RolloutJsonlPath)) { "missing" } elseif ($large) { "streaming_large_rollout" } else { "streaming" }
    }
}

function Write-TaskspaceCostInstrumentationArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [AllowEmptyString()][string]$JsonlPath = "",
        [AllowEmptyString()][string]$ObservabilityJsonPath = ""
    )
    if (-not (Test-Path -LiteralPath $ArtifactDir)) {
        New-Item -ItemType Directory -Path $ArtifactDir -Force | Out-Null
    }
    $rolloutJsonlPath = Join-Path $ArtifactDir "rollout.jsonl"
    $scanPolicy = Get-TaskspaceCostRolloutScanPolicy $rolloutJsonlPath
    $rolloutScanPath = [string]$scanPolicy.rollout_effective_scan_path
    $token = New-TaskspaceTokenSummary $JsonlPath
    $request = New-TaskspaceRequestSummary $JsonlPath $token $rolloutScanPath
    $request | Add-Member -NotePropertyName rollout_scan_policy -NotePropertyValue $scanPolicy -Force
    $visibility = New-TaskspaceProviderInputVisibilitySummary $JsonlPath $token
    $control = New-TaskspaceControlUsageSummary $JsonlPath $ObservabilityJsonPath $rolloutScanPath
    $replay = New-TaskspaceReplaySummary $ArtifactDir
    $outputRefEvents = @(New-TaskspaceOutputRefEvents $ObservabilityJsonPath $ArtifactDir)
    $projection = New-TaskspaceContextProjectionSummary $JsonlPath $ObservabilityJsonPath $rolloutScanPath
    $projection | Add-Member -NotePropertyName rollout_scan_policy -NotePropertyValue $scanPolicy -Force
    $budget = New-TaskspaceBudgetArtifacts $ObservabilityJsonPath $rolloutScanPath
    $exactPayloadScanEvents = @(New-TaskspaceExactPayloadScanEvents $ObservabilityJsonPath $rolloutScanPath)
    $activeReplacement = New-TaskspaceActiveReplacementArtifacts $budget.budget_events $exactPayloadScanEvents
    $providerRequest = New-TaskspaceProviderRequestArtifacts $budget.budget_events $request
    $providerWireTracePath = Join-Path $ArtifactDir "provider-wire-trace.jsonl"
    $providerCacheTrace = New-TaskspaceProviderCacheTraceArtifacts $budget.budget_events $providerWireTracePath
    $stateCommitDisplacement = New-TaskspaceStateCommitDisplacementSummary $ObservabilityJsonPath
    $spawnNodeBudget = New-TaskspaceSpawnNodeBudgetSummary $ObservabilityJsonPath $rolloutScanPath
    $spawnNodeBudget | Add-Member -NotePropertyName rollout_scan_policy -NotePropertyValue $scanPolicy -Force
    $tokenPath = Join-Path $ArtifactDir "token-summary.json"
    $scanPolicyPath = Join-Path $ArtifactDir "cost-scan-policy.json"
    $requestPath = Join-Path $ArtifactDir "request-summary.json"
    $visibilityPath = Join-Path $ArtifactDir "provider-input-visibility.json"
    $controlPath = Join-Path $ArtifactDir "taskspace-control-usage.json"
    $projectionPath = Join-Path $ArtifactDir "context-projection-summary.json"
    $projectionEventsPath = Join-Path $ArtifactDir "projection-events.jsonl"
    $outputRefEventsPath = Join-Path $ArtifactDir "output-ref-events.jsonl"
    $budgetEventsPath = Join-Path $ArtifactDir "budget-events.jsonl"
    $budgetQualityImpactEventsPath = Join-Path $ArtifactDir "budget-quality-impact-events.jsonl"
    $budgetQualityImpactSummaryPath = Join-Path $ArtifactDir "budget_induced_quality_impact_summary.json"
    $exactPayloadScanEventsPath = Join-Path $ArtifactDir "exact-payload-scan-events.jsonl"
    $activeReplacementReportPath = Join-Path $ArtifactDir "active-context-replacement-report.json"
    $providerRequestEventsPath = Join-Path $ArtifactDir "provider-request-events.jsonl"
    $requestPhaseSummaryPath = Join-Path $ArtifactDir "request-phase-summary.json"
    $requestReasonSummaryPath = Join-Path $ArtifactDir "request-reason-summary.json"
    $providerCacheTracePath = Join-Path $ArtifactDir "provider-cache-trace.jsonl"
    $providerCacheTraceSummaryPath = Join-Path $ArtifactDir "provider-cache-trace-summary.json"
    $stateCommitDisplacementPath = Join-Path $ArtifactDir "state-commit-displacement.json"
    $spawnNodeBudgetPath = Join-Path $ArtifactDir "spawn-node-budget-summary.json"
    $activeBudgetEventsPath = Join-Path $ArtifactDir "active-budget-events.jsonl"
    $token | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $tokenPath -Encoding UTF8
    $scanPolicy | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $scanPolicyPath -Encoding UTF8
    $request | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $visibility | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $visibilityPath -Encoding UTF8
    $control | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $controlPath -Encoding UTF8
    $projection | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $projectionPath -Encoding UTF8
    $projectionEventLines = @($projection.events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($projectionEventLines.Count -gt 0) {
        $projectionEventLines | Set-Content -LiteralPath $projectionEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($projectionEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $outputRefEventLines = @($outputRefEvents | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($outputRefEventLines.Count -gt 0) {
        $outputRefEventLines | Set-Content -LiteralPath $outputRefEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($outputRefEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $budgetEventLines = @($budget.budget_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($budgetEventLines.Count -gt 0) {
        $budgetEventLines | Set-Content -LiteralPath $budgetEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($budgetEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $activeBudgetEventLines = @($budget.active_budget_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($activeBudgetEventLines.Count -gt 0) {
        $activeBudgetEventLines | Set-Content -LiteralPath $activeBudgetEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($activeBudgetEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $budgetQualityImpactEventLines = @($budget.budget_quality_impact_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($budgetQualityImpactEventLines.Count -gt 0) {
        $budgetQualityImpactEventLines | Set-Content -LiteralPath $budgetQualityImpactEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($budgetQualityImpactEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $budget.budget_quality_impact_summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $budgetQualityImpactSummaryPath -Encoding UTF8
    $exactPayloadScanEventLines = @($exactPayloadScanEvents | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($exactPayloadScanEventLines.Count -gt 0) {
        $exactPayloadScanEventLines | Set-Content -LiteralPath $exactPayloadScanEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($exactPayloadScanEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $activeReplacement.active_context_replacement_report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $activeReplacementReportPath -Encoding UTF8
    $providerRequestEventLines = @($providerRequest.provider_request_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($providerRequestEventLines.Count -gt 0) {
        $providerRequestEventLines | Set-Content -LiteralPath $providerRequestEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($providerRequestEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $providerRequest.request_phase_summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $requestPhaseSummaryPath -Encoding UTF8
    $providerRequest.request_reason_summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $requestReasonSummaryPath -Encoding UTF8
    $providerCacheTraceEventLines = @($providerCacheTrace.provider_cache_trace_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($providerCacheTraceEventLines.Count -gt 0) {
        $providerCacheTraceEventLines | Set-Content -LiteralPath $providerCacheTracePath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($providerCacheTracePath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $providerCacheTrace.provider_cache_trace_summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $providerCacheTraceSummaryPath -Encoding UTF8
    $stateCommitDisplacement | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $stateCommitDisplacementPath -Encoding UTF8
    $spawnNodeBudget | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $spawnNodeBudgetPath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        cost_scan_policy_path = $scanPolicyPath
        request_summary_path = $requestPath
        provider_input_visibility_path = $visibilityPath
        taskspace_control_usage_path = $controlPath
        context_projection_summary_path = $projectionPath
        projection_events_path = $projectionEventsPath
        output_ref_events_path = $outputRefEventsPath
        budget_events_path = $budgetEventsPath
        active_budget_events_path = $activeBudgetEventsPath
        budget_quality_impact_events_path = $budgetQualityImpactEventsPath
        budget_quality_impact_summary_path = $budgetQualityImpactSummaryPath
        exact_payload_scan_events_path = $exactPayloadScanEventsPath
        active_context_replacement_report_path = $activeReplacementReportPath
        provider_request_events_path = $providerRequestEventsPath
        request_phase_summary_path = $requestPhaseSummaryPath
        request_reason_summary_path = $requestReasonSummaryPath
        provider_cache_trace_path = $providerCacheTracePath
        provider_cache_trace_summary_path = $providerCacheTraceSummaryPath
        provider_wire_trace_path = $providerWireTracePath
        state_commit_displacement_path = $stateCommitDisplacementPath
        spawn_node_budget_path = $spawnNodeBudgetPath
        token_summary = $token
        cost_scan_policy = $scanPolicy
        request_summary = $request
        provider_input_visibility = $visibility
        taskspace_control_usage = $control
        replay_summary = $replay
        output_ref_events = $outputRefEvents
        context_projection_summary = $projection
        budget_events = $budget.budget_events
        active_budget_events = $budget.active_budget_events
        budget_quality_impact_events = $budget.budget_quality_impact_events
        budget_quality_impact_summary = $budget.budget_quality_impact_summary
        exact_payload_scan_events = $exactPayloadScanEvents
        active_context_replacement_report = $activeReplacement.active_context_replacement_report
        provider_request_events = $providerRequest.provider_request_events
        request_phase_summary = $providerRequest.request_phase_summary
        request_reason_summary = $providerRequest.request_reason_summary
        provider_cache_trace_events = $providerCacheTrace.provider_cache_trace_events
        provider_cache_trace_summary = $providerCacheTrace.provider_cache_trace_summary
        state_commit_displacement = $stateCommitDisplacement
        spawn_node_budget = $spawnNodeBudget
    }
}

function Get-TaskspaceCostMetricNumber {
    param($Metric, [string]$Name)
    if ($Metric -and $Metric.PSObject.Properties.Name -contains $Name -and $null -ne $Metric.$Name) {
        try { return [double]$Metric.$Name } catch { return $null }
    }
    $null
}

function Add-TaskspaceCostMetricTotal {
    param([System.Collections.IDictionary]$Totals, $Metric, [string]$Field)
    $value = Get-TaskspaceCostMetricNumber $Metric $Field
    if ($null -eq $value) {
        $Totals["missing_$Field"]++
        return
    }
    $Totals[$Field] = [double]$Totals[$Field] + [double]$value
}

function Add-TaskspaceCostMetricTotalWithFallback {
    param(
        [System.Collections.IDictionary]$Totals,
        $Metric,
        [string]$Field,
        [string]$FallbackField
    )
    $value = Get-TaskspaceCostMetricNumber $Metric $Field
    if ($null -eq $value -and -not [string]::IsNullOrWhiteSpace($FallbackField)) {
        $value = Get-TaskspaceCostMetricNumber $Metric $FallbackField
        if ($null -ne $value) {
            $Totals["fallback_$Field"]++
        }
    }
    if ($null -eq $value) {
        $Totals["missing_$Field"]++
        return
    }
    $Totals[$Field] = [double]$Totals[$Field] + [double]$value
}

function New-TaskspaceCostSideTotals {
    param([string]$Mode)
    [ordered]@{
        logical_mode = $Mode
        side_count = 0
        complete_side_count = 0
        model_request_count = [double]0
        input_tokens = [double]0
        output_tokens = [double]0
        cached_input_tokens = [double]0
        uncached_input_tokens = [double]0
        jsonl_bytes = [double]0
        provider_input_tokens_per_jsonl_kb = [double]0
        provider_total_tokens_per_jsonl_kb = [double]0
        wall_time_ms = [double]0
        taskspace_control_count = [double]0
        native_taskspace_control_count = [double]0
        action_contract_taskspace_control_count = [double]0
        state_commit_count = [double]0
        runtime_state_commit_count = [double]0
        runtime_output_ref_created_count = [double]0
        runtime_output_ref_slice_read_count = [double]0
        taskspace_runtime_event_count = [double]0
        large_output_replay_count = [double]0
        projection_count = [double]0
        projection_tokens = [double]0
        projection_protected_miss_count = [double]0
        missing_model_request_count = 0
        missing_input_tokens = 0
        missing_output_tokens = 0
        missing_cached_input_tokens = 0
        missing_uncached_input_tokens = 0
        missing_jsonl_bytes = 0
        missing_provider_input_tokens_per_jsonl_kb = 0
        missing_provider_total_tokens_per_jsonl_kb = 0
        missing_wall_time_ms = 0
        missing_taskspace_control_count = 0
        missing_native_taskspace_control_count = 0
        missing_action_contract_taskspace_control_count = 0
        missing_state_commit_count = 0
        missing_runtime_state_commit_count = 0
        missing_runtime_output_ref_created_count = 0
        missing_runtime_output_ref_slice_read_count = 0
        missing_taskspace_runtime_event_count = 0
        missing_large_output_replay_count = 0
        missing_projection_count = 0
        missing_projection_tokens = 0
        missing_projection_protected_miss_count = 0
        fallback_model_request_count = 0
        fallback_input_tokens = 0
        fallback_output_tokens = 0
    }
}

function Get-TaskspaceCostRatio {
    param($Numerator, $Denominator)
    if ($null -eq $Numerator -or $null -eq $Denominator -or [double]$Denominator -le 0) { return $null }
    [Math]::Round([double]$Numerator / [double]$Denominator, 4)
}

function New-TaskspaceCostGate {
    param($Standard, $Taskspace)
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($field in @("model_request_count", "input_tokens", "output_tokens", "wall_time_ms")) {
        if ($Standard["missing_$field"] -gt 0 -or $Taskspace["missing_$field"] -gt 0 -or [double]$Standard[$field] -le 0) {
            $missing.Add($field)
        }
    }
    $standardDirect = [double]$Standard.input_tokens + [double]$Standard.output_tokens
    $taskspaceDirect = [double]$Taskspace.input_tokens + [double]$Taskspace.output_tokens
    $directRatio = Get-TaskspaceCostRatio $taskspaceDirect $standardDirect
    $wallRatio = Get-TaskspaceCostRatio $Taskspace.wall_time_ms $Standard.wall_time_ms
    $requestRatio = Get-TaskspaceCostRatio $Taskspace.model_request_count $Standard.model_request_count
    $status = "FAIL"
    $reason = "cost_gate_failed"
    if ($missing.Count -gt 0 -or $null -eq $directRatio -or $null -eq $wallRatio -or $null -eq $requestRatio) {
        $status = "FAIL"
        $reason = "missing_cost_data"
    } elseif ($directRatio -le 2.0 -and $wallRatio -le 2.0) {
        $status = "PASS"
        $reason = "primary_cost_gate_passed"
    } elseif ($directRatio -le 3.0 -and $wallRatio -le 3.0 -and $requestRatio -le 2.5) {
        $status = "PARTIAL"
        $reason = "engineering_partial_cost_gate_passed"
    }
    [pscustomobject]@{
        schema_version = "taskspace-suite-cost-gate-v1"
        status = $status
        reason = $reason
        missing_fields = @($missing.ToArray())
        ratios = [pscustomobject]@{
            direct_input_output_ratio = $directRatio
            walltime_ratio = $wallRatio
            model_request_count_ratio = $requestRatio
        }
        thresholds = [pscustomobject]@{
            pass_direct_input_output_ratio = 2.0
            pass_walltime_ratio = 2.0
            partial_direct_input_output_ratio = 3.0
            partial_walltime_ratio = 3.0
            partial_model_request_count_ratio = 2.5
        }
    }
}

function Write-TaskspaceCostAggregateArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$RootDir,
        [Parameter(Mandatory = $true)][ValidateSet("pair", "sample", "suite")][string]$Scope
    )
    if (-not (Test-Path -LiteralPath $RootDir)) { throw "Cost aggregate root does not exist: $RootDir" }
    $metricFiles = @(Get-ChildItem -LiteralPath $RootDir -Filter "metrics.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $parseErrors = New-Object System.Collections.Generic.List[string]
    $byMode = @{
        standard = New-TaskspaceCostSideTotals "standard"
        taskspace = New-TaskspaceCostSideTotals "taskspace"
        other = New-TaskspaceCostSideTotals "other"
    }
    foreach ($file in $metricFiles) {
        try { $metric = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json } catch { $parseErrors.Add($file.FullName); continue }
        $mode = if ($metric.PSObject.Properties.Name -contains "logical_mode") { [string]$metric.logical_mode } else { "other" }
        if (-not $byMode.ContainsKey($mode)) { $mode = "other" }
        $totals = $byMode[$mode]
        $totals.side_count++
        if ([string]$metric.token_summary_availability -eq "measured") { $totals.complete_side_count++ }
        foreach ($field in @("model_request_count", "input_tokens", "output_tokens", "cached_input_tokens", "uncached_input_tokens", "jsonl_bytes", "provider_input_tokens_per_jsonl_kb", "provider_total_tokens_per_jsonl_kb", "wall_time_ms", "taskspace_control_count", "native_taskspace_control_count", "action_contract_taskspace_control_count", "state_commit_count", "runtime_state_commit_count", "runtime_output_ref_created_count", "runtime_output_ref_slice_read_count", "taskspace_runtime_event_count", "large_output_replay_count", "projection_count", "projection_tokens", "projection_protected_miss_count")) {
            if ($field -eq "model_request_count") {
                Add-TaskspaceCostMetricTotalWithFallback $totals $metric $field "rollout_trace_model_request_count"
            } elseif ($field -eq "input_tokens") {
                Add-TaskspaceCostMetricTotalWithFallback $totals $metric $field "rollout_trace_input_tokens"
            } elseif ($field -eq "output_tokens") {
                Add-TaskspaceCostMetricTotalWithFallback $totals $metric $field "rollout_trace_output_tokens"
            } else {
                Add-TaskspaceCostMetricTotal $totals $metric $field
            }
        }
    }
    $tokenPath = Join-Path $RootDir "token-summary.json"
    $requestPath = Join-Path $RootDir "request-summary.json"
    $controlPath = Join-Path $RootDir "taskspace-control-usage.json"
    $projectionPath = Join-Path $RootDir "context-projection-summary.json"
    $providerCacheTracePath = Join-Path $RootDir "provider-cache-trace.jsonl"
    $providerCacheTraceSummaryPath = Join-Path $RootDir "provider-cache-trace-summary.json"
    $gatePath = Join-Path $RootDir "suite-cost-gate.json"
    $providerCacheTrace = New-TaskspaceProviderCacheTraceAggregateArtifacts $RootDir
    $summary = [pscustomobject]@{
        schema_version = "taskspace-cost-aggregate-v1"
        scope = $Scope
        root_dir = $RootDir
        metric_file_count = @($metricFiles).Count
        parse_error_count = $parseErrors.Count
        parse_error_paths = @($parseErrors.ToArray())
        modes = [pscustomobject]@{
            standard = [pscustomobject]$byMode.standard
            taskspace = [pscustomobject]$byMode.taskspace
            other = [pscustomobject]$byMode.other
        }
        generated_at = (Get-Date).ToString("o")
    }
    $request = [pscustomobject]@{
        schema_version = "taskspace-request-aggregate-v1"
        scope = $Scope
        standard = [pscustomobject]@{
            model_request_count = $byMode.standard.model_request_count
            avg_input_tokens_per_request = Get-TaskspaceCostRatio $byMode.standard.input_tokens $byMode.standard.model_request_count
            avg_output_tokens_per_request = Get-TaskspaceCostRatio $byMode.standard.output_tokens $byMode.standard.model_request_count
            jsonl_bytes = $byMode.standard.jsonl_bytes
            provider_input_tokens_per_jsonl_kb = Get-TaskspaceCostRatio $byMode.standard.input_tokens ([double]$byMode.standard.jsonl_bytes / 1024.0)
            provider_total_tokens_per_jsonl_kb = Get-TaskspaceCostRatio ([double]$byMode.standard.input_tokens + [double]$byMode.standard.output_tokens) ([double]$byMode.standard.jsonl_bytes / 1024.0)
        }
        taskspace = [pscustomobject]@{
            model_request_count = $byMode.taskspace.model_request_count
            avg_input_tokens_per_request = Get-TaskspaceCostRatio $byMode.taskspace.input_tokens $byMode.taskspace.model_request_count
            avg_output_tokens_per_request = Get-TaskspaceCostRatio $byMode.taskspace.output_tokens $byMode.taskspace.model_request_count
            jsonl_bytes = $byMode.taskspace.jsonl_bytes
            provider_input_tokens_per_jsonl_kb = Get-TaskspaceCostRatio $byMode.taskspace.input_tokens ([double]$byMode.taskspace.jsonl_bytes / 1024.0)
            provider_total_tokens_per_jsonl_kb = Get-TaskspaceCostRatio ([double]$byMode.taskspace.input_tokens + [double]$byMode.taskspace.output_tokens) ([double]$byMode.taskspace.jsonl_bytes / 1024.0)
        }
    }
    $control = [pscustomobject]@{
        schema_version = "taskspace-control-usage-aggregate-v1"
        scope = $Scope
        taskspace_control_count = $byMode.taskspace.taskspace_control_count
        taskspace_native_taskspace_control_count = $byMode.taskspace.native_taskspace_control_count
        taskspace_action_contract_taskspace_control_count = $byMode.taskspace.action_contract_taskspace_control_count
        state_commit_count = $byMode.taskspace.state_commit_count
        runtime_state_commit_count = $byMode.taskspace.runtime_state_commit_count
        runtime_output_ref_created_count = $byMode.taskspace.runtime_output_ref_created_count
        runtime_output_ref_slice_read_count = $byMode.taskspace.runtime_output_ref_slice_read_count
        taskspace_runtime_event_count = $byMode.taskspace.taskspace_runtime_event_count
        standard_taskspace_control_count = $byMode.standard.taskspace_control_count
        standard_native_taskspace_control_count = $byMode.standard.native_taskspace_control_count
        standard_action_contract_taskspace_control_count = $byMode.standard.action_contract_taskspace_control_count
        standard_runtime_state_commit_count = $byMode.standard.runtime_state_commit_count
        standard_runtime_output_ref_created_count = $byMode.standard.runtime_output_ref_created_count
        standard_runtime_output_ref_slice_read_count = $byMode.standard.runtime_output_ref_slice_read_count
        standard_taskspace_runtime_event_count = $byMode.standard.taskspace_runtime_event_count
    }
    $projection = [pscustomobject]@{
        schema_version = "taskspace-context-projection-aggregate-v1"
        scope = $Scope
        taskspace_projection_count = $byMode.taskspace.projection_count
        taskspace_projection_tokens = $byMode.taskspace.projection_tokens
        taskspace_projection_protected_miss_count = $byMode.taskspace.projection_protected_miss_count
        standard_projection_count = $byMode.standard.projection_count
        standard_projection_tokens = $byMode.standard.projection_tokens
        standard_projection_protected_miss_count = $byMode.standard.projection_protected_miss_count
        missing_taskspace_projection_count = $byMode.taskspace.missing_projection_count
        missing_taskspace_projection_tokens = $byMode.taskspace.missing_projection_tokens
        missing_taskspace_projection_protected_miss_count = $byMode.taskspace.missing_projection_protected_miss_count
    }
    $gate = New-TaskspaceCostGate $byMode.standard $byMode.taskspace
    $summary | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $tokenPath -Encoding UTF8
    $request | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $control | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $controlPath -Encoding UTF8
    $projection | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $projectionPath -Encoding UTF8
    $providerCacheTraceEventLines = @($providerCacheTrace.provider_cache_trace_events)
    if ($providerCacheTraceEventLines.Count -gt 0) {
        $providerCacheTraceEventLines | Set-Content -LiteralPath $providerCacheTracePath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($providerCacheTracePath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $providerCacheTrace.provider_cache_trace_summary | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $providerCacheTraceSummaryPath -Encoding UTF8
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $gatePath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        request_summary_path = $requestPath
        taskspace_control_usage_path = $controlPath
        context_projection_summary_path = $projectionPath
        provider_cache_trace_path = $providerCacheTracePath
        provider_cache_trace_summary_path = $providerCacheTraceSummaryPath
        suite_cost_gate_path = $gatePath
        provider_cache_trace_summary = $providerCacheTrace.provider_cache_trace_summary
        gate = $gate
    }
}
