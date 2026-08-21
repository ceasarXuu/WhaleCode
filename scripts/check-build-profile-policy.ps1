param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$CargoToml = Join-Path $RepoRoot "third_party/codex-cli/codex-rs/Cargo.toml"
$Workflow = Join-Path $RepoRoot "docs/runbooks/development-workflow.md"
$DistributionGuard = Join-Path $RepoRoot "scripts/release/check_distribution_identity.py"

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

$ReleaseRequirements = @(
    @("opt-level", "1"),
    @("lto", "false"),
    @("incremental", "true"),
    @("codegen-units", "256"),
    @("strip", '"none"')
)
$DistRequirements = @(
    @("inherits", '"release"'),
    @("opt-level", "3"),
    @("lto", '"fat"'),
    @("incremental", "false"),
    @("codegen-units", "1"),
    @("strip", '"symbols"')
)

foreach ($Requirement in $ReleaseRequirements) {
    Require-Value -Section $Release -Key $Requirement[0] -Expected $Requirement[1] `
        -Message "Release profile $($Requirement[0]) must be $($Requirement[1])."
}
foreach ($Requirement in $DistRequirements) {
    Require-Value -Section $Dist -Key $Requirement[0] -Expected $Requirement[1] `
        -Message "Dist profile $($Requirement[0]) must be $($Requirement[1])."
}

$WorkflowContent = Get-Content -Path $Workflow -Encoding UTF8 -Raw
if ($WorkflowContent -notmatch "cargo build -p codex-cli --bin whale --profile dist --locked") {
    throw "Development workflow must document the explicit Whale dist build command."
}

& python3 $DistributionGuard --repo-root $RepoRoot
if ($LASTEXITCODE -ne 0) {
    throw "Whale distribution identity guard failed."
}

Write-Host "Build profile policy check OK"
