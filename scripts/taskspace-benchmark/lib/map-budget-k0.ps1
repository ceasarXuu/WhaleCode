Set-StrictMode -Version Latest

function Assert-K0MapBudgetProbe {
    param([Parameter(Mandatory = $true)]$Probe)

    if ([string]$Probe.schema_version -ne "taskspace-map-budget-k0-probe-v1") {
        throw "Unsupported K0 probe schema: $($Probe.schema_version)"
    }
    $projectionRows = @($Probe.projection_rows)
    $crossings = @($Probe.budget_crossings)
    $replayRows = @($Probe.replay_rows)
    if ($projectionRows.Count -ne 9) {
        throw "K0 probe must contain 9 projection rows, got $($projectionRows.Count)"
    }
    if ($crossings.Count -ne 15) {
        throw "K0 probe must contain 15 budget crossings, got $($crossings.Count)"
    }
    if ($replayRows.Count -ne 3) {
        throw "K0 probe must contain 3 replay rows, got $($replayRows.Count)"
    }
    $failedReplay = @($replayRows | Where-Object { -not [bool]$_.replay_exact })
    if ($failedReplay.Count -gt 0) {
        throw "K0 probe contains non-exact replay rows"
    }
}

function Assert-K0LongReplayProbe {
    param([Parameter(Mandatory = $true)]$Probe)

    if ([string]$Probe.schema_version -ne "taskspace-map-budget-k0-long-replay-v1") {
        throw "Unsupported K0 long replay schema: $($Probe.schema_version)"
    }
    if ([int]$Probe.resume_cycles -lt 2) {
        throw "K0 long replay must contain multiple resume cycles"
    }
    if ([int]$Probe.compaction_boundaries -ne [int]$Probe.resume_cycles) {
        throw "K0 long replay must cover every resume with a compaction boundary"
    }
    if ([int]$Probe.code_revision_count -ne [int]$Probe.resume_cycles) {
        throw "K0 long replay must change the code revision in every cycle"
    }
    if ([int]$Probe.exact_replay_count -ne [int]$Probe.resume_cycles) {
        throw "K0 long replay contains a non-exact resume"
    }
    if ([int]$Probe.single_projection_outcome_count -ne [int]$Probe.resume_cycles) {
        throw "K0 long replay did not rebuild exactly one projection outcome per resume"
    }
}

function Get-K0GrowthSlope {
    param(
        [Parameter(Mandatory = $true)][object[]]$Rows,
        [Parameter(Mandatory = $true)][string]$EdgeProfile
    )
    $profileRows = @($Rows | Where-Object { [string]$_.edge_profile -eq $EdgeProfile } | Sort-Object node_count)
    $first = $profileRows | Where-Object { [int]$_.node_count -eq 1000 } | Select-Object -First 1
    $last = $profileRows | Where-Object { [int]$_.node_count -eq 10000 } | Select-Object -First 1
    if (-not $first -or -not $last) { return $null }
    return [math]::Round(
        ([double]$last.skeleton_estimated_tokens - [double]$first.skeleton_estimated_tokens) /
            ([double]$last.node_count - [double]$first.node_count),
        4
    )
}

function Convert-K0ProjectionRowsToCsv {
    param([Parameter(Mandatory = $true)][object[]]$Rows)
    return @($Rows | Select-Object node_count, edge_profile, edge_count,
        skeleton_estimated_tokens, estimated_tokens, skeleton_bytes, projection_bytes,
        map_node_bytes, map_edge_bytes, node_detail_bytes, input_build_duration_us,
        render_duration_us)
}

function Write-K0MapBudgetArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$ProbePath,
        [Parameter(Mandatory = $true)][string]$LongReplayProbePath,
        [Parameter(Mandatory = $true)][string]$OutputDir,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][string]$ProbeCommand,
        [Parameter(Mandatory = $true)][hashtable]$Verification
    )

    $probe = Get-Content -Raw -Encoding UTF8 $ProbePath | ConvertFrom-Json
    $longReplayProbe = Get-Content -Raw -Encoding UTF8 $LongReplayProbePath | ConvertFrom-Json
    Assert-K0MapBudgetProbe -Probe $probe
    Assert-K0LongReplayProbe -Probe $longReplayProbe
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

    $projectionRows = @($probe.projection_rows)
    $crossings = @($probe.budget_crossings)
    $replayRows = @($probe.replay_rows)
    $slopes = [ordered]@{}
    foreach ($profile in @("none", "chain", "forward_4")) {
        $slopes[$profile] = Get-K0GrowthSlope -Rows $projectionRows -EdgeProfile $profile
    }
    $chain10k = $projectionRows |
        Where-Object { [int]$_.node_count -eq 10000 -and [string]$_.edge_profile -eq "chain" } |
        Select-Object -First 1
    $nodeSkeletonShare = if ($chain10k) {
        [math]::Round([double]$chain10k.map_node_bytes / [double]$chain10k.skeleton_bytes, 6)
    } else { $null }
    $detailProjectionShare = if ($chain10k) {
        [math]::Round([double]$chain10k.node_detail_bytes / [double]$chain10k.projection_bytes, 6)
    } else { $null }

    $report = [ordered]@{
        schema_version = "taskspace-map-budget-k0-report-v1"
        generated_at = (Get-Date).ToString("o")
        source_commit = $SourceCommit
        probe_command = $ProbeCommand
        probe_schema_version = [string]$probe.schema_version
        projection_rows = $projectionRows
        budget_crossings = $crossings
        replay_rows = $replayRows
        long_replay_fixture = $longReplayProbe
        summary = [ordered]@{
            projection_row_count = $projectionRows.Count
            budget_crossing_count = $crossings.Count
            replay_row_count = $replayRows.Count
            replay_exact_count = @($replayRows | Where-Object { [bool]$_.replay_exact }).Count
            long_replay_exact_count = [int]$longReplayProbe.exact_replay_count
            skeleton_tokens_per_node_1k_to_10k = $slopes
            chain_10k_node_skeleton_share = $nodeSkeletonShare
            chain_10k_detail_projection_share = $detailProjectionShare
            max_measured_projection_bytes = [long](($projectionRows | Measure-Object projection_bytes -Maximum).Maximum)
            max_measured_snapshot_bytes = [long](($replayRows | Measure-Object final_snapshot_bytes -Maximum).Maximum)
        }
        corruption_contract = [ordered]@{
            snapshot_delta_matrix = [string]$Verification.snapshot_delta_matrix
            event_checkpoint_hash = [string]$Verification.event_checkpoint_hash
            session_resume_behavior_current = "panic_via_expect"
            selected_contract = "structured_session_fatal_error"
            partial_restore_allowed = $false
            silent_fallback_allowed = $false
            recoverable_operator_error = $false
        }
        verification = $Verification
    }

    $jsonPath = Join-Path $OutputDir "k0-map-budget-report.json"
    $csvPath = Join-Path $OutputDir "k0-map-budget-projection.csv"
    $markdownPath = Join-Path $OutputDir "k0-map-budget-report.md"
    $eventsPath = Join-Path $OutputDir "k0-map-budget-events.jsonl"
    $report | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 $jsonPath
    Convert-K0ProjectionRowsToCsv -Rows $projectionRows |
        Export-Csv -NoTypeInformation -Encoding UTF8 $csvPath

    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("# R5-K0 Map 规模与预算基线")
    $lines.Add("")
    $lines.Add("- Source commit: ``$SourceCommit``")
    $lines.Add("- Probe schema: ``$($probe.schema_version)``")
    $lines.Add("- Projection rows: $($projectionRows.Count)")
    $lines.Add("- Replay exact: $($report.summary.replay_exact_count)/$($replayRows.Count)")
    $lines.Add("")
    $lines.Add("## Projection 规模")
    $lines.Add("")
    $lines.Add("| Nodes | Edge profile | Edges | Skeleton tokens | Full tokens | Skeleton bytes | Full bytes | Render us |")
    $lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $projectionRows) {
        $lines.Add("| $($row.node_count) | $($row.edge_profile) | $($row.edge_count) | $($row.skeleton_estimated_tokens) | $($row.estimated_tokens) | $($row.skeleton_bytes) | $($row.projection_bytes) | $($row.render_duration_us) |")
    }
    $lines.Add("")
    $lines.Add("## 首次超预算节点")
    $lines.Add("")
    $lines.Add("| Edge profile | Budget tokens | First over nodes | Skeleton tokens |")
    $lines.Add("|---|---:|---:|---:|")
    foreach ($row in $crossings) {
        $nodes = if ($null -eq $row.first_over_budget_node_count) { "N/A" } else { [string]$row.first_over_budget_node_count }
        $tokens = if ($null -eq $row.skeleton_tokens_at_crossing) { "N/A" } else { [string]$row.skeleton_tokens_at_crossing }
        $lines.Add("| $($row.edge_profile) | $($row.max_projection_tokens) | $nodes | $tokens |")
    }
    $lines.Add("")
    $lines.Add("## Checkpoint / Replay")
    $lines.Add("")
    $lines.Add("| Initial nodes | Final nodes | Cycles | Checkpoint bytes | Delta bytes | Final snapshot | Delta build us | Replay us | Exact |")
    $lines.Add("|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    foreach ($row in $replayRows) {
        $lines.Add("| $($row.initial_node_count) | $($row.final_node_count) | $($row.checkpoint_cycles) | $($row.checkpoint_bytes) | $($row.delta_bytes) | $($row.final_snapshot_bytes) | $($row.delta_build_duration_us) | $($row.replay_duration_us) | $($row.replay_exact) |")
    }
    $lines.Add("")
    $lines.Add("## Session-native 长链恢复")
    $lines.Add("")
    $lines.Add("- Fixture: ``$($longReplayProbe.fixture_kind)``")
    $lines.Add("- Nodes / edges: $($longReplayProbe.node_count) / $($longReplayProbe.edge_count)")
    $lines.Add("- Resume / compaction / code revisions: $($longReplayProbe.resume_cycles) / $($longReplayProbe.compaction_boundaries) / $($longReplayProbe.code_revision_count)")
    $lines.Add("- Exact replay / single projection outcome: $($longReplayProbe.exact_replay_count) / $($longReplayProbe.single_projection_outcome_count)")
    $lines.Add("- Skeleton over budget outcomes: $($longReplayProbe.skeleton_over_budget_count)")
    $lines.Add("- Resume / projection duration us: $($longReplayProbe.resume_duration_us) / $($longReplayProbe.projection_duration_us)")
    $lines.Add("")
    $lines.Add("## Corruption 合同")
    $lines.Add("")
    $lines.Add("- Snapshot delta matrix: ``$($Verification.snapshot_delta_matrix)``")
    $lines.Add("- Event checkpoint hash: ``$($Verification.event_checkpoint_hash)``")
    $lines.Add("- Current resume behavior: ``panic_via_expect``")
    $lines.Add("- Selected K contract: ``structured_session_fatal_error``")
    $lines.Add("- Partial restore / silent fallback: forbidden")
    $lines | Set-Content -Encoding UTF8 $markdownPath

    $writeEvent = $true
    foreach ($row in $projectionRows) {
        $event = [ordered]@{
            event_name = "taskspace.map_budget_measured"
            schema_version = "taskspace-map-budget-k0-event-v1"
            node_count = [int]$row.node_count
            edge_profile = [string]$row.edge_profile
            edge_count = [int]$row.edge_count
            skeleton_tokens = [int]$row.skeleton_estimated_tokens
            projection_tokens = [int]$row.estimated_tokens
            skeleton_bytes = [long]$row.skeleton_bytes
            projection_bytes = [long]$row.projection_bytes
            render_duration_us = [long]$row.render_duration_us
        } | ConvertTo-Json -Compress
        if ($writeEvent) {
            $event | Set-Content -Encoding UTF8 $eventsPath
            $writeEvent = $false
        } else {
            $event | Add-Content -Encoding UTF8 $eventsPath
        }
    }
    foreach ($row in $replayRows) {
        [ordered]@{
            event_name = "taskspace.map_replay_measured"
            schema_version = "taskspace-map-budget-k0-event-v1"
            initial_node_count = [int]$row.initial_node_count
            final_node_count = [int]$row.final_node_count
            checkpoint_cycles = [int]$row.checkpoint_cycles
            checkpoint_bytes = [long]$row.checkpoint_bytes
            delta_bytes = [long]$row.delta_bytes
            replay_duration_us = [long]$row.replay_duration_us
            replay_exact = [bool]$row.replay_exact
        } | ConvertTo-Json -Compress | Add-Content -Encoding UTF8 $eventsPath
    }
    [ordered]@{
        event_name = "taskspace.map_replay_measured"
        schema_version = "taskspace-map-budget-k0-event-v1"
        fixture_kind = [string]$longReplayProbe.fixture_kind
        node_count = [int]$longReplayProbe.node_count
        edge_count = [int]$longReplayProbe.edge_count
        resume_cycles = [int]$longReplayProbe.resume_cycles
        compaction_boundaries = [int]$longReplayProbe.compaction_boundaries
        code_revision_count = [int]$longReplayProbe.code_revision_count
        checkpoint_bytes = [long]$longReplayProbe.checkpoint_bytes
        delta_bytes = [long]$longReplayProbe.delta_bytes
        resume_duration_us = [long]$longReplayProbe.resume_duration_us
        projection_duration_us = [long]$longReplayProbe.projection_duration_us
        skeleton_over_budget_count = [int]$longReplayProbe.skeleton_over_budget_count
        replay_exact = $true
    } | ConvertTo-Json -Compress | Add-Content -Encoding UTF8 $eventsPath
    [ordered]@{
        event_name = "taskspace.map_corruption_contract_frozen"
        schema_version = "taskspace-map-budget-k0-event-v1"
        selected_contract = "structured_session_fatal_error"
        partial_restore_allowed = $false
        silent_fallback_allowed = $false
    } | ConvertTo-Json -Compress | Add-Content -Encoding UTF8 $eventsPath

    return [pscustomobject]@{
        JsonPath = $jsonPath
        CsvPath = $csvPath
        MarkdownPath = $markdownPath
        EventsPath = $eventsPath
    }
}
