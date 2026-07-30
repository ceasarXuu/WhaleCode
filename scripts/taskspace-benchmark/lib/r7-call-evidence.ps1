function ConvertTo-R7CallDescriptor {
    param(
        [Parameter(Mandatory = $true)][string]$CallId,
        [Parameter(Mandatory = $true)][string]$ToolName,
        [Parameter(Mandatory = $true)][string]$Arguments,
        [string]$CallType = "function_call"
    )
    $parsed = $null
    $argumentParseStatus = "not_applicable"
    if (-not [string]::IsNullOrWhiteSpace($Arguments)) {
        try {
            $parsed = $Arguments | ConvertFrom-Json -Depth 100
            $argumentParseStatus = "valid_json"
        } catch {
            $argumentParseStatus = "invalid_json"
        }
    }
    $controlAction = if ($ToolName -eq "taskspace_control") {
        [string](Get-R7JsonProperty $parsed "action" "")
    } else {
        ""
    }
    $declaredActions = @(
        Get-R7JsonProperty $parsed "actions" @() |
            ForEach-Object {
                [pscustomobject]@{
                    node_id = [string](Get-R7JsonProperty $_ "node_id" "")
                    tool = [string](Get-R7JsonProperty $_ "tool" "")
                }
            }
    )
    $node = if ($controlAction -eq "finish_map") {
        [string](Get-R7JsonProperty $parsed "finish_node_id" "")
    } else {
        @($declaredActions | ForEach-Object node_id) -join ","
    }
    $detail = ""
    if ($ToolName -in @("exec_command", "shell_command", "local_shell")) {
        $detail = [string](Get-R7JsonProperty $parsed "cmd" "")
        if ([string]::IsNullOrWhiteSpace($detail)) {
            $detail = [string](Get-R7JsonProperty $parsed "action" "")
        }
        $detail = ($detail -replace '[\r\n]+', ' ').Trim()
        if ($detail.Length -gt 120) { $detail = $detail.Substring(0, 120) }
    } elseif ($ToolName -eq "apply_patch") {
        $patchText = [string](Get-R7JsonProperty $parsed "input" "")
        $fileCount = @([regex]::Matches($patchText, '(?m)^\*\*\* (?:Add|Update|Delete) File:')).Count
        $detail = "patch_files=$fileCount"
    }
    [pscustomobject]@{
        call_id = $CallId
        tool = $ToolName
        call_type = $CallType
        request_index = 0
        control_action = $controlAction
        declared_node_id = ""
        declared_actions = $declaredActions
        expected_reservation_id = ""
        node = $node
        detail = $detail
        argument_parse_status = $argumentParseStatus
        success = $null
        failure_class = ""
        failure_code = ""
        failure_schema_version = ""
        failure_provenance_scope = ""
        failure_copy_group_id = ""
        failure_affected_call_ids = @()
        zero_dispatch = $false
        parse_status = "output_pending"
        evidence_valid = $true
        output_count = 0
        observed_output_text = ""
        observed_output_tool_success = $null
        supplemental_count = 0
        violation_codes = @()
        violation_contexts = @()
        state_commit = $null
    }
}

function Get-R7FailureClass {
    param([string]$ErrorClass)
    switch ($ErrorClass) {
        "state_machine" { "taskspace_state_machine" }
        "protocol" { "taskspace_protocol" }
        "argument" { "taskspace_protocol" }
        "resource" { "taskspace_resource" }
        "tool" { "ordinary_tool" }
        default { "taskspace" }
    }
}

function Get-R7SupplementalFailureShapeError {
    param($Payload)
    if ($Payload -is [System.Array] -or $Payload -isnot [pscustomobject]) {
        return "structured failure root must be an object"
    }
    if (-not ($Payload.PSObject.Properties.Name -contains "schema_version") -or
        $Payload.schema_version -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$Payload.schema_version)) {
        return "schema_version must be a non-empty string"
    }
    $schemaVersion = [string]$Payload.schema_version
    $required = @("status", "success", "failure_provenance", "error")
    foreach ($field in $required) {
        if ($null -eq $Payload -or
            -not ($Payload.PSObject.Properties.Name -contains $field)) {
            return "$schemaVersion is missing $field"
        }
    }
    if ($Payload.status -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$Payload.status)) {
        return "$schemaVersion status must be a non-empty string"
    }
    if ($Payload.success -isnot [bool] -or [bool]$Payload.success) {
        return "$schemaVersion success must be boolean false"
    }
    $provenance = Get-R7JsonProperty $Payload "failure_provenance"
    if ($provenance -isnot [pscustomobject]) {
        return "$schemaVersion failure_provenance must be an object"
    }
    foreach ($field in @("scope", "copy_group_id")) {
        if ($provenance.$field -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$provenance.$field)) {
            return "$schemaVersion failure_provenance.$field must be a non-empty string"
        }
    }
    if ($provenance.zero_dispatch -isnot [bool]) {
        return "$schemaVersion failure_provenance.zero_dispatch must be boolean"
    }
    if (-not ($provenance.PSObject.Properties.Name -contains "affected_call_ids")) {
        return "$schemaVersion failure_provenance.affected_call_ids is missing"
    }
    $affectedCallIds = $provenance.affected_call_ids
    if ($affectedCallIds -isnot [System.Array] -or $affectedCallIds.Count -eq 0) {
        return "$schemaVersion failure_provenance.affected_call_ids must be a non-empty array"
    }
    foreach ($callId in $affectedCallIds) {
        if ($callId -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$callId)) {
            return "$schemaVersion affected_call_ids entries must be non-empty strings"
        }
    }
    $error = Get-R7JsonProperty $Payload "error"
    if ($error -isnot [pscustomobject]) {
        return "$schemaVersion error must be an object"
    }
    foreach ($field in @("class", "code")) {
        if (-not ($error.PSObject.Properties.Name -contains $field) -or
            $error.$field -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$error.$field)) {
            return "$schemaVersion error.$field must be a non-empty string"
        }
    }
    $allowedStatuses = @(
        switch ($schemaVersion) {
            "TaskSpaceResponseCommitFailureV3" {
                "state_rejected"
                "protocol_rejected"
                "resource_failed"
            }
            "ToolSequencePreflightResultV3" { "protocol_failed" }
            "ProviderToolResponsePreflightV2" { "protocol_failed" }
            "ToolSearchFailureV3" { "failed" }
            "TaskSpaceToolSkippedV2" {
                "skipped_due_to_prior_failure"
                "skipped_due_to_terminal_completion"
            }
            "TaskSpaceBoundResultCommitFailureV2" { "failed" }
        }
    )
    if ([string]$Payload.status -notin $allowedStatuses) {
        return "$schemaVersion has an invalid status"
    }
    $allowedErrorClasses = @(
        switch ($schemaVersion) {
            "TaskSpaceResponseCommitFailureV3" {
                "state_machine"
                "protocol"
                "resource"
            }
            "ToolSequencePreflightResultV3" { "protocol" }
            "ProviderToolResponsePreflightV2" { "protocol" }
            "ToolSearchFailureV3" { "tool" }
            "TaskSpaceToolSkippedV2" { "tool" }
            "TaskSpaceBoundResultCommitFailureV2" { "resource" }
        }
    )
    if ([string]$error.class -notin $allowedErrorClasses) {
        return "$schemaVersion has an invalid error.class"
    }
    if ($schemaVersion -notin @(
            "ToolSearchFailureV3",
            "TaskSpaceToolSkippedV2"
        )) {
        if (-not ($Payload.PSObject.Properties.Name -contains "state_commit") -or
            $Payload.state_commit -isnot [bool] -or
            [bool]$Payload.state_commit) {
            return "$schemaVersion state_commit must be boolean false"
        }
    }
    if ($schemaVersion -eq "ToolSearchFailureV3") {
        foreach ($field in @("call_id", "pairing_status", "execution_status")) {
            if (-not ($Payload.PSObject.Properties.Name -contains $field) -or
                $Payload.$field -isnot [string] -or
                [string]::IsNullOrWhiteSpace([string]$Payload.$field)) {
                return "$schemaVersion is missing $field"
            }
        }
        if ([string]$Payload.pairing_status -ne "completed" -or
            [string]$Payload.execution_status -ne "failed") {
            return "$schemaVersion has invalid pairing or execution status"
        }
        if (-not ($error.PSObject.Properties.Name -contains "cause") -or
            $error.cause -isnot [pscustomobject]) {
            return "$schemaVersion error.cause must be an object"
        }
        if ($Payload.tool -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$Payload.tool)) {
            return "$schemaVersion tool must be a non-empty string"
        }
    }
    if ($schemaVersion -in @(
            "ToolSearchFailureV3",
            "TaskSpaceToolSkippedV2",
            "TaskSpaceBoundResultCommitFailureV2"
        )) {
        if ($Payload.call_id -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$Payload.call_id)) {
            return "$schemaVersion call_id must be a non-empty string"
        }
    }
    if ($schemaVersion -in @(
            "ToolSearchFailureV3",
            "TaskSpaceToolSkippedV2"
        )) {
        if ($provenance.cause_call_id -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$provenance.cause_call_id)) {
            return "$schemaVersion failure_provenance.cause_call_id must be a non-empty string"
        }
    }
    if ($schemaVersion -eq "TaskSpaceToolSkippedV2") {
        if ($Payload.tool -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$Payload.tool) -or
            $Payload.cause -isnot [pscustomobject]) {
            return "$schemaVersion has an invalid tool or cause"
        }
        foreach ($field in @("field", "call_id")) {
            if ($Payload.cause.$field -isnot [string] -or
                [string]::IsNullOrWhiteSpace([string]$Payload.cause.$field)) {
                return "$schemaVersion cause.$field must be a non-empty string"
            }
        }
    }
    if ($schemaVersion -eq "TaskSpaceBoundResultCommitFailureV2" -and
        ($Payload.reservation_id -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$Payload.reservation_id))) {
        return "$schemaVersion reservation_id must be a non-empty string"
    }
    $stateShapeError = Get-R7StateFailureShapeError $Payload
    if (-not [string]::IsNullOrWhiteSpace($stateShapeError)) {
        return $stateShapeError
    }
    ""
}

function Get-R7StructuredFailureOutcome {
    param($Payload)
    $schemaVersion = [string](Get-R7JsonProperty $Payload "schema_version" "")
    $knownSchemas = @(
        "TaskSpaceControlResultV2",
        "TaskSpaceResponseCommitFailureV3",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV3",
        "TaskSpaceToolSkippedV2",
        "TaskSpaceBoundResultCommitFailureV2"
    )
    if ($schemaVersion -notin $knownSchemas) {
        return [pscustomobject]@{
            failure_class = "evidence_unclassified"
            failure_code = "failure_schema_unknown"
            failure_schema_version = $schemaVersion
            failure_provenance_scope = ""
            failure_copy_group_id = ""
            failure_affected_call_ids = @()
            zero_dispatch = $false
            parse_status = "unknown_failure_schema"
            evidence_valid = $false
            violation_codes = @()
            violation_contexts = @()
            state_commit = Get-R7JsonProperty $Payload "state_commit"
        }
    }

    $shapeError = if ($schemaVersion -eq "TaskSpaceControlResultV2") {
        Get-R7ControlFailureEnvelopeShapeError $Payload
    } else {
        Get-R7SupplementalFailureShapeError $Payload
    }
    $error = Get-R7JsonProperty $Payload "error"
    $errorClass = [string](Get-R7JsonProperty $error "class" "")
    $errorCode = [string](Get-R7JsonProperty $error "code" "")
    $actual = Get-R7JsonProperty $error "actual"
    $violations = @(Get-R7JsonProperty $error "violations" @())
    if (-not $violations.Count -and $actual -is [pscustomobject]) {
        $violations = @(Get-R7JsonProperty $actual "violations" @())
    }
    $provenance = Get-R7JsonProperty $Payload "failure_provenance"
    $valid = [string]::IsNullOrWhiteSpace($shapeError) -and
        -not [string]::IsNullOrWhiteSpace($errorCode)
    [pscustomobject]@{
        failure_class = if (-not $valid) {
            "evidence_unclassified"
        } elseif ($schemaVersion -eq "ToolSequencePreflightResultV3") {
            "tool_sequence_protocol"
        } else {
            Get-R7FailureClass $errorClass
        }
        failure_code = if ($valid) { $errorCode } else { "failure_payload_incomplete" }
        failure_schema_version = $schemaVersion
        failure_provenance_scope = [string](Get-R7JsonProperty $provenance "scope" "")
        failure_copy_group_id = [string](Get-R7JsonProperty $provenance "copy_group_id" "")
        failure_affected_call_ids = @(
            Get-R7JsonProperty $provenance "affected_call_ids" @() |
                ForEach-Object { [string]$_ }
        )
        zero_dispatch = [bool](Get-R7JsonProperty $provenance "zero_dispatch" $false)
        parse_status = if ($valid) { "structured_failure" } else { "incomplete_failure_payload" }
        evidence_valid = $valid
        violation_codes = @(
            $violations |
                ForEach-Object { [string](Get-R7JsonProperty $_ "code" "") } |
                Where-Object { $_ } |
                Sort-Object -Unique
        )
        violation_contexts = $violations
        state_commit = Get-R7JsonProperty $Payload "state_commit"
    }
}

function Get-R7CallOutcome {
    param(
        [bool]$ToolSuccess,
        [string]$Output,
        [string]$ToolName = "",
        [switch]$TrustedRuntimeCarrier
    )
    if ($ToolSuccess) {
        $stateCommit = $null
        $trimmed = $Output.Trim()
        if ($trimmed.StartsWith("{")) {
            try {
                $payload = $trimmed | ConvertFrom-Json -Depth 100
                $stateCommit = Get-R7JsonProperty $payload "state_commit"
            } catch {}
        }
        return [pscustomobject]@{
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
            state_commit = $stateCommit
        }
    }

    $trimmed = $Output.Trim()
    if ($trimmed.StartsWith("{")) {
        try {
            $payload = $trimmed | ConvertFrom-Json -Depth 100
        } catch {
            return [pscustomobject]@{
                success = $false
                failure_class = "evidence_unclassified"
                failure_code = "failure_payload_parse_failed"
                failure_schema_version = ""
                failure_provenance_scope = ""
                failure_copy_group_id = ""
                failure_affected_call_ids = @()
                zero_dispatch = $false
                parse_status = "malformed_failure_json"
                evidence_valid = $false
                violation_codes = @()
                violation_contexts = @()
                state_commit = $null
            }
        }
        $schemaVersion = [string](Get-R7JsonProperty $payload "schema_version" "")
        if (-not [string]::IsNullOrWhiteSpace($schemaVersion)) {
            $trustedControlResult =
                $schemaVersion -eq "TaskSpaceControlResultV2" -and
                $ToolName -eq "taskspace_control"
            if (-not $TrustedRuntimeCarrier -and -not $trustedControlResult) {
                return [pscustomobject]@{
                    success = $false
                    failure_class = "evidence_unclassified"
                    failure_code = "taskspace_failure_untrusted_carrier"
                    failure_schema_version = $schemaVersion
                    failure_provenance_scope = ""
                    failure_copy_group_id = ""
                    failure_affected_call_ids = @()
                    zero_dispatch = $false
                    parse_status = "untrusted_structured_failure_carrier"
                    evidence_valid = $false
                    violation_codes = @()
                    violation_contexts = @()
                    state_commit = Get-R7JsonProperty $payload "state_commit"
                }
            }
            $structured = Get-R7StructuredFailureOutcome $payload
            $structured | Add-Member -Force -NotePropertyName success -NotePropertyValue $false
            return $structured
        }
    }

    $ordinaryCode = Get-TaskspaceOrdinaryToolFailureCode $Output
    $valid = -not [string]::IsNullOrWhiteSpace($ordinaryCode)
    [pscustomobject]@{
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
}

function Set-R7CallOutcome {
    param($Call, $Outcome)
    foreach ($name in @(
            "success", "failure_class", "failure_code", "failure_schema_version",
            "failure_provenance_scope", "failure_copy_group_id",
            "failure_affected_call_ids", "zero_dispatch", "parse_status", "evidence_valid",
            "violation_codes", "violation_contexts", "state_commit"
        )) {
        $Call.$name = $Outcome.$name
    }
}
