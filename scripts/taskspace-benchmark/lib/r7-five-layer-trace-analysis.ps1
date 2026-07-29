function Get-R7JsonProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

. (Join-Path $PSScriptRoot "r7-request-observability.ps1")

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
    $declaredActions = @(
        Get-R7JsonProperty $parsed "actions" @() |
            ForEach-Object {
                [pscustomobject]@{
                    node_id = [string](Get-R7JsonProperty $_ "node_id" "")
                    tool = [string](Get-R7JsonProperty $_ "tool" "")
                }
            }
    )
    $node = if ($controlAction -eq "finish_map") {
        [string](Get-R7JsonProperty $parsed "finish_node_id" "")
    } else {
        @($declaredActions | ForEach-Object node_id) -join ","
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
        declared_node_id = ""
        declared_actions = $declaredActions
        node = $node
        detail = $detail
        success = $null
        failure_class = ""
        failure_code = ""
        violation_codes = @()
        violation_contexts = @()
        state_commit = $null
    }
}

function Get-R7CallOutcome {
    param([bool]$ToolSuccess, [string]$Output)
    $failureClass = ""
    $failureCode = ""
    $violationCodes = @()
    $violationContexts = @()
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
        $failurePayload = $payload
        $error = Get-R7JsonProperty $failurePayload "error"
        $errorClass = [string](Get-R7JsonProperty $error "class" "")
        $errorCode = [string](Get-R7JsonProperty $error "code" "")
        $violations = @(Get-R7JsonProperty $error "violations" @())
        if (-not $violations.Count) {
            $violations = @(Get-R7JsonProperty $failurePayload "violations" @())
        }
        $violationCodes = @(
            $violations |
                ForEach-Object { [string](Get-R7JsonProperty $_ "code" "") } |
                Where-Object { $_ } |
                Sort-Object -Unique
        )
        $violationContexts = @($violations)
        if ($schemaVersion -eq "ToolSequencePreflightResultV2") {
            $failureClass = "tool_sequence_protocol"
            $failureCode = $errorCode
        } elseif ($errorCode) {
            $failureClass = switch ($errorClass) {
                "state_machine" { "taskspace_state_machine" }
                "protocol" { "taskspace_protocol" }
                "argument" { "taskspace_protocol" }
                "resource" { "taskspace_resource" }
                "tool" { "ordinary_tool" }
                default { "taskspace" }
            }
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
        violation_codes = $violationCodes
        violation_contexts = $violationContexts
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
        $call.violation_codes = @($Outcome.violation_codes)
        $call.violation_contexts = @($Outcome.violation_contexts)
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
                receipt_before = $false
                receipt_count = 0
                receipt_original_role = ""
            })
    }
    $requests
}

function Complete-R7RequestRows {
    param([Collections.Generic.List[object]]$Requests)
    foreach ($request in $Requests) {
        $calls = @($request.calls)
        $control = @($calls | Where-Object tool -eq "taskspace_control" | Select-Object -First 1)
        if ($control.Count -eq 1 -and @($control[0].declared_actions).Count) {
            $ordinary = @($calls | Where-Object tool -ne "taskspace_control")
            for ($index = 0; $index -lt [Math]::Min($ordinary.Count, @($control[0].declared_actions).Count); $index++) {
                $ordinary[$index].declared_node_id = [string]$control[0].declared_actions[$index].node_id
                $ordinary[$index].node = [string]$control[0].declared_actions[$index].node_id
            }
        }
        $request.calls = $calls
        $request | Add-Member -NotePropertyName action_kind -NotePropertyValue $(if ($calls.Count) { "tool_calls" } else { "assistant_only" })
        $request | Add-Member -NotePropertyName failure_codes -NotePropertyValue @(
            $calls | Where-Object failure_code | ForEach-Object failure_code | Sort-Object -Unique
        )
        Add-R7RequestFailureFacts $request
    }
    @($Requests)
}

function Get-R7TaskspaceRequestPath {
    param([string]$RolloutPath, [int]$ProviderRequests)
    $requests = New-R7RequestRows $ProviderRequests
    $requestIndex = 1
    $observedRequestCount = 0
    $callsById = @{}
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $RolloutPath) {
        $event = $line | ConvertFrom-Json -Depth 100
        $payload = Get-R7JsonProperty $event "payload"
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "map_runtime" -and
            [string](Get-R7JsonProperty $payload "map_event_type" "") -eq "task_context_event_recorded") {
            $eventType = [string](Get-R7JsonProperty $payload "eventType" "")
            $raw = Get-R7JsonProperty $payload "rawPayload"
            if ($eventType -eq "function_call") {
                $call = ConvertTo-R7CallDescriptor -CallId ([string]$payload.callId) -ToolName ([string]$raw.name) -Arguments ([string]$raw.arguments)
                if ($callsById.ContainsKey($call.call_id)) {
                    throw "Duplicate Tool call id in TaskSpace rollout: $($call.call_id)"
                }
                $requests[$requestIndex - 1].calls.Add($call)
                $callsById[$call.call_id] = $call
            } elseif ($eventType -eq "function_call_output") {
                if (-not $callsById.ContainsKey([string]$payload.callId)) {
                    throw "Orphan Tool output in TaskSpace rollout: $([string]$payload.callId)"
                }
                $outcome = Get-R7CallOutcome -ToolSuccess ([bool]$payload.toolSuccess) -Output ([string]$raw.output)
                $call = $callsById[[string]$payload.callId]
                $call.success = [bool]$outcome.success
                $call.failure_class = [string]$outcome.failure_class
                $call.failure_code = [string]$outcome.failure_code
                $call.violation_codes = @($outcome.violation_codes)
                $call.violation_contexts = @($outcome.violation_contexts)
                $call.state_commit = $outcome.state_commit
            } elseif ($eventType -eq "message" -and
                (($raw | ConvertTo-Json -Compress -Depth 100) -match 'TaskSpaceResponseFinalReceiptV1')) {
                $request = $requests[$requestIndex - 1]
                $request.receipt_before = $true
                $request.receipt_count = [int]$request.receipt_count + 1
                $request.receipt_original_role = [string](Get-R7JsonProperty $payload "originalRole" "")
            }
            continue
        }
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "token_count") {
            $observedRequestCount++
            if ($observedRequestCount -gt $ProviderRequests) {
                throw "TaskSpace rollout has more token_count boundaries than provider requests"
            }
            if ($requestIndex -lt $ProviderRequests) { $requestIndex++ }
        }
    }
    if ($observedRequestCount -ne $ProviderRequests) {
        throw "TaskSpace rollout request boundary mismatch: observed=$observedRequestCount expected=$ProviderRequests"
    }
    $missingOutputs = @($callsById.Values | Where-Object { $null -eq $_.success })
    if ($missingOutputs.Count) {
        throw "TaskSpace rollout has $($missingOutputs.Count) Tool calls without outputs"
    }
    Complete-R7RequestRows $requests
}

function Get-R7StandardRequestPath {
    param([string]$RolloutPath, [int]$ProviderRequests)
    $requests = New-R7RequestRows $ProviderRequests
    $requestIndex = 1
    $observedRequestCount = 0
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
                $call.violation_codes = @($outcome.violation_codes)
                $call.violation_contexts = @($outcome.violation_contexts)
            }
        } elseif ($type -eq "event_msg" -and [string](Get-R7JsonProperty $payload "type" "") -eq "token_count") {
            $observedRequestCount++
            if ($observedRequestCount -gt $ProviderRequests) {
                throw "Standard rollout has more token_count boundaries than provider requests"
            }
            if ($requestIndex -lt $ProviderRequests) { $requestIndex++ }
        }
    }
    if ($observedRequestCount -ne $ProviderRequests) {
        throw "Standard rollout request boundary mismatch: observed=$observedRequestCount expected=$ProviderRequests"
    }
    $missingOutputs = @($callsById.Values | Where-Object { $null -eq $_.success })
    if ($missingOutputs.Count) {
        throw "Standard rollout has $($missingOutputs.Count) Tool calls without outputs"
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
