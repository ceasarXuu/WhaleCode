function Get-R7JsonProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

. (Join-Path $PSScriptRoot "r7-request-observability.ps1")
if (-not (Get-Command Get-TaskspaceOrdinaryToolFailureCode -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "ordinary-tool-outcome.ps1")
}
. (Join-Path $PSScriptRoot "canonical-rollout.ps1")
. (Join-Path $PSScriptRoot "r7-call-evidence.ps1")

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
            outcome = Get-R7CallOutcome `
                -ToolSuccess $success `
                -Output "ToolSearch pairing status: $status" `
                -ToolName "tool_search"
        }
    }
    $output = Get-R7JsonProperty $Item "output"
    $text = Get-R7OutputText $output
    $successProperty = Get-R7JsonProperty $output "success"
    $toolSuccess = Get-R7JsonProperty $OuterPayload "toolSuccess"
    $success = if ($null -ne $toolSuccess) {
        [bool]$toolSuccess
    } elseif ($null -ne $successProperty) {
        [bool]$successProperty
    } else {
        -not ($text -match 'Shell exit code: [1-9]' -or
            $text -match 'apply_patch verification failed')
    }
    [pscustomobject]@{
        call_id = $callId
        carrier_type = $type
        tool_success = $success
        output_text = $text
        outcome = Get-R7CallOutcome `
            -ToolSuccess $success `
            -Output $text
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
    $call.output_count = 1
    $outcome = if ([string]$call.tool -eq "taskspace_control") {
        Get-R7CallOutcome `
            -ToolSuccess ([bool]$Observed.tool_success) `
            -Output ([string]$Observed.output_text) `
            -ToolName ([string]$call.tool)
    } else {
        $Observed.outcome
    }
    Set-R7CallOutcome $call $outcome
}

function Apply-R7SupplementalFailure {
    param(
        [hashtable]$CallsById,
        [Collections.Generic.List[object]]$Requests,
        [string]$Text,
        [string]$OriginalRole
    )
    $trimmed = $Text.Trim()
    if (-not $trimmed.StartsWith("{")) { return }
    try { $payload = $trimmed | ConvertFrom-Json -Depth 100 } catch { return }
    $schemaVersion = [string](Get-R7JsonProperty $payload "schema_version" "")
    $knownSupplementalSchemas = @(
        "TaskSpaceResponseCommitFailureV3",
        "ToolSequencePreflightResultV3",
        "ProviderToolResponsePreflightV2",
        "ToolSearchFailureV3",
        "TaskSpaceToolSkippedV2",
        "TaskSpaceBoundResultCommitFailureV2"
    )
    if ($schemaVersion -notin $knownSupplementalSchemas) {
        return
    }
    if ($OriginalRole -ne "developer") {
        throw "Structured TaskSpace failure used an untrusted message role: $OriginalRole"
    }
    $provenance = Get-R7JsonProperty $payload "failure_provenance"
    $scope = [string](Get-R7JsonProperty $provenance "scope" "")
    $affectedCallIds = @(
        Get-R7JsonProperty $provenance "affected_call_ids" @() |
            ForEach-Object { [string]$_ }
    )
    if (-not $affectedCallIds.Count -or
        @($affectedCallIds | Sort-Object -Unique).Count -ne $affectedCallIds.Count) {
        throw "Structured failure fact has missing or duplicate affected_call_ids"
    }
    foreach ($affectedCallId in $affectedCallIds) {
        if (-not $CallsById.ContainsKey($affectedCallId)) {
            throw "Structured failure fact has no matching call: $affectedCallId"
        }
    }
    if ($schemaVersion -eq "ToolSearchFailureV3") {
        $callId = [string](Get-R7JsonProperty $payload "call_id" "")
        if ($affectedCallIds.Count -ne 1 -or $affectedCallIds[0] -ne $callId) {
            throw "ToolSearch failure provenance does not match call_id: $callId"
        }
        if (-not $CallsById.ContainsKey($callId) -or
            [string]$CallsById[$callId].call_type -ne "tool_search_call") {
            throw "ToolSearch failure fact has no matching ToolSearch call: $callId"
        }
    } elseif ($schemaVersion -eq "TaskSpaceToolSkippedV2" -or
        $schemaVersion -eq "TaskSpaceBoundResultCommitFailureV2") {
        $callId = [string](Get-R7JsonProperty $payload "call_id" "")
        if ($affectedCallIds.Count -ne 1 -or $affectedCallIds[0] -ne $callId) {
            throw "Per-call TaskSpace failure provenance does not match call_id: $callId"
        }
        $expectedScope = if ($schemaVersion -eq "TaskSpaceToolSkippedV2") {
            "tool_sequence_skip"
        } else {
            "tool_result_attribution"
        }
        if ($scope -ne $expectedScope) {
            throw "Per-call TaskSpace failure provenance has the wrong scope: $callId"
        }
    } else {
        $owningRequests = @(
            $Requests |
                Where-Object {
                    $requestCallIds = @(
                        $_.calls | ForEach-Object { [string]$_.call_id }
                    )
                    @($affectedCallIds | Where-Object { $_ -in $requestCallIds }).Count
                }
        )
        if ($owningRequests.Count -ne 1) {
            throw "Provider-response failure provenance spans multiple or no requests"
        }
        $requestCallIds = @(
            $owningRequests[0].calls | ForEach-Object { [string]$_.call_id }
        )
        if ($scope -ne "provider_response" -or
            -not [bool](Get-R7JsonProperty $provenance "zero_dispatch" $false) -or
            [string]::IsNullOrWhiteSpace(
                [string](Get-R7JsonProperty $provenance "copy_group_id" "")
            ) -or
            (Compare-Object @($requestCallIds | Sort-Object) @($affectedCallIds | Sort-Object))) {
            throw "Provider-response failure provenance does not match the request call set"
        }
    }
    foreach ($callId in $affectedCallIds) {
        $call = $CallsById[$callId]
        if ([int]$call.supplemental_count -ne 0) {
            throw "Duplicate structured failure fact for call: $callId"
        }
        $call.supplemental_count = 1
        Set-R7CallOutcome $call (
            Get-R7CallOutcome `
                -ToolSuccess $false `
                -Output $trimmed `
                -ToolName ([string]$call.tool) `
                -TrustedRuntimeCarrier
        )
    }
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
            foreach ($text in @(Get-R7MessageTexts $raw $eventType)) {
                Apply-R7SupplementalFailure `
                    $callsById `
                    $requests `
                    $text `
                    ([string](Get-R7JsonProperty $payload "originalRole" ""))
                if ($text -match '"schema_version"\s*:\s*"TaskSpaceResponseFinalReceiptV1"') {
                    $request = $requests[$requestIndex - 1]
                    $request.receipt_before = $true
                    $request.receipt_count = [int]$request.receipt_count + 1
                    $request.receipt_original_role =
                        [string](Get-R7JsonProperty $payload "originalRole" "")
                }
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
