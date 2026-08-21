param(
    [switch]$SkipCliPathCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$DistributionGuard = Join-Path $RepoRoot "scripts/release/check_distribution_identity.py"

& python3 $DistributionGuard --repo-root $RepoRoot
if ($LASTEXITCODE -ne 0) {
    throw "Whale distribution identity guard failed."
}

if (-not $SkipCliPathCheck -and $IsWindows) {
    & (Join-Path $PSScriptRoot "check-cli-isolation.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "Whale/Codex CLI path isolation check failed."
    }
}

Write-Host "Codex collision risk check OK"
