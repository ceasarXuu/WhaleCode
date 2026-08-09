Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
. (Join-Path $root 'lib/performance-observation.ps1')

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) { throw "$Message expected=$Expected actual=$Actual" }
}

$temp = Join-Path ([IO.Path]::GetTempPath()) "taskspace-exec-observation-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $temp -Force | Out-Null
try {
    $arguments = [pscustomobject]@{
        calls = @(
            [pscustomobject]@{ tool = 'initialize_map'; arguments = [pscustomobject]@{} },
            [pscustomobject]@{ tool = 'exec_command'; node_id = 'inspect'; arguments = [pscustomobject]@{ cmd = 'pwd' } },
            [pscustomobject]@{ tool = 'apply_patch'; node_id = 'fix'; arguments = [pscustomobject]@{ patch = 'x' } }
        )
        hosted_bindings = @(
            [pscustomobject]@{ tool = 'web_search'; node_ids = @('inspect', 'fix') }
        )
    }
    $result = [pscustomobject]@{
        kind = 'taskspace_exec_result'; status = 'completed'; outer_call_id = 'outer-1'
        client_results = @(
            [pscustomobject]@{ outcome = 'succeeded' },
            [pscustomobject]@{ outcome = 'failed' }
        )
        hosted_results = @([pscustomobject]@{ outcome = 'succeeded' })
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
    @(
        'INFO codex_core::taskspace_exec: event_name="taskspace.exec.response_finalized" provider_request_id="request-1" provider_response_id="response-1" outer_call_id="outer-1" map_id="map-1"',
        'INFO codex_core::taskspace_exec: event_name="taskspace.exec.completed" provider_request_id="request-1" provider_response_id="response-1" outer_call_id="outer-1" map_revision=Some(2)'
    ) | Set-Content -LiteralPath (Join-Path $temp 'whale-exec.stderr.log') -Encoding UTF8

    $facts = Get-TaskspaceExecObservation $temp $null
    Assert-Equal $facts.availability 'measured' 'valid evidence was not comparable'
    Assert-Equal $facts.exec_count 1 'outer Exec count drifted'
    Assert-Equal $facts.map_operation_count 1 'Map operation count drifted'
    Assert-Equal $facts.client_action_count 2 'client action count drifted'
    Assert-Equal $facts.hosted_binding_count 1 'Hosted binding count drifted'
    Assert-Equal $facts.node_binding_count 4 'node binding count drifted'
    Assert-Equal $facts.failed_action_count 1 'failure count drifted'
    Assert-Equal $facts.correlated_request_count 1 'request identity was not joined'
    Assert-Equal $facts.correlated_outer_call_count 1 'outer call identity was not joined'
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
