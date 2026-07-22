param(
    [string]$Contract = "benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v1.json",
    [string]$Golden = "benchmarks/taskspace/r7/five-layer-lifecycle-golden-v1.json",
    [switch]$Update
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$contractPath = Join-Path $repoRoot $Contract
$goldenPath = Join-Path $repoRoot $Golden
$rustWorkspace = Join-Path $repoRoot "third_party/codex-cli/codex-rs"
$outputRoot = Join-Path $repoRoot "target/taskspace-benchmark/fla7-fixtures"
$generatedPath = Join-Path $outputRoot "five-layer-lifecycle-golden-v1.generated.json"
$resultPath = Join-Path $outputRoot "freeze-result.json"

function Get-TextSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        return (($hasher.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") }) -join "")
    } finally {
        $hasher.Dispose()
    }
}

function Assert-Equal {
    param([object]$Actual, [object]$Expected, [string]$Message)
    if ($Actual -cne $Expected) {
        throw "$Message`nExpected: $Expected`nActual:   $Actual"
    }
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$contractObject = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath | ConvertFrom-Json -Depth 50

$canonicalHashes = [ordered]@{}
foreach ($name in @("revision_4", "revision_5")) {
    $fixture = $contractObject.fixture_maps.$name
    $canonicalJson = $fixture.canonical_state | ConvertTo-Json -Depth 50 -Compress
    $actualHash = Get-TextSha256 $canonicalJson
    Assert-Equal $actualHash ([string]$fixture.canonical_sha256) "Canonical Map hash drifted for $name"
    $canonicalHashes[$name] = $actualHash
}

$head = "0" * 64
$eventHeads = [ordered]@{}
foreach ($event in @($contractObject.fixture_event_chain.events)) {
    $revisionName = "revision_$($event.revision)"
    $line = [ordered]@{
        revision = [int64]$event.revision
        canonical_sha256 = [string]$canonicalHashes[$revisionName]
        action = [string]$event.action
    } | ConvertTo-Json -Compress
    $head = Get-TextSha256 "$head`n$line"
    $eventHeads[$revisionName] = $head
}
Assert-Equal $eventHeads.revision_4 ([string]$contractObject.fixture_event_chain.head_after_revision_4) "Revision 4 event-chain head drifted"
Assert-Equal $eventHeads.revision_5 ([string]$contractObject.fixture_event_chain.head_after_revision_5) "Revision 5 event-chain head drifted"

$activeFixtureIds = @($contractObject.fixtures | Where-Object { $_.id -match '^LC-(0[6-9]|1[0-2])-' } | ForEach-Object { [string]$_.id })
Assert-Equal $activeFixtureIds.Count 7 "FLA-7 must own exactly LC-06 through LC-12"
foreach ($number in 6..12) {
    $prefix = "LC-{0:D2}-" -f $number
    if (-not ($activeFixtureIds | Where-Object { $_.StartsWith($prefix) })) {
        throw "Missing active FLA-7 fixture: $prefix"
    }
}

$previousOutput = $env:R7_FLA7_GOLDEN_OUT
try {
    $env:R7_FLA7_GOLDEN_OUT = $generatedPath
    Push-Location $rustWorkspace
    try {
        & cargo test -p codex-core action_map::runtime::fla7_tests::fla7_fixture_maps_render_through_the_shared_production_carriers --lib -- --exact
        if ($LASTEXITCODE -ne 0) {
            throw "Production renderer fixture compiler failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
} finally {
    $env:R7_FLA7_GOLDEN_OUT = $previousOutput
}

if ($Update) {
    Copy-Item -Force -LiteralPath $generatedPath -Destination $goldenPath
} else {
    if (-not (Test-Path -LiteralPath $goldenPath -PathType Leaf)) {
        throw "Committed lifecycle golden is missing: $Golden"
    }
    Assert-Equal (Get-Content -Raw -Encoding UTF8 -LiteralPath $generatedPath) (Get-Content -Raw -Encoding UTF8 -LiteralPath $goldenPath) "Production lifecycle golden drifted; inspect the semantic change before using -Update"
}

$result = [ordered]@{
    schema_version = 1
    status = "passed"
    contract = $Contract
    golden = $Golden
    update = [bool]$Update
    canonical_hashes = $canonicalHashes
    event_chain_heads = $eventHeads
    active_fixture_ids = $activeFixtureIds
    generated_sha256 = Get-TextSha256 (Get-Content -Raw -Encoding UTF8 -LiteralPath $generatedPath)
    committed_sha256 = Get-TextSha256 (Get-Content -Raw -Encoding UTF8 -LiteralPath $goldenPath)
}
$result | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 -LiteralPath $resultPath
Write-Output "R7 FLA-7 lifecycle fixture freeze passed."
Write-Output "Result: $resultPath"
