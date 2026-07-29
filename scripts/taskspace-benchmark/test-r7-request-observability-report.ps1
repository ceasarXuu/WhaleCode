$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$runRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-request-observer-$([guid]::NewGuid().ToString('N'))"

function Write-Json([string]$Path, $Value) {
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    [IO.File]::WriteAllText(
        $Path,
        (($Value | ConvertTo-Json -Depth 100) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
}

function Write-JsonLines([string]$Path, [object[]]$Values) {
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    [IO.File]::WriteAllLines(
        $Path,
        @($Values | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 100 }),
        [Text.UTF8Encoding]::new($false)
    )
}

function New-WireShape(
    [string]$RequestId,
    [int]$Index,
    [object[]]$Messages,
    [int]$Lcp,
    [object[]]$Receipts = @()
) {
    [pscustomobject]@{
        schema_version = "provider-chat-wire-trace-v8"
        event_name = if ($Index -eq 1) { "provider.chat_wire_shape_recorded" } else { "provider.chat_wire_prefix_broken" }
        request_id = $RequestId
        request_index = $Index
        provider_wire_api = "ChatCompletions"
        lcp_message_count = $Lcp
        message_shapes = $Messages
        taskspace_final_receipt_identity = @{ count = $Receipts.Count; receipts = $Receipts }
        section_cost = @{
            sections = @(
                @{ kind = "tools"; estimated_tokens = 100 },
                @{ kind = "active_projection"; estimated_tokens = 20 }
            )
        }
    }
}

function New-ObservationRow([string]$LogicalMode, [string]$ArtifactDir, [int]$Requests) {
    [pscustomobject]@{
        observation_status = "complete"
        logical_mode = $LogicalMode
        artifact_dir = $ArtifactDir
        result = @{ business_success = $true; agent_completion_status = "completed" }
        actions = @{
            provider_requests = $Requests
            ordinary_tools = if ($LogicalMode -eq "taskspace") { 1 } else { 0 }
            failed_tools = 0
            taskspace_control = if ($LogicalMode -eq "taskspace") { 1 } else { 0 }
            initialize_and_execute = if ($LogicalMode -eq "taskspace") { 1 } else { 0 }
            committed_initialize_and_execute = if ($LogicalMode -eq "taskspace") { 1 } else { 0 }
            failed_initialize_and_execute = 0
            control_failures = 0
            control_protocol_failures = 0
            control_state_failures = 0
        }
        patch = @{ request_multi_patch_attempt_count = 0; patch_prepare_failure_count = 0 }
        cost = @{
            input_tokens = 300
            cached_input_tokens = 20
            uncached_input_tokens = 280
            output_tokens = 10
            wall_time_ms = 100
        }
        cache = @{
            request_2_plus_cached_input_tokens = if ($Requests -gt 1) { 20 } else { 0 }
            request_2_plus_uncached_input_tokens = if ($Requests -gt 1) { 180 } else { 0 }
            request_2_plus_hit_rate = if ($Requests -gt 1) { 0.1 } else { $null }
            prefix_preserved_rate = 1
            same_shape_zero_hit_count = 0
        }
        map = @{
            map_count = if ($LogicalMode -eq "taskspace") { 1 } else { 0 }
            node_count = if ($LogicalMode -eq "taskspace") { 3 } else { 0 }
            edge_count = if ($LogicalMode -eq "taskspace") { 2 } else { 0 }
            open_leaf_nodes = 0
            root_task_status = if ($LogicalMode -eq "taskspace") { "closed" } else { "" }
            nodes = if ($LogicalMode -eq "taskspace") {
                @(
                    @{ kind = "task_root"; status = "closed" },
                    @{ kind = "finish"; status = "closed" }
                )
            } else {
                @()
            }
        }
    }
}

try {
    $runs = [Collections.Generic.List[object]]::new()
    foreach ($arm in @("standard", "map-always", "map-append", "map-request")) {
        $logicalMode = if ($arm -eq "standard") { "standard" } else { "taskspace" }
        $runDir = Join-Path $runRoot $arm
        $artifactDir = Join-Path $runDir "artifacts"
        $requests = if ($logicalMode -eq "taskspace") { 2 } else { 1 }
        New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
        Write-Json (Join-Path $runDir "performance-observation.json") @{
            rows = @((New-ObservationRow $logicalMode $artifactDir $requests))
        }
        Write-Json (Join-Path $runDir "pair-001/manifest.resolved.json") @{
            container_image_digest = "sha256:request-observer-test"
        }
        Write-Json (Join-Path $artifactDir "request-summary.json") @{
            first_input_tokens_per_request = 100
        }

        if ($logicalMode -eq "standard") {
            Write-JsonLines (Join-Path $artifactDir "rollout.jsonl") @(
                @{ type = "event_msg"; payload = @{ type = "token_count" } }
            )
            Write-JsonLines (Join-Path $artifactDir "provider-wire-trace.jsonl") @(
                (New-WireShape "$arm-1" 1 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }) 0),
                @{ event_name = "provider.chat_wire_request_terminal"; request_id = "$arm-1"; input_tokens = 100; cached_input_tokens = 0 }
            )
        } else {
            $controlArgs = '{"action":"initialize_and_execute","root":{"node_id":"root","goal":"task"},"work_nodes":[{"node_id":"work","goal":"work"}],"finish":{"node_id":"finish","goal":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"actions":[{"node_id":"work","tool":"exec_command"}]}'
            Write-JsonLines (Join-Path $artifactDir "rollout.jsonl") @(
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "$arm-control"; rawPayload = @{ name = "taskspace_control"; arguments = $controlArgs } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "$arm-tool"; rawPayload = @{ name = "exec_command"; arguments = '{"cmd":"true"}' } } },
                @{ type = "event_msg"; payload = @{ type = "token_count" } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "$arm-control"; toolSuccess = $true; rawPayload = @{ output = '{"success":true,"state_commit":true}' } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "$arm-tool"; toolSuccess = $true; rawPayload = @{ output = "ok" } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "message"; originalRole = "developer"; rawPayload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = '{"schema_version":"TaskSpaceResponseFinalReceiptV1"}' }) } } },
                @{ type = "event_msg"; payload = @{ type = "token_count" } }
            )
            Write-JsonLines (Join-Path $artifactDir "provider-wire-trace.jsonl") @(
                (New-WireShape "$arm-1" 1 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }) 0),
                @{ event_name = "provider.chat_wire_request_terminal"; request_id = "$arm-1"; input_tokens = 100; cached_input_tokens = 0 },
                (New-WireShape "$arm-2" 2 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }, @{ index = 2; role = "assistant" }, @{ index = 3; role = "tool" }, @{ index = 4; role = "system" }) 2 @(
                    @{ message_index = 4; wire_role = "system"; control_call_id_sha256 = "hash"; reservation_revision_after = 1; canonical_revision = 2; revision_delta = 1; complete = $true }
                )),
                @{ event_name = "provider.chat_wire_request_terminal"; request_id = "$arm-2"; input_tokens = 200; cached_input_tokens = 20 }
            )
        }
        $runs.Add([pscustomobject]@{
                sample = "fixture"
                repeat = 1
                arm = $arm
                logical_mode = $logicalMode
                projection_policy = if ($logicalMode -eq "taskspace") { $arm.Substring(4) } else { "standard" }
                run_dir = $runDir
            })
    }
    Write-Json (Join-Path $runRoot "run-manifest.json") @{
        status = "completed"
        contract_id = "request-observer-test"
        repeats_per_arm_per_sample = 1
        samples = @("fixture")
        completed_run_count = 4
        runs = @($runs)
    }

    & (Join-Path $PSScriptRoot "report-r7-five-layer-matrix.ps1") -RunRoot $runRoot | Out-Null
    $summary = @(Import-Csv -LiteralPath (Join-Path $runRoot "summary.csv"))
    $trace = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runRoot "trace-analysis.json") |
        ConvertFrom-Json -Depth 100
    if ($summary.Count -ne 4 -or @($summary | Where-Object classification_reconciled -ne "True").Count) {
        throw "Matrix report did not reconcile every run"
    }
    $append = @($summary | Where-Object arm -eq "map-append")[0]
    if ([int]$append.receipt_before_requests -ne 1 -or
        [double]$append.receipt_before_cache_hit_rate -ne 0.1 -or
        [string]$append.receipt_wire_roles -ne "system") {
        throw "Matrix report lost receipt/cache carrier facts"
    }
    if ([int]$trace.schema_version -ne 2 -or
        [string]$trace.runs[2].request_path[1].primary_failure_class -ne "none") {
        throw "Trace analysis did not publish request-level taxonomy v2"
    }
    Write-Output "R7 request observability report passed."
} finally {
    if (Test-Path -LiteralPath $runRoot) {
        Remove-Item -Force -Recurse -LiteralPath $runRoot
    }
}
