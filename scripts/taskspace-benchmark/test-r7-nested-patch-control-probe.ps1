$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$runRoot = Join-Path $repoRoot 'target/r7-nested-patch-control-probe-selftest'
$fixturePath = Join-Path $runRoot 'fixture.json'
$resultPath = Join-Path $runRoot 'result.json'
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null

function New-Response {
    param([string]$Arguments, [string]$ToolName = 'taskspace_control')
    [ordered]@{
        choices = @([ordered]@{
                message = [ordered]@{
                    tool_calls = @([ordered]@{
                            function = [ordered]@{ name = $ToolName; arguments = $Arguments }
                        })
                }
            })
        usage = [ordered]@{ prompt_tokens = 10; prompt_cache_hit_tokens = 5; completion_tokens = 3 }
    }
}

$current = '{"action":"complete_then_continue","expected_revision":2,"current_node_id":"explore","next_node_id":"fix","continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","arguments":{"input":"patch"}}}}'
$flat = '{"action":"complete_then_continue","expected_revision":2,"current_node_id":"explore","next_node_id":"fix","continuation":{"kind":"patch_then_actions","patch":{"tool_name":"apply_patch","input":"patch"}}}'
$continuationPatchInput = '{"action":"complete_then_continue","expected_revision":2,"current_node_id":"explore","next_node_id":"fix","continuation":{"kind":"patch_then_actions","actions":[],"patch_input":"patch"}}'
$topLevel = '{"action":"complete_then_patch","expected_revision":2,"current_node_id":"explore","next_node_id":"fix","patch_input":"patch"}'
$responses = @()
for ($repeat = 1; $repeat -le 2; $repeat++) {
    $responses += [ordered]@{ arm = 'current_large'; repeat = $repeat; http_status = 200; payload = New-Response ($current + '}') }
    $responses += [ordered]@{ arm = 'flat_large'; repeat = $repeat; http_status = 200; payload = New-Response $flat }
    $responses += [ordered]@{ arm = 'current_short'; repeat = $repeat; http_status = 200; payload = New-Response $current }
    $responses += [ordered]@{ arm = 'direct_large'; repeat = $repeat; http_status = 200; payload = New-Response '{"input":"patch"}' 'apply_patch' }
    $responses += [ordered]@{ arm = 'continuation_patch_input_large'; repeat = $repeat; http_status = 200; payload = New-Response $continuationPatchInput }
    $responses += [ordered]@{ arm = 'control_top_level_large'; repeat = $repeat; http_status = 200; payload = New-Response $topLevel }
}
[System.IO.File]::WriteAllText(
    $fixturePath,
    ([ordered]@{ responses = $responses } | ConvertTo-Json -Depth 40),
    [System.Text.UTF8Encoding]::new($false)
)

& (Join-Path $PSScriptRoot 'probe-r7-nested-patch-control.ps1') `
    -Repeat 2 -FixturePath $fixturePath -OutputPath $resultPath
if ($LASTEXITCODE -ne 0) { throw "probe fixture failed with exit $LASTEXITCODE" }
$result = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json -Depth 80
if ($result.schema_version -ne 'r7-nested-patch-control-probe-v1') { throw 'unexpected schema version' }
if ($result.privacy.raw_arguments_recorded) { throw 'probe must not retain raw arguments' }
$currentSummary = @($result.summaries | Where-Object arm -eq 'current_large')[0]
$flatSummary = @($result.summaries | Where-Object arm -eq 'flat_large')[0]
$shortSummary = @($result.summaries | Where-Object arm -eq 'current_short')[0]
$directSummary = @($result.summaries | Where-Object arm -eq 'direct_large')[0]
$continuationPatchInputSummary = @($result.summaries | Where-Object arm -eq 'continuation_patch_input_large')[0]
$topLevelSummary = @($result.summaries | Where-Object arm -eq 'control_top_level_large')[0]
if ($currentSummary.json_valid -ne 0 -or $currentSummary.trailing_characters -ne 2) { throw 'current_large fixture classification failed' }
if ($flatSummary.json_valid -ne 2 -or $shortSummary.json_valid -ne 2) { throw 'valid fixture classification failed' }
if ($directSummary.expected_shape_valid -ne 2) { throw 'direct patch fixture classification failed' }
if ($continuationPatchInputSummary.expected_shape_valid -ne 2) { throw 'continuation patch input fixture classification failed' }
if ($topLevelSummary.expected_shape_valid -ne 2) { throw 'top-level patch fixture classification failed' }

Write-Host 'R7 nested patch control probe selftest passed'
