param(
    [switch]$SkipCliPathCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Violations = New-Object System.Collections.Generic.List[string]
$Warnings = New-Object System.Collections.Generic.List[string]

function Require-FileContains {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Message
    )

    $FullPath = Join-Path $RepoRoot $Path
    if (-not (Test-Path -LiteralPath $FullPath -PathType Leaf)) {
        $Violations.Add("Missing expected file: $Path")
        return
    }

    if (-not (Select-String -LiteralPath $FullPath -Pattern $Pattern -Quiet)) {
        $Violations.Add($Message)
    }
}

function Require-FileNotContains {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Message
    )

    $FullPath = Join-Path $RepoRoot $Path
    if (-not (Test-Path -LiteralPath $FullPath -PathType Leaf)) {
        return
    }

    if (Select-String -LiteralPath $FullPath -Pattern $Pattern -Quiet) {
        $Violations.Add($Message)
    }
}

function Test-EnvPathCollision {
    param(
        [string]$LeftName,
        [string]$LeftPath,
        [string]$RightName,
        [string]$RightPath
    )

    if ([string]::IsNullOrWhiteSpace($LeftPath) -or [string]::IsNullOrWhiteSpace($RightPath)) {
        return
    }

    $LeftFull = [System.IO.Path]::GetFullPath($LeftPath).TrimEnd("\")
    $RightFull = [System.IO.Path]::GetFullPath($RightPath).TrimEnd("\")
    if ($LeftFull.Equals($RightFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        $Violations.Add("$LeftName and $RightName point to the same path: $LeftFull")
    }
}

Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\utils\home-dir\src\lib.rs" `
    -Pattern 'std::env::var\("WHALE_HOME"\)' `
    -Message "Runtime home lookup must use WHALE_HOME, not CODEX_HOME."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\utils\home-dir\src\lib.rs" `
    -Pattern 'p\.push\("\.whale"\)' `
    -Message "Default runtime home must be .whale."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\utils\home-dir\src\lib.rs" `
    -Pattern 'official Codex state directory' `
    -Message "Runtime home lookup must reject official Codex state directories."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\login\src\auth\storage.rs" `
    -Pattern 'KEYRING_SERVICE: &str = "Whale Auth"' `
    -Message "Auth keyring service must be Whale-scoped."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\secrets\src\lib.rs" `
    -Pattern 'KEYRING_SERVICE: &str = "whale"' `
    -Message "Local secrets keyring service must be Whale-scoped."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\cli\Cargo.toml" `
    -Pattern 'name = "whale"' `
    -Message "Rust CLI binary must be whale."
Require-FileContains `
    -Path "third_party\codex-cli\codex-cli\package.json" `
    -Pattern '"name": "@ceasarxuu/whalecode"' `
    -Message "npm CLI package must be named @ceasarxuu/whalecode, not @openai/codex."
Require-FileContains `
    -Path "third_party\codex-cli\codex-cli\package.json" `
    -Pattern '"whale": "bin/whale\.js"' `
    -Message "npm CLI package must expose the whale command."
Require-FileContains `
    -Path "third_party\codex-cli\codex-cli\bin\whale.js" `
    -Pattern 'whalecode-win32-x64' `
    -Message "npm CLI launcher must resolve Whale platform packages."
Require-FileContains `
    -Path "third_party\codex-cli\codex-cli\bin\whale.js" `
    -Pattern 'WHALE_MANAGED_BY_NPM' `
    -Message "npm CLI launcher must mark Whale npm-managed launches with Whale env vars."
Require-FileContains `
    -Path "third_party\codex-cli\codex-cli\scripts\build_npm_package.py" `
    -Pattern 'WHALE_NPM_NAME = "@ceasarxuu/whalecode"' `
    -Message "npm package builder must stage the Whale package name."
Require-FileContains `
    -Path "third_party\codex-cli\codex-rs\tui\src\update_action.rs" `
    -Pattern '@ceasarxuu/whalecode@latest' `
    -Message "TUI npm update command must target the Whale npm package."

$NpmCodexLauncher = Join-Path $RepoRoot "third_party\codex-cli\codex-cli\bin\codex.js"
if (Test-Path -LiteralPath $NpmCodexLauncher -PathType Leaf) {
    $Violations.Add("npm CLI package must not retain a codex.js launcher.")
}
Require-FileNotContains `
    -Path "third_party\codex-cli\codex-cli\package.json" `
    -Pattern '@openai/codex|bin/codex\.js' `
    -Message "npm CLI package metadata must not point at official Codex package names or launcher paths."
Require-FileNotContains `
    -Path "third_party\codex-cli\codex-cli\bin\whale.js" `
    -Pattern '@openai/codex|CODEX_MANAGED_BY_NPM|CODEX_MANAGED_BY_BUN|codex\.exe' `
    -Message "npm CLI launcher must not resolve official Codex packages, env vars, or binary names."
Require-FileNotContains `
    -Path "third_party\codex-cli\codex-cli\scripts\build_npm_package.py" `
    -Pattern '@openai/codex|CODEX_NPM_NAME|CODEX_PLATFORM_PACKAGES|bin/codex\.js' `
    -Message "npm package builder must not stage official Codex package names."
Require-FileNotContains `
    -Path "third_party\codex-cli\codex-cli\README.md" `
    -Pattern '@openai/codex|npm install -g @openai|bin/codex\.js' `
    -Message "npm package README must not publish official Codex install instructions."

$ForbiddenCerts = Get-ChildItem -Path $RepoRoot -Recurse -File -Include `
    *.pfx,*.p12,*.pem,*.key,*.crt,*.cer,*.der,*.snk,*.keystore,*.jks,*.mobileprovision,*.entitlements `
    -ErrorAction SilentlyContinue |
    Where-Object {
        $_.FullName -notlike "*\third_party\codex-cli\codex-rs\codex-client\tests\fixtures\test-*.pem"
    }
foreach ($File in $ForbiddenCerts) {
    $Violations.Add("Potential signing key/certificate material in repo: $($File.FullName)")
}

Test-EnvPathCollision `
    -LeftName "WHALE_HOME" `
    -LeftPath $env:WHALE_HOME `
    -RightName "CODEX_HOME" `
    -RightPath $env:CODEX_HOME

if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    $TargetDir = [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR).TrimEnd("\")
    $ForbiddenBuildRoots = @(
        (Join-Path $env:USERPROFILE ".codex"),
        (Join-Path $env:APPDATA "npm"),
        "$env:ProgramFiles\WindowsApps"
    ) | ForEach-Object { [System.IO.Path]::GetFullPath($_).TrimEnd("\") }

    foreach ($Root in $ForbiddenBuildRoots) {
        if ($TargetDir.Equals($Root, [System.StringComparison]::OrdinalIgnoreCase) -or
            $TargetDir.StartsWith("$Root\", [System.StringComparison]::OrdinalIgnoreCase)) {
            $Violations.Add("CARGO_TARGET_DIR is under an official Codex or shared package path: $TargetDir")
        }
    }
}

if (-not $SkipCliPathCheck) {
    & (Join-Path $PSScriptRoot "check-cli-isolation.ps1")
}

foreach ($Warning in $Warnings) {
    Write-Warning $Warning
}

if ($Violations.Count -gt 0) {
    $Violations | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host "Codex collision risk check OK"
