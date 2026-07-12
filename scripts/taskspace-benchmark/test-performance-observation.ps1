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
        [int]$InputTokens, [int]$CachedTokens, [int]$OutputTokens
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
    Write-Json ([pscustomobject]@{
            provider_request_count = $Requests; trace_coverage = 1.0; request_2_plus_count = $Requests - 1
            request_2_plus_cached_input_tokens = $CachedTokens - 100; request_2_plus_uncached_input_tokens = 100
            request_2_plus_hit_rate = ($CachedTokens - 100) / [double]$CachedTokens
            prefix_comparison_count = $Requests - 1; prefix_preserved_count = $Requests - 1; prefix_preserved_rate = 1.0
            zero_cache_hit_count = 0; cache_warmup_candidate_count = 0; same_shape_zero_hit_count = 0
            tool_choice_transition_count = $(if ($isTaskspace) { 1 } else { 0 })
            cache_shape_transition_count = $(if ($isTaskspace) { 1 } else { 0 })
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
                action_counts = [pscustomobject]@{ initialize_then_actions = 1; finish_nodes = 1; finish_then_end = 1 }
                control_failure_count = 1
                control_protocol_failure_count = 0; control_state_failure_count = 0; nested_action_failure_count = 1
                taskspace_runtime_event_count = 120; runtime_event_counts = [pscustomobject]@{ snapshot_updated = 30 }
            }) (Join-Path $artifactDir "taskspace-control-usage.json")
        $rollout = @(
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{
                    type = "task_context_event_recorded"; eventType = "function_call"; callId = "init-control"; parentCallId = $null
                    rawPayload = [pscustomobject]@{
                        type = "function_call"; name = "taskspace_control"; call_id = "init-control"
                        arguments = ([ordered]@{
                                action = "initialize_then_actions"
                                initial_nodes = @([ordered]@{ node_id = "node-1"; kind = "inspect_code_context"; goal = "Inspect" })
                                current_node_id = "node-1"
                                continuation = [ordered]@{ kind = "actions"; actions = @([ordered]@{ tool_name = "exec_command"; arguments = [ordered]@{ z = "last"; cmd = "pwd" } }) }
                            } | ConvertTo-Json -Compress -Depth 10)
                    }
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call_output"; call_id = "init-control"
                    output = (@{
                            schema_version = "TaskSpaceControlResultV2"; status = "committed"; success = $true
                            steps = @(@{ kind = "map_initialized"; task_id = "task-1"; map_id = "map-1"; created_node_ids = @("node-1"); current_node_id = "node-1" })
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
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{ type = "snapshot_updated"; snapshot = [pscustomobject]@{ node_id = "node-1"; status = "open" } } },
            [pscustomobject]@{ type = "event_msg"; payload = [pscustomobject]@{ type = "snapshot_delta"; baseCheckpointId = "checkpoint-1"; sequence = 1; patch = @([pscustomobject]@{ op = "replace"; path = "/routingRequired"; value = $false }) } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "gate-1"; output = "same gate" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "gate-2"; output = "same gate" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call"; name = "taskspace_control"; call_id = "finish-nodes-control"
                    arguments = (@{ action = "finish_nodes"; finishes = @(@{ node_id = "node-1"; next = @{ kind = "existing"; node_id = "node-2" } }) } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call_output"; call_id = "finish-nodes-control"
                    output = (@{
                            schema_version = "TaskSpaceControlResultV2"; status = "committed"; success = $true
                            steps = @(@{ kind = "state_transition"; index = 0; finished_node_id = "node-1"; result_id = "result-1"; next = @{ kind = "existing"; node_id = "node-2" }; current_node_id = "node-2" })
                        } | ConvertTo-Json -Compress -Depth 10)
                } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "apply_patch"; arguments = '{"input":"patch"}' } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output" } },
            [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{
                    type = "function_call"; name = "taskspace_control"
                    arguments = (@{ action = "finish_then_end"; terminal_node_id = "node-2"; final_candidate = "done" } | ConvertTo-Json -Compress -Depth 10)
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
    action = "initialize_then_actions"
    initial_nodes = @(@{ node_id = "inspect"; kind = "inspect_code_context"; goal = "Inspect" })
    current_node_id = "inspect"
    continuation = @{ kind = "actions"; actions = @(@{ tool_name = "exec_command"; arguments = @{ cmd = "pwd" } }) }
} | ConvertTo-Json -Compress -Depth 10
$finishArgs = @{
    action = "finish_nodes"
    finishes = @(
        @{ node_id = "inspect"; next = @{ kind = "existing"; node_id = "plan" } },
        @{ node_id = "plan"; next = @{ kind = "existing"; node_id = "validate" } }
    )
} | ConvertTo-Json -Compress -Depth 10
$terminalArgs = @{
    action = "finish_then_end"; terminal_node_id = "validate"; final_candidate = "Agent final"
} | ConvertTo-Json -Compress -Depth 10
Write-JsonLines @(
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "init"; arguments = $initializeArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "init"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "finish-nodes"; arguments = $finishArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "apply_patch"; call_id = "patch"; arguments = '{"input":"patch"}' } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "finish-nodes"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "patch"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call"; name = "taskspace_control"; call_id = "finish"; arguments = $terminalArgs } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "function_call_output"; call_id = "finish"; output = "ok" } },
    [pscustomobject]@{ type = "response_item"; payload = [pscustomobject]@{ type = "message"; role = "assistant"; phase = "final_answer"; content = @([pscustomobject]@{ type = "output_text"; text = "Agent final" }) } }
) (Join-Path $cadenceFixture "rollout.jsonl")
$cadence = Get-TaskspaceNativeCadenceFacts $cadenceFixture $null
Assert-True ($cadence.provider_tool_response_count -eq 3) "provider tool response count is incorrect"
Assert-True ($cadence.control_carrier_response_count -eq 3) "control carrier response count is incorrect"
Assert-True ($cadence.nested_action_count -eq 1) "bootstrap nested action count is incorrect"
Assert-True ($cadence.initialize_then_actions_count -eq 1) "init carrier was not observed"
Assert-True ($cadence.finish_nodes_count -eq 1) "finish barrier was not observed"
Assert-True ($cadence.finish_then_end_count -eq 1) "terminal carrier was not observed"
Assert-True ($cadence.multi_finish_carrier_count -eq 1) "multi-finish carrier was not observed"
Assert-True ($cadence.direct_tool_mixed_response_count -eq 1) "finish barrier and sibling tool were not observed"
Assert-True ($cadence.multi_control_carrier_response_count -eq 0) "multiple carriers were emitted in one response"
Assert-True ($cadence.finish_without_sibling_action_count -eq 0) "finish barrier lost its sibling action"
Assert-True ($cadence.terminal_candidate_count -eq 1 -and $cadence.terminal_extra_request_count -eq 0) "terminal candidate cadence was not measured"

if (Test-Path -LiteralPath $RunRoot) { Remove-Item -LiteralPath $RunRoot -Recurse -Force }
$pair1 = Join-Path $RunRoot "pair-001"; $pair2 = Join-Path $RunRoot "pair-002"
Write-Json ([pscustomobject]@{ repeat = 1; left = "standard"; right = "taskspace" }) (Join-Path $pair1 "logical-mode-map.json")
Write-Json ([pscustomobject]@{ repeat = 2; left = "taskspace"; right = "standard" }) (Join-Path $pair2 "logical-mode-map.json")
New-SideFixture $pair1 "left" "standard" 6 8 6000 5000 500
New-SideFixture $pair1 "right" "taskspace" 10 12 12000 10800 800
New-SideFixture $pair2 "left" "taskspace" 8 10 10000 9000 700
New-SideFixture $pair2 "right" "standard" 4 6 4000 3200 400
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
Assert-True ($taskspace.totals.nested_actions -eq 2) "nested continuation actions were not aggregated"
Assert-True ($taskspace.totals.request_patch_count -eq 2) "patch declarations are missing from aggregate"
Assert-True ($report.ratios.provider_requests -eq 1.8) "request ratio is incorrect"
Assert-True (@($report.rows | Where-Object { $_.observation_status -eq "skipped" }).Count -eq 1) "right-only placeholder side was not classified as skipped"
Assert-True ($standard.observed_side_count -eq 3 -and $standard.excluded_side_count -eq 1) "skipped side contaminated the aggregate"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.map.nodes.Count -eq 3 }).Count -eq 2) "map node details are missing"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout.duplicate_output_bodies -eq 1 }).Count -eq 2) "exact duplicate output bodies were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.provider_wire.final_content_duplicates -eq 1 }).Count -eq 2) "wire content duplicates were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.final_candidate_assistant_exact_equal_count -eq 1 }).Count -eq 2) "final candidate exact assistant equality was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.expanded_nested_call_exact_json_match_count -eq 1 }).Count -eq 2) "expanded nested exact JSON lineage was not measured"
Assert-True (@($report.rows | Where-Object {
            $_.logical_mode -eq "taskspace" -and
            $_.duplication.cross_carrier_lineage.control_success_v2_count -eq 2 -and
            $_.duplication.cross_carrier_lineage.control_identity_step_count -eq 2 -and
            $_.duplication.cross_carrier_lineage.control_identity_missing_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.committed_repeat_finish_count -eq 0 -and
            $_.duplication.cross_carrier_lineage.control_output_init_node_id_echo_count -eq 1 -and
            $_.duplication.cross_carrier_lineage.control_output_finished_node_id_echo_count -eq 1 -and
            $_.duplication.cross_carrier_lineage.control_output_next_node_echo_count -eq 1 -and
            $_.duplication.cross_carrier_lineage.control_output_current_node_echo_count -eq 2
        }).Count -eq 2) "control V2 identity coverage was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.cross_carrier_lineage.stale_blank_developer_marker_count -eq 1 -and $_.duplication.cross_carrier_lineage.stale_mode_developer_marker_count -eq 1 }).Count -eq 2) "stale developer marker counts were not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout_storage.snapshot_updated_line_count -eq 1 -and $_.duplication.rollout_storage.snapshot_updated_payload_bytes -gt 0 -and $_.duplication.rollout_storage.snapshot_updated_payload_ratio -gt 0 }).Count -eq 2) "rollout storage snapshot byte ratio was not measured"
Assert-True (@($report.rows | Where-Object { $_.logical_mode -eq "taskspace" -and $_.duplication.rollout_storage.snapshot_delta_line_count -eq 1 -and $_.duplication.rollout_storage.snapshot_delta_payload_bytes -gt 0 -and $_.duplication.rollout_storage.internal_replay_payload_bytes -gt $_.duplication.rollout_storage.snapshot_updated_payload_bytes }).Count -eq 2) "rollout storage delta and aggregate replay bytes were not measured"
Assert-True (Test-Path -LiteralPath $result.markdown_path) "markdown report was not written"
Assert-True (Test-Path -LiteralPath $result.event_log_path) "event log was not written"
$markdown = Get-Content -Raw -Encoding UTF8 -LiteralPath $result.markdown_path
Assert-True ($markdown -match "## Map 节点") "markdown omitted map node details"
Assert-True ($markdown -match "## Map 语义保存") "markdown omitted map semantic preservation details"
Assert-True ($markdown -match "## 精确重复载体") "markdown omitted exact carrier duplication details"
Assert-True ($markdown -match "## Cross carrier lineage") "markdown omitted cross carrier lineage details"
Assert-True ($markdown -match "## Rollout storage") "markdown omitted rollout storage details"
Assert-True ($markdown -match "Finish without sibling") "markdown omitted finish barrier validation counts"
Assert-True ($markdown -match "## Patch lifecycle") "markdown omitted patch lifecycle metrics"
Assert-True ($markdown -match "root_task_active_after_nodes_closed") "mechanical map warning was not rendered"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "performance observation self-test passed"
