param([string]$OutputRoot = '')

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $repoRoot 'target/r6-f5-bootstrap-probe-selftest'
}
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function New-Payload {
    param([string]$Arm, [string]$Marker)
    $finish = [ordered]@{ node_id = 'finish' }
    if ($Arm -eq 'A') { $finish.goal = $Marker }
    $arguments = [ordered]@{
        action = 'initialize_map'
        root = [ordered]@{ node_id = 'root'; goal = $Marker }
        initial_work_node = [ordered]@{ node_id = 'inspect'; goal = $Marker }
        finish = $finish
        additional_work_nodes = @()
        edges = @(
            [ordered]@{ from = 'root'; to = 'inspect' },
            [ordered]@{ from = 'inspect'; to = 'finish' }
        )
        continuation = [ordered]@{
            kind = 'actions'
            actions = @([ordered]@{ tool_name = 'exec_command'; arguments = [ordered]@{ cmd = 'pwd' } })
        }
    }
    [ordered]@{
        choices = @([ordered]@{
                message = [ordered]@{
                    content = $null
                    reasoning_content = 'private-reasoning'
                    tool_calls = @([ordered]@{
                            type = 'function'
                            function = [ordered]@{
                                name = 'taskspace_control'
                                arguments = $arguments | ConvertTo-Json -Depth 30 -Compress
                            }
                        })
                }
            })
        usage = [ordered]@{
            prompt_tokens = 100
            prompt_cache_hit_tokens = 80
            prompt_cache_miss_tokens = 20
            completion_tokens = 30
            total_tokens = 130
        }
    }
}

$marker = 'SENSITIVE-FIXTURE-TEXT-MUST-NOT-BE-LOGGED'
$responses = @()
foreach ($repeat in 1..3) {
    foreach ($arm in @('A', 'B', 'C')) {
        foreach ($sample in @('simple', 'complex')) {
            $responses += [ordered]@{
                arm = $arm
                sample = $sample
                repeat = $repeat
                http_status = 200
                payload = New-Payload $arm $marker
            }
        }
    }
}
$fixturePath = Join-Path $OutputRoot 'fixture.json'
$resultPath = Join-Path $OutputRoot 'provider-capability.json'
[System.IO.File]::WriteAllText($fixturePath, ([ordered]@{ responses = $responses } | ConvertTo-Json -Depth 50), [System.Text.UTF8Encoding]::new($false))

& (Join-Path $PSScriptRoot 'probe-r6-f5-bootstrap-contract.ps1') -FixturePath $fixturePath -OutputPath $resultPath -Repeat 3
if ($LASTEXITCODE -ne 0) { throw "probe self-test exited with $LASTEXITCODE" }
$resultText = Get-Content -Raw -LiteralPath $resultPath
$result = $resultText | ConvertFrom-Json -Depth 80

Assert-True ($result.diagnostic.infrastructure_valid -eq $true) 'probe infrastructure was not valid'
Assert-True ([string]$result.diagnostic.attribution -eq 'schema_breadth_supported') 'probe attribution was unexpected'
Assert-True (@($result.events).Count -eq 18) 'probe did not emit 18 observations'
Assert-True (@($result.events | Where-Object { $_.request.tool_choice_kind -ne 'named_function' -or $_.request.thinking_type -ne 'disabled' }).Count -eq 0) 'probe request shape diverged from production named control behavior'
$a = @($result.summaries | Where-Object arm -eq 'A')[0]
$b = @($result.summaries | Where-Object arm -eq 'B')[0]
$c = @($result.summaries | Where-Object arm -eq 'C')[0]
Assert-True ($a.finish_goal_count -eq 6 -and $b.finish_goal_count -eq 0 -and $c.finish_goal_count -eq 0) 'finish.goal counts were not classified'
Assert-True ($a.field_error_count -eq 6 -and $b.field_error_count -eq 0 -and $c.field_error_count -eq 0) 'field errors were not classified'
Assert-True ($a.duration_ms.total -eq 6 -and $a.duration_ms.mean -eq 1 -and $a.duration_ms.median -eq 1) 'duration statistics were not aggregated'
Assert-True ($a.request2plus_cache.request_count -eq 4 -and $a.request2plus_cache.hit_rate -eq 0.8) 'request2+ cache statistics were not aggregated'
Assert-True ($resultText -notmatch [regex]::Escape($marker)) 'redacted node goal leaked into result artifact'
Assert-True ($resultText -notmatch 'private-reasoning') 'reasoning text leaked into result artifact'
Assert-True (@($result.source_builder.full_action_variants).Count -ge 7) 'production builder did not export the full lifecycle schema'
$armA = @($result.arm_contracts | Where-Object arm -eq 'A')[0]
$armB = @($result.arm_contracts | Where-Object arm -eq 'B')[0]
$armC = @($result.arm_contracts | Where-Object arm -eq 'C')[0]
Assert-True ($armB.schema_bytes -lt $armA.schema_bytes) 'bootstrap projection did not reduce the full schema'
Assert-True ($armB.description_sha256 -eq $armA.description_sha256) 'arm B changed the generic description'
Assert-True ($armC.description_sha256 -ne $armB.description_sha256) 'arm C did not isolate description salience'
Assert-True ((Get-Content -LiteralPath (Join-Path $OutputRoot 'probe-events.jsonl')).Count -eq 18) 'event log did not contain 18 lines'

$refutationResponses = @($responses | ForEach-Object {
        [ordered]@{
            arm = $_.arm
            sample = $_.sample
            repeat = $_.repeat
            http_status = 200
            payload = New-Payload 'A' $marker
        }
    })
$refutationFixturePath = Join-Path $OutputRoot 'refutation-fixture.json'
$refutationResultPath = Join-Path $OutputRoot 'refutation-result.json'
[System.IO.File]::WriteAllText($refutationFixturePath, ([ordered]@{ responses = $refutationResponses } | ConvertTo-Json -Depth 50), [System.Text.UTF8Encoding]::new($false))
& (Join-Path $PSScriptRoot 'probe-r6-f5-bootstrap-contract.ps1') -FixturePath $refutationFixturePath -OutputPath $refutationResultPath -Repeat 3
if ($LASTEXITCODE -ne 0) { throw "refutation probe self-test exited with $LASTEXITCODE" }
$refutationResult = Get-Content -Raw -LiteralPath $refutationResultPath | ConvertFrom-Json -Depth 80
Assert-True ([string]$refutationResult.diagnostic.attribution -eq 'refuted_schema_breadth_and_description_salience') 'probe did not classify the refutation branch'
Assert-True ([string]$refutationResult.diagnostic.h008_evidence_gate -eq 'satisfied') 'refutation did not satisfy the H-008 evidence gate'

Write-Host 'R6 F5 bootstrap contract probe self-test passed.'
