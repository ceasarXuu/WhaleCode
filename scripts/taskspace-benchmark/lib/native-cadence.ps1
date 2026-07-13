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
            multi_control_carrier_response_count = $null; multi_finish_carrier_count = $null
            finish_without_sibling_action_count = $null; nested_action_count = $null
            initialize_then_actions_count = $null; finish_nodes_count = $null
            finish_then_end_count = $null; terminal_candidate_count = $null
            terminal_extra_request_count = $null
        }
    }

    $batches = New-Object System.Collections.Generic.List[object]
    $current = New-Object System.Collections.Generic.List[object]
    $rowIndex = 0
    $lastFinishIndex = -1
    $lastFinalIndex = -1
    $terminalCandidateCount = 0
    $nestedActionCount = 0
    $actionCounts = @{ initialize_then_actions = 0; finish_nodes = 0; finish_then_end = 0 }
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
            $nestedCount = 0
            $finishCount = 0
            $hasTerminalCandidate = $false
            if ($name -eq "taskspace_control") {
                try {
                    $arguments = ([string]$payload.arguments) | ConvertFrom-Json
                    $action = [string]$arguments.action
                    if ($actionCounts.ContainsKey($action)) { $actionCounts[$action]++ }
                    $continuationProperty = $arguments.PSObject.Properties["continuation"]
                    if ($null -ne $continuationProperty) {
                        $continuation = $continuationProperty.Value
                        $actionsProperty = $continuation.PSObject.Properties["actions"]
                        if ($null -ne $actionsProperty) { $nestedCount += @($actionsProperty.Value).Count }
                        if ([string]$continuation.kind -eq "patch_then_actions" -and $null -ne $continuation.PSObject.Properties["patch"]) { $nestedCount++ }
                    }
                    $finishesProperty = $arguments.PSObject.Properties["finishes"]
                    if ($null -ne $finishesProperty) { $finishCount = @($finishesProperty.Value).Count }
                    if ($action -eq "finish_then_end") {
                        $chainProperty = $arguments.PSObject.Properties["finish_node_ids"]
                        if ($null -ne $chainProperty) { $finishCount = @($chainProperty.Value).Count }
                        $candidateProperty = $arguments.PSObject.Properties["final_candidate"]
                        $hasTerminalCandidate = $null -ne $candidateProperty -and -not [string]::IsNullOrWhiteSpace([string]$candidateProperty.Value)
                    }
                } catch { }
                $nestedActionCount += $nestedCount
                if ($action -in @("finish_nodes", "finish_then_end")) { $lastFinishIndex = $rowIndex }
                if ($hasTerminalCandidate) { $terminalCandidateCount++ }
            }
            $current.Add([pscustomobject]@{
                    name = $name; action = $action; nested_action_count = $nestedCount
                    finish_count = $finishCount; terminal_candidate = [bool]$hasTerminalCandidate
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
    $multiFinishCarriers = 0
    $finishWithoutSiblingAction = 0
    foreach ($batch in $batches) {
        $calls = @($batch)
        $controls = @($calls | Where-Object { $_.name -eq "taskspace_control" })
        if ($controls.Count -gt 0) { $carrierResponses++ }
        if ($controls.Count -gt 0 -and $controls.Count -lt $calls.Count) { $directToolMixedResponses++ }
        if ($controls.Count -gt 1) { $multiControlCarrierResponses++ }
        $multiFinishCarriers += @($controls | Where-Object { $_.finish_count -gt 1 }).Count
        for ($index = 0; $index -lt $calls.Count; $index++) {
            if ($calls[$index].action -ne "finish_nodes") { continue }
            $following = if ($index + 1 -lt $calls.Count) { @($calls[($index + 1)..($calls.Count - 1)]) } else { @() }
            if (@($following | Where-Object { $_.name -ne "taskspace_control" }).Count -eq 0) {
                $finishWithoutSiblingAction++
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
        availability = "measured"
        provider_tool_response_count = [int]$batches.Count
        control_carrier_response_count = [int]$carrierResponses
        direct_tool_mixed_response_count = [int]$directToolMixedResponses
        multi_control_carrier_response_count = [int]$multiControlCarrierResponses
        multi_finish_carrier_count = [int]$multiFinishCarriers
        finish_without_sibling_action_count = [int]$finishWithoutSiblingAction
        nested_action_count = [int]$nestedActionCount
        initialize_then_actions_count = [int]$actionCounts.initialize_then_actions
        finish_nodes_count = [int]$actionCounts.finish_nodes
        finish_then_end_count = [int]$actionCounts.finish_then_end
        terminal_candidate_count = [int]$terminalCandidateCount
        terminal_extra_request_count = $terminalExtra
    }
}
