param(
    [string]$BinaryPath,
    [string]$InstallDir,
    [switch]$PersistUserPath,
    [switch]$BackupLegacyCopies
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    if ([string]::IsNullOrWhiteSpace($env:WHALE_INSTALL_DIR)) {
        $InstallDir = Join-Path $env:USERPROFILE ".whale\bin"
    } else {
        $InstallDir = $env:WHALE_INSTALL_DIR
    }
}

function Resolve-ExistingFile {
    param([string[]]$Candidates)

    foreach ($Candidate in $Candidates) {
        if (-not [string]::IsNullOrWhiteSpace($Candidate) -and (Test-Path -LiteralPath $Candidate -PathType Leaf)) {
            return (Resolve-Path -LiteralPath $Candidate).Path
        }
    }

    throw "Cannot find whale.exe. Build first or pass -BinaryPath."
}

function Assert-IsolatedInstallDir {
    param([string]$Path)

    $Resolved = [System.IO.Path]::GetFullPath($Path).TrimEnd("\")
    $Forbidden = @(
        (Join-Path $env:USERPROFILE ".cargo\bin"),
        (Join-Path $env:USERPROFILE ".local\bin"),
        (Join-Path $env:APPDATA "npm"),
        (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps"),
        "$env:ProgramFiles\WindowsApps"
    ) | ForEach-Object { [System.IO.Path]::GetFullPath($_).TrimEnd("\") }

    foreach ($Root in $Forbidden) {
        if ($Resolved.Equals($Root, [System.StringComparison]::OrdinalIgnoreCase) -or
            $Resolved.StartsWith("$Root\", [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to install Whale into shared or official CLI path: $Resolved"
        }
    }
}

function Add-ProcessPath {
    param([string]$Path)

    $Parts = @($env:Path -split ";" | Where-Object { $_ })
    if ($Parts -notcontains $Path) {
        $env:Path = (@($Path) + $Parts) -join ";"
    }
}

function Add-UserPath {
    param([string]$Path)

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $Parts = @()
    if (-not [string]::IsNullOrWhiteSpace($UserPath)) {
        $Parts = @($UserPath -split ";" | Where-Object { $_ })
    }
    if ($Parts -notcontains $Path) {
        [Environment]::SetEnvironmentVariable("Path", (@($Path) + $Parts) -join ";", "User")
    }
}

function Backup-LegacyWhale {
    param(
        [string]$Path,
        [string]$BackupRoot
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return
    }

    New-Item -ItemType Directory -Force $BackupRoot | Out-Null
    $BinDir = Split-Path -Parent $Path
    $ToolRoot = Split-Path -Leaf (Split-Path -Parent $BinDir)
    $ToolRoot = $ToolRoot -replace "[^A-Za-z0-9._-]", "_"
    $Name = "whale-$ToolRoot-$(Get-Date -Format 'yyyyMMddHHmmss').exe"
    $Destination = Join-Path $BackupRoot $Name
    $Index = 1
    while (Test-Path -LiteralPath $Destination) {
        $Destination = Join-Path $BackupRoot ("whale-$ToolRoot-$(Get-Date -Format 'yyyyMMddHHmmss')-$Index.exe")
        $Index += 1
    }
    Move-Item -LiteralPath $Path -Destination $Destination
}

Assert-IsolatedInstallDir -Path $InstallDir

$Candidates = @()
if (-not [string]::IsNullOrWhiteSpace($BinaryPath)) {
    $Candidates += $BinaryPath
}
if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    $Candidates += (Join-Path $env:CARGO_TARGET_DIR "debug\whale.exe")
    $Candidates += (Join-Path $env:CARGO_TARGET_DIR "release\whale.exe")
    $Candidates += (Join-Path $env:CARGO_TARGET_DIR "dist\whale.exe")
}
$Candidates += (Join-Path $RepoRoot "third_party\codex-cli\codex-rs\target\debug\whale.exe")
$Candidates += (Join-Path $RepoRoot "third_party\codex-cli\codex-rs\target\release\whale.exe")
$Candidates += (Join-Path $RepoRoot "third_party\codex-cli\codex-rs\target\dist\whale.exe")

$Source = Resolve-ExistingFile -Candidates $Candidates
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
$Destination = Join-Path $InstallDir "whale.exe"

New-Item -ItemType Directory -Force $InstallDir | Out-Null
Copy-Item -LiteralPath $Source -Destination $Destination -Force

if ($BackupLegacyCopies) {
    $BackupRoot = Join-Path $env:USERPROFILE ".whale\backups\legacy-bin"
    @(
        (Join-Path $env:USERPROFILE ".cargo\bin\whale.exe"),
        (Join-Path $env:USERPROFILE ".local\bin\whale.exe")
    ) | Where-Object {
        -not $_.Equals($Destination, [System.StringComparison]::OrdinalIgnoreCase)
    } | ForEach-Object {
        Backup-LegacyWhale -Path $_ -BackupRoot $BackupRoot
    }
}

Add-ProcessPath -Path $InstallDir
if ($PersistUserPath) {
    Add-UserPath -Path $InstallDir
}

Write-Host "Installed Whale: $Destination"
Write-Host "Source: $Source"
Write-Host "Hash:"
Get-FileHash -LiteralPath $Destination -Algorithm SHA256 | Select-Object Path, Hash
