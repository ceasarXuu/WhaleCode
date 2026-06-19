param([string]$RunRoot = "")
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\cost-instrumentation.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\cost-instrumentation-selftest" }
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

Remove-Item -LiteralPath $RunRoot -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$artifactDir = Join-Path $RunRoot "artifacts"
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
$jsonlPath = Join-Path $RunRoot "whale-exec.jsonl"
$obsPath = Join-Path $RunRoot "action-map-observability.json"

(@(
    [pscustomobject]@{ type = "response.completed"; response = [pscustomobject]@{ usage = [pscustomobject]@{ input_tokens = 10; output_tokens = 5; cached_input_tokens = 2 } } }
) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 8 }) | Set-Content -LiteralPath $jsonlPath -Encoding UTF8

$obs = [pscustomobject]@{
    timeline = @(
        [pscustomobject]@{
            kind = "provider_request_budget"
            trace_event_id = "trace-budget-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-1"
            tags = @(
                "schema:taskspace-provider-request-budget-event-v1",
                "transport:responses_http",
                "status:started",
                "request_count_before:0",
                "request_count_after:1",
                "max_requests:1",
                "request_phase:model_sampling",
                "provider_payload_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "provider_payload_bytes:4321",
                "exact_payload_scan_passed:true",
                "active_projection_present:true",
                "legacy_taskspace_history_present:false",
                "large_raw_output_tokens:0",
                "protected_items_present:true",
                "replacement_confirmed:true"
            )
        },
        [pscustomobject]@{
            kind = "budget_quality_impact"
            trace_event_id = "trace-quality-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-1"
            tags = @(
                "schema:taskspace-budget-quality-impact-v1",
                "provider_request_budget_trace_event_id:trace-budget-1",
                "budget_action:observe",
                "provider_request_status:started",
                "counter_name:provider_request_count",
                "counter_value:1",
                "counter_limit:1",
                "request_phase:model_sampling",
                "score_eligible:true",
                "budget_induced_validation_skip:false",
                "manual_override_used:false",
                "bounded_recovery_used:false",
                "final_classification:score_eligible"
            )
        },
        [pscustomobject]@{
            kind = "provider_request_budget"
            trace_event_id = "trace-budget-2"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-2"
            tags = @(
                "schema:taskspace-provider-request-budget-event-v1",
                "transport:responses_http",
                "status:blocked",
                "request_count_before:1",
                "request_count_after:1",
                "max_requests:1",
                "request_phase:model_sampling",
                "budget_response_action_taken:true"
            )
        },
        [pscustomobject]@{
            kind = "budget_quality_impact"
            trace_event_id = "trace-quality-2"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-2"
            tags = @(
                "schema:taskspace-budget-quality-impact-v1",
                "provider_request_budget_trace_event_id:trace-budget-2",
                "budget_action:hard_stop",
                "provider_request_status:blocked",
                "counter_name:provider_request_count",
                "counter_value:1",
                "counter_limit:1",
                "request_phase:model_sampling",
                "score_eligible:false",
                "budget_induced_validation_skip:false",
                "manual_override_used:false",
                "bounded_recovery_used:false",
                "final_classification:blocked_by_budget"
            )
        }
    )
}
$obs | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $obsPath -Encoding UTF8

$instrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $artifactDir -JsonlPath $jsonlPath -ObservabilityJsonPath $obsPath
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget-events.jsonl")) "budget-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget-quality-impact-events.jsonl")) "budget-quality-impact-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget-induced-quality-impact-summary.json")) "budget-induced-quality-impact-summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "exact-payload-scan-events.jsonl")) "exact-payload-scan-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "active-context-replacement-report.json")) "active-context-replacement-report.json was not written"

$budgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$qualityEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-quality-impact-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$scanEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "exact-payload-scan-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$summary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-induced-quality-impact-summary.json") | ConvertFrom-Json
$replacement = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "active-context-replacement-report.json") | ConvertFrom-Json
Assert-True ($budgetEvents.Count -eq 2) "budget event count was not extracted from runtime trace"
Assert-True ($qualityEvents.Count -eq 2) "budget quality event count was not extracted from runtime trace"
Assert-True ($scanEvents.Count -eq 1 -and [bool]$scanEvents[0].passed) "exact payload scan event was not derived from runtime payload trace"
Assert-True ([bool]$replacement.exact_payload_scan_passed -and [bool]$replacement.replacement_confirmed) "active replacement report did not use exact payload scan"
Assert-True ([bool]$summary.budget_quality_impact_logged_for_every_budget_action) "budget action was not matched to quality impact"
Assert-True ([int]$summary.budget_quality_impact_missing_count -eq 0) "budget quality impact missing count should be zero"
Assert-True ([int]$summary.blocked_by_budget_samples_count -eq 1) "blocked budget quality impact was not summarized"
Assert-True ([int]$instrumentation.budget_quality_impact_summary.budget_action_count -eq 1) "returned instrumentation object omitted budget summary"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "cost instrumentation selftest passed"
