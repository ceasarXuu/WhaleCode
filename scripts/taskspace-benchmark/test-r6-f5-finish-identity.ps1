param([string]$OutputRoot = '')

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $repoRoot 'target/r6-f5-finish-identity-selftest'
}
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function New-Payload {
    param([string]$Arm, [bool]$IdentityError, [string]$Marker, [bool]$CommonError = $false)
    $arguments = [ordered]@{
        action = 'initialize_map'
        root = [ordered]@{ node_id = 'root'; goal = $Marker }
        initial_work_node = [ordered]@{ node_id = 'inspect'; goal = $Marker }
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
    if ($Arm -eq 'D') {
        $arguments.finish = [ordered]@{ node_id = 'finish' }
        if ($IdentityError) { $arguments.finish.goal = $Marker }
    } elseif ($Arm -eq 'E') {
        $arguments.finish_identity = [ordered]@{ id = 'finish' }
        if ($IdentityError) { $arguments.finish_identity.goal = $Marker }
    } elseif ($IdentityError) {
        $arguments.finish_identity = [ordered]@{ id = 'finish'; goal = $Marker }
    } else {
        $arguments.finish_identity = 'finish'
    }
    if ($CommonError) { $arguments.edges[0].goal = $Marker }
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

function Invoke-Scenario {
    param([string]$Name, [string]$Mode, [string]$Marker)
    $scenarioRoot = Join-Path $OutputRoot $Name
    New-Item -ItemType Directory -Force -Path $scenarioRoot | Out-Null
    $responses = @()
    foreach ($repeat in 1..3) {
        foreach ($arm in @('D', 'E', 'F')) {
            foreach ($sample in @('simple', 'complex')) {
                $identityError = if ($Mode -eq 'scalar') {
                    $arm -ne 'F'
                } elseif ($Mode -eq 'naming') {
                    $arm -eq 'D'
                } else {
                    $true
                }
                $commonError = $Mode -eq 'none' -and $arm -eq 'F' -and $sample -eq 'simple' -and $repeat -eq 1
                $responses += [ordered]@{
                    arm = $arm
                    sample = $sample
                    repeat = $repeat
                    http_status = 200
                    payload = New-Payload $arm $identityError $Marker $commonError
                }
            }
        }
    }
    $fixturePath = Join-Path $scenarioRoot 'fixture.json'
    $resultPath = Join-Path $scenarioRoot 'provider-capability.json'
    [System.IO.File]::WriteAllText($fixturePath, ([ordered]@{ responses = $responses } | ConvertTo-Json -Depth 50), [System.Text.UTF8Encoding]::new($false))
    & (Join-Path $PSScriptRoot 'probe-r6-f5-finish-identity.ps1') -FixturePath $fixturePath -OutputPath $resultPath -Repeat 3
    if ($LASTEXITCODE -ne 0) { throw "$Name scenario exited with $LASTEXITCODE" }
    [ordered]@{
        result = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json -Depth 80
        result_text = Get-Content -Raw -LiteralPath $resultPath
        event_count = (Get-Content -LiteralPath (Join-Path $scenarioRoot 'probe-events.jsonl')).Count
    }
}

$marker = 'SENSITIVE-FINISH-FIXTURE-MUST-NOT-BE-LOGGED'
$scalar = Invoke-Scenario 'scalar' 'scalar' $marker
$scalarD = @($scalar.result.summaries | Where-Object arm -eq 'D')[0]
$scalarE = @($scalar.result.summaries | Where-Object arm -eq 'E')[0]
$scalarF = @($scalar.result.summaries | Where-Object arm -eq 'F')[0]
Assert-True ($scalar.result.diagnostic.infrastructure_valid -eq $true) 'scalar infrastructure was invalid'
Assert-True ([string]$scalar.result.diagnostic.attribution -eq 'scalar_identity_supported') 'scalar attribution was unexpected'
Assert-True ([string]$scalar.result.diagnostic.winning_arm -eq 'F') 'scalar winner was not F'
Assert-True ($scalarD.identity_error_count -eq 6 -and $scalarE.identity_error_count -eq 6 -and $scalarF.identity_error_count -eq 0) 'scalar identity errors were not classified'
Assert-True ($scalarF.common_field_error_count -eq 0 -and $scalarF.valid_count -eq 6) 'scalar candidate was not fully valid'
Assert-True ($scalar.event_count -eq 18) 'scalar event log count was wrong'
Assert-True ($scalar.result_text -notmatch [regex]::Escape($marker)) 'sensitive goal leaked into scalar result'
Assert-True ($scalar.result_text -notmatch 'private-reasoning') 'reasoning leaked into scalar result'

$naming = Invoke-Scenario 'naming' 'naming' $marker
$namingE = @($naming.result.summaries | Where-Object arm -eq 'E')[0]
Assert-True ([string]$naming.result.diagnostic.attribution -eq 'identity_naming_supported') 'naming attribution was unexpected'
Assert-True ([string]$naming.result.diagnostic.winning_arm -eq 'E') 'naming winner was not E'
Assert-True ($namingE.identity_error_count -eq 0 -and $namingE.valid_count -eq 6) 'named object candidate was not valid'

$none = Invoke-Scenario 'none' 'none' $marker
$noneF = @($none.result.summaries | Where-Object arm -eq 'F')[0]
$noneError = @($none.result.events | Where-Object { $_.arm -eq 'F' -and $_.sample -eq 'simple' -and $_.repeat -eq 1 })[0]
Assert-True ([string]$none.result.diagnostic.attribution -eq 'no_candidate_reduced_identity_errors') 'no-candidate attribution was unexpected'
Assert-True ([string]$none.result.diagnostic.finish_identity_evidence_gate -eq 'not_satisfied') 'no-candidate case incorrectly passed the identity gate'
Assert-True ($noneF.common_field_error_count -eq 1) 'common regression was not counted'
Assert-True (@($noneError.response.arguments.common_field_errors) -contains 'unexpected:edges[0].goal') 'common regression path was not preserved'

$contracts = @($scalar.result.arm_contracts)
Assert-True (@($contracts | Select-Object -ExpandProperty schema_sha256 | Sort-Object -Unique).Count -eq 3) 'arm schemas were not distinct'
Assert-True (@($contracts | Select-Object -ExpandProperty description_sha256 | Sort-Object -Unique).Count -eq 1) 'arm descriptions diverged'
Assert-True ((@($contracts | Where-Object arm -eq 'D')[0].identity_shape) -eq 'finish.object.node_id') 'D shape was wrong'
Assert-True ((@($contracts | Where-Object arm -eq 'E')[0].identity_shape) -eq 'finish_identity.object.id') 'E shape was wrong'
Assert-True ((@($contracts | Where-Object arm -eq 'F')[0].identity_shape) -eq 'finish_identity.string') 'F shape was wrong'
$contractD = @($contracts | Where-Object arm -eq 'D')[0]
$contractE = @($contracts | Where-Object arm -eq 'E')[0]
$contractF = @($contracts | Where-Object arm -eq 'F')[0]
Assert-True ($contractD.identity_type -eq 'object' -and @($contractD.identity_required) -contains 'node_id') 'D actual identity schema was wrong'
Assert-True ($contractE.identity_type -eq 'object' -and @($contractE.identity_required) -contains 'id') 'E actual identity schema was wrong'
Assert-True ($contractF.identity_type -eq 'string' -and @($contractF.identity_required).Count -eq 0) 'F actual identity schema was wrong'
Assert-True (@($contractD.initialize_required) -contains 'finish') 'D initialize required did not contain finish'
Assert-True (@($contractE.initialize_required) -contains 'finish_identity' -and @($contractF.initialize_required) -contains 'finish_identity') 'E/F initialize required did not contain finish_identity'

Write-Host 'R6 F5 finish identity probe self-test passed.'
