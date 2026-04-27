param()

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$CargoToml = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\Cargo.toml"
$Workflow = Join-Path $RepoRoot "docs\runbooks\development-workflow.md"
$RustReleaseWorkflow = Join-Path $RepoRoot "third_party\codex-cli\.github\workflows\rust-release.yml"
$RustReleaseWindowsWorkflow = Join-Path $RepoRoot "third_party\codex-cli\.github\workflows\rust-release-windows.yml"
$MacosSignAction = Join-Path $RepoRoot "third_party\codex-cli\.github\actions\macos-code-sign\action.yml"
$WindowsSignAction = Join-Path $RepoRoot "third_party\codex-cli\.github\actions\windows-code-sign\action.yml"

function Get-ProfileSection {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Content,
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    $Pattern = "(?ms)^\[profile\.$([regex]::Escape($Name))\]\s*(.*?)(?=^\[|\z)"
    $Match = [regex]::Match($Content, $Pattern)
    if (-not $Match.Success) {
        throw "Missing [profile.$Name] in $CargoToml"
    }

    return $Match.Groups[1].Value
}

function Require-Value {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Section,
        [Parameter(Mandatory = $true)]
        [string] $Key,
        [Parameter(Mandatory = $true)]
        [string] $Expected,
        [Parameter(Mandatory = $true)]
        [string] $Message
    )

    $Pattern = "(?m)^\s*$([regex]::Escape($Key))\s*=\s*$([regex]::Escape($Expected))\s*(#.*)?$"
    if ($Section -notmatch $Pattern) {
        throw $Message
    }
}

$CargoContent = Get-Content -Path $CargoToml -Encoding UTF8 -Raw
$Release = Get-ProfileSection -Content $CargoContent -Name "release"
$Dist = Get-ProfileSection -Content $CargoContent -Name "dist"

Require-Value `
    -Section $Release `
    -Key "opt-level" `
    -Expected "1" `
    -Message "The default release profile must use a local-build opt level."
Require-Value `
    -Section $Release `
    -Key "lto" `
    -Expected "false" `
    -Message "The default release profile must keep LTO disabled for local build latency."
Require-Value `
    -Section $Release `
    -Key "incremental" `
    -Expected "true" `
    -Message "The default release profile must keep incremental compilation enabled locally."
Require-Value `
    -Section $Release `
    -Key "codegen-units" `
    -Expected "256" `
    -Message "The default release profile must keep parallel release codegen enabled."
Require-Value `
    -Section $Dist `
    -Key "inherits" `
    -Expected '"release"' `
    -Message "The dist profile must inherit release."
Require-Value `
    -Section $Dist `
    -Key "opt-level" `
    -Expected "3" `
    -Message "The dist profile must keep production opt level for final distribution."
Require-Value `
    -Section $Dist `
    -Key "lto" `
    -Expected '"fat"' `
    -Message "The dist profile must keep size-first fat LTO for final distribution."
Require-Value `
    -Section $Dist `
    -Key "incremental" `
    -Expected "false" `
    -Message "The dist profile must disable incremental state for final distribution."
Require-Value `
    -Section $Dist `
    -Key "codegen-units" `
    -Expected "1" `
    -Message "The dist profile must keep single-unit codegen for final distribution."

$WorkflowContent = Get-Content -Path $Workflow -Encoding UTF8 -Raw
if ($WorkflowContent -match "CARGO_PROFILE_RELEASE_LTO\s*=") {
    throw "Development workflow must not recommend process-local release-profile overrides."
}
if ($WorkflowContent -notmatch "cargo build -p codex-cli --bin whale --profile dist --locked") {
    throw "Development workflow must document the explicit dist profile command."
}

$ReleaseWorkflowContent = Get-Content -Path $RustReleaseWorkflow -Encoding UTF8 -Raw
$WindowsWorkflowContent = Get-Content -Path $RustReleaseWindowsWorkflow -Encoding UTF8 -Raw
if ($ReleaseWorkflowContent -notmatch "cargo build --target \$\{\{ matrix\.target \}\} --profile dist") {
    throw "Rust release workflow must build distribution artifacts with --profile dist."
}
if ($WindowsWorkflowContent -notmatch "cargo build --target \$\{\{ matrix\.target \}\} --profile dist") {
    throw "Windows release workflow must build distribution artifacts with --profile dist."
}
if ($ReleaseWorkflowContent -match "cargo build[^\r\n]*--release") {
    throw "Rust release workflow must not build distribution artifacts with --release."
}
if ($WindowsWorkflowContent -match "cargo build[^\r\n]*--release") {
    throw "Windows release workflow must not build distribution artifacts with --release."
}
if ($ReleaseWorkflowContent -notmatch "profile-dir: dist") {
    throw "Rust release workflow must pass profile-dir: dist to signing actions."
}
if ($WindowsWorkflowContent -notmatch "profile-dir: dist") {
    throw "Windows release workflow must pass profile-dir: dist to signing actions."
}

$MacosSignContent = Get-Content -Path $MacosSignAction -Encoding UTF8 -Raw
$WindowsSignContent = Get-Content -Path $WindowsSignAction -Encoding UTF8 -Raw
foreach ($Action in @($MacosSignContent, $WindowsSignContent)) {
    if ($Action -notmatch "profile-dir:") {
        throw "Signing actions must expose a profile-dir input."
    }
}
if ($MacosSignContent -notmatch 'dmg_name="whale-\$\{TARGET\}\.dmg"') {
    throw "macOS signing action must sign Whale dmg names."
}

Write-Host "Build profile policy check OK"
