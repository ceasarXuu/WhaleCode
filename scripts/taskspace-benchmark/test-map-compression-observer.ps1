$ErrorActionPreference = "Stop"
$root = Join-Path ([System.IO.Path]::GetTempPath()) "whale-map-compression-observer-$([guid]::NewGuid())"
$output = Join-Path $root "observation"
New-Item -ItemType Directory -Force -Path $root | Out-Null

$rolloutPath = Join-Path $root "rollout.jsonl"
$traceBase = @{
    timestamp = "2026-07-13T00:00:00Z"
    type = "event_msg"
    payload = @{
        type = "map_runtime"
        map_event_type = "taskspace_trace_event_recorded"
        kind = "projection_budget"
    }
}
$events = @(
    @(
        "schema:taskspace-projection-budget-v1",
        "strategy_id:S4",
        "strategy_activation_count:0",
        "projection_bytes_before_strategy:500",
        "projection_bytes_after_strategy:500",
        "folded_node_count:0",
        "fold_eligible_node_count:0",
        "node_detail_bytes_before_strategy:100",
        "node_detail_bytes_after_strategy:100",
        "skeleton_bytes_before_strategy:400",
        "skeleton_bytes_after_strategy:400"
    ),
    @(
        "schema:taskspace-projection-budget-v1",
        "strategy_id:S4",
        "strategy_activation_count:1",
        "projection_bytes_before_strategy:1000",
        "projection_bytes_after_strategy:800",
        "folded_node_count:3",
        "fold_eligible_node_count:4",
        "node_detail_bytes_before_strategy:500",
        "node_detail_bytes_after_strategy:300",
        "skeleton_bytes_before_strategy:500",
        "skeleton_bytes_after_strategy:500"
    )
)
$lines = foreach ($tags in $events) {
    $record = $traceBase.Clone()
    $record.payload = $traceBase.payload.Clone()
    $record.payload.tags = $tags
    $record | ConvertTo-Json -Depth 8 -Compress
}
$lines | Set-Content -LiteralPath $rolloutPath -Encoding UTF8

$metricsPath = Join-Path $root "metrics.json"
@{
    business_success = $true
    agent_completion_status = "agent_completed"
    external_validation_status = "passed"
    model_request_count = 2
    input_tokens = 100
    cached_input_tokens = 80
    uncached_input_tokens = 20
    output_tokens = 20
    wall_time_ms = 1000
    projection_tokens_max = 250
    nodes = 4
    edges = 0
    rollout_path = $rolloutPath
} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $metricsPath -Encoding UTF8

$indexPath = Join-Path $root "run-index.json"
@{
    phase = "observer-test"
    p0_alias_of = "B0"
    results = @(@{
        sample_class = "complex"
        scenario = "fixture"
        repeat = 1
        arm = "C"
        logical_mode = "taskspace"
        metrics_path = $metricsPath
    })
} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $indexPath -Encoding UTF8

$observer = Join-Path $PSScriptRoot "observe-map-compression-experiment.ps1"
& pwsh -NoProfile -File $observer -RunIndexPath $indexPath -OutputDir $output | Out-Null
if ($LASTEXITCODE -ne 0) { throw "observer failed with exit code $LASTEXITCODE" }
$row = (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $output "map-compression-observation.json") |
    ConvertFrom-Json).rows[0]

if ($row.compression_trace_availability -ne "rollout_trace") { throw "trace availability mismatch" }
if ([double]$row.strategy_activation_count -ne 1) { throw "activation count mismatch" }
if ([double]$row.activated_projection_before_median -ne 1000) { throw "before bytes mismatch" }
if ([double]$row.activated_projection_after_median -ne 800) { throw "after bytes mismatch" }
if ([double]$row.activated_projection_ratio -ne 0.8) { throw "ratio mismatch" }
if ([double]$row.folded_node_count_median -ne 3) { throw "folded node count mismatch" }
if ([double]$row.eligible_node_count_median -ne 4) { throw "eligible node count mismatch" }
if ([double]$row.node_detail_before_median -ne 500 -or [double]$row.node_detail_after_median -ne 300) { throw "detail bytes mismatch" }
if ([double]$row.skeleton_before_median -ne 500 -or [double]$row.skeleton_after_median -ne 500) { throw "skeleton bytes mismatch" }

Write-Host "Map compression observer test: PASS"
