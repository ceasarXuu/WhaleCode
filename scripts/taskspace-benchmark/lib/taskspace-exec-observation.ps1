Set-StrictMode -Version Latest

function Get-TaskspaceExecProperty {
    param($Object, [string]$Name)
    if ($null -eq $Object) { return $null }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) { return $null }
    $property.Value
}

function Get-TaskspaceExecTraceFields {
    param([string]$Line)
    $fields = @{}
    foreach ($match in [regex]::Matches($Line, '(?<key>[a-z_]+)=(?:"(?<quoted>[^"]*)"|(?<bare>[^\s]+))')) {
        $fields[[string]$match.Groups['key'].Value] = if ($match.Groups['quoted'].Success) {
            [string]$match.Groups['quoted'].Value
        } else {
            [string]$match.Groups['bare'].Value
        }
    }
    $fields
}

function New-TaskspaceExecObservation {
    param([string]$Availability = 'missing')
    [pscustomobject]@{
        protocol = 'taskspace_exec'
        availability = $Availability
        source = 'rollout+taskspace_exec_trace+request_facts'
        provider_outer_tool_calls = $null
        nested_action_count = $null
        shell = $null
        patch = $null
        other = $null
        exec_count = $null
        map_operation_count = $null
        client_action_count = $null
        hosted_binding_count = $null
        node_binding_count = $null
        client_result_count = $null
        hosted_result_count = $null
        failed_action_count = $null
        trace_event_count = $null
        correlated_request_count = $null
        correlated_outer_call_count = $null
        capability_identity = $null
        wire_capability_identity = $null
        findings = @()
    }
}

function Get-TaskspaceExecObservation {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [System.Collections.Generic.List[object]]$Events
    )
    $rolloutPath = Join-Path $ArtifactDir 'rollout.jsonl'
    if (-not (Test-Path -LiteralPath $rolloutPath -PathType Leaf)) {
        return New-TaskspaceExecObservation
    }

    $mapTools = @('initialize_map', 'update_map', 'read_map', 'reopen_map', 'finish_map')
    $execCalls = @{}
    $execCount = 0; $mapCalls = 0; $clientCalls = 0; $hostedBindings = 0
    $nodeBindings = 0; $shell = 0; $patch = 0; $other = 0
    $clientResults = 0; $hostedResults = 0; $failedActions = 0
    $findings = [Collections.Generic.List[string]]::new()

    foreach ($line in [IO.File]::ReadLines($rolloutPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $row = $line | ConvertFrom-Json } catch { continue }
        $payload = Get-TaskspaceCanonicalResponseItem $row
        if ($null -eq $payload) { continue }
        $type = [string]$payload.type
        if ($type -eq 'function_call' -and [string]$payload.name -eq 'taskspace_exec') {
            $callId = [string](Get-TaskspaceExecProperty $payload 'call_id')
            if ([string]::IsNullOrWhiteSpace($callId)) {
                $callId = "missing-call-$execCount"
                $findings.Add('exec_call_id_missing')
            }
            $execCalls[$callId] = $false
            $execCount++
            try {
                $arguments = if ($payload.arguments -is [string]) {
                    ([string]$payload.arguments) | ConvertFrom-Json
                } else { $payload.arguments }
                $callIndex = 0
                foreach ($call in @($arguments.calls)) {
                    $map = Get-TaskspaceExecProperty $call 'map'
                    $client = Get-TaskspaceExecProperty $call 'client'
                    if (($null -eq $map) -eq ($null -eq $client)) {
                        $findings.Add("exec_call_shape_invalid:${callId}:$callIndex")
                        $callIndex++
                        continue
                    }
                    if ($null -ne $map) {
                        $operation = [string](Get-TaskspaceExecProperty $map 'operation')
                        if ($operation -notin $mapTools) {
                            $findings.Add("exec_map_operation_unknown:${callId}:$callIndex")
                        }
                        $mapCalls++
                        $callIndex++
                        continue
                    }
                    $tool = [string](Get-TaskspaceExecProperty $client 'name')
                    $clientCalls++
                    if (-not [string]::IsNullOrWhiteSpace([string](Get-TaskspaceExecProperty $client 'node_id'))) { $nodeBindings++ }
                    switch ($tool) {
                        'exec_command' { $shell++ }
                        'apply_patch' { $patch++ }
                        default { $other++ }
                    }
                    $callIndex++
                }
                $hostedBindingValues = Get-TaskspaceExecProperty $arguments 'hosted_bindings'
                if ($null -ne $hostedBindingValues) {
                    foreach ($binding in @($hostedBindingValues)) {
                        $hostedBindings++
                        $nodeBindings += @($binding.node_ids).Count
                    }
                }
            } catch {
                $findings.Add("exec_arguments_invalid:$callId")
            }
            continue
        }
        $outputCallId = [string](Get-TaskspaceExecProperty $payload 'call_id')
        if ($type -eq 'function_call_output' -and $execCalls.ContainsKey($outputCallId)) {
            $callId = $outputCallId
            $execCalls[$callId] = $true
            $rawOutput = [string]$payload.output
            if ($rawOutput.StartsWith('taskspace_exec rejected:')) {
                $failedActions++
                continue
            }
            try {
                $result = $rawOutput | ConvertFrom-Json
                if ([string]$result.kind -ne 'taskspace_exec_result') {
                    throw 'unexpected taskspace_exec result kind'
                }
                if ([string]$result.outer_call_id -ne $callId) {
                    $findings.Add("exec_result_outer_call_mismatch:$callId")
                }
                $clientResults += @($result.client_results).Count
                $hostedResults += @($result.hosted_results).Count
                $failedActions += @($result.client_results | Where-Object { [string]$_.outcome -ne 'succeeded' }).Count
                $failedActions += @($result.hosted_results | Where-Object { [string]$_.outcome -ne 'succeeded' }).Count
            } catch {
                $findings.Add("exec_result_invalid:$callId")
            }
        }
    }
    if ($execCount -eq 0) { return New-TaskspaceExecObservation }
    foreach ($callId in $execCalls.Keys) {
        if (-not [bool]$execCalls[$callId]) { $findings.Add("exec_result_missing:$callId") }
    }

    $requestIds = @{}
    $requestFactsPath = Join-Path $ArtifactDir 'request-facts.json'
    if (Test-Path -LiteralPath $requestFactsPath -PathType Leaf) {
        try {
            $requestFacts = Get-Content -Raw -Encoding UTF8 -LiteralPath $requestFactsPath | ConvertFrom-Json
            foreach ($request in @($requestFacts.rows)) { $requestIds[[string]$request.request_id] = $true }
        } catch { $findings.Add('request_facts_invalid') }
    } else { $findings.Add('request_facts_missing') }

    $traceEvents = 0
    $correlatedRequests = @{}
    $correlatedOuterCalls = @{}
    $traceCapabilityByRequest = @{}
    $stderrPath = Join-Path $ArtifactDir 'whale-exec.stderr.log'
    if (Test-Path -LiteralPath $stderrPath -PathType Leaf) {
        foreach ($line in [IO.File]::ReadLines($stderrPath)) {
            if ($line -notmatch 'event_name="taskspace\.(?:exec|action_settlement)\.') { continue }
            $traceEvents++
            $fields = Get-TaskspaceExecTraceFields $line
            $eventName = [string]$fields['event_name']
            $requestId = [string]$fields['provider_request_id']
            if (-not [string]::IsNullOrWhiteSpace($requestId) -and -not $requestIds.ContainsKey($requestId)) {
                $findings.Add("trace_request_missing_from_canonical:$requestId")
            } elseif (-not [string]::IsNullOrWhiteSpace($requestId)) { $correlatedRequests[$requestId] = $true }
            $outerCallId = [string]$fields['outer_call_id']
            if (-not [string]::IsNullOrWhiteSpace($outerCallId) -and -not $execCalls.ContainsKey($outerCallId)) {
                $findings.Add("trace_outer_call_missing_from_rollout:$outerCallId")
            } elseif (-not [string]::IsNullOrWhiteSpace($outerCallId)) { $correlatedOuterCalls[$outerCallId] = $true }
            if ($eventName -eq 'taskspace.exec.completed') {
                if ([string]::IsNullOrWhiteSpace([string]$fields['provider_response_id'])) {
                    $findings.Add("trace_response_id_missing:$outerCallId")
                }
                if (-not $fields.ContainsKey('map_revision')) {
                    $findings.Add("trace_map_revision_missing:$outerCallId")
                }
            }
            if ($eventName -in @('taskspace.exec.response_finalized', 'taskspace.exec.completed')) {
                $capabilityIdentity = [string]$fields['capability_identity']
                if ($capabilityIdentity -notmatch '^[a-fA-F0-9]{64}$') {
                    $findings.Add("trace_capability_identity_missing_or_invalid:$requestId")
                } elseif (-not [string]::IsNullOrWhiteSpace($requestId)) {
                    if ($traceCapabilityByRequest.ContainsKey($requestId) -and
                        [string]$traceCapabilityByRequest[$requestId] -ne $capabilityIdentity) {
                        $findings.Add("trace_capability_identity_conflict:$requestId")
                    } else { $traceCapabilityByRequest[$requestId] = $capabilityIdentity }
                }
            }
        }
    }
    if ($traceEvents -eq 0) { $findings.Add('taskspace_exec_trace_missing') }
    if ($correlatedRequests.Count -eq 0) { $findings.Add('trace_request_identity_missing') }
    foreach ($callId in $execCalls.Keys) {
        if (-not $correlatedOuterCalls.ContainsKey($callId)) { $findings.Add("trace_outer_call_missing:$callId") }
    }

    $wireCapabilityByRequest = @{}
    $wireTracePath = Join-Path $ArtifactDir 'provider-wire-trace.jsonl'
    if (Test-Path -LiteralPath $wireTracePath -PathType Leaf) {
        foreach ($line in [IO.File]::ReadLines($wireTracePath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try { $wire = $line | ConvertFrom-Json } catch { continue }
            if ([string](Get-TaskspaceExecProperty $wire 'status') -ne 'payload_captured') { continue }
            $requestId = [string](Get-TaskspaceExecProperty $wire 'request_id')
            if (-not $correlatedRequests.ContainsKey($requestId)) { continue }
            $identity = [string](Get-TaskspaceExecProperty $wire 'taskspace_capability_identity')
            if ($identity -notmatch '^[a-fA-F0-9]{64}$') {
                $findings.Add("wire_capability_identity_missing_or_invalid:$requestId")
                continue
            }
            $wireCapabilityByRequest[$requestId] = $identity
        }
    } else { $findings.Add('provider_wire_trace_missing') }
    foreach ($requestId in $correlatedRequests.Keys) {
        if (-not $traceCapabilityByRequest.ContainsKey($requestId)) {
            $findings.Add("trace_capability_identity_missing:$requestId")
            continue
        }
        if (-not $wireCapabilityByRequest.ContainsKey($requestId)) {
            $findings.Add("wire_capability_identity_missing:$requestId")
            continue
        }
        if ([string]$traceCapabilityByRequest[$requestId] -ne [string]$wireCapabilityByRequest[$requestId]) {
            $findings.Add("capability_identity_mismatch:$requestId")
        }
    }
    $traceIdentities = @($traceCapabilityByRequest.Values | Sort-Object -Unique)
    $wireIdentities = @($wireCapabilityByRequest.Values | Sort-Object -Unique)
    if ($traceIdentities.Count -gt 1) { $findings.Add('trace_capability_identity_changed') }
    if ($wireIdentities.Count -gt 1) { $findings.Add('wire_capability_identity_changed') }

    [pscustomobject]@{
        protocol = 'taskspace_exec'; availability = if ($findings.Count) { 'incomparable' } else { 'measured' }
        source = 'rollout+taskspace_exec_trace+request_facts'
        provider_outer_tool_calls = [int]$execCount
        nested_action_count = [int]($mapCalls + $clientCalls + $hostedBindings)
        shell = [int]$shell; patch = [int]$patch; other = [int]$other
        exec_count = [int]$execCount; map_operation_count = [int]$mapCalls
        client_action_count = [int]$clientCalls; hosted_binding_count = [int]$hostedBindings
        node_binding_count = [int]$nodeBindings; client_result_count = [int]$clientResults
        hosted_result_count = [int]$hostedResults; failed_action_count = [int]$failedActions
        trace_event_count = [int]$traceEvents; correlated_request_count = [int]$correlatedRequests.Count
        correlated_outer_call_count = [int]$correlatedOuterCalls.Count
        capability_identity = if ($traceIdentities.Count -eq 1) { [string]$traceIdentities[0] } else { $null }
        wire_capability_identity = if ($wireIdentities.Count -eq 1) { [string]$wireIdentities[0] } else { $null }
        findings = @($findings | Sort-Object -Unique)
    }
}

function Get-PerformanceActionCounts {
    param([string]$ArtifactDir, [System.Collections.Generic.List[object]]$Events)
    $exec = Get-TaskspaceExecObservation $ArtifactDir $Events
    if ([string]$exec.availability -ne 'missing') { return $exec }
    $shell = 0; $patch = 0; $control = 0; $other = 0
    $providerOuterCalls = 0
    $rolloutPath = Join-Path $ArtifactDir 'rollout.jsonl'
    if (Test-Path -LiteralPath $rolloutPath) {
        foreach ($line in [IO.File]::ReadLines($rolloutPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try {
                $row = $line | ConvertFrom-Json
                $payload = Get-TaskspaceCanonicalResponseItem $row
                if ($null -eq $payload -or [string]$payload.type -notin @('function_call', 'custom_tool_call')) { continue }
                $providerOuterCalls++
                switch ([string]$payload.name) {
                    'exec_command' { $shell++ }
                    'apply_patch' { $patch++ }
                    'taskspace_control' { $control++ }
                    default { $other++ }
                }
            } catch {
                if ($null -ne $Events) {
                    $Events.Add([pscustomobject]@{ event = 'rollout_line_parse_failed'; path = $rolloutPath; error = [string]$_.Exception.Message })
                }
            }
        }
        return [pscustomobject]@{
            protocol = 'legacy_top_level_tools'; shell = $shell; patch = $patch
            control = $control; other = $other; provider_outer_tool_calls = $providerOuterCalls
            nested_action_count = 0; source = 'rollout'
        }
    }
    $execPath = Join-Path $ArtifactDir 'whale-exec.jsonl'
    if (Test-Path -LiteralPath $execPath) {
        foreach ($line in [IO.File]::ReadLines($execPath)) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            try {
                $row = $line | ConvertFrom-Json
                if ([string]$row.type -ne 'item.completed') { continue }
                switch ([string]$row.item.type) {
                    'command_execution' { $shell++ }
                    'file_change' { $patch++ }
                }
            } catch {
                if ($null -ne $Events) {
                    $Events.Add([pscustomobject]@{ event = 'exec_line_parse_failed'; path = $execPath; error = [string]$_.Exception.Message })
                }
            }
        }
        return [pscustomobject]@{
            protocol = 'standard_exec_output'; shell = $shell; patch = $patch; control = 0; other = 0
            provider_outer_tool_calls = $null; nested_action_count = 0; source = 'whale_exec'
        }
    }
    [pscustomobject]@{
        protocol = 'unavailable'; shell = $null; patch = $null; control = $null; other = $null
        provider_outer_tool_calls = $null; nested_action_count = $null; source = 'unavailable'
    }
}
