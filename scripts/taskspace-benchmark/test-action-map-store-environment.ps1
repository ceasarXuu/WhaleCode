$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $repoRoot "scripts/action-map-store-export-lib.ps1")

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "taskspace-map-home-$([guid]::NewGuid().ToString('N'))"
$whaleHome = Join-Path $tempRoot "home/.whale"
New-Item -ItemType Directory -Force -Path $whaleHome | Out-Null
$previousWhaleHome = [Environment]::GetEnvironmentVariable("WHALE_HOME", "Process")
$previousSqliteHome = [Environment]::GetEnvironmentVariable("CODEX_SQLITE_HOME", "Process")

try {
    $env:WHALE_HOME = "outer-whale-home"
    $env:CODEX_SQLITE_HOME = "outer-sqlite-home"
    $resolved = (Resolve-Path -LiteralPath $whaleHome).Path
    $observed = Invoke-WithActionMapStoreHome -WhaleHome $whaleHome -Operation {
        param($activeWhaleHome)
        [pscustomobject]@{
            active_home = $activeWhaleHome
            whale_home = $env:WHALE_HOME
            sqlite_home = $env:CODEX_SQLITE_HOME
        }
    }
    if ([string]$observed.active_home -ne $resolved -or
        [string]$observed.whale_home -ne $resolved -or
        [string]$observed.sqlite_home -ne $resolved) {
        throw "Run-scoped Map Store environment was not applied"
    }
    if ($env:WHALE_HOME -ne "outer-whale-home" -or $env:CODEX_SQLITE_HOME -ne "outer-sqlite-home") {
        throw "Map Store environment leaked into the caller"
    }
    Write-Output "Action Map Store environment contract passed."
}
finally {
    if ($null -eq $previousWhaleHome) { Remove-Item Env:\WHALE_HOME -ErrorAction SilentlyContinue }
    else { $env:WHALE_HOME = $previousWhaleHome }
    if ($null -eq $previousSqliteHome) { Remove-Item Env:\CODEX_SQLITE_HOME -ErrorAction SilentlyContinue }
    else { $env:CODEX_SQLITE_HOME = $previousSqliteHome }
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -Recurse -Force -LiteralPath $tempRoot
    }
}
