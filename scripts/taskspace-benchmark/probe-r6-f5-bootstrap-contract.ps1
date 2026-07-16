param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$Endpoint = 'https://api.deepseek.com/chat/completions',
    [string]$OutputPath = '',
    [ValidateRange(1, 20)][int]$Repeat = 3,
    [string]$FixturePath = '',
    [switch]$LibraryOnly
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (-not $LibraryOnly -and [string]::IsNullOrWhiteSpace($OutputPath)) {
    $runId = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff')
    $OutputPath = Join-Path $repoRoot "target/r6-f5-bootstrap-ab/$runId/provider-capability.json"
}

function Get-Sha256 {
    param([AllowEmptyString()][string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [System.Security.Cryptography.SHA256]::HashData($bytes)
    [Convert]::ToHexString($hash).ToLowerInvariant()
}

function Get-Utf8Bytes {
    param([AllowEmptyString()][string]$Text)
    [System.Text.Encoding]::UTF8.GetByteCount($Text)
}

function Get-NumberStats {
    param([double[]]$Values)
    if ($Values.Count -eq 0) {
        return [ordered]@{ total = 0; mean = 0; median = 0 }
    }
    $sorted = @($Values | Sort-Object)
    $total = [double](($sorted | Measure-Object -Sum).Sum)
    $middle = [int][Math]::Floor($sorted.Count / 2)
    $median = if ($sorted.Count % 2 -eq 1) {
        [double]$sorted[$middle]
    } else {
        ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2
    }
    [ordered]@{
        total = $total
        mean = $total / $sorted.Count
        median = $median
    }
}

function Get-PropertyNames {
    param($Value)
    if ($null -eq $Value) { return @() }
    @($Value.PSObject.Properties.Name | Sort-Object)
}

function Copy-JsonValue {
    param($Value)
    (($Value | ConvertTo-Json -Depth 80 -Compress) | ConvertFrom-Json -Depth 80)
}

function Import-LocalCredentialIfNeeded {
    if (-not [string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) { return }
    $envPath = Join-Path $repoRoot '.env.local'
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) { return }
    foreach ($line in Get-Content -LiteralPath $envPath) {
        if ($line -match '^\s*DEEPSEEK_API_KEY\s*=\s*(.+?)\s*$') {
            $value = $Matches[1].Trim().Trim('"').Trim("'")
            if (-not [string]::IsNullOrWhiteSpace($value)) {
                $env:DEEPSEEK_API_KEY = $value
                return
            }
        }
    }
}

function Get-ProductionControlTool {
    Push-Location (Join-Path $repoRoot 'third_party/codex-cli/codex-rs')
    try {
        $json = & cargo run --quiet -p codex-tools --example r6_f5_control_schema 2>$null
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace([string]$json)) {
            throw 'production taskspace_control schema exporter failed'
        }
        ([string]$json) | ConvertFrom-Json -Depth 80
    } finally {
        Pop-Location
    }
}

function ConvertTo-ChatTool {
    param($ResponsesTool)
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = [string]$ResponsesTool.name
            description = [string]$ResponsesTool.description
            parameters = $ResponsesTool.parameters
        }
    }
}

function Get-ActionName {
    param($Variant)
    if ($null -eq $Variant.properties.action.enum) { return '' }
    [string]@($Variant.properties.action.enum)[0]
}

function New-ProbeArms {
    param($ResponsesTool)
    $full = ConvertTo-ChatTool $ResponsesTool
    $bootstrap = Copy-JsonValue $full
    $initialize = @($bootstrap.function.parameters.anyOf | Where-Object {
            (Get-ActionName $_) -eq 'initialize_map'
        })
    if ($initialize.Count -ne 1) {
        throw "expected exactly one initialize_map variant, got $($initialize.Count)"
    }
    $bootstrap.function.parameters.anyOf = @($initialize[0])
    $explicit = Copy-JsonValue $bootstrap
    $explicit.function.description = 'Mandatory mechanical TaskSpace bootstrap tool. initialize_map declares the rooted DAG and its immediate continuation. Finish is terminal identity only and accepts node_id only; executable work belongs to Work nodes.'
    [ordered]@{
        A = $full
        B = $bootstrap
        C = $explicit
    }
}

function Get-StringFingerprint {
    param($Value)
    if ($null -eq $Value) { return $null }
    $text = [string]$Value
    [ordered]@{
        bytes = Get-Utf8Bytes $text
        sha256 = Get-Sha256 $text
    }
}

function Get-NodeShape {
    param($Node)
    if ($null -eq $Node) { return $null }
    [ordered]@{
        keys = @(Get-PropertyNames $Node)
        node_id = [string]$Node.node_id
        has_goal = (Get-PropertyNames $Node) -contains 'goal'
        goal = Get-StringFingerprint $Node.goal
    }
}

function Get-ArgumentShape {
    param([AllowEmptyString()][string]$RawArguments)
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
            field_errors = @('arguments:not_json')
        }
    }
    $allowedTop = @('action', 'root', 'initial_work_node', 'finish', 'additional_work_nodes', 'edges', 'continuation')
    $requiredTop = $allowedTop
    $topKeys = @(Get-PropertyNames $arguments)
    $errors = [System.Collections.Generic.List[string]]::new()
    foreach ($key in $requiredTop) {
        if ($topKeys -notcontains $key) { $errors.Add("missing:$key") }
    }
    foreach ($key in $topKeys) {
        if ($allowedTop -notcontains $key) { $errors.Add("unexpected:$key") }
    }
    if ([string]$arguments.action -ne 'initialize_map') { $errors.Add('action:not_initialize_map') }
    $finishKeys = @(Get-PropertyNames $arguments.finish)
    if ($finishKeys -notcontains 'node_id') { $errors.Add('missing:finish.node_id') }
    foreach ($key in $finishKeys) {
        if ($key -ne 'node_id') { $errors.Add("unexpected:finish.$key") }
    }
    foreach ($entry in @(
            @{ path = 'root'; value = $arguments.root },
            @{ path = 'initial_work_node'; value = $arguments.initial_work_node }
        )) {
        $keys = @(Get-PropertyNames $entry.value)
        foreach ($required in @('node_id', 'goal')) {
            if ($keys -notcontains $required) { $errors.Add("missing:$($entry.path).$required") }
        }
        foreach ($key in $keys) {
            if (@('node_id', 'goal') -notcontains $key) {
                $errors.Add("unexpected:$($entry.path).$key")
            }
        }
    }
    $additionalNodes = @($arguments.additional_work_nodes)
    for ($index = 0; $index -lt $additionalNodes.Count; $index++) {
        $keys = @(Get-PropertyNames $additionalNodes[$index])
        foreach ($required in @('node_id', 'goal')) {
            if ($keys -notcontains $required) { $errors.Add("missing:additional_work_nodes[$index].$required") }
        }
        foreach ($key in $keys) {
            if (@('node_id', 'goal') -notcontains $key) {
                $errors.Add("unexpected:additional_work_nodes[$index].$key")
            }
        }
    }
    $edges = @($arguments.edges)
    for ($index = 0; $index -lt $edges.Count; $index++) {
        $keys = @(Get-PropertyNames $edges[$index])
        foreach ($required in @('from', 'to')) {
            if ($keys -notcontains $required) { $errors.Add("missing:edges[$index].$required") }
        }
        foreach ($key in $keys) {
            if (@('from', 'to') -notcontains $key) { $errors.Add("unexpected:edges[$index].$key") }
        }
    }
    $continuationKeys = @(Get-PropertyNames $arguments.continuation)
    $continuationKind = [string]$arguments.continuation.kind
    if ($continuationKeys -notcontains 'kind') { $errors.Add('missing:continuation.kind') }
    if (@('actions', 'patch_then_actions') -notcontains $continuationKind) {
        $errors.Add('continuation:invalid_kind')
    }
    $actionItems = @($arguments.continuation.actions)
    if ($continuationKind -eq 'actions' -and $actionItems.Count -eq 0) {
        $errors.Add('missing:continuation.actions')
    }
    for ($index = 0; $index -lt $actionItems.Count; $index++) {
        $keys = @(Get-PropertyNames $actionItems[$index])
        foreach ($required in @('tool_name', 'arguments')) {
            if ($keys -notcontains $required) { $errors.Add("missing:continuation.actions[$index].$required") }
        }
        foreach ($key in $keys) {
            if (@('tool_name', 'arguments') -notcontains $key) {
                $errors.Add("unexpected:continuation.actions[$index].$key")
            }
        }
    }
    $toolNames = @($actionItems | ForEach-Object { [string]$_.tool_name })
    [ordered]@{
        raw = $raw
        parsed = $true
        verdict = if ($errors.Count -eq 0) { 'valid_initialize_shape' } else { 'field_error' }
        field_errors = @($errors)
        top_level_keys = $topKeys
        action = [string]$arguments.action
        root = Get-NodeShape $arguments.root
        initial_work_node = Get-NodeShape $arguments.initial_work_node
        finish = Get-NodeShape $arguments.finish
        additional_work_node_count = $additionalNodes.Count
        edge_count = $edges.Count
        continuation_kind = $continuationKind
        continuation_action_count = $actionItems.Count
        continuation_tool_names = $toolNames
    }
}

function Get-ResponseObservation {
    param($Payload, [int]$HttpStatus)
    $choices = if ($null -eq $Payload) { @() } else { @($Payload.choices) }
    $message = if ($choices.Count -gt 0) { $choices[0].message } else { $null }
    $calls = if ($null -eq $message) { @() } else { @($message.tool_calls) }
    $firstCall = if ($calls.Count -gt 0) { $calls[0] } else { $null }
    $rawArguments = if ($null -ne $firstCall) { [string]$firstCall.function.arguments } else { '' }
    $shape = if ($calls.Count -eq 1 -and [string]$firstCall.function.name -eq 'taskspace_control') {
        Get-ArgumentShape $rawArguments
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

function Get-FixtureResponse {
    param($Fixture, [string]$Arm, [string]$Sample, [int]$RepeatIndex)
    $matches = @($Fixture.responses | Where-Object {
            [string]$_.arm -eq $Arm -and [string]$_.sample -eq $Sample -and [int]$_.repeat -eq $RepeatIndex
        })
    if ($matches.Count -ne 1) { throw "fixture response missing for $Arm/$Sample/$RepeatIndex" }
    $matches[0]
}

function Invoke-ProbeRequest {
    param($Body, $Fixture, [string]$Arm, [string]$Sample, [int]$RepeatIndex)
    if ($null -ne $Fixture) {
        $entry = Get-FixtureResponse $Fixture $Arm $Sample $RepeatIndex
        return [ordered]@{ status = [int]$entry.http_status; payload = $entry.payload; duration_ms = 1 }
    }
    $bodyJson = $Body | ConvertTo-Json -Depth 80 -Compress
    $started = Get-Date
    $response = Invoke-WebRequest -Method Post -Uri $Endpoint -Headers @{
        Authorization = "Bearer $env:DEEPSEEK_API_KEY"
    } -ContentType 'application/json' -Body $bodyJson -SkipHttpErrorCheck -TimeoutSec 120
    $responseText = if ($response.Content -is [byte[]]) {
        [System.Text.Encoding]::UTF8.GetString($response.Content)
    } else { [string]$response.Content }
    [ordered]@{
        status = [int]$response.StatusCode
        payload = if ([string]::IsNullOrWhiteSpace($responseText)) { $null } else { $responseText | ConvertFrom-Json -Depth 80 }
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
    }
}

function Get-F5ProbeSamples {
    [ordered]@{
        simple = 'A small project has one failing tax calculation test. Inspect the README and relevant tests, fix the implementation, and run the tests.'
        complex = 'A subscription billing project has regressions in usage parsing, plan pricing, annual discounts, tax, and invoice totals. One test may conflict with the README. Understand the rules, repair code or the incorrect test, and verify the suite.'
    }
}

function Get-F5ProbeSystemPrompt {
    'You are a coding agent using mandatory TaskSpace. Start by declaring the rooted task map and its immediate first executable action through taskspace_control. Do not solve or explain the task yet.'
}

if ($LibraryOnly) { return }

$responsesTool = Get-ProductionControlTool
$sourceJson = $responsesTool | ConvertTo-Json -Depth 80 -Compress
$arms = New-ProbeArms $responsesTool
$fixture = if ([string]::IsNullOrWhiteSpace($FixturePath)) { $null } else {
    (Get-Content -Raw -LiteralPath $FixturePath) | ConvertFrom-Json -Depth 80
}
if ($null -eq $fixture) {
    Import-LocalCredentialIfNeeded
    if ([string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) {
        throw 'DEEPSEEK_API_KEY is required for the live F5.0 probe'
    }
}

$samples = Get-F5ProbeSamples
$armOrders = @(@('A', 'B', 'C'), @('B', 'C', 'A'), @('C', 'A', 'B'))
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
                # Production disables thinking when DeepSeek receives named tool_choice.
                thinking = [ordered]@{ type = 'disabled' }
                stream = $false
                temperature = 0
            }
            $bodyJson = $body | ConvertTo-Json -Depth 80 -Compress
            $toolJson = $tool | ConvertTo-Json -Depth 80 -Compress
            $invocation = Invoke-ProbeRequest $body $fixture $armName $sampleName $repeatIndex
            $observation = Get-ResponseObservation $invocation.payload $invocation.status
            $events.Add([ordered]@{
                    event_name = 'r6.f5.bootstrap_contract_observed'
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
foreach ($armName in @('A', 'B', 'C')) {
    $armEvents = @($events | Where-Object { $_.arm -eq $armName })
    $finishGoal = @($armEvents | Where-Object { $_.response.arguments.finish.has_goal }).Count
    $fieldErrors = @($armEvents | Where-Object { $_.response.arguments.verdict -eq 'field_error' }).Count
    $inputStats = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.input_tokens })
    $cachedStats = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.cached_input_tokens })
    $uncachedStats = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.response.usage.uncached_input_tokens })
    $durationStats = Get-NumberStats -Values @($armEvents | ForEach-Object { [double]$_.duration_ms })
    $request2Plus = @($armEvents | Where-Object { $_.repeat -gt 1 })
    $request2PlusInput = [double](($request2Plus | ForEach-Object { $_.response.usage.input_tokens } | Measure-Object -Sum).Sum)
    $request2PlusCached = [double](($request2Plus | ForEach-Object { $_.response.usage.cached_input_tokens } | Measure-Object -Sum).Sum)
    $summaries += [ordered]@{
        arm = $armName
        request_count = $armEvents.Count
        http_200_count = @($armEvents | Where-Object { $_.response.http_status -eq 200 }).Count
        parsed_count = @($armEvents | Where-Object { $_.response.arguments.parsed }).Count
        valid_initialize_shape_count = @($armEvents | Where-Object { $_.response.arguments.verdict -eq 'valid_initialize_shape' }).Count
        field_error_count = $fieldErrors
        finish_goal_count = $finishGoal
        input_tokens = $inputStats
        cached_input_tokens = $cachedStats
        uncached_input_tokens = $uncachedStats
        duration_ms = $durationStats
        request2plus_cache = [ordered]@{
            request_count = $request2Plus.Count
            input_tokens = $request2PlusInput
            cached_input_tokens = $request2PlusCached
            hit_rate = if ($request2PlusInput -eq 0) { 0 } else { $request2PlusCached / $request2PlusInput }
        }
    }
}
$a = @($summaries | Where-Object arm -eq 'A')[0]
$b = @($summaries | Where-Object arm -eq 'B')[0]
$c = @($summaries | Where-Object arm -eq 'C')[0]
$highReproductionThreshold = [Math]::Max(1, (2 * $Repeat) - 1)
$attribution = if ($a.finish_goal_count -le 1) {
    'inconclusive_prior_behavior_not_reproduced'
} elseif ($b.finish_goal_count -le 1) {
    'schema_breadth_supported'
} elseif ($c.finish_goal_count -le 1) {
    'description_salience_supported'
} elseif ($b.finish_goal_count -ge $highReproductionThreshold -and $c.finish_goal_count -ge $highReproductionThreshold) {
    'refuted_schema_breadth_and_description_salience'
} else {
    'inconclusive_no_arm_reduced_finish_goal'
}
$infrastructureValid = @($summaries | Where-Object {
        $_.request_count -ne (2 * $Repeat) -or $_.http_200_count -ne $_.request_count -or $_.parsed_count -ne $_.request_count
    }).Count -eq 0
$result = [ordered]@{
    schema_version = 'r6-f5-bootstrap-contract-probe-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    repeat = $Repeat
    source_builder = [ordered]@{
        command = 'cargo run --quiet -p codex-tools --example r6_f5_control_schema'
        responses_tool_bytes = Get-Utf8Bytes $sourceJson
        responses_tool_sha256 = Get-Sha256 $sourceJson
        full_action_variants = @($responsesTool.parameters.anyOf | ForEach-Object { Get-ActionName $_ })
    }
    arm_contracts = @($arms.GetEnumerator() | ForEach-Object {
            $json = $_.Value | ConvertTo-Json -Depth 80 -Compress
            [ordered]@{ arm = $_.Key; schema_bytes = Get-Utf8Bytes $json; schema_sha256 = Get-Sha256 $json; description_sha256 = Get-Sha256 ([string]$_.Value.function.description) }
        })
    summaries = $summaries
    diagnostic = [ordered]@{
        infrastructure_valid = $infrastructureValid
        attribution = $attribution
        h008_evidence_gate = if ($infrastructureValid -and $attribution -notlike 'inconclusive*') { 'satisfied' } else { 'not_satisfied' }
    }
    events = @($events)
}
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText($OutputPath, ($result | ConvertTo-Json -Depth 80), [System.Text.UTF8Encoding]::new($false))
$eventsPath = Join-Path $parent 'probe-events.jsonl'
$eventLines = @($events | ForEach-Object { $_ | ConvertTo-Json -Depth 80 -Compress })
[System.IO.File]::WriteAllLines($eventsPath, $eventLines, [System.Text.UTF8Encoding]::new($false))
Write-Host "R6F5BootstrapProbe: $OutputPath"
Write-Host "Attribution: $attribution"
if (-not $infrastructureValid) { exit 2 }
