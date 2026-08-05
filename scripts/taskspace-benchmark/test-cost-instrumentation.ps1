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
                "projection_is_message_tail:true",
                "large_raw_output_tokens:0",
                "runtime_boundary_forbidden_markers:none",
                "protected_items_present:true",
                "projection_kind:current_projection",
                "projection_map_id_sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "projection_revision:4",
                "projection_canonical_sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "projection_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "projection_policy:map-always",
                "expected_projection_kind:current_projection",
                "expected_projection_map_id_sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expected_projection_revision:4",
                "expected_projection_canonical_sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "expected_projection_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "projection_identity_confirmed:true",
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
                "negative_checks_performed:active_projection_uniqueness,large_raw_output,runtime_boundary_forbidden_markers",
                "projection_required:true",
                "active_projection_present:true",
                "active_projection_count:1",
                "projection_is_message_tail:true",
                "large_raw_output_tokens:0",
                "runtime_boundary_forbidden_markers:none",
                "protected_items_present:true",
                "projection_kind:current_projection",
                "projection_map_id_sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "projection_revision:4",
                "projection_canonical_sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "projection_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "projection_policy:map-always",
                "expected_projection_kind:current_projection",
                "expected_projection_map_id_sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expected_projection_revision:4",
                "expected_projection_canonical_sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "expected_projection_sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "projection_identity_confirmed:true",
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
Assert-True ([string]$scanEvents[0].runtime_boundary_forbidden_markers -eq "none") "exact payload scan should preserve boundary marker proof"
Assert-True ([int]$scanEvents[0].active_projection_count -eq 1) "exact payload scan should preserve active projection count"
Assert-True ([bool]$scanEvents[0].projection_required) "exact payload scan should preserve projection requirement"
Assert-True ([bool]$scanEvents[0].projection_identity_confirmed -and [string]$scanEvents[0].projection_policy -eq "map-always" -and [int64]$scanEvents[0].projection_revision -eq 4 -and [int64]$scanEvents[0].expected_projection_revision -eq 4 -and [string]$scanEvents[0].projection_canonical_sha256 -eq [string]$scanEvents[0].expected_projection_canonical_sha256) "exact payload scan should preserve policy-aware projection identity proof"
Assert-True ($providerEvents.Count -eq 3 -and [string]$providerEvents[0].schema_version -eq "taskspace-provider-request-budget-event-v1") "provider request events were not derived from runtime budget trace"
Assert-True (@($providerEvents | Where-Object { [string]$_.producer -eq "provider_lifecycle" }).Count -eq 3) "provider request events did not preserve provider_lifecycle producer"
Assert-True ([string]$providerEvents[0].provider_wire_api -eq "ChatCompletions" -and [int]$providerEvents[0].tools_count -eq 24 -and [string]$providerEvents[0].request_shape_classifier -eq "native_tools_schema_hot_path") "provider request events did not preserve cache request shape fields"
Assert-True ([int64]$providerEvents[0].model_request_duration_ms -eq 615 -and [int64]$providerEvents[0].latency_ms -eq 615 -and [int64]$providerEvents[0].started_at_ms -eq 100 -and [int64]$providerEvents[0].completed_at_ms -eq 715) "provider request events did not preserve provider lifecycle timing fields"
Assert-True ([bool]$providerEvents[0].request_reason_schema_present -and [string]$providerEvents[0].trigger_kind -eq "model_sampling" -and [string]$providerEvents[0].request_reason_delta -eq "initial_request") "provider request events did not preserve request reason fields"
Assert-True ([string]$providerEvents[1].response_actionability_previous -eq "tool_feedback_recovery" -and [string]$providerEvents[1].latest_tool_result_refs -eq "result-1" -and [string]$providerEvents[1].model_visible_feedback_refs -eq "result-1|trace-response-1") "provider request events did not preserve feedback refs"
Assert-True ([int]$providerEvents[2].repeated_same_reason_count -eq 1 -and [string]$providerEvents[2].request_reason_delta -eq "none") "provider request events did not preserve repeated same-reason detector fields"
Assert-True ($cacheTraceEvents.Count -eq 0 -and [string]$cacheTraceSummary.source -eq "request_facts_without_wire_shape") "budget events without canonical request identity should not synthesize cache requests"
Assert-True ([int]$cacheTraceSummary.provider_request_count -eq 0 -and [double]$cacheTraceSummary.trace_coverage -eq 0.0) "cache trace without canonical request facts should be explicitly unavailable"
Assert-True ([bool]$replacement.exact_payload_scan_passed -and [bool]$replacement.replacement_confirmed) "active replacement report did not use exact payload scan"
Assert-True ([int]$replacement.active_projection_count_max -eq 1 -and [int]$replacement.active_projection_uniqueness_violation_count -eq 0) "active replacement report did not enforce projection uniqueness"
Assert-True ([string]$replacement.runtime_boundary_forbidden_markers -eq "none") "active replacement report did not preserve boundary marker proof"
Assert-True ([bool]$replacement.protected_items_present -and [bool]$replacement.exact_payload_scan_matching_provider_event) "active replacement report did not preserve exact scan join evidence"
Assert-True ([bool]$replacement.projection_identity_confirmed -and [string]$replacement.projection_policy -eq "map-always" -and [int]$replacement.projection_identity_unconfirmed_count -eq 0 -and [int64]$replacement.projection_revision -eq 4) "active replacement report did not require policy-aware projection identity"
$repeatedLifecycleReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($scanEvents[0], $scanEvents[0])
Assert-True ([int]$repeatedLifecycleReplacement.active_context_replacement_report.matching_payload_scan_count -eq 1) "repeated lifecycle scans should be deduplicated by scan event id"
$budgetOnlyReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @()
Assert-True (-not [bool]$budgetOnlyReplacement.active_context_replacement_report.exact_payload_scan_passed -and @($budgetOnlyReplacement.exact_payload_scan_events).Count -eq 0) "budget-only payload booleans should not synthesize exact scan evidence"
$blankBootstrapScan = [pscustomobject]@{
    scan_event_id = "scan-provider-request-blank"
    request_id = [string]$scanEvents[0].request_id
    provider_payload_sha256 = [string]$scanEvents[0].provider_payload_sha256
    producer = "provider_payload_scanner"
    matching_provider_event = $true
    projection_required = $false
    passed = $true
    replacement_confirmed = $true
    active_projection_count = 0
    large_raw_output_tokens = 0
    runtime_boundary_forbidden_markers = "none"
    protected_items_present = $false
}
$mixedBootstrapReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($blankBootstrapScan, $scanEvents[0])
Assert-True ([bool]$mixedBootstrapReplacement.active_context_replacement_report.replacement_confirmed -and [int]$mixedBootstrapReplacement.active_context_replacement_report.active_projection_uniqueness_violation_count -eq 0) "blank bootstrap should not count as active projection uniqueness violation"
$duplicateProjectionScan = [pscustomobject]@{
    scan_event_id = "scan-provider-request-duplicate"
    request_id = [string]$scanEvents[0].request_id
    provider_payload_sha256 = [string]$scanEvents[0].provider_payload_sha256
    producer = "provider_payload_scanner"
    matching_provider_event = $true
    projection_required = $true
    passed = $false
    replacement_confirmed = $false
    active_projection_count = 2
    projection_is_message_tail = $true
    large_raw_output_tokens = 0
    runtime_boundary_forbidden_markers = "none"
    protected_items_present = $true
}
$duplicateProjectionReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($duplicateProjectionScan)
Assert-True (-not [bool]$duplicateProjectionReplacement.active_context_replacement_report.replacement_confirmed) "duplicate active projection should not confirm replacement"
Assert-True ([int]$duplicateProjectionReplacement.active_context_replacement_report.active_projection_uniqueness_violation_count -eq 1) "duplicate active projection violation was not reported"
$appendIdentity = ConvertTo-TaskspaceProjectionIdentity ([pscustomobject]@{
    count = 3
    kind = "request_snapshot"
    map_id_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    revision = 4
    canonical_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    projection_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    unavailable_reason = $null
})
Assert-True ([string]$appendIdentity.kind -eq "active" -and [int]$appendIdentity.count -eq 3 -and [int64]$appendIdentity.revision -eq 4) "map-append latest projection identity was not normalized"
$appendProjectionScan = [pscustomobject]@{
    scan_event_id = "scan-provider-request-append"
    request_id = [string]$scanEvents[0].request_id
    provider_payload_sha256 = [string]$scanEvents[0].provider_payload_sha256
    producer = "provider_payload_scanner"
    matching_provider_event = $true
    projection_required = $true
    passed = $true
    replacement_confirmed = $true
    projection_identity_confirmed = $true
    projection_policy = "map-append"
    active_projection_count = 3
    projection_is_message_tail = $true
    large_raw_output_tokens = 0
    runtime_boundary_forbidden_markers = "none"
    protected_items_present = $true
}
$appendProjectionReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($appendProjectionScan)
Assert-True ([bool]$appendProjectionReplacement.active_context_replacement_report.replacement_confirmed) "valid map-append history should confirm latest projection"
Assert-True ([int]$appendProjectionReplacement.active_context_replacement_report.active_projection_count_max -eq 3 -and [int]$appendProjectionReplacement.active_context_replacement_report.active_projection_uniqueness_violation_count -eq 0) "map-append history should not be treated as a projection uniqueness violation"
Assert-True ([bool]$appendProjectionReplacement.active_context_replacement_report.projection_is_message_tail -and [int]$appendProjectionReplacement.active_context_replacement_report.projection_message_tail_violation_count -eq 0) "map-append history did not preserve request-tail projection evidence"
$tailViolationScan = $appendProjectionScan.PSObject.Copy()
$tailViolationScan.scan_event_id = "scan-provider-request-tail-violation"
$tailViolationScan.passed = $false
$tailViolationScan.replacement_confirmed = $false
$tailViolationScan.projection_is_message_tail = $false
$tailViolationReplacement = New-TaskspaceActiveReplacementArtifacts $budgetEvents @($tailViolationScan)
Assert-True (-not [bool]$tailViolationReplacement.active_context_replacement_report.replacement_confirmed -and [int]$tailViolationReplacement.active_context_replacement_report.projection_message_tail_violation_count -eq 1) "request-tail projection violation was not reported"
$appendProjectionEvent = New-TaskspaceContextProjectionEvent @"
TaskSpaceMapProjectionR7V1:
- schema_version: taskspace-map-projection-r7-v1
- projection_kind: request_snapshot
- supersedes_all_prior_projections: true
- current_state_rule: last_projection_only
- map_id: map-1
- revision: 4
- canonical_sha256: cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
- root_node_id: root
- finish_node_id: finish
- complete: false
- root_source_event_ids: none
- current_terminal: none
- terminal_history: none
- active_frontier: work
- map_nodes: root|work|finish
- map_edges: root->work|work->finish
- node_details: none
TaskSpaceMapProjectionR7V1 end.
"@
Assert-True ([string]$appendProjectionEvent.projection_kind -eq "request_snapshot" -and [int]$appendProjectionEvent.protected_miss_count -eq 0) "model-visible map-append projection envelope was not preserved"
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
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"execute\",\"expected_revision\":4,\"mutations\":[],\"actions\":[{\"node_id\":\"work\",\"tool\":\"exec_command\"}]}","call_id":"native-control-1"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"execute\",\"expected_revision\":4,\"mutations\":[],\"actions\":[{\"node_id\":\"work\",\"tool\":\"apply_patch\"}]}","call_id":"native-control-2"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"initialize_and_execute\",\"root\":{\"node_id\":\"root\",\"goal\":\"solve\"},\"work_nodes\":[{\"node_id\":\"work\",\"goal\":\"work\"}],\"finish\":{\"node_id\":\"finish\",\"goal\":\"finish\"},\"edges\":[],\"actions\":[{\"node_id\":\"work\",\"tool\":\"exec_command\"}]}","call_id":"native-control-3"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"initialize_and_execute\",\"root\":{\"node_id\":\"root\",\"goal\":\"solve\"},\"work_nodes\":[{\"node_id\":\"work\",\"goal\":\"work\"}],\"finish\":{\"node_id\":\"finish\",\"goal\":\"finish\"},\"edges\":[{\"from\":\"root\",\"to\":\"work\"},{\"from\":\"work\",\"to\":\"finish\"}],\"actions\":[{\"node_id\":\"work\",\"tool\":\"exec_command\"}]}","call_id":"native-control-4"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"initialize_and_execute\",\"root\":{\"node_id\":\"root\",\"goal\":\"solve\"},\"work_nodes\":[{\"node_id\":\"work\",\"goal\":\"work\"}],\"finish\":{\"node_id\":\"finish\",\"goal\":\"finish\"},\"edges\":[{\"from\":\"root\",\"to\":\"work\"},{\"from\":\"work\",\"to\":\"finish\"}],\"actions\":[{\"node_id\":\"work\",\"tool\":\"exec_command\"}]}","call_id":"native-control-5"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"read_map\"}","call_id":"native-read-map-1"}}',
    '{"type":"response_item","payload":{"type":"function_call","name":"taskspace_control","arguments":"{\"action\":\"read_map\"}","call_id":"native-read-map-2"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-1","output":"{\"schema_version\":\"ToolSequencePreflightResultV3\",\"status\":\"protocol_failed\",\"success\":false,\"error\":{\"class\":\"protocol\",\"code\":\"taskspace_action_tool_mismatch\",\"message\":\"missing paired action\"},\"request\":{\"executed_tool_call_count\":0}}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-2","output":"{\"schema_version\":\"TaskSpaceResponseCommitFailureV3\",\"action\":\"execute\",\"success\":false,\"status\":\"state_rejected\",\"state_commit\":false,\"canonical_revision\":4,\"error\":{\"class\":\"state_machine\",\"code\":\"taskspace_response_state_commit_failed\",\"detail\":\"invalid\"}}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-3","output":"{\"schema_version\":\"TaskSpaceControlResultV2\",\"action\":\"initialize_and_execute\",\"success\":false,\"status\":\"argument_failed\",\"state_commit\":false,\"error\":{\"class\":\"argument\",\"code\":\"TASKSPACE_INVALID_ARGUMENT\",\"message\":\"invalid\"}}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-4","output":"{\"schema_version\":\"TaskSpaceResponseResultV2\",\"action\":\"initialize_and_execute\",\"success\":true,\"status\":\"settled\",\"state_commit\":true,\"canonical_revision\":6,\"settlement\":{\"prepared_action_count\":1,\"attributed_result_count\":1,\"outstanding_reservation_count\":0}}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-control-5","output":"{\"schema_version\":\"ToolSequencePreflightResultV3\",\"status\":\"protocol_failed\",\"success\":false,\"state_commit\":false,\"error\":{\"class\":\"protocol\",\"code\":\"taskspace_action_tool_mismatch\",\"message\":\"TaskSpace action tool did not match sibling Tool\"},\"request\":{\"executed_tool_call_count\":0}}"}}',
    '{"type":"event_msg","payload":{"type":"map_runtime","map_event_type":"store_committed","mapId":"map-1","mapRevision":4}}',
    '{"type":"event_msg","payload":{"type":"map_runtime","map_event_type":"store_committed","mapId":"map-1","mapRevision":5}}',
    '{"type":"event_msg","payload":{"type":"map_runtime","map_event_type":"store_committed","mapId":"map-1","mapRevision":6}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-read-map-1","output":"{\"schema_version\":\"TaskSpaceControlResultV2\",\"action\":\"read_map\",\"success\":true,\"status\":\"read_ok\",\"state_commit\":false,\"read\":{\"kind\":\"map_projection\",\"revision\":6,\"content\":\"TaskSpaceMapProjectionR7V1:\\n- revision: 6\\nTaskSpaceMapProjectionR7V1 end.\"}}"}}',
    '{"type":"response_item","payload":{"type":"function_call_output","call_id":"native-read-map-2","output":"{\"schema_version\":\"TaskSpaceControlResultV2\",\"action\":\"read_map\",\"success\":true,\"status\":\"read_ok\",\"state_commit\":false,\"read\":{\"kind\":\"map_projection\",\"revision\":6,\"content\":\"TaskSpaceMapProjectionR7V1:\\n- revision: 6\\nTaskSpaceMapProjectionR7V1 end.\"}}"}}'
) | Set-Content -LiteralPath (Join-Path $rolloutControlDir "rollout.jsonl") -Encoding UTF8
$rolloutControlInstrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $rolloutControlDir -JsonlPath $rolloutControlJsonl -ObservabilityJsonPath ""
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count -eq 7) "rollout taskspace_control calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.native_taskspace_control_count -eq 7) "rollout native taskspace_control calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_contract_taskspace_control_count -eq 0) "obsolete action-contract calls were counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_counts.execute -eq 2) "execute actions were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_counts.initialize_and_execute -eq 3) "map initialization attempts were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_counts.read_map -eq 2) "explicit map reads were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.action_manifest_count -eq 5) "action manifests were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.declared_action_count -eq 5) "declared actions were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.initialize_and_execute_count -eq 3) "initializations were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.committed_initialize_and_execute_count -eq 1) "committed initialization was not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.failed_initialize_and_execute_count -eq 2) "failed initialization was not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.sequence_preflight_rejected_call_count -eq 2) "sequence preflight rejected calls were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_failure_count -eq 4) "rollout control failures were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_protocol_failure_count -eq 2) "protocol control failures were not classified"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_preflight_failure_count -eq 2) "control preflight failures were not separated"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_handler_failure_count -eq 2) "control handler failures were not separated"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_state_failure_count -eq 1) "state control failure was not classified"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.control_argument_failure_count -eq 1) "argument control failure was not classified"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.nested_action_failure_count -eq 0) "removed nested action contract reappeared"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.ordinary_gate_failure_count -eq 0) "ordinary-tool gate logic reappeared"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.committed_control_count -eq 1) "committed final control result was not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.state_commit_count -eq 1) "state_commit_count does not reflect the current final control result schema"
Assert-True ([string]$rolloutControlInstrumentation.taskspace_control_usage.state_commit_count_source -eq "control_result_state_commit") "state commit source is ambiguous"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.graph_revision_commit_count -eq 3) "raw graph revision commits were not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.response_final_control_result_count -eq 1) "response-final control result was not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.response_final_control_result_settled_count -eq 1) "settled response-final control result was not counted"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.response_final_control_result_incomplete_count -eq 0) "settled control result was classified as incomplete"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.latest_response_final_revision -eq 6) "response-final canonical revision was not retained"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_request_count -eq 2) "map read requests were not measured"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_completion_count -eq 2) "map read completions were not measured"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_failure_count -eq 0) "successful map reads were classified as failures"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_repeated_revision_count -eq 1) "repeated map revision was not measured"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_revision_lag_sample_count -eq 2) "map read revision lag coverage was not measured"
Assert-True ([double]$rolloutControlInstrumentation.taskspace_control_usage.read_map_revision_lag_mean -eq 0.0) "current map reads reported revision lag"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_revision_lag_max -eq 0) "current map reads reported maximum revision lag"
Assert-True ([int]$rolloutControlInstrumentation.taskspace_control_usage.read_map_stale_revision_error_count -eq 0) "read_map should not require an expected revision"
Assert-True ([string]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count_source -eq "rollout_trace") "rollout trace should be the authoritative control-count source"
Assert-True (-not [bool]$rolloutControlInstrumentation.taskspace_control_usage.taskspace_control_count_source_mismatch) "an exec file without response-item telemetry should not be treated as a conflicting control count"

$wireTraceDir = Join-Path $RunRoot "provider-wire-trace-artifacts"
New-Item -ItemType Directory -Path $wireTraceDir -Force | Out-Null
$wireTraceJsonl = Join-Path $RunRoot "provider-wire-trace-whale-exec.jsonl"
'{"type":"turn.completed","usage":{"input_tokens":300,"cached_input_tokens":200,"output_tokens":20}}' | Set-Content -LiteralPath $wireTraceJsonl -Encoding UTF8
@(
    '{"schema_version":"provider-chat-wire-trace-v10","event_name":"provider.chat_wire_shape_recorded","request_id":"wire-1","logical_request_id":"wire-logical-1","attempt_seq":1,"epoch_id":"epoch-1","request_index":1,"provider_wire_api":"ChatCompletions","pre_wire_payload_sha256":"pre-1","provider_payload_sha256":"0000000000000000000000000000000000000000000000000000000000000001","provider_payload_bytes":100,"messages_hash":"messages-1","tools_hash":"tools-1","cache_shape_hash":"shape-named","tools_count":2,"tool_choice_kind":"named_function","tool_choice_name":"taskspace_control","message_count":2,"message_shapes":[{"index":0,"role":"system","bytes":40,"message_sha256":"m0","content_sha256":"c0"},{"index":1,"role":"user","bytes":20,"message_sha256":"m1","content_sha256":"c1"}],"previous_request_id":null,"lcp_message_count":0,"lcp_message_bytes":0,"message_prefix_preserved":null,"tool_choice_preserved":null,"tool_choice_changed":null,"prefix_preserved":null,"first_diff_index":null,"first_diff_path":null,"status":"payload_captured"}',
    '{"schema_version":"provider-chat-wire-trace-v10","event_name":"provider.chat_wire_request_terminal","request_id":"wire-1","logical_request_id":"wire-logical-1","attempt_seq":1,"status":"response_completed","input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":1,"total_tokens":110}',
    '{"schema_version":"provider-chat-wire-trace-v10","event_name":"provider.chat_wire_prefix_broken","request_id":"wire-2","logical_request_id":"wire-logical-2","attempt_seq":1,"epoch_id":"epoch-1","request_index":2,"provider_wire_api":"ChatCompletions","pre_wire_payload_sha256":"pre-2","provider_payload_sha256":"0000000000000000000000000000000000000000000000000000000000000002","provider_payload_bytes":140,"messages_hash":"messages-2","tools_hash":"tools-1","cache_shape_hash":"shape-auto","tools_count":2,"tool_choice_kind":"auto","tool_choice_name":null,"message_count":3,"message_shapes":[{"index":0,"role":"system","bytes":40,"message_sha256":"m0","content_sha256":"c0"},{"index":1,"role":"user","bytes":20,"message_sha256":"m1","content_sha256":"c1"},{"index":2,"role":"assistant","bytes":30,"message_sha256":"m2","content_sha256":"c2"}],"previous_request_id":"wire-1","lcp_message_count":2,"lcp_message_bytes":60,"message_prefix_preserved":true,"tool_choice_preserved":false,"tool_choice_changed":true,"prefix_preserved":false,"first_diff_index":null,"first_diff_path":"tool_choice","status":"payload_captured"}',
    '{"schema_version":"provider-chat-wire-trace-v10","event_name":"provider.chat_wire_request_terminal","request_id":"wire-2","logical_request_id":"wire-logical-2","attempt_seq":1,"status":"response_completed","input_tokens":200,"cached_input_tokens":180,"output_tokens":10,"reasoning_output_tokens":1,"total_tokens":210}'
) | Set-Content -LiteralPath (Join-Path $wireTraceDir "provider-wire-trace.jsonl") -Encoding UTF8
$wireTraceInstrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $wireTraceDir -JsonlPath $wireTraceJsonl -ObservabilityJsonPath ""
$wireTraceSummary = $wireTraceInstrumentation.provider_cache_trace_summary
$wireTraceEvents = @($wireTraceInstrumentation.provider_cache_trace_events)
Assert-True ([string]$wireTraceSummary.schema_version -eq "TaskSpaceProviderCacheTraceSummaryV4" -and [string]$wireTraceSummary.source -eq "provider_final_wire_trace") "provider final-wire trace was not selected as cache source"
Assert-True ($null -eq $wireTraceSummary.provider_request_count -and [string]$wireTraceSummary.provider_request_source -eq "request_facts_unavailable") "provider request count was inferred without boundary evidence"
Assert-True ([int]$wireTraceSummary.provider_attempt_count -eq 2 -and [int]$wireTraceSummary.shape_observation_count -eq 2 -and [double]$wireTraceSummary.trace_coverage -eq 1.0) "provider attempt trace coverage was not complete"
Assert-True ([double]$wireTraceSummary.request_2_plus_hit_rate -eq 0.9) "provider final-wire request-2+ cache usage was not aggregated"
Assert-True ([int]$wireTraceSummary.prefix_preserved_count -eq 0 -and [double]$wireTraceSummary.prefix_preserved_rate -eq 0.0) "tool-choice transition was incorrectly reported as full prefix preservation"
Assert-True ([int]$wireTraceSummary.zero_cache_hit_count -eq 1 -and [int]$wireTraceSummary.cache_warmup_candidate_count -eq 1 -and [int]$wireTraceSummary.same_shape_zero_hit_count -eq 0) "cache warmup classification was not aggregated"
Assert-True ([int]$wireTraceSummary.tool_choice_transition_count -eq 1 -and [int]$wireTraceSummary.cache_shape_transition_count -eq 1) "cache-shape transition was not aggregated"
Assert-True ($wireTraceEvents.Count -eq 2 -and [string]$wireTraceEvents[1].first_diff_path -eq "tool_choice" -and [bool]$wireTraceEvents[1].message_prefix_preserved -and -not [bool]$wireTraceEvents[1].prefix_preserved) "provider final-wire request shape fields were not preserved"
Assert-True ([string]$wireTraceSummary.section_cost_summary.availability -eq "unavailable" -and [int]$wireTraceSummary.section_cost_summary.unavailable_request_count -eq 2) "provider wire without section cost section cost should be explicitly unavailable"
Assert-True ([int]$wireTraceSummary.section_cost_summary.unavailable_reason_counts.provider_wire_v3_section_cost_missing -eq 2 -and $null -eq $wireTraceSummary.section_cost_summary.section_bytes_total) "provider wire without section cost section cost was reported as zero"
Assert-True (@($wireTraceEvents | Where-Object { $_.section_cost.availability -eq "unavailable" -and $_.section_cost.unavailable_reason -eq "provider_wire_v3_section_cost_missing" }).Count -eq 2) "provider wire without section cost cache events omitted section-cost provenance"
Assert-True ([int]$wireTraceSummary.section_cost_summary.active_projection_identity_summary.unavailable_count -eq 2 -and [int]$wireTraceSummary.section_cost_summary.active_projection_identity_summary.unique_projection_sha256_count -eq 0) "provider wire without section cost projection identity should be explicitly unavailable"

$conflictFacts = Invoke-TaskspaceRequestFactsGenerator -WireTracePath (Join-Path $wireTraceDir "provider-wire-trace.jsonl")
$conflictFacts.availability.usage = "incomparable"
$conflictFacts.findings = @([pscustomobject]@{ code = "usage_source_conflict"; source = "reconcile" })
$conflictSummary = (New-TaskspaceProviderWireCacheTraceArtifacts `
        (Join-Path $wireTraceDir "provider-wire-trace.jsonl") $conflictFacts).provider_cache_trace_summary
Assert-True (-not [bool]$conflictSummary.comparison_eligible -and $null -eq $conflictSummary.request_2_plus_hit_rate) "incomparable canonical usage still produced a cache rate"
Assert-True ($null -eq $conflictSummary.request_2_plus_cached_input_tokens -and $null -eq $conflictSummary.zero_cache_hit_count) "incomparable canonical usage still produced precise cache totals"
Assert-True (@($conflictSummary.request_facts_findings) -contains "usage_source_conflict") "cache summary omitted canonical conflict findings"

$v3WireTraceDir = Join-Path $RunRoot "provider-wire-trace-v3-artifacts"
New-Item -ItemType Directory -Path $v3WireTraceDir -Force | Out-Null
$v3WireTraceJsonl = Join-Path $RunRoot "provider-wire-trace-v3-whale-exec.jsonl"
'{"type":"turn.completed","usage":{"input_tokens":500,"cached_input_tokens":0,"output_tokens":20}}' | Set-Content -LiteralPath $v3WireTraceJsonl -Encoding UTF8
$v3Sections = @(
    [pscustomobject]@{ kind = "system_messages"; count = 1; bytes = 100; estimated_tokens = 25; sha256 = "system-hash" },
    [pscustomobject]@{ kind = "natural_history"; count = 3; bytes = 200; estimated_tokens = 50; sha256 = "history-hash" },
    [pscustomobject]@{ kind = "active_projection"; count = 1; bytes = 30; estimated_tokens = 8; sha256 = "projection-hash" },
    [pscustomobject]@{ kind = "taskspace_control_feedback"; count = 1; bytes = 20; estimated_tokens = 5; sha256 = "control-hash" },
    [pscustomobject]@{ kind = "ordinary_tool_feedback"; count = 2; bytes = 40; estimated_tokens = 10; sha256 = "feedback-hash" },
    [pscustomobject]@{ kind = "tools"; count = 2; bytes = 50; estimated_tokens = 13; sha256 = "tools-hash" },
    [pscustomobject]@{ kind = "tool_choice"; count = 1; bytes = 10; estimated_tokens = 3; sha256 = "choice-hash" },
    [pscustomobject]@{ kind = "other_payload"; count = 2; bytes = 50; estimated_tokens = 12; sha256 = "other-hash" }
)
$v3Shape = [pscustomobject]@{
    schema_version = "provider-chat-wire-trace-v10"; event_name = "provider.chat_wire_shape_recorded"
    request_id = "wire-v3-1"; request_index = 1; provider_wire_api = "ChatCompletions"; status = "payload_captured"
    provider_payload_sha256 = "wire-v3-hash"; provider_payload_bytes = 500; cache_shape_hash = "wire-v3-shape"
    messages_hash = "wire-v3-messages"; tools_hash = "wire-v3-tools"; tools_count = 2; message_count = 4
    section_cost = [pscustomobject]@{
        schema_version = "provider-wire-section-cost-v1"; availability = "measured"; unavailable_reason = $null
        section_bytes_total = 500
        active_projection_identity = [pscustomobject]@{
            count = 1; kind = "current_projection"
            map_id_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            revision = 7
            canonical_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            projection_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            unavailable_reason = $null
        }
        sections = $v3Sections
    }
}
@(
    $v3Shape,
    [pscustomobject]@{ schema_version = "provider-chat-wire-trace-v10"; event_name = "provider.chat_wire_request_terminal"; request_id = "wire-v3-1"; status = "response_completed"; input_tokens = 500; cached_input_tokens = 0; output_tokens = 20 }
) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 12 } | Set-Content -LiteralPath (Join-Path $v3WireTraceDir "provider-wire-trace.jsonl") -Encoding UTF8
$v3Instrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $v3WireTraceDir -JsonlPath $v3WireTraceJsonl -ObservabilityJsonPath ""
$v3Summary = $v3Instrumentation.provider_cache_trace_summary.section_cost_summary
$v3Event = @($v3Instrumentation.provider_cache_trace_events)[0]
$v3SectionBytes = [int64](@($v3Summary.sections | Measure-Object -Property bytes -Sum).Sum)
$v3ProjectionSection = @($v3Summary.sections | Where-Object { $_.kind -eq "active_projection" })[0]
Assert-True ([string]$v3Event.section_cost.availability -eq "measured" -and @($v3Event.section_cost.sections).Count -eq 8) "measured v3 cache event omitted section cost"
Assert-True ([int]$v3Summary.measured_request_count -eq 1 -and [int]$v3Summary.unavailable_request_count -eq 0 -and [int64]$v3Summary.section_bytes_total -eq 500) "measured v3 section summary coverage is incorrect"
Assert-True ($v3SectionBytes -eq [int64]$v3Summary.section_bytes_total -and $v3SectionBytes -eq [int64]$v3Event.provider_payload_bytes -and [int64]$v3Summary.estimated_tokens_total -eq 126) "v3 section summary did not reconcile exact payload/section/token totals"
Assert-True ([string]$v3Event.section_cost.active_projection_identity.kind -eq "active" -and [int64]$v3Event.section_cost.active_projection_identity.revision -eq 7 -and [string]$v3Event.section_cost.active_projection_identity.canonical_sha256 -eq "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc") "v3 cache event lost current R7 projection identity"
Assert-True ([int]$v3Summary.active_projection_identity_summary.active_count -eq 1 -and [int]$v3Summary.active_projection_identity_summary.unique_revision_count -eq 1 -and [int]$v3Summary.active_projection_identity_summary.unique_projection_sha256_count -eq 1) "v3 section summary lost projection freshness evidence"
Assert-True ([int]$v3ProjectionSection.request_sample_count -eq 1 -and [double]$v3ProjectionSection.bytes_per_request_mean -eq 30 -and [double]$v3ProjectionSection.bytes_per_request_median -eq 30) "v3 section summary omitted per-request distribution statistics"
$v5WireTraceDir = Join-Path $RunRoot "provider-wire-trace-v5-artifacts"
New-Item -ItemType Directory -Path $v5WireTraceDir -Force | Out-Null
$v5Shape = $v3Shape | ConvertTo-Json -Depth 12 | ConvertFrom-Json
$v5Shape.schema_version = "provider-chat-wire-trace-v10"
$v5Shape.request_id = "wire-v5-1"
$v5Shape | Add-Member -NotePropertyName base_instructions_identity -NotePropertyValue ([pscustomobject]@{
    count = 1; message_index = 0; wire_role = "system"; message_bytes = 21727; estimated_tokens = 5432
    profile = "taskspace"; version = "1.0.1"
    sha256 = "0cea4c521de4659b43b29e9ada83f836f84d92f5ae88e301f04860ec301106d2"
    matches_current_contract = $true; unavailable_reason = $null
})
@(
    $v5Shape,
    [pscustomobject]@{ schema_version = "provider-chat-wire-trace-v10"; event_name = "provider.chat_wire_request_terminal"; request_id = "wire-v5-1"; status = "response_completed"; input_tokens = 6000; cached_input_tokens = 0; output_tokens = 20 }
) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 12 } | Set-Content -LiteralPath (Join-Path $v5WireTraceDir "provider-wire-trace.jsonl") -Encoding UTF8
$v5Instrumentation = Write-TaskspaceCostInstrumentationArtifacts -ArtifactDir $v5WireTraceDir -JsonlPath $v3WireTraceJsonl -ObservabilityJsonPath ""
$v5BaseEvent = @($v5Instrumentation.provider_cache_trace_events)[0].base_instructions_identity
$v5BaseSummary = $v5Instrumentation.provider_cache_trace_summary.base_instructions_identity_summary
$v5SectionSummary = $v5Instrumentation.provider_cache_trace_summary.section_cost_summary
Assert-True ([string]$v5BaseEvent.profile -eq "taskspace" -and [string]$v5BaseEvent.version -eq "1.0.1" -and [string]$v5BaseEvent.wire_role -eq "system" -and [bool]$v5BaseEvent.matches_current_contract) "v5 cache event lost base instructions identity"
Assert-True ([int]$v5BaseSummary.present_count -eq 1 -and [int]$v5BaseSummary.current_contract_match_count -eq 1 -and [int64]$v5BaseSummary.estimated_tokens_total -eq 5432) "v5 base instructions summary is incorrect"
Assert-True ([string]$v5SectionSummary.availability -eq "measured" -and [int]$v5SectionSummary.measured_request_count -eq 1) "v5 section cost was rejected as an unsupported wire schema"
$mismatchShape = $v3Shape | ConvertTo-Json -Depth 12 | ConvertFrom-Json
$mismatchShape.section_cost.section_bytes_total = 499
$mismatchCost = ConvertTo-TaskspaceProviderSectionCost $mismatchShape
Assert-True ([string]$mismatchCost.availability -eq "unavailable" -and [string]$mismatchCost.unavailable_reason -eq "section_bytes_total_mismatch" -and $null -eq $mismatchCost.section_bytes_total) "section byte mismatch should not emit a measured zero or partial value"
$payloadMismatchShape = $v3Shape | ConvertTo-Json -Depth 12 | ConvertFrom-Json
$payloadMismatchShape.provider_payload_bytes = 501
$payloadMismatchCost = ConvertTo-TaskspaceProviderSectionCost $payloadMismatchShape
Assert-True ([string]$payloadMismatchCost.availability -eq "unavailable" -and [string]$payloadMismatchCost.unavailable_reason -eq "section_bytes_payload_mismatch") "section total must reconcile to exact provider payload bytes"

$aggregateCacheRoot = Join-Path $RunRoot "aggregate-cache-root"
$leftArtifacts = Join-Path $aggregateCacheRoot "pair-001\left\artifacts"
$rightArtifacts = Join-Path $aggregateCacheRoot "pair-001\right\artifacts"
New-Item -ItemType Directory -Path $leftArtifacts, $rightArtifacts -Force | Out-Null
([pscustomobject]@{ logical_mode = "standard"; model_request_count = 1 }) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $leftArtifacts "metrics.json") -Encoding UTF8
'{' | Set-Content -LiteralPath (Join-Path $rightArtifacts "metrics.json") -Encoding UTF8
([pscustomobject]@{ repeat = 1; left = "standard"; right = "taskspace" }) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $aggregateCacheRoot "pair-001/logical-mode-map.json") -Encoding UTF8
([pscustomobject]@{
    schema_version = "TaskSpaceProviderCacheTraceSummaryV4"
    provider_request_count = 1
    provider_attempt_count = 1
    comparison_eligible = $true
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
    schema_version = "TaskSpaceProviderCacheTraceSummaryV4"
    provider_request_count = 2
    provider_attempt_count = 2
    comparison_eligible = $true
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
    section_cost_summary = [pscustomobject]@{
        schema_version = "provider-wire-section-cost-summary-v1"
        availability = "measured"; request_count = 2; measured_request_count = 2; unavailable_request_count = 0
        unavailable_reason_counts = [pscustomobject]@{}; section_bytes_total = 300; estimated_tokens_total = 75
        active_projection_identity_summary = [pscustomobject]@{
            schema_version = "provider-wire-active-projection-identity-summary-v1"
            bootstrap_count = 1; active_count = 1; unavailable_count = 0
            unavailable_reason_counts = [pscustomobject]@{}
            projection_sha256_counts = [pscustomobject]@{
                'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc' = 1
                'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd' = 1
            }
            revision_counts = [pscustomobject]@{ '1' = 1 }
            unique_projection_sha256_count = 2; unique_revision_count = 1
        }
        sections = @(
            [pscustomobject]@{ kind = "system_messages"; count = 2; bytes = 100; estimated_tokens = 25; request_bytes = @(50, 50); request_estimated_tokens = @(12, 13) },
            [pscustomobject]@{ kind = "natural_history"; count = 2; bytes = 50; estimated_tokens = 12; request_bytes = @(25, 25); request_estimated_tokens = @(6, 6) },
            [pscustomobject]@{ kind = "active_projection"; count = 2; bytes = 40; estimated_tokens = 10; request_bytes = @(20, 20); request_estimated_tokens = @(5, 5) },
            [pscustomobject]@{ kind = "taskspace_control_feedback"; count = 2; bytes = 30; estimated_tokens = 8; request_bytes = @(15, 15); request_estimated_tokens = @(4, 4) },
            [pscustomobject]@{ kind = "ordinary_tool_feedback"; count = 2; bytes = 20; estimated_tokens = 5; request_bytes = @(10, 10); request_estimated_tokens = @(2, 3) },
            [pscustomobject]@{ kind = "tools"; count = 2; bytes = 30; estimated_tokens = 8; request_bytes = @(15, 15); request_estimated_tokens = @(4, 4) },
            [pscustomobject]@{ kind = "tool_choice"; count = 2; bytes = 10; estimated_tokens = 2; request_bytes = @(5, 5); request_estimated_tokens = @(1, 1) },
            [pscustomobject]@{ kind = "other_payload"; count = 2; bytes = 20; estimated_tokens = 5; request_bytes = @(10, 10); request_estimated_tokens = @(2, 3) }
        )
    }
}) | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $rightArtifacts "provider-cache-trace-summary.json") -Encoding UTF8
([pscustomobject]@{ schema_version = "TaskSpaceProviderCacheTraceV1"; request_id = "left-1"; request_shape_classifier = "native_tools_schema_hot_path" } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $leftArtifacts "provider-cache-trace.jsonl") -Encoding UTF8
([pscustomobject]@{ schema_version = "TaskSpaceProviderCacheTraceV1"; request_id = "right-1"; request_shape_classifier = "tool_free_action_contract" } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $rightArtifacts "provider-cache-trace.jsonl") -Encoding UTF8
$aggregateCache = Write-TaskspaceCostAggregateArtifacts -RootDir $aggregateCacheRoot -Scope "sample"
$aggregateCacheSummary = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $aggregateCacheRoot "provider-cache-trace-summary.json") | ConvertFrom-Json
$aggregateCacheEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $aggregateCacheRoot "provider-cache-trace.jsonl") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
Assert-True ([int]$aggregateCacheSummary.provider_request_count -eq 2) "aggregate provider cache trace should include only taskspace/right artifacts"
Assert-True ([int]$aggregateCacheSummary.native_tools_schema_hot_path_count -eq 0) "aggregate provider cache trace included standard native tools hot path"
Assert-True ([int]$aggregateCacheSummary.tool_free_action_contract_count -eq 2 -and [double]$aggregateCacheSummary.request_2_plus_hit_rate -eq 0.95) "aggregate provider cache trace did not preserve taskspace cache metrics"
Assert-True ([string]$aggregateCacheSummary.schema_version -eq "TaskSpaceProviderCacheTraceSummaryV4" -and [int]$aggregateCacheSummary.cache_warmup_candidate_count -eq 1 -and [int]$aggregateCacheSummary.tool_choice_transition_count -eq 1) "aggregate provider cache trace omitted cache-shape diagnostics"
Assert-True ($aggregateCacheEvents.Count -eq 1 -and [string]$aggregateCacheEvents[0].request_id -eq "right-1") "aggregate provider cache trace events did not filter to taskspace/right artifacts"
Assert-True ([string]$aggregateCache.provider_cache_trace_summary_path -eq (Join-Path $aggregateCacheRoot "provider-cache-trace-summary.json")) "aggregate return object omitted provider cache trace summary path"
Assert-True ([string]$aggregateCacheSummary.section_cost_summary.availability -eq "measured" -and [int]$aggregateCacheSummary.section_cost_summary.measured_request_count -eq 2) "aggregate provider cache trace omitted measured section coverage"
$aggregateActiveProjection = @($aggregateCacheSummary.section_cost_summary.sections | Where-Object { $_.kind -eq "active_projection" })[0]
Assert-True ([int64]$aggregateCacheSummary.section_cost_summary.section_bytes_total -eq 300 -and [int64]$aggregateActiveProjection.bytes -eq 40) "aggregate provider section totals are incorrect"
Assert-True ([int]$aggregateActiveProjection.request_sample_count -eq 2 -and [double]$aggregateActiveProjection.bytes_per_request_mean -eq 20 -and [double]$aggregateActiveProjection.bytes_per_request_median -eq 20) "aggregate provider section request statistics are incorrect"
Assert-True ([int]$aggregateCacheSummary.section_cost_summary.active_projection_identity_summary.bootstrap_count -eq 1 -and [int]$aggregateCacheSummary.section_cost_summary.active_projection_identity_summary.active_count -eq 1) "aggregate provider projection identity counts are incorrect"
Assert-True ([int]$aggregateCacheSummary.section_cost_summary.active_projection_identity_summary.unique_projection_sha256_count -eq 2 -and [int]$aggregateCacheSummary.section_cost_summary.active_projection_identity_summary.unique_revision_count -eq 1) "aggregate provider projection freshness evidence is incorrect"

$pair2Left = Join-Path $aggregateCacheRoot "pair-002/left/artifacts"
New-Item -ItemType Directory -Path $pair2Left -Force | Out-Null
([pscustomobject]@{ repeat = 2; left = "taskspace"; right = "standard" }) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $aggregateCacheRoot "pair-002/logical-mode-map.json") -Encoding UTF8
'{' | Set-Content -LiteralPath (Join-Path $pair2Left "metrics.json") -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $rightArtifacts "provider-cache-trace-summary.json") -Destination (Join-Path $pair2Left "provider-cache-trace-summary.json")
$alternatingAggregate = (New-TaskspaceProviderCacheTraceAggregateArtifacts $aggregateCacheRoot).provider_cache_trace_summary
Assert-True ([int]$alternatingAggregate.provider_request_count -eq 4 -and [int]$alternatingAggregate.expected_summary_count -eq 2) "logical mode map did not discover a left-side TaskSpace artifact"
Assert-True ([bool]$alternatingAggregate.comparison_eligible -and @($alternatingAggregate.aggregate_findings).Count -eq 0) "malformed side metrics overrode the authoritative logical mode map"

$rightSummaryPath = Join-Path $rightArtifacts "provider-cache-trace-summary.json"
$rightIncomparable = Get-Content -Raw -Encoding UTF8 -LiteralPath $rightSummaryPath | ConvertFrom-Json -Depth 20
$rightIncomparable.comparison_eligible = $false
$rightIncomparable.request_2_plus_cached_input_tokens = $null
$rightIncomparable.request_2_plus_uncached_input_tokens = $null
$rightIncomparable | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $rightSummaryPath -Encoding UTF8
$incomparableAggregate = (New-TaskspaceProviderCacheTraceAggregateArtifacts $aggregateCacheRoot).provider_cache_trace_summary
Assert-True (-not [bool]$incomparableAggregate.comparison_eligible -and $null -eq $incomparableAggregate.request_2_plus_hit_rate) "aggregate restored a cache rate from an incomparable side"
Assert-True ($null -eq $incomparableAggregate.request_2_plus_cached_input_tokens -and $null -eq $incomparableAggregate.cache_usage_missing_count) "aggregate coerced incomparable cache totals to zero"
'{' | Set-Content -LiteralPath $rightSummaryPath -Encoding UTF8
$invalidAggregate = (New-TaskspaceProviderCacheTraceAggregateArtifacts $aggregateCacheRoot).provider_cache_trace_summary
Assert-True ($null -eq $invalidAggregate.provider_request_count -and $null -eq $invalidAggregate.provider_attempt_count -and -not [bool]$invalidAggregate.comparison_eligible) "malformed side summary produced a partial exact aggregate"
Assert-True (@($invalidAggregate.aggregate_findings) -contains "cache_summary_invalid") "malformed side summary did not emit an aggregate finding"
$emptyAggregateRoot = Join-Path $RunRoot "empty-cache-aggregate"
New-Item -ItemType Directory -Path $emptyAggregateRoot -Force | Out-Null
$emptyAggregate = (New-TaskspaceProviderCacheTraceAggregateArtifacts $emptyAggregateRoot).provider_cache_trace_summary
Assert-True ($null -eq $emptyAggregate.provider_request_count -and $null -eq $emptyAggregate.provider_attempt_count -and -not [bool]$emptyAggregate.comparison_eligible) "empty cache aggregate was reported as measured zero"
Assert-True (@($emptyAggregate.aggregate_findings) -contains "cache_summary_scope_empty") "empty cache aggregate did not expose missing scope"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "cost instrumentation selftest passed"
