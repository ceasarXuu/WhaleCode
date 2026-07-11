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
                "max_rollout_model_requests:8",
                "max_model_requests_per_node:3",
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
                "schema:taskspace-provider-request-reason-v1",
                "transport:responses_http",
                "status:response_completed",
                "request_count_before:0",
                "request_count_after:1",
                "max_requests:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_request_count:0",
                "max_model_requests_per_node:3",
                "post_budget_grace_requests:1",
                "runtime_budget_state:normal",
                "request_phase:model_sampling",
                "node_kind:inspect_code_context",
                "trigger_kind:model_sampling",
                "response_actionability_previous:none",
                "previous_response_recovery_action:none",
                "latest_tool_result_refs:none",
                "model_visible_feedback_refs:none",
                "adoption_blockers:none",
                "projection_bundle_hash:dynamic-suffix-hash",
                "request_reason_delta:initial_request",
                "repeated_same_reason_count:0",
                "reason_confidence:derived",
                "producer:provider_lifecycle",
                "started_at_ms:100",
                "completed_at_ms:715",
                "latency_ms:615",
                "model_request_duration_ms:615",
                "input_tokens:10",
                "cached_input_tokens:2",
                "output_tokens:5",
                "provider_payload_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "provider_payload_bytes:4321",
                "provider_wire_api:ChatCompletions",
                "tools_count:24",
                "tools_present:true",
                "request_shape_classifier:native_tools_schema_hot_path",
                "messages_hash:messages-hash",
                "stable_prefix_hash:stable-prefix-hash",
                "dynamic_suffix_hash:dynamic-suffix-hash",
                "exact_payload_scan_passed:true",
                "exact_payload_scan_event_id:scan-provider-request-1",
                "active_projection_present:true",
                "active_projection_count:1",
                "context_bundle_present:true",
                "exact_context_bundle_verified:true",
                "cache_plan_verified:true",
                "legacy_taskspace_history_present:false",
                "raw_taskspace_control_history_tokens:0",
                "completed_stale_node_history_tokens:0",
                "rejected_subagent_body_tokens:0",
                "large_raw_output_tokens:0",
                "protected_items_present:true",
                "replacement_confirmed:true"
            )
        },
        [pscustomobject]@{
            kind = "exact_payload_scan"
            trace_event_id = "trace-scan-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-1"
            tags = @(
                "schema:taskspace-exact-payload-scan-event-v1",
                "producer:provider_payload_scanner",
                "scan_event_id:scan-provider-request-1",
                "provider_request_budget_trace_event_id:trace-budget-1",
                "provider_payload_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "provider_payload_bytes:4321",
                "scanner_version:v005-exact-scan-2",
                "matcher_version:v005-marker-and-structural-negative-checks-2",
                "checked_byte_ranges:0-4321",
                "negative_checks_performed:legacy_taskspace_history,raw_taskspace_control_history,large_raw_output",
                "active_projection_present:true",
                "active_projection_count:1",
                "context_bundle_present:true",
                "exact_context_bundle_verified:true",
                "cache_plan_verified:true",
                "legacy_taskspace_history_present:false",
                "raw_taskspace_control_history_tokens:0",
                "completed_stale_node_history_tokens:0",
                "rejected_subagent_body_tokens:0",
                "large_raw_output_tokens:0",
                "protected_items_present:true",
                "replacement_confirmed:true",
                "passed:true",
                "failure_reasons:none"
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
                "active_budget_source:runtime",
                "route_mode:thin",
                "budget_state_before:normal",
                "budget_state_after:normal",
                "budget_transition_reason:provider_request_within_profile_hint",
                "request_phase:model_sampling",
                "logical_request_id:logical-1",
                "attempt_seq:1",
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
                "schema:taskspace-provider-request-reason-v1",
                "transport:responses_http",
                "status:response_completed",
                "request_count_before:1",
                "request_count_after:2",
                "max_requests:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_request_count:2",
                "max_model_requests_per_node:3",
                "post_budget_grace_requests:1",
                "runtime_budget_state:over_profile_hint",
                "request_phase:validation_recovery",
                "node_kind:smoke_test",
                "trigger_kind:response_recovery",
                "response_actionability_previous:tool_feedback_recovery",
                "previous_response_recovery_action:tool_feedback_recovery",
                "previous_response_trace_event_id:trace-response-1",
                "latest_tool_result_refs:result-1",
                "model_visible_feedback_refs:result-1|trace-response-1",
                "adoption_blockers:validation_rework_artifacts:output.json",
                "projection_bundle_hash:dynamic-suffix-hash-2",
                "request_reason_delta:changed_trigger",
                "repeated_same_reason_count:0",
                "reason_confidence:direct",
                "producer:provider_lifecycle",
                "input_tokens:4",
                "cached_input_tokens:1",
                "output_tokens:2",
                "provider_payload_sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "provider_payload_bytes:2048",
                "provider_wire_api:Responses",
                "tools_count:0",
                "tools_present:false",
                "request_shape_classifier:tool_free_action_contract",
                "messages_hash:messages-hash-2",
                "stable_prefix_hash:stable-prefix-hash",
                "dynamic_suffix_hash:dynamic-suffix-hash-2",
                "budget_response_action_taken:false"
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
                "budget_action:observe",
                "provider_request_status:response_completed",
                "counter_name:provider_request_count",
                "counter_value:2",
                "counter_limit:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "budget_state_before:normal",
                "budget_state_after:over_profile_hint",
                "budget_transition_reason:provider_request_profile_hint_exceeded",
                "request_phase:validation_recovery",
                "logical_request_id:logical-2",
                "attempt_seq:1",
                "score_eligible:true",
                "budget_induced_validation_skip:false",
                "manual_override_used:false",
                "bounded_recovery_used:false",
                "final_classification:score_eligible"
            )
        },
        [pscustomobject]@{
            kind = "provider_request_budget"
            trace_event_id = "trace-budget-3"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-3"
            tags = @(
                "schema:taskspace-provider-request-budget-event-v1",
                "schema:taskspace-provider-request-reason-v1",
                "transport:responses_http",
                "status:response_completed",
                "request_count_before:2",
                "request_count_after:3",
                "max_requests:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_request_count:2",
                "max_model_requests_per_node:3",
                "post_budget_grace_requests:1",
                "runtime_budget_state:over_profile_hint",
                "request_phase:state_commit",
                "node_kind:implement_solution",
                "trigger_kind:response_recovery",
                "response_actionability_previous:tool_feedback_recovery",
                "previous_response_recovery_action:tool_feedback_recovery",
                "previous_response_trace_event_id:trace-response-2",
                "latest_tool_result_refs:result-1",
                "model_visible_feedback_refs:result-1|trace-response-2",
                "adoption_blockers:none",
                "projection_bundle_hash:dynamic-suffix-hash-3",
                "request_reason_delta:none",
                "repeated_same_reason_count:1",
                "reason_confidence:direct",
                "producer:provider_lifecycle",
                "input_tokens:7",
                "cached_input_tokens:3",
                "output_tokens:1",
                "provider_payload_sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "provider_payload_bytes:3072",
                "provider_wire_api:Responses",
                "tools_count:0",
                "tools_present:false",
                "request_shape_classifier:tool_free_action_contract",
                "messages_hash:messages-hash-3",
                "stable_prefix_hash:stable-prefix-hash",
                "dynamic_suffix_hash:dynamic-suffix-hash-3",
                "budget_response_action_taken:false"
            )
        },
        [pscustomobject]@{
            kind = "budget_quality_impact"
            trace_event_id = "trace-quality-3"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "provider-request-3"
            tags = @(
                "schema:taskspace-budget-quality-impact-v1",
                "provider_request_budget_trace_event_id:trace-budget-3",
                "budget_action:observe",
                "provider_request_status:response_completed",
                "counter_name:provider_request_count",
                "counter_value:3",
                "counter_limit:1",
                "active_budget_source:runtime",
                "route_mode:thin",
                "budget_state_before:over_profile_hint",
                "budget_state_after:over_profile_hint",
                "budget_transition_reason:provider_request_profile_hint_exceeded",
                "request_phase:state_commit",
                "logical_request_id:logical-3",
                "attempt_seq:1",
                "score_eligible:true",
                "budget_induced_validation_skip:false",
                "manual_override_used:false",
                "bounded_recovery_used:false",
                "final_classification:score_eligible"
            )
        },
        [pscustomobject]@{
            kind = "legacy_state_action_attempt"
            trace_event_id = "trace-legacy-attempt-1"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "record_fact"
            tags = @(
                "schema:taskspace-legacy-state-action-attempt-v1",
                "producer:runtime",
                "action:record_fact",
                "displaced:true",
                "allowed:false",
                "reason:active_profile_requires_state_commit",
                "active_budget_source:runtime",
                "budget_response_action_taken:false"
            )
        },
        [pscustomobject]@{
            kind = "legacy_state_action_attempt"
            trace_event_id = "trace-legacy-attempt-2"
            task_id = "task-1"
            map_id = "map-1"
            node_id = "node-1"
            call_id = "record_decision"
            tags = @(
                "schema:taskspace-legacy-state-action-attempt-v1",
                "producer:runtime",
                "action:record_decision",
                "displaced:true",
                "allowed:false",
                "reason:active_profile_requires_state_commit",
                "active_budget_source:runtime",
                "budget_response_action_taken:false"
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
                "state_commit_section_count:2",
                "state_commit_count:1",
                "model_visible_state_commit_count:1",
                "runtime_synthesized_state_commit_count:0",
                "legacy_state_action_attempt_count:0",
                "legacy_state_action_displaced_count:0",
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
                "status:allowed",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-active",
                "node_count_before:4",
                "node_count_after:5",
                "max_nodes:4",
                "budget_response_action_taken:false"
            )
        },
        [pscustomobject]@{
            kind = "snapshot_updated"
            details = [pscustomobject]@{
                type = "snapshot_updated"
                snapshot = [pscustomobject]@{
                    maps = @(
                        [pscustomobject]@{
                            id = "map-1"
                            results = @(
                                [pscustomobject]@{
                                    id = "result-subagent-1"
                                    nodeId = "node-subagent-1"
                                    kind = "result"
                                    subagentPlanId = "subagent-plan-1"
                                    evidencePackage = [pscustomobject]@{
                                        validity = "accepted"
                                    }
                                }
                            )
                        }
                    )
                }
            }
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
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "request-reason-summary.json")) "request-reason-summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "provider-cache-trace.jsonl")) "provider-cache-trace.jsonl was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "provider-cache-trace-summary.json")) "provider-cache-trace-summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "state-commit-displacement.json")) "state-commit-displacement.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $artifactDir "spawn-node-budget-summary.json")) "spawn-node-budget-summary.json was not written"

$budgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$activeBudgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "active-budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$qualityEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget-quality-impact-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$scanEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "exact-payload-scan-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$providerEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "provider-request-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$cacheTraceEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "provider-cache-trace.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$summary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "budget_induced_quality_impact_summary.json") | ConvertFrom-Json
$cacheTraceSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "provider-cache-trace-summary.json") | ConvertFrom-Json
$replacement = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "active-context-replacement-report.json") | ConvertFrom-Json
$phaseSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "request-phase-summary.json") | ConvertFrom-Json
$reasonSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "request-reason-summary.json") | ConvertFrom-Json
$stateCommit = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "state-commit-displacement.json") | ConvertFrom-Json
$spawnBudget = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifactDir "spawn-node-budget-summary.json") | ConvertFrom-Json
Assert-True ($budgetEvents.Count -eq 3) "budget event count was not extracted from runtime trace"
Assert-True ($activeBudgetEvents.Count -eq 1 -and [string]$activeBudgetEvents[0].route_mode -eq "thin" -and [int]$activeBudgetEvents[0].max_rollout_model_requests -eq 8) "active budget event was not extracted from runtime trace"
Assert-True ([string]$budgetEvents[0].active_budget_source -eq "runtime" -and [string]$budgetEvents[0].route_mode -eq "thin" -and [int]$budgetEvents[0].max_model_requests_per_node -eq 3) "provider budget event did not preserve active budget fields"
Assert-True ($qualityEvents.Count -eq 3) "budget quality event count was not extracted from runtime trace"
Assert-True ([string]$qualityEvents[1].active_budget_source -eq "runtime" -and [string]$qualityEvents[1].route_mode -eq "thin") "budget quality event did not preserve active budget route fields"
Assert-True ([string]$qualityEvents[1].budget_state_after -eq "over_profile_hint" -and [string]$qualityEvents[1].budget_transition_reason -eq "provider_request_profile_hint_exceeded") "budget quality event did not preserve budget state transition fields"
Assert-True ([string]$qualityEvents[1].logical_request_id -eq "logical-2" -and [int]$qualityEvents[1].attempt_seq -eq 1) "budget quality event did not preserve logical request identity"
Assert-True ($scanEvents.Count -eq 1 -and [bool]$scanEvents[0].passed -and [string]$scanEvents[0].producer -eq "provider_payload_scanner") "exact payload scan event was not read from provider-owned runtime trace"
Assert-True ([bool]$scanEvents[0].protected_items_present) "exact payload scan should preserve protected item proof"
Assert-True ([bool]$scanEvents[0].exact_context_bundle_verified) "exact payload scan should preserve bundle proof"
Assert-True ([int]$scanEvents[0].active_projection_count -eq 1) "exact payload scan should preserve active projection count"
Assert-True ($providerEvents.Count -eq 3 -and [string]$providerEvents[0].schema_version -eq "taskspace-provider-request-budget-event-v1") "provider request events were not derived from runtime budget trace"
Assert-True (@($providerEvents | Where-Object { [string]$_.producer -eq "provider_lifecycle" }).Count -eq 3) "provider request events did not preserve provider_lifecycle producer"
Assert-True ([string]$providerEvents[0].provider_wire_api -eq "ChatCompletions" -and [int]$providerEvents[0].tools_count -eq 24 -and [string]$providerEvents[0].request_shape_classifier -eq "native_tools_schema_hot_path") "provider request events did not preserve cache request shape fields"
Assert-True ([int64]$providerEvents[0].model_request_duration_ms -eq 615 -and [int64]$providerEvents[0].latency_ms -eq 615 -and [int64]$providerEvents[0].started_at_ms -eq 100 -and [int64]$providerEvents[0].completed_at_ms -eq 715) "provider request events did not preserve provider lifecycle timing fields"
Assert-True ([bool]$providerEvents[0].request_reason_schema_present -and [string]$providerEvents[0].trigger_kind -eq "model_sampling" -and [string]$providerEvents[0].request_reason_delta -eq "initial_request") "provider request events did not preserve request reason fields"
Assert-True ([string]$providerEvents[1].response_actionability_previous -eq "tool_feedback_recovery" -and [string]$providerEvents[1].latest_tool_result_refs -eq "result-1" -and [string]$providerEvents[1].model_visible_feedback_refs -eq "result-1|trace-response-1") "provider request events did not preserve feedback refs"
Assert-True ([int]$providerEvents[2].repeated_same_reason_count -eq 1 -and [string]$providerEvents[2].request_reason_delta -eq "none") "provider request events did not preserve repeated same-reason detector fields"
Assert-True ($cacheTraceEvents.Count -eq 3 -and [string]$cacheTraceEvents[0].schema_version -eq "TaskSpaceProviderCacheTraceV1" -and [double]$cacheTraceEvents[0].hit_rate -eq 0.2) "provider cache trace events were not derived from terminal provider requests"
Assert-True ([int]$cacheTraceSummary.native_tools_schema_hot_path_count -eq 1 -and [int]$cacheTraceSummary.tool_free_action_contract_count -eq 2 -and [double]$cacheTraceSummary.trace_coverage -eq 1.0) "provider cache trace summary did not classify completed request shapes"
Assert-True ([bool]$replacement.exact_payload_scan_passed -and [bool]$replacement.replacement_confirmed) "active replacement report did not use exact payload scan"
Assert-True ([int]$replacement.active_projection_count_max -eq 1 -and [int]$replacement.active_projection_uniqueness_violation_count -eq 0) "active replacement report did not enforce projection uniqueness"
Assert-True ([bool]$replacement.context_bundle_present -and [bool]$replacement.exact_context_bundle_verified -and [bool]$replacement.cache_plan_verified) "active replacement report did not preserve bundle proof"
Assert-True ([bool]$replacement.protected_items_present -and [bool]$replacement.exact_payload_scan_matching_provider_event) "active replacement report did not preserve exact scan join evidence"
$repeatedLifecycleReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($scanEvents[0], $scanEvents[0])
Assert-True ([int]$repeatedLifecycleReplacement.active_context_replacement_report.matching_payload_scan_count -eq 1) "repeated lifecycle scans should be deduplicated by scan event id"
$budgetOnlyReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @()
Assert-True (-not [bool]$budgetOnlyReplacement.active_context_replacement_report.exact_payload_scan_passed -and @($budgetOnlyReplacement.exact_payload_scan_events).Count -eq 0) "budget-only payload booleans should not synthesize exact scan evidence"
$duplicateProjectionScan = [pscustomobject]@{
    scan_event_id = "scan-provider-request-duplicate"
    request_id = [string]$scanEvents[0].request_id
    provider_payload_sha256 = [string]$scanEvents[0].provider_payload_sha256
    producer = "provider_payload_scanner"
    matching_provider_event = $true
    passed = $false
    replacement_confirmed = $false
    active_projection_count = 2
    context_bundle_present = $false
    exact_context_bundle_verified = $false
    cache_plan_verified = $false
    legacy_taskspace_history_present = $false
    raw_taskspace_control_history_tokens = 0
    completed_stale_node_history_tokens = 0
    rejected_subagent_body_tokens = 0
    large_raw_output_tokens = 0
    protected_items_present = $true
}
$duplicateProjectionReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($duplicateProjectionScan)
Assert-True (-not [bool]$duplicateProjectionReplacement.active_context_replacement_report.replacement_confirmed) "duplicate active projection should not confirm replacement"
Assert-True ([int]$duplicateProjectionReplacement.active_context_replacement_report.active_projection_uniqueness_violation_count -eq 1) "duplicate active projection violation was not reported"
Assert-True ([int]$phaseSummary.provider_request_hook_coverage -eq 100 -and [int]$phaseSummary.request_phase_attribution_coverage -eq 100) "request phase summary did not reflect provider events"
Assert-True ([int]$phaseSummary.provider_request_terminal_coverage -eq 100 -and [int]$phaseSummary.expected_model_request_count -eq 3 -and [int]$phaseSummary.provider_request_distinct_count -eq 3) "request phase summary did not use expected provider request denominator"
Assert-True ([int]$phaseSummary.phase_counts.model_sampling -eq 1 -and [int]$phaseSummary.phase_counts.validation_recovery -eq 1 -and [int]$phaseSummary.phase_counts.state_commit -eq 1) "request phase summary did not expose phase counts"
Assert-True ([bool]$phaseSummary.phase_diversity_gate_pass -and [int]$phaseSummary.non_model_sampling_distinct_phase_count -eq 2) "request phase summary did not enforce non-model phase diversity"
Assert-True ([int64]$phaseSummary.phase_token_summary.state_commit.input_tokens -eq 7 -and [int64]$phaseSummary.phase_token_summary.validation_recovery.cached_input_tokens -eq 1) "request phase summary did not expose phase token totals"
Assert-True ([string]$reasonSummary.request_reason_coverage_status -eq "measured" -and [int]$reasonSummary.request_reason_unknown_count -eq 0 -and [int]$reasonSummary.request_reason_attribution_coverage -eq 100) "request reason summary did not report complete measured coverage"
Assert-True ([int]$reasonSummary.repeated_same_reason_no_delta_count -eq 1 -and [int]$reasonSummary.trigger_kind_counts.response_recovery -eq 2 -and [int]$reasonSummary.request_reason_delta_counts.none -eq 1) "request reason summary did not expose repeated no-delta and trigger counts"
Assert-True ([string]$stateCommit.status -eq "pass" -and [string]$stateCommit.source_status -eq "runtime" -and [int]$stateCommit.runtime_event_count -eq 1 -and [bool]$stateCommit.has_displacement_denominator -and [int]$stateCommit.legacy_state_action_attempt_count -eq 2) "state commit displacement summary should pass with runtime denominator evidence"
Assert-True ([int]$stateCommit.legacy_state_action_attempt_event_count -eq 2 -and [int]$stateCommit.state_commit_section_count -eq 2) "state commit displacement should report legacy attempts separately from state_commit sections"
Assert-True ([string]$spawnBudget.status -eq "pass" -and [string]$spawnBudget.source_status -eq "runtime" -and [int]$spawnBudget.runtime_event_count -eq 2) "spawn/node budget should pass with runtime producer evidence"
Assert-True ([string]$spawnBudget.active_budget_source -eq "runtime" -and [string]$spawnBudget.route_mode -eq "thin" -and [int]$spawnBudget.max_nodes -eq 4) "spawn/node budget summary did not preserve active budget route fields"
Assert-True ([string]$spawnBudget.within_budget_status -eq "over_profile_hint" -and [string]$spawnBudget.over_budget_enforcement_status -eq "advisory_only" -and [bool]$spawnBudget.over_profile_hint -and [int]$spawnBudget.blocked_budget_event_count -eq 0) "spawn/node budget should report over-profile hints without enforcing a hard budget"
Assert-True ([string]$spawnBudget.subagent_review_debt_status -eq "no_unreviewed_subagent_results" -and [int]$spawnBudget.subagent_result_count -eq 1 -and [int]$spawnBudget.reviewed_subagent_result_count -eq 1 -and [int]$spawnBudget.unreviewed_subagent_result_count -eq 0) "spawn/node budget should expose reviewed subagent result debt status"
Assert-True ([bool]$summary.budget_quality_impact_logged_for_every_budget_action) "budget action was not matched to quality impact"
Assert-True ([string]$summary.active_budget_source -eq "runtime" -and [string]$summary.route_mode -eq "thin" -and [int]$summary.max_rollout_model_requests -eq 8 -and [int]$summary.max_model_requests_per_node -eq 3) "budget quality summary did not expose active budget fields"
Assert-True ([int]$summary.budget_quality_impact_missing_count -eq 0) "budget quality impact missing count should be zero"
Assert-True ([int]$summary.blocked_by_budget_samples_count -eq 0) "budget quality impact should not summarize profile overruns as blocked_by_budget"
Assert-True ([int]$instrumentation.budget_quality_impact_summary.budget_action_count -eq 0) "returned instrumentation object should not classify profile hints as budget actions"

$rolloutOnlyArtifactDir = Join-Path $RunRoot "rollout-only-artifacts"
New-Item -ItemType Directory -Path $rolloutOnlyArtifactDir -Force | Out-Null
$rolloutOnlyJsonl = Join-Path $RunRoot "rollout-only-whale-exec.jsonl"
(@(
    [pscustomobject]@{ type = "response.completed"; response = [pscustomobject]@{ usage = [pscustomobject]@{ input_tokens = 12; output_tokens = 6; cached_input_tokens = 3 } } }
) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 8 }) | Set-Content -LiteralPath $rolloutOnlyJsonl -Encoding UTF8
$rolloutOnlyPath = Join-Path $rolloutOnlyArtifactDir "rollout.jsonl"
@(
    [pscustomobject]@{
        type = "event_msg"
        payload = [pscustomobject]@{
            kind = "active_budget"
            traceEventId = "rollout-active-budget-1"
            taskId = "task-rollout"
            mapId = "map-rollout"
            nodeId = "node-rollout"
            tags = @(
                "schema:taskspace-active-budget-v1",
                "producer:runtime",
                "active_budget_source:runtime",
                "profile_name:taskspace-v005-thin",
                "route_mode:thin",
                "max_rollout_model_requests:8",
                "max_model_requests_per_node:3",
                "max_spawn_agent_calls:0",
                "max_nodes:4",
                "max_projection_tokens:12000"
            )
        }
    },
    [pscustomobject]@{
        type = "event_msg"
        payload = [pscustomobject]@{
            kind = "provider_request_budget"
            traceEventId = "rollout-budget-1"
            taskId = "task-rollout"
            mapId = "map-rollout"
            nodeId = "node-rollout"
            callId = "provider-request-rollout-1"
            tags = @(
                "schema:taskspace-provider-request-budget-event-v1",
                "transport:responses_http",
                "status:response_completed",
                "request_count_before:4",
                "request_count_after:5",
                "max_requests:4",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-thin",
                "node_request_count:2",
                "max_model_requests_per_node:3",
                "post_budget_grace_requests:1",
                "runtime_budget_state:over_profile_hint",
                "request_phase:model_sampling",
                "producer:provider_lifecycle",
                "budget_response_action_taken:false"
            )
        }
    },
    [pscustomobject]@{
        type = "event_msg"
        payload = [pscustomobject]@{
            kind = "budget_quality_impact"
            traceEventId = "rollout-quality-1"
            taskId = "task-rollout"
            mapId = "map-rollout"
            nodeId = "node-rollout"
            callId = "provider-request-rollout-1"
            tags = @(
                "schema:taskspace-budget-quality-impact-v1",
                "provider_request_budget_trace_event_id:rollout-budget-1",
                "budget_action:observe",
                "provider_request_status:response_completed",
                "counter_name:provider_request_count",
                "counter_value:5",
                "counter_limit:4",
                "active_budget_source:runtime",
                "route_mode:thin",
                "budget_state_before:over_profile_hint",
                "budget_state_after:over_profile_hint",
                "budget_transition_reason:provider_request_profile_hint_exceeded",
                "request_phase:model_sampling",
                "logical_request_id:logical-rollout-1",
                "attempt_seq:1",
                "score_eligible:true",
                "budget_induced_validation_skip:false",
                "manual_override_used:false",
                "bounded_recovery_used:false",
                "final_classification:score_eligible"
            )
        }
    },
    [pscustomobject]@{
        type = "event_msg"
        payload = [pscustomobject]@{
            kind = "spawn_node_budget"
            traceEventId = "rollout-node-budget-1"
            taskId = "task-rollout"
            mapId = "map-rollout"
            nodeId = "node-rollout"
            tags = @(
                "schema:taskspace-spawn-node-budget-event-v1",
                "producer:runtime",
                "budget_kind:node",
                "action:create_node",
                "status:allowed",
                "active_budget_source:runtime",
                "route_mode:thin",
                "profile_name:taskspace-v005-thin",
                "node_count_before:0",
                "node_count_after:1",
                "max_nodes:4",
                "budget_response_action_taken:false"
            )
        }
    }
) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 12 } | Set-Content -LiteralPath $rolloutOnlyPath -Encoding UTF8
$rolloutOnlyInstrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $rolloutOnlyArtifactDir -JsonlPath $rolloutOnlyJsonl -ObservabilityJsonPath ""
$rolloutOnlyBudgetEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $rolloutOnlyArtifactDir "budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$rolloutOnlyActiveEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $rolloutOnlyArtifactDir "active-budget-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$rolloutOnlyQualityEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $rolloutOnlyArtifactDir "budget-quality-impact-events.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
$rolloutOnlySummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $rolloutOnlyArtifactDir "budget_induced_quality_impact_summary.json") | ConvertFrom-Json
$rolloutOnlySpawn = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $rolloutOnlyArtifactDir "spawn-node-budget-summary.json") | ConvertFrom-Json
Assert-True ($rolloutOnlyActiveEvents.Count -eq 1 -and [string]$rolloutOnlyActiveEvents[0].profile_name -eq "taskspace-v005-thin") "rollout-only active budget event was not extracted"
Assert-True ($rolloutOnlyBudgetEvents.Count -eq 1 -and [string]$rolloutOnlyBudgetEvents[0].status -eq "response_completed" -and -not [bool]$rolloutOnlyBudgetEvents[0].budget_response_action_taken) "rollout-only provider budget event was not extracted as advisory-only"
Assert-True ($rolloutOnlyQualityEvents.Count -eq 1 -and [string]$rolloutOnlyQualityEvents[0].final_classification -eq "score_eligible") "rollout-only quality impact event was not extracted as advisory-only"
Assert-True ([string]$rolloutOnlyQualityEvents[0].budget_state_after -eq "over_profile_hint" -and [string]$rolloutOnlyQualityEvents[0].logical_request_id -eq "logical-rollout-1") "rollout-only quality impact event did not preserve full budget quality fields"
Assert-True ([int]$rolloutOnlySummary.budget_event_count -eq 1 -and [int]$rolloutOnlySummary.budget_quality_impact_event_count -eq 1 -and [string]$rolloutOnlySummary.route_mode -eq "thin") "rollout-only budget summary did not use rollout trace events"
Assert-True ([string]$rolloutOnlySpawn.source_status -eq "runtime" -and [int]$rolloutOnlySpawn.runtime_event_count -eq 1) "rollout-only spawn/node budget summary did not use rollout trace events"
Assert-True ([string]$rolloutOnlySpawn.subagent_review_debt_status -eq "not_measured" -and [int]$rolloutOnlySpawn.unreviewed_subagent_result_count -eq 0) "rollout-only spawn/node summary should make missing snapshot review evidence explicit"
Assert-True ([int]$rolloutOnlyInstrumentation.budget_quality_impact_summary.blocked_by_budget_samples_count -eq 0) "returned rollout-only instrumentation should not classify profile hints as blocked_by_budget"

$rolloutControlDir = Join-Path $RunRoot "rollout-control-artifacts"
New-Item -ItemType Directory -Path $rolloutControlDir -Force | Out-Null
$rolloutControlJsonl = Join-Path $RunRoot "rollout-control-whale-exec.jsonl"
'{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}' | Set-Content -LiteralPath $rolloutControlJsonl -Encoding UTF8
@(
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"finish_then_actions\"}","call_id":"native-control-1"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"finish_then_actions\"}","call_id":"native-control-2"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"bind_node\"}","call_id":"taskspace-action-contract-3-taskspace_control"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-1","output":"{\"schema_version\":\"TaskSpaceControlBatchResultV1\",\"success\":false,\"status\":\"partial\"}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-2","output":"TaskSpaceGateRecoveryV1: {\"allowed\":false}"}}'
) | Set-Content -LiteralPath (Join-Path $rolloutControlDir "rollout.jsonl") -Encoding UTF8
$rolloutControlInstrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $rolloutControlDir -JsonlPath $rolloutControlJsonl -ObservabilityJsonPath ""
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count -eq 3) "rollout taskspace_control calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.native_taskspace_control_count -eq 2) "rollout native taskspace_control calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_contract_taskspace_control_count -eq 1) "rollout action-contract taskspace_control calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_counts.finish_then_actions -eq 2) "rollout control actions were not deduplicated by call id"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_failure_count -eq 2) "rollout control failures were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_protocol_failure_count -eq 1) "protocol control failures were not classified"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_state_failure_count -eq 0) "state control failures were misclassified"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.nested_action_failure_count -eq 1) "nested action failures were not classified"
Assert-True ([string]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count_source -eq "rollout_trace") "rollout trace should be the authoritative control-count source"
Assert-True (-not [bool]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count_source_mismatch) "an exec file without response-item telemetry should not be treated as a conflicting control count"

$wireTraceDir = Join-Path $RunRoot "provider-wire-trace-artifacts"
New-Item -ItemType Directory -Path $wireTraceDir -Force | Out-Null
$wireTraceJsonl = Join-Path $RunRoot "provider-wire-trace-whale-exec.jsonl"
'{"type":"turn.completed","usage":{"input_tokens":300,"cached_input_tokens":200,"output_tokens":20}}' | Set-Content -LiteralPath $wireTraceJsonl -Encoding UTF8
@(
    '{"schema_version":"provider-chat-wire-trace-v2","event_name":"provider.chat_wire_shape_recorded","request_id":"wire-1","epoch_id":"epoch-1","request_index":1,"provider_wire_api":"ChatCompletions","pre_wire_payload_sha256":"pre-1","provider_payload_sha256":"wire-hash-1","provider_payload_bytes":100,"messages_hash":"messages-1","tools_hash":"tools-1","cache_shape_hash":"shape-named","tools_count":2,"tool_choice_kind":"named_function","tool_choice_name":"taskspace_control","message_count":2,"message_shapes":[{"index":0,"role":"system","bytes":40,"message_sha256":"m0","content_sha256":"c0"},{"index":1,"role":"user","bytes":20,"message_sha256":"m1","content_sha256":"c1"}],"previous_request_id":null,"lcp_message_count":0,"lcp_message_bytes":0,"message_prefix_preserved":null,"tool_choice_preserved":null,"tool_choice_changed":null,"prefix_preserved":null,"first_diff_index":null,"first_diff_path":null,"status":"payload_captured"}',
    '{"schema_version":"provider-chat-wire-trace-v2","event_name":"provider.chat_wire_request_terminal","request_id":"wire-1","status":"response_completed","input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":1,"total_tokens":110}',
    '{"schema_version":"provider-chat-wire-trace-v2","event_name":"provider.chat_wire_prefix_broken","request_id":"wire-2","epoch_id":"epoch-1","request_index":2,"provider_wire_api":"ChatCompletions","pre_wire_payload_sha256":"pre-2","provider_payload_sha256":"wire-hash-2","provider_payload_bytes":140,"messages_hash":"messages-2","tools_hash":"tools-1","cache_shape_hash":"shape-auto","tools_count":2,"tool_choice_kind":"auto","tool_choice_name":null,"message_count":3,"message_shapes":[{"index":0,"role":"system","bytes":40,"message_sha256":"m0","content_sha256":"c0"},{"index":1,"role":"user","bytes":20,"message_sha256":"m1","content_sha256":"c1"},{"index":2,"role":"assistant","bytes":30,"message_sha256":"m2","content_sha256":"c2"}],"previous_request_id":"wire-1","lcp_message_count":2,"lcp_message_bytes":60,"message_prefix_preserved":true,"tool_choice_preserved":false,"tool_choice_changed":true,"prefix_preserved":false,"first_diff_index":null,"first_diff_path":"tool_choice","status":"payload_captured"}',
    '{"schema_version":"provider-chat-wire-trace-v2","event_name":"provider.chat_wire_request_terminal","request_id":"wire-2","status":"response_completed","input_tokens":200,"cached_input_tokens":180,"output_tokens":10,"reasoning_output_tokens":1,"total_tokens":210}'
) | Set-Content -LiteralPath (Join-Path $wireTraceDir "provider-wire-trace.jsonl") -Encoding UTF8
$wireTraceInstrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $wireTraceDir -JsonlPath $wireTraceJsonl -ObservabilityJsonPath ""
$wireTraceSummary = $wireTraceInstrumentation.provider_cache_trace_summary
$wireTraceEvents = @($wireTraceInstrumentation.provider_cache_trace_events)
Assert-True ([string]$wireTraceSummary.schema_version -eq "TaskSpaceProviderCacheTraceSummaryV3" -and [string]$wireTraceSummary.source -eq "provider_final_wire_trace") "provider final-wire trace was not selected as cache source"
Assert-True ([int]$wireTraceSummary.provider_request_count -eq 2 -and [double]$wireTraceSummary.trace_coverage -eq 1.0) "provider final-wire trace coverage was not complete"
Assert-True ([double]$wireTraceSummary.request_2_plus_hit_rate -eq 0.9) "provider final-wire request-2+ cache usage was not aggregated"
Assert-True ([int]$wireTraceSummary.prefix_preserved_count -eq 0 -and [double]$wireTraceSummary.prefix_preserved_rate -eq 0.0) "tool-choice transition was incorrectly reported as full prefix preservation"
Assert-True ([int]$wireTraceSummary.zero_cache_hit_count -eq 1 -and [int]$wireTraceSummary.cache_warmup_candidate_count -eq 1 -and [int]$wireTraceSummary.same_shape_zero_hit_count -eq 0) "cache warmup classification was not aggregated"
Assert-True ([int]$wireTraceSummary.tool_choice_transition_count -eq 1 -and [int]$wireTraceSummary.cache_shape_transition_count -eq 1) "cache-shape transition was not aggregated"
Assert-True ($wireTraceEvents.Count -eq 2 -and [string]$wireTraceEvents[1].first_diff_path -eq "tool_choice" -and [bool]$wireTraceEvents[1].message_prefix_preserved -and -not [bool]$wireTraceEvents[1].prefix_preserved) "provider final-wire request shape fields were not preserved"

$aggregateCacheRoot = Join-Path $RunRoot "aggregate-cache-root"
$leftArtifacts = Join-Path $aggregateCacheRoot "pair-001\left\artifacts"
$rightArtifacts = Join-Path $aggregateCacheRoot "pair-001\right\artifacts"
New-Item -ItemType Directory -Path $leftArtifacts, $rightArtifacts -Force | Out-Null
([pscustomobject]@{ logical_mode = "standard"; model_request_count = 1 }) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $leftArtifacts "metrics.json") -Encoding UTF8
([pscustomobject]@{ logical_mode = "taskspace"; model_request_count = 2 }) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $rightArtifacts "metrics.json") -Encoding UTF8
([pscustomobject]@{
    schema_version = "TaskSpaceProviderCacheTraceSummaryV3"
    provider_request_count = 1
    trace_coverage = 1.0
    cache_usage_missing_count = 0
    request_shape_counts = [pscustomobject]@{ native_tools_schema_hot_path = 1 }
    native_tools_schema_hot_path_count = 1
    tool_free_action_contract_count = 0
    unknown_or_unclassified_count = 0
    request_2_plus_count = 0
    request_2_plus_cached_input_tokens = 0
    request_2_plus_uncached_input_tokens = 0
    request_2_plus_hit_rate = $null
}) | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $leftArtifacts "provider-cache-trace-summary.json") -Encoding UTF8
([pscustomobject]@{
    schema_version = "TaskSpaceProviderCacheTraceSummaryV3"
    provider_request_count = 2
    trace_coverage = 1.0
    cache_usage_missing_count = 0
    request_shape_counts = [pscustomobject]@{ tool_free_action_contract = 2 }
    native_tools_schema_hot_path_count = 0
    tool_free_action_contract_count = 2
    unknown_or_unclassified_count = 0
    request_2_plus_count = 1
    request_2_plus_cached_input_tokens = 950
    request_2_plus_uncached_input_tokens = 50
    request_2_plus_hit_rate = 0.95
    zero_cache_hit_count = 1
    cache_warmup_candidate_count = 1
    same_shape_zero_hit_count = 0
    tool_choice_transition_count = 1
    cache_shape_transition_count = 1
}) | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $rightArtifacts "provider-cache-trace-summary.json") -Encoding UTF8
([pscustomobject]@{ schema_version = "TaskSpaceProviderCacheTraceV1"; request_id = "left-1"; request_shape_classifier = "native_tools_schema_hot_path" } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $leftArtifacts "provider-cache-trace.jsonl") -Encoding UTF8
([pscustomobject]@{ schema_version = "TaskSpaceProviderCacheTraceV1"; request_id = "right-1"; request_shape_classifier = "tool_free_action_contract" } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $rightArtifacts "provider-cache-trace.jsonl") -Encoding UTF8
$aggregateCache = Write-TaskspaceCostAggregateArtifacts -RootDir $aggregateCacheRoot -Scope "sample"
$aggregateCacheSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $aggregateCacheRoot "provider-cache-trace-summary.json") | ConvertFrom-Json
$aggregateCacheEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $aggregateCacheRoot "provider-cache-trace.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
Assert-True ([int]$aggregateCacheSummary.provider_request_count -eq 2) "aggregate provider cache trace should include only taskspace/right artifacts"
Assert-True ([int]$aggregateCacheSummary.native_tools_schema_hot_path_count -eq 0) "aggregate provider cache trace included standard native tools hot path"
Assert-True ([int]$aggregateCacheSummary.tool_free_action_contract_count -eq 2 -and [double]$aggregateCacheSummary.request_2_plus_hit_rate -eq 0.95) "aggregate provider cache trace did not preserve taskspace cache metrics"
Assert-True ([string]$aggregateCacheSummary.schema_version -eq "TaskSpaceProviderCacheTraceSummaryV3" -and [int]$aggregateCacheSummary.cache_warmup_candidate_count -eq 1 -and [int]$aggregateCacheSummary.tool_choice_transition_count -eq 1) "aggregate provider cache trace omitted cache-shape diagnostics"
Assert-True ($aggregateCacheEvents.Count -eq 1 -and [string]$aggregateCacheEvents[0].request_id -eq "right-1") "aggregate provider cache trace events did not filter to taskspace/right artifacts"
Assert-True ([string]$aggregateCache.provider_cache_trace_summary_path -eq (Join-Path $aggregateCacheRoot "provider-cache-trace-summary.json")) "aggregate return object omitted provider cache trace summary path"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "cost instrumentation selftest passed"
