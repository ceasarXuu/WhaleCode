param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$StableEndpoint = 'https://api.deepseek.com/chat/completions',
    [string]$StrictEndpoint = 'https://api.deepseek.com/beta/chat/completions',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot 'target/r5-j6-schema-probe/provider-capability.json'
}
$apiKey = [string]$env:DEEPSEEK_API_KEY
if ([string]::IsNullOrWhiteSpace($apiKey)) {
    throw 'DEEPSEEK_API_KEY is required for the schema-first provider probe'
}

function New-ActionSchema {
    param([string]$Action, [hashtable]$ExtraProperties, [string[]]$ExtraRequired)
    $properties = [ordered]@{
        action = [ordered]@{ type = 'string'; enum = @($Action) }
    }
    foreach ($entry in $ExtraProperties.GetEnumerator()) {
        $properties[$entry.Key] = $entry.Value
    }
    [ordered]@{
        type = 'object'
        properties = $properties
        required = @('action') + @($ExtraRequired)
        additionalProperties = $false
    }
}

$nestedAction = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        tool_name = [ordered]@{ type = 'string'; enum = @('exec_command') }
        arguments = [ordered]@{
            type = 'object'
            properties = [ordered]@{ cmd = [ordered]@{ type = 'string' } }
            required = @('cmd')
            additionalProperties = $false
        }
    }
    required = @('tool_name', 'arguments')
    additionalProperties = $false
}
$finishStep = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        node_id = [ordered]@{ type = 'string' }
        next_node_id = [ordered]@{ type = 'string' }
    }
    required = @('node_id', 'next_node_id')
    additionalProperties = $false
}
$terminalFinishStep = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        node_id = [ordered]@{ type = 'string' }
    }
    required = @('node_id')
    additionalProperties = $false
}
$actionsProperty = [ordered]@{ type = 'array'; minItems = 1; items = $nestedAction }
$finishesProperty = [ordered]@{ type = 'array'; minItems = 1; items = $finishStep }
$initialNode = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        node_id = [ordered]@{ type = 'string' }
        kind = [ordered]@{ type = 'string'; enum = @('inspect_code_context') }
        goal = [ordered]@{ type = 'string' }
    }
    required = @('node_id', 'kind', 'goal')
    additionalProperties = $false
}
$parameters = [ordered]@{
    type = 'object'
    anyOf = @(
        (New-ActionSchema 'initialize_then_actions' ([ordered]@{
                    initial_nodes = [ordered]@{ type = 'array'; minItems = 1; items = $initialNode }
                    current_node_id = [ordered]@{ type = 'string' }
                    actions = $actionsProperty
                }) @('initial_nodes', 'current_node_id', 'actions')),
        (New-ActionSchema 'finish_nodes' ([ordered]@{
                    finishes = $finishesProperty
                }) @('finishes')),
        (New-ActionSchema 'finish_then_end' ([ordered]@{
                    terminal_finish = $terminalFinishStep
                    final_candidate = [ordered]@{ type = 'string' }
                }) @('terminal_finish', 'final_candidate'))
    )
}

function New-ControlTool {
    param([bool]$Strict)
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = 'taskspace_control'
            description = 'Emit one schema-first TaskSpace control call.'
            strict = $Strict
            parameters = $parameters
        }
    }
}

function Get-Shape {
    param($Message)
    $calls = @($Message.tool_calls)
    $arguments = if ($calls.Count -eq 1) {
        try { ([string]$calls[0].function.arguments) | ConvertFrom-Json } catch { $null }
    } else { $null }
    [ordered]@{
        call_count = $calls.Count
        tool_name = if ($calls.Count -eq 1) { [string]$calls[0].function.name } else { '' }
        action = if ($null -ne $arguments) { [string]$arguments.action } else { '' }
        finish_count = if ($null -ne $arguments -and $null -ne $arguments.finishes) { @($arguments.finishes).Count } else { 0 }
        has_terminal_finish = ($null -ne $arguments -and $null -ne $arguments.terminal_finish)
        action_count = if ($null -ne $arguments -and $null -ne $arguments.actions) { @($arguments.actions).Count } else { 0 }
        has_final_candidate = ($null -ne $arguments -and -not [string]::IsNullOrWhiteSpace([string]$arguments.final_candidate))
        parsed = ($null -ne $arguments)
    }
}

function Invoke-Probe {
    param([string]$Name, [string]$Endpoint, [bool]$Strict, [string]$Prompt)
    $body = [ordered]@{
        model = $Model
        messages = @(
            [ordered]@{ role = 'system'; content = 'Provider schema probe. Call the tool exactly once and emit no prose.' },
            [ordered]@{ role = 'user'; content = $Prompt }
        )
        tools = @(New-ControlTool $Strict)
        tool_choice = [ordered]@{ type = 'function'; function = [ordered]@{ name = 'taskspace_control' } }
        thinking = [ordered]@{ type = 'disabled' }
        stream = $false
        temperature = 0
    }
    $started = Get-Date
    $response = Invoke-WebRequest -Method Post -Uri $Endpoint -Headers @{
        Authorization = "Bearer $apiKey"
    } -ContentType 'application/json' -Body ($body | ConvertTo-Json -Depth 40 -Compress) `
        -SkipHttpErrorCheck -TimeoutSec 90
    $responseText = if ($response.Content -is [byte[]]) {
        [System.Text.Encoding]::UTF8.GetString($response.Content)
    } else {
        [string]$response.Content
    }
    $payload = if ([string]::IsNullOrWhiteSpace($responseText)) {
        $null
    } else {
        $responseText | ConvertFrom-Json
    }
    $hasChoices = $null -ne $payload -and
        $payload.PSObject.Properties.Name -contains 'choices' -and
        $null -ne $payload.choices -and
        @($payload.choices).Count -gt 0
    $message = if ($hasChoices) {
        $payload.choices[0].message
    } else { $null }
    $hasError = $null -ne $payload -and $payload.PSObject.Properties.Name -contains 'error'
    $errorMessage = if ($hasError -and $payload.error -is [string]) {
        [string]$payload.error
    } elseif ($hasError -and $null -ne $payload.error -and
        $payload.error.PSObject.Properties.Name -contains 'message') {
        [string]$payload.error.message
    } elseif ($null -ne $payload -and $payload.PSObject.Properties.Name -contains 'message') {
        [string]$payload.message
    } else { '' }
    $responsePreview = if ($responseText.Length -gt 512) {
        $responseText.Substring(0, 512)
    } else { $responseText }
    [ordered]@{
        name = $Name
        endpoint_kind = if ($Strict) { 'beta_strict' } else { 'stable_non_strict' }
        http_status = [int]$response.StatusCode
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
        shape = if ($null -ne $message) { Get-Shape $message } else { $null }
        error_class = if ([string]::IsNullOrWhiteSpace($errorMessage)) { '' } else { 'provider_rejected' }
        error_message = $errorMessage
        response_preview = $responsePreview
    }
}

$prompts = [ordered]@{
    initialize_then_actions = 'Call initialize_then_actions with one inspect node node-1 whose goal is inspect, current_node_id node-1, and one exec_command action whose cmd is pwd.'
    finish_nodes = 'Call finish_nodes with one finish for node-1 and next node-2.'
    finish_then_end = 'Call finish_then_end with terminal_finish node-2 and final_candidate DONE.'
}
$probes = @()
foreach ($strictMode in @($false, $true)) {
    $endpoint = if ($strictMode) { $StrictEndpoint } else { $StableEndpoint }
    foreach ($entry in $prompts.GetEnumerator()) {
        $probes += Invoke-Probe "$($entry.Key)_$($strictMode ? 'strict' : 'non_strict')" `
            $endpoint $strictMode ([string]$entry.Value)
    }
}

function Test-ProbeShape {
    param($Probe)
    if ($Probe.http_status -ne 200 -or $null -eq $Probe.shape -or
        -not $Probe.shape.parsed -or $Probe.shape.call_count -ne 1 -or
        $Probe.shape.tool_name -ne 'taskspace_control') {
        return $false
    }
    if ($Probe.name -like 'initialize_then_actions_*') {
        return $Probe.shape.action -eq 'initialize_then_actions' -and
            $Probe.shape.action_count -eq 1
    }
    if ($Probe.name -like 'finish_nodes_*') {
        return $Probe.shape.action -eq 'finish_nodes' -and
            $Probe.shape.finish_count -eq 1 -and $Probe.shape.action_count -eq 0
    }
    if ($Probe.name -like 'finish_then_end_*') {
        return $Probe.shape.action -eq 'finish_then_end' -and
            $Probe.shape.has_terminal_finish -and $Probe.shape.has_final_candidate
    }
    return $false
}

$stableSchemaAccepted = (@($probes | Where-Object {
            $_.endpoint_kind -eq 'stable_non_strict' -and $_.http_status -eq 200
        }).Count -eq 3)
$stableShapesValid = (@($probes | Where-Object {
            $_.endpoint_kind -eq 'stable_non_strict' -and (Test-ProbeShape $_)
        }).Count -eq 3)
$strictSchemaAccepted = (@($probes | Where-Object {
            $_.endpoint_kind -eq 'beta_strict' -and $_.http_status -eq 200
        }).Count -eq 3)
$strictShapesValid = (@($probes | Where-Object {
            $_.endpoint_kind -eq 'beta_strict' -and (Test-ProbeShape $_)
        }).Count -eq 3)
$result = [ordered]@{
    schema_version = 'r5-j6-schema-first-provider-capability-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    probes = $probes
    capabilities = [ordered]@{
        stable_schema_accepted = $stableSchemaAccepted
        stable_all_shapes_valid = $stableShapesValid
        strict_schema_accepted = $strictSchemaAccepted
        strict_all_shapes_valid = $strictShapesValid
    }
}
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText(
    $OutputPath,
    ($result | ConvertTo-Json -Depth 40),
    [System.Text.UTF8Encoding]::new($false)
)
Write-Host "SchemaFirstProviderCapability: $OutputPath"

if (-not $stableSchemaAccepted -or -not $stableShapesValid) {
    exit 2
}
