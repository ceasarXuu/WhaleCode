param(
    [string]$OutputRoot = "target/r5-k0-map-budget",
    [string]$CargoManifest = "third_party/codex-cli/codex-rs/Cargo.toml",
    [string]$CapturedRolloutPath = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$manifestPath = (Resolve-Path (Join-Path $repoRoot $CargoManifest)).Path
$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = Join-Path $repoRoot (Join-Path $OutputRoot $stamp)
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$probePath = Join-Path $runDir "k0-probe.raw.json"
$longReplayProbePath = Join-Path $runDir "k0-long-replay.raw.json"
$capturedReplayProbePath = ""
$cargoLog = Join-Path $runDir "cargo-tests.log"
$probeCommand = "cargo test -p codex-core --lib action_map::k0_scale_tests::writes_k0_scale_probe_artifact -- --nocapture"

function Invoke-K0CargoTest {
    param(
        [Parameter(Mandatory = $true)][string]$Filter,
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][int]$ExpectedPasses
    )
    Push-Location $repoRoot
    try {
        "[$Label] cargo test filter=$Filter" | Add-Content -Encoding UTF8 $cargoLog
        $output = @(& cargo test --manifest-path $manifestPath -p codex-core --lib $Filter -- --nocapture 2>&1)
        $output | Tee-Object -FilePath $cargoLog -Append | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "K0 cargo test failed: $Label"
        }
        $outputText = $output -join "`n"
        if ($outputText -notmatch "test result: ok\. $ExpectedPasses passed;") {
            throw "K0 cargo test did not execute $ExpectedPasses expected tests: $Label"
        }
    } finally {
        Pop-Location
    }
}

$oldProbePath = $env:TASKSPACE_K0_SCALE_OUTPUT
try {
    $env:TASKSPACE_K0_SCALE_OUTPUT = $probePath
    Invoke-K0CargoTest `
        -Filter "action_map::k0_scale_tests::writes_k0_scale_probe_artifact" `
        -Label "scale-probe" `
        -ExpectedPasses 1
} finally {
    $env:TASKSPACE_K0_SCALE_OUTPUT = $oldProbePath
}
$oldLongReplayProbePath = $env:TASKSPACE_K0_LONG_REPLAY_OUTPUT
try {
    $env:TASKSPACE_K0_LONG_REPLAY_OUTPUT = $longReplayProbePath
    Invoke-K0CargoTest `
        -Filter "session::k0_long_replay_tests::writes_k0_session_native_long_replay_probe" `
        -Label "session-native-long-replay" `
        -ExpectedPasses 1
} finally {
    $env:TASKSPACE_K0_LONG_REPLAY_OUTPUT = $oldLongReplayProbePath
}
if (-not [string]::IsNullOrWhiteSpace($CapturedRolloutPath)) {
    $capturedRolloutCandidate = if ([IO.Path]::IsPathRooted($CapturedRolloutPath)) {
        $CapturedRolloutPath
    } else {
        Join-Path $repoRoot $CapturedRolloutPath
    }
    $resolvedCapturedRollout = (Resolve-Path $capturedRolloutCandidate).Path
    $capturedReplayProbePath = Join-Path $runDir "k0-captured-replay.raw.json"
    $oldCapturedRollout = $env:TASKSPACE_K0_CAPTURED_ROLLOUT
    $oldCapturedReplayOutput = $env:TASKSPACE_K0_CAPTURED_REPLAY_OUTPUT
    try {
        $env:TASKSPACE_K0_CAPTURED_ROLLOUT = $resolvedCapturedRollout
        $env:TASKSPACE_K0_CAPTURED_REPLAY_OUTPUT = $capturedReplayProbePath
        Invoke-K0CargoTest `
            -Filter "session::k0_long_replay_tests::writes_k0_captured_rollout_replay_probe" `
            -Label "captured-docker-rollout-replay" `
            -ExpectedPasses 1
    } finally {
        $env:TASKSPACE_K0_CAPTURED_ROLLOUT = $oldCapturedRollout
        $env:TASKSPACE_K0_CAPTURED_REPLAY_OUTPUT = $oldCapturedReplayOutput
    }
}
Invoke-K0CargoTest `
    -Filter "action_map::snapshot_delta::tests" `
    -Label "snapshot-delta-corruption" `
    -ExpectedPasses 4
Invoke-K0CargoTest `
    -Filter "action_map::event_store::tests::restore_rejects_checkpoint_when_covered_raw_event_changed" `
    -Label "event-checkpoint-corruption" `
    -ExpectedPasses 1
Invoke-K0CargoTest `
    -Filter "session::rollout_reconstruction_tests::resumed_history_rejects" `
    -Label "session-resume-corruption" `
    -ExpectedPasses 2

$sourceCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
$verification = @{
    snapshot_delta_matrix = "passed"
    event_checkpoint_hash = "passed"
    session_resume_corruption = "passed_expected_panic"
    session_native_long_replay = "passed"
    captured_rollout_replay = if ($capturedReplayProbePath) { "passed" } else { "not_requested" }
    cargo_log = $cargoLog
}
. (Join-Path $PSScriptRoot "lib/map-budget-k0.ps1")
$artifacts = Write-K0MapBudgetArtifacts `
    -ProbePath $probePath `
    -LongReplayProbePath $longReplayProbePath `
    -CapturedReplayProbePath $capturedReplayProbePath `
    -OutputDir $runDir `
    -SourceCommit $sourceCommit `
    -ProbeCommand $probeCommand `
    -Verification $verification

[ordered]@{
    schema_version = "taskspace-map-budget-k0-run-status-v1"
    status = "complete"
    run_dir = $runDir
    source_commit = $sourceCommit
    probe_path = $probePath
    long_replay_probe_path = $longReplayProbePath
    captured_replay_probe_path = $capturedReplayProbePath
    report_path = $artifacts.JsonPath
    events_path = $artifacts.EventsPath
    completed_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $runDir "run-status.json")

Write-Host "RunDir: $runDir"
Write-Host "ReportJson: $($artifacts.JsonPath)"
Write-Host "ReportMarkdown: $($artifacts.MarkdownPath)"
Write-Host "Events: $($artifacts.EventsPath)"
