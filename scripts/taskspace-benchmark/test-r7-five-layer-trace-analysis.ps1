$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-trace-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

function Write-Lines([string]$Path, [object[]]$Rows) {
    $lines = @($Rows | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 30 })
    [IO.File]::WriteAllLines($Path, $lines, [Text.UTF8Encoding]::new($false))
}

try {
    $standardPath = Join-Path $tempRoot "standard.jsonl"
    Write-Lines $standardPath @(
        @{ type = "response_item"; payload = @{ type = "function_call"; name = "exec_command"; arguments = '{"cmd":"pytest"}'; call_id = "s1" } },
        @{ type = "event_msg"; payload = @{ type = "token_count" } },
        @{ type = "response_item"; payload = @{ type = "function_call_output"; call_id = "s1"; output = "Execution outcome: exited`nShell exit code: 1" } },
        @{ type = "response_item"; payload = @{ type = "message"; role = "assistant" } },
        @{ type = "event_msg"; payload = @{ type = "token_count" } }
    )
    $standard = @(Get-R7StandardRequestPath $standardPath 2)
    if ($standard.Count -ne 2 -or $standard[0].calls.Count -ne 1 -or $standard[1].action_kind -ne "assistant_only") {
        throw "Standard request path was not reconstructed"
    }
    if ($standard[0].calls[0].failure_code -ne "shell_exit_1") { throw "Standard failure was not classified" }

    $taskspacePath = Join-Path $tempRoot "taskspace.jsonl"
    $initArgs = '{"cmd":"ls","taskspace_action":{"action":"initialize_map","initial_work_node":{"node_id":"explore"}}}'
    $finishArgs = '{"action":"finish_map","terminal_node_id":"verify"}'
    Write-Lines $taskspacePath @(
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t1"; rawPayload = @{ name = "exec_command"; arguments = $initArgs } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t1"; toolSuccess = $true; rawPayload = @{ output = "ok" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "taskspace_trace_event_recorded"; kind = "provider_response_actionability"; tags = @("request_count:1") } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t2"; rawPayload = @{ name = "taskspace_control"; arguments = $finishArgs } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t2"; toolSuccess = $true; rawPayload = @{ output = '{"action":"finish_map","success":true,"state_commit":true}' } } },
        @{ type = "event_msg"; payload = @{ type = "task_complete" } }
    )
    $taskspace = @(Get-R7TaskspaceRequestPath $taskspacePath 2)
    if ($taskspace[0].calls[0].taskspace_action -ne "initialize_map") { throw "TaskSpace init carrier was not parsed" }
    if ($taskspace[1].calls[0].taskspace_action -ne "finish_map") { throw "TaskSpace terminal request was not flushed" }
    $stateFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"action_result":{"state_commit":false,"error":{"class":"state_machine","code":"TASKSPACE_LIFECYCLE_INVARIANT"}}}'
    if ($stateFailure.failure_class -ne "taskspace_state_machine" -or $stateFailure.failure_code -ne "TASKSPACE_LIFECYCLE_INVARIANT") {
        throw "TaskSpace state failure was not classified"
    }
    $multiPatchFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"error":{"class":"protocol","code":"request_multiple_apply_patch_calls_not_allowed"}}'
    if ($multiPatchFailure.failure_class -ne "tool_sequence_protocol") { throw "Multi-patch failure was not separated" }

    $wirePath = Join-Path $tempRoot "wire.jsonl"
    Write-Lines $wirePath @(
        @{ section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 20 }) } },
        @{ section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 40 }) } }
    )
    $sections = Get-R7WireSectionSummary $wirePath
    if ($sections.request_count -ne 2 -or $sections.estimated_tokens_total.tools -ne 200 -or $sections.estimated_tokens_mean.tools -ne 100 -or $sections.estimated_tokens_mean.active_projection -ne 30) {
        throw "Wire section means were not calculated"
    }
    Write-Output "R7 five-layer trace analysis passed."
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        $backupRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-trace-complete"
        New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null
        Move-Item -Force -LiteralPath $tempRoot -Destination (Join-Path $backupRoot ([IO.Path]::GetFileName($tempRoot)))
    }
}
