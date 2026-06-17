param(
    [Parameter(Mandatory = $true)][string]$RunRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Read-JsonFile {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $null }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Get-Num {
    param($Value, [double]$Default = 0)
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { return $Default }
    try { return [double]$Value } catch { return $Default }
}

function Ratio {
    param($Numerator, $Denominator)
    $den = Get-Num $Denominator
    if ($den -le 0) { return $null }
    [Math]::Round((Get-Num $Numerator) / $den, 4)
}

function First-Metric {
    param([object[]]$Metrics, [string]$Mode)
    foreach ($metric in $Metrics) {
        if ([string]$metric.logical_mode -eq $Mode) { return $metric }
    }
    $null
}

if (-not (Test-Path -LiteralPath $RunRoot)) { throw "RunRoot does not exist: $RunRoot" }

$metricFiles = @(Get-ChildItem -LiteralPath $RunRoot -Filter "metrics.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
$metrics = @($metricFiles | ForEach-Object { Read-JsonFile $_.FullName } | Where-Object { $null -ne $_ })
$standard = First-Metric $metrics "standard"
$taskspace = First-Metric $metrics "taskspace"
$tokenSummary = Read-JsonFile (Join-Path $RunRoot "token-summary.json")
$requestSummary = Read-JsonFile (Join-Path $RunRoot "request-summary.json")
$costGate = Read-JsonFile (Join-Path $RunRoot "suite-cost-gate.json")
$control = Read-JsonFile (Join-Path $RunRoot "taskspace-control-usage.json")
$projection = Read-JsonFile (Join-Path $RunRoot "context-projection-summary.json")

if ($null -eq $standard -or $null -eq $taskspace) {
    throw "Need one standard metrics.json and one taskspace metrics.json under $RunRoot"
}

$stdTotal = (Get-Num $standard.input_tokens) + (Get-Num $standard.output_tokens)
$taskTotal = (Get-Num $taskspace.input_tokens) + (Get-Num $taskspace.output_tokens)
$providerDirectRatio = Ratio $taskTotal $stdTotal
$wallRatio = Ratio $taskspace.wall_time_ms $standard.wall_time_ms
$providerReqRatio = Ratio $taskspace.model_request_count $standard.model_request_count
$rolloutReqRatio = Ratio $taskspace.rollout_trace_model_request_count $standard.model_request_count
$avgProviderInputRatio = Ratio $taskspace.avg_input_tokens_per_request $standard.avg_input_tokens_per_request
$uncachedRatio = Ratio $taskspace.uncached_input_tokens $standard.uncached_input_tokens
$jsonlDensityRatio = Ratio $taskspace.provider_input_tokens_per_jsonl_kb $standard.provider_input_tokens_per_jsonl_kb
$projectionTokenShare = Ratio $taskspace.projection_tokens $taskspace.input_tokens
$runtimeEventsPerRolloutRequest = Ratio $taskspace.taskspace_runtime_event_count $taskspace.rollout_trace_model_request_count
$stateCommitPerRolloutRequest = Ratio $taskspace.runtime_state_commit_count $taskspace.rollout_trace_model_request_count

$drivers = New-Object System.Collections.Generic.List[string]
if ($providerDirectRatio -gt 3.0) { $drivers.Add("direct_tokens_over_partial_budget") }
if ($wallRatio -gt 2.0) { $drivers.Add("walltime_over_release_budget") }
if ($rolloutReqRatio -gt 2.5) { $drivers.Add("rollout_request_count_over_partial_budget") }
if ($avgProviderInputRatio -gt 3.0) { $drivers.Add("avg_input_per_provider_record_over_3x") }
if ($uncachedRatio -gt 3.0) { $drivers.Add("uncached_input_over_3x") }
if ((Get-Num $taskspace.large_output_replay_count) -gt 0) { $drivers.Add("large_output_replay_present") }
if ((Get-Num $taskspace.projection_protected_miss_count) -gt 0) { $drivers.Add("projection_protected_miss_present") }
if ((Get-Num $taskspace.spawn_agent_calls) -gt 0) { $drivers.Add("subagent_fanout_present") }
if ((Get-Num $taskspace.runtime_state_commit_count) -eq 0) { $drivers.Add("state_commit_runtime_adoption_absent") }

$rootCause = "unknown"
if ($rolloutReqRatio -gt 2.5 -and (Get-Num $taskspace.projection_tokens) -lt 10000 -and (Get-Num $taskspace.large_output_replay_count) -eq 0) {
    $rootCause = "active_profile_repeats_compact_taskspace_context_across_many_model_turns"
} elseif ($avgProviderInputRatio -gt 3.0) {
    $rootCause = "provider_visible_base_context_too_large"
} elseif ($wallRatio -gt 2.0) {
    $rootCause = "walltime_overhead_without_token_root_cause"
}

$diagnostics = [pscustomobject]@{
    schema_version = "taskspace-cost-diagnostics-v1"
    run_root = (Resolve-Path -LiteralPath $RunRoot).Path
    generated_at = (Get-Date).ToString("o")
    cost_gate_status = if ($costGate) { [string]$costGate.status } else { "MISSING" }
    root_cause = $rootCause
    drivers = @($drivers.ToArray())
    ratios = [pscustomobject]@{
        provider_direct_input_output_ratio = $providerDirectRatio
        walltime_ratio = $wallRatio
        provider_model_request_count_ratio = $providerReqRatio
        rollout_trace_model_request_count_ratio = $rolloutReqRatio
        avg_provider_input_per_record_ratio = $avgProviderInputRatio
        uncached_input_ratio = $uncachedRatio
        provider_input_per_jsonl_kb_ratio = $jsonlDensityRatio
        projection_token_share_of_taskspace_input = $projectionTokenShare
        runtime_events_per_rollout_request = $runtimeEventsPerRolloutRequest
        runtime_state_commit_per_rollout_request = $stateCommitPerRolloutRequest
    }
    standard = [pscustomobject]@{
        provider_model_request_count = $standard.model_request_count
        rollout_trace_model_request_count = $standard.rollout_trace_model_request_count
        input_tokens = $standard.input_tokens
        uncached_input_tokens = $standard.uncached_input_tokens
        wall_time_ms = $standard.wall_time_ms
    }
    taskspace = [pscustomobject]@{
        provider_model_request_count = $taskspace.model_request_count
        rollout_trace_model_request_count = $taskspace.rollout_trace_model_request_count
        input_tokens = $taskspace.input_tokens
        uncached_input_tokens = $taskspace.uncached_input_tokens
        wall_time_ms = $taskspace.wall_time_ms
        taskspace_runtime_event_count = $taskspace.taskspace_runtime_event_count
        runtime_state_commit_count = $taskspace.runtime_state_commit_count
        projection_count = $taskspace.projection_count
        projection_tokens = $taskspace.projection_tokens
        projection_tokens_max = $taskspace.projection_tokens_max
        large_output_replay_count = $taskspace.large_output_replay_count
        spawn_agent_calls = $taskspace.spawn_agent_calls
    }
    evidence_paths = [pscustomobject]@{
        token_summary = if ($tokenSummary) { Join-Path $RunRoot "token-summary.json" } else { "" }
        request_summary = if ($requestSummary) { Join-Path $RunRoot "request-summary.json" } else { "" }
        suite_cost_gate = if ($costGate) { Join-Path $RunRoot "suite-cost-gate.json" } else { "" }
        taskspace_control_usage = if ($control) { Join-Path $RunRoot "taskspace-control-usage.json" } else { "" }
        context_projection_summary = if ($projection) { Join-Path $RunRoot "context-projection-summary.json" } else { "" }
    }
}

$jsonPath = Join-Path $RunRoot "cost-diagnostics.json"
$mdPath = Join-Path $RunRoot "cost-diagnostics.md"
$diagnostics | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# TaskSpace Cost Diagnostics")
$lines.Add("")
$lines.Add("- status: $($diagnostics.cost_gate_status)")
$lines.Add("- root_cause: $($diagnostics.root_cause)")
$lines.Add("- provider_direct_input_output_ratio: $($diagnostics.ratios.provider_direct_input_output_ratio)")
$lines.Add("- walltime_ratio: $($diagnostics.ratios.walltime_ratio)")
$lines.Add("- rollout_trace_model_request_count_ratio: $($diagnostics.ratios.rollout_trace_model_request_count_ratio)")
$lines.Add("- uncached_input_ratio: $($diagnostics.ratios.uncached_input_ratio)")
$lines.Add("- projection_token_share_of_taskspace_input: $($diagnostics.ratios.projection_token_share_of_taskspace_input)")
$lines.Add("- drivers: $(@($diagnostics.drivers) -join ', ')")
$lines.Add("")
$lines.Add("## Interpretation")
$lines.Add("")
$lines.Add("The diagnostic separates provider aggregate usage from rollout trace turns. A high rollout trace request ratio with low projection token share and zero large-output replay points to repeated compact TaskSpace context across many model turns, not raw tool-output replay or subagent fan-out.")
$lines | Set-Content -LiteralPath $mdPath -Encoding UTF8

[pscustomobject]@{ cost_diagnostics_path = $jsonPath; cost_diagnostics_markdown_path = $mdPath; diagnostics = $diagnostics }
