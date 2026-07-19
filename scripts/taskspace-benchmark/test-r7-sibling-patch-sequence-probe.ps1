$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$runRoot = Join-Path $repoRoot 'target/r7-sibling-patch-sequence-probe-selftest'
$fixturePath = Join-Path $runRoot 'fixture.json'
$resultPath = Join-Path $runRoot 'result.json'
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null

function New-Call {
    param([string]$Name, [string]$Arguments)
    [ordered]@{ function = [ordered]@{ name = $Name; arguments = $Arguments } }
}

function New-Response {
    param($Calls)
    [ordered]@{
        choices = @([ordered]@{ message = [ordered]@{ tool_calls = @($Calls) } })
        usage = [ordered]@{ prompt_tokens = 10; prompt_cache_hit_tokens = 5; completion_tokens = 3 }
    }
}

$control = '{"action":"complete_then_continue","expected_revision":2,"current_node_id":"explore","next_node_id":"fix","continuation":"next_apply_patch"}'
$responses = @(
    [ordered]@{
        repeat = 1
        http_status = 200
        payload = New-Response @(
            (New-Call 'taskspace_control' $control),
            (New-Call 'apply_patch' '{"input":"patch"}')
        )
    },
    [ordered]@{
        repeat = 2
        http_status = 200
        payload = New-Response @(
            (New-Call 'apply_patch' '{"input":"patch"}'),
            (New-Call 'taskspace_control' $control)
        )
    }
)
[System.IO.File]::WriteAllText(
    $fixturePath,
    ([ordered]@{ responses = $responses } | ConvertTo-Json -Depth 40),
    [System.Text.UTF8Encoding]::new($false)
)

& (Join-Path $PSScriptRoot 'probe-r7-sibling-patch-sequence.ps1') `
    -Repeat 2 -FixturePath $fixturePath -OutputPath $resultPath
if ($LASTEXITCODE -ne 0) { throw "probe fixture failed with exit $LASTEXITCODE" }
$result = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json -Depth 80
if ($result.schema_version -ne 'r7-sibling-patch-sequence-probe-v1') { throw 'unexpected schema version' }
if ($result.privacy.raw_arguments_recorded) { throw 'probe must not retain raw arguments' }
if ($result.arm -ne 'sibling_control_first') { throw 'unexpected default arm' }
if ($result.summary.expected_call_names_match -ne 1) { throw 'call order classification failed' }
if ($result.summary.control_shape_valid -ne 2) { throw 'control shape classification failed' }
if ($result.summary.patch_json_valid -ne 2) { throw 'patch JSON classification failed' }

Write-Host 'R7 sibling patch sequence probe selftest passed'
