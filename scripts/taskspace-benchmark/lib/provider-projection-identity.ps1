function New-TaskspaceUnavailableProjectionIdentity {
    param([Parameter(Mandatory = $true)][string]$Reason)
    [pscustomobject]@{
        count = $null; kind = "unavailable"; map_id_sha256 = $null; revision = $null
        canonical_sha256 = $null; projection_sha256 = $null; unavailable_reason = $Reason
    }
}

function ConvertTo-TaskspaceProjectionIdentity {
    param($Raw)
    if ($null -eq $Raw) { return New-TaskspaceUnavailableProjectionIdentity "active_projection_identity_missing" }
    $count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Raw @("count"))
    $kind = [string](Get-TaskspaceCostProperty $Raw @("kind"))
    $mapHash = [string](Get-TaskspaceCostProperty $Raw @("map_id_sha256"))
    $canonicalHash = [string](Get-TaskspaceCostProperty $Raw @("canonical_sha256"))
    $projectionHash = [string](Get-TaskspaceCostProperty $Raw @("projection_sha256"))
    $revisionRaw = Get-TaskspaceCostProperty $Raw @("revision")
    $reason = [string](Get-TaskspaceCostProperty $Raw @("unavailable_reason"))
    if ($null -eq $count -or $kind -notin @("bootstrap", "bootstrap_required", "active", "current_projection", "revision_snapshot", "unavailable")) {
        return New-TaskspaceUnavailableProjectionIdentity "active_projection_identity_shape_invalid"
    }
    $normalizedKind = if ($kind -eq "bootstrap_required") { "bootstrap" } elseif ($kind -in @("current_projection", "revision_snapshot")) { "active" } else { $kind }
    $hashPattern = '^[0-9a-f]{64}$'
    if ($normalizedKind -eq "bootstrap" -and ($count -ne 1 -or $projectionHash -notmatch $hashPattern -or
        -not [string]::IsNullOrWhiteSpace($mapHash) -or -not [string]::IsNullOrWhiteSpace($canonicalHash) -or
        $null -ne $revisionRaw -or -not [string]::IsNullOrWhiteSpace($reason))) {
        return New-TaskspaceUnavailableProjectionIdentity "bootstrap_projection_identity_invalid"
    }
    if ($normalizedKind -eq "active") {
        $revision = ConvertTo-TaskspaceProviderSectionInt64 $revisionRaw
        if ($count -lt 1 -or $mapHash -notmatch $hashPattern -or $projectionHash -notmatch $hashPattern -or
            $null -eq $revision -or -not [string]::IsNullOrWhiteSpace($reason)) {
            return New-TaskspaceUnavailableProjectionIdentity "active_projection_identity_invalid"
        }
        if ($kind -in @("current_projection", "revision_snapshot") -and $canonicalHash -notmatch $hashPattern) {
            return New-TaskspaceUnavailableProjectionIdentity "current_projection_canonical_identity_invalid"
        }
    } elseif ($normalizedKind -eq "unavailable" -and (-not [string]::IsNullOrWhiteSpace($mapHash) -or
        -not [string]::IsNullOrWhiteSpace($canonicalHash) -or $null -ne $revisionRaw -or
        [string]::IsNullOrWhiteSpace($reason) -or (-not [string]::IsNullOrWhiteSpace($projectionHash) -and $projectionHash -notmatch $hashPattern))) {
        return New-TaskspaceUnavailableProjectionIdentity "unavailable_projection_identity_invalid"
    }
    [pscustomobject]@{
        count = [int64]$count; kind = $normalizedKind
        map_id_sha256 = if ([string]::IsNullOrWhiteSpace($mapHash)) { $null } else { $mapHash }
        revision = if ($normalizedKind -eq "active") { [int64]$revision } else { $null }
        canonical_sha256 = if ([string]::IsNullOrWhiteSpace($canonicalHash)) { $null } else { $canonicalHash }
        projection_sha256 = if ([string]::IsNullOrWhiteSpace($projectionHash)) { $null } else { $projectionHash }
        unavailable_reason = if ([string]::IsNullOrWhiteSpace($reason)) { $null } else { $reason }
    }
}

function Add-TaskspaceProviderProjectionCount {
    param([hashtable]$Table, [string]$Key, [int]$Count)
    if ($Count -le 0 -or [string]::IsNullOrWhiteSpace($Key)) { return }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = 0 }
    $Table[$Key] = [int]$Table[$Key] + $Count
}

function Add-TaskspaceProviderProjectionIdentity {
    param($Accumulator, $Identity, [int]$Weight = 1)
    $kind = [string](Get-TaskspaceCostProperty $Identity @("kind"))
    if ($kind -notin @("bootstrap", "active", "unavailable")) { $kind = "unavailable" }
    $field = "${kind}_count"
    $Accumulator.projection_identity[$field] = [int]$Accumulator.projection_identity[$field] + $Weight
    if ($kind -eq "unavailable") {
        Add-TaskspaceProviderProjectionCount $Accumulator.projection_identity.unavailable_reason_counts ([string](Get-TaskspaceCostProperty $Identity @("unavailable_reason"))) $Weight
    }
    Add-TaskspaceProviderProjectionCount $Accumulator.projection_identity.projection_sha256_counts ([string](Get-TaskspaceCostProperty $Identity @("projection_sha256"))) $Weight
    $revision = Get-TaskspaceCostProperty $Identity @("revision")
    if ($null -ne $revision) { Add-TaskspaceProviderProjectionCount $Accumulator.projection_identity.revision_counts ([string]$revision) $Weight }
}

function Add-TaskspaceProviderProjectionIdentitySummary {
    param($Accumulator, $Summary, [int]$FallbackRequestCount)
    if ($null -eq $Summary) {
        Add-TaskspaceProviderProjectionIdentity $Accumulator (New-TaskspaceUnavailableProjectionIdentity "active_projection_identity_summary_missing") $FallbackRequestCount
        return
    }
    $accounted = 0
    foreach ($kind in @("bootstrap", "active", "unavailable")) {
        $count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Summary @("${kind}_count"))
        if ($null -eq $count) { $count = 0 }
        $Accumulator.projection_identity["${kind}_count"] += [int]$count; $accounted += [int]$count
    }
    foreach ($spec in @(@("unavailable_reason_counts", "unavailable_reason_counts"), @("projection_sha256_counts", "projection_sha256_counts"), @("revision_counts", "revision_counts"))) {
        $values = Get-TaskspaceCostProperty $Summary @($spec[0])
        foreach ($property in $(if ($null -ne $values) { @($values.PSObject.Properties) } else { @() })) {
            Add-TaskspaceProviderProjectionCount $Accumulator.projection_identity[$spec[1]] ([string]$property.Name) ([int]$property.Value)
        }
    }
    if ($accounted -lt $FallbackRequestCount) {
        Add-TaskspaceProviderProjectionIdentity $Accumulator (New-TaskspaceUnavailableProjectionIdentity "active_projection_identity_request_unaccounted") ($FallbackRequestCount - $accounted)
    }
}
