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
        }
    }
}

function Assert-StandardSupplementalRejected(
    [string]$Name,
    [string[]]$Messages,
    [string]$ExpectedError
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
    $rows.Add(@{
            type = "response_item"
            payload = @{
                type = "tool_search_output"
                call_id = "$Name-search"
                status = "completed"
                execution = "client"
                tools = @()
            }
        })
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

    Assert-StandardSupplementalRejected `
        "malformed" `
        @('{"schema_version":"ToolSearchFailureV3"') `
        "Malformed structured failure message"
    Assert-StandardSupplementalRejected `
        "malformed-reordered" `
        @('{"padding":0,"schema_version":"ToolSearchFailureV3"') `
        "Malformed structured failure message"
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

    $duplicateCall = "duplicate-search"
    $duplicate = $valid.Replace($callId, $duplicateCall)
    Assert-StandardSupplementalRejected `
        "duplicate" `
        @($duplicate, $duplicate) `
        "Duplicate structured failure fact for call:*"

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
