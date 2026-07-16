if (-not (Get-Command ConvertTo-TaskspaceProjectionIdentity -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "provider-projection-identity.ps1")
}

function Get-TaskspaceProviderSectionKinds {
    @(
        "system_messages"
        "natural_history"
        "active_projection"
        "taskspace_control_feedback"
        "ordinary_tool_feedback"
        "tools"
        "tool_choice"
        "other_payload"
    )
}

function New-TaskspaceUnavailableProviderSectionCost {
    param([Parameter(Mandatory = $true)][string]$Reason)
    [pscustomobject]@{
        schema_version = "provider-wire-section-cost-v1"
        availability = "unavailable"
        unavailable_reason = $Reason
        section_bytes_total = $null
        active_projection_identity = New-TaskspaceUnavailableProjectionIdentity $Reason
        sections = @()
    }
}

function ConvertTo-TaskspaceProviderSectionInt64 {
    param($Value)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $null }
    try {
        $number = [int64]$Value
        if ($number -lt 0 -or [double]$Value -ne [double]$number) { return $null }
        $number
    } catch {
        $null
    }
}

function ConvertTo-TaskspaceProviderSectionCost {
    param([Parameter(Mandatory = $true)]$Shape)
    $traceSchema = [string](Get-TaskspaceCostProperty $Shape @("schema_version"))
    if ($traceSchema -eq "provider-chat-wire-trace-v2") {
        return New-TaskspaceUnavailableProviderSectionCost "historical_provider_wire_trace_v2"
    }
    if ($traceSchema -ne "provider-chat-wire-trace-v3") {
        return New-TaskspaceUnavailableProviderSectionCost "unsupported_provider_wire_trace_schema"
    }
    $raw = Get-TaskspaceCostProperty $Shape @("section_cost")
    if ($null -eq $raw) {
        return New-TaskspaceUnavailableProviderSectionCost "provider_wire_v3_section_cost_missing"
    }
    $availability = [string](Get-TaskspaceCostProperty $raw @("availability"))
    if ($availability -notin @("measured", "unavailable")) {
        return New-TaskspaceUnavailableProviderSectionCost "section_cost_availability_invalid"
    }
    if ([string](Get-TaskspaceCostProperty $raw @("schema_version")) -ne "provider-wire-section-cost-v1") {
        return New-TaskspaceUnavailableProviderSectionCost "unsupported_section_cost_schema"
    }
    $declaredTotal = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $raw @("section_bytes_total"))
    if ($null -eq $declaredTotal) {
        return New-TaskspaceUnavailableProviderSectionCost "section_bytes_total_invalid"
    }
    $providerPayloadBytes = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Shape @("provider_payload_bytes"))
    if ($null -eq $providerPayloadBytes) {
        return New-TaskspaceUnavailableProviderSectionCost "provider_payload_bytes_invalid"
    }
    $byKind = @{}
    $stableKinds = @(Get-TaskspaceProviderSectionKinds)
    foreach ($section in @((Get-TaskspaceCostProperty $raw @("sections")))) {
        $kind = [string](Get-TaskspaceCostProperty $section @("kind"))
        if ($kind -notin $stableKinds) {
            return New-TaskspaceUnavailableProviderSectionCost "section_kind_invalid"
        }
        if ($byKind.ContainsKey($kind)) {
            return New-TaskspaceUnavailableProviderSectionCost "section_kind_duplicate"
        }
        $count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("count"))
        $bytes = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("bytes"))
        $tokens = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("estimated_tokens"))
        if ($null -eq $count -or $null -eq $bytes -or $null -eq $tokens) {
            return New-TaskspaceUnavailableProviderSectionCost "section_measurement_invalid"
        }
        $byKind[$kind] = [pscustomobject]@{
            kind = $kind
            count = [int64]$count
            bytes = [int64]$bytes
            estimated_tokens = [int64]$tokens
            sha256 = Get-TaskspaceCostProperty $section @("sha256")
        }
    }
    $sections = New-Object System.Collections.Generic.List[object]
    $computedTotal = [int64]0
    foreach ($kind in $stableKinds) {
        if (-not $byKind.ContainsKey($kind)) {
            return New-TaskspaceUnavailableProviderSectionCost "section_kind_missing"
        }
        $normalized = $byKind[$kind]
        $computedTotal += [int64]$normalized.bytes
        $sections.Add($normalized)
    }
    if ($computedTotal -ne $declaredTotal) {
        return New-TaskspaceUnavailableProviderSectionCost "section_bytes_total_mismatch"
    }
    if ($declaredTotal -ne $providerPayloadBytes) {
        return New-TaskspaceUnavailableProviderSectionCost "section_bytes_payload_mismatch"
    }
    $reason = if ($availability -eq "unavailable") {
        $value = [string](Get-TaskspaceCostProperty $raw @("unavailable_reason"))
        if ([string]::IsNullOrWhiteSpace($value)) { "provider_reported_section_cost_unavailable" } else { $value }
    } else {
        $null
    }
    [pscustomobject]@{
        schema_version = "provider-wire-section-cost-v1"
        availability = $availability
        unavailable_reason = $reason
        section_bytes_total = [int64]$declaredTotal
        active_projection_identity = ConvertTo-TaskspaceProjectionIdentity (Get-TaskspaceCostProperty $raw @("active_projection_identity"))
        sections = @($sections.ToArray())
    }
}

function New-TaskspaceProviderSectionAccumulator {
    $sections = [ordered]@{}
    foreach ($kind in @(Get-TaskspaceProviderSectionKinds)) {
        $sections[$kind] = [ordered]@{ count = [int64]0; bytes = [int64]0; estimated_tokens = [int64]0 }
    }
    [ordered]@{
        measured_request_count = 0
        unavailable_request_count = 0
        section_bytes_total = [int64]0
        estimated_tokens_total = [int64]0
        unavailable_reason_counts = @{}
        sections = $sections
        projection_identity = [ordered]@{
            bootstrap_count = 0; active_count = 0; unavailable_count = 0
            unavailable_reason_counts = @{}; projection_sha256_counts = @{}; revision_counts = @{}
        }
    }
}

function Add-TaskspaceProviderSectionUnavailable {
    param($Accumulator, [int]$Count, [string]$Reason)
    if ($Count -le 0) { return }
    if ([string]::IsNullOrWhiteSpace($Reason)) { $Reason = "unknown" }
    $Accumulator.unavailable_request_count = [int]$Accumulator.unavailable_request_count + $Count
    if (-not $Accumulator.unavailable_reason_counts.ContainsKey($Reason)) {
        $Accumulator.unavailable_reason_counts[$Reason] = 0
    }
    $Accumulator.unavailable_reason_counts[$Reason] = [int]$Accumulator.unavailable_reason_counts[$Reason] + $Count
}

function Add-TaskspaceProviderMeasuredSections {
    param($Accumulator, [int]$RequestCount, [int64]$SectionBytesTotal, [object[]]$Sections)
    if ($RequestCount -le 0) { return }
    $Accumulator.measured_request_count = [int]$Accumulator.measured_request_count + $RequestCount
    $Accumulator.section_bytes_total = [int64]$Accumulator.section_bytes_total + $SectionBytesTotal
    foreach ($section in @($Sections)) {
        $kind = [string](Get-TaskspaceCostProperty $section @("kind"))
        if (-not $Accumulator.sections.Contains($kind)) { continue }
        $count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("count"))
        $bytes = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("bytes"))
        $tokens = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("estimated_tokens"))
        if ($null -eq $count -or $null -eq $bytes -or $null -eq $tokens) { continue }
        $Accumulator.sections[$kind].count = [int64]$Accumulator.sections[$kind].count + $count
        $Accumulator.sections[$kind].bytes = [int64]$Accumulator.sections[$kind].bytes + $bytes
        $Accumulator.sections[$kind].estimated_tokens = [int64]$Accumulator.sections[$kind].estimated_tokens + $tokens
        $Accumulator.estimated_tokens_total = [int64]$Accumulator.estimated_tokens_total + $tokens
    }
}

function ConvertFrom-TaskspaceProviderSectionAccumulator {
    param($Accumulator)
    $measured = [int]$Accumulator.measured_request_count
    $unavailable = [int]$Accumulator.unavailable_request_count
    $availability = if ($measured -gt 0 -and $unavailable -eq 0) {
        "measured"
    } elseif ($measured -gt 0) {
        "partial"
    } else {
        "unavailable"
    }
    $sections = @()
    if ($measured -gt 0) {
        $sections = @(Get-TaskspaceProviderSectionKinds | ForEach-Object {
            $value = $Accumulator.sections[$_]
            [pscustomobject]@{
                kind = $_
                count = [int64]$value.count
                bytes = [int64]$value.bytes
                estimated_tokens = [int64]$value.estimated_tokens
            }
        })
    }
    [pscustomobject]@{
        schema_version = "provider-wire-section-cost-summary-v1"
        availability = $availability
        request_count = $measured + $unavailable
        measured_request_count = $measured
        unavailable_request_count = $unavailable
        unavailable_reason_counts = Convert-TaskspaceCostTable $Accumulator.unavailable_reason_counts
        section_bytes_total = if ($measured -gt 0) { [int64]$Accumulator.section_bytes_total } else { $null }
        estimated_tokens_total = if ($measured -gt 0) { [int64]$Accumulator.estimated_tokens_total } else { $null }
        sections = $sections
        active_projection_identity_summary = [pscustomobject]@{
            schema_version = "provider-wire-active-projection-identity-summary-v1"
            bootstrap_count = [int]$Accumulator.projection_identity.bootstrap_count
            active_count = [int]$Accumulator.projection_identity.active_count
            unavailable_count = [int]$Accumulator.projection_identity.unavailable_count
            unavailable_reason_counts = Convert-TaskspaceCostTable $Accumulator.projection_identity.unavailable_reason_counts
            projection_sha256_counts = Convert-TaskspaceCostTable $Accumulator.projection_identity.projection_sha256_counts
            revision_counts = Convert-TaskspaceCostTable $Accumulator.projection_identity.revision_counts
            unique_projection_sha256_count = [int]$Accumulator.projection_identity.projection_sha256_counts.Count
            unique_revision_count = [int]$Accumulator.projection_identity.revision_counts.Count
        }
    }
}

function New-TaskspaceProviderSectionCostSummary {
    param([object[]]$SectionCosts)
    $accumulator = New-TaskspaceProviderSectionAccumulator
    foreach ($sectionCost in @($SectionCosts)) {
        Add-TaskspaceProviderProjectionIdentity $accumulator (Get-TaskspaceCostProperty $sectionCost @("active_projection_identity"))
        if ([string](Get-TaskspaceCostProperty $sectionCost @("availability")) -eq "measured") {
            Add-TaskspaceProviderMeasuredSections $accumulator 1 ([int64]$sectionCost.section_bytes_total) @($sectionCost.sections)
        } else {
            Add-TaskspaceProviderSectionUnavailable $accumulator 1 ([string](Get-TaskspaceCostProperty $sectionCost @("unavailable_reason")))
        }
    }
    ConvertFrom-TaskspaceProviderSectionAccumulator $accumulator
}

function Merge-TaskspaceProviderSectionCostSummaries {
    param([object[]]$CacheSummaries)
    $accumulator = New-TaskspaceProviderSectionAccumulator
    foreach ($cacheSummary in @($CacheSummaries)) {
        $providerRequestCount = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $cacheSummary @("provider_request_count"))
        if ($null -eq $providerRequestCount) { $providerRequestCount = 0 }
        $summary = Get-TaskspaceCostProperty $cacheSummary @("section_cost_summary")
        if ($null -eq $summary) {
            Add-TaskspaceProviderSectionUnavailable $accumulator ([int]$providerRequestCount) "section_cost_summary_missing"
            Add-TaskspaceProviderProjectionIdentitySummary $accumulator $null ([int]$providerRequestCount)
            continue
        }
        $measured = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $summary @("measured_request_count"))
        $unavailable = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $summary @("unavailable_request_count"))
        if ($null -eq $measured) { $measured = 0 }
        if ($null -eq $unavailable) { $unavailable = 0 }
        Add-TaskspaceProviderProjectionIdentitySummary $accumulator (Get-TaskspaceCostProperty $summary @("active_projection_identity_summary")) ([int]($measured + $unavailable))
        if ($measured -gt 0) {
            $sectionTotal = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $summary @("section_bytes_total"))
            $summarySections = @((Get-TaskspaceCostProperty $summary @("sections")))
            $computedTotal = [int64]0
            $sectionMeasurementValid = $true
            $seenKinds = @{}
            foreach ($section in $summarySections) {
                $kind = [string](Get-TaskspaceCostProperty $section @("kind"))
                $count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("count"))
                $bytes = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("bytes"))
                $tokens = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $section @("estimated_tokens"))
                if ($kind -notin @(Get-TaskspaceProviderSectionKinds) -or $seenKinds.ContainsKey($kind) -or
                    $null -eq $count -or $null -eq $bytes -or $null -eq $tokens) {
                    $sectionMeasurementValid = $false
                    break
                }
                $seenKinds[$kind] = $true
                $computedTotal += [int64]$bytes
            }
            if ($null -eq $sectionTotal -or -not $sectionMeasurementValid -or
                $seenKinds.Count -ne @(Get-TaskspaceProviderSectionKinds).Count -or $computedTotal -ne $sectionTotal) {
                Add-TaskspaceProviderSectionUnavailable $accumulator ([int]$measured) "section_cost_summary_total_mismatch"
            } else {
                Add-TaskspaceProviderMeasuredSections $accumulator ([int]$measured) ([int64]$sectionTotal) $summarySections
            }
        }
        $reasonCounts = Get-TaskspaceCostProperty $summary @("unavailable_reason_counts")
        $reasonCountTotal = 0
        foreach ($reasonProperty in $(if ($null -ne $reasonCounts) { @($reasonCounts.PSObject.Properties) } else { @() })) {
            Add-TaskspaceProviderSectionUnavailable $accumulator ([int]$reasonProperty.Value) ([string]$reasonProperty.Name)
            $reasonCountTotal += [int]$reasonProperty.Value
        }
        if ($unavailable -gt $reasonCountTotal) {
            Add-TaskspaceProviderSectionUnavailable $accumulator ([int]($unavailable - $reasonCountTotal)) "section_cost_unavailable_reason_missing"
        }
        $accounted = [int64]$measured + [int64]$unavailable
        if ($providerRequestCount -gt $accounted) {
            Add-TaskspaceProviderSectionUnavailable $accumulator ([int]($providerRequestCount - $accounted)) "section_cost_request_count_unaccounted"
        }
    }
    ConvertFrom-TaskspaceProviderSectionAccumulator $accumulator
}

function New-TaskspaceProviderWireCacheTraceArtifacts {
    param([Parameter(Mandatory = $true)][string]$TracePath)
    $shapes = @{}
    $terminals = @{}
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $TracePath -ErrorAction SilentlyContinue)) {
        if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
        try { $event = $line | ConvertFrom-Json } catch { continue }
        if ([string]$event.schema_version -notin @("provider-chat-wire-trace-v2", "provider-chat-wire-trace-v3")) { continue }
        $requestId = [string]$event.request_id
        if ([string]::IsNullOrWhiteSpace($requestId)) { continue }
        if ([string]$event.status -eq "payload_captured") {
            $shapes[$requestId] = $event
        } elseif ([string]$event.event_name -eq "provider.chat_wire_request_terminal") {
            $terminals[$requestId] = $event
        }
    }
    $events = New-Object System.Collections.Generic.List[object]
    $sectionCosts = New-Object System.Collections.Generic.List[object]
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
        if ($requestIndex -ge 2 -and $cacheShapeHash -ne $previousCacheShapeHash) { $cacheShapeTransitionCount++ }
        if ($requestIndex -ge 2 -and $null -ne $cachedTokens -and $null -ne $uncachedTokens) {
            $request2PlusHit += [int64]$cachedTokens
            $request2PlusMiss += [int64]$uncachedTokens
            $request2PlusCount++
        }
        if ($requestIndex -ge 2) {
            $prefixComparisonCount++
            if ([bool]$shape.prefix_preserved) { $prefixPreservedCount++ }
            $firstDiffPath = [string]$shape.first_diff_path
            if (-not [string]::IsNullOrWhiteSpace($firstDiffPath)) { Add-TaskspaceCostCount $firstDiffPathCounts $firstDiffPath }
        }
        $sectionCost = ConvertTo-TaskspaceProviderSectionCost $shape
        $sectionCosts.Add($sectionCost)
        $events.Add([pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceV3"
            request_id = $requestId; logical_request_id = $requestId; model_request_index = $requestIndex; attempt_seq = 1
            request_phase = "transport_observed"; task_id = ""; map_id = ""; node_id = ""
            provider_wire_api = [string]$shape.provider_wire_api; transport = "responses_http"
            tools_count = $toolsCount; tools_present = ($toolsCount -gt 0); request_shape_classifier = $classifier
            stable_prefix_hash = $cacheShapeHash; dynamic_suffix_hash = ""; messages_hash = [string]$shape.messages_hash
            tools_hash = [string]$shape.tools_hash; cache_shape_hash = $cacheShapeHash
            tool_choice_kind = [string]$shape.tool_choice_kind; tool_choice_name = [string]$shape.tool_choice_name
            provider_payload_sha256 = [string]$shape.provider_payload_sha256; pre_wire_payload_sha256 = [string]$shape.pre_wire_payload_sha256
            provider_payload_bytes = Convert-TaskspaceTraceInt $shape.provider_payload_bytes
            epoch_id = [string]$shape.epoch_id; previous_request_id = [string]$shape.previous_request_id
            message_count = Convert-TaskspaceTraceInt $shape.message_count; message_shapes = @($shape.message_shapes)
            lcp_message_count = Convert-TaskspaceTraceInt $shape.lcp_message_count; lcp_message_bytes = Convert-TaskspaceTraceInt $shape.lcp_message_bytes
            message_prefix_preserved = if ($requestIndex -ge 2) { [bool]$shape.message_prefix_preserved } else { $null }
            tool_choice_preserved = if ($requestIndex -ge 2) { [bool]$shape.tool_choice_preserved } else { $null }
            tool_choice_changed = if ($requestIndex -ge 2) { [bool]$shape.tool_choice_changed } else { $null }
            prefix_preserved = if ($requestIndex -ge 2) { [bool]$shape.prefix_preserved } else { $null }
            first_diff_index = Get-TaskspaceCostProperty $shape @("first_diff_index"); first_diff_path = [string]$shape.first_diff_path
            input_tokens = $inputTokens; cached_input_tokens = $cachedTokens; uncached_input_tokens = $uncachedTokens
            hit_rate = $hitRate; cache_hit_class = $cacheHitClass
            same_cache_shape_seen_before = [bool]$sameCacheShapeSeenBefore
            cache_warmup_candidate = [bool]$cacheWarmupCandidate; same_shape_zero_hit = [bool]$sameShapeZeroHit
            section_cost = $sectionCost
            status = if ($null -ne $terminal) { [string]$terminal.status } else { "terminal_missing" }
        })
        if (-not [string]::IsNullOrWhiteSpace($cacheShapeHash)) { $seenCacheShapes[$cacheShapeHash] = $true }
        $previousCacheShapeHash = $cacheShapeHash
    }
    $count = [int]$events.Count
    $covered = @($events.ToArray() | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_.provider_payload_sha256) -and [string]$_.status -ne "terminal_missing"
    }).Count
    $request2PlusDenominator = [double]$request2PlusHit + [double]$request2PlusMiss
    [pscustomobject]@{
        provider_cache_trace_events = @($events.ToArray())
        provider_cache_trace_summary = [pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceSummaryV3"; source = "provider_final_wire_trace"
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
            section_cost_summary = New-TaskspaceProviderSectionCostSummary @($sectionCosts.ToArray())
        }
    }
}
