function Get-R7JsonProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

function ConvertTo-R7CallDescriptor {
    param(
        [Parameter(Mandatory = $true)][string]$CallId,
        [Parameter(Mandatory = $true)][string]$ToolName,
        [Parameter(Mandatory = $true)][string]$Arguments
    )
    $parsed = $null
    try { $parsed = $Arguments | ConvertFrom-Json -Depth 100 } catch {}
    $controlAction = if ($ToolName -eq "taskspace_control") {
        [string](Get-R7JsonProperty $parsed "action" "")
    } else {
        ""
    }
    $bindingValue = Get-R7JsonProperty $parsed "taskspace_binding"
    $taskspaceBinding = if ($ToolName -eq "taskspace_control") {
        ""
    } elseif ($bindingValue -is [string]) {
        [string]$bindingValue
    } elseif ([string](Get-R7JsonProperty $bindingValue "action" "") -eq "initialize_map") {
        "initialize_map"
    } else {
        ""
    }
    $currentNode = [string](Get-R7JsonProperty $parsed "current_node_id" "")
    $nextNode = [string](Get-R7JsonProperty $parsed "next_node_id" "")
    $initialization = if ($taskspaceBinding -eq "initialize_map") { $bindingValue } else { $parsed }
    $initialWork = Get-R7JsonProperty $initialization "initial_work_node"
    $initialNode = [string](Get-R7JsonProperty $initialWork "node_id" "")
    $terminalNode = [string](Get-R7JsonProperty $parsed "terminal_node_id" "")
    $node = if ($currentNode -and $nextNode) {
        "$currentNode->$nextNode"
    } elseif ($currentNode) {
        $currentNode
    } elseif ($nextNode) {
        $nextNode
    } elseif ($initialNode) {
        $initialNode
    } else {
        $terminalNode
    }
    $detail = ""
    if ($ToolName -eq "exec_command") {
        $detail = [string](Get-R7JsonProperty $parsed "cmd" "")
        $detail = ($detail -replace '[\r\n]+', ' ').Trim()
        if ($detail.Length -gt 120) { $detail = $detail.Substring(0, 120) }
    } elseif ($ToolName -eq "apply_patch") {
        $patchText = [string](Get-R7JsonProperty $parsed "input" "")
        $fileCount = @([regex]::Matches($patchText, '(?m)^\*\*\* (?:Add|Update|Delete) File:')).Count
        $detail = "patch_files=$fileCount"
    }
    [pscustomobject]@{
        call_id = $CallId
        tool = $ToolName
        control_action = $controlAction
        taskspace_binding = $taskspaceBinding
        node = $node
        detail = $detail
        success = $null
        failure_class = ""
        failure_code = ""
        state_commit = $null
    }
}

function Get-R7CallOutcome {
    param([bool]$ToolSuccess, [string]$Output)
    $failureClass = ""
    $failureCode = ""
    $stateCommit = $null
    $firstLine = @($Output -split "`r?`n", 2)[0]
    $payload = $null
    if ($firstLine.StartsWith("{")) {
        try { $payload = $firstLine | ConvertFrom-Json -Depth 100 } catch {}
    }
    if ($null -ne $payload) {
        $stateCommitValue = Get-R7JsonProperty $payload "state_commit"
        if ($null -ne $stateCommitValue) { $stateCommit = [bool]$stateCommitValue }
    }
    if (-not $ToolSuccess) {
        $schemaVersion = [string](Get-R7JsonProperty $payload "schema_version" "")
        $error = Get-R7JsonProperty $payload "error"
        $errorClass = [string](Get-R7JsonProperty $error "class" "")
        $errorCode = [string](Get-R7JsonProperty $error "code" "")
        if ($schemaVersion -eq "ToolSequencePreflightResultV1") {
            $failureClass = "tool_sequence_protocol"
            $failureCode = $errorCode
        } elseif ($errorCode) {
            $failureClass = if ($errorClass) { "taskspace_$errorClass" } else { "taskspace" }
            $failureCode = $errorCode
        } elseif ($Output -match 'apply_patch verification failed') {
            $failureClass = "ordinary_tool"
            $failureCode = "apply_patch_verification_failed"
        } elseif ($Output -match 'Shell exit code: ([0-9]+)') {
            $failureClass = "ordinary_tool"
            $failureCode = "shell_exit_$($Matches[1])"
        } else {
            $failureClass = "ordinary_tool"
            $failureCode = "tool_failed_unclassified"
        }
    }
    [pscustomobject]@{
        success = $ToolSuccess
        failure_class = $failureClass
        failure_code = $failureCode
        state_commit = $stateCommit
    }
}

function Set-R7CallOutcome {
    param([Collections.Generic.List[object]]$Calls, [string]$CallId, $Outcome)
    foreach ($call in $Calls) {
        if ([string]$call.call_id -ne $CallId) { continue }
        $call.success = [bool]$Outcome.success
        $call.failure_class = [string]$Outcome.failure_class
        $call.failure_code = [string]$Outcome.failure_code
        $call.state_commit = $Outcome.state_commit
        return
    }
}

function New-R7RequestRows {
    param([int]$ProviderRequests)
    $requests = [Collections.Generic.List[object]]::new()
    foreach ($index in 1..$ProviderRequests) {
        $requests.Add([pscustomobject]@{
                request_index = $index
                calls = [Collections.Generic.List[object]]::new()
            })
    }
    $requests
}

function Complete-R7RequestRows {
    param([Collections.Generic.List[object]]$Requests)
    foreach ($request in $Requests) {
        $calls = @($request.calls)
        $request.calls = $calls
        $request | Add-Member -NotePropertyName action_kind -NotePropertyValue $(if ($calls.Count) { "tool_calls" } else { "assistant_only" })
        $request | Add-Member -NotePropertyName failure_codes -NotePropertyValue @($calls | Where-Object failure_code | ForEach-Object failure_code)
    }
    @($Requests)
}

function Get-R7TaskspaceRequestPath {
    param([string]$RolloutPath, [int]$ProviderRequests)
    $requests = New-R7RequestRows $ProviderRequests
    $buffer = [Collections.Generic.List[object]]::new()
    $lastCommittedRequest = 0
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $RolloutPath) {
        $event = $line | ConvertFrom-Json -Depth 100
        $payload = Get-R7JsonProperty $event "payload"
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "map_runtime" -and
            [string](Get-R7JsonProperty $payload "map_event_type" "") -eq "task_context_event_recorded") {
            $eventType = [string](Get-R7JsonProperty $payload "eventType" "")
            $raw = Get-R7JsonProperty $payload "rawPayload"
            if ($eventType -eq "function_call") {
                $buffer.Add((ConvertTo-R7CallDescriptor -CallId ([string]$payload.callId) -ToolName ([string]$raw.name) -Arguments ([string]$raw.arguments)))
            } elseif ($eventType -eq "function_call_output") {
                $outcome = Get-R7CallOutcome -ToolSuccess ([bool]$payload.toolSuccess) -Output ([string]$raw.output)
                Set-R7CallOutcome $buffer ([string]$payload.callId) $outcome
            }
            continue
        }
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "map_runtime" -and
            [string](Get-R7JsonProperty $payload "map_event_type" "") -eq "taskspace_trace_event_recorded" -and
            [string](Get-R7JsonProperty $payload "kind" "") -eq "provider_response_actionability") {
            $requestTag = @($payload.tags | Where-Object { [string]$_ -like "request_count:*" } | Select-Object -First 1)
            if ($requestTag.Count) {
                $requestIndex = [int](([string]$requestTag[0]).Substring("request_count:".Length))
                if ($requestIndex -ge 1 -and $requestIndex -le $requests.Count) {
                    foreach ($call in $buffer) { $requests[$requestIndex - 1].calls.Add($call) }
                    $buffer.Clear()
                    $lastCommittedRequest = $requestIndex
                }
            }
            continue
        }
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "task_complete" -and $buffer.Count) {
            $requestIndex = [Math]::Min($ProviderRequests, $lastCommittedRequest + 1)
            foreach ($call in $buffer) { $requests[$requestIndex - 1].calls.Add($call) }
            $buffer.Clear()
        }
    }
    Complete-R7RequestRows $requests
}

function Get-R7StandardRequestPath {
    param([string]$RolloutPath, [int]$ProviderRequests)
    $requests = New-R7RequestRows $ProviderRequests
    $requestIndex = 1
    $callsById = @{}
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $RolloutPath) {
        $event = $line | ConvertFrom-Json -Depth 100
        $type = [string](Get-R7JsonProperty $event "type" "")
        $payload = Get-R7JsonProperty $event "payload"
        if ($type -eq "response_item" -and [string](Get-R7JsonProperty $payload "type" "") -eq "function_call") {
            $call = ConvertTo-R7CallDescriptor -CallId ([string]$payload.call_id) -ToolName ([string]$payload.name) -Arguments ([string]$payload.arguments)
            $requests[$requestIndex - 1].calls.Add($call)
            $callsById[$call.call_id] = $call
        } elseif ($type -eq "response_item" -and [string](Get-R7JsonProperty $payload "type" "") -eq "function_call_output") {
            $outcome = Get-R7CallOutcome -ToolSuccess $true -Output ([string]$payload.output)
            if ([string]$payload.output -match 'Shell exit code: [1-9]' -or [string]$payload.output -match 'apply_patch verification failed') {
                $outcome = Get-R7CallOutcome -ToolSuccess $false -Output ([string]$payload.output)
            }
            if ($callsById.ContainsKey([string]$payload.call_id)) {
                $call = $callsById[[string]$payload.call_id]
                $call.success = [bool]$outcome.success
                $call.failure_class = [string]$outcome.failure_class
                $call.failure_code = [string]$outcome.failure_code
            }
        } elseif ($type -eq "event_msg" -and [string](Get-R7JsonProperty $payload "type" "") -eq "token_count") {
            if ($requestIndex -lt $ProviderRequests) { $requestIndex++ }
        }
    }
    Complete-R7RequestRows $requests
}

function Get-R7WireSectionSummary {
    param([string]$WireTracePath)
    $totals = @{}
    $requestCount = 0
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $WireTracePath) {
        $event = $line | ConvertFrom-Json -Depth 100
        $sectionCost = Get-R7JsonProperty $event "section_cost"
        if ($null -eq $sectionCost) { continue }
        $requestCount++
        foreach ($section in @($sectionCost.sections)) {
            $kind = [string]$section.kind
            if (-not $totals.ContainsKey($kind)) { $totals[$kind] = [double]0 }
            $totals[$kind] += [double]$section.estimated_tokens
        }
    }
    $means = [ordered]@{}
    foreach ($kind in @($totals.Keys | Sort-Object)) {
        $means[$kind] = if ($requestCount) { [Math]::Round($totals[$kind] / $requestCount, 3) } else { 0 }
    }
    $orderedTotals = [ordered]@{}
    foreach ($kind in @($totals.Keys | Sort-Object)) { $orderedTotals[$kind] = [Math]::Round($totals[$kind], 3) }
    [pscustomobject]@{
        request_count = $requestCount
        estimated_tokens_total = [pscustomobject]$orderedTotals
        estimated_tokens_mean = [pscustomobject]$means
    }
}
