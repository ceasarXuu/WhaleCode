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
    nodes = @()
    toolCalls = @()
    timeline = @(
        [pscustomobject]@{
            kind = "active_budget"
            trace_event_id = "trace-active-budget-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            tags = @(
                "schema:taskspace-active-budget-v1",
                "producer:runtime",
                "active_budget_source:runtime",
                "profile_name:taskspace-v005-active",
                "route_mode:thin",
                "max_rollout_model_requests:4",
                "max_model_requests_per_node:2",
                "max_spawn_agent_calls:0",
                "max_nodes:4",
                "max_projection_tokens:12000"
            )
        },
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
                "status:response_completed",
                "request_count_before:0",
                "request_count_after:1",
                "max_requests:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_request_count:0",
                "max_model_requests_per_node:2",
                "post_budget_grace_requests:1",
                "runtime_budget_state:normal",
                "request_phase:model_sampling",
                "producer:provider_lifecycle",
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
                "provider_request_status:response_completed",
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
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_request_count:2",
                "max_model_requests_per_node:2",
                "post_budget_grace_requests:1",
                "runtime_budget_state:hard_stopped",
                "request_phase:model_sampling",
                "producer:provider_lifecycle",
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
        },
        [pscustomobject]@{
            kind = "state_commit_displacement"
            trace_event_id = "trace-state-commit-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "commit-1"
            tags = @(
                "schema:taskspace-state-commit-displacement-event-v1",
                "producer:runtime",
                "status:accepted",
                "commit_id:commit-1",
                "accepted_section_count:2",
                "rejected_section_count:0",
                "state_commit_count:1",
                "model_visible_state_commit_count:1",
                "runtime_synthesized_state_commit_count:0",
                "legacy_state_action_attempt_count:2",
                "legacy_state_action_displaced_count:2",
                "legacy_state_action_count:0",
                "active_budget_source:runtime",
                "legacy_state_action_budget:0",
                "budget_response_action_taken:false"
            )
        },
        [pscustomobject]@{
            kind = "spawn_node_budget"
            trace_event_id = "trace-node-budget-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            tags = @(
                "schema:taskspace-spawn-node-budget-event-v1",
                "producer:runtime",
                "budget_kind:node",
                "action:create_node",
                "status:allowed",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_count_before:0",
                "node_count_after:1",
                "max_nodes:4",
                "budget_response_action_taken:false"
            )
        },
        [pscustomobject]@{
            kind = "spawn_node_budget"
            trace_event_id = "trace-node-budget-2"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            tags = @(
                "schema:taskspace-spawn-node-budget-event-v1",
                "producer:runtime",
                "budget_kind:node",
                "action:create_node",
                "status:blocked",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_count_before:4",
                "node_count_after:4",
                "max_nodes:4",
                "budget_response_action_taken:true"
            )
        }
    )
}
$obs | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $obsPath -Encoding UTF8

$instrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $artifactDir -JsonlPath $jsonlPath -ObservabilityJsonPath $obsPath
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget-events.jsonl")) "budget-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "active-budget-events.jsonl")) "active-budget-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget-quality-impact-events.jsonl")) "budget-quality-impact-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "budget_induced_quality_impact_summary.json")) "budget_induced_quality_impact_summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "exact-payload-scan-events.jsonl")) "exact-payload-scan-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "active-context-replacement-report.json")) "active-context-replacement-report.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "provider-request-events.jsonl")) "provider-request-events.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "request-phase-summary.json")) "request-phase-summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "state-commit-displacement.json")) "state-commit-displacement.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "spawn-node-budget-summary.json")) "spawn-node-budget-summary.json was not written"

$budgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$activeBudgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "active-budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$qualityEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-quality-impact-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$scanEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "exact-payload-scan-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$providerEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "provider-request-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$summary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget_induced_quality_impact_summary.json") | ConvertFrom-Json
$replacement = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "active-context-replacement-report.json") | ConvertFrom-Json
$phaseSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "request-phase-summary.json") | ConvertFrom-Json
$stateCommit = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "state-commit-displacement.json") | ConvertFrom-Json
$spawnBudget = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "spawn-node-budget-summary.json") | ConvertFrom-Json
Assert-True ($budgetEvents.Count -eq 2) "budget event count was not extracted from runtime trace"
Assert-True ($activeBudgetEvents.Count -eq 1 -and [string]$activeBudgetEvents[0].route_mode -eq "thin" -and [int]$activeBudgetEvents[0].max_rollout_model_requests -eq 4) "active budget event was not extracted from runtime trace"
Assert-True ([string]$budgetEvents[0].active_budget_source -eq "runtime" -and [string]$budgetEvents[0].route_mode -eq "thin" -and [int]$budgetEvents[0].max_model_requests_per_node -eq 2) "provider budget event did not preserve active budget fields"
Assert-True ($qualityEvents.Count -eq 2) "budget quality event count was not extracted from runtime trace"
Assert-True ($scanEvents.Count -eq 1 -and [bool]$scanEvents[0].passed) "exact payload scan event was not derived from runtime payload trace"
Assert-True ($providerEvents.Count -eq 2 -and [string]$providerEvents[0].schema_version -eq "taskspace-provider-request-budget-event-v1") "provider request events were not derived from runtime budget trace"
Assert-True (@($providerEvents | Where-Object { [string]$_.producer -eq "provider_lifecycle" }).Count -eq 2) "provider request events did not preserve provider_lifecycle producer"
Assert-True ([bool]$replacement.exact_payload_scan_passed -and [bool]$replacement.replacement_confirmed) "active replacement report did not use exact payload scan"
Assert-True ([int]$phaseSummary.provider_request_hook_coverage -eq 100 -and [int]$phaseSummary.request_phase_attribution_coverage -eq 100) "request phase summary did not reflect provider events"
Assert-True ([int]$phaseSummary.provider_request_terminal_coverage -eq 100 -and [int]$phaseSummary.expected_model_request_count -eq 2 -and [int]$phaseSummary.provider_request_distinct_count -eq 2) "request phase summary did not use expected provider request denominator"
Assert-True ([string]$stateCommit.status -eq "pass" -and [string]$stateCommit.source_status -eq "runtime" -and [int]$stateCommit.runtime_event_count -eq 1 -and [bool]$stateCommit.has_displacement_denominator -and [int]$stateCommit.legacy_state_action_attempt_count -eq 2) "state commit displacement summary should pass with runtime denominator evidence"
Assert-True ([string]$spawnBudget.status -eq "pass" -and [string]$spawnBudget.source_status -eq "runtime" -and [int]$spawnBudget.runtime_event_count -eq 2) "spawn/node budget should pass with runtime producer evidence"
Assert-True ([string]$spawnBudget.active_budget_source -eq "runtime" -and [string]$spawnBudget.route_mode -eq "thin" -and [int]$spawnBudget.max_nodes -eq 4) "spawn/node budget summary did not preserve active budget route fields"
Assert-True ([string]$spawnBudget.within_budget_status -eq "fail" -and [string]$spawnBudget.over_budget_enforcement_status -eq "pass" -and [int]$spawnBudget.blocked_budget_event_count -eq 1) "spawn/node budget should split within-budget failure from successful over-budget enforcement"
Assert-True ([bool]$summary.budget_quality_impact_logged_for_every_budget_action) "budget action was not matched to quality impact"
Assert-True ([string]$summary.active_budget_source -eq "runtime" -and [string]$summary.route_mode -eq "thin" -and [int]$summary.max_rollout_model_requests -eq 4 -and [int]$summary.max_model_requests_per_node -eq 2) "budget quality summary did not expose active budget fields"
Assert-True ([int]$summary.budget_quality_impact_missing_count -eq 0) "budget quality impact missing count should be zero"
Assert-True ([int]$summary.blocked_by_budget_samples_count -eq 1) "blocked budget quality impact was not summarized"
Assert-True ([int]$instrumentation.budget_quality_impact_summary.budget_action_count -eq 1) "returned instrumentation object omitted budget summary"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "cost instrumentation selftest passed"
