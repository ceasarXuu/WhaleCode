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
            action_manifest_count = $null
            action_manifest_pair_count = $null
            action_manifest_violation_count = $null
            orphan_sibling_count = $null
            declared_action_count = $null
            owned_sibling_count = $null
            initialize_and_execute_pair_count = $null
            execute_pair_count = $null
            reopen_pair_count = $null
            finish_map_count = $null
            finish_map_final_work_count = $null
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
    $finishMapFinalWorkCount = 0
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
            $declaredActions = @()
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
                            [string]$arguments.exact_summary
                        )
                        if ($hasTerminalCandidate) { $terminalCandidateCount++ }
                        $callIdProperty = $payload.PSObject.Properties["call_id"]
                        if ($null -ne $callIdProperty -and
                            -not [string]::IsNullOrWhiteSpace([string]$callIdProperty.Value)) {
                            $finishCallsAwaitingCanonicalRole[[string]$callIdProperty.Value] = $true
                        }
                    }
                    if ($action -in @("initialize_and_execute", "execute", "reopen_map")) {
                        $declaredActions = @(
                            $arguments.actions |
                                ForEach-Object {
                                    [pscustomobject]@{
                                        node_id = [string]$_.node_id
                                        tool = [string]$_.tool
                                    }
                                }
                        )
                    }
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
                    declared_actions = $declaredActions
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
                        if (@($terminalStep[0].completed_work_node_ids).Count -gt 0) {
                            $finishMapFinalWorkCount++
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
    $actionManifests = 0
    $actionManifestPairs = 0
    $actionManifestViolations = 0
    $orphanSiblings = 0
    $declaredActions = 0
    $ownedSiblings = 0
    $initializePairs = 0
    $executePairs = 0
    $reopenPairs = 0
    $manifestActions = @("initialize_and_execute", "execute", "reopen_map")

    foreach ($batch in $batches) {
        $calls = @($batch)
        $controls = @($calls | Where-Object { $_.name -eq "taskspace_control" })
        if ($controls.Count -gt 0) { $controlResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -lt $calls.Count) { $mixedResponses++ }
        if ($controls.Count -gt 1) { $multiControlResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -eq $calls.Count) {
            $standaloneControlResponses++
        }

        $manifestControl = @($controls | Where-Object { $_.action -in $manifestActions })
        if ($manifestControl.Count -gt 0) {
            $actionManifests += $manifestControl.Count
            if ($manifestControl.Count -ne 1 -or $calls[0].name -ne "taskspace_control") {
                $actionManifestViolations++
                continue
            }
            $declared = @($manifestControl[0].declared_actions)
            $siblings = @($calls | Where-Object name -ne "taskspace_control")
            $declaredActions += $declared.Count
            $matched = $declared.Count -eq $siblings.Count
            if ($matched) {
                for ($index = 0; $index -lt $declared.Count; $index++) {
                    if ([string]$declared[$index].tool -ne [string]$siblings[$index].name -or
                        [string]::IsNullOrWhiteSpace([string]$declared[$index].node_id)) {
                        $matched = $false
                        break
                    }
                }
            }
            if ($matched -and $declared.Count -gt 0) {
                $actionManifestPairs++
                $ownedSiblings += $siblings.Count
                switch ([string]$manifestControl[0].action) {
                    "initialize_and_execute" { $initializePairs++ }
                    "execute" { $executePairs++ }
                    "reopen_map" { $reopenPairs++ }
                }
            } else {
                $actionManifestViolations++
                $orphanSiblings += $siblings.Count
            }
        } elseif ($controls.Count -eq 0 -and $calls.Count -gt 0) {
            $orphanSiblings += $calls.Count
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
        action_manifest_count = [int]$actionManifests
        action_manifest_pair_count = [int]$actionManifestPairs
        action_manifest_violation_count = [int]$actionManifestViolations
        orphan_sibling_count = [int]$orphanSiblings
        declared_action_count = [int]$declaredActions
        owned_sibling_count = [int]$ownedSiblings
        initialize_and_execute_pair_count = [int]$initializePairs
        execute_pair_count = [int]$executePairs
        reopen_pair_count = [int]$reopenPairs
        finish_map_count = [int]$finishMapCount
        finish_map_final_work_count = [int]$finishMapFinalWorkCount
        standalone_control_response_count = [int]$standaloneControlResponses
        terminal_candidate_count = [int]$terminalCandidateCount
        terminal_extra_request_count = $terminalExtra
        control_argument_parse_error_count = [int]$controlArgumentParseErrors
    }
}
