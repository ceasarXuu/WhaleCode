$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/request-facts.ps1")
. (Join-Path $PSScriptRoot "lib/r7-request-facts-provenance.ps1")

$root = Join-Path $repoRoot "target/r7-request-facts-provenance-selftest"
$artifactDir = Join-Path $root ([guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
$wirePath = Join-Path $artifactDir "provider-wire-trace.jsonl"
$factsPath = Join-Path $artifactDir "request-facts.json"
$digest = ("a" * 64) -join ""
@(
    [ordered]@{
        schema_version = "provider-chat-wire-trace-v10"; status = "payload_captured"
        request_id = "request-1"; logical_request_id = "logical-1"; attempt_seq = 1
        request_index = 1; provider_payload_sha256 = $digest
    },
    [ordered]@{
        schema_version = "provider-chat-wire-trace-v10"; status = "response_completed"
        request_id = "request-1"; logical_request_id = "logical-1"; attempt_seq = 1
        input_tokens = 100; cached_input_tokens = 20; output_tokens = 10
        reasoning_output_tokens = 2; total_tokens = 110
    }
) | ForEach-Object { $_ | ConvertTo-Json -Compress } |
    Set-Content -LiteralPath $wirePath -Encoding UTF8
Invoke-TaskspaceRequestFactsGenerator -WireTracePath $wirePath -OutputPath $factsPath | Out-Null

$identity = Get-R7RequestFactsIdentity $factsPath $artifactDir
if ([string]$identity.analyzer.version -ne "i07-review-fixes-v3" -or
    [string]$identity.sources.wire.status -ne "read" -or
    [string]$identity.sources.boundary.status -ne "unavailable" -or
    [string]$identity.boundary_identity.lifecycle_status -ne "unavailable") {
    throw "Request facts provenance identity was not sealed"
}

Add-Content -LiteralPath $wirePath -Encoding UTF8 -Value " "
$sourceRejected = $false
try { Get-R7RequestFactsIdentity $factsPath $artifactDir | Out-Null } catch {
    $sourceRejected = $_.Exception.Message -match "source hash is stale"
}
if (-not $sourceRejected) { throw "Changed request source was accepted" }

Write-Output "R7 request facts provenance self-test passed."
