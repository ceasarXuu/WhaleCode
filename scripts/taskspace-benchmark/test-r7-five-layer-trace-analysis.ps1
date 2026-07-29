$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-trace-$([guid]::NewGuid().ToString('N'))"
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

try {
    $standardPath = Join-Path $tempRoot "standard.jsonl"
    Write-Lines $standardPath @(
        @{ type = "response_item"; payload = @{ type = "function_call"; name = "exec_command"; arguments = '{"cmd":"pytest"}'; call_id = "s1" } },
        (New-TokenBoundary "standard-1"),
        @{ type = "response_item"; payload = @{ type = "function_call_output"; call_id = "s1"; output = "Execution outcome: exited`nShell exit code: 1" } },
        @{ type = "response_item"; payload = @{ type = "message"; role = "assistant" } },
        (New-TokenBoundary "standard-2")
    )
    $standard = @(Get-R7StandardRequestPath $standardPath 2)
    if ($standard.Count -ne 2 -or $standard[0].calls.Count -ne 1 -or $standard[1].action_kind -ne "assistant_only") {
        throw "Standard request path was not reconstructed"
    }
    if ($standard[0].calls[0].failure_code -ne "shell_exit_1") { throw "Standard failure was not classified" }
    $missingStandardBoundary = Join-Path $tempRoot "standard-missing-boundary.jsonl"
    Write-Lines $missingStandardBoundary @(
        @{ type = "response_item"; payload = @{ type = "message"; role = "assistant" } }
    )
    $standardBoundaryRejected = $false
    try { Get-R7StandardRequestPath $missingStandardBoundary 1 | Out-Null }
    catch { $standardBoundaryRejected = $_.Exception.Message -like "Standard rollout request boundary mismatch:*" }
    if (-not $standardBoundaryRejected) { throw "Missing Standard request boundaries did not fail closed" }

    $standardShapesPath = Join-Path $tempRoot "standard-shapes.jsonl"
    $toolSearchFailure = '{"schema_version":"ToolSearchFailureV2","status":"failed","success":false,"call_id":"search-1","tool":"tool_search","pairing_status":"completed","execution_status":"failed","error":{"class":"tool","cause":{"schema_version":"ProviderToolResponsePreflightV2","failure_provenance":{"scope":"provider_response","copy_group_id":"provider_response:search-1","zero_dispatch":true},"error":{"class":"protocol","code":"provider_tool_declaration_invalid"}}}}'
    Write-Lines $standardShapesPath @(
        @{ type = "response_item"; payload = @{ type = "custom_tool_call"; name = "apply_patch"; input = "*** Begin Patch"; call_id = "custom-1" } },
        @{ type = "response_item"; payload = @{ type = "tool_search_call"; arguments = @{ query = "read_file" }; call_id = "search-1" } },
        @{ type = "response_item"; payload = @{ type = "local_shell_call"; action = @{ command = @("pwd") }; call_id = "local-1"; status = "completed" } },
        (New-TokenBoundary "standard-shapes-1"),
        @{ type = "response_item"; payload = @{ type = "custom_tool_call_output"; call_id = "custom-1"; output = "Done!" } },
        @{ type = "response_item"; payload = @{ type = "tool_search_output"; call_id = "search-1"; status = "completed"; execution = "client"; tools = @() } },
        @{ type = "response_item"; payload = @{ type = "function_call_output"; call_id = "local-1"; output = "Execution outcome: exited`nShell exit code: 0" } },
        @{ type = "response_item"; payload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = $toolSearchFailure }) } }
    )
    $standardShapes = @(Get-R7StandardRequestPath $standardShapesPath 1)
    if ($standardShapes[0].calls.Count -ne 3 -or
        @($standardShapes[0].calls | Where-Object tool -eq "tool_search")[0].success -ne $false -or
        @($standardShapes[0].calls | Where-Object tool -eq "local_shell")[0].success -ne $true) {
        throw "Standard non-function Tool shapes were not reconciled"
    }

    $taskspacePath = Join-Path $tempRoot "taskspace.jsonl"
    $initControlArgs = '{"action":"initialize_and_execute","root":{"node_id":"root","goal":"task"},"work_nodes":[{"node_id":"explore","goal":"inspect"}],"finish":{"node_id":"finish","goal":"summarize"},"edges":[{"from":"root","to":"explore"},{"from":"explore","to":"finish"}],"actions":[{"node_id":"explore","tool":"shell_command"}]}'
    $finishArgs = '{"action":"finish_map","expected_revision":2,"finish_node_id":"finish","complete_work_node_ids":["explore"],"exact_summary":"done"}'
    Write-Lines $taskspacePath @(
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t0"; rawPayload = @{ name = "taskspace_control"; arguments = $initControlArgs } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t1"; rawPayload = @{ name = "shell_command"; arguments = '{"cmd":"ls"}' } } },
        (New-TokenBoundary "wire-1"),
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t0"; toolSuccess = $true; rawPayload = @{ output = '{"schema_version":"TaskSpaceResponseCommitV1","action":"initialize_and_execute","success":true,"state_commit":true}' } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "t1"; toolSuccess = $true; rawPayload = @{ output = "ok" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "message"; originalRole = "developer"; rawPayload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = '{"schema_version":"TaskSpaceResponseFinalReceiptV1"}' }) } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "t2"; rawPayload = @{ name = "taskspace_control"; arguments = $finishArgs } } },
        (New-TokenBoundary "wire-2"),
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
    $stateFailureJson = '{"schema_version":"TaskSpaceResponseCommitFailureV2","state_commit":false,"failure_provenance":{"scope":"provider_response","copy_group_id":"provider_response:copy-control","zero_dispatch":true},"error":{"class":"state_machine","code":"taskspace_response_state_commit_failed","violations":[{"code":"node_state_invalid","subjects":["reservation-1"],"node_id":"join","canonical_state_before_transaction":"waiting","evaluated_state_at_violation":"completed","allowed_states_at_violation":["ready","in_flight"],"canonical_unsatisfied_predecessor_ids_before_transaction":["left"],"evaluated_unsatisfied_predecessor_ids_at_violation":[]}]}}'
    $stateFailure = Get-R7CallOutcome -ToolSuccess $false -Output $stateFailureJson
    if ($stateFailure.failure_class -ne "taskspace_state_machine" -or
        $stateFailure.failure_code -ne "taskspace_response_state_commit_failed") {
        throw "TaskSpace state failure was not classified"
    }
    if (@($stateFailure.violation_codes) -notcontains "node_state_invalid" -or
        [string]$stateFailure.violation_contexts[0].canonical_state_before_transaction -ne "waiting" -or
        [string]$stateFailure.violation_contexts[0].evaluated_state_at_violation -ne "completed") {
        throw "TaskSpace structured state violation was not preserved"
    }
    $copiedRequests = New-R7RequestRows 1
    $copiedRequests[0].rollout_provider_request_id = "copy-request"
    $copiedRequests[0].rollout_provider_logical_request_id = "copy-request-logical"
    $copiedRequests[0].rollout_provider_attempt_seq = 1
    foreach ($callId in @("copy-control", "copy-sibling")) {
        $copy = ConvertTo-R7CallDescriptor -CallId $callId -ToolName "taskspace_control" -Arguments '{"action":"execute"}'
        $copy.success = $false
        $copy.failure_class = $stateFailure.failure_class
        $copy.failure_code = $stateFailure.failure_code
        $copy.failure_schema_version = $stateFailure.failure_schema_version
        $copy.failure_copy_group_id = $stateFailure.failure_copy_group_id
        $copy.zero_dispatch = $stateFailure.zero_dispatch
        $copy.parse_status = $stateFailure.parse_status
        $copy.evidence_valid = $stateFailure.evidence_valid
        $copy.violation_codes = @($stateFailure.violation_codes)
        $copy.violation_contexts = @($stateFailure.violation_contexts)
        $copiedRequests[0].calls.Add($copy)
    }
    $copied = @(Complete-R7RequestRows $copiedRequests)
    if ($copied[0].primary_failure_class -ne "taskspace_state_machine" -or
        $copied[0].sibling_failure_copy_count -ne 1) {
        throw "Sibling failure copies were not collapsed into one request-level primary class"
    }
    $independentRequests = New-R7RequestRows 1
    $independentRequests[0].rollout_provider_request_id = "independent-request"
    $independentRequests[0].rollout_provider_logical_request_id =
        "independent-request-logical"
    $independentRequests[0].rollout_provider_attempt_seq = 1
    foreach ($callId in @("independent-1", "independent-2")) {
        $call = ConvertTo-R7CallDescriptor -CallId $callId -ToolName "exec_command" -Arguments '{"cmd":"false"}'
        Set-R7CallOutcome $call (
            Get-R7CallOutcome -ToolSuccess $false -Output "Execution outcome: exited`nShell exit code: 1"
        )
        $independentRequests[0].calls.Add($call)
    }
    $independent = @(Complete-R7RequestRows $independentRequests)
    if ($independent[0].sibling_failure_copy_count -ne 0) {
        throw "Independent same-code failures were incorrectly collapsed"
    }
    $multiPatchFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":"ToolSequencePreflightResultV3","failure_provenance":{"scope":"provider_response","copy_group_id":"provider_response:patch-1","zero_dispatch":true},"error":{"class":"protocol","code":"request_multiple_apply_patch_calls_not_allowed"}}'
    if ($multiPatchFailure.failure_class -ne "tool_sequence_protocol") { throw "Multi-patch failure was not separated" }
    $initializationFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":"TaskSpaceControlResultV2","action":"initialize_and_execute","success":false,"state_commit":false,"error":{"class":"state_machine","code":"TASKSPACE_INITIAL_GRAPH_INVALID"}}'
    if ($initializationFailure.failure_class -ne "taskspace_state_machine" -or
        $initializationFailure.failure_code -ne "TASKSPACE_INITIAL_GRAPH_INVALID" -or
        $initializationFailure.state_commit -ne $false) {
        throw "Initialization carrier failure was not classified"
    }
    $malformedFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":'
    if ($malformedFailure.evidence_valid -or
        $malformedFailure.failure_class -ne "evidence_unclassified" -or
        $malformedFailure.failure_code -ne "failure_payload_parse_failed") {
        throw "Malformed failure JSON did not fail closed"
    }
    $unknownFailure = Get-R7CallOutcome -ToolSuccess $false -Output '{"schema_version":"UnknownFailureV1","error":{"class":"tool","code":"failed"}}'
    if ($unknownFailure.evidence_valid -or $unknownFailure.failure_code -ne "failure_schema_unknown") {
        throw "Unknown failure schema did not fail closed"
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

    $taskspaceShapesPath = Join-Path $tempRoot "taskspace-shapes.jsonl"
    Write-Lines $taskspaceShapesPath @(
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "custom_tool_call"; callId = "tc-1"; rawPayload = @{ type = "custom_tool_call"; name = "apply_patch"; input = "*** Begin Patch"; call_id = "tc-1" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "tool_search_call"; callId = "ts-1"; rawPayload = @{ type = "tool_search_call"; arguments = @{ query = "read_file" }; call_id = "ts-1" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "local_shell_call"; callId = "tl-1"; rawPayload = @{ type = "local_shell_call"; action = @{ command = @("pwd") }; call_id = "tl-1"; status = "completed" } } },
        (New-TokenBoundary "taskspace-shapes-1"),
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "custom_tool_call_output"; callId = "tc-1"; toolSuccess = $true; rawPayload = @{ type = "custom_tool_call_output"; call_id = "tc-1"; output = "Done!" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "tool_search_output"; callId = "ts-1"; toolSuccess = $true; rawPayload = @{ type = "tool_search_output"; call_id = "ts-1"; status = "completed"; execution = "client"; tools = @() } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "tl-1"; toolSuccess = $true; rawPayload = @{ type = "function_call_output"; call_id = "tl-1"; output = "Execution outcome: exited`nShell exit code: 0" } } },
        @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "message"; rawPayload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = ($toolSearchFailure -replace 'search-1', 'ts-1') }) } } }
    )
    $taskspaceShapes = @(Get-R7TaskspaceRequestPath $taskspaceShapesPath 1)
    if ($taskspaceShapes[0].calls.Count -ne 3 -or
        @($taskspaceShapes[0].calls | Where-Object tool -eq "tool_search")[0].success -ne $false) {
        throw "TaskSpace non-function Tool shapes were not reconciled"
    }

    $wirePath = Join-Path $tempRoot "wire.jsonl"
    $receiptHash = ("a" * 64) -join ""
    Write-Lines $wirePath @(
        @{ schema_version = "provider-chat-wire-trace-v9"; event_name = "provider.chat_wire_shape_recorded"; request_id = "wire-1"; logical_request_id = "wire-1-logical"; attempt_seq = 1; request_index = 1; provider_wire_api = "ChatCompletions"; lcp_message_count = 0; message_shapes = @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }); taskspace_final_receipt_identity = @{ count = 0; receipts = @() }; section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 20 }) } },
        @{ schema_version = "provider-chat-wire-trace-v9"; event_name = "provider.chat_wire_request_terminal"; request_id = "wire-1"; logical_request_id = "wire-1-logical"; attempt_seq = 1; status = "response_completed"; input_tokens = 100; cached_input_tokens = 0 },
        @{ schema_version = "provider-chat-wire-trace-v9"; event_name = "provider.chat_wire_prefix_broken"; request_id = "wire-2"; logical_request_id = "wire-2-logical"; attempt_seq = 1; request_index = 2; provider_wire_api = "ChatCompletions"; lcp_message_count = 2; message_shapes = @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }, @{ index = 2; role = "assistant" }, @{ index = 3; role = "tool" }, @{ index = 4; role = "system" }); taskspace_final_receipt_identity = @{ count = 1; receipts = @(@{ message_index = 4; wire_role = "system"; control_call_id_sha256 = $receiptHash; reservation_revision_after = 1; canonical_revision = 2; revision_delta = 1; complete = $true }) }; section_cost = @{ sections = @(@{ kind = "tools"; estimated_tokens = 100 }, @{ kind = "active_projection"; estimated_tokens = 40 }) } },
        @{ schema_version = "provider-chat-wire-trace-v9"; event_name = "provider.chat_wire_request_terminal"; request_id = "wire-2"; logical_request_id = "wire-2-logical"; attempt_seq = 1; status = "response_completed"; input_tokens = 200; cached_input_tokens = 20 }
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

    $identityMismatch = @(
        ($taskspace | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100)
    )
    $identityMismatch[0].rollout_provider_request_id = "wire-missing"
    $identityRejected = $false
    try { Add-R7WireFactsToRequestPath $identityMismatch $wirePath | Out-Null }
    catch { $identityRejected = $_.Exception.Message -like "Rollout provider request identity is absent*" }
    if (-not $identityRejected) {
        throw "Rollout/wire identity mismatch did not fail closed"
    }

    $badReceiptPath = Join-Path $tempRoot "wire-bad-receipt.jsonl"
    $badReceiptRows = @(
        Get-Content -Encoding UTF8 -LiteralPath $wirePath |
            ForEach-Object { $_ | ConvertFrom-Json -Depth 100 }
    )
    $badReceiptShape = @(
        $badReceiptRows |
            Where-Object {
                [string]$_.request_id -eq "wire-2" -and
                $null -ne $_.request_index
            }
    )[0]
    $badReceiptShape.taskspace_final_receipt_identity.receipts[0].complete = $false
    Write-Lines $badReceiptPath $badReceiptRows
    $receiptRejected = $false
    try { Add-R7WireFactsToRequestPath $taskspace $badReceiptPath | Out-Null }
    catch { $receiptRejected = $_.Exception.Message -like "Provider response-final receipt identity is incomplete*" }
    if (-not $receiptRejected) {
        throw "Incomplete response-final receipt did not fail closed"
    }
    Write-Output "R7 five-layer trace analysis passed."
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -Force -Recurse -LiteralPath $tempRoot
    }
}
