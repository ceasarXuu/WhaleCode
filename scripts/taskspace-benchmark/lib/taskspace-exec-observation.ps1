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
                foreach ($call in @($arguments.calls)) {
                    $tool = [string]$call.tool
                    if ($tool -in $mapTools) { $mapCalls++; continue }
                    $clientCalls++
                    if (-not [string]::IsNullOrWhiteSpace([string]$call.node_id)) { $nodeBindings++ }
                    switch ($tool) {
                        'exec_command' { $shell++ }
                        'apply_patch' { $patch++ }
                        default { $other++ }
                    }
                }
                foreach ($binding in @($arguments.hosted_bindings)) {
                    $hostedBindings++
                    $nodeBindings += @($binding.node_ids).Count
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
            try {
                $result = ([string]$payload.output) | ConvertFrom-Json
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
        }
    }
    if ($traceEvents -eq 0) { $findings.Add('taskspace_exec_trace_missing') }
    if ($correlatedRequests.Count -eq 0) { $findings.Add('trace_request_identity_missing') }
    foreach ($callId in $execCalls.Keys) {
        if (-not $correlatedOuterCalls.ContainsKey($callId)) { $findings.Add("trace_outer_call_missing:$callId") }
    }

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
