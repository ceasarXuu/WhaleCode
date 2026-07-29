function ConvertTo-WhaleCodeBaseInstructionsIdentity {
    param($Identity, [AllowEmptyString()][string]$TraceSchema = "")

    if ($null -eq $Identity) {
        $reason = if ($TraceSchema -eq "provider-chat-wire-trace-v9") {
            "base_instructions_identity_missing"
        } else {
            "trace_without_base_instructions_identity"
        }
        return [pscustomobject]@{
            schema_version = "WhaleCodeBaseInstructionsIdentityV1"
            availability = "unavailable"
            count = $null
            message_index = $null
            wire_role = $null
            message_bytes = $null
            estimated_tokens = $null
            profile = $null
            version = $null
            sha256 = $null
            matches_current_contract = $false
            unavailable_reason = $reason
        }
    }

    [pscustomobject]@{
        schema_version = "WhaleCodeBaseInstructionsIdentityV1"
        availability = "measured"
        count = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Identity @("count"))
        message_index = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Identity @("message_index"))
        wire_role = [string](Get-TaskspaceCostProperty $Identity @("wire_role"))
        message_bytes = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Identity @("message_bytes"))
        estimated_tokens = ConvertTo-TaskspaceProviderSectionInt64 (Get-TaskspaceCostProperty $Identity @("estimated_tokens"))
        profile = [string](Get-TaskspaceCostProperty $Identity @("profile"))
        version = [string](Get-TaskspaceCostProperty $Identity @("version"))
        sha256 = [string](Get-TaskspaceCostProperty $Identity @("sha256"))
        matches_current_contract = [bool](Get-TaskspaceCostProperty $Identity @("matches_current_contract"))
        unavailable_reason = [string](Get-TaskspaceCostProperty $Identity @("unavailable_reason"))
    }
}

function Add-WhaleCodeBaseInstructionsCount {
    param([hashtable]$Table, [AllowEmptyString()][string]$Key)
    if ([string]::IsNullOrWhiteSpace($Key)) { return }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = 0 }
    $Table[$Key] = [int]$Table[$Key] + 1
}

function New-WhaleCodeBaseInstructionsIdentitySummary {
    param([object[]]$Identities)

    $present = 0
    $absent = 0
    $invalid = 0
    $matching = 0
    $unavailable = 0
    $messageBytes = [int64]0
    $estimatedTokens = [int64]0
    $profiles = @{}
    $versions = @{}
    $hashes = @{}
    $messageIndexes = @{}
    $wireRoles = @{}
    $reasons = @{}

    foreach ($identity in @($Identities)) {
        if ([string]$identity.availability -ne "measured") {
            $unavailable++
            Add-WhaleCodeBaseInstructionsCount $reasons ([string]$identity.unavailable_reason)
            continue
        }
        $count = ConvertTo-TaskspaceProviderSectionInt64 $identity.count
        if ($count -eq 0) {
            $absent++
            Add-WhaleCodeBaseInstructionsCount $reasons ([string]$identity.unavailable_reason)
            continue
        }
        if ($count -ne 1) {
            $invalid++
            Add-WhaleCodeBaseInstructionsCount $reasons ([string]$identity.unavailable_reason)
            continue
        }
        $present++
        if ([bool]$identity.matches_current_contract) { $matching++ } else { $invalid++ }
        Add-WhaleCodeBaseInstructionsCount $profiles ([string]$identity.profile)
        Add-WhaleCodeBaseInstructionsCount $versions ([string]$identity.version)
        Add-WhaleCodeBaseInstructionsCount $hashes ([string]$identity.sha256)
        Add-WhaleCodeBaseInstructionsCount $messageIndexes ([string]$identity.message_index)
        Add-WhaleCodeBaseInstructionsCount $wireRoles ([string]$identity.wire_role)
        $bytes = ConvertTo-TaskspaceProviderSectionInt64 $identity.message_bytes
        $tokens = ConvertTo-TaskspaceProviderSectionInt64 $identity.estimated_tokens
        if ($null -ne $bytes) { $messageBytes += $bytes }
        if ($null -ne $tokens) { $estimatedTokens += $tokens }
        Add-WhaleCodeBaseInstructionsCount $reasons ([string]$identity.unavailable_reason)
    }

    $requestCount = @($Identities).Count
    [pscustomobject]@{
        schema_version = "WhaleCodeBaseInstructionsIdentitySummaryV1"
        request_count = [int]$requestCount
        present_count = [int]$present
        absent_count = [int]$absent
        invalid_count = [int]$invalid
        unavailable_count = [int]$unavailable
        current_contract_match_count = [int]$matching
        current_contract_match_rate = if ($present -gt 0) { [Math]::Round([double]$matching / [double]$present, 6) } else { $null }
        message_bytes_total = [int64]$messageBytes
        message_bytes_per_present_request_mean = if ($present -gt 0) { [Math]::Round([double]$messageBytes / [double]$present, 6) } else { $null }
        estimated_tokens_total = [int64]$estimatedTokens
        estimated_tokens_per_present_request_mean = if ($present -gt 0) { [Math]::Round([double]$estimatedTokens / [double]$present, 6) } else { $null }
        profile_counts = Convert-TaskspaceCostTable $profiles
        version_counts = Convert-TaskspaceCostTable $versions
        sha256_counts = Convert-TaskspaceCostTable $hashes
        message_index_counts = Convert-TaskspaceCostTable $messageIndexes
        wire_role_counts = Convert-TaskspaceCostTable $wireRoles
        unavailable_reason_counts = Convert-TaskspaceCostTable $reasons
    }
}
