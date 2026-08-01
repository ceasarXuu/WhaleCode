function Get-R7StructuredFailureSchemas {
    @(
        "TaskSpaceControlResultV2",
        "TaskSpaceResponseCommitFailureV3",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV3",
        "TaskSpaceToolSkippedV2",
        "TaskSpaceBoundResultCommitFailureV2"
    )
}

function Get-R7ReservedTaskspaceCarrierSchemas {
    @(
        Get-R7StructuredFailureSchemas
        "TaskSpaceResponseCommitV1"
    )
}

function New-R7InvalidCallOutcome {
    param(
        [string]$ReasonCode,
        [string]$ParseStatus,
        [string]$CarrierSchema = "",
        $StateCommit = $null
    )
    [pscustomobject]@{
        success = $false
        failure_class = "evidence_unclassified"
        failure_code = $ReasonCode
        failure_schema_version = $CarrierSchema
        failure_provenance_scope = ""
        failure_copy_group_id = ""
        failure_affected_call_ids = @()
        zero_dispatch = $false
        parse_status = $ParseStatus
        evidence_valid = $false
        violation_codes = @()
        violation_contexts = @()
        state_commit = $StateCommit
    }
}

function New-R7SuccessfulCallOutcome {
    param($StateCommit = $null)
    [pscustomobject]@{
        success = $true
        failure_class = ""
        failure_code = ""
        failure_schema_version = ""
        failure_provenance_scope = ""
        failure_copy_group_id = ""
        failure_affected_call_ids = @()
        zero_dispatch = $false
        parse_status = "success"
        evidence_valid = $true
        violation_codes = @()
        violation_contexts = @()
        state_commit = $StateCommit
    }
}

function Resolve-R7StructuredCallOutcome {
    param($Payload, $ToolSuccess = $null)
    $schemaVersion = [string](Get-R7JsonProperty $Payload "schema_version" "")
    $stateCommit = Get-R7JsonProperty $Payload "state_commit"
    $innerSuccess = Get-R7JsonProperty $Payload "success"
    if ($innerSuccess -isnot [bool]) {
        return New-R7InvalidCallOutcome `
            "failure_payload_incomplete" `
            "incomplete_failure_payload" `
            $schemaVersion `
            $stateCommit
    }
    if ($ToolSuccess -is [bool] -and
        [bool]$innerSuccess -ne [bool]$ToolSuccess) {
        return New-R7InvalidCallOutcome `
            "outer_inner_success_mismatch" `
            "outer_inner_success_mismatch" `
            $schemaVersion `
            $stateCommit
    }
    if ([bool]$innerSuccess) {
        return New-R7SuccessfulCallOutcome $stateCommit
    }
    $structured = Get-R7StructuredFailureOutcome $Payload
    $structured | Add-Member -Force -NotePropertyName success -NotePropertyValue $false
    $structured
}

function Get-R7ResponseCommitShapeError {
    param($Payload)
    foreach ($field in @(
            "status", "success", "state_commit", "map_id", "action",
            "revision_before", "revision_after", "reserved_actions"
        )) {
        if (-not ($Payload.PSObject.Properties.Name -contains $field)) {
            return "TaskSpaceResponseCommitV1 is missing $field"
        }
    }
    if ([string]$Payload.status -cne "accepted" -or
        $Payload.success -isnot [bool] -or -not [bool]$Payload.success -or
        $Payload.state_commit -isnot [bool] -or -not [bool]$Payload.state_commit) {
        return "TaskSpaceResponseCommitV1 has invalid success semantics"
    }
    if ($Payload.map_id -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$Payload.map_id)) {
        return "TaskSpaceResponseCommitV1 map_id must be a non-empty string"
    }
    if ([string]$Payload.action -cnotin @(
            "initialize_and_execute", "execute", "reopen_map"
        )) {
        return "TaskSpaceResponseCommitV1 action is unsupported"
    }
    foreach ($field in @("revision_before", "revision_after")) {
        if ($Payload.$field -isnot [long] -and $Payload.$field -isnot [int]) {
            return "TaskSpaceResponseCommitV1 $field must be an integer"
        }
        if ([int64]$Payload.$field -lt 0) {
            return "TaskSpaceResponseCommitV1 $field must be nonnegative"
        }
    }
    if ([int64]$Payload.revision_after -le [int64]$Payload.revision_before) {
        return "TaskSpaceResponseCommitV1 revision_after must advance"
    }
    if ($Payload.reserved_actions -isnot [System.Array] -or
        $Payload.reserved_actions.Count -eq 0) {
        return "TaskSpaceResponseCommitV1 reserved_actions must be non-empty"
    }
    $callIds = [Collections.Generic.HashSet[string]]::new(
        [StringComparer]::Ordinal
    )
    $reservationIds = [Collections.Generic.HashSet[string]]::new(
        [StringComparer]::Ordinal
    )
    for ($index = 0; $index -lt $Payload.reserved_actions.Count; $index++) {
        $reservation = $Payload.reserved_actions[$index]
        if ($reservation -isnot [pscustomobject] -or
            $reservation.call_index -isnot [long] -and
            $reservation.call_index -isnot [int] -or
            [int64]$reservation.call_index -ne $index) {
            return "TaskSpaceResponseCommitV1 call_index is invalid"
        }
        foreach ($field in @("call_id", "node_id", "tool", "reservation_id")) {
            if ($reservation.$field -isnot [string] -or
                [string]::IsNullOrWhiteSpace([string]$reservation.$field)) {
                return "TaskSpaceResponseCommitV1 reserved action $field is invalid"
            }
        }
        if (-not $callIds.Add([string]$reservation.call_id) -or
            -not $reservationIds.Add([string]$reservation.reservation_id)) {
            return "TaskSpaceResponseCommitV1 reservation identities must be unique"
        }
    }
    ""
}

function Resolve-R7ResponseCommitOutcome {
    param($Payload, $ToolSuccess = $null)
    $shapeError = Get-R7ResponseCommitShapeError $Payload
    if (-not [string]::IsNullOrWhiteSpace($shapeError)) {
        return New-R7InvalidCallOutcome `
            "response_commit_payload_incomplete" `
            "incomplete_response_commit_payload" `
            "TaskSpaceResponseCommitV1" `
            (Get-R7JsonProperty $Payload "state_commit")
    }
    if ($ToolSuccess -is [bool] -and -not [bool]$ToolSuccess) {
        return New-R7InvalidCallOutcome `
            "outer_inner_success_mismatch" `
            "outer_inner_success_mismatch" `
            "TaskSpaceResponseCommitV1" `
            $true
    }
    New-R7SuccessfulCallOutcome $true
}

function Complete-R7CallOutcomeFacts {
    param(
        $Outcome,
        $Payload = $null,
        [string]$CarrierSchema = ""
    )
    if ([string]::IsNullOrWhiteSpace($CarrierSchema)) {
        $CarrierSchema = [string](
            Get-R7JsonProperty $Payload "schema_version" ""
        )
    }
    if ([string]::IsNullOrWhiteSpace($CarrierSchema)) {
        $CarrierSchema = if ([string]$Outcome.parse_status -in @(
                "malformed_failure_json",
                "duplicate_failure_json_property"
            )) {
            "unparsed"
        } else {
            "none"
        }
    }
    $reasonCode = [string]$Outcome.failure_code
    if ([string]::IsNullOrWhiteSpace($reasonCode)) {
        $reasonCode = "none"
    }
    $Outcome | Add-Member -Force `
        -NotePropertyName carrier_schema `
        -NotePropertyValue $CarrierSchema
    $Outcome | Add-Member -Force `
        -NotePropertyName reason_code `
        -NotePropertyValue $reasonCode
    $Outcome | Add-Member -Force `
        -NotePropertyName parsed_payload `
        -NotePropertyValue $Payload
    $Outcome | Add-Member -Force `
        -NotePropertyName carrier_action `
        -NotePropertyValue ([string](
            Get-R7JsonProperty $Payload "action" ""
        ))
    $Outcome | Add-Member -Force `
        -NotePropertyName carrier_revision_before `
        -NotePropertyValue (
            Get-R7JsonProperty $Payload "revision_before"
        )
    $Outcome
}

function Get-R7CallOutcome {
    param(
        $ToolSuccess = $null,
        [string]$Output,
        [string]$ToolName = "",
        [switch]$TrustedRuntimeCarrier
    )
    $trimmed = $Output.Trim()
    $payload = $null
    $parseError = $null
    if ($trimmed.StartsWith("{")) {
        try {
            $payload = ConvertFrom-R7StrictJsonObject $trimmed
        } catch {
            $parseError = $_
        }
    }
    $schemaVersion = [string](Get-R7JsonProperty $payload "schema_version" "")
    $structuredFailureSchemas = @(Get-R7StructuredFailureSchemas)
    $reservedSchemas = @(Get-R7ReservedTaskspaceCarrierSchemas)
    $isControlTool = $ToolName -ceq "taskspace_control"
    $isStructuredFailureSchema =
        $schemaVersion -cin $structuredFailureSchemas
    $isResponseCommitSchema = $schemaVersion -ceq "TaskSpaceResponseCommitV1"
    $isReservedTaskspaceSchema = $schemaVersion -cin $reservedSchemas

    if ($isControlTool) {
        if ($null -ne $parseError -or $null -eq $payload) {
            $reason = Get-R7StrictJsonFailureReason $parseError
            $outcome = New-R7InvalidCallOutcome $reason.code $reason.status
            return Complete-R7CallOutcomeFacts $outcome $null "unparsed"
        }
        if ($ToolSuccess -isnot [bool]) {
            $outcome = New-R7InvalidCallOutcome `
                "tool_transport_status_missing" `
                "missing_tool_transport_status" `
                $schemaVersion `
                (Get-R7JsonProperty $payload "state_commit")
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        if ($isResponseCommitSchema) {
            $outcome = Resolve-R7ResponseCommitOutcome $payload $ToolSuccess
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        if (-not $isStructuredFailureSchema) {
            $outcome = New-R7InvalidCallOutcome `
                "control_result_schema_mismatch" `
                "control_result_schema_mismatch" `
                $schemaVersion
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        $outcome = Resolve-R7StructuredCallOutcome $payload $ToolSuccess
        return Complete-R7CallOutcomeFacts $outcome $payload
    }

    if ($isReservedTaskspaceSchema) {
        if (-not $TrustedRuntimeCarrier) {
            $outcome = New-R7InvalidCallOutcome `
                "taskspace_failure_untrusted_carrier" `
                "untrusted_structured_failure_carrier" `
                $schemaVersion `
                (Get-R7JsonProperty $payload "state_commit")
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        if ($ToolSuccess -isnot [bool]) {
            $outcome = New-R7InvalidCallOutcome `
                "tool_transport_status_missing" `
                "missing_tool_transport_status" `
                $schemaVersion `
                (Get-R7JsonProperty $payload "state_commit")
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        if ($isResponseCommitSchema) {
            $outcome = Resolve-R7ResponseCommitOutcome $payload $ToolSuccess
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        $outcome = Resolve-R7StructuredCallOutcome $payload $ToolSuccess
        return Complete-R7CallOutcomeFacts $outcome $payload
    }

    if ($TrustedRuntimeCarrier) {
        if ($null -ne $parseError -or $null -eq $payload) {
            $reason = Get-R7StrictJsonFailureReason $parseError
            $outcome = New-R7InvalidCallOutcome $reason.code $reason.status
            return Complete-R7CallOutcomeFacts $outcome $null "unparsed"
        }
        if ($ToolSuccess -isnot [bool]) {
            $outcome = New-R7InvalidCallOutcome `
                "tool_transport_status_missing" `
                "missing_tool_transport_status" `
                $schemaVersion `
                (Get-R7JsonProperty $payload "state_commit")
            return Complete-R7CallOutcomeFacts $outcome $payload
        }
        $outcome = New-R7InvalidCallOutcome `
            "failure_schema_unknown" `
            "unknown_failure_schema" `
            $schemaVersion `
            (Get-R7JsonProperty $payload "state_commit")
        return Complete-R7CallOutcomeFacts $outcome $payload
    }

    if ($ToolSuccess -is [bool] -and [bool]$ToolSuccess) {
        return Complete-R7CallOutcomeFacts (
            New-R7SuccessfulCallOutcome
        ) $payload
    }

    $ordinaryCode = Get-TaskspaceOrdinaryToolFailureCode $Output
    $valid = -not [string]::IsNullOrWhiteSpace($ordinaryCode)
    if ($ToolSuccess -isnot [bool] -and -not $valid) {
        return Complete-R7CallOutcomeFacts (
            New-R7SuccessfulCallOutcome
        ) $payload
    }
    $outcome = [pscustomobject]@{
        success = $false
        failure_class = if ($valid) { "ordinary_tool" } else { "evidence_unclassified" }
        failure_code = if ($valid) { $ordinaryCode } else { "tool_failed_unclassified" }
        failure_schema_version = ""
        failure_provenance_scope = ""
        failure_copy_group_id = ""
        failure_affected_call_ids = @()
        zero_dispatch = $false
        parse_status = if ($valid) { "ordinary_failure" } else { "ordinary_failure_unclassified" }
        evidence_valid = $valid
        violation_codes = @()
        violation_contexts = @()
        state_commit = $null
    }
    Complete-R7CallOutcomeFacts $outcome $payload
}

function Get-R7StrictJsonFailureReason {
    param($ErrorRecord)
    if ($null -ne $ErrorRecord -and
        [string]$ErrorRecord.Exception.Message -like "Duplicate JSON property:*") {
        return [pscustomobject]@{
            code = "failure_payload_duplicate_property"
            status = "duplicate_failure_json_property"
        }
    }
    [pscustomobject]@{
        code = "failure_payload_parse_failed"
        status = "malformed_failure_json"
    }
}
