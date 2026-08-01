function Get-R7JsonProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

. (Join-Path $PSScriptRoot "r7-integer-facts.ps1")
. (Join-Path $PSScriptRoot "r7-request-observability.ps1")
. (Join-Path $PSScriptRoot "r7-state-rejection-summary.ps1")
. (Join-Path $PSScriptRoot "r7-json-facts.ps1")
if (-not (Get-Command Get-TaskspaceOrdinaryToolFailureCode -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "ordinary-tool-outcome.ps1")
}
. (Join-Path $PSScriptRoot "canonical-rollout.ps1")
. (Join-Path $PSScriptRoot "r7-state-failure-contract.ps1")
. (Join-Path $PSScriptRoot "r7-call-evidence.ps1")
. (Join-Path $PSScriptRoot "r7-direct-failure-carrier.ps1")
. (Join-Path $PSScriptRoot "r7-final-control-result-binding.ps1")
. (Join-Path $PSScriptRoot "r7-supplemental-failure.ps1")

function New-R7RequestRows {
    param([int]$ProviderRequests)
    $requests = [Collections.Generic.List[object]]::new()
    foreach ($index in 1..$ProviderRequests) {
            $requests.Add([pscustomobject]@{
                request_index = $index
                calls = [Collections.Generic.List[object]]::new()
                final_control_result_before = $false
                final_control_result_count = 0
                final_control_result_item_kind = ""
                rollout_provider_request_id = ""
                rollout_provider_logical_request_id = ""
                rollout_provider_attempt_seq = 0
            })
    }
    $requests
}

function Get-R7ResponseItemCallDescriptor {
    param($Item, [string]$OuterCallId = "", [string]$OuterType = "")
    $type = [string](Get-R7JsonProperty $Item "type" "")
    if ([string]::IsNullOrWhiteSpace($type)) { $type = $OuterType }
    if ($type -notin @(
            "function_call", "custom_tool_call", "tool_search_call",
            "local_shell_call"
        )) {
        return $null
    }
    $callId = [string](Get-R7JsonProperty $Item "call_id" $OuterCallId)
    if ([string]::IsNullOrWhiteSpace($callId)) { $callId = $OuterCallId }
    if ([string]::IsNullOrWhiteSpace($callId)) {
        throw "Executable Tool call has no call_id: $type"
    }
    $toolName = switch ($type) {
        "tool_search_call" { "tool_search" }
        "local_shell_call" { "local_shell" }
        default {
            $namespace = [string](Get-R7JsonProperty $Item "namespace" "")
            $namespace + [string](Get-R7JsonProperty $Item "name" "")
        }
    }
    if ([string]::IsNullOrWhiteSpace($toolName)) {
        throw "Executable Tool call has no tool name: $callId"
    }
    $arguments = switch ($type) {
        "custom_tool_call" { [string](Get-R7JsonProperty $Item "input" "") }
        "local_shell_call" {
            Get-R7JsonProperty $Item "action" @{} | ConvertTo-Json -Compress -Depth 100
        }
        "tool_search_call" {
            Get-R7JsonProperty $Item "arguments" @{} | ConvertTo-Json -Compress -Depth 100
        }
        default { [string](Get-R7JsonProperty $Item "arguments" "") }
    }
    ConvertTo-R7CallDescriptor `
        -CallId $callId `
        -ToolName $toolName `
        -Arguments $arguments `
        -CallType $type
}

function Get-R7OutputText {
    param($Output)
    if ($null -eq $Output) { return "" }
    if ($Output -is [string]) { return [string]$Output }
    foreach ($name in @("text", "body", "content")) {
        $value = Get-R7JsonProperty $Output $name
        if ($null -ne $value) {
            if ($value -is [string]) { return [string]$value }
            return ($value | ConvertTo-Json -Compress -Depth 100)
        }
    }
    $Output | ConvertTo-Json -Compress -Depth 100
}

function Get-R7ResponseItemOutcome {
    param($Item, $OuterPayload = $null, [string]$OuterType = "")
    $type = [string](Get-R7JsonProperty $Item "type" "")
    if ([string]::IsNullOrWhiteSpace($type)) { $type = $OuterType }
    if ($type -notin @(
            "function_call_output", "custom_tool_call_output",
            "tool_search_output", "tool_search_call_output",
            "local_shell_call_output"
        )) {
        return $null
    }
    $callId = [string](Get-R7JsonProperty $Item "call_id" "")
    if ([string]::IsNullOrWhiteSpace($callId)) {
        $callId = [string](Get-R7JsonProperty $OuterPayload "callId" "")
    }
    if ([string]::IsNullOrWhiteSpace($callId)) {
        throw "Executable Tool output has no call_id: $type"
    }
    if ($type -in @("tool_search_output", "tool_search_call_output")) {
        $status = [string](Get-R7JsonProperty $Item "status" "")
        $success = $status -eq "completed"
        return [pscustomobject]@{
            call_id = $callId
            carrier_type = $type
            tool_success = $success
            output_text = "ToolSearch pairing status: $status"
        }
    }
    $output = Get-R7JsonProperty $Item "output"
    $text = Get-R7OutputText $output
    $toolSuccess = Get-R7JsonProperty $OuterPayload "toolSuccess"
    $success = if ($toolSuccess -is [bool]) {
        [bool]$toolSuccess
    } elseif ($output -is [pscustomobject] -and
        (Get-R7JsonProperty $output "success") -is [bool] -and
        $null -ne (Get-R7JsonProperty $output "content")) {
        [bool](Get-R7JsonProperty $output "success")
    } else {
        $null
    }
    [pscustomobject]@{
        call_id = $callId
        carrier_type = $type
        tool_success = $success
        output_text = $text
    }
}

function Get-R7MessageTexts {
    param($Item, [string]$OuterType = "")
    $type = [string](Get-R7JsonProperty $Item "type" "")
    if ([string]::IsNullOrWhiteSpace($type)) { $type = $OuterType }
    if ($type -ne "message") { return @() }
    @(
        Get-R7JsonProperty $Item "content" @() |
            ForEach-Object { [string](Get-R7JsonProperty $_ "text" "") } |
            Where-Object { $_ }
    )
}

function Add-R7ObservedCall {
    param($Requests, [int]$RequestIndex, [hashtable]$CallsById, $Call)
    if ($CallsById.ContainsKey($Call.call_id)) {
        throw "Duplicate Tool call id in rollout: $($Call.call_id)"
    }
    $Call.request_index = $RequestIndex
    $Requests[$RequestIndex - 1].calls.Add($Call)
    $CallsById[$Call.call_id] = $Call
}

function Apply-R7ObservedOutcome {
    param([hashtable]$CallsById, $Observed)
    if (-not $CallsById.ContainsKey([string]$Observed.call_id)) {
        throw "Orphan Tool output in rollout: $([string]$Observed.call_id)"
    }
    $call = $CallsById[[string]$Observed.call_id]
    if ([int]$call.output_count -ne 0) {
        throw "Duplicate Tool output in rollout: $([string]$Observed.call_id)"
    }
    $compatibleCarriers = switch ([string]$call.call_type) {
        "custom_tool_call" { @("custom_tool_call_output") }
        "tool_search_call" { @("tool_search_output", "tool_search_call_output") }
        "local_shell_call" { @("local_shell_call_output", "function_call_output") }
        default { @("function_call_output") }
    }
    if ([string]$Observed.carrier_type -notin $compatibleCarriers) {
        throw "Tool call/output carrier mismatch for $([string]$Observed.call_id)"
    }
    if ([int]$call.supplemental_count -ne 0) {
        throw "Tool output followed its supplemental failure: $([string]$Observed.call_id)"
    }
    $call.output_count = 1
    $call.observed_output_text = [string]$Observed.output_text
    $call.observed_output_tool_success = $Observed.tool_success
    $outcome = Get-R7CallOutcome `
        -ToolSuccess $Observed.tool_success `
        -Output ([string]$Observed.output_text) `
        -ToolName ([string]$call.tool)
    if ([string]$call.tool -ceq "taskspace_control" -and
        $outcome.success -eq $true -and
        [string]$outcome.carrier_schema -ceq "TaskSpaceResponseResultV2") {
        $bindingError = Get-R7FinalControlResultRequestBindingError `
            $call `
            $outcome.parsed_payload
        $bindingReason = "final_control_result_request_mismatch"
        if ([string]::IsNullOrWhiteSpace([string]$bindingError)) {
            $bindingError = Set-R7ExpectedReservations `
                $CallsById `
                $call `
                $outcome.parsed_payload
            $bindingReason = "final_control_result_reservation_mismatch"
        }
        if (-not [string]::IsNullOrWhiteSpace([string]$bindingError)) {
            $outcome = Complete-R7CallOutcomeFacts (
                New-R7InvalidCallOutcome `
                    $bindingReason `
                    $bindingReason `
                    "TaskSpaceResponseResultV2" `
                    $true
            ) $outcome.parsed_payload
        } else {
            $call.reservation_mutated = $true
        }
    }
    Set-R7CallOutcome $call $outcome
}

function Complete-R7RequestRows {
    param([Collections.Generic.List[object]]$Requests)
    $identityGroups = @(
        $Requests |
            Group-Object rollout_provider_request_id |
            Where-Object {
                [string]::IsNullOrWhiteSpace([string]$_.Name) -or $_.Count -ne 1
            }
    )
    if ($identityGroups.Count) {
        $identitySummary = @(
            $Requests | ForEach-Object { [string]$_.rollout_provider_request_id }
        ) -join ","
        throw "Rollout provider request identities are missing or duplicated: $identitySummary"
    }
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
            $raw = Get-R7JsonProperty $payload "rawPayload"
            $eventType = [string](Get-R7JsonProperty $payload "eventType" "")
            $call = Get-R7ResponseItemCallDescriptor $raw ([string]$payload.callId) $eventType
            if ($null -ne $call) {
                Add-R7ObservedCall $requests $requestIndex $callsById $call
            }
            $observedOutcome = Get-R7ResponseItemOutcome $raw $payload $eventType
            if ($null -ne $observedOutcome) {
                Apply-R7ObservedOutcome $callsById $observedOutcome
            }
            $rawType = [string](Get-R7JsonProperty $raw "type" $eventType)
            if ($rawType -in @(
                    "function_call_output", "custom_tool_call_output",
                    "tool_search_output", "tool_search_call_output"
                )) {
                $resultText = [string](Get-R7JsonProperty $raw "output" "")
                $resultFact = $null
                if (-not [string]::IsNullOrWhiteSpace($resultText)) {
                    try { $resultFact = $resultText | ConvertFrom-Json } catch {}
                }
                if ([string](Get-R7JsonProperty $resultFact "schema_version" "") -eq
                    "TaskSpaceResponseResultV2") {
                    $request = $requests[$requestIndex - 1]
                    $request.final_control_result_before = $true
                    $request.final_control_result_count =
                        [int]$request.final_control_result_count + 1
                    $request.final_control_result_item_kind = $rawType
                }
            }
            foreach ($text in @(Get-R7MessageTexts $raw $eventType)) {
                Apply-R7SupplementalFailure `
                    $callsById `
                    $requests `
                    $text `
                    ([string](Get-R7JsonProperty $payload "originalRole" ""))
            }
            continue
        }
        if ([string](Get-R7JsonProperty $event "type" "") -eq "event_msg" -and
            [string](Get-R7JsonProperty $payload "type" "") -eq "token_count") {
            $providerRequestId = [string](Get-R7JsonProperty $payload "provider_request_id" "")
            $logicalRequestId =
                [string](Get-R7JsonProperty $payload "provider_logical_request_id" "")
            $attemptSeq = [int](Get-R7JsonProperty $payload "provider_attempt_seq" 0)
            $hasAnyIdentity = -not [string]::IsNullOrWhiteSpace($providerRequestId) -or
                -not [string]::IsNullOrWhiteSpace($logicalRequestId) -or $attemptSeq -gt 0
            if (-not $hasAnyIdentity) {
                continue
            }
            if ([string]::IsNullOrWhiteSpace($providerRequestId) -or
                [string]::IsNullOrWhiteSpace($logicalRequestId) -or $attemptSeq -lt 1) {
                throw "TaskSpace token_count boundary has incomplete provider request identity"
            }
            $observedRequestCount++
            if ($observedRequestCount -gt $ProviderRequests) {
                throw "TaskSpace rollout has more token_count boundaries than provider requests"
            }
            $requests[$requestIndex - 1].rollout_provider_request_id = $providerRequestId
            $requests[$requestIndex - 1].rollout_provider_logical_request_id = $logicalRequestId
            $requests[$requestIndex - 1].rollout_provider_attempt_seq = $attemptSeq
            if ($requestIndex -lt $ProviderRequests) { $requestIndex++ }
        }
    }
    if ($observedRequestCount -ne $ProviderRequests) {
        throw "TaskSpace rollout request boundary mismatch: observed=$observedRequestCount expected=$ProviderRequests"
    }
    $missingOutputs = @($callsById.Values | Where-Object { [int]$_.output_count -ne 1 })
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
        if ($type -eq "response_item") {
            $call = Get-R7ResponseItemCallDescriptor $payload
            if ($null -ne $call) {
                Add-R7ObservedCall $requests $requestIndex $callsById $call
            }
            $observedOutcome = Get-R7ResponseItemOutcome $payload
            if ($null -ne $observedOutcome) {
                Apply-R7ObservedOutcome $callsById $observedOutcome
            }
            foreach ($text in @(Get-R7MessageTexts $payload)) {
                Apply-R7SupplementalFailure `
                    $callsById `
                    $requests `
                    $text `
                    ([string](Get-R7JsonProperty $payload "role" ""))
            }
        } elseif ($type -eq "event_msg" -and [string](Get-R7JsonProperty $payload "type" "") -eq "token_count") {
            $providerRequestId = [string](Get-R7JsonProperty $payload "provider_request_id" "")
            $logicalRequestId =
                [string](Get-R7JsonProperty $payload "provider_logical_request_id" "")
            $attemptSeq = [int](Get-R7JsonProperty $payload "provider_attempt_seq" 0)
            $hasAnyIdentity = -not [string]::IsNullOrWhiteSpace($providerRequestId) -or
                -not [string]::IsNullOrWhiteSpace($logicalRequestId) -or $attemptSeq -gt 0
            if (-not $hasAnyIdentity) {
                continue
            }
            if ([string]::IsNullOrWhiteSpace($providerRequestId) -or
                [string]::IsNullOrWhiteSpace($logicalRequestId) -or $attemptSeq -lt 1) {
                throw "Standard token_count boundary has incomplete provider request identity"
            }
            $observedRequestCount++
            if ($observedRequestCount -gt $ProviderRequests) {
                throw "Standard rollout has more token_count boundaries than provider requests"
            }
            $requests[$requestIndex - 1].rollout_provider_request_id = $providerRequestId
            $requests[$requestIndex - 1].rollout_provider_logical_request_id = $logicalRequestId
            $requests[$requestIndex - 1].rollout_provider_attempt_seq = $attemptSeq
            if ($requestIndex -lt $ProviderRequests) { $requestIndex++ }
        }
    }
    if ($observedRequestCount -ne $ProviderRequests) {
        throw "Standard rollout request boundary mismatch: observed=$observedRequestCount expected=$ProviderRequests"
    }
    $missingOutputs = @($callsById.Values | Where-Object { [int]$_.output_count -ne 1 })
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
