Set-StrictMode -Version Latest
if (-not (Get-Command Get-TaskspaceCanonicalResponseItem -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "canonical-rollout.ps1")
}

function Get-TaskspaceNativeCadenceFacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [System.Collections.Generic.List[object]]$Events
    )
    $rolloutPath = Join-Path $ArtifactDir "rollout.jsonl"
    if (-not (Test-Path -LiteralPath $rolloutPath -PathType Leaf)) {
        return [pscustomobject]@{
            availability = "missing"
            provider_tool_response_count = $null
            control_response_count = $null
            mixed_control_action_response_count = $null
            multi_control_response_count = $null
            boundary_action_count = $null
            boundary_pair_count = $null
            boundary_violation_count = $null
            orphan_after_boundary_count = $null
            ordinary_binding_count = $null
            active_binding_count = $null
            after_boundary_binding_count = $null
            initialize_pair_count = $null
            bind_pair_count = $null
            complete_handoff_count = $null
            complete_handoff_pair_count = $null
            finish_map_count = $null
            finish_map_last_running_work_count = $null
            finish_map_ready_finish_count = $null
            standalone_control_response_count = $null
            terminal_candidate_count = $null
            terminal_extra_request_count = $null
            control_argument_parse_error_count = $null
        }
    }

    $batches = [Collections.Generic.List[object]]::new()
    $current = [Collections.Generic.List[object]]::new()
    $rowIndex = 0
    $lastFinishIndex = -1
    $lastFinalIndex = -1
    $terminalCandidateCount = 0
    $finishMapCount = 0
    $finishMapLastWorkCount = 0
    $finishMapReadyFinishCount = 0
    $controlArgumentParseErrors = 0
    $finishCallsAwaitingCanonicalRole = @{}

    foreach ($line in [IO.File]::ReadLines($rolloutPath)) {
        $rowIndex++
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $row = $line | ConvertFrom-Json
        } catch {
            if ($Events) {
                $Events.Add([pscustomobject]@{
                        event = "cadence_rollout_line_parse_failed"
                        path = $rolloutPath
                        row = $rowIndex
                        error = [string]$_.Exception.Message
                    })
            }
            continue
        }
        $payload = Get-TaskspaceCanonicalResponseItem $row
        if ($null -eq $payload) { continue }
        $payloadType = [string]$payload.type
        $isCall = $payloadType -in @(
            "function_call",
            "custom_tool_call",
            "local_shell_call",
            "tool_search_call",
            "mcp_tool_call"
        )
        if ($isCall) {
            $nameProperty = $payload.PSObject.Properties["name"]
            $name = if ($payloadType -eq "local_shell_call") {
                "local_shell"
            } elseif ($payloadType -eq "tool_search_call") {
                "tool_search"
            } elseif ($payloadType -eq "mcp_tool_call") {
                "mcp"
            } elseif ($null -ne $nameProperty) {
                [string]$nameProperty.Value
            } else {
                ""
            }
            $action = ""
            $binding = ""
            $hasTerminalCandidate = $false
            try {
                $argumentsProperty = $payload.PSObject.Properties["arguments"]
                $argumentsValue = if ($null -ne $argumentsProperty) {
                    $argumentsProperty.Value
                } else {
                    $null
                }
                $arguments = if ($argumentsValue -is [string]) {
                    ([string]$argumentsValue) | ConvertFrom-Json
                } else {
                    $argumentsValue
                }
                if ($name -eq "taskspace_control") {
                    $action = [string]$arguments.action
                    if ($action -eq "finish_map") {
                        $finishMapCount++
                        $lastFinishIndex = $rowIndex
                        $hasTerminalCandidate = -not [string]::IsNullOrWhiteSpace(
                            [string]$arguments.final_summary
                        )
                        if ($hasTerminalCandidate) { $terminalCandidateCount++ }
                        $callIdProperty = $payload.PSObject.Properties["call_id"]
                        if ($null -ne $callIdProperty -and
                            -not [string]::IsNullOrWhiteSpace([string]$callIdProperty.Value)) {
                            $finishCallsAwaitingCanonicalRole[[string]$callIdProperty.Value] = $true
                        }
                    }
                } else {
                    $binding = [string]$arguments.taskspace_binding
                }
            } catch {
                if ($name -eq "taskspace_control") {
                    $controlArgumentParseErrors++
                    if ($Events) {
                        $Events.Add([pscustomobject]@{
                                event = "cadence_control_arguments_parse_failed"
                                path = $rolloutPath
                                row = $rowIndex
                                error = [string]$_.Exception.Message
                            })
                    }
                }
            }
            $current.Add([pscustomobject]@{
                    name = $name
                    action = $action
                    taskspace_binding = $binding
                    terminal_candidate = [bool]$hasTerminalCandidate
                })
            continue
        }

        if ($payloadType -eq "function_call_output") {
            $outputCallIdProperty = $payload.PSObject.Properties["call_id"]
            $outputCallId = if ($null -ne $outputCallIdProperty) {
                [string]$outputCallIdProperty.Value
            } else {
                ""
            }
            if ($finishCallsAwaitingCanonicalRole.ContainsKey($outputCallId)) {
                try {
                    $controlResult = ([string]$payload.output) | ConvertFrom-Json
                    $terminalStep = @(
                        $controlResult.steps |
                            Where-Object { [string]$_.kind -eq "finish_map" } |
                            Select-Object -First 1
                    )
                    if ($terminalStep.Count -gt 0) {
                        if ([string]$terminalStep[0].terminal_node_role -eq "work") {
                            $finishMapLastWorkCount++
                        }
                        if ([string]$terminalStep[0].terminal_node_role -eq "finish") {
                            $finishMapReadyFinishCount++
                        }
                    }
                } catch {
                    if ($Events) {
                        $Events.Add([pscustomobject]@{
                                event = "cadence_finish_result_parse_failed"
                                path = $rolloutPath
                                row = $rowIndex
                                call_id = $outputCallId
                                error = [string]$_.Exception.Message
                            })
                    }
                }
                $finishCallsAwaitingCanonicalRole.Remove($outputCallId)
            }
        }

        $isCallOutput = $payloadType -in @(
            "function_call_output",
            "custom_tool_call_output",
            "local_shell_call_output",
            "tool_search_output",
            "tool_search_call_output",
            "mcp_tool_call_output"
        )
        if ($isCallOutput -and $current.Count -gt 0) {
            $batches.Add(@($current.ToArray()))
            $current.Clear()
        }
        if ($payloadType -eq "message" -and [string]$payload.role -eq "assistant") {
            $content = @($payload.content | ForEach-Object { [string]$_.text }) -join ""
            if (-not [string]::IsNullOrWhiteSpace($content)) { $lastFinalIndex = $rowIndex }
        }
    }
    if ($current.Count -gt 0) { $batches.Add(@($current.ToArray())) }

    $controlResponses = 0
    $mixedResponses = 0
    $multiControlResponses = 0
    $standaloneControlResponses = 0
    $boundaryActions = 0
    $boundaryPairs = 0
    $boundaryViolations = 0
    $orphanAfterBoundary = 0
    $ordinaryBindings = 0
    $activeBindings = 0
    $afterBoundaryBindings = 0
    $initializePairs = 0
    $bindPairs = 0
    $completeHandoffs = 0
    $completePairs = 0
    $boundaryNames = @("initialize_map", "bind_node", "complete_then_continue")

    foreach ($batch in $batches) {
        $calls = @($batch)
        $controls = @($calls | Where-Object { $_.name -eq "taskspace_control" })
        if ($controls.Count -gt 0) { $controlResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -lt $calls.Count) { $mixedResponses++ }
        if ($controls.Count -gt 1) { $multiControlResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -eq $calls.Count) {
            $standaloneControlResponses++
        }

        for ($index = 0; $index -lt $calls.Count; $index++) {
            $call = $calls[$index]
            if ($call.name -ne "taskspace_control" -and $call.taskspace_binding) {
                $ordinaryBindings++
                if ($call.taskspace_binding -eq "active") { $activeBindings++ }
                if ($call.taskspace_binding -eq "after_boundary") { $afterBoundaryBindings++ }
            }
            if ($call.name -eq "taskspace_control" -and $call.action -eq "complete_then_continue") {
                $completeHandoffs++
            }
            if ($call.name -eq "taskspace_control" -and $call.action -in $boundaryNames) {
                $boundaryActions++
                $next = if ($index + 1 -lt $calls.Count) { $calls[$index + 1] } else { $null }
                $paired = $null -ne $next -and
                    $next.name -ne "taskspace_control" -and
                    $next.taskspace_binding -eq "after_boundary"
                if ($paired) {
                    $boundaryPairs++
                    if ($call.action -eq "initialize_map") { $initializePairs++ }
                    if ($call.action -eq "bind_node") { $bindPairs++ }
                    if ($call.action -eq "complete_then_continue") { $completePairs++ }
                } else {
                    $boundaryViolations++
                }
            }
            if ($call.name -ne "taskspace_control" -and $call.taskspace_binding -eq "after_boundary") {
                $previous = if ($index -gt 0) { $calls[$index - 1] } else { $null }
                $paired = $null -ne $previous -and
                    $previous.name -eq "taskspace_control" -and
                    $previous.action -in $boundaryNames
                if (-not $paired) { $orphanAfterBoundary++ }
            }
        }
    }

    $terminalExtra = if ($terminalCandidateCount -gt 0) {
        0
    } elseif ($lastFinishIndex -ge 0 -and $lastFinalIndex -gt $lastFinishIndex) {
        1
    } else {
        $null
    }
    [pscustomobject]@{
        availability = if ($controlArgumentParseErrors -gt 0) {
            "partial_with_parse_errors"
        } else {
            "measured"
        }
        provider_tool_response_count = [int]$batches.Count
        control_response_count = [int]$controlResponses
        mixed_control_action_response_count = [int]$mixedResponses
        multi_control_response_count = [int]$multiControlResponses
        boundary_action_count = [int]$boundaryActions
        boundary_pair_count = [int]$boundaryPairs
        boundary_violation_count = [int]$boundaryViolations
        orphan_after_boundary_count = [int]$orphanAfterBoundary
        ordinary_binding_count = [int]$ordinaryBindings
        active_binding_count = [int]$activeBindings
        after_boundary_binding_count = [int]$afterBoundaryBindings
        initialize_pair_count = [int]$initializePairs
        bind_pair_count = [int]$bindPairs
        complete_handoff_count = [int]$completeHandoffs
        complete_handoff_pair_count = [int]$completePairs
        finish_map_count = [int]$finishMapCount
        finish_map_last_running_work_count = [int]$finishMapLastWorkCount
        finish_map_ready_finish_count = [int]$finishMapReadyFinishCount
        standalone_control_response_count = [int]$standaloneControlResponses
        terminal_candidate_count = [int]$terminalCandidateCount
        terminal_extra_request_count = $terminalExtra
        control_argument_parse_error_count = [int]$controlArgumentParseErrors
    }
}
