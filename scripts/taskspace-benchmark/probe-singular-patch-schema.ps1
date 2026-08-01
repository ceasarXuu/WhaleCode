param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$StableEndpoint = 'https://api.deepseek.com/chat/completions',
    [string]$StrictEndpoint = 'https://api.deepseek.com/beta/chat/completions',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot 'target/r5-j7-schema-probe/singular-patch-capability.json'
}
$apiKey = [string]$env:DEEPSEEK_API_KEY
if ([string]::IsNullOrWhiteSpace($apiKey)) {
    throw 'DEEPSEEK_API_KEY is required for the singular patch schema probe'
}

$patchAction = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        tool_name = [ordered]@{ type = 'string'; enum = @('apply_patch') }
        input = [ordered]@{ type = 'string' }
    }
    required = @('tool_name', 'input')
    additionalProperties = $false
}
$execAction = [ordered]@{
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
$parameters = [ordered]@{
    type = 'object'
    properties = [ordered]@{
        action = [ordered]@{ type = 'string'; enum = @('initialize_then_actions') }
        actions = [ordered]@{
            type = 'array'
            minItems = 1
            items = [ordered]@{ anyOf = @($execAction, $patchAction) }
            contains = $patchAction
            maxContains = 1
        }
    }
    required = @('action', 'actions')
    additionalProperties = $false
}

function New-ControlTool {
    param([bool]$Strict)
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = 'taskspace_control'
            description = 'Schema capability probe. Follow the requested action shape.'
            strict = $Strict
            parameters = $parameters
        }
    }
}

function Get-PatchCount {
    param($Message)
    $calls = @($Message.tool_calls)
    if ($calls.Count -ne 1) { return -1 }
    try {
        $arguments = ([string]$calls[0].function.arguments) | ConvertFrom-Json
    } catch {
        return -1
    }
    return @($arguments.actions | Where-Object { [string]$_.tool_name -eq 'apply_patch' }).Count
}

function Invoke-Probe {
    param(
        [string]$Name,
        [string]$Endpoint,
        [bool]$Strict,
        [string]$Prompt
    )
    $body = [ordered]@{
        model = $Model
        messages = @(
            [ordered]@{
                role = 'system'
                content = 'Provider JSON Schema capability probe. Call the named tool exactly once and emit no prose.'
            },
            [ordered]@{ role = 'user'; content = $Prompt }
        )
        tools = @(New-ControlTool $Strict)
        tool_choice = [ordered]@{
            type = 'function'
            function = [ordered]@{ name = 'taskspace_control' }
        }
        thinking = [ordered]@{ type = 'disabled' }
        stream = $false
        temperature = 0
    }
    $started = Get-Date
    try {
        $response = Invoke-WebRequest -Method Post -Uri $Endpoint -Headers @{
            Authorization = "Bearer $apiKey"
        } -ContentType 'application/json' -Body ($body | ConvertTo-Json -Depth 40 -Compress) `
            -SkipHttpErrorCheck -TimeoutSec 90
        $responseText = if ($response.Content -is [byte[]]) {
            [System.Text.Encoding]::UTF8.GetString($response.Content)
        } else {
            [string]$response.Content
        }
        $payload = if ($responseText) { $responseText | ConvertFrom-Json } else { $null }
        $message = if ($null -ne $payload -and @($payload.choices).Count -gt 0) {
            $payload.choices[0].message
        } else { $null }
        $errorMessage = if ($null -ne $payload.error) {
            if ($payload.error -is [string]) { [string]$payload.error }
            else { [string]$payload.error.message }
        } else { '' }
        return [ordered]@{
            name = $Name
            endpoint_kind = if ($Strict) { 'beta_strict' } else { 'stable_non_strict' }
            http_status = [int]$response.StatusCode
            duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
            tool_call_count = if ($null -ne $message) { @($message.tool_calls).Count } else { 0 }
            patch_count = if ($null -ne $message) { Get-PatchCount $message } else { -1 }
            error_class = if ($errorMessage) { 'provider_rejected' } else { '' }
            error_message = $errorMessage
        }
    } catch {
        return [ordered]@{
            name = $Name
            endpoint_kind = if ($Strict) { 'beta_strict' } else { 'stable_non_strict' }
            http_status = 0
            duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
            tool_call_count = 0
            patch_count = -1
            error_class = 'transport_error'
            error_message = $_.Exception.Message
        }
    }
}

$singlePrompt = 'Call initialize_then_actions with exactly one apply_patch action whose input adds a.txt.'
$multiPrompt = 'Call initialize_then_actions with two separate apply_patch actions, one for a.txt and one for b.txt.'
$probes = @()
foreach ($strictMode in @($false, $true)) {
    $endpoint = if ($strictMode) { $StrictEndpoint } else { $StableEndpoint }
    $probes += Invoke-Probe 'single_patch' $endpoint $strictMode $singlePrompt
    $probes += Invoke-Probe 'requested_two_patches' $endpoint $strictMode $multiPrompt
}

$stable = @($probes | Where-Object { $_.endpoint_kind -eq 'stable_non_strict' })
$strict = @($probes | Where-Object { $_.endpoint_kind -eq 'beta_strict' })
$result = [ordered]@{
    schema_version = 'r5-j7-singular-patch-provider-capability-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    probes = $probes
    capabilities = [ordered]@{
        stable_schema_accepted = (@($stable | Where-Object { $_.http_status -eq 200 }).Count -eq 2)
        strict_schema_accepted = (@($strict | Where-Object { $_.http_status -eq 200 }).Count -eq 2)
        stable_generated_at_most_one_patch = (@($stable | Where-Object { $_.patch_count -ge 0 -and $_.patch_count -le 1 }).Count -eq 2)
        strict_generated_at_most_one_patch = (@($strict | Where-Object { $_.patch_count -ge 0 -and $_.patch_count -le 1 }).Count -eq 2)
        enforcement_proven = $false
    }
    decision_note = 'HTTP acceptance and observed generation do not prove keyword enforcement. Production uses an explicit singular patch slot plus request-wide preflight.'
}

$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText(
    $OutputPath,
    ($result | ConvertTo-Json -Depth 40),
    [System.Text.UTF8Encoding]::new($false)
)
Write-Host "SingularPatchProviderCapability: $OutputPath"
