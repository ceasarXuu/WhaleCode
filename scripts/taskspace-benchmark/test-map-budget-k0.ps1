$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "lib/map-budget-k0.ps1")

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("taskspace-k0-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
try {
    $projectionRows = @()
    foreach ($nodes in @(100, 1000, 10000)) {
        foreach ($profile in @("none", "chain", "forward_4")) {
            $projectionRows += [ordered]@{
                node_count = $nodes
                edge_profile = $profile
                edge_count = if ($profile -eq "none") { 0 } else { $nodes - 1 }
                skeleton_estimated_tokens = $nodes * 10
                estimated_tokens = $nodes * 20
                skeleton_bytes = $nodes * 40
                projection_bytes = $nodes * 80
                map_node_bytes = $nodes * 30
                map_edge_bytes = $nodes * 10
                node_detail_bytes = $nodes * 40
                input_build_duration_us = 10
                render_duration_us = 20
            }
        }
    }
    $crossings = @()
    foreach ($profile in @("none", "chain", "forward_4")) {
        foreach ($budget in @(12000, 16000, 24000, 32000, 48000)) {
            $crossings += [ordered]@{
                edge_profile = $profile
                max_projection_tokens = $budget
                first_over_budget_node_count = 100
                skeleton_tokens_at_crossing = $budget + 1
            }
        }
    }
    $replayRows = @()
    foreach ($nodes in @(100, 1000, 10000)) {
        $replayRows += [ordered]@{
            initial_node_count = $nodes
            final_node_count = $nodes + 5
            checkpoint_cycles = 5
            nodes_appended_per_cycle = 1
            checkpoint_bytes = 100
            delta_bytes = 20
            final_snapshot_bytes = 110
            snapshot_generation_duration_us = 10
            delta_build_duration_us = 20
            replay_duration_us = 30
            replay_exact = $true
            final_snapshot_sha256 = "abc"
        }
    }
    $probePath = Join-Path $tempRoot "probe.json"
    [ordered]@{
        schema_version = "taskspace-map-budget-k0-probe-v1"
        projection_rows = $projectionRows
        budget_crossings = $crossings
        replay_rows = $replayRows
    } | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 $probePath
    $longReplayProbePath = Join-Path $tempRoot "long-replay.json"
    [ordered]@{
        schema_version = "taskspace-map-budget-k0-long-replay-v1"
        fixture_kind = "session_native_resume_compaction_code_change"
        node_count = 1000
        edge_count = 999
        resume_cycles = 5
        compaction_boundaries = 5
        code_revision_count = 5
        checkpoint_bytes = 1000
        delta_bytes = 100
        resume_duration_us = 30
        projection_duration_us = 40
        exact_replay_count = 5
        single_projection_outcome_count = 5
        skeleton_over_budget_count = 5
        final_snapshot_sha256 = "def"
    } | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $longReplayProbePath
    $capturedReplayProbePath = Join-Path $tempRoot "captured-replay.json"
    [ordered]@{
        schema_version = "taskspace-map-budget-k0-captured-replay-v1"
        fixture_kind = "captured_docker_rollout"
        rollout_bytes = 10000
        rollout_item_count = 100
        snapshot_checkpoint_count = 1
        snapshot_delta_count = 10
        compaction_count = 0
        replay_cycles = 3
        stable_snapshot_count = 3
        replay_duration_us = 100
        final_node_count = 5
        final_snapshot_sha256 = "ghi"
    } | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $capturedReplayProbePath
    $artifacts = Write-K0MapBudgetArtifacts `
        -ProbePath $probePath `
        -LongReplayProbePath $longReplayProbePath `
        -CapturedReplayProbePath $capturedReplayProbePath `
        -OutputDir (Join-Path $tempRoot "report") `
        -SourceCommit "test-commit" `
        -ProbeCommand "test-command" `
        -Verification @{
            snapshot_delta_matrix = "passed"
            event_checkpoint_hash = "passed"
        }
    foreach ($path in @($artifacts.JsonPath, $artifacts.CsvPath, $artifacts.MarkdownPath, $artifacts.EventsPath)) {
        if (-not (Test-Path $path)) { throw "Missing K0 artifact: $path" }
    }
    $report = Get-Content -Raw -Encoding UTF8 $artifacts.JsonPath | ConvertFrom-Json
    if ($report.summary.projection_row_count -ne 9) { throw "Projection count mismatch" }
    if ($report.summary.replay_exact_count -ne 3) { throw "Replay exact count mismatch" }
    if ($report.summary.long_replay_exact_count -ne 5) { throw "Long replay exact count mismatch" }
    if ($report.summary.captured_replay_stable_count -ne 3) { throw "Captured replay count mismatch" }
    if ($report.corruption_contract.selected_contract -ne "structured_session_fatal_error") {
        throw "Corruption contract mismatch"
    }
    $events = @(Get-Content -Encoding UTF8 $artifacts.EventsPath)
    if ($events.Count -ne 15) { throw "Expected 15 K0 events, got $($events.Count)" }
    Write-Host "K0 map budget report selftest passed"
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
