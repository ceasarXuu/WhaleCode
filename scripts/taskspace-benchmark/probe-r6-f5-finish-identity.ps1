param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$Endpoint = 'https://api.deepseek.com/chat/completions',
    [string]$OutputPath = '',
    [ValidateRange(1, 20)][int]$Repeat = 3,
    [string]$FixturePath = ''
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'probe-r6-f5-bootstrap-contract.ps1') `
    -Model $Model -Endpoint $Endpoint -OutputPath $OutputPath -Repeat $Repeat -FixturePath $FixturePath -LibraryOnly

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $runId = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff')
    $OutputPath = Join-Path $repoRoot "target/r6-f5-finish-identity-ab/$runId/provider-capability.json"
}

function Get-BootstrapChatTool {
    param($ResponsesTool)
    $tool = ConvertTo-ChatTool $ResponsesTool
    $initialize = @($tool.function.parameters.anyOf | Where-Object {
            (Get-ActionName $_) -eq 'initialize_map'
        })
    if ($initialize.Count -ne 1) {
        throw "expected exactly one initialize_map variant, got $($initialize.Count)"
    }
    $tool.function.parameters.anyOf = @(Copy-JsonValue $initialize[0])
    $tool
}

function New-ObjectIdentitySchema {
    [pscustomobject][ordered]@{
        type = 'object'
        description = 'Terminal identity only. All executable work, including validation, belongs to Work nodes.'
        properties = [pscustomobject][ordered]@{
            id = [pscustomobject][ordered]@{
                type = 'string'
                description = 'Stable Agent-authored Finish identifier.'
            }
        }
        required = @('id')
        additionalProperties = $false
    }
}

function New-ScalarIdentitySchema {
    [pscustomobject][ordered]@{
        type = 'string'
        description = 'Stable Agent-authored Finish identifier. Terminal identity only.'
    }
}

function Replace-FinishProperty {
    param($Tool, [string]$PropertyName, $PropertySchema)
    $variant = @($Tool.function.parameters.anyOf)[0]
    $properties = [ordered]@{}
    foreach ($property in $variant.properties.PSObject.Properties) {
        if ($property.Name -eq 'finish') {
            $properties[$PropertyName] = $PropertySchema
        } else {
            $properties[$property.Name] = $property.Value
        }
    }
    $variant.properties = [pscustomobject]$properties
    $variant.required = @($variant.required | ForEach-Object {
            if ([string]$_ -eq 'finish') { $PropertyName } else { [string]$_ }
        })
}

function New-FinishIdentityArms {
    param($ResponsesTool)
    $baseline = Get-BootstrapChatTool $ResponsesTool
    $namedObject = Copy-JsonValue $baseline
    Replace-FinishProperty $namedObject 'finish_identity' (New-ObjectIdentitySchema)
    $scalar = Copy-JsonValue $baseline
    Replace-FinishProperty $scalar 'finish_identity' (New-ScalarIdentitySchema)
    [ordered]@{
        D = $baseline
        E = $namedObject
        F = $scalar
    }
}

function Test-JsonObject {
    param($Value)
    $Value -is [pscustomobject] -or $Value -is [System.Collections.IDictionary]
}

$baseArgumentShape = ${function:Get-ArgumentShape}

function Get-FinishArgumentShape {
    param([AllowEmptyString()][string]$RawArguments, [string]$Arm)
    $raw = [ordered]@{
        bytes = Get-Utf8Bytes $RawArguments
        sha256 = Get-Sha256 $RawArguments
    }
    try {
        $arguments = $RawArguments | ConvertFrom-Json -Depth 80
    } catch {
        return [ordered]@{
            raw = $raw
            parsed = $false
            verdict = 'invalid_json'
            identity_errors = @('arguments:not_json')
            common_field_errors = @()
        }
    }

    $identityErrors = [System.Collections.Generic.List[string]]::new()
    $topKeys = @(Get-PropertyNames $arguments)
    $expectedField = if ($Arm -eq 'D') { 'finish' } else { 'finish_identity' }
    $expectedIdField = if ($Arm -eq 'D') { 'node_id' } elseif ($Arm -eq 'E') { 'id' } else { '' }
    foreach ($field in @('finish', 'finish_identity')) {
        if ($field -ne $expectedField -and $topKeys -contains $field) {
            $identityErrors.Add("unexpected:$field")
        }
    }
    if ($topKeys -notcontains $expectedField) {
        $identityErrors.Add("missing:$expectedField")
    }

    $identityValue = $arguments.$expectedField
    $identityKind = if ($null -eq $identityValue) {
        'missing'
    } elseif (Test-JsonObject $identityValue) {
        'object'
    } elseif ($identityValue -is [string]) {
        'string'
    } else {
        'other'
    }
    $identityKeys = @()
    $identityId = ''
    $hasGoal = $false
    $goal = $null
    if ($Arm -in @('D', 'E')) {
        if ($identityKind -ne 'object') {
            $identityErrors.Add("type:$expectedField:not_object")
        } else {
            $identityKeys = @(Get-PropertyNames $identityValue)
            if ($identityKeys -notcontains $expectedIdField) {
                $identityErrors.Add("missing:$expectedField.$expectedIdField")
            } else {
                $identityId = [string]$identityValue.$expectedIdField
            }
            foreach ($key in $identityKeys) {
                if ($key -ne $expectedIdField) {
                    $identityErrors.Add("unexpected:$expectedField.$key")
                }
            }
            $hasGoal = $identityKeys -contains 'goal'
            $goal = Get-StringFingerprint $identityValue.goal
        }
    } else {
        if ($identityKind -ne 'string') {
            $identityErrors.Add('type:finish_identity:not_string')
        } else {
            $identityId = [string]$identityValue
            if ([string]::IsNullOrWhiteSpace($identityId)) {
                $identityErrors.Add('invalid:finish_identity:empty')
            }
        }
    }

    $normalized = Copy-JsonValue $arguments
    foreach ($field in @('finish', 'finish_identity')) {
        $normalized.PSObject.Properties.Remove($field)
    }
    $normalized | Add-Member -NotePropertyName finish -NotePropertyValue (
        [pscustomobject][ordered]@{ node_id = $identityId }
    )
    $common = & $baseArgumentShape ($normalized | ConvertTo-Json -Depth 80 -Compress)
    $commonErrors = @($common.field_errors)
    $allErrors = @($identityErrors) + $commonErrors
    [ordered]@{
        raw = $raw
        parsed = $true
        verdict = if ($allErrors.Count -eq 0) { 'valid_initialize_shape' } else { 'field_error' }
        field_errors = $allErrors
        identity_errors = @($identityErrors)
        common_field_errors = $commonErrors
        top_level_keys = $topKeys
        action = $common.action
        root = $common.root
        initial_work_node = $common.initial_work_node
        identity = [ordered]@{
            expected_field = $expectedField
            expected_kind = if ($Arm -eq 'F') { 'string' } else { 'object' }
            actual_kind = $identityKind
            keys = $identityKeys
            id = Get-StringFingerprint $identityId
            has_goal = $hasGoal
            goal = $goal
        }
        additional_work_node_count = $common.additional_work_node_count
        edge_count = $common.edge_count
        continuation_kind = $common.continuation_kind
        continuation_action_count = $common.continuation_action_count
        continuation_tool_names = $common.continuation_tool_names
    }
}

function Get-FinishResponseObservation {
    param($Payload, [int]$HttpStatus, [string]$Arm)
    $choices = if ($null -eq $Payload) { @() } else { @($Payload.choices) }
    $message = if ($choices.Count -gt 0) { $choices[0].message } else { $null }
    $calls = if ($null -eq $message) { @() } else { @($message.tool_calls) }
    $firstCall = if ($calls.Count -gt 0) { $calls[0] } else { $null }
    $rawArguments = if ($null -ne $firstCall) { [string]$firstCall.function.arguments } else { '' }
    $shape = if ($calls.Count -eq 1 -and [string]$firstCall.function.name -eq 'taskspace_control') {
        Get-FinishArgumentShape $rawArguments $Arm
    } else { $null }
    $reasoning = if ($null -eq $message) { '' } else { [string]$message.reasoning_content }
    $content = if ($null -eq $message) { '' } else { [string]$message.content }
    [ordered]@{
        http_status = $HttpStatus
        tool_call_count = $calls.Count
        first_tool_name = if ($null -eq $firstCall) { '' } else { [string]$firstCall.function.name }
        arguments = $shape
        reasoning = [ordered]@{ present = -not [string]::IsNullOrWhiteSpace($reasoning); bytes = Get-Utf8Bytes $reasoning; sha256 = Get-Sha256 $reasoning }
        content = [ordered]@{ present = -not [string]::IsNullOrWhiteSpace($content); bytes = Get-Utf8Bytes $content; sha256 = Get-Sha256 $content }
        usage = [ordered]@{
            input_tokens = [int64]$Payload.usage.prompt_tokens
            cached_input_tokens = [int64]$Payload.usage.prompt_cache_hit_tokens
            uncached_input_tokens = [int64]$Payload.usage.prompt_cache_miss_tokens
            output_tokens = [int64]$Payload.usage.completion_tokens
            total_tokens = [int64]$Payload.usage.total_tokens
        }
        provider_error = if ($null -eq $Payload.error) { '' } else { [string]$Payload.error.message }
    }
}

$responsesTool = Get-ProductionControlTool
$sourceJson = $responsesTool | ConvertTo-Json -Depth 80 -Compress
$arms = New-FinishIdentityArms $responsesTool
$fixture = if ([string]::IsNullOrWhiteSpace($FixturePath)) { $null } else {
    (Get-Content -Raw -LiteralPath $FixturePath) | ConvertFrom-Json -Depth 80
}
if ($null -eq $fixture) {
    Import-LocalCredentialIfNeeded
    if ([string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) {
        throw 'DEEPSEEK_API_KEY is required for the live F5.0b probe'
    }
}

$samples = Get-F5ProbeSamples
$armOrders = @(@('D', 'E', 'F'), @('E', 'F', 'D'), @('F', 'D', 'E'))
$events = [System.Collections.Generic.List[object]]::new()
for ($repeatIndex = 1; $repeatIndex -le $Repeat; $repeatIndex++) {
    $order = $armOrders[($repeatIndex - 1) % $armOrders.Count]
    $sampleOrder = if ($repeatIndex % 2 -eq 0) { @('complex', 'simple') } else { @('simple', 'complex') }
    foreach ($armName in $order) {
        foreach ($sampleName in $sampleOrder) {
            $tool = $arms[$armName]
            $body = [ordered]@{
                model = $Model
                messages = @(
                    [ordered]@{ role = 'system'; content = Get-F5ProbeSystemPrompt },
                    [ordered]@{ role = 'user'; content = [string]$samples[$sampleName] }
                )
                tools = @($tool)
                tool_choice = [ordered]@{ type = 'function'; function = [ordered]@{ name = 'taskspace_control' } }
                thinking = [ordered]@{ type = 'disabled' }
                stream = $false
                temperature = 0
            }
            $bodyJson = $body | ConvertTo-Json -Depth 80 -Compress
            $toolJson = $tool | ConvertTo-Json -Depth 80 -Compress
            $invocation = Invoke-ProbeRequest $body $fixture $armName $sampleName $repeatIndex
            $observation = Get-FinishResponseObservation $invocation.payload $invocation.status $armName
            $events.Add([ordered]@{
                    event_name = 'r6.f5.finish_identity_observed'
                    arm = $armName
                    sample = $sampleName
                    repeat = $repeatIndex
                    request = [ordered]@{
                        endpoint_kind = if ($null -eq $fixture) { 'deepseek_live' } else { 'fixture' }
                        body_bytes = Get-Utf8Bytes $bodyJson
                        body_sha256 = Get-Sha256 $bodyJson
                        prompt_bytes = Get-Utf8Bytes ([string]$samples[$sampleName])
                        prompt_sha256 = Get-Sha256 ([string]$samples[$sampleName])
                        schema_bytes = Get-Utf8Bytes $toolJson
                        schema_sha256 = Get-Sha256 $toolJson
                        tool_choice_kind = 'named_function'
                        thinking_type = 'disabled'
                    }
                    duration_ms = $invocation.duration_ms
                    response = $observation
                })
        }
    }
}

$summaries = @()
foreach ($armName in @('D', 'E', 'F')) {
    $armEvents = @($events | Where-Object { $_.arm -eq $armName })
    $later = @($armEvents | Where-Object { $_.repeat -gt 1 })
    $laterInput = [double](($later | ForEach-Object { $_.response.usage.input_tokens } | Measure-Object -Sum).Sum)
    $laterCached = [double](($later | ForEach-Object { $_.response.usage.cached_input_tokens } | Measure-Object -Sum).Sum)
    $summaries += [ordered]@{
        arm = $armName
        request_count = $armEvents.Count
        http_200_count = @($armEvents | Where-Object { $_.response.http_status -eq 200 }).Count
        parsed_count = @($armEvents | Where-Object { $_.response.arguments.parsed }).Count
        valid_count = @($armEvents | Where-Object { $_.response.arguments.verdict -eq 'valid_initialize_shape' }).Count
        identity_error_count = @($armEvents | Where-Object { @($_.response.arguments.identity_errors).Count -gt 0 }).Count
        common_field_error_count = @($armEvents | Where-Object { @($_.response.arguments.common_field_errors).Count -gt 0 }).Count
        goal_present_count = @($armEvents | Where-Object { $_.response.arguments.identity.has_goal }).Count
        input_tokens = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.input_tokens })
        cached_input_tokens = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.cached_input_tokens })
        uncached_input_tokens = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.uncached_input_tokens })
        duration_ms = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.duration_ms })
        request2plus_cache = [ordered]@{
            request_count = $later.Count
            input_tokens = $laterInput
            cached_input_tokens = $laterCached
            hit_rate = if ($laterInput -eq 0) { 0 } else { $laterCached / $laterInput }
        }
    }
}

$d = @($summaries | Where-Object arm -eq 'D')[0]
$e = @($summaries | Where-Object arm -eq 'E')[0]
$f = @($summaries | Where-Object arm -eq 'F')[0]
$highThreshold = [Math]::Max(1, (2 * $Repeat) - 1)
$ePass = $e.identity_error_count -le 1 -and $e.common_field_error_count -eq 0
$fPass = $f.identity_error_count -le 1 -and $f.common_field_error_count -eq 0
$attribution = if ($d.identity_error_count -lt $highThreshold) {
    'inconclusive_baseline_not_reproduced'
} elseif ($ePass) {
    'identity_naming_supported'
} elseif ($fPass) {
    'scalar_identity_supported'
} elseif ($e.identity_error_count -ge $highThreshold -and $f.identity_error_count -ge $highThreshold) {
    'no_candidate_reduced_identity_errors'
} else {
    'inconclusive_candidate_distribution'
}
$winningArm = if ($attribution -eq 'identity_naming_supported') { 'E' } elseif ($attribution -eq 'scalar_identity_supported') { 'F' } else { '' }
$infrastructureValid = @($summaries | Where-Object {
        $_.request_count -ne (2 * $Repeat) -or $_.http_200_count -ne $_.request_count -or $_.parsed_count -ne $_.request_count
    }).Count -eq 0
$result = [ordered]@{
    schema_version = 'r6-f5-finish-identity-probe-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    repeat = $Repeat
    source_builder = [ordered]@{
        command = 'cargo run --quiet -p codex-tools --example r6_f5_control_schema'
        responses_tool_bytes = Get-Utf8Bytes $sourceJson
        responses_tool_sha256 = Get-Sha256 $sourceJson
    }
    arm_contracts = @($arms.GetEnumerator() | ForEach-Object {
            $armName = [string]$_.Key
            $variant = @($_.Value.function.parameters.anyOf)[0]
            $identityField = if ($armName -eq 'D') { 'finish' } else { 'finish_identity' }
            $identitySchema = $variant.properties.$identityField
            $json = $_.Value | ConvertTo-Json -Depth 80 -Compress
            [ordered]@{
                arm = $armName
                identity_shape = if ($armName -eq 'D') { 'finish.object.node_id' } elseif ($armName -eq 'E') { 'finish_identity.object.id' } else { 'finish_identity.string' }
                initialize_required = @($variant.required)
                initialize_properties = @(Get-PropertyNames $variant.properties)
                identity_type = [string]$identitySchema.type
                identity_required = @($identitySchema.required | Where-Object { $null -ne $_ })
                identity_properties = @(Get-PropertyNames $identitySchema.properties)
                schema_bytes = Get-Utf8Bytes $json
                schema_sha256 = Get-Sha256 $json
                description_sha256 = Get-Sha256 ([string]$_.Value.function.description)
            }
        })
    summaries = $summaries
    diagnostic = [ordered]@{
        infrastructure_valid = $infrastructureValid
        attribution = $attribution
        winning_arm = $winningArm
        finish_identity_evidence_gate = if ($infrastructureValid -and -not [string]::IsNullOrWhiteSpace($winningArm)) { 'satisfied' } else { 'not_satisfied' }
    }
    events = @($events)
}

$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText($OutputPath, ($result | ConvertTo-Json -Depth 80), [System.Text.UTF8Encoding]::new($false))
$eventLines = @($events | ForEach-Object { $_ | ConvertTo-Json -Depth 80 -Compress })
[System.IO.File]::WriteAllLines((Join-Path $parent 'probe-events.jsonl'), $eventLines, [System.Text.UTF8Encoding]::new($false))
Write-Host "R6F5FinishIdentityProbe: $OutputPath"
Write-Host "Attribution: $attribution"
if (-not $infrastructureValid) { exit 2 }
