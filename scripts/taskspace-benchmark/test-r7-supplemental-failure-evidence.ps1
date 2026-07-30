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
    try {
        Get-R7StandardRequestPath $path 1 | Out-Null
    } catch {
        $rejected = $_.Exception.Message -like $ExpectedError
    }
    if (-not $rejected) {
        throw "$Name supplemental evidence did not fail closed"
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
        "Malformed structured developer message"
    Assert-StandardSupplementalRejected `
        "unknown" `
        @('{"schema_version":"ToolSearchFailureV4","status":"failed","success":false}') `
        "Unknown structured failure schema:*"
    Assert-StandardSupplementalRejected `
        "incomplete" `
        @('{"schema_version":"ToolSearchFailureV3","status":"failed","success":false,"failure_provenance":{},"error":{"class":"tool","code":"tool_search_failed"}}') `
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
