param()

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$CargoToml = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\Cargo.toml"
$CliCargoToml = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\cli\Cargo.toml"
$TuiCargoToml = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\tui\Cargo.toml"
$TuiBuildRs = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\tui\build.rs"
$TuiVersionRs = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\tui\src\version.rs"
$CloudTasksCargoToml = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\cloud-tasks\Cargo.toml"
$BuildNumber = Join-Path $RepoRoot "third_party\codex-cli\BUILD_NUMBER"
$InstallWhaleLocal = Join-Path $RepoRoot "scripts\install-whale-local.ps1"
$BuildNpmPackage = Join-Path $RepoRoot "third_party\codex-cli\codex-cli\scripts\build_npm_package.py"
$InstallNativeDeps = Join-Path $RepoRoot "third_party\codex-cli\codex-cli\scripts\install_native_deps.py"
$ConfigOverride = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\utils\cli\src\config_override.rs"
$CloudTasksCli = Join-Path $RepoRoot "third_party\codex-cli\codex-rs\cloud-tasks\src\cli.rs"
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
$CliCargoContent = Get-Content -Path $CliCargoToml -Encoding UTF8 -Raw
$TuiCargoContent = Get-Content -Path $TuiCargoToml -Encoding UTF8 -Raw
$TuiBuildContent = Get-Content -Path $TuiBuildRs -Encoding UTF8 -Raw
$TuiVersionContent = Get-Content -Path $TuiVersionRs -Encoding UTF8 -Raw
$CloudTasksCargoContent = Get-Content -Path $CloudTasksCargoToml -Encoding UTF8 -Raw
$BuildNumberContent = (Get-Content -Path $BuildNumber -Encoding UTF8 -Raw).Trim()
$InstallWhaleLocalContent = Get-Content -Path $InstallWhaleLocal -Encoding UTF8 -Raw
$BuildNpmPackageContent = Get-Content -Path $BuildNpmPackage -Encoding UTF8 -Raw
$InstallNativeDepsContent = Get-Content -Path $InstallNativeDeps -Encoding UTF8 -Raw
$ConfigOverrideContent = Get-Content -Path $ConfigOverride -Encoding UTF8 -Raw
$CloudTasksCliContent = Get-Content -Path $CloudTasksCli -Encoding UTF8 -Raw
$Release = Get-ProfileSection -Content $CargoContent -Name "release"
$Dist = Get-ProfileSection -Content $CargoContent -Name "dist"

$WorkspaceVersionMatch = [regex]::Match($CargoContent, '(?ms)^\[workspace\.package\]\s*(.*?)(?=^\[|\z)')
if (-not $WorkspaceVersionMatch.Success -or $WorkspaceVersionMatch.Groups[1].Value -notmatch '(?m)^\s*version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?"') {
    throw "Cargo workspace package version must be the Whale release semver source of truth."
}
if ($BuildNumberContent -notmatch '^[1-9][0-9]*$') {
    throw "third_party\codex-cli\BUILD_NUMBER must be a positive integer."
}
if ($TuiCargoContent -notmatch '(?m)^\s*build\s*=\s*"build\.rs"') {
    throw "codex-tui must use build.rs so Whale build number is embedded in the TUI."
}
if ($TuiBuildContent -notmatch 'BUILD_NUMBER' -or $TuiBuildContent -notmatch 'WHALE_BUILD_NUMBER') {
    throw "codex-tui build.rs must read BUILD_NUMBER and export WHALE_BUILD_NUMBER."
}
if ($TuiVersionContent -notmatch 'WHALE_BUILD_NUMBER' -or $TuiVersionContent -notmatch 'whale_version_display') {
    throw "TUI version module must expose the Whale version/build display string."
}

$DisallowedCliDeps = @(
    "codex-app-server",
    "codex-app-server-test-client",
    "codex-cloud-tasks",
    "codex-exec-server",
    "codex-mcp-server",
    "codex-responses-api-proxy",
    "codex-stdio-to-uds"
)
foreach ($Dependency in $DisallowedCliDeps) {
    if ($CliCargoContent -match "(?m)^$([regex]::Escape($Dependency))\s*=") {
        throw "The whale CLI must not directly depend on $Dependency; forward to the helper binary instead."
    }
}
$CloudTasksDependencies = [regex]::Match($CloudTasksCargoContent, '(?ms)^\[dependencies\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
if ($CloudTasksDependencies -match '(?m)^codex-cloud-tasks-mock-client\s*=') {
    throw "codex-cloud-tasks-mock-client must stay in cloud-tasks dev-dependencies, not normal dependencies."
}

$WhaleHelperBinaries = @(
    "whale-app-server",
    "whale-app-server-test-client",
    "whale-cloud-tasks",
    "whale-exec-server",
    "whale-mcp-server",
    "whale-responses-api-proxy",
    "whale-stdio-to-uds"
)
foreach ($Helper in $WhaleHelperBinaries) {
    if ($InstallWhaleLocalContent -notmatch "$([regex]::Escape($Helper))\.exe") {
        throw "install-whale-local.ps1 must copy $Helper.exe next to whale.exe."
    }
}
if ($InstallWhaleLocalContent -notmatch "legacy-helper") {
    throw "install-whale-local.ps1 must move old codex-named helper binaries into a backup directory."
}

$WhaleNpmComponents = @(
    "whale-app-server",
    "whale-app-server-test-client",
    "whale-cloud-tasks",
    "whale-exec-server",
    "whale-mcp-server",
    "whale-responses-api-proxy",
    "whale-stdio-to-uds"
)
foreach ($Component in $WhaleNpmComponents) {
    if ($BuildNpmPackageContent -notmatch [regex]::Escape($Component)) {
        throw "build_npm_package.py must stage $Component in Whale platform packages."
    }
    if ($InstallNativeDepsContent -notmatch [regex]::Escape($Component)) {
        throw "install_native_deps.py must install $Component into Whale native vendor payloads."
    }
}
if ($ConfigOverrideContent -match 'model="o3"') {
    throw "CLI config override examples must use a DeepSeek model, not an OpenAI model."
}
if ($CloudTasksCliContent -match "Codex Cloud|codex cloud") {
    throw "Cloud task help text must use Whale Cloud branding."
}

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
    -Section $Release `
    -Key "strip" `
    -Expected '"none"' `
    -Message "The default release profile must not strip local build artifacts."
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
Require-Value `
    -Section $Dist `
    -Key "strip" `
    -Expected '"symbols"' `
    -Message "The dist profile must strip distribution artifacts."

$WorkflowContent = Get-Content -Path $Workflow -Encoding UTF8 -Raw
if ($WorkflowContent -match "CARGO_PROFILE_RELEASE_LTO\s*=") {
    throw "Development workflow must not recommend process-local release-profile overrides."
}
if ($WorkflowContent -notmatch "cargo build -p codex-cli --bin whale --profile dist --locked") {
    throw "Development workflow must document the explicit dist profile command."
}

$ReleaseWorkflowContent = Get-Content -Path $RustReleaseWorkflow -Encoding UTF8 -Raw
$WindowsWorkflowContent = Get-Content -Path $RustReleaseWindowsWorkflow -Encoding UTF8 -Raw
if ($ReleaseWorkflowContent -notmatch 'BUILD_NUMBER') {
    throw "Rust release workflow must validate Whale BUILD_NUMBER."
}
if ($ReleaseWorkflowContent -notmatch 'steps\.release_name\.outputs\.version') {
    throw "Rust release workflow must keep semver output separate from display name for npm and WinGet."
}
if ($ReleaseWorkflowContent -notmatch 'display_name') {
    throw "Rust release workflow must include the build number in the GitHub Release display name."
}
foreach ($Helper in $WhaleHelperBinaries) {
    if ($ReleaseWorkflowContent -notmatch [regex]::Escape($Helper)) {
        throw "Rust release workflow must build and publish $Helper for Whale platform packages."
    }
    if ($WindowsWorkflowContent -notmatch [regex]::Escape($Helper)) {
        throw "Windows release workflow must build and publish $Helper for Whale platform packages."
    }
}
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
