function ConvertTo-R7CallDescriptor {
    param(
        [Parameter(Mandatory = $true)][string]$CallId,
        [Parameter(Mandatory = $true)][string]$ToolName,
        [Parameter(Mandatory = $true)][string]$Arguments
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
        control_action = $controlAction
        declared_node_id = ""
        declared_actions = $declaredActions
        node = $node
        detail = $detail
        argument_parse_status = $argumentParseStatus
        success = $null
        failure_class = ""
        failure_code = ""
        failure_schema_version = ""
        failure_copy_group_id = ""
        zero_dispatch = $false
        parse_status = "output_pending"
        evidence_valid = $true
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

function Get-R7StructuredFailureOutcome {
    param($Payload)
    $schemaVersion = [string](Get-R7JsonProperty $Payload "schema_version" "")
    $knownSchemas = @(
        "TaskSpaceControlResultV2",
        "TaskSpaceResponseCommitFailureV2",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV2",
        "TaskSpaceToolSkippedV1"
    )
    if ($schemaVersion -notin $knownSchemas) {
        return [pscustomobject]@{
            failure_class = "evidence_unclassified"
            failure_code = "failure_schema_unknown"
            failure_schema_version = $schemaVersion
            failure_copy_group_id = ""
            zero_dispatch = $false
            parse_status = "unknown_failure_schema"
            evidence_valid = $false
            violation_codes = @()
            violation_contexts = @()
            state_commit = Get-R7JsonProperty $Payload "state_commit"
        }
    }

    $error = Get-R7JsonProperty $Payload "error"
    if ($schemaVersion -eq "ToolSearchFailureV2") {
        $cause = Get-R7JsonProperty $error "cause"
        if ($null -ne $cause -and [string](Get-R7JsonProperty $cause "format" "") -ne "text") {
            $causeOutcome = Get-R7StructuredFailureOutcome $cause
            $causeOutcome.failure_schema_version = $schemaVersion
            return $causeOutcome
        }
    }
    $errorClass = [string](Get-R7JsonProperty $error "class" "")
    $errorCode = [string](Get-R7JsonProperty $error "code" "")
    if ($schemaVersion -eq "ToolSearchFailureV2" -and [string]::IsNullOrWhiteSpace($errorCode)) {
        $cause = Get-R7JsonProperty $error "cause"
        $causeText = [string](Get-R7JsonProperty $cause "text" "")
        $errorCode = Get-TaskspaceOrdinaryToolFailureCode $causeText
        if ([string]::IsNullOrWhiteSpace($errorCode)) { $errorCode = "tool_failed_unclassified" }
    }
    $violations = @(Get-R7JsonProperty $error "violations" @())
    if (-not $violations.Count) { $violations = @(Get-R7JsonProperty $Payload "violations" @()) }
    $provenance = Get-R7JsonProperty $Payload "failure_provenance"
    $valid = -not [string]::IsNullOrWhiteSpace($errorCode)
    [pscustomobject]@{
        failure_class = if ($schemaVersion -eq "ToolSequencePreflightResultV3") {
            "tool_sequence_protocol"
        } else {
            Get-R7FailureClass $errorClass
        }
        failure_code = if ($valid) { $errorCode } else { "failure_code_missing" }
        failure_schema_version = $schemaVersion
        failure_copy_group_id = [string](Get-R7JsonProperty $provenance "copy_group_id" "")
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
    param([bool]$ToolSuccess, [string]$Output)
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
            failure_copy_group_id = ""
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
                failure_copy_group_id = ""
                zero_dispatch = $false
                parse_status = "malformed_failure_json"
                evidence_valid = $false
                violation_codes = @()
                violation_contexts = @()
                state_commit = $null
            }
        }
        if (-not [string]::IsNullOrWhiteSpace([string](Get-R7JsonProperty $payload "schema_version" "")) -or
            $null -ne (Get-R7JsonProperty $payload "error")) {
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
        failure_copy_group_id = ""
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
            "failure_copy_group_id", "zero_dispatch", "parse_status", "evidence_valid",
            "violation_codes", "violation_contexts", "state_commit"
        )) {
        $Call.$name = $Outcome.$name
    }
}
