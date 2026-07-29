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
    $initControlArgs = '{"action":"initialize_and_execute","root":{"node_id":"root","goal":"task"},"work_nodes":[{"node_id":"explore","goal":"inspect"}],"finish":{"node_id":"finish","goal":"summarize"},"edges":[{"from":"root","to":"explore"},{"from":"explore","to":"finish"}],"actions":[{"node_id":"explore","tool":"shell_command"}]}'
    $finishArgs = '{"action":"finish_map","expected_revision":2,"finish_node_id":"finish","complete_work_node_ids":["explore"],"exact_summary":"done"}'
    Write-Lines $taskspacePath @(
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t0"; rawPayload = @{ name = "taskspace_control"; arguments = $initControlArgs } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t1"; rawPayload = @{ name = "shell_command"; arguments = '{"cmd":"ls"}' } } },
        @{ type = "event_msg"; payload = @{ type = "token_count" } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t0"; toolSuccess = $true; rawPayload = @{ output = '{"schema_version":"TaskSpaceResponseCommitV1","action":"initialize_and_execute","success":true,"state_commit":true}' } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t1"; toolSuccess = $true; rawPayload = @{ output = "ok" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "message"; originalRole = "developer"; rawPayload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = '{"schema_version":"TaskSpaceResponseFinalReceiptV1"}' }) } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t2"; rawPayload = @{ name = "taskspace_control"; arguments = $finishArgs } } },
        @{ type = "event_msg"; payload = @{ type = "token_count" } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t2"; toolSuccess = $true; rawPayload = @{ output = '{"schema_version":"TaskSpaceControlResultV2","action":"finish_map","success":true,"state_commit":true}' } } },
        @{ type = "event_msg"; payload = @{ type = "task_complete" } }
    )
    $taskspace = @(Get-R7TaskspaceRequestPath $taskspacePath 2)
    if ($taskspace[0].calls[0].control_action -ne "initialize_and_execute") { throw "TaskSpace initialization manifest was not parsed" }
    if ($taskspace[0].calls[1].declared_node_id -ne "explore") { throw "TaskSpace sibling ownership was not reconstructed" }
    if ($taskspace[1].calls[0].control_action -ne "finish_map") { throw "TaskSpace terminal request was not flushed" }
    if (-not $taskspace[1].receipt_before -or $taskspace[1].receipt_original_role -ne "developer") {
        throw "TaskSpace response-final receipt was not assigned to the following provider request"
    }
    $stateFailureJson = '{"state_commit":false,"error":{"class":"state_machine","code":"taskspace_response_state_commit_failed","violations":[{"code":"node_state_invalid","subjects":["reservation-1"],"node_id":"join","actual_state":"waiting","allowed_states":["ready","in_flight"],"unsatisfied_predecessor_ids":["left"]}]}}'
    $stateFailure = Get-R7CallOutcome -ToolSuccess $false -Output $stateFailureJson
    if ($stateFailure.failure_class -ne "taskspace_state_machine" -or
        $stateFailure.failure_code -ne "taskspace_response_state_commit_failed") {
        throw "TaskSpace state failure was not classified"
    }
    if (@($stateFailure.violation_codes) -notcontains "node_state_invalid" -or
        [string]$stateFailure.violation_contexts[0].actual_state -ne "waiting") {
        throw "TaskSpace structured state violation was not preserved"
    }
    $copiedRequests = New-R7RequestRows 1
    foreach ($callId in @("copy-control", "copy-sibling")) {
        $copy = ConvertTo-R7CallDescriptor -CallId $callId -ToolName "taskspace_control" -Arguments '{"action":"execute"}'
        $copy.success = $false
        $copy.failure_class = $stateFailure.failure_class
        $copy.failure_code = $stateFailure.failure_code
        $copy.violation_codes = @($stateFailure.violation_codes)
        $copy.violation_contexts = @($stateFailure.violation_contexts)
        $copiedRequests[0].calls.Add($copy)
    }
    $copied = @(Complete-R7RequestRows $copiedRequests)
    if ($copied[0].primary_failure_class -ne "taskspace_state_machine" -or
        $copied[0].sibling_failure_copy_count -ne 1) {
        throw "Sibling failure copies were not collapsed into one request-level primary class"
    }
    $multiPatchFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":"ToolSequencePreflightResultV2","error":{"class":"protocol","code":"request_multiple_apply_patch_calls_not_allowed"}}'
    if ($multiPatchFailure.failure_class -ne "tool_sequence_protocol") { throw "Multi-patch failure was not separated" }
    $initializationFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":"TaskSpaceControlResultV2","action":"initialize_and_execute","success":false,"state_commit":false,"error":{"class":"state_machine","code":"TASKSPACE_INITIAL_GRAPH_INVALID"}}'
    if ($initializationFailure.failure_class -ne "taskspace_state_machine" -or
        $initializationFailure.failure_code -ne "TASKSPACE_INITIAL_GRAPH_INVALID" -or
        $initializationFailure.state_commit -ne $false) {
        throw "Initialization carrier failure was not classified"
    }
    $missingBoundaryPath = Join-Path $tempRoot "taskspace-missing-boundary.jsonl"
    Write-Lines $missingBoundaryPath @(
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "m1"; rawPayload = @{ name = "exec_command"; arguments = '{"cmd":"ls"}' } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "m1"; toolSuccess = $true; rawPayload = @{ output = "ok" } } }
    )
    $missingBoundaryRejected = $false
    try { Get-R7TaskspaceRequestPath $missingBoundaryPath 1 | Out-Null }
    catch { $missingBoundaryRejected = $_.Exception.Message -like "TaskSpace rollout request boundary mismatch:*" }
    if (-not $missingBoundaryRejected) { throw "Missing TaskSpace request boundaries did not fail closed" }

    $wirePath = Join-Path $tempRoot "wire.jsonl"
    Write-Lines $wirePath @(
        @{ event_name = "provider.chat_wire_shape_recorded"; request_id = "wire-1"; request_index = 1; provider_wire_api = "ChatCompletions"; lcp_message_count = 0; message_shapes = @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }); section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 20 }) } },
        @{ event_name = "provider.chat_wire_request_terminal"; request_id = "wire-1"; input_tokens = 100; cached_input_tokens = 0 },
        @{ event_name = "provider.chat_wire_prefix_broken"; request_id = "wire-2"; request_index = 2; provider_wire_api = "ChatCompletions"; lcp_message_count = 2; message_shapes = @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }, @{ index = 2; role = "assistant" }, @{ index = 3; role = "tool" }, @{ index = 4; role = "system" }); section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 40 }) } },
        @{ event_name = "provider.chat_wire_request_terminal"; request_id = "wire-2"; input_tokens = 200; cached_input_tokens = 20 }
    )
    $sections = Get-R7WireSectionSummary $wirePath
    if ($sections.request_count -ne 2 -or $sections.estimated_tokens_total.tools -ne 200 -or $sections.estimated_tokens_mean.tools -ne 100 -or $sections.estimated_tokens_mean.active_projection -ne 30) {
        throw "Wire section means were not calculated"
    }
    $taskspace = @(Add-R7WireFactsToRequestPath $taskspace $wirePath)
    $requestSummary = Get-R7RequestObservabilitySummary $taskspace
    if (-not $requestSummary.classification_reconciled -or
        $requestSummary.receipt_before_requests -ne 1 -or
        $requestSummary.receipt_before_cache_hit_rate -ne 0.1 -or
        $taskspace[1].receipt_wire_role -ne "system") {
        throw "Request taxonomy or receipt/cache attribution did not reconcile"
    }
    Write-Output "R7 five-layer trace analysis passed."
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -Force -Recurse -LiteralPath $tempRoot
    }
}
