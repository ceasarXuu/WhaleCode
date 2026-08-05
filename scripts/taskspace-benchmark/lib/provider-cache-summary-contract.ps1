function Test-TaskspaceProviderCacheNumberIdentity {
    param($Actual, [double]$Expected)
    if ($null -eq $Actual -or $Actual -is [bool]) { return $false }
    try {
        $number = [double]$Actual
        -not [double]::IsNaN($number) -and -not [double]::IsInfinity($number) -and
            [Math]::Abs($number - $Expected) -le 0.000001
    } catch { $false }
}

function Get-TaskspaceProviderCacheCountTableFacts {
    param($Value)
    $total = [int64]0
    if ($null -eq $Value) { return [pscustomobject]@{ valid = $false; total = $null; count = 0 } }
    $properties = @($Value.PSObject.Properties | ForEach-Object { $_ })
    foreach ($entry in $properties) {
        if (-not (Test-TaskspaceProviderCacheInt64 $entry.Value)) {
            return [pscustomobject]@{ valid = $false; total = $null; count = 0 }
        }
        $entryValue = [int64]$entry.Value
        if ($entryValue -gt [int64]::MaxValue - $total) {
            return [pscustomobject]@{ valid = $false; total = $null; count = 0 }
        }
        $total += $entryValue
    }
    [pscustomobject]@{ valid = $true; total = $total; count = $properties.Count }
}

function Test-TaskspaceProviderSectionSummaryContract {
    param($Summary, [int64]$ShapeCount, [System.Collections.Generic.List[string]]$Invalid)
    $get = { param($Value, [string]$Name) Get-TaskspaceCostProperty $Value @($Name) }
    if ([string](& $get $Summary "schema_version") -ne "provider-wire-section-cost-summary-v1") { $Invalid.Add("section_cost_summary.schema_version") }
    $availability = [string](& $get $Summary "availability")
    $requestCount = & $get $Summary "request_count"
    $measured = & $get $Summary "measured_request_count"
    $unavailable = & $get $Summary "unavailable_request_count"
    foreach ($entry in @(
            @{ name = "request_count"; value = $requestCount },
            @{ name = "measured_request_count"; value = $measured },
            @{ name = "unavailable_request_count"; value = $unavailable }
        )) {
        if (-not (Test-TaskspaceProviderCacheInt64 $entry.value)) { $Invalid.Add("section_cost_summary.$($entry.name)") }
    }
    $countsValid = (Test-TaskspaceProviderCacheInt64 $requestCount) -and
        (Test-TaskspaceProviderCacheInt64 $measured) -and (Test-TaskspaceProviderCacheInt64 $unavailable)
    if ($countsValid) {
        if ([int64]$measured + [int64]$unavailable -ne [int64]$requestCount -or [int64]$requestCount -ne $ShapeCount) {
            $Invalid.Add("section_cost_summary.request_count_identity")
        }
        $expectedAvailability = if ([int64]$measured -gt 0 -and [int64]$unavailable -eq 0) { "measured" } elseif ([int64]$measured -gt 0) { "partial" } else { "unavailable" }
        if ($availability -ne $expectedAvailability) { $Invalid.Add("section_cost_summary.availability") }
    } elseif ($availability -notin @("measured", "partial", "unavailable")) {
        $Invalid.Add("section_cost_summary.availability")
    }
    $reasonFacts = Get-TaskspaceProviderCacheCountTableFacts (& $get $Summary "unavailable_reason_counts")
    if (-not $reasonFacts.valid) { $Invalid.Add("section_cost_summary.unavailable_reason_counts") }
    elseif ($countsValid -and [int64]$reasonFacts.total -ne [int64]$unavailable) { $Invalid.Add("section_cost_summary.unavailable_reason_counts.total") }

    $sectionBytes = & $get $Summary "section_bytes_total"
    $sectionTokens = & $get $Summary "estimated_tokens_total"
    $sections = @((& $get $Summary "sections"))
    if ($countsValid -and [int64]$measured -gt 0) {
        if (-not (Test-TaskspaceProviderCacheInt64 $sectionBytes)) { $Invalid.Add("section_cost_summary.section_bytes_total") }
        if (-not (Test-TaskspaceProviderCacheInt64 $sectionTokens)) { $Invalid.Add("section_cost_summary.estimated_tokens_total") }
        $seen = @{}
        $computedBytes = [int64]0
        $computedTokens = [int64]0
        foreach ($section in $sections) {
            $kind = [string](& $get $section "kind")
            if ($kind -notin @(Get-TaskspaceProviderSectionKinds) -or $seen.ContainsKey($kind)) {
                $Invalid.Add("section_cost_summary.sections.kind")
                continue
            }
            $seen[$kind] = $true
            $count = & $get $section "count"
            $bytes = & $get $section "bytes"
            $tokens = & $get $section "estimated_tokens"
            foreach ($entry in @(@{ name = "count"; value = $count }, @{ name = "bytes"; value = $bytes }, @{ name = "estimated_tokens"; value = $tokens })) {
                if (-not (Test-TaskspaceProviderCacheInt64 $entry.value)) { $Invalid.Add("section_cost_summary.sections.$kind.$($entry.name)") }
            }
            $requestBytes = @((& $get $section "request_bytes"))
            $requestTokens = @((& $get $section "request_estimated_tokens"))
            $arraysValid = $requestBytes.Count -eq [int64]$measured -and $requestTokens.Count -eq [int64]$measured -and
                @($requestBytes | Where-Object { -not (Test-TaskspaceProviderCacheInt64 $_) }).Count -eq 0 -and
                @($requestTokens | Where-Object { -not (Test-TaskspaceProviderCacheInt64 $_) }).Count -eq 0
            if (-not $arraysValid) { $Invalid.Add("section_cost_summary.sections.$kind.request_arrays"); continue }
            $requestBytesTotal = [int64](($requestBytes | Measure-Object -Sum).Sum)
            $requestTokensTotal = [int64](($requestTokens | Measure-Object -Sum).Sum)
            if ((Test-TaskspaceProviderCacheInt64 $bytes) -and $requestBytesTotal -ne [int64]$bytes) { $Invalid.Add("section_cost_summary.sections.$kind.request_bytes.total") }
            if ((Test-TaskspaceProviderCacheInt64 $tokens) -and $requestTokensTotal -ne [int64]$tokens) { $Invalid.Add("section_cost_summary.sections.$kind.request_estimated_tokens.total") }
            if (-not (Test-TaskspaceProviderCacheInt64 (& $get $section "request_sample_count")) -or [int64](& $get $section "request_sample_count") -ne [int64]$measured) {
                $Invalid.Add("section_cost_summary.sections.$kind.request_sample_count")
            }
            if ((Test-TaskspaceProviderCacheInt64 $bytes) -and -not (Test-TaskspaceProviderCacheNumberIdentity (& $get $section "bytes_per_request_mean") ([double]$bytes / [double]$measured))) {
                $Invalid.Add("section_cost_summary.sections.$kind.bytes_per_request_mean")
            }
            if (-not (Test-TaskspaceProviderCacheNumberIdentity (& $get $section "bytes_per_request_median") (Get-TaskspaceProviderSectionMedian $requestBytes))) {
                $Invalid.Add("section_cost_summary.sections.$kind.bytes_per_request_median")
            }
            if ((Test-TaskspaceProviderCacheInt64 $tokens) -and -not (Test-TaskspaceProviderCacheNumberIdentity (& $get $section "estimated_tokens_per_request_mean") ([double]$tokens / [double]$measured))) {
                $Invalid.Add("section_cost_summary.sections.$kind.estimated_tokens_per_request_mean")
            }
            if (-not (Test-TaskspaceProviderCacheNumberIdentity (& $get $section "estimated_tokens_per_request_median") (Get-TaskspaceProviderSectionMedian $requestTokens))) {
                $Invalid.Add("section_cost_summary.sections.$kind.estimated_tokens_per_request_median")
            }
            if (Test-TaskspaceProviderCacheInt64 $bytes) { $computedBytes += [int64]$bytes }
            if (Test-TaskspaceProviderCacheInt64 $tokens) { $computedTokens += [int64]$tokens }
        }
        if ($seen.Count -ne @(Get-TaskspaceProviderSectionKinds).Count -or
            ((Test-TaskspaceProviderCacheInt64 $sectionBytes) -and $computedBytes -ne [int64]$sectionBytes) -or
            ((Test-TaskspaceProviderCacheInt64 $sectionTokens) -and $computedTokens -ne [int64]$sectionTokens)) {
            $Invalid.Add("section_cost_summary.sections.total")
        }
    } elseif ($null -ne $sectionBytes -or $null -ne $sectionTokens -or $sections.Count -gt 0) {
        $Invalid.Add("section_cost_summary.unavailable_values")
    }
}

function Test-TaskspaceBaseInstructionsSummaryContract {
    param($Summary, [int64]$ShapeCount, [System.Collections.Generic.List[string]]$Invalid)
    $get = { param($Value, [string]$Name) Get-TaskspaceCostProperty $Value @($Name) }
    if ([string](& $get $Summary "schema_version") -ne "WhaleCodeBaseInstructionsIdentitySummaryV1") { $Invalid.Add("base_instructions_identity_summary.schema_version") }
    $counts = @{}
    foreach ($field in @("request_count", "present_count", "absent_count", "invalid_count", "unavailable_count", "current_contract_match_count", "message_bytes_total", "estimated_tokens_total")) {
        $counts[$field] = & $get $Summary $field
        if (-not (Test-TaskspaceProviderCacheInt64 $counts[$field])) { $Invalid.Add("base_instructions_identity_summary.$field") }
    }
    if (@($counts.Values | Where-Object { -not (Test-TaskspaceProviderCacheInt64 $_) }).Count -eq 0) {
        if ([int64]$counts.request_count -ne $ShapeCount) { $Invalid.Add("base_instructions_identity_summary.request_count_identity") }
        if ([int64]$counts.present_count + [int64]$counts.absent_count + [int64]$counts.unavailable_count -gt [int64]$counts.request_count) { $Invalid.Add("base_instructions_identity_summary.classification_identity") }
        if ([int64]$counts.invalid_count -gt [int64]$counts.request_count -or [int64]$counts.current_contract_match_count -gt [int64]$counts.present_count) { $Invalid.Add("base_instructions_identity_summary.count_bounds") }
        $present = [int64]$counts.present_count
        $expectedRate = if ($present -gt 0) { [double]$counts.current_contract_match_count / [double]$present } else { $null }
        foreach ($entry in @(
                @{ field = "current_contract_match_rate"; total = $counts.current_contract_match_count },
                @{ field = "message_bytes_per_present_request_mean"; total = $counts.message_bytes_total },
                @{ field = "estimated_tokens_per_present_request_mean"; total = $counts.estimated_tokens_total }
            )) {
            $actual = & $get $Summary $entry.field
            $expected = if ($entry.field -eq "current_contract_match_rate") { $expectedRate } elseif ($present -gt 0) { [double]$entry.total / [double]$present } else { $null }
            if (($null -eq $expected -and $null -ne $actual) -or ($null -ne $expected -and -not (Test-TaskspaceProviderCacheNumberIdentity $actual $expected))) {
                $Invalid.Add("base_instructions_identity_summary.$($entry.field)")
            }
        }
    }
    foreach ($field in @("profile_counts", "version_counts", "sha256_counts", "message_index_counts", "wire_role_counts", "unavailable_reason_counts")) {
        $facts = Get-TaskspaceProviderCacheCountTableFacts (& $get $Summary $field)
        if (-not $facts.valid) { $Invalid.Add("base_instructions_identity_summary.$field") }
        elseif ($field -ne "unavailable_reason_counts" -and (Test-TaskspaceProviderCacheInt64 $counts.present_count) -and [int64]$facts.total -gt [int64]$counts.present_count) {
            $Invalid.Add("base_instructions_identity_summary.$field.total")
        }
    }
}

function Test-TaskspaceProviderCacheSummaryContract {
    param($Summary)
    $invalid = [System.Collections.Generic.List[string]]::new()
    $get = { param($Value, [string]$Name) Get-TaskspaceCostProperty $Value @($Name) }
    if ([string](& $get $Summary "schema_version") -ne "TaskSpaceProviderCacheTraceSummaryV4") { $invalid.Add("schema_version") }
    if ([string](& $get $Summary "source") -ne "provider_final_wire_trace") { $invalid.Add("source") }
    $availability = & $get $Summary "request_facts_availability"
    $availabilityValues = @{}
    foreach ($name in @("attempt", "boundary", "completion", "usage")) {
        $value = [string](& $get $availability $name)
        $availabilityValues[$name] = $value
        if ($value -notin @("measured", "unavailable", "incomparable")) { $invalid.Add("request_facts_availability.$name") }
    }
    $comparison = & $get $Summary "comparison_eligible"
    if ($comparison -isnot [bool]) { $invalid.Add("comparison_eligible") }
    $comparisonEligible = $comparison -is [bool] -and [bool]$comparison
    if ($comparisonEligible -and $availabilityValues.usage -ne "measured") { $invalid.Add("comparison_eligible.usage") }

    $requestCount = & $get $Summary "provider_request_count"
    if ($availabilityValues.boundary -eq "measured") {
        if ([string](& $get $Summary "provider_request_source") -ne "request_facts_boundary") { $invalid.Add("provider_request_source") }
        if (-not (Test-TaskspaceProviderCacheInt64 $requestCount)) { $invalid.Add("provider_request_count") }
    } elseif ([string](& $get $Summary "provider_request_source") -ne "request_facts_unavailable" -or $null -ne $requestCount) { $invalid.Add("provider_request_count") }
    foreach ($entry in @(
            @{ field = "provider_attempt_count"; availability = "attempt" },
            @{ field = "completed_response_count"; availability = "completion" },
            @{ field = "failed_or_cancelled_attempt_count"; availability = "completion" }
        )) {
        $value = & $get $Summary $entry.field
        if ($availabilityValues[$entry.availability] -eq "measured") {
            if (-not (Test-TaskspaceProviderCacheInt64 $value)) { $invalid.Add($entry.field) }
        } elseif ($null -ne $value) { $invalid.Add($entry.field) }
    }

    $shapeCount = & $get $Summary "shape_observation_count"
    if (-not (Test-TaskspaceProviderCacheInt64 $shapeCount)) { $invalid.Add("shape_observation_count") }
    $shapeFacts = Get-TaskspaceProviderCacheCountTableFacts (& $get $Summary "request_shape_counts")
    if (-not $shapeFacts.valid) { $invalid.Add("request_shape_counts") }
    elseif ((Test-TaskspaceProviderCacheInt64 $shapeCount) -and [int64]$shapeFacts.total -ne [int64]$shapeCount) { $invalid.Add("request_shape_counts.total") }
    if (-not (Test-TaskspaceProviderCacheCountTable (& $get $Summary "first_diff_path_counts"))) { $invalid.Add("first_diff_path_counts") }
    if ([string]::IsNullOrWhiteSpace([string](& $get $Summary "request_facts_analyzer_version"))) { $invalid.Add("request_facts_analyzer_version") }
    if ($Summary.PSObject.Properties.Name -notcontains "request_facts_findings") { $invalid.Add("request_facts_findings") }
    $classifierTotal = [int64]0
    foreach ($field in @("native_tools_schema_hot_path_count", "tool_free_action_contract_count", "unknown_or_unclassified_count")) {
        $value = & $get $Summary $field
        if (-not (Test-TaskspaceProviderCacheInt64 $value)) { $invalid.Add($field) } else { $classifierTotal += [int64]$value }
    }
    foreach ($field in @("prefix_comparison_count", "prefix_preserved_count", "tool_choice_transition_count", "cache_shape_transition_count")) {
        if (-not (Test-TaskspaceProviderCacheInt64 (& $get $Summary $field))) { $invalid.Add($field) }
    }
    if ((Test-TaskspaceProviderCacheInt64 $shapeCount) -and $classifierTotal -ne [int64]$shapeCount) { $invalid.Add("shape_classifier_count.total") }

    $traceCoverage = & $get $Summary "trace_coverage"
    if ($availabilityValues.attempt -eq "measured") {
        if (-not (Test-TaskspaceProviderCacheRatio $traceCoverage)) { $invalid.Add("trace_coverage") }
    } elseif ($null -ne $traceCoverage) { $invalid.Add("trace_coverage") }
    $prefixCount = & $get $Summary "prefix_comparison_count"
    $prefixKept = & $get $Summary "prefix_preserved_count"
    $prefixRate = & $get $Summary "prefix_preserved_rate"
    if ((Test-TaskspaceProviderCacheInt64 $prefixCount) -and (Test-TaskspaceProviderCacheInt64 $prefixKept)) {
        if ([int64]$prefixKept -gt [int64]$prefixCount) { $invalid.Add("prefix_preserved_count") }
        elseif ([int64]$prefixCount -eq 0 -and $null -ne $prefixRate) { $invalid.Add("prefix_preserved_rate") }
        elseif ([int64]$prefixCount -gt 0 -and -not (Test-TaskspaceProviderCacheNumberIdentity $prefixRate ([double]$prefixKept / [double]$prefixCount))) { $invalid.Add("prefix_preserved_rate") }
    }

    $cacheFields = @("cache_usage_missing_count", "request_2_plus_count", "request_2_plus_cached_input_tokens", "request_2_plus_uncached_input_tokens", "zero_cache_hit_count", "cache_warmup_candidate_count", "same_shape_zero_hit_count")
    if ($comparisonEligible) {
        foreach ($field in $cacheFields) { if (-not (Test-TaskspaceProviderCacheInt64 (& $get $Summary $field))) { $invalid.Add($field) } }
        if ((Test-TaskspaceProviderCacheInt64 (& $get $Summary "cache_usage_missing_count")) -and [int64](& $get $Summary "cache_usage_missing_count") -ne 0) { $invalid.Add("cache_usage_missing_count") }
        $hit = & $get $Summary "request_2_plus_cached_input_tokens"
        $miss = & $get $Summary "request_2_plus_uncached_input_tokens"
        $rate = & $get $Summary "request_2_plus_hit_rate"
        if ((Test-TaskspaceProviderCacheInt64 $hit) -and (Test-TaskspaceProviderCacheInt64 $miss)) {
            $denominator = [double]$hit + [double]$miss
            if (($denominator -eq 0 -and $null -ne $rate) -or ($denominator -gt 0 -and -not (Test-TaskspaceProviderCacheNumberIdentity $rate ([double]$hit / $denominator)))) { $invalid.Add("request_2_plus_hit_rate") }
        }
    } else {
        foreach ($field in $cacheFields + @("request_2_plus_hit_rate")) { if ($null -ne (& $get $Summary $field)) { $invalid.Add($field) } }
    }

    if (Test-TaskspaceProviderCacheInt64 $shapeCount) {
        Test-TaskspaceProviderSectionSummaryContract (& $get $Summary "section_cost_summary") ([int64]$shapeCount) $invalid
        Test-TaskspaceBaseInstructionsSummaryContract (& $get $Summary "base_instructions_identity_summary") ([int64]$shapeCount) $invalid
    }
    [pscustomobject]@{ valid = $invalid.Count -eq 0; invalid_fields = @($invalid.ToArray() | Sort-Object -Unique) }
}
