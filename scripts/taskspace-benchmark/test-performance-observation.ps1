param([string]$RunRoot = "")

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/performance-observation.ps1")
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target/performance-observation-selftest" }

$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}
function Write-Json($Value, [string]$Path) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    $Value | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $Path -Encoding UTF8
}
function Write-JsonLines([object[]]$Rows, [string]$Path) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    @($Rows) | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 } | Set-Content -LiteralPath $Path -Encoding UTF8
}
function New-SideFixture {
    param(
        [string]$PairDir, [string]$Side, [string]$Mode, [int]$Requests, [int]$Tools,
        [int]$InputTokens, [int]$CachedTokens, [int]$OutputTokens,
        [ValidateSet("measured", "historical_v2")][string]$SectionAvailability = "measured"
    )
    $artifactDir = Join-Path (Join-Path $PairDir $Side) "artifacts"
    New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
    $isTaskspace = $Mode -eq "taskspace"
    Write-Json ([pscustomobject]@{
            logical_mode = $Mode; business_success = $true; agent_completion_status = "complete"
            public_validation_exit_code = 0; hidden_oracle_exit_code = 0; external_validation_status = "passed"
            tool_call_count = $Tools; failed_tool_call_count = 1; wall_time_ms = $Requests * 1000
            input_tokens = $InputTokens; cached_input_tokens = $CachedTokens
            uncached_input_tokens = $InputTokens - $CachedTokens; output_tokens = $OutputTokens
            maps = $(if ($isTaskspace) { 1 } else { 0 }); nodes = $(if ($isTaskspace) { 3 } else { 0 })
            edges = $(if ($isTaskspace) { 2 } else { 0 }); open_leaf_nodes = 0; changed_paths = @("src/fix.py")
        }) (Join-Path $artifactDir "metrics.json")
    $sectionRows = @(
        [pscustomobject]@{ kind = "system_messages"; count = $Requests; bytes = 40 * $Requests; estimated_tokens = 10 * $Requests },
        [pscustomobject]@{ kind = "natural_history"; count = $Requests; bytes = 30 * $Requests; estimated_tokens = 8 * $Requests },
        [pscustomobject]@{ kind = "active_projection"; count = $(if ($isTaskspace) { $Requests } else { 0 }); bytes = $(if ($isTaskspace) { 20 * $Requests } else { 0 }); estimated_tokens = $(if ($isTaskspace) { 5 * $Requests } else { 0 }) },
        [pscustomobject]@{ kind = "taskspace_control_feedback"; count = $(if ($isTaskspace) { $Requests } else { 0 }); bytes = $(if ($isTaskspace) { 10 * $Requests } else { 0 }); estimated_tokens = $(if ($isTaskspace) { 3 * $Requests } else { 0 }) },
        [pscustomobject]@{ kind = "ordinary_tool_feedback"; count = $Requests; bytes = 15 * $Requests; estimated_tokens = 4 * $Requests },
        [pscustomobject]@{ kind = "tools"; count = $Requests; bytes = 10 * $Requests; estimated_tokens = 3 * $Requests },
        [pscustomobject]@{ kind = "tool_choice"; count = $Requests; bytes = 5 * $Requests; estimated_tokens = $Requests },
        [pscustomobject]@{ kind = "other_payload"; count = $Requests; bytes = 10 * $Requests; estimated_tokens = 3 * $Requests }
    )
    foreach ($section in $sectionRows) {
        $bytesPerRequest = [int64]$section.bytes / $Requests
        $tokensPerRequest = [int64]$section.estimated_tokens / $Requests
        $section | Add-Member -NotePropertyName request_bytes -NotePropertyValue @(1..$Requests | ForEach-Object { $bytesPerRequest })
        $section | Add-Member -NotePropertyName request_estimated_tokens -NotePropertyValue @(1..$Requests | ForEach-Object { $tokensPerRequest })
        $section | Add-Member -NotePropertyName request_sample_count -NotePropertyValue $Requests
        $section | Add-Member -NotePropertyName bytes_per_request_mean -NotePropertyValue $bytesPerRequest
        $section | Add-Member -NotePropertyName bytes_per_request_median -NotePropertyValue $bytesPerRequest
        $section | Add-Member -NotePropertyName estimated_tokens_per_request_mean -NotePropertyValue $tokensPerRequest
        $section | Add-Member -NotePropertyName estimated_tokens_per_request_median -NotePropertyValue $tokensPerRequest
    }
    $sectionBytes = [int64](($sectionRows | Measure-Object -Property bytes -Sum).Sum)
    $sectionTokens = [int64](($sectionRows | Measure-Object -Property estimated_tokens -Sum).Sum)
    $projectionIdentity = if ($SectionAvailability -eq "historical_v2") {
        [pscustomobject]@{
            schema_version = "provider-wire-active-projection-identity-summary-v1"
            bootstrap_count = 0; active_count = 0; unavailable_count = $Requests
            unavailable_reason_counts = [pscustomobject]@{ historical_provider_wire_trace_v2 = $Requests }
            projection_sha256_counts = [pscustomobject]@{}; revision_counts = [pscustomobject]@{}
            unique_projection_sha256_count = 0; unique_revision_count = 0
        }
    } elseif ($isTaskspace) {
        [pscustomobject]@{
            schema_version = "provider-wire-active-projection-identity-summary-v1"
            bootstrap_count = 1; active_count = $Requests - 1; unavailable_count = 0
            unavailable_reason_counts = [pscustomobject]@{}
            projection_sha256_counts = [pscustomobject]@{ 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' = $Requests }
            revision_counts = [pscustomobject]@{ '1' = $Requests - 1 }
            unique_projection_sha256_count = 1; unique_revision_count = 1
        }
    } else {
        [pscustomobject]@{
            schema_version = "provider-wire-active-projection-identity-summary-v1"
            bootstrap_count = 0; active_count = 0; unavailable_count = $Requests
            unavailable_reason_counts = [pscustomobject]@{ active_projection_missing = $Requests }
            projection_sha256_counts = [pscustomobject]@{}; revision_counts = [pscustomobject]@{}
            unique_projection_sha256_count = 0; unique_revision_count = 0
        }
    }
    $sectionCostSummary = if ($SectionAvailability -eq "measured") {
        [pscustomobject]@{
            schema_version = "provider-wire-section-cost-summary-v1"; availability = "measured"
            request_count = $Requests; measured_request_count = $Requests; unavailable_request_count = 0
            unavailable_reason_counts = [pscustomobject]@{}; section_bytes_total = $sectionBytes
            estimated_tokens_total = $sectionTokens; sections = $sectionRows
            active_projection_identity_summary = $projectionIdentity
        }
    } else {
        [pscustomobject]@{
            schema_version = "provider-wire-section-cost-summary-v1"; availability = "unavailable"
            request_count = $Requests; measured_request_count = 0; unavailable_request_count = $Requests
            unavailable_reason_counts = [pscustomobject]@{ historical_provider_wire_trace_v2 = $Requests }
            section_bytes_total = $null; estimated_tokens_total = $null; sections = @()
            active_projection_identity_summary = $projectionIdentity
        }
    }
    Write-Json ([pscustomobject]@{
            provider_request_count = $Requests; trace_coverage = 1.0; request_2_plus_count = $Requests - 1
            request_2_plus_cached_input_tokens = $CachedTokens - 100; request_2_plus_uncached_input_tokens = 100
            request_2_plus_hit_rate = ($CachedTokens - 100) / [double]$CachedTokens
            prefix_comparison_count = $Requests - 1; prefix_preserved_count = $Requests - 1; prefix_preserved_rate = 1.0
            zero_cache_hit_count = 0; cache_warmup_candidate_count = 0; same_shape_zero_hit_count = 0
            tool_choice_transition_count = $(if ($isTaskspace) { 1 } else { 0 })
            cache_shape_transition_count = $(if ($isTaskspace) { 1 } else { 0 })
            section_cost_summary = $sectionCostSummary
        }) (Join-Path $artifactDir "provider-cache-trace-summary.json")
    Write-JsonLines @(
        [pscustomobject]@{ event_name = "provider.chat_wire_shape_recorded"; request_index = 1; message_shapes = @(
                [pscustomobject]@{ content_sha256 = "same"; bytes = 11 },
                [pscustomobject]@{ content_sha256 = "same"; bytes = 13 },
                [pscustomobject]@{ content_sha256 = "unique"; bytes = 17 }
            ) }
    ) (Join-Path $artifactDir "provider-wire-trace.jsonl")
    if ($isTaskspace) {
        $nodes = @(
            [pscustomobject]@{ id = "node-1"; title = "Inspect"; kind = "inspect_code_context"; status = "completed"; results = @([pscustomobject]@{ id = "result-1" }) },
            [pscustomobject]@{ id = "node-2"; title = "Fix"; kind = "implement_solution"; status = "completed"; results = @([pscustomobject]@{ id = "result-2" }) },
            [pscustomobject]@{ id = "node-3"; title = "Validate"; kind = "smoke_test"; status = "completed"; results = @([pscustomobject]@{ id = "result-3" }) }
        )
        Write-Json ([pscustomobject]@{
                nodes = $nodes; edges = @([pscustomobject]@{ from = "node-1"; to = "node-2" }, [pscustomobject]@{ from = "node-2"; to = "node-3" })
                tasks = @([pscustomobject]@{ id = "task-1"; status = "active" })
            }) (Join-Path $artifactDir "observability/action-map-observability.json")
        Write-Json ([pscustomobject]@{
                node_count = 3; edge_count = 2; result_count = 3; accepted_result_count = 0; unreviewed_result_count = 3
            }) (Join-Path $artifactDir "graph-health.json")
        Write-Json ([pscustomobject]@{
                retention_coverage_ratio = 1.0; salience_coverage_ratio = 1.0; semantic_replacement_rate = 0.0
                protected_miss_count = 0; compaction_event_count = 0
            }) (Join-Path $artifactDir "map-management-summary.json")
        Write-Json ([pscustomobject]@{
                taskspace_control_count = 3
                action_counts = [pscustomobject]@{ initialize_map = 1; transition_node = 1; finish_map = 1 }
                control_failure_count = 1
                control_protocol_failure_count = 0; control_state_failure_count = 0; nested_action_failure_count = 1
                read_map_request_count = 2; read_map_completion_count = 2; read_map_failure_count = 0
                read_map_repeated_revision_count = 1; read_map_revision_lag_sample_count = 2
                read_map_revision_lag_mean = 0.0; read_map_revision_lag_max = 0; read_map_stale_revision_error_count = 0
                taskspace_runtime_event_count = 120; runtime_event_counts = [pscustomobject]@{ snapshot_updated = 30 }
            }) (Join-Path $artifactDir "taskspace-control-usage.json")
        $rollout = @(
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{
                    type = "task_context_event_recorded"; eventType = "function_call"; callId = "init-control"; parentCallId = $null
                    rawPayload = [pscustomobject]@{
                        type = "function_call"; name = "taskspace_control"; call_id = "init-control"
                        arguments = ([ordered]@{
                                action = "initialize_map"
                                root = [ordered]@{ node_id = "root"; goal = "Solve" }
                                initial_work_node = [ordered]@{ node_id = "node-1"; goal = "Inspect" }
                                finish_identity = [ordered]@{ id = "finish" }
                                additional_work_nodes = @()
                                edges = @([ordered]@{ from = "root"; to = "node-1" }, [ordered]@{ from = "node-1"; to = "finish" })
                                required_next_call = "ordinary_tool"
                            } | ConvertTo-Json -Compress -Depth 10)
                    }
                } },
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{
                    type = "task_context_event_recorded"; eventType = "function_call"; callId = "top-1"; parentCallId = $null
                    rawPayload = [pscustomobject]@{
                        type = "function_call"; name = "exec_command"; call_id = "top-1"; arguments = ([ordered]@{ cmd = "pwd"; z = "last" } | ConvertTo-Json -Compress -Depth 10)
                    }
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call_output"; call_id = "init-control"
                    output = (@{
                            schema_version = "TaskSpaceControlResultV2"; action = "initialize_map"; status = "committed"; success = $true
                            state_commit = $true; committed_revision = 2
                            delta = @{ map_id = "map-1"; committed_revision = 2; graph_event_refs = @(@{ revision = 1 }, @{ revision = 1 }, @{ revision = 2 }); node_detail_event_refs = @() }
                            steps = @(
                                @{ kind = "map_initialized"; map_id = "map-1"; revision = 2 },
                                @{ kind = "node_bound"; map_id = "map-1"; node_id = "node-1"; status = "running"; revision = 2 }
                            )
                        } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{
                    type = "task_context_event_recorded"; eventType = "function_call"; callId = "nested-1"; parentCallId = "init-control"
                    rawPayload = [pscustomobject]@{
                        type = "function_call"; name = "exec_command"; call_id = "nested-1"; arguments = ([ordered]@{ cmd = "pwd"; z = "last" } | ConvertTo-Json -Compress -Depth 10)
                    }
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "message"; role = "developer"; content = @(
                        [pscustomobject]@{ type = "input_text"; text = "TaskSpace mode is now active." },
                        [pscustomobject]@{ type = "input_text"; text = "ContextProjectionV1 epoch snapshot: active_task_path_without_nodes TaskSpace blank TaskSpace v0.0.5 thin bootstrap" }
                    ) } },
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{ type = "map_runtime"; map_event_type = "snapshot_updated"; snapshot = [pscustomobject]@{ node_id = "node-1"; status = "open" } } },
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{ type = "map_runtime"; map_event_type = "snapshot_delta"; baseCheckpointId = "checkpoint-1"; sequence = 1; patch = @([pscustomobject]@{ op = "replace"; path = "/routingRequired"; value = $false }) } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "gate-1"; output = "same gate" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "gate-2"; output = "same gate" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call"; name = "taskspace_control"; call_id = "finish-nodes-control"
                    arguments = (@{ action = "transition_node"; expected_revision = 2; node_id = "node-1"; transition = "complete" } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call_output"; call_id = "finish-nodes-control"
                    output = (@{
                            schema_version = "TaskSpaceControlResultR6V1"; status = "committed"; success = $true
                            state_commit = $true; committed_revision = 3
                            delta = @{ map_id = "map-1"; committed_revision = 3; graph_event_refs = @(@{ revision = 3 }); node_detail_event_refs = @() }
                            steps = @(@{ kind = "node_transition"; map_id = "map-1"; node_id = "node-1"; revision = 3; status = "completed" })
                        } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call"; name = "taskspace_control"; call_id = "terminal-failure-control"
                    arguments = (@{ action = "finish_map"; expected_revision = 3; terminal_node_id = "finish"; final_summary = "Rejected candidate" } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call_output"; call_id = "terminal-failure-control"
                    output = (@{
                            schema_version = "TaskSpaceControlResultR6V1"; status = "state_machine_failed"; success = $false
                            state_commit = $false; committed_revision = $null; delta = $null
                            steps = @(@{ kind = "state_rejection"; success = $false; error = @{ code = "fixture_reject" } })
                        } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "apply_patch"; arguments = '{"input":"patch"}' } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call"; name = "taskspace_control"
                    arguments = (@{ action = "finish_map"; expected_revision = 3; terminal_node_id = "finish"; final_summary = "done" } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "message"; role = "assistant"; phase = "final_answer"; content = @([pscustomobject]@{ type = "output_text"; text = "done" }) } }
        )
        Write-JsonLines $rollout (Join-Path $artifactDir "rollout.jsonl")
    } else {
        $execRows = @()
        foreach ($i in 1..$Tools) { $execRows += [pscustomobject]@{ type = "item.completed"; item = [pscustomobject]@{ type = "command_execution" } } }
        $execRows += [pscustomobject]@{ type = "item.completed"; item = [pscustomobject]@{ type = "file_change" } }
        Write-JsonLines $execRows (Join-Path $artifactDir "whale-exec.jsonl")
    }
}

$cadenceFixture = Join-Path $RunRoot "cadence-fixture"
$initializeArgs = @{
    action = "initialize_map"
    root = @{ node_id = "root"; goal = "Solve" }
    initial_work_node = @{ node_id = "inspect"; goal = "Inspect" }
    additional_work_nodes = @(@{ node_id = "plan"; goal = "Plan" })
    finish_identity = @{ id = "finish" }
    edges = @(@{ from = "root"; to = "inspect" }, @{ from = "inspect"; to = "plan" }, @{ from = "plan"; to = "finish" })
} | ConvertTo-Json -Compress -Depth 10
$mutationArgs = @{
    action = "mutate_graph"; expected_revision = 2
    add_nodes = @(@{ node_id = "verify"; goal = "Verify" })
    add_edges = @(@{ from = "plan"; to = "verify" }, @{ from = "verify"; to = "finish" })
    remove_edges = @(@{ from = "plan"; to = "finish" })
} | ConvertTo-Json -Compress -Depth 10
$completeArgs = @{
    action = "complete_then_continue"; expected_revision = 3; current_node_id = "inspect"; next_node_id = "plan"
} | ConvertTo-Json -Compress -Depth 10
$terminalArgs = @{
    action = "finish_map"; expected_revision = 4; terminal_node_id = "plan"; final_summary = "Agent final"
} | ConvertTo-Json -Compress -Depth 10
Write-JsonLines @(
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "init"; arguments = $initializeArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "exec_command"; call_id = "init-tool"; arguments = '{"cmd":"pwd","taskspace_binding":"after_boundary"}' } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "init"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "init-tool"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "mutation"; arguments = $mutationArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "exec_command"; call_id = "mutation-tool"; arguments = '{"cmd":"git status","taskspace_binding":"active"}' } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "mutation"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "mutation-tool"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "complete"; arguments = $completeArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "apply_patch"; call_id = "patch"; arguments = '{"input":"patch","taskspace_binding":"after_boundary"}' } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "complete"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "patch"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "finish"; arguments = $terminalArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "finish"; output = (@{ schema_version = "TaskSpaceControlResultV2"; status = "committed"; success = $true; steps = @(@{ kind = "finish_map"; terminal_node_id = "plan"; terminal_node_role = "work" }) } | ConvertTo-Json -Compress -Depth 10) } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "message"; role = "assistant"; phase = "final_answer"; content = @([pscustomobject]@{ type = "output_text"; text = "Agent final" }) } }
) (Join-Path $cadenceFixture "rollout.jsonl")
$cadence = Get-TaskspaceNativeCadenceFacts $cadenceFixture $null
Assert-True ($cadence.provider_tool_response_count -eq 4) "provider tool response count is incorrect"
Assert-True ($cadence.control_response_count -eq 4) "control response count is incorrect"
Assert-True ($cadence.mixed_control_action_response_count -eq 3) "mixed control/action responses were not observed"
Assert-True ($cadence.multi_control_response_count -eq 0) "unexpected multi-control response was observed"
Assert-True ($cadence.boundary_action_count -eq 2 -and $cadence.boundary_pair_count -eq 2) "boundary actions were not paired"
Assert-True ($cadence.boundary_violation_count -eq 0 -and $cadence.orphan_after_boundary_count -eq 0) "valid sequence was classified as invalid"
Assert-True ($cadence.ordinary_binding_count -eq 3 -and $cadence.active_binding_count -eq 1 -and $cadence.after_boundary_binding_count -eq 2) "ordinary bindings were not classified"
Assert-True ($cadence.initialize_pair_count -eq 1 -and $cadence.bind_pair_count -eq 0) "boundary action types were not classified"
Assert-True ($cadence.complete_handoff_count -eq 1 -and $cadence.complete_handoff_pair_count -eq 1) "atomic handoff was not observed"
Assert-True ($cadence.finish_map_count -eq 1 -and $cadence.finish_map_last_running_work_count -eq 1 -and $cadence.finish_map_ready_finish_count -eq 0) "finish_map state counts are incorrect"
Assert-True ($cadence.standalone_control_response_count -eq 1) "standalone control response count is incorrect"
Assert-True ($cadence.terminal_candidate_count -eq 1 -and $cadence.terminal_extra_request_count -eq 0) "terminal candidate cadence was not measured"
Assert-True ($cadence.control_argument_parse_error_count -eq 0) "cadence argument parsing was incomplete"

$cadenceViolationFixture = Join-Path $RunRoot "cadence-violation-fixture"
Write-JsonLines @(
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "bad-handoff"; arguments = $completeArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "exec_command"; call_id = "wrong-sibling"; arguments = '{"cmd":"pwd","taskspace_binding":"active"}' } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "bad-handoff"; output = "protocol_failed" } }
) (Join-Path $cadenceViolationFixture "rollout.jsonl")
$cadenceViolation = Get-TaskspaceNativeCadenceFacts $cadenceViolationFixture $null
Assert-True ($cadenceViolation.boundary_action_count -eq 1) "invalid boundary action was not counted"
Assert-True ($cadenceViolation.boundary_pair_count -eq 0 -and $cadenceViolation.boundary_violation_count -eq 1) "invalid boundary pair was not classified"

if (Test-Path -LiteralPath $RunRoot) { Remove-Item -LiteralPath $RunRoot -Recurse -Force }
$pair1 = Join-Path $RunRoot "pair-001"; $pair2 = Join-Path $RunRoot "pair-002"
Write-Json ([pscustomobject]@{ repeat = 1; left = "standard"; right = "taskspace" }) (Join-Path $pair1 "logical-mode-map.json")
Write-Json ([pscustomobject]@{ repeat = 2; left = "taskspace"; right = "standard" }) (Join-Path $pair2 "logical-mode-map.json")
New-SideFixture $pair1 "left" "standard" 6 8 6000 5000 500
New-SideFixture $pair1 "right" "taskspace" 10 12 12000 10800 800
New-SideFixture $pair2 "left" "taskspace" 8 10 10000 9000 700
New-SideFixture $pair2 "right" "standard" 4 6 4000 3200 400 -SectionAvailability "historical_v2"
$pair3 = Join-Path $RunRoot "pair-003"
Write-Json ([pscustomobject]@{ repeat = 3; left = "standard"; right = "taskspace" }) (Join-Path $pair3 "logical-mode-map.json")
Write-Json ([pscustomobject]@{
        logical_mode = "standard"; business_success = $false; agent_completion_status = $null
        tool_call_count = 0; wall_time_ms = 0; input_tokens = 0; cached_input_tokens = 0; uncached_input_tokens = 0; output_tokens = 0
        maps = 0; nodes = 0; edges = 0; open_leaf_nodes = 0; metrics_taints = @("side_selection_skipped:left:run_side=right")
    }) (Join-Path $pair3 "left/artifacts/metrics.json")

$result = Write-TaskspacePerformanceObservation -RunRoot $RunRoot
$report = Get-Content -Raw -Encoding UTF8 -LiteralPath $result.json_path | ConvertFrom-Json
$standard = @($report.aggregates | Where-Object { $_.logical_mode -eq "standard" })[0]
$taskspace = @($report.aggregates | Where-Object { $_.logical_mode -eq "taskspace" })[0]
Assert-True ($report.rows.Count -eq 5) "report did not include measured and skipped sides"
Assert-True ($standard.totals.provider_requests -eq 10) "standard request aggregate ignored alternating side mapping"
Assert-True ($taskspace.totals.provider_requests -eq 18) "taskspace request aggregate ignored alternating side mapping"
Assert-True ($taskspace.totals.node_count -eq 6 -and $taskspace.totals.edge_count -eq 4) "map totals are incorrect"
Assert-True ($taskspace.totals.unreviewed_result_count -eq 6) "result lifecycle totals are incorrect"
Assert-True ($taskspace.totals.control_failures -eq 2) "control failures are missing"
Assert-True ($taskspace.totals.request_patch_count -eq 2) "patch declarations are missing from aggregate"
Assert-True ($report.ratios.provider_requests -eq 1.8) "request ratio is incorrect"
Assert-True ([string]$taskspace.section_cost.availability -eq "measured" -and [int]$taskspace.section_cost.measured_request_count -eq 18 -and [int64]$taskspace.section_cost.section_bytes_total -eq 2520) "taskspace mode section totals are incorrect"
Assert-True ([string]$standard.section_cost.availability -eq "partial" -and [int]$standard.section_cost.measured_request_count -eq 6 -and [int]$standard.section_cost.unavailable_request_count -eq 4) "standard mode did not preserve mixed v3/v2 section availability"
Assert-True ([int]$taskspace.section_cost.active_projection_identity_summary.bootstrap_count -eq 2 -and [int]$taskspace.section_cost.active_projection_identity_summary.active_count -eq 16) "taskspace projection identity lifecycle was not aggregated"
Assert-True ([int]$taskspace.section_cost.active_projection_identity_summary.unique_projection_sha256_count -eq 1 -and [int]$taskspace.section_cost.active_projection_identity_summary.unique_revision_count -eq 1) "taskspace projection identity hashes or revisions were lost"
Assert-True ([int]$standard.section_cost.active_projection_identity_summary.unavailable_count -eq 10) "standard projection unavailability was not explicit"
$taskspaceSystemSection = @($taskspace.section_cost.sections | Where-Object { $_.kind -eq "system_messages" })[0]
Assert-True ([double]$taskspaceSystemSection.bytes_per_request_mean -eq 40 -and [double]$taskspaceSystemSection.bytes_per_request_median -eq 40 -and [int]$taskspaceSystemSection.request_sample_count -eq 18) "taskspace section request statistics are incorrect"
$standardMeasuredRow = @($report.rows | Where-Object { $_.repeat -eq 1 -and $_.logical_mode -eq "standard" })[0]
$standardHistoricalRow = @($report.rows | Where-Object { $_.repeat -eq 2 -and $_.logical_mode -eq "standard" })[0]
$standardActiveProjection = @($standardMeasuredRow.section_cost.sections | Where-Object { $_.kind -eq "active_projection" })[0]
$standardControlFeedback = @($standardMeasuredRow.section_cost.sections | Where-Object { $_.kind -eq "taskspace_control_feedback" })[0]
Assert-True ([int64]$standardActiveProjection.bytes -eq 0 -and [int64]$standardControlFeedback.bytes -eq 0) "measured Standard side should report zero TaskSpace-only sections"
Assert-True ($null -eq $standardHistoricalRow.section_cost.section_bytes_total -and [int]$standardHistoricalRow.section_cost.unavailable_reason_counts.historical_provider_wire_trace_v2 -eq 4) "historical v2 side fabricated section totals"
Assert-True ([int64](($standardMeasuredRow.section_cost.sections | Measure-Object -Property bytes -Sum).Sum) -eq [int64]$standardMeasuredRow.section_cost.section_bytes_total) "side section bytes did not reconcile exactly"
Assert-True (@($report.rows | Where-Object { $_.observation_status -eq "skipped" }).Count -eq 1) "right-only placeholder side was not classified as skipped"
Assert-True ($standard.observed_side_count -eq 3 -and $standard.excluded_side_count -eq 1) "skipped side contaminated the aggregate"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.map.nodes.Count -eq 3 }).Count -eq 2) "map node details are missing"
Assert-True (@($report.rows | Where-Object {
            $_.logical_mode -eq "taskspace" -and
            $_.map.read_map_request_count -eq 2 -and
            $_.map.read_map_completion_count -eq 2 -and
            $_.map.read_map_failure_count -eq 0 -and
            $_.map.read_map_repeated_revision_count -eq 1 -and
            $_.map.read_map_revision_lag_sample_count -eq 2 -and
            $_.map.read_map_revision_lag_mean -eq 0.0 -and
            $_.map.read_map_revision_lag_max -eq 0 -and
            $_.map.read_map_stale_revision_error_count -eq 0
        }).Count -eq 2) "map read lifecycle metrics are missing"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout.duplicate_output_bodies -eq 1 }).Count -eq 2) "exact duplicate output bodies were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.provider_wire.final_content_duplicates -eq 1 }).Count -eq 2) "wire content duplicates were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.final_candidate_assistant_exact_equal_count -eq 1 }).Count -eq 2) "final candidate exact assistant equality was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.expanded_nested_call_exact_json_match_count -eq 0 }).Count -eq 2) "string continuations should not report nested JSON lineage"
Assert-True (@($report.rows | Where-Object {
            $_.logical_mode -eq "taskspace" -and
            $_.duplication.cross_carrier_lineage.control_success_count -eq 2 -and
            $_.duplication.cross_carrier_lineage.control_identity_step_count -eq 3 -and
            $_.duplication.cross_carrier_lineage.control_identity_missing_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_continuation_action_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_delta_present_count -eq 2 -and
            $_.duplication.cross_carrier_lineage.control_delta_missing_count -eq 1 -and
            $_.duplication.cross_carrier_lineage.control_graph_event_ref_count -eq 4 -and
            $_.duplication.cross_carrier_lineage.control_node_detail_event_ref_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.terminal_failure_nonzero_commit_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_output_init_node_id_echo_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_output_finished_node_id_echo_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_output_next_node_echo_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_output_current_node_echo_count -eq 1
        }).Count -eq 2) "R6/V2 control identity coverage was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.stale_blank_developer_marker_count -eq 1 -and $_.duplication.cross_carrier_lineage.stale_mode_developer_marker_count -eq 1 }).Count -eq 2) "stale developer marker counts were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout_storage.snapshot_updated_line_count -eq 1 -and $_.duplication.rollout_storage.snapshot_updated_payload_bytes -gt 0 -and $_.duplication.rollout_storage.snapshot_updated_payload_ratio -gt 0 }).Count -eq 2) "rollout storage snapshot byte ratio was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout_storage.snapshot_delta_line_count -eq 1 -and $_.duplication.rollout_storage.snapshot_delta_payload_bytes -gt 0 -and $_.duplication.rollout_storage.internal_replay_payload_bytes -gt $_.duplication.rollout_storage.snapshot_updated_payload_bytes }).Count -eq 2) "rollout storage delta and aggregate replay bytes were not measured"
Assert-True (Test-Path -LiteralPath $result.markdown_path) "markdown report was not written"
Assert-True (Test-Path -LiteralPath $result.event_log_path) "event log was not written"
$markdown = Get-Content -Raw -Encoding UTF8 -LiteralPath $result.markdown_path
Assert-True ($markdown -match "## Map 节点") "markdown omitted map node details"
Assert-True ($markdown -match "## Map 语义保存") "markdown omitted map semantic preservation details"
Assert-True ($markdown -match "## Map 显式读取") "markdown omitted explicit map read metrics"
Assert-True ($markdown -match "## 精确重复载体") "markdown omitted exact carrier duplication details"
Assert-True ($markdown -match "## Cross carrier lineage") "markdown omitted cross carrier lineage details"
Assert-True ($markdown -match "## Rollout storage") "markdown omitted rollout storage details"
Assert-True ($markdown -match "Boundary actions" -and $markdown -match "Violations") "markdown omitted boundary pairing counts"
Assert-True ($markdown -match "## Patch lifecycle") "markdown omitted patch lifecycle metrics"
Assert-True ($markdown -match "## Provider wire section cost" -and $markdown -match "active_projection") "markdown omitted provider section totals"
Assert-True ($markdown -match "### Active projection identity" -and $markdown -match "active_projection_missing=6") "markdown omitted projection identity freshness evidence"
Assert-True ($markdown -match "Bytes/request mean" -and $markdown -match "Bytes/request median") "markdown omitted section request distribution statistics"
Assert-True ($markdown -match "historical_provider_wire_trace_v2=4") "markdown omitted historical section-cost unavailability"
Assert-True ($markdown -match "root_task_active_after_nodes_closed") "mechanical map warning was not rendered"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "performance observation self-test passed"
