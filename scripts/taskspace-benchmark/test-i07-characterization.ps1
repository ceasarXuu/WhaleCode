param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path,
    [string]$RunRoot = (Join-Path $RepoRoot "target/i07-characterization")
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot "lib/cost-instrumentation.ps1")

function Assert-I07Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected=$Expected actual=$Actual"
    }
}

$fixtures = Join-Path $PSScriptRoot "fixtures/i07"
$usage = New-TaskspaceRolloutRequestTraceSummary (
    Join-Path $fixtures "usage-double-count-rollout.jsonl"
)
Assert-I07Equal $usage.model_request_count 8 "request facts did not collapse 8/15 usage snapshots"
Assert-I07Equal $usage.state_snapshot_count 7 "request facts lost no-ID state snapshots"

New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null
$boundaryOutput = Join-Path $RunRoot "provider-boundary-evidence.json"
& python3 (Join-Path $PSScriptRoot "docker/verify_provider_boundary.py") `
    --events (Join-Path $fixtures "attempt-boundary-events.jsonl") `
    --wire (Join-Path $fixtures "attempt-boundary-wire.jsonl") `
    --model "deepseek-v4-flash" `
    --output $boundaryOutput
$verifierExit = $LASTEXITCODE
Assert-I07Equal $verifierExit 0 "boundary verifier did not reconcile a local-only failed attempt"
$boundary = Get-Content -Raw -Encoding UTF8 -LiteralPath $boundaryOutput | ConvertFrom-Json
Assert-I07Equal $boundary.boundary_request_count 10 "boundary fixture request count drifted"
Assert-I07Equal $boundary.wire_request_count 10 "boundary-observed wire request count drifted"
Assert-I07Equal $boundary.local_attempt_count 11 "local attempt count drifted"
Assert-I07Equal $boundary.local_only_attempt_count 1 "local-only failed attempt was lost"
Assert-I07Equal @($boundary.errors).Count 0 "valid local-only failure produced a mismatch"

Write-Host "I07 characterization: PASS (usage 8/15 and boundary 10/11 fixed)"
