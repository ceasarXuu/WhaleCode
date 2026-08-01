param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$Endpoint = 'https://api.deepseek.com/chat/completions',
    [ValidateRange(1, 20)][int]$Repeat = 3,
    [string]$OutputPath = '',
    [string]$FixturePath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $runId = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff')
    $OutputPath = Join-Path $repoRoot "target/r7-nested-patch-control-probe/$runId/provider-capability.json"
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
        $json = & cargo run --quiet -p codex-tools --example r7_nested_patch_control_schema 2>$null
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
            strict = $false
            parameters = $ResponsesTool.parameters
        }
    }
}

function New-PatchArgumentsSchema {
    [ordered]@{
        type = 'object'
        properties = [ordered]@{
            input = [ordered]@{ type = 'string'; description = 'Exact apply_patch input.' }
        }
        required = @('input')
        additionalProperties = $false
    }
}

function New-SyntheticLegacyControlTool {
    param($CurrentTool)
    $tool = Copy-JsonValue $CurrentTool
    foreach ($actionVariant in @($tool.function.parameters.anyOf)) {
        if ($null -eq $actionVariant.properties.continuation) { continue }
        $actionVariant.properties.continuation = [ordered]@{
            type = 'object'
            properties = [ordered]@{
                kind = [ordered]@{ type = 'string'; enum = @('patch_then_actions') }
                patch = [ordered]@{
                    type = 'object'
                    properties = [ordered]@{
                        tool_name = [ordered]@{ type = 'string'; enum = @('apply_patch') }
                        arguments = New-PatchArgumentsSchema
                    }
                    required = @('tool_name', 'arguments')
                    additionalProperties = $false
                }
                actions = [ordered]@{ type = 'array'; items = [ordered]@{ type = 'object' } }
            }
            required = @('kind', 'patch')
            additionalProperties = $false
        }
    }
    $tool.function.description += ' Synthetic frozen legacy nested-patch carrier used only by this historical diagnostic.'
    $tool
}

function New-FlatPatchTool {
    param($CurrentTool)
    $tool = Copy-JsonValue $CurrentTool
    foreach ($actionVariant in @($tool.function.parameters.anyOf)) {
        $continuation = $actionVariant.properties.continuation
        if ($null -eq $continuation -or $null -eq $continuation.properties.patch) { continue }
        $continuation.properties.patch = [ordered]@{
            type = 'object'
            properties = [ordered]@{
                tool_name = [ordered]@{ type = 'string'; enum = @('apply_patch') }
                input = [ordered]@{ type = 'string'; description = 'Exact apply_patch input.' }
            }
            required = @('tool_name', 'input')
            additionalProperties = $false
        }
    }
    $tool
}

function New-DirectPatchTool {
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = 'apply_patch'
            description = 'Use apply_patch to edit files.'
            strict = $false
            parameters = New-PatchArgumentsSchema
        }
    }
}

function New-ContinuationPatchInputTool {
    param($CurrentTool)
    $tool = Copy-JsonValue $CurrentTool
    foreach ($actionVariant in @($tool.function.parameters.anyOf)) {
        $continuation = $actionVariant.properties.continuation
        if ($null -eq $continuation) { continue }
        $continuation.properties.PSObject.Properties.Remove('patch')
        $continuation.properties | Add-Member -NotePropertyName 'patch_input' -NotePropertyValue ([ordered]@{
                type = 'string'
                description = 'Exact apply_patch input; this is the continuation patch slot.'
            })
        $continuation.required = @($continuation.required | ForEach-Object {
                if ($_ -eq 'patch') { 'patch_input' } else { $_ }
            })
    }
    $tool
}

function New-TopLevelPatchControlTool {
    param($CurrentTool)
    $tool = Copy-JsonValue $CurrentTool
    $variant = [ordered]@{
        type = 'object'
        properties = [ordered]@{
            action = [ordered]@{ type = 'string'; enum = @('complete_then_patch') }
            expected_revision = [ordered]@{ type = 'integer' }
            current_node_id = [ordered]@{ type = 'string' }
            next_node_id = [ordered]@{ type = 'string' }
            patch_input = [ordered]@{ type = 'string'; description = 'Exact apply_patch input.' }
        }
        required = @('action', 'expected_revision', 'current_node_id', 'next_node_id', 'patch_input')
        additionalProperties = $false
    }
    $tool.function.parameters.anyOf = @($tool.function.parameters.anyOf) + @($variant)
    $tool
}

$largePatch = @'
*** Begin Patch
*** Update File: src/billing_service/usage.py
@@
 def parse_usage_row(row):
-    account, plan, seats, billing_period = row.split(",")
+    parts = row.split(",")
+    if len(parts) != 4:
+        raise ValueError("expected 4 fields")
+    account, plan, seats, billing_period = parts
+    account = account.strip()
+    plan = plan.strip().lower()
+    billing_period = billing_period.strip().lower()
+    seat_count = int(seats)
+    if seat_count < 1:
+        raise ValueError(f"seats must be positive, got {seat_count}")
     return {
         "account": account,
         "plan": plan,
-        "seats": int(seats),
+        "seats": seat_count,
         "billing_period": billing_period,
     }
*** Update File: src/billing_service/plans.py
@@
 PRICES = {
     "basic": 10,
-    "pro": 30,
+    "pro": 29,
     "enterprise": 99,
 }
@@
 def plan_subtotal(plan, seats, billing_period):
+    if plan not in PRICES:
+        raise ValueError(f"unknown plan: {plan}")
     monthly = PRICES[plan] * seats
     if billing_period == "annual":
-        return monthly * 12
+        return monthly * 10
*** Update File: src/billing_service/tax.py
@@
-    "EU": 0.19,
+    "EU": 0.20,
*** Update File: tests/test_plans.py
@@
-def test_enterprise_annual_uses_twelve_months():
-    assert plan_subtotal("enterprise", 1, "annual") == 1188
+def test_enterprise_annual_charges_ten_months():
+    assert plan_subtotal("enterprise", 1, "annual") == 990
*** End Patch
'@
$shortPatch = @'
*** Begin Patch
*** Update File: src/tax_calc.py
@@
-    return round(value, 1)
+    return round(value, 2)
*** End Patch
'@

function Get-Prompt {
    param([string]$Arm, [string]$Patch)
    if ($Arm -eq 'direct_large') {
        return @"
Use apply_patch exactly once. Put the following exact patch in input.
Do not emit prose or call any other tool.

$Patch
"@
    }
    if ($Arm -eq 'control_top_level_large') {
        return @"
Use taskspace_control exactly once. Emit complete_then_patch with expected_revision 2,
current_node_id explore, next_node_id fix, and put the following exact patch in patch_input.
Do not emit prose or call any other tool.

$Patch
"@
    }
    if ($Arm -eq 'continuation_patch_input_large') {
        return @"
Use taskspace_control exactly once. Emit complete_then_continue with expected_revision 2,
current_node_id explore, next_node_id fix, and continuation kind patch_then_actions.
Set continuation.actions to an empty array and put the following exact patch in continuation.patch_input.
Do not emit prose or call any other tool.

$Patch
"@
    }
    $patchField = if ($Arm -eq 'flat_large') {
        'continuation.patch.input'
    } else {
        'continuation.patch.arguments.input'
    }
    @"
Use taskspace_control exactly once. Emit complete_then_continue with expected_revision 2,
current_node_id explore, next_node_id fix, and continuation kind patch_then_actions.
Set continuation.patch.tool_name to apply_patch and put the following exact patch in $patchField.
Do not emit prose or additional actions.

$Patch
"@
}

function Get-FixtureResponse {
    param($Fixture, [string]$Arm, [int]$RepeatIndex)
    $matches = @($Fixture.responses | Where-Object {
            [string]$_.arm -eq $Arm -and [int]$_.repeat -eq $RepeatIndex
        })
    if ($matches.Count -ne 1) { throw "fixture response missing for $Arm/$RepeatIndex" }
    [ordered]@{ status = [int]$matches[0].http_status; payload = $matches[0].payload; duration_ms = 1 }
}

function Invoke-ProbeRequest {
    param($Body, $Fixture, [string]$Arm, [int]$RepeatIndex)
    if ($null -ne $Fixture) { return Get-FixtureResponse $Fixture $Arm $RepeatIndex }
    $started = Get-Date
    $response = Invoke-WebRequest -Method Post -Uri $Endpoint -Headers @{
        Authorization = "Bearer $env:DEEPSEEK_API_KEY"
    } -ContentType 'application/json' -Body ($Body | ConvertTo-Json -Depth 80 -Compress) `
        -SkipHttpErrorCheck -TimeoutSec 120
    $text = if ($response.Content -is [byte[]]) {
        [System.Text.Encoding]::UTF8.GetString($response.Content)
    } else { [string]$response.Content }
    [ordered]@{
        status = [int]$response.StatusCode
        payload = if ([string]::IsNullOrWhiteSpace($text)) { $null } else { $text | ConvertFrom-Json -Depth 80 }
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
    }
}

function Get-Observation {
    param($Invocation, [string]$Arm, [string]$ExpectedPatch)
    $calls = if ($null -eq $Invocation.payload) { @() } else { @($Invocation.payload.choices[0].message.tool_calls) }
    $arguments = if ($calls.Count -eq 1) { [string]$calls[0].function.arguments } else { '' }
    $parsed = $null
    $parseError = ''
    if (-not [string]::IsNullOrWhiteSpace($arguments)) {
        try { $parsed = $arguments | ConvertFrom-Json -Depth 80 } catch { $parseError = $_.Exception.Message }
    }
    $expectedToolName = if ($Arm -eq 'direct_large') { 'apply_patch' } else { 'taskspace_control' }
    $patchInput = if ($null -eq $parsed) { '' } elseif ($Arm -eq 'direct_large') {
        [string]$parsed.input
    } elseif ($Arm -eq 'control_top_level_large') {
        [string]$parsed.patch_input
    } elseif ($Arm -eq 'continuation_patch_input_large') {
        [string]$parsed.continuation.patch_input
    } elseif ($Arm -eq 'flat_large') {
        [string]$parsed.continuation.patch.input
    } else {
        [string]$parsed.continuation.patch.arguments.input
    }
    [ordered]@{
        http_status = [int]$Invocation.status
        duration_ms = [int64]$Invocation.duration_ms
        call_count = $calls.Count
        tool_name = if ($calls.Count -eq 1) { [string]$calls[0].function.name } else { '' }
        arguments_bytes = Get-Utf8Bytes $arguments
        arguments_sha256 = Get-Sha256 $arguments
        parsed = ($null -ne $parsed)
        parse_error_class = if ([string]::IsNullOrWhiteSpace($parseError)) { '' } elseif ($parseError -match 'Additional text') { 'trailing_characters' } else { 'invalid_json' }
        action = if ($null -eq $parsed) { '' } else { [string]$parsed.action }
        patch_present = -not [string]::IsNullOrWhiteSpace($patchInput)
        expected_shape_valid = ($calls.Count -eq 1 -and [string]$calls[0].function.name -eq $expectedToolName -and
            $null -ne $parsed -and -not [string]::IsNullOrWhiteSpace($patchInput) -and
            ($Arm -eq 'direct_large' -or
                ($Arm -eq 'control_top_level_large' -and [string]$parsed.action -eq 'complete_then_patch') -or
                ($Arm -ne 'control_top_level_large' -and [string]$parsed.action -eq 'complete_then_continue')))
        patch_exact = ($patchInput -ceq $ExpectedPatch)
        patch_bytes = Get-Utf8Bytes $patchInput
        patch_sha256 = Get-Sha256 $patchInput
        usage = [ordered]@{
            input_tokens = [int64]$Invocation.payload.usage.prompt_tokens
            cached_input_tokens = [int64]$Invocation.payload.usage.prompt_cache_hit_tokens
            output_tokens = [int64]$Invocation.payload.usage.completion_tokens
        }
    }
}

$responsesTool = Get-ProductionControlTool
$currentTool = New-SyntheticLegacyControlTool (ConvertTo-ChatTool $responsesTool)
$arms = [ordered]@{
    current_large = [ordered]@{ tool = $currentTool; patch = $largePatch }
    flat_large = [ordered]@{ tool = (New-FlatPatchTool $currentTool); patch = $largePatch }
    current_short = [ordered]@{ tool = $currentTool; patch = $shortPatch }
    direct_large = [ordered]@{ tool = (New-DirectPatchTool); patch = $largePatch }
    continuation_patch_input_large = [ordered]@{ tool = (New-ContinuationPatchInputTool $currentTool); patch = $largePatch }
    control_top_level_large = [ordered]@{ tool = (New-TopLevelPatchControlTool $currentTool); patch = $largePatch }
}
$armOrder = @(
    'current_large',
    'flat_large',
    'current_short',
    'direct_large',
    'continuation_patch_input_large',
    'control_top_level_large'
)
$fixture = if ([string]::IsNullOrWhiteSpace($FixturePath)) { $null } else {
    (Get-Content -Raw -LiteralPath $FixturePath) | ConvertFrom-Json -Depth 80
}
if ($null -eq $fixture) {
    Import-LocalCredentialIfNeeded
    if ([string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) {
        throw 'DEEPSEEK_API_KEY is required for the live nested patch control probe'
    }
}

$events = [System.Collections.Generic.List[object]]::new()
for ($repeatIndex = 1; $repeatIndex -le $Repeat; $repeatIndex++) {
    foreach ($armName in $armOrder) {
        $arm = $arms[$armName]
        $prompt = Get-Prompt $armName $arm.patch
        $body = [ordered]@{
            model = $Model
            messages = @(
                [ordered]@{ role = 'system'; content = 'Diagnostic provider probe. Follow the tool schema and call the requested tool once.' },
                [ordered]@{ role = 'user'; content = $prompt }
            )
            tools = @($arm.tool)
            tool_choice = 'auto'
            thinking = [ordered]@{ type = 'enabled' }
            reasoning_effort = 'max'
            stream = $false
        }
        $invocation = Invoke-ProbeRequest $body $fixture $armName $repeatIndex
        $observation = Get-Observation $invocation $armName $arm.patch
        $events.Add([ordered]@{
                event_name = 'r7.nested_patch_control_observed'
                arm = $armName
                repeat = $repeatIndex
                transport = 'non_streaming_chat_completions'
                request = [ordered]@{
                    schema_bytes = Get-Utf8Bytes ($arm.tool | ConvertTo-Json -Depth 80 -Compress)
                    schema_sha256 = Get-Sha256 ($arm.tool | ConvertTo-Json -Depth 80 -Compress)
                    prompt_bytes = Get-Utf8Bytes $prompt
                    prompt_sha256 = Get-Sha256 $prompt
                    expected_patch_bytes = Get-Utf8Bytes $arm.patch
                    expected_patch_sha256 = Get-Sha256 $arm.patch
                }
                response = $observation
            })
    }
}

$summaries = foreach ($armName in $armOrder) {
    $rows = @($events | Where-Object arm -eq $armName)
    [ordered]@{
        arm = $armName
        requests = $rows.Count
        http_200 = @($rows | Where-Object { $_.response.http_status -eq 200 }).Count
        one_tool_call = @($rows | Where-Object { $_.response.call_count -eq 1 }).Count
        json_valid = @($rows | Where-Object { $_.response.parsed }).Count
        expected_shape_valid = @($rows | Where-Object { $_.response.expected_shape_valid }).Count
        trailing_characters = @($rows | Where-Object { $_.response.parse_error_class -eq 'trailing_characters' }).Count
        patch_exact = @($rows | Where-Object { $_.response.patch_exact }).Count
    }
}
$result = [ordered]@{
    schema_version = 'r7-nested-patch-control-probe-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    endpoint = if ($null -eq $fixture) { $Endpoint } else { 'fixture' }
    repeat = $Repeat
    source_builder = 'synthetic legacy carrier derived from current lifecycle schema'
    privacy = [ordered]@{ api_key_recorded = $false; raw_arguments_recorded = $false; patch_content_recorded = $false }
    summaries = @($summaries)
    events = @($events)
}
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText($OutputPath, ($result | ConvertTo-Json -Depth 80), [System.Text.UTF8Encoding]::new($false))
Write-Host "R7NestedPatchControlProbe: $OutputPath"

if (@($summaries | Where-Object { $_.http_200 -ne $_.requests }).Count -gt 0) { exit 2 }
