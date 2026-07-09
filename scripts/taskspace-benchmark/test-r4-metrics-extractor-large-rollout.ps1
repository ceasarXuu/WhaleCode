param(
    [string]$RunRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")

if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target\r4-metrics-extractor-large-rollout"
}
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null

function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)]$Expected,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if ($Actual -ne $Expected) {
        throw "$Message actual=$Actual expected=$Expected"
    }
}

$ordinaryBeforeBinding = Join-Path $RunRoot "ordinary-before-binding.jsonl"
@(
    '{"type":"response_item","payload":{"type":"message","role":"user","content":[]}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"shell_command","arguments":"{\"command\":\"rg --files\"}"}}',
    '{"type":"event_msg","payload":{"type":"lease_created","mapId":"map-1","nodeId":"node-1","leaseId":"lease-1"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $ordinaryBeforeBinding

$bindingFirst = Join-Path $RunRoot "binding-first.jsonl"
@(
    '{"type":"event_msg","payload":{"type":"lease_created","mapId":"map-1","nodeId":"node-1","leaseId":"lease-1"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"shell_command","arguments":"{\"command\":\"rg --files\"}"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $bindingFirst

$largeBindingFirst = Join-Path $RunRoot "large-binding-first.jsonl"
$writer = [System.IO.StreamWriter]::new($largeBindingFirst, $false, [System.Text.UTF8Encoding]::new($false))
try {
    $writer.WriteLine('{"type":"event_msg","payload":{"type":"taskspace_trace_event_recorded","kind":"mechanical_blank_map_initialized","taskId":"task-1","mapId":"map-1","nodeId":"node-1"}}')
    for ($i = 0; $i -lt 20000; $i++) {
        $writer.WriteLine('{"type":"event_msg","payload":{"type":"message","text":"' + ('x' * 200) + '"}}')
    }
} finally {
    $writer.Dispose()
}

Assert-Equal (Test-TaskspaceOrdinaryToolBeforeBindingInRollout $ordinaryBeforeBinding) $true "ordinary tool before binding was not detected"
Assert-Equal (Test-TaskspaceOrdinaryToolBeforeBindingInRollout $bindingFirst) $false "binding-first rollout was incorrectly flagged"
Assert-Equal (Test-TaskspaceOrdinaryToolBeforeBindingInRollout $largeBindingFirst) $false "large binding-first rollout was incorrectly flagged"

$toolStatsPath = Join-Path $RunRoot "tool-stats.jsonl"
@(
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{}","call_id":"control-1"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"control-1","output":"ok"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"shell_command","arguments":"{\"command\":\"rg --files\"}","call_id":"read-1"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"read-1","output":"Exit code: 0\nOutput:\nok"}}',
    '{"type":"response_item","payload":{"type":"custom_tool_call","name":"apply_patch","input":"*** Begin Patch","call_id":"patch-1"}}',
    '{"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"patch-1","output":"{\"output\":\"Success\",\"metadata\":{\"exit_code\":0}}"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"shell_command","arguments":"{\"command\":\"bash run_pipeline.sh\"}","call_id":"test-1"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"test-1","output":"Tool call failed before producing a result. local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $toolStatsPath
$toolStats = Get-TaskspaceRolloutToolStats $toolStatsPath
Assert-Equal $toolStats.Completed 3 "rollout ordinary tool calls were not counted"
Assert-Equal $toolStats.Failed 1 "rollout failed tool calls were not counted"
Assert-Equal $toolStats.Control 1 "rollout taskspace_control count was not separated"

$obsStats = Get-TaskspaceObservabilityToolStats ([pscustomobject]@{
        nodes = @(
            [pscustomobject]@{ results = @([pscustomobject]@{ kind = "main_tool_call" }, [pscustomobject]@{ kind = "result" }) },
            [pscustomobject]@{ results = @([pscustomobject]@{ kind = "main_tool_call" }) }
        )
    })
Assert-Equal $obsStats.Completed 2 "observability main_tool_call fallback did not count results"
Assert-Equal $obsStats.Availability "observability_results" "observability fallback did not record source"

$obsRuntimeStats = Get-TaskspaceObservabilityToolStats ([pscustomobject]@{
        nodes = @()
        summary = [pscustomobject]@{
            runtimeEventCounts = [pscustomobject]@{ function_call = 4; custom_tool_call = 1 }
        }
    })
Assert-Equal $obsRuntimeStats.Completed 5 "observability runtime-count fallback did not count calls"
Assert-Equal $obsRuntimeStats.Availability "observability_runtime_counts" "observability runtime fallback did not record source"

$completionDir = Join-Path $RunRoot "completion"
New-Item -ItemType Directory -Force -Path $completionDir | Out-Null
$standardFinal = Join-Path $completionDir "standard-final.jsonl"
@(
    '{"type":"item.completed","item":{"type":"command_execution","status":"completed"}}',
    '{"type":"item.completed","item":{"type":"agent_message","text":"Done."}}',
    '{"type":"turn.completed","usage":{"input_tokens":10}}'
) | Set-Content -Encoding UTF8 -LiteralPath $standardFinal
$taskspaceFinal = Join-Path $completionDir "taskspace-final.jsonl"
@(
    '{"type":"item.completed","item":{"type":"agent_message","text":"Done."}}',
    '{"type":"item.completed","item":{"type":"error","message":"TaskSpaceProviderResponseActionabilityV1 actionability=final_candidate recovery_action=none"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $taskspaceFinal
$taskspaceRejected = Join-Path $completionDir "taskspace-rejected.jsonl"
@(
    '{"type":"item.completed","item":{"type":"agent_message","text":"Done."}}',
    '{"type":"item.completed","item":{"type":"error","message":"TaskSpaceProviderResponseActionabilityV1 actionability=final_rejected recovery_action=none"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $taskspaceRejected
$messageThenTool = Join-Path $completionDir "message-then-tool.jsonl"
@(
    '{"type":"item.completed","item":{"type":"agent_message","text":"Let me verify."}}',
    '{"type":"item.completed","item":{"type":"command_execution","status":"completed"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $messageThenTool

Assert-Equal (Get-TaskspaceAgentCompletionEvidence $standardFinal "standard").agent_final_observed $true "terminal standard Agent message was not detected"
Assert-Equal (Get-TaskspaceAgentCompletionEvidence $taskspaceFinal "taskspace").agent_final_observed $true "TaskSpace final candidate was not detected"
Assert-Equal (Get-TaskspaceAgentCompletionEvidence $taskspaceRejected "taskspace").agent_final_observed $false "rejected TaskSpace final was classified complete"
Assert-Equal (Get-TaskspaceAgentCompletionEvidence $messageThenTool "standard").agent_final_observed $false "nonterminal Agent progress message was classified complete"

Write-Host "PASS: R4 metrics extractor large rollout gate passed"
