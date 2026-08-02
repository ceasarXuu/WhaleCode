param(
    [Parameter(Mandatory = $true)][string]$WhaleBin,
    [Parameter(Mandatory = $true)][string]$RepoRoot,
    [Parameter(Mandatory = $true)][string]$OutputPath
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib\bootstrap.ps1") -RepoRoot $RepoRoot -BenchmarkRoot $PSScriptRoot

$health = New-TaskspaceWhaleBinaryHealth $WhaleBin $RepoRoot
Write-TaskspaceJson $health $OutputPath
if ([string]$health.status -ne "pass") {
    $finding = @($health.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    Write-Error "whale_binary_preflight_failed:$([string]$finding.stable_code)"
    exit 3
}
