function Apply-R7SupplementalFailure {
    param(
        [hashtable]$CallsById,
        [Collections.Generic.List[object]]$Requests,
        [string]$Text,
        [string]$OriginalRole
    )
    $trimmed = $Text.Trim()
    if (-not $trimmed.StartsWith("{")) { return }
    try {
        $payload = $trimmed | ConvertFrom-Json -Depth 100
    } catch {
        if ($OriginalRole -eq "developer" -and
            $trimmed -match '^\{\s*"schema_version"\s*:') {
            throw "Malformed structured developer message"
        }
        return
    }
    $schemaVersion = [string](Get-R7JsonProperty $payload "schema_version" "")
    $knownSchemas = @(
        "TaskSpaceResponseCommitFailureV3",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV3",
        "TaskSpaceToolSkippedV2",
        "TaskSpaceBoundResultCommitFailureV2"
    )
    if ($schemaVersion -notin $knownSchemas) {
        $reservedFamily =
            '^(TaskSpaceResponseCommitFailure|ToolSequencePreflightResult|' +
            'ProviderToolResponsePreflight|ToolSearchFailure|' +
            'TaskSpaceToolSkipped|TaskSpaceBoundResultCommitFailure)'
        if ($schemaVersion -match $reservedFamily) {
            throw "Unknown structured failure schema: $schemaVersion"
        }
        return
    }
    if ($OriginalRole -ne "developer") {
        throw "Structured TaskSpace failure used an untrusted message role: $OriginalRole"
    }
    $shapeError = Get-R7SupplementalFailureShapeError $payload
    if (-not [string]::IsNullOrWhiteSpace($shapeError)) {
        throw "Incomplete structured failure fact: $shapeError"
    }
    $provenance = Get-R7JsonProperty $payload "failure_provenance"
    $scope = [string](Get-R7JsonProperty $provenance "scope" "")
    $affectedCallIds = @(
        Get-R7JsonProperty $provenance "affected_call_ids" @() |
            ForEach-Object { [string]$_ }
    )
    if (-not $affectedCallIds.Count -or
        @($affectedCallIds | Sort-Object -Unique).Count -ne $affectedCallIds.Count) {
        throw "Structured failure fact has missing or duplicate affected_call_ids"
    }
    foreach ($affectedCallId in $affectedCallIds) {
        if (-not $CallsById.ContainsKey($affectedCallId)) {
            throw "Structured failure fact has no matching call: $affectedCallId"
        }
    }
    if ($schemaVersion -eq "ToolSearchFailureV3") {
        Assert-R7ToolSearchFailureProvenance `
            $CallsById `
            $payload `
            $provenance `
            $affectedCallIds
    } elseif ($schemaVersion -eq "TaskSpaceToolSkippedV2" -or
        $schemaVersion -eq "TaskSpaceBoundResultCommitFailureV2") {
        Assert-R7PerCallFailureProvenance `
            $schemaVersion `
            $payload `
            $provenance `
            $affectedCallIds
    } else {
        Assert-R7ProviderResponseFailureProvenance `
            $Requests `
            $provenance `
            $scope `
            $affectedCallIds
    }
    foreach ($callId in $affectedCallIds) {
        $call = $CallsById[$callId]
        if ([int]$call.supplemental_count -ne 0) {
            throw "Duplicate structured failure fact for call: $callId"
        }
        $call.supplemental_count = 1
        Set-R7CallOutcome $call (
            Get-R7CallOutcome `
                -ToolSuccess $false `
                -Output $trimmed `
                -ToolName ([string]$call.tool) `
                -TrustedRuntimeCarrier
        )
    }
}

function Assert-R7ToolSearchFailureProvenance {
    param($CallsById, $Payload, $Provenance, [string[]]$AffectedCallIds)
    $callId = [string](Get-R7JsonProperty $Payload "call_id" "")
    if ($AffectedCallIds.Count -ne 1 -or $AffectedCallIds[0] -ne $callId) {
        throw "ToolSearch failure provenance does not match call_id: $callId"
    }
    if (-not $CallsById.ContainsKey($callId) -or
        [string]$CallsById[$callId].call_type -ne "tool_search_call") {
        throw "ToolSearch failure fact has no matching ToolSearch call: $callId"
    }
    $hasZeroDispatch = $Provenance.PSObject.Properties.Name -contains "zero_dispatch"
    if ([string](Get-R7JsonProperty $Provenance "scope" "") -ne "tool_execution" -or
        -not $hasZeroDispatch -or
        [bool](Get-R7JsonProperty $Provenance "zero_dispatch" $true) -or
        [string](Get-R7JsonProperty $Provenance "copy_group_id" "") -ne
            "tool_execution:$callId" -or
        [string](Get-R7JsonProperty $Provenance "cause_call_id" "") -ne $callId) {
        throw "ToolSearch failure provenance is not a real execution: $callId"
    }
}

function Assert-R7PerCallFailureProvenance {
    param(
        [string]$SchemaVersion,
        $Payload,
        $Provenance,
        [string[]]$AffectedCallIds
    )
    $callId = [string](Get-R7JsonProperty $Payload "call_id" "")
    if ($AffectedCallIds.Count -ne 1 -or $AffectedCallIds[0] -ne $callId) {
        throw "Per-call TaskSpace failure provenance does not match call_id: $callId"
    }
    $expectedScope = if ($SchemaVersion -eq "TaskSpaceToolSkippedV2") {
        "tool_sequence_skip"
    } else {
        "tool_result_attribution"
    }
    if ([string](Get-R7JsonProperty $Provenance "scope" "") -ne $expectedScope) {
        throw "Per-call TaskSpace failure provenance has the wrong scope: $callId"
    }
}

function Assert-R7ProviderResponseFailureProvenance {
    param($Requests, $Provenance, [string]$Scope, [string[]]$AffectedCallIds)
    $owningRequests = @(
        $Requests |
            Where-Object {
                $requestCallIds = @($_.calls | ForEach-Object { [string]$_.call_id })
                @($AffectedCallIds | Where-Object { $_ -in $requestCallIds }).Count
            }
    )
    if ($owningRequests.Count -ne 1) {
        throw "Provider-response failure provenance spans multiple or no requests"
    }
    $requestCallIds = @(
        $owningRequests[0].calls | ForEach-Object { [string]$_.call_id }
    )
    if ($Scope -ne "provider_response" -or
        -not [bool](Get-R7JsonProperty $Provenance "zero_dispatch" $false) -or
        [string]::IsNullOrWhiteSpace(
            [string](Get-R7JsonProperty $Provenance "copy_group_id" "")
        ) -or
        (Compare-Object @($requestCallIds | Sort-Object) @($AffectedCallIds | Sort-Object))) {
        throw "Provider-response failure provenance does not match the request call set"
    }
}
