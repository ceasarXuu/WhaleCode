function New-TestActionMapStoreWhale {
    param([Parameter(Mandatory = $true)][string]$RunRoot)
    [void](New-Item -ItemType Directory -Force -Path $RunRoot)
    $producer = Join-Path $RunRoot "fake-taskspace-map-store.ps1"
    @'
$ErrorActionPreference = "Stop"
$threadId = ""
$outputPath = ""
for ($index = 0; $index -lt $args.Count; $index++) {
    if ($args[$index] -eq "--thread-id") { $threadId = [string]$args[$index + 1] }
    if ($args[$index] -eq "--output") { $outputPath = [string]$args[$index + 1] }
}
if (-not $threadId -or -not $outputPath) {
    throw "fake Map Store export requires --thread-id and --output"
}
$fixturePath = Join-Path $PSScriptRoot "map-store-export.json"
if (-not (Test-Path -LiteralPath $fixturePath -PathType Leaf)) {
    throw "fake Map Store fixture is missing: $fixturePath"
}
$fixture = Get-Content -Raw -Encoding UTF8 -LiteralPath $fixturePath | ConvertFrom-Json
if ([string]$fixture.binding.thread_id -ne $threadId) {
    throw "fake Map Store fixture thread '$($fixture.binding.thread_id)' does not match '$threadId'"
}
Copy-Item -LiteralPath $fixturePath -Destination $outputPath -Force
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

function Set-TestActionMapStoreFixture {
    param(
        [Parameter(Mandatory = $true)][string]$WhalePath,
        [Parameter(Mandatory = $true)][string]$ThreadId,
        [Parameter(Mandatory = $true)][object]$Snapshot
    )
    $mapId = if ($Snapshot.map -and $Snapshot.map.id) {
        [string]$Snapshot.map.id
    }
    elseif ($Snapshot.maps -and @($Snapshot.maps).Count -gt 0) {
        [string]$Snapshot.maps[0].id
    }
    else {
        "map-$ThreadId"
    }
    $fixture = [ordered]@{
        schema_version = "TaskSpaceMapExportR7V1"
        status = "ok"
        map = [ordered]@{
            map_id = $mapId
            owner_thread_id = $ThreadId
            snapshot = $Snapshot
            snapshot_sha256 = "fixture"
            store_revision = 1
            graph_revision = 1
            complete = $false
            created_at_ms = 1
            updated_at_ms = 1
        }
        binding = [ordered]@{
            thread_id = $ThreadId
            map_id = $mapId
            relation = "owner"
            parent_thread_id = $null
            node_id = $null
            lease_id = $null
            created_at_ms = 1
            updated_at_ms = 1
        }
    }
    $fixturePath = Join-Path (Split-Path -Parent (Resolve-Path -LiteralPath $WhalePath).Path) "map-store-export.json"
    $fixture | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $fixturePath -Encoding UTF8
}
