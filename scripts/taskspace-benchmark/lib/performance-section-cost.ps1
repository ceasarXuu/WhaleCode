function Get-PerformanceProviderSectionKinds {
    @(
        "system_messages"
        "natural_history"
        "active_projection"
        "ordinary_tool_feedback"
        "base_instructions"
        "tools"
        "tool_choice"
        "other_payload"
    )
}

function New-PerformanceSectionReasonCounts {
    param([string]$Reason, $Count)
    $counts = [ordered]@{}
    $number = Get-PerformanceNumber $Count
    if (-not [string]::IsNullOrWhiteSpace($Reason) -and $null -ne $number -and $number -gt 0) {
        $counts[$Reason] = [int64]$number
    }
    [pscustomobject]$counts
}

function New-PerformanceProjectionIdentitySummary {
    param([string]$Reason, $Count)
    $number = Get-PerformanceNumber $Count
    [pscustomobject]@{
        schema_version = "provider-wire-active-projection-identity-summary-v1"
        bootstrap_count = 0
        active_count = 0
        unavailable_count = $number
        unavailable_reason_counts = New-PerformanceSectionReasonCounts $Reason $number
        projection_sha256_counts = [pscustomobject]@{}
        revision_counts = [pscustomobject]@{}
        unique_projection_sha256_count = 0
        unique_revision_count = 0
    }
}

function Add-PerformanceProjectionCount {
    param([hashtable]$Table, [string]$Key, $Count)
    $number = Get-PerformanceNumber $Count
    if ([string]::IsNullOrWhiteSpace($Key) -or $null -eq $number -or $number -le 0) { return }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = [double]0 }
    $Table[$Key] += $number
}

function Get-PerformanceSectionMedian {
    param([object[]]$Values)
    $ordered = @($Values | ForEach-Object { [double]$_ } | Sort-Object)
    if ($ordered.Count -eq 0) { return $null }
    $middle = [Math]::Floor($ordered.Count / 2)
    if ($ordered.Count % 2 -eq 1) { return $ordered[$middle] }
    [Math]::Round(($ordered[$middle - 1] + $ordered[$middle]) / 2.0, 6)
}

function ConvertFrom-PerformanceProjectionAccumulator {
    param($Accumulator)
    $orderedReasons = [ordered]@{}
    $orderedHashes = [ordered]@{}
    $orderedRevisions = [ordered]@{}
    foreach ($key in @($Accumulator.unavailable_reason_counts.Keys | Sort-Object)) { $orderedReasons[$key] = $Accumulator.unavailable_reason_counts[$key] }
    foreach ($key in @($Accumulator.projection_sha256_counts.Keys | Sort-Object)) { $orderedHashes[$key] = $Accumulator.projection_sha256_counts[$key] }
    foreach ($key in @($Accumulator.revision_counts.Keys | Sort-Object { [int64]$_ })) { $orderedRevisions[$key] = $Accumulator.revision_counts[$key] }
    [pscustomobject]@{
        schema_version = "provider-wire-active-projection-identity-summary-v1"
        bootstrap_count = $Accumulator.bootstrap_count
        active_count = $Accumulator.active_count
        unavailable_count = $Accumulator.unavailable_count
        unavailable_reason_counts = [pscustomobject]$orderedReasons
        projection_sha256_counts = [pscustomobject]$orderedHashes
        revision_counts = [pscustomobject]$orderedRevisions
        unique_projection_sha256_count = $orderedHashes.Count
        unique_revision_count = $orderedRevisions.Count
    }
}

function Get-PerformanceSectionCostFacts {
    param($CacheSummary)
    $providerRequestCount = Get-PerformanceNumber (Get-PerformanceProperty $CacheSummary "provider_request_count")
    $summary = Get-PerformanceProperty $CacheSummary "section_cost_summary"
    if ($null -eq $summary) {
        return [pscustomobject]@{
            availability = "unavailable"
            request_count = $providerRequestCount
            measured_request_count = if ($null -ne $providerRequestCount) { 0 } else { $null }
            unavailable_request_count = $providerRequestCount
            unavailable_reason_counts = New-PerformanceSectionReasonCounts "section_cost_summary_missing" $providerRequestCount
            section_bytes_total = $null
            estimated_tokens_total = $null
            section_bytes_reconciled = $null
            sections = @()
            active_projection_identity_summary = New-PerformanceProjectionIdentitySummary "section_cost_summary_missing" $providerRequestCount
        }
    }
    $measured = Get-PerformanceNumber (Get-PerformanceProperty $summary "measured_request_count")
    $unavailable = Get-PerformanceNumber (Get-PerformanceProperty $summary "unavailable_request_count")
    $sectionTotal = Get-PerformanceNumber (Get-PerformanceProperty $summary "section_bytes_total")
    $sections = @((Get-PerformanceProperty $summary "sections" @()))
    $computedTotal = [double]0
    $validSections = $true
    $seenKinds = @{}
    foreach ($section in $sections) {
        $kind = [string](Get-PerformanceProperty $section "kind")
        $count = Get-PerformanceNumber (Get-PerformanceProperty $section "count")
        $bytes = Get-PerformanceNumber (Get-PerformanceProperty $section "bytes")
        $tokens = Get-PerformanceNumber (Get-PerformanceProperty $section "estimated_tokens")
        $requestBytes = @((Get-PerformanceProperty $section "request_bytes" @()))
        $requestTokens = @((Get-PerformanceProperty $section "request_estimated_tokens" @()))
        if ($kind -notin @(Get-PerformanceProviderSectionKinds) -or $seenKinds.ContainsKey($kind) -or
            $null -eq $count -or $null -eq $bytes -or $null -eq $tokens -or
            ($null -ne $measured -and ($requestBytes.Count -ne $measured -or $requestTokens.Count -ne $measured))) {
            $validSections = $false
            break
        }
        $seenKinds[$kind] = $true
        $computedTotal += $bytes
    }
    $reconciled = if ($null -ne $measured -and $measured -gt 0) {
        $null -ne $sectionTotal -and $validSections -and
        $seenKinds.Count -eq @(Get-PerformanceProviderSectionKinds).Count -and $computedTotal -eq $sectionTotal
    } else {
        $null
    }
    if ($reconciled -eq $false) {
        $requestCount = if ($null -ne $providerRequestCount) { $providerRequestCount } else { $measured + $unavailable }
        return [pscustomobject]@{
            availability = "unavailable"
            request_count = $requestCount
            measured_request_count = 0
            unavailable_request_count = $requestCount
            unavailable_reason_counts = New-PerformanceSectionReasonCounts "section_cost_summary_total_mismatch" $requestCount
            section_bytes_total = $null
            estimated_tokens_total = $null
            section_bytes_reconciled = $false
            sections = @()
            active_projection_identity_summary = New-PerformanceProjectionIdentitySummary "section_cost_summary_total_mismatch" $requestCount
        }
    }
    [pscustomobject]@{
        availability = [string](Get-PerformanceProperty $summary "availability" "unavailable")
        request_count = Get-PerformanceNumber (Get-PerformanceProperty $summary "request_count" $providerRequestCount)
        measured_request_count = $measured
        unavailable_request_count = $unavailable
        unavailable_reason_counts = Get-PerformanceProperty $summary "unavailable_reason_counts" ([pscustomobject]@{})
        section_bytes_total = $sectionTotal
        estimated_tokens_total = Get-PerformanceNumber (Get-PerformanceProperty $summary "estimated_tokens_total")
        section_bytes_reconciled = $reconciled
        sections = $sections
        active_projection_identity_summary = Get-PerformanceProperty $summary "active_projection_identity_summary" (New-PerformanceProjectionIdentitySummary "active_projection_identity_summary_missing" (Get-PerformanceProperty $summary "request_count" $providerRequestCount))
    }
}

function Get-PerformanceModeSectionCostAggregate {
    param([object[]]$Rows)
    $measured = [double]0
    $unavailable = [double]0
    $knownRequestCount = [double]0
    $knownRequestSides = 0
    $unknownSides = 0
    $sectionTotal = [double]0
    $tokenTotal = [double]0
    $reasonCounts = @{}
    $sectionTotals = @{}
    $projection = [ordered]@{
        bootstrap_count = [double]0; active_count = [double]0; unavailable_count = [double]0
        unavailable_reason_counts = @{}; projection_sha256_counts = @{}; revision_counts = @{}
    }
    foreach ($row in @($Rows)) {
        $facts = $row.section_cost
        $requestCount = Get-PerformanceNumber (Get-PerformanceProperty $facts "request_count")
        if ($null -eq $requestCount) { $unknownSides++ } else { $knownRequestCount += $requestCount; $knownRequestSides++ }
        $sideMeasured = Get-PerformanceNumber (Get-PerformanceProperty $facts "measured_request_count")
        $sideUnavailable = Get-PerformanceNumber (Get-PerformanceProperty $facts "unavailable_request_count")
        if ($null -ne $sideMeasured) { $measured += $sideMeasured }
        if ($null -ne $sideUnavailable) { $unavailable += $sideUnavailable }
        $bytes = Get-PerformanceNumber (Get-PerformanceProperty $facts "section_bytes_total")
        $tokens = Get-PerformanceNumber (Get-PerformanceProperty $facts "estimated_tokens_total")
        if ($null -ne $bytes) { $sectionTotal += $bytes }
        if ($null -ne $tokens) { $tokenTotal += $tokens }
        foreach ($reason in @((Get-PerformanceProperty $facts "unavailable_reason_counts" ([pscustomobject]@{})).PSObject.Properties)) {
            if (-not $reasonCounts.ContainsKey($reason.Name)) { $reasonCounts[$reason.Name] = [double]0 }
            $reasonCounts[$reason.Name] += [double]$reason.Value
        }
        foreach ($section in @((Get-PerformanceProperty $facts "sections" @()))) {
            $kind = [string](Get-PerformanceProperty $section "kind")
            if ([string]::IsNullOrWhiteSpace($kind)) { continue }
            if (-not $sectionTotals.ContainsKey($kind)) {
                $sectionTotals[$kind] = [ordered]@{
                    count = [double]0; bytes = [double]0; estimated_tokens = [double]0
                    request_bytes = New-Object System.Collections.Generic.List[object]
                    request_estimated_tokens = New-Object System.Collections.Generic.List[object]
                }
            }
            foreach ($field in @("count", "bytes", "estimated_tokens")) {
                $value = Get-PerformanceNumber (Get-PerformanceProperty $section $field)
                if ($null -ne $value) { $sectionTotals[$kind][$field] += $value }
            }
            foreach ($value in @((Get-PerformanceProperty $section "request_bytes" @()))) { $sectionTotals[$kind].request_bytes.Add([double]$value) }
            foreach ($value in @((Get-PerformanceProperty $section "request_estimated_tokens" @()))) { $sectionTotals[$kind].request_estimated_tokens.Add([double]$value) }
        }
        $identity = Get-PerformanceProperty $facts "active_projection_identity_summary" (New-PerformanceProjectionIdentitySummary "active_projection_identity_summary_missing" $requestCount)
        foreach ($kind in @("bootstrap", "active", "unavailable")) {
            $value = Get-PerformanceNumber (Get-PerformanceProperty $identity "${kind}_count")
            if ($null -ne $value) { $projection["${kind}_count"] += $value }
        }
        foreach ($spec in @("unavailable_reason_counts", "projection_sha256_counts", "revision_counts")) {
            $counts = Get-PerformanceProperty $identity $spec ([pscustomobject]@{})
            foreach ($property in @($counts.PSObject.Properties)) {
                Add-PerformanceProjectionCount $projection[$spec] ([string]$property.Name) $property.Value
            }
        }
    }
    $availability = if ($measured -gt 0 -and $unavailable -eq 0 -and $unknownSides -eq 0) {
        "measured"
    } elseif ($measured -gt 0) {
        "partial"
    } else {
        "unavailable"
    }
    $sections = @()
    if ($measured -gt 0) {
        $sections = @(Get-PerformanceProviderSectionKinds | Where-Object { $sectionTotals.ContainsKey($_) } | ForEach-Object {
            $requestBytes = @($sectionTotals[$_].request_bytes.ToArray())
            $requestTokens = @($sectionTotals[$_].request_estimated_tokens.ToArray())
            [pscustomobject]@{
                kind = $_
                count = $sectionTotals[$_].count
                bytes = $sectionTotals[$_].bytes
                estimated_tokens = $sectionTotals[$_].estimated_tokens
                request_sample_count = $requestBytes.Count
                bytes_per_request_mean = [Math]::Round($sectionTotals[$_].bytes / [double]$measured, 6)
                bytes_per_request_median = Get-PerformanceSectionMedian $requestBytes
                estimated_tokens_per_request_mean = [Math]::Round($sectionTotals[$_].estimated_tokens / [double]$measured, 6)
                estimated_tokens_per_request_median = Get-PerformanceSectionMedian $requestTokens
                request_bytes = $requestBytes
                request_estimated_tokens = $requestTokens
            }
        })
    }
    $orderedReasons = [ordered]@{}
    foreach ($reason in @($reasonCounts.Keys | Sort-Object)) { $orderedReasons[$reason] = $reasonCounts[$reason] }
    [pscustomobject]@{
        availability = $availability
        side_count = @($Rows).Count
        unknown_side_count = $unknownSides
        request_count = if ($knownRequestSides -gt 0) { $knownRequestCount } else { $null }
        measured_request_count = if ($knownRequestSides -gt 0) { $measured } else { $null }
        unavailable_request_count = if ($knownRequestSides -gt 0) { $unavailable } else { $null }
        unavailable_reason_counts = [pscustomobject]$orderedReasons
        section_bytes_total = if ($measured -gt 0) { $sectionTotal } else { $null }
        estimated_tokens_total = if ($measured -gt 0) { $tokenTotal } else { $null }
        sections = $sections
        active_projection_identity_summary = ConvertFrom-PerformanceProjectionAccumulator $projection
    }
}

function Format-PerformanceSectionReasons {
    param($ReasonCounts)
    if ($null -eq $ReasonCounts) { return "N/A" }
    $parts = @($ReasonCounts.PSObject.Properties | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" })
    if ($parts.Count -eq 0) { return "none" }
    ($parts -join ", ").Replace("|", "\|")
}

function Format-PerformanceControlActions {
    param($Actions)
    if ($null -eq $Actions) { return "N/A" }
    $parts = @($Actions.PSObject.Properties | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" })
    if ($parts.Count -eq 0) { return "N/A" }
    ($parts -join ", ").Replace("|", "\|")
}

function Add-PerformanceSectionCostMarkdown {
    param([System.Collections.Generic.List[string]]$Lines, [object[]]$Rows, [object[]]$Aggregates)
    $Lines.Add("")
    $Lines.Add("## Provider wire section cost")
    $Lines.Add("")
    $Lines.Add("| Scope | Repeat | Mode | Availability | Measured requests | Unavailable requests | Section bytes | Estimated tokens | Unavailable reasons |")
    $Lines.Add("|---|---:|---|---|---:|---:|---:|---:|---|")
    foreach ($row in $Rows) {
        $facts = $row.section_cost
        $Lines.Add("| side | $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $facts.availability) | $(Format-PerformanceValue $facts.measured_request_count) | $(Format-PerformanceValue $facts.unavailable_request_count) | $(Format-PerformanceValue $facts.section_bytes_total) | $(Format-PerformanceValue $facts.estimated_tokens_total) | $(Format-PerformanceSectionReasons $facts.unavailable_reason_counts) |")
    }
    foreach ($aggregate in $Aggregates) {
        $facts = $aggregate.section_cost
        $Lines.Add("| mode | N/A | $($aggregate.logical_mode) | $(Format-PerformanceValue $facts.availability) | $(Format-PerformanceValue $facts.measured_request_count) | $(Format-PerformanceValue $facts.unavailable_request_count) | $(Format-PerformanceValue $facts.section_bytes_total) | $(Format-PerformanceValue $facts.estimated_tokens_total) | $(Format-PerformanceSectionReasons $facts.unavailable_reason_counts) |")
    }
    $Lines.Add("")
    $Lines.Add("| Scope | Repeat | Mode | Kind | Count | Bytes | Bytes/request mean | Bytes/request median | Estimated tokens | Tokens/request mean | Tokens/request median |")
    $Lines.Add("|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        $sections = @($row.section_cost.sections)
        if ($sections.Count -eq 0) {
            $Lines.Add("| side | $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | all sections | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
            continue
        }
        foreach ($section in $sections) {
            $Lines.Add("| side | $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $($section.kind) | $(Format-PerformanceValue $section.count) | $(Format-PerformanceValue $section.bytes) | $(Format-PerformanceValue $section.bytes_per_request_mean) | $(Format-PerformanceValue $section.bytes_per_request_median) | $(Format-PerformanceValue $section.estimated_tokens) | $(Format-PerformanceValue $section.estimated_tokens_per_request_mean) | $(Format-PerformanceValue $section.estimated_tokens_per_request_median) |")
        }
    }
    foreach ($aggregate in $Aggregates) {
        $sections = @($aggregate.section_cost.sections)
        if ($sections.Count -eq 0) {
            $Lines.Add("| mode | N/A | $($aggregate.logical_mode) | all sections | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
            continue
        }
        foreach ($section in $sections) {
            $Lines.Add("| mode | N/A | $($aggregate.logical_mode) | $($section.kind) | $(Format-PerformanceValue $section.count) | $(Format-PerformanceValue $section.bytes) | $(Format-PerformanceValue $section.bytes_per_request_mean) | $(Format-PerformanceValue $section.bytes_per_request_median) | $(Format-PerformanceValue $section.estimated_tokens) | $(Format-PerformanceValue $section.estimated_tokens_per_request_mean) | $(Format-PerformanceValue $section.estimated_tokens_per_request_median) |")
        }
    }
    $Lines.Add("")
    $Lines.Add("### Active projection identity")
    $Lines.Add("")
    $Lines.Add("| Scope | Repeat | Mode | Bootstrap | Active | Unavailable | Unique revisions | Revisions | Unique projection hashes | Unavailable reasons |")
    $Lines.Add("|---|---:|---|---:|---:|---:|---:|---|---:|---|")
    foreach ($row in $Rows) {
        $identity = $row.section_cost.active_projection_identity_summary
        $Lines.Add("| side | $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $identity.bootstrap_count) | $(Format-PerformanceValue $identity.active_count) | $(Format-PerformanceValue $identity.unavailable_count) | $(Format-PerformanceValue $identity.unique_revision_count) | $(Format-PerformanceSectionReasons $identity.revision_counts) | $(Format-PerformanceValue $identity.unique_projection_sha256_count) | $(Format-PerformanceSectionReasons $identity.unavailable_reason_counts) |")
    }
    foreach ($aggregate in $Aggregates) {
        $identity = $aggregate.section_cost.active_projection_identity_summary
        $Lines.Add("| mode | N/A | $($aggregate.logical_mode) | $(Format-PerformanceValue $identity.bootstrap_count) | $(Format-PerformanceValue $identity.active_count) | $(Format-PerformanceValue $identity.unavailable_count) | $(Format-PerformanceValue $identity.unique_revision_count) | $(Format-PerformanceSectionReasons $identity.revision_counts) | $(Format-PerformanceValue $identity.unique_projection_sha256_count) | $(Format-PerformanceSectionReasons $identity.unavailable_reason_counts) |")
    }
}
