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

function Get-TaskspaceCostJsonlRows {
    param([string]$Path)
    $rows = New-Object System.Collections.Generic.List[object]
    $parseErrors = 0
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ rows = @(); parse_errors = 0; source_status = "missing" }
    }
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $Path)) {
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
    param([string]$ObservabilityJsonPath, [string[]]$Kinds)
    $events = New-Object System.Collections.Generic.List[object]
    if ([string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -or -not (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        return @()
    }
    try {
        $obs = (Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath) | ConvertFrom-Json
        foreach ($event in @($obs.timeline)) {
            $kind = [string](Get-TaskspaceTraceField $event @("kind"))
            if ($Kinds -contains $kind) { $events.Add($event) }
        }
    } catch {}
    @($events.ToArray())
}

function Convert-TaskspaceTraceInt {
    param($Value, [int]$Default = 0)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $Default }
    try { return [int]$Value } catch { return $Default }
}

function Convert-TaskspaceTraceBool {
    param($Value, [bool]$Default = $false)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $Default }
    $text = [string]$Value
    if ($text -ieq "true") { return $true }
    if ($text -ieq "false") { return $false }
    return $Default
}

function New-TaskspaceBudgetArtifacts {
    param([string]$ObservabilityJsonPath)
    $budgetEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("provider_request_budget"))) {
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
            request_count_before = Convert-TaskspaceTraceInt $tags.request_count_before
            request_count_after = Convert-TaskspaceTraceInt $tags.request_count_after
            max_requests = Convert-TaskspaceTraceInt $tags.max_requests
            budget_response_action_taken = Convert-TaskspaceTraceBool $tags.budget_response_action_taken $false
            provider_payload_sha256 = [string]$tags.provider_payload_sha256
            provider_payload_bytes = Convert-TaskspaceTraceInt $tags.provider_payload_bytes
            exact_payload_scan_passed = Convert-TaskspaceTraceBool $tags.exact_payload_scan_passed $false
            active_projection_present = Convert-TaskspaceTraceBool $tags.active_projection_present $false
            legacy_taskspace_history_present = Convert-TaskspaceTraceBool $tags.legacy_taskspace_history_present $false
            large_raw_output_tokens = Convert-TaskspaceTraceInt $tags.large_raw_output_tokens
            protected_items_present = Convert-TaskspaceTraceBool $tags.protected_items_present $false
            replacement_confirmed = Convert-TaskspaceTraceBool $tags.replacement_confirmed $false
        })
    }
    $qualityEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @(Get-TaskspaceTraceEvents $ObservabilityJsonPath @("budget_quality_impact"))) {
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
            request_phase = [string]$tags.request_phase
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
        budget_quality_impact_events = @($qualityEvents.ToArray())
        budget_quality_impact_summary = $summary
    }
}

function New-TaskspaceActiveReplacementArtifacts {
    param([object[]]$BudgetEvents)
    $scanEvents = New-Object System.Collections.Generic.List[object]
    foreach ($event in @($BudgetEvents)) {
        if ([string]::IsNullOrWhiteSpace([string]$event.provider_payload_sha256)) { continue }
        if (-not [bool]$event.active_projection_present -and -not [bool]$event.legacy_taskspace_history_present) { continue }
        $scanId = "scan-$($event.trace_event_id)"
        $passed = [bool]$event.exact_payload_scan_passed `
            -and [bool]$event.replacement_confirmed `
            -and -not [bool]$event.legacy_taskspace_history_present `
            -and [int]$event.large_raw_output_tokens -eq 0 `
            -and [bool]$event.protected_items_present
        $scanEvents.Add([pscustomobject]@{
            schema_version = "taskspace-exact-payload-scan-event-v1"
            scan_event_id = $scanId
            provider_budget_trace_event_id = [string]$event.trace_event_id
            request_id = [string]$event.request_id
            provider_payload_sha256 = [string]$event.provider_payload_sha256
            provider_payload_bytes = [int]$event.provider_payload_bytes
            passed = [bool]$passed
            active_projection_present = [bool]$event.active_projection_present
            legacy_taskspace_history_present = [bool]$event.legacy_taskspace_history_present
            large_raw_output_tokens = [int]$event.large_raw_output_tokens
            protected_items_present = [bool]$event.protected_items_present
            replacement_confirmed = [bool]$event.replacement_confirmed
        })
    }
    $selected = @($scanEvents.ToArray() | Where-Object { [bool]$_.passed } | Select-Object -First 1)
    if ($selected.Count -eq 0) {
        $selected = @($scanEvents.ToArray() | Select-Object -First 1)
    }
    $first = if ($selected.Count -gt 0) { $selected[0] } else { $null }
    $report = [pscustomobject]@{
        schema_version = "taskspace-active-context-replacement-report-v1"
        provider_payload_available = ($null -ne $first -and -not [string]::IsNullOrWhiteSpace([string]$first.provider_payload_sha256))
        request_id = if ($null -ne $first) { [string]$first.request_id } else { "" }
        provider_payload_sha256 = if ($null -ne $first) { [string]$first.provider_payload_sha256 } else { "" }
        exact_payload_scan_passed = if ($null -ne $first) { [bool]$first.passed } else { $false }
        exact_payload_scan_event_id = if ($null -ne $first) { [string]$first.scan_event_id } else { "" }
        replacement_confirmed = if ($null -ne $first) { [bool]$first.replacement_confirmed } else { $false }
        legacy_taskspace_history_present = if ($null -ne $first) { [bool]$first.legacy_taskspace_history_present } else { $true }
        large_raw_output_tokens = if ($null -ne $first) { [int]$first.large_raw_output_tokens } else { 0 }
        protected_items_present = if ($null -ne $first) { [bool]$first.protected_items_present } else { $false }
    }
    [pscustomobject]@{
        exact_payload_scan_events = @($scanEvents.ToArray())
        active_context_replacement_report = $report
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
    $parsed = Get-TaskspaceCostJsonlRows $RolloutJsonlPath
    $inputValues = New-Object System.Collections.Generic.List[Int64]
    $outputValues = New-Object System.Collections.Generic.List[Int64]
    $cachedValues = New-Object System.Collections.Generic.List[Int64]
    foreach ($row in @($parsed.rows)) {
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
    $inputArray = @($inputValues.ToArray())
    $outputArray = @($outputValues.ToArray())
    $cachedArray = @($cachedValues.ToArray())
    [pscustomobject]@{
        source_path = $RolloutJsonlPath
        source_status = [string]$parsed.source_status
        parse_errors = [int]$parsed.parse_errors
        availability = if ($parsed.source_status -ne "read") { "source_missing" } elseif ($inputArray.Count -eq 0 -and $outputArray.Count -eq 0) { "usage_unavailable" } else { "measured" }
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
    [pscustomobject]@{
        schema_version = "taskspace-request-summary-v1"
        source_path = $JsonlPath
        rollout_source_path = $RolloutJsonlPath
        availability = [string]$TokenSummary.availability
        model_request_count = $TokenSummary.model_request_count
        avg_input_tokens_per_request = if ($TokenSummary.model_request_count -and $TokenSummary.input_tokens -ne $null) { [Math]::Round([double]$TokenSummary.input_tokens / [double]$TokenSummary.model_request_count, 4) } else { $null }
        avg_output_tokens_per_request = if ($TokenSummary.model_request_count -and $TokenSummary.output_tokens -ne $null) { [Math]::Round([double]$TokenSummary.output_tokens / [double]$TokenSummary.model_request_count, 4) } else { $null }
        max_input_tokens_per_request = $TokenSummary.request_distribution.max_input_tokens
        p95_input_tokens_per_request = $TokenSummary.request_distribution.p95_input_tokens
        first_input_tokens_per_request = $TokenSummary.request_distribution.first_input_tokens
        last_input_tokens_per_request = $TokenSummary.request_distribution.last_input_tokens
        max_output_tokens_per_request = $TokenSummary.request_distribution.max_output_tokens
        p95_output_tokens_per_request = $TokenSummary.request_distribution.p95_output_tokens
        first_output_tokens_per_request = $TokenSummary.request_distribution.first_output_tokens
        last_output_tokens_per_request = $TokenSummary.request_distribution.last_output_tokens
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
    param([string]$JsonlPath, [string]$ObservabilityJsonPath = "")
    $parsed = Get-TaskspaceCostJsonlRows $JsonlPath
    $actions = @{}
    $total = 0
    $stateCommit = 0
    function Visit-ControlValue($Current) {
        if ($null -eq $Current -or $Current -is [string] -or $Current -is [ValueType]) { return }
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
            $script:taskspaceCostControlTotal++
            if ($action -eq "state_commit") { $script:taskspaceCostStateCommit++ }
            Add-TaskspaceCostCount $script:taskspaceCostActions $action
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
    foreach ($row in @($parsed.rows)) { Visit-ControlValue $row }
    $total = $script:taskspaceCostControlTotal
    $stateCommit = $script:taskspaceCostStateCommit
    Remove-Variable -Name taskspaceCostActions -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostControlTotal -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostStateCommit -Scope Script -ErrorAction SilentlyContinue
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
        schema_version = "taskspace-control-usage-v1"
        source_path = $JsonlPath
        observability_source_path = $ObservabilityJsonPath
        source_status = [string]$parsed.source_status
        observability_source_status = $runtimeSourceStatus
        parse_errors = [int]$parsed.parse_errors
        availability = if ($parsed.source_status -eq "read") { "measured" } else { "source_missing" }
        taskspace_control_count = [int]$total
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
    foreach ($path in @($JsonlPath, $ObservabilityJsonPath, $RolloutJsonlPath)) {
        if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path)) { continue }
        $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        $unescaped = $raw -replace "\\r\\n", "`n" -replace "\\n", "`n"
        $texts.Add($unescaped)
    }
    $blocks = New-Object System.Collections.Generic.List[string]
    foreach ($text in @($texts.ToArray())) {
        foreach ($match in [regex]::Matches($text, "(?s)ContextProjectionV1 (?:active replacement|shadow \(not active replacement\)):.*?- estimated_tokens:\s*\d+")) {
            $block = [string]$match.Value
            if (-not [string]::IsNullOrWhiteSpace($block)) { $blocks.Add($block) }
        }
    }
    @($blocks.ToArray() | Select-Object -Unique)
}

function New-TaskspaceContextProjectionEvent {
    param([Parameter(Mandatory = $true)][string]$Block)
    $requiredSections = @(
        "success_criteria",
        "current_node",
        "blockers",
        "decisions",
        "facts",
        "relevant_results",
        "next_valid_actions",
        "hidden_refs_available"
    )
    $missing = @($requiredSections | Where-Object { $Block -notmatch "(?m)^\s*$([regex]::Escape($_)):" })
    $projectionId = ""
    $taskId = ""
    $mode = ""
    $projectionKind = if ($Block -match "ContextProjectionV1 active replacement:") { "active_replacement" } elseif ($Block -match "ContextProjectionV1 shadow \(not active replacement\):") { "shadow" } else { "unknown" }
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
    $events = @(Get-TaskspaceContextProjectionBlocks $JsonlPath $ObservabilityJsonPath $RolloutJsonlPath | ForEach-Object {
            New-TaskspaceContextProjectionEvent $_
        })
    $tokenValues = @($events | Where-Object { $null -ne $_.estimated_tokens } | ForEach-Object { [int64]$_.estimated_tokens })
    $tokenTotal = [int64]0
    foreach ($value in $tokenValues) { $tokenTotal += [int64]$value }
    $protectedMiss = 0
    foreach ($event in $events) { $protectedMiss += [int]$event.protected_miss_count }
    $activeProjectionCount = @($events | Where-Object { [string]$_.projection_kind -eq "active_replacement" }).Count
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
    $token = New-TaskspaceTokenSummary $JsonlPath
    $request = New-TaskspaceRequestSummary $JsonlPath $token $rolloutJsonlPath
    $visibility = New-TaskspaceProviderInputVisibilitySummary $JsonlPath $token
    $control = New-TaskspaceControlUsageSummary $JsonlPath $ObservabilityJsonPath
    $replay = New-TaskspaceReplaySummary $ArtifactDir
    $outputRefEvents = @(New-TaskspaceOutputRefEvents $ObservabilityJsonPath $ArtifactDir)
    $projection = New-TaskspaceContextProjectionSummary $JsonlPath $ObservabilityJsonPath $rolloutJsonlPath
    $budget = New-TaskspaceBudgetArtifacts $ObservabilityJsonPath
    $activeReplacement = New-TaskspaceActiveReplacementArtifacts $budget.budget_events
    $tokenPath = Join-Path $ArtifactDir "token-summary.json"
    $requestPath = Join-Path $ArtifactDir "request-summary.json"
    $visibilityPath = Join-Path $ArtifactDir "provider-input-visibility.json"
    $controlPath = Join-Path $ArtifactDir "taskspace-control-usage.json"
    $projectionPath = Join-Path $ArtifactDir "context-projection-summary.json"
    $projectionEventsPath = Join-Path $ArtifactDir "projection-events.jsonl"
    $outputRefEventsPath = Join-Path $ArtifactDir "output-ref-events.jsonl"
    $budgetEventsPath = Join-Path $ArtifactDir "budget-events.jsonl"
    $budgetQualityImpactEventsPath = Join-Path $ArtifactDir "budget-quality-impact-events.jsonl"
    $budgetQualityImpactSummaryPath = Join-Path $ArtifactDir "budget-induced-quality-impact-summary.json"
    $exactPayloadScanEventsPath = Join-Path $ArtifactDir "exact-payload-scan-events.jsonl"
    $activeReplacementReportPath = Join-Path $ArtifactDir "active-context-replacement-report.json"
    $token | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $tokenPath -Encoding UTF8
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
    $budgetQualityImpactEventLines = @($budget.budget_quality_impact_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($budgetQualityImpactEventLines.Count -gt 0) {
        $budgetQualityImpactEventLines | Set-Content -LiteralPath $budgetQualityImpactEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($budgetQualityImpactEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $budget.budget_quality_impact_summary | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $budgetQualityImpactSummaryPath -Encoding UTF8
    $exactPayloadScanEventLines = @($activeReplacement.exact_payload_scan_events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
    if ($exactPayloadScanEventLines.Count -gt 0) {
        $exactPayloadScanEventLines | Set-Content -LiteralPath $exactPayloadScanEventsPath -Encoding UTF8
    } else {
        [System.IO.File]::WriteAllText($exactPayloadScanEventsPath, "", [System.Text.UTF8Encoding]::new($false))
    }
    $activeReplacement.active_context_replacement_report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $activeReplacementReportPath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        request_summary_path = $requestPath
        provider_input_visibility_path = $visibilityPath
        taskspace_control_usage_path = $controlPath
        context_projection_summary_path = $projectionPath
        projection_events_path = $projectionEventsPath
        output_ref_events_path = $outputRefEventsPath
        budget_events_path = $budgetEventsPath
        budget_quality_impact_events_path = $budgetQualityImpactEventsPath
        budget_quality_impact_summary_path = $budgetQualityImpactSummaryPath
        exact_payload_scan_events_path = $exactPayloadScanEventsPath
        active_context_replacement_report_path = $activeReplacementReportPath
        token_summary = $token
        request_summary = $request
        provider_input_visibility = $visibility
        taskspace_control_usage = $control
        replay_summary = $replay
        output_ref_events = $outputRefEvents
        context_projection_summary = $projection
        budget_events = $budget.budget_events
        budget_quality_impact_events = $budget.budget_quality_impact_events
        budget_quality_impact_summary = $budget.budget_quality_impact_summary
        exact_payload_scan_events = $activeReplacement.exact_payload_scan_events
        active_context_replacement_report = $activeReplacement.active_context_replacement_report
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
        missing_state_commit_count = 0
        missing_runtime_state_commit_count = 0
        missing_runtime_output_ref_created_count = 0
        missing_runtime_output_ref_slice_read_count = 0
        missing_taskspace_runtime_event_count = 0
        missing_large_output_replay_count = 0
        missing_projection_count = 0
        missing_projection_tokens = 0
        missing_projection_protected_miss_count = 0
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
        foreach ($field in @("model_request_count", "input_tokens", "output_tokens", "cached_input_tokens", "uncached_input_tokens", "jsonl_bytes", "provider_input_tokens_per_jsonl_kb", "provider_total_tokens_per_jsonl_kb", "wall_time_ms", "taskspace_control_count", "state_commit_count", "runtime_state_commit_count", "runtime_output_ref_created_count", "runtime_output_ref_slice_read_count", "taskspace_runtime_event_count", "large_output_replay_count", "projection_count", "projection_tokens", "projection_protected_miss_count")) {
            Add-TaskspaceCostMetricTotal $totals $metric $field
        }
    }
    $tokenPath = Join-Path $RootDir "token-summary.json"
    $requestPath = Join-Path $RootDir "request-summary.json"
    $controlPath = Join-Path $RootDir "taskspace-control-usage.json"
    $projectionPath = Join-Path $RootDir "context-projection-summary.json"
    $gatePath = Join-Path $RootDir "suite-cost-gate.json"
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
        state_commit_count = $byMode.taskspace.state_commit_count
        runtime_state_commit_count = $byMode.taskspace.runtime_state_commit_count
        runtime_output_ref_created_count = $byMode.taskspace.runtime_output_ref_created_count
        runtime_output_ref_slice_read_count = $byMode.taskspace.runtime_output_ref_slice_read_count
        taskspace_runtime_event_count = $byMode.taskspace.taskspace_runtime_event_count
        standard_taskspace_control_count = $byMode.standard.taskspace_control_count
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
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $gatePath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        request_summary_path = $requestPath
        taskspace_control_usage_path = $controlPath
        context_projection_summary_path = $projectionPath
        suite_cost_gate_path = $gatePath
        gate = $gate
    }
}
