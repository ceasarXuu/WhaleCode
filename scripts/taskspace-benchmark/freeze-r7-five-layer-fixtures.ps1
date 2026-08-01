param(
    [string]$Contract = "benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v2.json"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$contractPath = Join-Path $repoRoot $Contract
$rustWorkspace = Join-Path $repoRoot "third_party/codex-cli/codex-rs"
$outputRoot = Join-Path $repoRoot "target/taskspace-benchmark/r7-a2-lifecycle"
$resultPath = Join-Path $outputRoot "freeze-result.json"

function Assert-Contract {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        throw $Message
    }
}

$raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath
$contractObject = $raw | ConvertFrom-Json -Depth 50
Assert-Contract ([string]$contractObject.status -eq "active_verified") "Lifecycle contract is not active"
Assert-Contract ([string]$contractObject.canonical_schema -eq "taskspace-canonical-map-v2") "Canonical schema drifted"
$oracleIds = @($contractObject.oracles | ForEach-Object { [string]$_.id })
Assert-Contract (($oracleIds -join ",") -eq "LC2-01-final-work-close,LC2-02-reopen-after-user-feedback,LC2-03-invalid-reopen,LC2-04-restart-round-trip") "Lifecycle oracle set drifted"

Push-Location $rustWorkspace
try {
    & cargo test -p codex-core action_map::rooted_dag::replay_tests::close_reopen_and_close_again_preserves_terminal_and_work_history --lib -- --exact
    Assert-Contract ($LASTEXITCODE -eq 0) "Rooted DAG close/reopen oracle failed"
    & cargo test -p codex-core action_map::runtime::transactions::tests::runtime_close_reopen_close_preserves_one_map_and_terminal_history --lib -- --exact
    Assert-Contract ($LASTEXITCODE -eq 0) "Runtime close/reopen oracle failed"
    & cargo test -p codex-state runtime::taskspace_maps_tests::taskspace_map_survives_state_runtime_restart --lib -- --exact
    Assert-Contract ($LASTEXITCODE -eq 0) "Store restart oracle failed"
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$result = [ordered]@{
    schema_version = 1
    status = "passed"
    contract = $Contract
    canonical_schema = [string]$contractObject.canonical_schema
    oracle_ids = $oracleIds
}
$result | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 -LiteralPath $resultPath
Write-Output "R7.1 A2 lifecycle fixture verification passed."
Write-Output "Result: $resultPath"
