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
            availability = "missing"; provider_tool_response_count = $null
            control_carrier_response_count = $null; direct_tool_mixed_response_count = $null
            multi_control_carrier_response_count = $null; nested_action_count = $null
            initialize_continuation_count = $null; mutation_continuation_count = $null
            bind_continuation_count = $null; state_only_control_count = $null
            complete_handoff_count = $null; complete_handoff_continuation_count = $null
            finish_map_count = $null; finish_map_last_running_work_count = $null
            finish_map_ready_finish_count = $null; standalone_complete_count = $null
            nonterminal_transition_without_follow_up_count = $null
            terminal_candidate_count = $null
            terminal_extra_request_count = $null
            control_argument_parse_error_count = $null
            continuation_declaration_count = $null
            continuation_satisfied_count = $null
            continuation_violation_count = $null
        }
    }

    $batches = New-Object System.Collections.Generic.List[object]
    $current = New-Object System.Collections.Generic.List[object]
    $rowIndex = 0
    $lastFinishIndex = -1
    $lastFinalIndex = -1
    $terminalCandidateCount = 0
    $nestedActionCount = 0
    $stateOnlyControlCount = 0
    $initializeContinuationCount = 0
    $mutationContinuationCount = 0
    $bindContinuationCount = 0
    $completeHandoffCount = 0
    $completeHandoffContinuationCount = 0
    $finishMapCount = 0
    $finishMapLastWorkCount = 0
    $standaloneCompleteCount = 0
    $finishMapReadyFinishCount = 0
    $controlArgumentParseErrors = 0
    foreach ($line in [System.IO.File]::ReadLines($rolloutPath)) {
        $rowIndex++
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $row = $line | ConvertFrom-Json } catch {
            if ($Events) {
                $Events.Add([pscustomobject]@{
                        event = "cadence_rollout_line_parse_failed"; path = $rolloutPath
                        row = $rowIndex; error = [string]$_.Exception.Message
                    })
            }
            continue
        }
        $payload = Get-TaskspaceCanonicalResponseItem $row
        if ($null -eq $payload) { continue }
        $payloadType = [string]$payload.type
        $isCall = $payloadType -in @("function_call", "custom_tool_call", "local_shell_call")
        if ($isCall) {
            $name = if ($payloadType -eq "local_shell_call") { "local_shell" } else { [string]$payload.name }
            $action = ""
            $transition = ""
            $nestedCount = 0
            $hasContinuation = $false
            $continuationKind = ""
            $hasTerminalCandidate = $false
            if ($name -eq "taskspace_control") {
                try {
                    $arguments = ([string]$payload.arguments) | ConvertFrom-Json
                    $action = [string]$arguments.action
                    $transitionProperty = $arguments.PSObject.Properties["transition"]
                    if ($null -ne $transitionProperty) { $transition = [string]$transitionProperty.Value }
                    $continuationProperty = $arguments.PSObject.Properties["required_next_call"]
                    if ($null -eq $continuationProperty) {
                        # Historical benchmark artifacts used continuation before protocol 1.0.4.
                        $continuationProperty = $arguments.PSObject.Properties["continuation"]
                    }
                    if ($null -ne $continuationProperty) {
                        $hasContinuation = $true
                        $continuation = $continuationProperty.Value
                        if ($continuation -is [string]) {
                            $continuationKind = [string]$continuation
                        } else {
                            $continuationKind = [string]$continuation.kind
                            $actionsProperty = $continuation.PSObject.Properties["actions"]
                            if ($null -ne $actionsProperty) { $nestedCount += @($actionsProperty.Value).Count }
                            if ($continuationKind -eq "patch_then_actions" -and $null -ne $continuation.PSObject.Properties["patch"]) { $nestedCount++ }
                        }
                        if ($action -eq "initialize_map") { $initializeContinuationCount++ }
                        if ($action -eq "mutate_graph") { $mutationContinuationCount++ }
                        if ($action -eq "transition_node" -and $transition -eq "bind") { $bindContinuationCount++ }
                        if ($action -eq "complete_then_continue") { $completeHandoffContinuationCount++ }
                    }
                    if ($action -eq "complete_then_continue") { $completeHandoffCount++ }
                    if ($action -eq "finish_map") {
                        $finishMapCount++
                        $terminalStateProperty = $arguments.PSObject.Properties["terminal_state"]
                        if ($null -ne $terminalStateProperty -and [string]$terminalStateProperty.Value -eq "last_running_work") { $finishMapLastWorkCount++ }
                        if ($null -ne $terminalStateProperty -and [string]$terminalStateProperty.Value -eq "no_active_work_ready_finish") { $finishMapReadyFinishCount++ }
                    }
                    if ($action -eq "transition_node" -and $transition -eq "complete") { $standaloneCompleteCount++ }
                    if ($action -eq "finish_map") {
                        $candidateProperty = $arguments.PSObject.Properties["final_summary"]
                        $hasTerminalCandidate = $null -ne $candidateProperty -and -not [string]::IsNullOrWhiteSpace([string]$candidateProperty.Value)
                    }
                } catch {
                    $controlArgumentParseErrors++
                    if ($Events) {
                        $Events.Add([pscustomobject]@{
                                event = "cadence_control_arguments_parse_failed"
                                path = $rolloutPath; row = $rowIndex
                                error = [string]$_.Exception.Message
                            })
                    }
                }
                $nestedActionCount = [int]$nestedActionCount + [int]$nestedCount
                if (-not $hasContinuation) { $stateOnlyControlCount++ }
                if ($action -eq "finish_map") { $lastFinishIndex = $rowIndex }
                if ($hasTerminalCandidate) { $terminalCandidateCount++ }
            }
                $current.Add([pscustomobject]@{
                    name = $name; action = $action; transition = $transition
                    continuation_kind = $continuationKind
                    nested_action_count = $nestedCount; terminal_candidate = [bool]$hasTerminalCandidate
                })
            continue
        }

        if ($current.Count -gt 0) {
            $batches.Add(@($current.ToArray()))
            $current.Clear()
        }
        if ($payloadType -eq "message" -and [string]$payload.role -eq "assistant") {
            $content = @($payload.content | ForEach-Object { [string]$_.text }) -join ""
            if (-not [string]::IsNullOrWhiteSpace($content)) { $lastFinalIndex = $rowIndex }
        }
    }
    if ($current.Count -gt 0) { $batches.Add(@($current.ToArray())) }

    $carrierResponses = 0
    $directToolMixedResponses = 0
    $multiControlCarrierResponses = 0
    $nonterminalTransitionWithoutFollowUp = 0
    $continuationDeclarations = 0
    $continuationSatisfied = 0
    $continuationViolations = 0
    foreach ($batch in $batches) {
        $calls = @($batch)
        $controls = @($calls | Where-Object { $_.name -eq "taskspace_control" })
        if ($controls.Count -gt 0) { $carrierResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -lt $calls.Count) { $directToolMixedResponses++ }
        if ($controls.Count -gt 1) { $multiControlCarrierResponses++ }
        for ($index = 0; $index -lt $calls.Count; $index++) {
            $continuationKind = [string]$calls[$index].continuation_kind
            if ($continuationKind -in @("ordinary_tool", "apply_patch", "next_tool", "next_apply_patch")) {
                $continuationDeclarations++
                $next = if ($index + 1 -lt $calls.Count) { $calls[$index + 1] } else { $null }
                $satisfied = if ($continuationKind -in @("apply_patch", "next_apply_patch")) {
                    $null -ne $next -and $next.name -eq "apply_patch"
                } else {
                    $null -ne $next -and $next.name -notin @("taskspace_control", "apply_patch")
                }
                if ($satisfied) { $continuationSatisfied++ } else { $continuationViolations++ }
            }
            if ($calls[$index].action -ne "transition_node" -or $calls[$index].transition -eq "bind") { continue }
            if ($index + 1 -ge $calls.Count) {
                $nonterminalTransitionWithoutFollowUp++
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
        availability = if ($controlArgumentParseErrors -gt 0) { "partial_with_parse_errors" } else { "measured" }
        provider_tool_response_count = [int]$batches.Count
        control_carrier_response_count = [int]$carrierResponses
        direct_tool_mixed_response_count = [int]$directToolMixedResponses
        multi_control_carrier_response_count = [int]$multiControlCarrierResponses
        nested_action_count = [int]$nestedActionCount
        initialize_continuation_count = [int]$initializeContinuationCount
        mutation_continuation_count = [int]$mutationContinuationCount
        bind_continuation_count = [int]$bindContinuationCount
        complete_handoff_count = [int]$completeHandoffCount
        complete_handoff_continuation_count = [int]$completeHandoffContinuationCount
        finish_map_count = [int]$finishMapCount
        finish_map_last_running_work_count = [int]$finishMapLastWorkCount
        finish_map_ready_finish_count = [int]$finishMapReadyFinishCount
        standalone_complete_count = [int]$standaloneCompleteCount
        state_only_control_count = [int]$stateOnlyControlCount
        nonterminal_transition_without_follow_up_count = [int]$nonterminalTransitionWithoutFollowUp
        terminal_candidate_count = [int]$terminalCandidateCount
        terminal_extra_request_count = $terminalExtra
        control_argument_parse_error_count = [int]$controlArgumentParseErrors
        continuation_declaration_count = [int]$continuationDeclarations
        continuation_satisfied_count = [int]$continuationSatisfied
        continuation_violation_count = [int]$continuationViolations
    }
}
