param(
    [string]$Model = 'deepseek-v4-flash',
    [string]$Endpoint = 'https://api.deepseek.com/chat/completions',
    [ValidateRange(1, 20)][int]$Repeat = 6,
    [ValidateSet('sibling_control_first', 'sibling_required_next_call', 'direct_with_control_visible', 'sibling_minimal_control', 'sibling_lean_control', 'sibling_patch_first', 'direct_only')]
    [string]$Arm = 'sibling_control_first',
    [string]$OutputPath = '',
    [string]$FixturePath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $runId = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss-fff')
    $OutputPath = Join-Path $repoRoot "target/r7-sibling-patch-sequence-probe/$runId/provider-capability.json"
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

function Get-ProductionSequenceTools {
    Push-Location (Join-Path $repoRoot 'third_party/codex-cli/codex-rs')
    try {
        $json = & cargo run --quiet -p codex-tools --example r7_response_tool_sequence_schema 2>$null
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace([string]$json)) {
            throw 'production response tool sequence schema exporter failed'
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

function New-MinimalControlTool {
    [ordered]@{
        type = 'function'
        function = [ordered]@{
            name = 'taskspace_control'
            description = 'Commit the declared lifecycle handoff. For next_apply_patch, emit apply_patch immediately after this call in the same response.'
            strict = $false
            parameters = [ordered]@{
                type = 'object'
                properties = [ordered]@{
                    action = [ordered]@{ type = 'string'; enum = @('complete_then_continue') }
                    expected_revision = [ordered]@{ type = 'integer' }
                    current_node_id = [ordered]@{ type = 'string' }
                    next_node_id = [ordered]@{ type = 'string' }
                    continuation = [ordered]@{ type = 'string'; enum = @('next_apply_patch') }
                }
                required = @('action', 'expected_revision', 'current_node_id', 'next_node_id', 'continuation')
                additionalProperties = $false
            }
        }
    }
}

function New-LeanControlTool {
    param($CurrentTool)
    Copy-JsonValue $CurrentTool
}

function New-RequiredNextCallControlTool {
    param($CurrentTool)
    $control = Copy-JsonValue $CurrentTool
    foreach ($actionVariant in @($control.function.parameters.anyOf)) {
        $continuation = $actionVariant.properties.continuation
        if ($null -eq $continuation) { continue }
        $requiredNextCall = Copy-JsonValue $continuation
        $requiredNextCall.enum = @($requiredNextCall.enum | ForEach-Object {
                if ($_ -eq 'next_apply_patch') { 'apply_patch' } else { 'ordinary_tool' }
            })
        $requiredNextCall.description = 'Declaration only: emit the selected top-level sibling immediately after taskspace_control in this same response. This field does not execute or schedule that call.'
        $actionVariant.properties | Add-Member -NotePropertyName 'required_next_call' -NotePropertyValue $requiredNextCall
        $actionVariant.properties.PSObject.Properties.Remove('continuation')
        $actionVariant.required = @($actionVariant.required | ForEach-Object {
                if ($_ -eq 'continuation') { 'required_next_call' } else { $_ }
            })
    }
    $control.function.description += ' required_next_call only declares the immediately following top-level sibling. It never executes or schedules that sibling; emit both calls in this response.'
    $control
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

function Get-FixtureInvocation {
    param($Fixture, [int]$RepeatIndex)
    $row = @($Fixture.responses | Where-Object { [int]$_.repeat -eq $RepeatIndex })
    if ($row.Count -ne 1) { throw "fixture response missing for repeat $RepeatIndex" }
    [ordered]@{ status = [int]$row[0].http_status; payload = $row[0].payload; duration_ms = 1 }
}

function Invoke-ProbeRequest {
    param($Body, $Fixture, [int]$RepeatIndex)
    if ($null -ne $Fixture) { return Get-FixtureInvocation $Fixture $RepeatIndex }
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

function ConvertFrom-JsonOrNull {
    param([AllowEmptyString()][string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) { return $null }
    try { $Text | ConvertFrom-Json -Depth 80 } catch { $null }
}

function Get-Observation {
    param($Invocation, [string]$ExpectedPatch, [string[]]$ExpectedCallNames)
    $calls = if ($null -eq $Invocation.payload) { @() } else { @($Invocation.payload.choices[0].message.tool_calls) }
    $controlCall = @($calls | Where-Object { $_.function.name -eq 'taskspace_control' })
    $patchCall = @($calls | Where-Object { $_.function.name -eq 'apply_patch' })
    $controlArguments = if ($controlCall.Count -eq 1) { [string]$controlCall[0].function.arguments } else { '' }
    $patchArguments = if ($patchCall.Count -eq 1) { [string]$patchCall[0].function.arguments } else { '' }
    $control = ConvertFrom-JsonOrNull $controlArguments
    $patch = ConvertFrom-JsonOrNull $patchArguments
    $patchInput = if ($null -eq $patch) { '' } else { [string]$patch.input }
    $continuationKind = if ($null -eq $control) { '' } elseif ($null -ne $control.required_next_call) {
        [string]$control.required_next_call
    } elseif ($control.continuation -is [string]) {
        [string]$control.continuation
    } else { [string]$control.continuation.kind }
    $callNames = @($calls | ForEach-Object { [string]$_.function.name })
    $callNamesMatch = ($callNames.Count -eq $ExpectedCallNames.Count) -and
        -not (Compare-Object -ReferenceObject $ExpectedCallNames -DifferenceObject $callNames -SyncWindow 0)
    [ordered]@{
        http_status = [int]$Invocation.status
        duration_ms = [int64]$Invocation.duration_ms
        call_count = $calls.Count
        call_names = $callNames
        expected_call_names_match = $callNamesMatch
        control_json_valid = $null -ne $control
        control_shape_valid = $null -ne $control -and [string]$control.action -eq 'complete_then_continue' -and $continuationKind -in @('next_apply_patch', 'apply_patch')
        patch_json_valid = $null -ne $patch
        patch_exact = $patchInput -ceq $ExpectedPatch
        patch_bytes = Get-Utf8Bytes $patchInput
        patch_sha256 = Get-Sha256 $patchInput
        control_arguments_bytes = Get-Utf8Bytes $controlArguments
        control_arguments_sha256 = Get-Sha256 $controlArguments
        patch_arguments_bytes = Get-Utf8Bytes $patchArguments
        patch_arguments_sha256 = Get-Sha256 $patchArguments
        usage = [ordered]@{
            input_tokens = [int64]$Invocation.payload.usage.prompt_tokens
            cached_input_tokens = [int64]$Invocation.payload.usage.prompt_cache_hit_tokens
            output_tokens = [int64]$Invocation.payload.usage.completion_tokens
        }
    }
}

$productionTools = Get-ProductionSequenceTools
$controlTool = ConvertTo-ChatTool $productionTools.taskspace_control
$patchTool = ConvertTo-ChatTool $productionTools.apply_patch
$fixture = if ([string]::IsNullOrWhiteSpace($FixturePath)) { $null } else {
    (Get-Content -Raw -LiteralPath $FixturePath) | ConvertFrom-Json -Depth 80
}
if ($null -eq $fixture) {
    Import-LocalCredentialIfNeeded
    if ([string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) {
        throw 'DEEPSEEK_API_KEY is required for the live sibling patch sequence probe'
    }
}

$controlInstruction = 'taskspace_control with action complete_then_continue, expected_revision 2, current_node_id explore, next_node_id fix, and required_next_call apply_patch'
$legacyControlInstruction = 'taskspace_control with action complete_then_continue, expected_revision 2, current_node_id explore, next_node_id fix, and continuation next_apply_patch'
$armConfig = switch ($Arm) {
    'sibling_control_first' {
        [ordered]@{
            tools = @($controlTool, $patchTool)
            expected_call_names = @('taskspace_control', 'apply_patch')
            prompt = "Call exactly two tools in this response and emit no prose.`nFirst call $controlInstruction.`nImmediately after it, call apply_patch and put the following exact patch in input.`n`n$largePatch"
        }
    }
    'sibling_required_next_call' {
        [ordered]@{
            tools = @((New-RequiredNextCallControlTool $controlTool), $patchTool)
            expected_call_names = @('taskspace_control', 'apply_patch')
            prompt = "Call exactly two tools in this response and emit no prose.`nFirst call taskspace_control with action complete_then_continue, expected_revision 2, current_node_id explore, next_node_id fix, and required_next_call apply_patch.`nImmediately after it, call apply_patch and put the following exact patch in input.`n`n$largePatch"
        }
    }
    'direct_with_control_visible' {
        [ordered]@{
            tools = @($controlTool, $patchTool)
            expected_call_names = @('apply_patch')
            prompt = "Call only apply_patch and emit no prose. Put the following exact patch in input.`n`n$largePatch"
        }
    }
    'sibling_minimal_control' {
        [ordered]@{
            tools = @((New-MinimalControlTool), $patchTool)
            expected_call_names = @('taskspace_control', 'apply_patch')
            prompt = "Call exactly two tools in this response and emit no prose.`nFirst call $legacyControlInstruction.`nImmediately after it, call apply_patch and put the following exact patch in input.`n`n$largePatch"
        }
    }
    'sibling_lean_control' {
        [ordered]@{
            tools = @((New-LeanControlTool $controlTool), $patchTool)
            expected_call_names = @('taskspace_control', 'apply_patch')
            prompt = "Call exactly two tools in this response and emit no prose.`nFirst call $controlInstruction.`nImmediately after it, call apply_patch and put the following exact patch in input.`n`n$largePatch"
        }
    }
    'sibling_patch_first' {
        [ordered]@{
            tools = @($controlTool, $patchTool)
            expected_call_names = @('apply_patch', 'taskspace_control')
            prompt = "Call exactly two tools in this response and emit no prose.`nFirst call apply_patch and put the following exact patch in input.`nImmediately after it, call $controlInstruction.`n`n$largePatch"
        }
    }
    'direct_only' {
        [ordered]@{
            tools = @($patchTool)
            expected_call_names = @('apply_patch')
            prompt = "Call only apply_patch and emit no prose. Put the following exact patch in input.`n`n$largePatch"
        }
    }
}
$tools = $armConfig.tools
$prompt = [string]$armConfig.prompt
$events = @()
for ($repeatIndex = 1; $repeatIndex -le $Repeat; $repeatIndex++) {
    $body = [ordered]@{
        model = $Model
        messages = @(
            [ordered]@{ role = 'system'; content = 'Diagnostic provider probe. Follow the tool schemas and requested top-level tool order exactly.' },
            [ordered]@{ role = 'user'; content = $prompt }
        )
        tools = $tools
        tool_choice = 'auto'
        thinking = [ordered]@{ type = 'enabled' }
        reasoning_effort = 'max'
        stream = $false
    }
    $invocation = Invoke-ProbeRequest $body $fixture $repeatIndex
    $events += [ordered]@{
        event_name = 'r7.sibling_patch_sequence_observed'
        repeat = $repeatIndex
        transport = 'non_streaming_chat_completions'
        request = [ordered]@{
            tools_sha256 = Get-Sha256 ($tools | ConvertTo-Json -Depth 80 -Compress)
            prompt_sha256 = Get-Sha256 $prompt
            expected_patch_bytes = Get-Utf8Bytes $largePatch
            expected_patch_sha256 = Get-Sha256 $largePatch
        }
        response = Get-Observation $invocation $largePatch $armConfig.expected_call_names
    }
}

$result = [ordered]@{
    schema_version = 'r7-sibling-patch-sequence-probe-v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    model = $Model
    arm = $Arm
    endpoint = if ($null -eq $fixture) { $Endpoint } else { 'fixture' }
    repeat = $Repeat
    privacy = [ordered]@{ api_key_recorded = $false; raw_arguments_recorded = $false; patch_content_recorded = $false }
    summary = [ordered]@{
        requests = $events.Count
        http_200 = @($events | Where-Object { $_.response.http_status -eq 200 }).Count
        expected_call_names_match = @($events | Where-Object { $_.response.expected_call_names_match }).Count
        control_shape_valid = @($events | Where-Object { $_.response.control_shape_valid }).Count
        patch_json_valid = @($events | Where-Object { $_.response.patch_json_valid }).Count
        patch_exact = @($events | Where-Object { $_.response.patch_exact }).Count
    }
    events = $events
}
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
[System.IO.File]::WriteAllText($OutputPath, ($result | ConvertTo-Json -Depth 80), [System.Text.UTF8Encoding]::new($false))
Write-Host "R7SiblingPatchSequenceProbe: $OutputPath"

if ($result.summary.http_200 -ne $Repeat) { exit 2 }
