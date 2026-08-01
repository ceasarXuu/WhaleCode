param(
    [Parameter(Mandatory = $true)][string]$WhaleBin,
    [string]$BuildCommand = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")

$path = Write-TaskspaceWhaleBinaryAttestation `
    -WhaleBin ([System.IO.Path]::GetFullPath($WhaleBin)) `
    -RepoRoot $repoRoot `
    -BuildCommand $BuildCommand

Write-Host "WhaleBinaryAttestation: $path"
