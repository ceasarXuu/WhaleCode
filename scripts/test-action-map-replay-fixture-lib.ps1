function New-TestActionMapReplayWhale {
    param([Parameter(Mandatory = $true)][string]$RunRoot)
    [void](New-Item -ItemType Directory -Force -Path $RunRoot)
    $producer = Join-Path $RunRoot "fake-taskspace-replay.ps1"
    @'
$ErrorActionPreference = "Stop"
$rolloutPath = ""
$outputPath = ""
for ($index = 0; $index -lt $args.Count; $index++) {
    if ($args[$index] -eq "--rollout") { $rolloutPath = [string]$args[$index + 1] }
    if ($args[$index] -eq "--output") { $outputPath = [string]$args[$index + 1] }
}
if (-not $rolloutPath -or -not $outputPath) { throw "fake replay requires --rollout and --output" }
$snapshot = $null
$checkpointCount = 0
foreach ($line in Get-Content -LiteralPath $rolloutPath -Encoding UTF8) {
    try { $row = $line | ConvertFrom-Json } catch { continue }
    if ($row.payload -and $row.payload.snapshot) {
        $snapshot = $row.payload.snapshot
        $checkpointCount++
    }
}
if ($null -eq $snapshot) {
    [ordered]@{
        schema_version = "TaskSpaceReplayProofR6V1"
        status = "error"
        error = [ordered]@{ code = "missing_checkpoint"; message = "fixture has no snapshot" }
    } | ConvertTo-Json -Depth 50 | Set-Content -LiteralPath $outputPath -Encoding UTF8
    exit 1
}
[ordered]@{
    schema_version = "TaskSpaceReplayProofR6V1"
    status = "ok"
    proof = [ordered]@{
        rollout_sha256 = "fixture"
        final_snapshot_sha256 = "fixture"
        checkpoint_id = "fixture-checkpoint"
        base_snapshot_sha256 = "fixture"
        parsed_checkpoint_count = $checkpointCount
        parsed_delta_count = 0
        surviving_checkpoint_count = $checkpointCount
        surviving_delta_count = 0
        active_checkpoint_id = "fixture-checkpoint"
        active_chain_applied_delta_count = 0
        active_chain_last_delta_sequence = $null
    }
    snapshot = $snapshot
} | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $outputPath -Encoding UTF8
'@ | Set-Content -LiteralPath $producer -Encoding UTF8

    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        $wrapper = Join-Path $RunRoot "fake-whale.cmd"
        "@echo off`r`npowershell -NoProfile -ExecutionPolicy Bypass -File `"$producer`" %*`r`n" | Set-Content -LiteralPath $wrapper -Encoding ASCII
        return $wrapper
    }
    $wrapper = Join-Path $RunRoot "fake-whale"
    "#!/usr/bin/env bash`nexec pwsh -NoProfile -File '$producer' `"`$@`"`n" | Set-Content -LiteralPath $wrapper -Encoding ASCII
    & chmod +x $wrapper
    $wrapper
}
