Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
. (Join-Path $root 'lib/performance-observation.ps1')
. (Join-Path $root 'lib/metrics-extractor.ps1')

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) { throw "$Message expected=$Expected actual=$Actual" }
}

$temp = Join-Path ([IO.Path]::GetTempPath()) "taskspace-exec-observation-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $temp -Force | Out-Null
try {
    $arguments = [pscustomobject]@{
        type = 'initialize_and_work'
        initialize_map = [pscustomobject]@{}
        tools = @(
            [pscustomobject]@{ tool = 'exec_command'; node_id = 'inspect'; input = [pscustomobject]@{ cmd = 'pwd' } },
            [pscustomobject]@{ tool = 'apply_patch'; node_id = 'fix'; input = 'x' }
        )
    }
    $result = [pscustomobject]@{
        client_results = @(
            [pscustomobject]@{ outcome = 'succeeded' },
            [pscustomobject]@{ outcome = 'failed' }
        )
    }
    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = ($arguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'
                output = ($result | ConvertTo-Json -Compress -Depth 12)
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    [pscustomobject]@{
        schema_version = 'whalecode-request-facts-v1'
        rows = @([pscustomobject]@{ request_id = 'request-1' })
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $temp 'request-facts.json') -Encoding UTF8
    $capabilityIdentity = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
    [pscustomobject]@{
        schema_version = 'provider-chat-wire-trace-v11'; status = 'payload_captured'
        request_id = 'request-1'; taskspace_capability_identity = $capabilityIdentity
    } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $temp 'provider-wire-trace.jsonl') -Encoding UTF8
    @(
        "INFO codex_core::taskspace_exec: event_name=`"taskspace.exec.response_finalized`" provider_request_id=`"request-1`" provider_response_id=`"response-1`" outer_call_id=`"outer-1`" map_id=`"map-1`" capability_identity=`"$capabilityIdentity`"",
        "INFO codex_core::taskspace_exec: event_name=`"taskspace.exec.completed`" provider_request_id=`"request-1`" provider_response_id=`"response-1`" outer_call_id=`"outer-1`" map_revision=Some(2) capability_identity=`"$capabilityIdentity`"",
        "INFO codex_core::taskspace: event_name=`"taskspace.provider_actions_recorded`" map_id=`"map-1`" map_revision=3 provider_action_count=1 provider_failed_action_count=0"
    ) | Set-Content -LiteralPath (Join-Path $temp 'whale-exec.stderr.log') -Encoding UTF8

    $facts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $facts.availability 'measured' 'valid evidence was not comparable'
    Assert-Equal $facts.exec_count 1 'outer Exec count drifted'
    Assert-Equal $facts.map_operation_count 1 'Map operation count drifted'
    Assert-Equal $facts.client_action_count 2 'client action count drifted'
    Assert-Equal $facts.provider_action_count 1 'Provider action count drifted'
    Assert-Equal $facts.node_binding_count 2 'node binding count drifted'
    Assert-Equal $facts.failed_action_count 1 'failure count drifted'
    Assert-Equal $facts.correlated_request_count 1 'request identity was not joined'
    Assert-Equal $facts.correlated_outer_call_count 1 'outer call identity was not joined'
    Assert-Equal $facts.capability_identity $capabilityIdentity 'Exec capability identity was not observed'
    Assert-Equal $facts.wire_capability_identity $capabilityIdentity 'wire capability identity was not observed'

    $multiOuterDir = Join-Path $temp 'multi-outer'
    New-Item -ItemType Directory -Path $multiOuterDir -Force | Out-Null
    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = ($arguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-2'
                arguments = ($arguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'
                output = 'taskspace_exec rejected: invalid top-level contract: one provider response must contain exactly one outer taskspace_exec; put all client actions in that call''s tools[]. No Map or Tool actions were executed.'
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-2'
                output = 'taskspace_exec rejected: invalid top-level contract: one provider response must contain exactly one outer taskspace_exec; put all client actions in that call''s tools[]. No Map or Tool actions were executed.'
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $multiOuterDir 'rollout.jsonl') -Encoding UTF8
    [pscustomobject]@{
        schema_version = 'whalecode-request-facts-v1'
        rows = @([pscustomobject]@{ request_id = 'request-1' })
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $multiOuterDir 'request-facts.json') -Encoding UTF8
    [pscustomobject]@{
        schema_version = 'provider-chat-wire-trace-v11'; status = 'payload_captured'
        request_id = 'request-1'; taskspace_capability_identity = $capabilityIdentity
    } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $multiOuterDir 'provider-wire-trace.jsonl') -Encoding UTF8
    @(
        "INFO codex_core::taskspace_exec: event_name=`"taskspace.exec.response_finalized`" provider_request_id=`"request-1`" provider_response_id=`"response-1`" outer_call_id=`"outer-1`" map_id=`"map-1`" capability_identity=`"$capabilityIdentity`" exec_call_count=2 accepted=false",
        'WARN codex_core::taskspace_exec: event_name="taskspace.exec.rejected" reason_code="response_cardinality_rejected" outer_call_id="outer-1"',
        'WARN codex_core::taskspace_exec: event_name="taskspace.exec.rejected" reason_code="response_cardinality_rejected" outer_call_id="outer-2"'
    ) | Set-Content -LiteralPath (Join-Path $multiOuterDir 'whale-exec.stderr.log') -Encoding UTF8

    $multiOuterFacts = Get-TaskspaceExecObservation $multiOuterDir $null
    Assert-Equal $multiOuterFacts.availability 'measured' 'recoverable multi-outer rejection was not measurable'
    Assert-Equal $multiOuterFacts.exec_count 2 'multi-outer call count drifted'
    Assert-Equal $multiOuterFacts.rejected_call_count 2 'multi-outer rejection total drifted'
    Assert-Equal $multiOuterFacts.rejected_contract_call_count 2 'multi-outer contract classification drifted'
    Assert-Equal $multiOuterFacts.rejected_unknown_call_count 0 'multi-outer rejection was classified as unknown'
    Assert-Equal $multiOuterFacts.correlated_request_count 1 'multi-outer request identity was not joined'
    Assert-Equal $multiOuterFacts.correlated_outer_call_count 2 'both multi-outer call identities were not joined'
    if (@($multiOuterFacts.findings | Where-Object { $_ -like 'exec_result_missing:*' }).Count -ne 0) {
        throw 'recoverable multi-outer rejection was reported as missing Exec output'
    }

    $referencedResult = $result | ConvertTo-Json -Compress -Depth 12
    $referencedSha = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
    $referenceDir = Join-Path $temp 'home/.whale/session-store/output-refs/sha256'
    New-Item -ItemType Directory -Path $referenceDir -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $referenceDir "$referencedSha.stdout") -Value $referencedResult -Encoding UTF8
    $referenceOutput = "OutputReferenceV1:`nartifact_ref: output-ref://sha256/$referencedSha`nraw_output_elided: true"
    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = ($arguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'; output = $referenceOutput
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    $referencedFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $referencedFacts.availability 'measured' 'referenced Exec result was not resolved'
    Assert-Equal $referencedFacts.client_result_count 2 'referenced client results were not counted'
    Assert-Equal $referencedFacts.provider_result_count 1 'referenced Provider results were not counted'

    $sequenceCases = @(
        @('work', @{ type = 'work'; tools = @([pscustomobject]@{ tool = 'exec_command'; node_id = 'n'; input = @{} }) }, 'client'),
        @('update_map', @{ type = 'update_map'; update_map = @{} }, 'map'),
        @('update_and_work', @{ type = 'update_and_work'; update_map = @{}; tools = @([pscustomobject]@{ tool = 'exec_command'; node_id = 'n'; input = @{} }) }, 'map,client'),
        @('update_and_finish', @{ type = 'update_and_finish'; update_map = @{}; finish_map = @{} }, 'map,map'),
        @('read_map', @{ type = 'read_map'; read_map = @{} }, 'map'),
        @('reopen_update_and_work', @{ type = 'reopen_update_and_work'; reopen_map = @{}; update_map = @{}; tools = @([pscustomobject]@{ tool = 'exec_command'; node_id = 'n'; input = @{} }) }, 'map,map,client'),
        @('finish_map', @{ type = 'finish_map'; finish_map = @{} }, 'map')
    )
    foreach ($case in $sequenceCases) {
        $payload = [pscustomobject]@{
            type = 'function_call'; name = 'taskspace_exec'; arguments = (($case[1]) | ConvertTo-Json -Compress -Depth 8)
        }
        $kinds = @((Get-TaskspaceExecDeclaredCalls $payload).kind) -join ','
        Assert-Equal $kinds $case[2] "sequence observer drifted: $($case[0])"
    }
    $identity = Get-PerformanceCountIdentity `
        ([pscustomobject]@{ tool_call_count = 99; failed_tool_call_count = 99 }) `
        $facts `
        ([pscustomobject]@{ map_count = 1; node_count = 4; edge_count = 3 }) `
        ([pscustomobject]@{ availability = 'missing' }) `
        ([pscustomobject]@{ availability = 'missing' }) `
        ([pscustomobject]@{ same_shape_zero_hit_count = 0 }) `
        'taskspace' `
        $false
    Assert-Equal $identity.valid $true 'R8 count identity fell back to legacy fields'
    Assert-Equal $identity.values.tool_call_count 3 'R8 action total used outer Tool count'

    $zeroHostedArguments = [pscustomobject]@{
        type = 'initialize_and_work'
        initialize_map = [pscustomobject]@{}
        tools = @([pscustomobject]@{ tool = 'exec_command'; node_id = 'inspect'; input = [pscustomobject]@{ cmd = 'pwd' } })
    }
    $zeroHostedResult = [pscustomobject]@{
        client_results = @([pscustomobject]@{ outcome = 'succeeded' })
    }
    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = ($zeroHostedArguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'
                output = ($zeroHostedResult | ConvertTo-Json -Compress -Depth 12)
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    @(
        "INFO codex_core::taskspace_exec: event_name=`"taskspace.exec.response_finalized`" provider_request_id=`"request-1`" provider_response_id=`"response-1`" outer_call_id=`"outer-1`" map_id=`"map-1`" capability_identity=`"$capabilityIdentity`"",
        "INFO codex_core::taskspace_exec: event_name=`"taskspace.exec.completed`" provider_request_id=`"request-1`" provider_response_id=`"response-1`" outer_call_id=`"outer-1`" map_revision=Some(2) capability_identity=`"$capabilityIdentity`""
    ) | Set-Content -LiteralPath (Join-Path $temp 'whale-exec.stderr.log') -Encoding UTF8
    $zeroHostedFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $zeroHostedFacts.availability 'measured' 'omitted Hosted bindings were treated as invalid'
    Assert-Equal $zeroHostedFacts.nested_action_count 2 'zero-Hosted nested action count drifted'
    Assert-Equal $zeroHostedFacts.provider_action_count 0 'omitted Provider actions were not empty'
    Assert-Equal $zeroHostedFacts.node_binding_count 1 'zero-Hosted node binding count drifted'

    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = ($zeroHostedArguments | ConvertTo-Json -Compress -Depth 12)
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'
                output = 'taskspace_exec rejected: preflight: invalid_map_boundary'
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    $rejectedFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $rejectedFacts.availability 'measured' 'canonical rejection was treated as malformed output'
    Assert-Equal $rejectedFacts.failed_action_count 1 'outer Exec rejection was not counted as failure'
    Assert-Equal $rejectedFacts.rejected_call_count 1 'outer Exec rejection total drifted'
    Assert-Equal $rejectedFacts.rejected_preflight_other_call_count 1 'generic preflight rejection was not classified'
    Assert-Equal $rejectedFacts.rejected_preflight_call_count 1 'preflight rejection aggregate drifted'

    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'outer-1'
                arguments = '{"type":"work","tools":['
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'outer-1'
                output = 'taskspace_exec rejected: invalid JSON syntax; no Map or Tool actions executed'
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    $invalidArgumentsFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $invalidArgumentsFacts.availability 'measured' 'protocol rejection invalidated intact observation evidence'
    Assert-Equal $invalidArgumentsFacts.exec_count 1 'malformed outer Exec was not counted'
    Assert-Equal $invalidArgumentsFacts.nested_action_count 0 'unparsed calls were counted as executed actions'
    Assert-Equal $invalidArgumentsFacts.failed_action_count 1 'malformed outer Exec rejection was not counted'
    Assert-Equal $invalidArgumentsFacts.rejected_syntax_call_count 1 'JSON syntax rejection was not classified'
    Assert-Equal $invalidArgumentsFacts.rejected_unknown_call_count 0 'known syntax rejection was classified as unknown'
    if (@($invalidArgumentsFacts.findings | Where-Object { $_ -eq 'exec_arguments_invalid:outer-1' }).Count -ne 1) {
        throw 'malformed outer Exec diagnostic finding missing'
    }

    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'contract-1'; arguments = '{}'
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'contract-1'
                output = 'taskspace_exec rejected: invalid top-level contract: $.initialize_map: value has the wrong JSON type. No Map or Tool actions were executed.'
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'state-1'; arguments = '{}'
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'state-1'
                output = 'taskspace_exec rejected: Tool action 0 targeted work node `diagnose` in state `waiting`; incomplete direct parent nodes: ["understand"]. No Map or Tool actions were executed.'
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    $classifiedFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $classifiedFacts.rejected_call_count 2 'classified rejection total drifted'
    Assert-Equal $classifiedFacts.rejected_contract_call_count 1 'top-level contract rejection was not classified'
    Assert-Equal $classifiedFacts.rejected_state_call_count 1 'waiting-node rejection was not classified'
    Assert-Equal $classifiedFacts.rejected_preflight_call_count 1 'state rejection was not included in the preflight aggregate'
    Assert-Equal $classifiedFacts.rejected_unknown_call_count 0 'known rejections were classified as unknown'

    @(
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call'; name = 'taskspace_exec'; call_id = 'transition-1'; arguments = '{}'
            } },
        [pscustomobject]@{ type = 'response_item'; payload = [pscustomobject]@{
                type = 'function_call_output'; call_id = 'transition-1'
                output = 'taskspace_exec rejected: preflight: MapOperationRejected { violations: [Violation { code: TransitionInvalid, subjects: ["analyze"] }] }. No Map or Tool actions were executed.'
            } }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 15 } |
        Set-Content -LiteralPath (Join-Path $temp 'rollout.jsonl') -Encoding UTF8
    $transitionFacts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $transitionFacts.rejected_call_count 1 'TransitionInvalid rejection total drifted'
    Assert-Equal $transitionFacts.rejected_state_call_count 1 'TransitionInvalid was not classified as state'
    Assert-Equal $transitionFacts.rejected_preflight_other_call_count 0 'TransitionInvalid leaked into generic preflight'
    Assert-Equal $transitionFacts.rejected_preflight_call_count 1 'TransitionInvalid was absent from preflight aggregate'
    $transitionMetrics = [pscustomobject]@{}
    Add-TaskspaceExecObservationMetrics $transitionMetrics 'taskspace' $temp | Out-Null
    Assert-Equal $transitionMetrics.taskspace_exec_rejected_call_count 1 'default metrics omitted TaskSpace Exec rejection total'
    Assert-Equal $transitionMetrics.taskspace_exec_rejected_state_call_count 1 'default metrics omitted TaskSpace Exec state rejection'
    Assert-Equal $transitionMetrics.taskspace_exec_rejected_preflight_call_count 1 'default metrics omitted TaskSpace Exec preflight aggregate'

    $wire = Get-Content -Raw -Encoding UTF8 (Join-Path $temp 'provider-wire-trace.jsonl') | ConvertFrom-Json
    $wire.taskspace_capability_identity = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
    $wire | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $temp 'provider-wire-trace.jsonl') -Encoding UTF8
    $capabilityMismatch = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $capabilityMismatch.availability 'incomparable' 'capability mismatch did not fail closed'
    if (@($capabilityMismatch.findings | Where-Object { $_ -eq 'capability_identity_mismatch:request-1' }).Count -ne 1) {
        throw 'capability mismatch finding missing'
    }
    $wire.taskspace_capability_identity = $capabilityIdentity
    $wire | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $temp 'provider-wire-trace.jsonl') -Encoding UTF8

    (Get-Content -Raw -Encoding UTF8 (Join-Path $temp 'whale-exec.stderr.log')).Replace(
        'provider_request_id="request-1"', 'provider_request_id="unknown"'
    ) | Set-Content -LiteralPath (Join-Path $temp 'whale-exec.stderr.log') -Encoding UTF8
    $mismatch = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $mismatch.availability 'incomparable' 'unknown request identity did not fail closed'
    if (@($mismatch.findings | Where-Object { $_ -eq 'trace_request_missing_from_canonical:unknown' }).Count -ne 1) {
        throw 'request mismatch finding missing'
    }
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force
}

Write-Host 'TaskSpace Exec observation tests passed'
