$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-provider-token-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

function Write-WireFixture([string]$Path, $Terminal) {
    $shape = [pscustomobject]@{
        schema_version = "provider-chat-wire-trace-v10"
        event_name = "provider.chat_wire_shape_recorded"
        request_id = "request-1"
        logical_request_id = "logical-1"
        attempt_seq = 1
        transport = "responses_http"
        request_index = 1
        provider_wire_api = "ChatCompletions"
        lcp_message_count = 0
        message_shapes = @()
        taskspace_final_receipt_identity = @{ count = 0; receipts = @() }
    }
    [IO.File]::WriteAllLines(
        $Path,
        @($shape, $Terminal | ForEach-Object {
                $_ | ConvertTo-Json -Compress -Depth 30
            }),
        [Text.UTF8Encoding]::new($false)
    )
}

function New-Terminal {
    [pscustomobject]@{
        schema_version = "provider-chat-wire-trace-v10"
        event_name = "provider.chat_wire_request_terminal"
        request_id = "request-1"
        logical_request_id = "logical-1"
        attempt_seq = 1
        transport = "responses_http"
        status = "response_completed"
        input_tokens = 100
        cached_input_tokens = 20
        output_tokens = 12
        reasoning_output_tokens = 5
        total_tokens = 112
    }
}

function Assert-InvalidTokenFixture([string]$Name, [scriptblock]$Mutation) {
    $terminal = New-Terminal
    & $Mutation $terminal
    $path = Join-Path $tempRoot "$Name.jsonl"
    Write-WireFixture $path $terminal
    $rejected = $false
    try {
        Get-R7WireRequestInventory $path | Out-Null
    } catch {
        $rejected = $_.Exception.Message -match "incomplete physical request rows"
    }
    if (-not $rejected) {
        throw "Provider token fixture was accepted: $Name"
    }
}

try {
    $validPath = Join-Path $tempRoot "valid.jsonl"
    Write-WireFixture $validPath (New-Terminal)
    $valid = @(Get-R7WireRequestInventory $validPath)
    if ($valid.Count -ne 1 -or
        [double]$valid[0].output_tokens -ne 12 -or
        [double]$valid[0].reasoning_output_tokens -ne 5 -or
        [double]$valid[0].total_tokens -ne 112) {
        throw "Provider request token identity was not preserved"
    }
    Assert-InvalidTokenFixture "missing-output" {
        param($terminal) $terminal.PSObject.Properties.Remove("output_tokens")
    }
    Assert-InvalidTokenFixture "string-output" {
        param($terminal) $terminal.output_tokens = "12"
    }
    Assert-InvalidTokenFixture "fractional-output" {
        param($terminal)
        $terminal.input_tokens = 100.5
        $terminal.cached_input_tokens = 20.25
        $terminal.output_tokens = 12.5
        $terminal.reasoning_output_tokens = 5.25
        $terminal.total_tokens = 113.0
    }
    Assert-InvalidTokenFixture "precision-loss" {
        param($terminal)
        $terminal.input_tokens = [int64]9007199254740993
        $terminal.cached_input_tokens = 0
        $terminal.output_tokens = 1
        $terminal.reasoning_output_tokens = 0
        $terminal.total_tokens = [int64]9007199254740993
    }
    Assert-InvalidTokenFixture "reasoning-over-output" {
        param($terminal) $terminal.reasoning_output_tokens = 13
    }
    Assert-InvalidTokenFixture "cached-over-input" {
        param($terminal) $terminal.cached_input_tokens = 101
    }
    Assert-InvalidTokenFixture "total-mismatch" {
        param($terminal) $terminal.total_tokens = 111
    }
    $stringIdentity = New-Terminal
    $stringIdentity.attempt_seq = "1"
    $identityPath = Join-Path $tempRoot "string-identity.jsonl"
    $shape = [pscustomobject]@{
        schema_version = "provider-chat-wire-trace-v10"
        event_name = "provider.chat_wire_shape_recorded"
        request_id = "request-1"
        logical_request_id = "logical-1"
        attempt_seq = "1"
        transport = "responses_http"
        request_index = "1"
        provider_wire_api = "ChatCompletions"
        lcp_message_count = 0
        message_shapes = @()
        taskspace_final_receipt_identity = @{ count = 0; receipts = @() }
    }
    [IO.File]::WriteAllLines(
        $identityPath,
        @($shape, $stringIdentity | ForEach-Object {
                $_ | ConvertTo-Json -Compress -Depth 30
            }),
        [Text.UTF8Encoding]::new($false)
    )
    $identityRejected = $false
    try {
        Get-R7WireRequestInventory $identityPath | Out-Null
    } catch {
        $identityRejected = $_.Exception.Message -match (
            "invalid nonnegative Int64|incomplete physical request rows"
        )
    }
    if (-not $identityRejected) {
        throw "Provider string request identity was accepted"
    }
    $missingRejected = $false
    try {
        Get-R7ExactInt64Sum @($null) "input_tokens" | Out-Null
    } catch {
        $missingRejected =
            $_.Exception.Message -eq "R7 exact sum contains a missing input_tokens"
    }
    if (-not $missingRejected) {
        throw "Missing aggregate token fact was treated as zero"
    }
    $largeSum = Get-R7ExactInt64Sum @(
        [int64]"9007199254740991",
        [int64]1
    ) "input_tokens"
    if ($largeSum -ne [int64]"9007199254740992") {
        throw "Exact aggregate token sum lost large integer identity"
    }
    Write-Output "R7 provider token identity passed."
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -Force -Recurse -LiteralPath $tempRoot
    }
}
