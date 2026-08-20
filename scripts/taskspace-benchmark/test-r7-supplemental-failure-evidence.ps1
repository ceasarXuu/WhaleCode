$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-supplemental-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

function Write-Lines([string]$Path, [object[]]$Rows) {
    $lines = @($Rows | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 30 })
    [IO.File]::WriteAllLines($Path, $lines, [Text.UTF8Encoding]::new($false))
}

function New-TokenBoundary([string]$RequestId) {
    @{
        type = "event_msg"
        payload = @{
            type = "token_count"
            provider_request_id = $RequestId
            provider_logical_request_id = "$RequestId-logical"
            provider_attempt_seq = 1
            info = @{ last_token_usage = @{ input_tokens = 10; cached_input_tokens = 0; output_tokens = 2; reasoning_output_tokens = 1; total_tokens = 12 } }
        }
    }
}

function Assert-StandardSupplementalRejected(
    [string]$Name,
    [string[]]$Messages,
    [string]$ExpectedError,
    [switch]$OutputAfterMessages
) {
    $path = Join-Path $tempRoot "$Name.jsonl"
    $rows = [Collections.Generic.List[object]]::new()
    $rows.Add(@{
            type = "response_item"
            payload = @{
                type = "tool_search_call"
                arguments = @{ query = "read_file" }
                call_id = "$Name-search"
            }
        })
    $rows.Add((New-TokenBoundary "$Name-request"))
    $output = @{
        type = "response_item"
        payload = @{
            type = "tool_search_output"
            call_id = "$Name-search"
            status = "completed"
            execution = "client"
            tools = @()
        }
    }
    if (-not $OutputAfterMessages) { $rows.Add($output) }
    foreach ($message in $Messages) {
        $rows.Add(@{
                type = "response_item"
                payload = @{
                    type = "message"
                    role = "developer"
                    content = @(@{ type = "input_text"; text = $message })
                }
            })
    }
    if ($OutputAfterMessages) { $rows.Add($output) }
    Write-Lines $path @($rows)
    $rejected = $false
    $observedError = ""
    try {
        Get-R7StandardRequestPath $path 1 | Out-Null
    } catch {
        $observedError = $_.Exception.Message
        $rejected = $_.Exception.Message -like $ExpectedError
    }
    if (-not $rejected) {
        throw "$Name supplemental evidence did not fail closed: $observedError"
    }
}

function New-ProvenanceFixture([string]$Kind) {
    if ($Kind -eq "provider") {
        $calls = @(
            (ConvertTo-R7CallDescriptor "provider-control" "taskspace_control" "{}"),
            (ConvertTo-R7CallDescriptor "provider-tool" "exec_command" "{}")
        )
        $payload = [pscustomobject]@{
            schema_version = "TaskSpaceResponseCommitFailureV3"
            status = "state_rejected"
            success = $false
            state_commit = $false
            canonical_revision = 4
            current_revision = 4
            rejected_candidate_committed = $false
            executed_tool_call_count = 0
            failure_provenance = [pscustomobject]@{
                scope = "provider_response"
                copy_group_id = "provider_response:provider-control"
                zero_dispatch = $true
                affected_call_ids = @("provider-control", "provider-tool")
            }
            error = [pscustomobject]@{
                class = "state_machine"
                code = "taskspace_response_state_commit_failed"
                violations = @([pscustomobject]@{ code = "stale_revision"; subjects = @("map") })
            }
        }
    } elseif ($Kind -eq "skip") {
        $causeCall = ConvertTo-R7CallDescriptor "skip-cause" "exec_command" "{}"
        $causeCall.output_count = 1
        Set-R7CallOutcome $causeCall (
            Get-R7CallOutcome -ToolSuccess $false -Output "Shell exit code: 1"
        )
        $targetCall = ConvertTo-R7CallDescriptor "skip-target" "exec_command" "{}"
        $targetCall.output_count = 1
        $calls = @($causeCall, $targetCall)
        $payload = [pscustomobject]@{
            schema_version = "TaskSpaceToolSkippedV2"
            status = "skipped_due_to_prior_failure"
            success = $false
            call_id = "skip-target"
            tool = "exec_command"
            failure_provenance = [pscustomobject]@{
                scope = "tool_sequence_skip"
                copy_group_id = "tool_sequence_skip:skip-cause"
                zero_dispatch = $true
                cause_call_id = "skip-cause"
                affected_call_ids = @("skip-target")
            }
            error = [pscustomobject]@{
                class = "tool"
                code = "skipped_due_to_prior_failure"
            }
            cause = [pscustomobject]@{
                field = "prior_call_id"
                call_id = "skip-cause"
            }
        }
    } else {
        $call = ConvertTo-R7CallDescriptor "bound-target" "exec_command" "{}"
        $call.expected_reservation_id = "reservation:bound-target"
        $call.output_count = 1
        Set-R7CallOutcome $call (Get-R7CallOutcome -ToolSuccess $true -Output "ok")
        $calls = @($call)
        $payload = [pscustomobject]@{
            schema_version = "TaskSpaceBoundResultCommitFailureV2"
            status = "failed"
            success = $false
            state_commit = $false
            call_id = "bound-target"
            reservation_id = "reservation:bound-target"
            failure_provenance = [pscustomobject]@{
                scope = "tool_result_attribution"
                copy_group_id = "tool_result_attribution:bound-target"
                zero_dispatch = $false
                affected_call_ids = @("bound-target")
            }
            error = [pscustomobject]@{
                class = "resource"
                code = "taskspace_bound_result_commit_failed"
                detail = "failed"
            }
        }
    }
    [pscustomobject]@{
        calls = $calls
        payload = $payload
    }
}

function Assert-ProvenanceContract(
    [string]$Name,
    [string]$Kind,
    [scriptblock]$Mutation,
    [bool]$ExpectedAccepted
) {
    $fixture = New-ProvenanceFixture $Kind
    if ($null -ne $Mutation) { & $Mutation $fixture.payload $fixture.calls }
    $rawPayload = $fixture.payload | ConvertTo-Json -Compress -Depth 30
    if ($Kind -eq "provider") {
        foreach ($call in $fixture.calls) {
            if ([int]$call.output_count -eq 0) {
                $call.output_count = 1
                $call.observed_output_text = $rawPayload
                $call.observed_output_tool_success = $false
                Set-R7CallOutcome $call (
                    Get-R7CallOutcome -ToolSuccess $false -Output $rawPayload
                )
            }
        }
    }
    $callsById = @{}
    $requestCalls = [Collections.Generic.List[object]]::new()
    foreach ($call in $fixture.calls) {
        $callsById[[string]$call.call_id] = $call
        $requestCalls.Add($call)
    }
    $requests = [Collections.Generic.List[object]]::new()
    $requests.Add([pscustomobject]@{ calls = $requestCalls })
    $accepted = $true
    try {
        Apply-R7SupplementalFailure `
            $callsById `
            $requests `
            $rawPayload `
            "developer"
    } catch {
        $accepted = $false
    }
    if ($accepted -ne $ExpectedAccepted) {
        throw "$Name provenance contract acceptance was $accepted"
    }
}

try {
    $callId = "valid-search"
    $valid = '{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"' +
        $callId +
        '","tool":"tool_search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:' +
        $callId +
        '","zero_dispatch":false,"cause_call_id":"' +
        $callId +
        '","affected_call_ids":["' +
        $callId +
        '"]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}'
    $serializedOutcome = Get-R7ResponseItemOutcome ([pscustomobject]@{
            type = "function_call_output"
            call_id = "serialized"
            output = $valid
        })
    if ($null -ne $serializedOutcome.tool_success -or
        [string]$serializedOutcome.output_text -cne $valid) {
        throw "Serialized domain success leaked into Tool transport status"
    }
    $nonBooleanOutcome = Get-R7ResponseItemOutcome ([pscustomobject]@{
            type = "function_call_output"
            call_id = "non-boolean"
            output = '{"success":"false"}'
        })
    if ($null -ne $nonBooleanOutcome.tool_success) {
        throw "Non-boolean serialized success was coerced into a Tool fact"
    }

    Assert-StandardSupplementalRejected `
        "malformed" `
        @('{"schema_version":"ToolSearchFailureV3"') `
        "Malformed structured failure message"
    Assert-StandardSupplementalRejected `
        "malformed-reordered" `
        @('{"padding":0,"schema_version":"ToolSearchFailureV3"') `
        "Malformed structured failure message"
    Assert-StandardSupplementalRejected `
        "malformed-escaped-key" `
        @('{"schema_\u0076ersion":"ToolSearchFailureV3"') `
        "Malformed structured failure message"
    Assert-StandardSupplementalRejected `
        "root-array" `
        @('[{"schema_version":"ToolSearchFailureV3"}]') `
        "Structured failure root must be an object"
    Assert-StandardSupplementalRejected `
        "schema-array" `
        @('{"schema_version":["ToolSearchFailureV3"]}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "root-scalar" `
        @('"Tool\u0053earchFailureV3"') `
        "Structured failure root must be an object"
    Assert-StandardSupplementalRejected `
        "duplicate-escaped-schema" `
        @('{"schema_version":"ToolSearchFailureV3","schema_\u0076ersion":"benign"}') `
        "Duplicate JSON property:*"
    Assert-StandardSupplementalRejected `
        "nested-reserved-family" `
        @('{"schema_version":"benign","detail":"ToolSearchFailureV3"}') `
        "Unknown structured failure schema:*"
    Assert-StandardSupplementalRejected `
        "unknown" `
        @('{"schema_version":"ToolSearchFailureV4","status":"failed","success":false}') `
        "Unknown structured failure schema:*"
    Assert-StandardSupplementalRejected `
        "incomplete" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"failure_provenance":{},"error":{"class":"tool","code":"tool_search_failed"}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "non-boolean-success" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":"false","call_id":"non-boolean-success-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:non-boolean-success-search","zero_dispatch":false,"cause_call_id":"non-boolean-success-search","affected_call_ids":["non-boolean-success-search"]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "non-string-status" `
        @('{"schema_version":"ToolSearchFailureV3","status":1,"success":false,"call_id":"non-string-status-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:non-string-status-search","zero_dispatch":false,"cause_call_id":"non-string-status-search","affected_call_ids":["non-string-status-search"]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "invalid-status" `
        @('{"schema_version":"ToolSearchFailureV3","status":"completed","success":false,"call_id":"invalid-status-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:invalid-status-search","zero_dispatch":false,"cause_call_id":"invalid-status-search","affected_call_ids":["invalid-status-search"]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "scalar-provenance" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"scalar-provenance-search","pairing_status":"completed","execution_status":"failed","failure_provenance":"tool_execution","error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "scalar-affected" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"scalar-affected-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:scalar-affected-search","zero_dispatch":false,"cause_call_id":"scalar-affected-search","affected_call_ids":"scalar-affected-search"},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "string-zero-dispatch" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"string-zero-dispatch-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:string-zero-dispatch-search","zero_dispatch":"","cause_call_id":"string-zero-dispatch-search","affected_call_ids":["string-zero-dispatch-search"]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "scalar-cause" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"scalar-cause-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:scalar-cause-search","zero_dispatch":false,"cause_call_id":"scalar-cause-search","affected_call_ids":["scalar-cause-search"]},"error":{"class":"tool","code":"tool_search_failed","cause":"failed"}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "numeric-affected" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"numeric-affected-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:numeric-affected-search","zero_dispatch":false,"cause_call_id":"numeric-affected-search","affected_call_ids":[1]},"error":{"class":"tool","code":"tool_search_failed","cause":{"format":"text","text":"failed"}}}') `
        "Incomplete structured failure fact:*"
    Assert-StandardSupplementalRejected `
        "scalar-error" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"call_id":"scalar-error-search","pairing_status":"completed","execution_status":"failed","failure_provenance":{"scope":"tool_execution","copy_group_id":"tool_execution:scalar-error-search","zero_dispatch":false,"cause_call_id":"scalar-error-search","affected_call_ids":["scalar-error-search"]},"error":"failed"}') `
        "Incomplete structured failure fact:*"

    foreach ($kind in @("provider", "skip", "bound")) {
        Assert-ProvenanceContract "$kind-valid" $kind $null $true
    }
    $reservedCall = ConvertTo-R7CallDescriptor "reserved-call" "exec_command" "{}"
    $reservedCall.request_index = 1
    $reservationControl = ConvertTo-R7CallDescriptor `
        "reservation-control" `
        "taskspace_control" `
        '{"action":"execute","actions":[{"node_id":"work","tool":"exec_command"}]}'
    $reservationControl.request_index = 1
    $reservationCalls = @{
        "reservation-control" = $reservationControl
        "reserved-call" = $reservedCall
    }
    $reservationPayload =
        '{"schema_version":"TaskSpaceResponseResultV2",' +
        '"reserved_actions":[{"call_index":0,"call_id":"reserved-call",' +
        '"node_id":"work","tool":"exec_command",' +
        '"reservation_id":"reservation:reserved-call"}]}' |
        ConvertFrom-Json -Depth 20
    $reservationError = Set-R7ExpectedReservations `
        $reservationCalls `
        $reservationControl `
        $reservationPayload
    if (-not [string]::IsNullOrWhiteSpace([string]$reservationError) -or
        [string]$reservedCall.expected_reservation_id -ne
            "reservation:reserved-call") {
        throw "Control result reservation identity was not preserved"
    }
    $reservedCall.request_index = 2
    $crossRequestError = Set-R7ExpectedReservations `
        $reservationCalls `
        $reservationControl `
        $reservationPayload
    if ([string]::IsNullOrWhiteSpace([string]$crossRequestError)) {
        throw "Control result reservation crossed request identity"
    }
    Assert-ProvenanceContract "provider-copy-group" "provider" {
        param($payload) $payload.failure_provenance.copy_group_id = "forged"
    } $false
    Assert-ProvenanceContract "provider-call-order" "provider" {
        param($payload) [array]::Reverse($payload.failure_provenance.affected_call_ids)
    } $false
    Assert-ProvenanceContract "provider-missing-violations" "provider" {
        param($payload) $payload.error.PSObject.Properties.Remove("violations")
    } $false
    Assert-ProvenanceContract "provider-without-output" "provider" {
        param($payload, $calls) $calls[0].output_count = 2
    } $false
    Assert-ProvenanceContract "provider-successful-output" "provider" {
        param($payload, $calls)
        $calls[0].output_count = 1
        $calls[0].observed_output_tool_success = $true
    } $false
    Assert-ProvenanceContract "provider-different-output" "provider" {
        param($payload, $calls)
        $calls[0].output_count = 1
        $calls[0].observed_output_text = "different"
        $calls[0].observed_output_tool_success = $false
    } $false
    Assert-ProvenanceContract "skip-copy-group" "skip" {
        param($payload) $payload.failure_provenance.copy_group_id = "forged"
    } $false
    Assert-ProvenanceContract "skip-zero-dispatch" "skip" {
        param($payload) $payload.failure_provenance.zero_dispatch = $false
    } $false
    Assert-ProvenanceContract "skip-cause" "skip" {
        param($payload)
        $payload.failure_provenance.cause_call_id = "missing"
        $payload.cause.call_id = "missing"
    } $false
    Assert-ProvenanceContract "skip-successful-cause" "skip" {
        param($payload, $calls)
        Set-R7CallOutcome $calls[0] (Get-R7CallOutcome -ToolSuccess $true -Output "ok")
    } $false
    Assert-ProvenanceContract "skip-tool-and-field" "skip" {
        param($payload)
        $payload.tool = "wrong"
        $payload.cause.field = "terminal_call_id"
    } $false
    Assert-ProvenanceContract "bound-copy-group" "bound" {
        param($payload) $payload.failure_provenance.copy_group_id = "forged"
    } $false
    Assert-ProvenanceContract "bound-zero-dispatch" "bound" {
        param($payload) $payload.failure_provenance.zero_dispatch = $true
    } $false
    Assert-ProvenanceContract "bound-reservation" "bound" {
        param($payload) $payload.reservation_id = "forged"
    } $false
    Assert-ProvenanceContract "bound-error-code" "bound" {
        param($payload) $payload.error.code = "forged"
    } $false
    Assert-ProvenanceContract "bound-without-output" "bound" {
        param($payload, $calls) $calls[0].output_count = 0
    } $false

    $duplicateCall = "duplicate-search"
    $duplicate = $valid.Replace($callId, $duplicateCall)
    Assert-StandardSupplementalRejected `
        "duplicate" `
        @($duplicate, $duplicate) `
        "Duplicate structured failure fact for call:*"
    Assert-StandardSupplementalRejected `
        "supplemental-before-output" `
        @($valid.Replace($callId, "supplemental-before-output-search")) `
        "ToolSearch failure precedes its pairing output:*" `
        -OutputAfterMessages

    $taskspacePath = Join-Path $tempRoot "taskspace-incomplete.jsonl"
    Write-Lines $taskspacePath @(
        @{
            type = "event_msg"
            payload = @{
                type = "map_runtime"
                map_event_type = "task_context_event_recorded"
                eventType = "tool_search_call"
                callId = "taskspace-search"
                rawPayload = @{
                    type = "tool_search_call"
                    arguments = @{ query = "read_file" }
                    call_id = "taskspace-search"
                }
            }
        },
        (New-TokenBoundary "taskspace-request"),
        @{
            type = "event_msg"
            payload = @{
                type = "map_runtime"
                map_event_type = "task_context_event_recorded"
                eventType = "tool_search_output"
                callId = "taskspace-search"
                toolSuccess = $true
                rawPayload = @{
                    type = "tool_search_output"
                    call_id = "taskspace-search"
                    status = "completed"
                    execution = "client"
                    tools = @()
                }
            }
        },
        @{
            type = "event_msg"
            payload = @{
                type = "map_runtime"
                map_event_type = "task_context_event_recorded"
                eventType = "message"
                originalRole = "developer"
                rawPayload = @{
                    type = "message"
                    role = "developer"
                    content = @(
                        @{
                            type = "input_text"
                            text = '{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"failure_provenance":{},"error":{"class":"tool","code":"tool_search_failed"}}'
                        }
                    )
                }
            }
        }
    )
    $taskspaceRejected = $false
    try {
        Get-R7TaskspaceRequestPath $taskspacePath 1 | Out-Null
    } catch {
        $taskspaceRejected =
            $_.Exception.Message -like "Incomplete structured failure fact:*"
    }
    if (-not $taskspaceRejected) {
        throw "TaskSpace incomplete supplemental evidence did not fail closed"
    }

    Write-Output "R7 supplemental failure evidence passed."
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -Force -Recurse -LiteralPath $tempRoot
    }
}
