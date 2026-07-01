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
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"start_task\"}"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $ordinaryBeforeBinding

$bindingFirst = Join-Path $RunRoot "binding-first.jsonl"
@(
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"start_task\"}"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"shell_command","arguments":"{\"command\":\"rg --files\"}"}}'
) | Set-Content -Encoding UTF8 -LiteralPath $bindingFirst

$largeBindingFirst = Join-Path $RunRoot "large-binding-first.jsonl"
$writer = [System.IO.StreamWriter]::new($largeBindingFirst, $false, [System.Text.UTF8Encoding]::new($false))
try {
    $writer.WriteLine('{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"start_task\"}"}}')
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

Write-Host "PASS: R4 metrics extractor large rollout gate passed"
