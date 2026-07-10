param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$Endpoint = 'https://api.deepseek.com/chat/completions',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot 'target/r5-j0-provider-probe/provider-capability.json'
}
$apiKey = [string]$env:DEEPSEEK_API_KEY
if ([string]::IsNullOrWhiteSpace($apiKey)) {
    throw 'DEEPSEEK_API_KEY is required for the provider capability probe'
}

function New-ProbeTool {
    param([string]$Name, [string]$Description, [hashtable]$Properties, [string[]]$Required)
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = $Name
            description = $Description
            parameters = [ordered]@{
                type = 'object'
                properties = $Properties
                required = @($Required)
                additionalProperties = $false
            }
        }
    }
}

function Get-TextSha256 {
    param([string]$Text)
    if ([string]::IsNullOrEmpty($Text)) { return '' }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [System.Security.Cryptography.SHA256]::HashData($bytes)
    ([System.BitConverter]::ToString($hash)).Replace('-', '').ToLowerInvariant()
}

function Invoke-ProviderProbe {
    param([string]$Name, [hashtable]$Body)
    $started = Get-Date
    $response = Invoke-WebRequest -Method Post -Uri $Endpoint -Headers @{
        Authorization = "Bearer $apiKey"
    } -ContentType 'application/json' -Body ($Body | ConvertTo-Json -Depth 30 -Compress) `
        -SkipHttpErrorCheck -TimeoutSec 90
    $payload = if ([string]::IsNullOrWhiteSpace([string]$response.Content)) {
        $null
    } else {
        $response.Content | ConvertFrom-Json
    }
    $hasChoice = $null -ne $payload -and $null -ne $payload.choices -and @($payload.choices).Count -gt 0
    $message = if ($hasChoice) {
        $payload.choices[0].message
    } else {
        $null
    }
    $content = if ($message) { [string]$message.content } else { '' }
    $toolNames = if ($message) {
        @($message.tool_calls | ForEach-Object { [string]$_.function.name })
    } else {
        @()
    }
    $errorValue = if ($null -ne $payload -and $payload.PSObject.Properties.Name -contains 'error') {
        $payload.error
    } else {
        $null
    }
    $errorMessage = if ($errorValue -is [string]) {
        [string]$errorValue
    } elseif ($null -ne $errorValue -and $errorValue.PSObject.Properties.Name -contains 'message') {
        [string]$errorValue.message
    } else {
        ''
    }
    [pscustomobject]@{
        name = $Name
        http_status = [int]$response.StatusCode
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
        finish_reason = if ($hasChoice) { [string]$payload.choices[0].finish_reason } else { '' }
        tool_names = @($toolNames)
        tool_call_count = @($toolNames).Count
        content_present = -not [string]::IsNullOrEmpty($content)
        content_bytes = [System.Text.Encoding]::UTF8.GetByteCount($content)
        content_sha256 = Get-TextSha256 $content
        error_class = if ($errorMessage -match 'Thinking mode does not support this tool_choice') {
            'thinking_tool_choice_incompatible'
        } elseif (-not [string]::IsNullOrWhiteSpace($errorMessage)) {
            'provider_rejected'
        } else {
            ''
        }
    }
}

$controlTool = New-ProbeTool 'taskspace_control' 'State control probe' `
    @{ action = @{ type = 'string' } } @('action')
$readTool = New-ProbeTool 'read_file' 'Read probe' @{ path = @{ type = 'string' } } @('path')
$firstTool = New-ProbeTool 'first_step' 'First ordered probe' @{ value = @{ type = 'integer' } } @('value')
$secondTool = New-ProbeTool 'second_step' 'Second ordered probe' @{ value = @{ type = 'integer' } } @('value')
$finishTool = New-ProbeTool 'finish_node' 'Finish probe' @{ result = @{ type = 'string' } } @('result')

$namedBody = @{
    model = $Model
    messages = @(
        @{ role = 'system'; content = 'Provider capability probe. Follow the requested tool call exactly.' },
        @{ role = 'user'; content = 'Call taskspace_control exactly once with action initialize_map. Do not call read_file.' }
    )
    tools = @($controlTool, $readTool)
    tool_choice = @{ type = 'function'; function = @{ name = 'taskspace_control' } }
    thinking = @{ type = 'disabled' }
    stream = $false
    temperature = 0
}
$namedThinkingBody = $namedBody.Clone()
$namedThinkingBody.thinking = @{ type = 'enabled' }
$orderedBody = @{
    model = $Model
    messages = @(
        @{ role = 'system'; content = 'Provider capability probe. Emit tool calls in the exact requested order.' },
        @{ role = 'user'; content = 'In one response, call first_step with value 1, then second_step with value 2. Emit both tool calls and no prose.' }
    )
    tools = @($firstTool, $secondTool)
    tool_choice = 'required'
    parallel_tool_calls = $true
    thinking = @{ type = 'disabled' }
    stream = $false
    temperature = 0
}
$terminalBody = @{
    model = $Model
    messages = @(
        @{ role = 'system'; content = 'Provider capability probe. Preserve both assistant text and the requested tool call.' },
        @{ role = 'user'; content = 'Reply with the exact text TERMINAL_CANDIDATE and in the same response call finish_node with result ok.' }
    )
    tools = @($finishTool)
    tool_choice = 'required'
    thinking = @{ type = 'disabled' }
    stream = $false
    temperature = 0
}

$probes = @(
    Invoke-ProviderProbe 'named_tool_choice_thinking_disabled' $namedBody
    Invoke-ProviderProbe 'named_tool_choice_thinking_enabled' $namedThinkingBody
    Invoke-ProviderProbe 'ordered_multi_tool_calls' $orderedBody
    Invoke-ProviderProbe 'assistant_content_with_tool_call' $terminalBody
)
$named = @($probes | Where-Object { $_.name -eq 'named_tool_choice_thinking_disabled' })[0]
$namedThinking = @($probes | Where-Object { $_.name -eq 'named_tool_choice_thinking_enabled' })[0]
$ordered = @($probes | Where-Object { $_.name -eq 'ordered_multi_tool_calls' })[0]
$terminal = @($probes | Where-Object { $_.name -eq 'assistant_content_with_tool_call' })[0]
$result = [pscustomobject]@{
    schema_version = 'r5-j0-provider-capability-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    endpoint = $Endpoint
    model = $Model
    probes = $probes
    capabilities = [ordered]@{
        named_tool_choice_with_thinking_disabled = ($named.http_status -eq 200 -and (@($named.tool_names) -join ',') -eq 'taskspace_control')
        named_tool_choice_with_thinking_enabled = ($namedThinking.http_status -eq 200)
        ordered_multi_tool_calls = ($ordered.http_status -eq 200 -and (@($ordered.tool_names) -join ',') -eq 'first_step,second_step')
        assistant_content_with_required_tool_call_observed = ($terminal.http_status -eq 200 -and $terminal.content_present -and @($terminal.tool_names) -contains 'finish_node')
    }
    decision = [ordered]@{
        hard_state_selection = 'named_tool_choice_with_thinking_disabled'
        ordered_barrier_source = 'provider_response_item_order'
        terminal_carrier = 'finish_node.final_candidate_tool_argument'
    }
}
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText(
    $OutputPath,
    ($result | ConvertTo-Json -Depth 30),
    [System.Text.UTF8Encoding]::new($false)
)
Write-Host "ProviderCapability: $OutputPath"

$passed = [bool]$result.capabilities.named_tool_choice_with_thinking_disabled -and
    -not [bool]$result.capabilities.named_tool_choice_with_thinking_enabled -and
    [bool]$result.capabilities.ordered_multi_tool_calls -and
    -not [bool]$result.capabilities.assistant_content_with_required_tool_call_observed
if (-not $passed) { exit 2 }
