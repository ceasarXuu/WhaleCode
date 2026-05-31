function Get-TaskspaceBenchmarkRepoRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    (Resolve-Path (Join-Path $scriptDir "..\..\..")).Path
}

function Get-TaskspaceScenarioRoot {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Scenario
    )
    Join-Path $RepoRoot "benchmarks\taskspace\scenarios\$Scenario"
}

function Assert-TaskspaceManifestField {
    param($Manifest, [string]$Name)
    $value = $Manifest.PSObject.Properties[$Name]
    if ($null -eq $value -or $null -eq $value.Value -or [string]::IsNullOrWhiteSpace([string]$value.Value)) {
        throw "Scenario manifest missing required field: $Name"
    }
}

function Read-TaskspaceScenarioManifest {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Scenario
    )
    $scenarioRoot = Get-TaskspaceScenarioRoot $RepoRoot $Scenario
    $manifestPath = Join-Path $scenarioRoot "scenario.json"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "Scenario manifest not found: $manifestPath"
    }
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
    foreach ($field in @("id", "level", "evidence_target", "prompt_file", "fixture_dir", "narrative_contract", "mode_delta_contract", "oracle", "expected", "thresholds")) {
        Assert-TaskspaceManifestField $manifest $field
    }
    if ([string]$manifest.id -ne $Scenario) {
        throw "Scenario id '$($manifest.id)' does not match requested scenario '$Scenario'"
    }
    $promptPath = Join-Path $scenarioRoot ([string]$manifest.prompt_file)
    $fixturePath = Join-Path $scenarioRoot ([string]$manifest.fixture_dir)
    if (-not (Test-Path -LiteralPath $promptPath)) { throw "Scenario prompt not found: $promptPath" }
    if (-not (Test-Path -LiteralPath $fixturePath)) { throw "Scenario fixture not found: $fixturePath" }
    $hiddenStrategy = [string]$manifest.oracle.hidden_strategy
    if ([string]::IsNullOrWhiteSpace($hiddenStrategy)) {
        throw "Scenario manifest missing required field: oracle.hidden_strategy"
    }
    [pscustomobject]@{
        Id = [string]$manifest.id
        Level = [string]$manifest.level
        EvidenceTarget = [string]$manifest.evidence_target
        Raw = $manifest
        ScenarioRoot = (Resolve-Path -LiteralPath $scenarioRoot).Path
        PromptPath = (Resolve-Path -LiteralPath $promptPath).Path
        FixtureDir = (Resolve-Path -LiteralPath $fixturePath).Path
        HiddenOracleStrategy = $hiddenStrategy
        PublicValidation = $manifest.oracle.public_validation
        Expected = $manifest.expected
        Thresholds = $manifest.thresholds
    }
}

function Get-TaskspaceFileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-TaskspaceDirectorySha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    $root = (Resolve-Path -LiteralPath $Path).Path
    $rows = Get-ChildItem -LiteralPath $root -Recurse -File |
        Sort-Object FullName |
        ForEach-Object {
            $relative = $_.FullName.Substring($root.Length).TrimStart("\", "/").Replace("\", "/")
            "$relative=$((Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant())"
        }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($rows -join "`n"))
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
}
