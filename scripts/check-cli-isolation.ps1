param(
    [string]$WhaleInstallDir,
    [switch]$UseCurrentProcessPath,
    [switch]$SkipVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($WhaleInstallDir)) {
    if ([string]::IsNullOrWhiteSpace($env:WHALE_INSTALL_DIR)) {
        $WhaleInstallDir = Join-Path $env:USERPROFILE ".whale\bin"
    } else {
        $WhaleInstallDir = $env:WHALE_INSTALL_DIR
    }
}

$ExpectedWhaleRoot = [System.IO.Path]::GetFullPath($WhaleInstallDir).TrimEnd("\")
$Violations = New-Object System.Collections.Generic.List[string]

if (-not $UseCurrentProcessPath) {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $MachinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $PathParts = @($ExpectedWhaleRoot, $UserPath, $MachinePath, $env:Path) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    $env:Path = $PathParts -join ";"
}

function Test-IsUnderRoot {
    param(
        [string]$Path,
        [string]$Root
    )

    $Resolved = [System.IO.Path]::GetFullPath($Path).TrimEnd("\")
    return $Resolved.Equals($Root, [System.StringComparison]::OrdinalIgnoreCase) -or
        $Resolved.StartsWith("$Root\", [System.StringComparison]::OrdinalIgnoreCase)
}

$WhaleCommands = @(Get-Command whale -All -ErrorAction SilentlyContinue)
if ($WhaleCommands.Count -eq 0) {
    $Violations.Add("whale is not on PATH")
}

foreach ($Command in $WhaleCommands) {
    if (-not (Test-IsUnderRoot -Path $Command.Source -Root $ExpectedWhaleRoot)) {
        $Violations.Add("whale resolves outside isolated Whale bin: $($Command.Source)")
    }
}

$CodexCommands = @(Get-Command codex -All -ErrorAction SilentlyContinue)
foreach ($Command in $CodexCommands) {
    if ((Test-IsUnderRoot -Path $Command.Source -Root $ExpectedWhaleRoot) -or
        $Command.Source -match "\\\.whale\\") {
        $Violations.Add("codex resolves into Whale-managed path: $($Command.Source)")
    }
}

if (-not $SkipVersion) {
    if ($WhaleCommands.Count -gt 0) {
        & whale --version | Out-Host
    }
    if ($CodexCommands.Count -gt 0) {
        & codex --version | Out-Host
    }
}

if ($Violations.Count -gt 0) {
    $Violations | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host "CLI isolation OK"
Write-Host "Whale root: $ExpectedWhaleRoot"
Write-Host "Whale commands:"
$WhaleCommands | Select-Object Source | Format-Table -AutoSize
Write-Host "Codex commands:"
$CodexCommands | Select-Object Source | Format-Table -AutoSize
