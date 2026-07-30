function Test-R7ReservedFailureFamilyText {
    param([string]$Text)
    $normalized = [regex]::Replace(
        $Text,
        '\\u([0-9a-fA-F]{4})',
        { param($match) [char][Convert]::ToInt32($match.Groups[1].Value, 16) }
    )
    $normalized -match (
        'TaskSpaceResponseCommitFailure|ToolSequencePreflightResult|' +
        'ProviderToolResponsePreflight|ToolSearchFailure|' +
        'TaskSpaceToolSkipped|TaskSpaceBoundResultCommitFailure'
    )
}

function Assert-R7UniqueJsonProperties {
    param(
        [System.Text.Json.JsonElement]$Element,
        [string]$Path = "$"
    )
    if ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Object) {
        $names = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::OrdinalIgnoreCase
        )
        foreach ($property in $Element.EnumerateObject()) {
            if (-not $names.Add([string]$property.Name)) {
                throw "Duplicate JSON property: $Path.$($property.Name)"
            }
            Assert-R7UniqueJsonProperties $property.Value "$Path.$($property.Name)"
        }
    } elseif ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Array) {
        $index = 0
        foreach ($item in $Element.EnumerateArray()) {
            Assert-R7UniqueJsonProperties $item "$Path[$index]"
            $index++
        }
    }
}

function Apply-R7SupplementalFailure {
    param(
        [hashtable]$CallsById,
        [Collections.Generic.List[object]]$Requests,
        [string]$Text,
        [string]$OriginalRole
    )
    $trimmed = $Text.Trim()
    $reservedFamily =
        '(TaskSpaceResponseCommitFailure|ToolSequencePreflightResult|' +
        'ProviderToolResponsePreflight|ToolSearchFailure|' +
        'TaskSpaceToolSkipped|TaskSpaceBoundResultCommitFailure)'
    $containsReservedFamily = Test-R7ReservedFailureFamilyText $trimmed
    if (-not (
            $trimmed.StartsWith("{") -or
            $trimmed.StartsWith("[") -or
            $trimmed.StartsWith('"')
        )) {
        return
    }
    $document = $null
    try {
        $document = [System.Text.Json.JsonDocument]::Parse($trimmed)
    } catch {
        if ($containsReservedFamily) {
            throw "Malformed structured failure message"
        }
        return
    }
    try {
        Assert-R7UniqueJsonProperties $document.RootElement
        if ($document.RootElement.ValueKind -ne
            [System.Text.Json.JsonValueKind]::Object) {
            if ($containsReservedFamily) {
                throw "Structured failure root must be an object"
            }
            return
        }
    } finally {
        $document.Dispose()
    }
    $payload = $trimmed | ConvertFrom-Json -Depth 100 -NoEnumerate
    if ($payload -is [System.Array] -or $payload -isnot [pscustomobject]) {
        if ($containsReservedFamily) {
            throw "Structured failure root must be an object"
        }
        return
    }
    $schemaValue = Get-R7JsonProperty $payload "schema_version"
    if ($schemaValue -isnot [string]) {
        if ($containsReservedFamily) {
            throw "Incomplete structured failure fact: schema_version must be a non-empty string"
        }
        return
    }
    $schemaVersion = [string]$schemaValue
    $knownSchemas = @(
        "TaskSpaceResponseCommitFailureV3",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV3",
        "TaskSpaceToolSkippedV2",
        "TaskSpaceBoundResultCommitFailureV2"
    )
    if ($schemaVersion -notin $knownSchemas) {
        if ($containsReservedFamily -or $schemaVersion -match "^$reservedFamily") {
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
        if ([int]$CallsById[$affectedCallId].supplemental_count -ne 0) {
            throw "Duplicate structured failure fact for call: $affectedCallId"
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
            $CallsById `
            $Requests `
            $schemaVersion `
            $payload `
            $provenance `
            $affectedCallIds
    } else {
        Assert-R7ProviderResponseFailureProvenance `
            $CallsById `
            $Requests `
            $schemaVersion `
            $payload `
            $provenance `
            $scope `
            $affectedCallIds `
            $trimmed
    }
    foreach ($callId in $affectedCallIds) {
        $call = $CallsById[$callId]
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
    $call = $CallsById[$callId]
    if ([int]$call.output_count -ne 1 -or -not [bool]$call.success) {
        throw "ToolSearch failure precedes its pairing output: $callId"
    }
    $error = Get-R7JsonProperty $Payload "error"
    $hasZeroDispatch = $Provenance.PSObject.Properties.Name -contains "zero_dispatch"
    if ([string](Get-R7JsonProperty $Provenance "scope" "") -ne "tool_execution" -or
        -not $hasZeroDispatch -or
        [bool](Get-R7JsonProperty $Provenance "zero_dispatch" $true) -or
        [string](Get-R7JsonProperty $Provenance "copy_group_id" "") -ne
            "tool_execution:$callId" -or
        [string](Get-R7JsonProperty $Provenance "cause_call_id" "") -ne $callId -or
        [string](Get-R7JsonProperty $Payload "tool" "") -ne
            [string]$CallsById[$callId].tool -or
        [string](Get-R7JsonProperty $error "code" "") -ne "tool_search_failed") {
        throw "ToolSearch failure provenance is not a real execution: $callId"
    }
}

function Assert-R7PerCallFailureProvenance {
    param(
        $CallsById,
        $Requests,
        [string]$SchemaVersion,
        $Payload,
        $Provenance,
        [string[]]$AffectedCallIds
    )
    $callId = [string](Get-R7JsonProperty $Payload "call_id" "")
    if ($AffectedCallIds.Count -ne 1 -or $AffectedCallIds[0] -ne $callId) {
        throw "Per-call TaskSpace failure provenance does not match call_id: $callId"
    }
    $call = $CallsById[$callId]
    $expectedScope = if ($SchemaVersion -eq "TaskSpaceToolSkippedV2") {
        "tool_sequence_skip"
    } else {
        "tool_result_attribution"
    }
    $zeroDispatch = [bool](Get-R7JsonProperty $Provenance "zero_dispatch" $false)
    $copyGroupId = [string](Get-R7JsonProperty $Provenance "copy_group_id" "")
    $error = Get-R7JsonProperty $Payload "error"
    if ([string](Get-R7JsonProperty $Provenance "scope" "") -ne $expectedScope -or
        $null -eq $call) {
        throw "Per-call TaskSpace failure provenance has the wrong scope: $callId"
    }
    if ($SchemaVersion -eq "TaskSpaceBoundResultCommitFailureV2") {
        $expectedReservationId = [string]$call.expected_reservation_id
        if ([int]$call.output_count -ne 1 -or
            $zeroDispatch -or
            $copyGroupId -ne "tool_result_attribution:$callId" -or
            [string](Get-R7JsonProperty $error "code" "") -ne
                "taskspace_bound_result_commit_failed" -or
            [string]::IsNullOrWhiteSpace($expectedReservationId) -or
            [string](Get-R7JsonProperty $Payload "reservation_id" "") -ne
                $expectedReservationId) {
            throw "Bound-result failure provenance does not match its reservation: $callId"
        }
        return
    }
    $status = [string](Get-R7JsonProperty $Payload "status" "")
    $cause = Get-R7JsonProperty $Payload "cause"
    $causeCallId = [string](Get-R7JsonProperty $Provenance "cause_call_id" "")
    $expectedCauseField = if ($status -eq "skipped_due_to_prior_failure") {
        "prior_call_id"
    } else {
        "terminal_call_id"
    }
    $owningRequest = @(
        $Requests |
            Where-Object { $callId -in @($_.calls | ForEach-Object call_id) }
    )
    $requestCallIds = if ($owningRequest.Count -eq 1) {
        @($owningRequest[0].calls | ForEach-Object { [string]$_.call_id })
    } else {
        @()
    }
    $callIndex = [array]::IndexOf($requestCallIds, $callId)
    $causeIndex = [array]::IndexOf($requestCallIds, $causeCallId)
    $causeCall = if ($CallsById.ContainsKey($causeCallId)) {
        $CallsById[$causeCallId]
    } else {
        $null
    }
    $causeOutcomeMatches = if ($status -eq "skipped_due_to_prior_failure") {
        $null -ne $causeCall -and
            [int]$causeCall.output_count -eq 1 -and
            $causeCall.success -eq $false
    } else {
        $null -ne $causeCall -and
            [int]$causeCall.output_count -eq 1 -and
            [bool]$causeCall.success -and
            [string]$causeCall.tool -eq "taskspace_control" -and
            [string]$causeCall.control_action -eq "finish_map"
    }
    if (-not $zeroDispatch -or
        [int]$call.output_count -ne 1 -or
        $copyGroupId -ne "tool_sequence_skip:$causeCallId" -or
        [string](Get-R7JsonProperty $Payload "tool" "") -ne [string]$call.tool -or
        [string](Get-R7JsonProperty $error "code" "") -ne $status -or
        [string](Get-R7JsonProperty $cause "field" "") -ne $expectedCauseField -or
        [string](Get-R7JsonProperty $cause "call_id" "") -ne $causeCallId -or
        $callIndex -lt 1 -or $causeIndex -lt 0 -or $causeIndex -ge $callIndex -or
        -not $causeOutcomeMatches) {
        throw "Skipped-call failure provenance does not match its causal call: $callId"
    }
}

function Assert-R7ProviderResponseFailureProvenance {
    param(
        $CallsById,
        $Requests,
        [string]$SchemaVersion,
        $Payload,
        $Provenance,
        [string]$Scope,
        [string[]]$AffectedCallIds,
        [string]$RawText
    )
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
    $expectedCopyGroupId = "provider_response:$($requestCallIds[0])"
    if ($Scope -ne "provider_response" -or
        -not [bool](Get-R7JsonProperty $Provenance "zero_dispatch" $false) -or
        [string](Get-R7JsonProperty $Provenance "copy_group_id" "") -ne
            $expectedCopyGroupId -or
        (Compare-Object @($requestCallIds | Sort-Object) @($AffectedCallIds | Sort-Object))) {
        throw "Provider-response failure provenance does not match the request call set"
    }
    foreach ($callId in $AffectedCallIds) {
        $call = $CallsById[$callId]
        if ([int]$call.output_count -ne 1 -or
            $call.observed_output_tool_success -ne $false -or
            -not [string]::Equals(
                [string]$call.observed_output_text,
                $RawText,
                [StringComparison]::Ordinal
            )) {
            throw "Provider-response failure does not match its Tool output: $callId"
        }
    }
    $status = [string](Get-R7JsonProperty $Payload "status" "")
    $error = Get-R7JsonProperty $Payload "error"
    $errorClass = [string](Get-R7JsonProperty $error "class" "")
    $errorCode = [string](Get-R7JsonProperty $error "code" "")
    $semanticMatch = switch ($SchemaVersion) {
        "ProviderToolResponsePreflightV2" {
            $errorCode -eq "provider_tool_declaration_invalid"
        }
        "ToolSequencePreflightResultV3" {
            $errorCode -in @(
                "request_multiple_apply_patch_calls_not_allowed",
                "taskspace_control_required",
                "taskspace_control_multiple",
                "taskspace_control_must_be_first",
                "taskspace_control_arguments_invalid",
                "taskspace_action_count_mismatch",
                "taskspace_action_tool_mismatch",
                "taskspace_duplicate_call_id",
                "taskspace_empty_call_id",
                "taskspace_control_only_action_has_siblings"
            )
        }
        "TaskSpaceResponseCommitFailureV3" {
            ($status -eq "state_rejected" -and
                $errorClass -eq "state_machine" -and
                $errorCode -eq "taskspace_response_state_commit_failed") -or
            ($status -eq "protocol_rejected" -and $errorClass -eq "protocol") -or
            ($status -eq "resource_failed" -and $errorClass -eq "resource")
        }
        default { $false }
    }
    if (-not $semanticMatch) {
        throw "Provider-response failure payload does not match its production schema"
    }
}
