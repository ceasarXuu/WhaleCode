$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$runRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-request-observer-$([guid]::NewGuid().ToString('N'))"
. (Join-Path $PSScriptRoot "lib/harness-health.ps1")
. (Join-Path $PSScriptRoot "lib/r7-artifact-provenance.ps1")
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

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
        schema_version = "provider-chat-wire-trace-v10"
        event_name = if ($Index -eq 1) { "provider.chat_wire_shape_recorded" } else { "provider.chat_wire_prefix_broken" }
        request_id = $RequestId
        logical_request_id = "$RequestId-logical"
        attempt_seq = 1
        transport = "responses_http"
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

function New-WireTerminal([string]$RequestId, [int]$InputTokens, [int]$CachedInputTokens) {
    [pscustomobject]@{
        schema_version = "provider-chat-wire-trace-v10"
        event_name = "provider.chat_wire_request_terminal"
        request_id = $RequestId
        logical_request_id = "$RequestId-logical"
        attempt_seq = 1
        transport = "responses_http"
        status = "response_completed"
        input_tokens = $InputTokens
        cached_input_tokens = $CachedInputTokens
    }
}

function New-TokenBoundary([string]$RequestId) {
    [pscustomobject]@{
        type = "event_msg"
        payload = @{
            type = "token_count"
            provider_request_id = $RequestId
            provider_logical_request_id = "$RequestId-logical"
            provider_attempt_seq = 1
        }
    }
}

function New-ObservationRow([string]$LogicalMode, [string]$ArtifactDir, [int]$Requests) {
    [pscustomobject]@{
        observation_status = "complete"
        comparison_eligible = $true
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
    New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
    $repoCommit = ((& git -C $repoRoot rev-parse HEAD) | Select-Object -First 1).Trim()
    $sourceCommit = ((& git -C $repoRoot rev-list -1 HEAD -- third_party/codex-cli) | Select-Object -First 1).Trim()
    $gitIdentity = Get-TaskspaceGitBuildIdentity $repoRoot
    $fixtureBinary = (Get-Process -Id $PID).Path
    $fixtureBinarySha = (Get-FileHash -Algorithm SHA256 -LiteralPath $fixtureBinary).Hash.ToLowerInvariant()
    $attestationPath = Join-Path $runRoot "fixture-binary-attestation.json"
    $probe = Get-TaskspaceWhaleVersionProbe $fixtureBinary
    Write-Json $attestationPath @{
        schema_version = 2
        status = "pass"
        repo_root = $repoRoot
        current_git_head = $repoCommit
        head_tree_id = [string]$gitIdentity.head_tree_id
        codex_tree_id = [string]$gitIdentity.codex_tree_id
        worktree_clean = $true
        codex_source_latest_commit = $sourceCommit
        whale_bin = $fixtureBinary
        whale_binary_sha256 = $fixtureBinarySha
        build_command = "fixture executable identity"
        executable_probe = $probe
    }
    $runs = [Collections.Generic.List[object]]::new()
    foreach ($arm in @("standard", "map-always", "map-append", "map-request")) {
        $logicalMode = if ($arm -eq "standard") { "standard" } else { "taskspace" }
        $runDir = Join-Path $runRoot $arm
        $artifactDir = Join-Path $runDir "artifacts"
        $requests = if ($logicalMode -eq "taskspace") { 2 } else { 1 }
        New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
        $observationRow = New-ObservationRow $logicalMode $artifactDir $requests
        if ($arm -eq "map-always") {
            $observationRow.observation_status = "incomplete"
            $observationRow.comparison_eligible = $false
            $observationRow.result.business_success = $false
            $observationRow.result.agent_completion_status = "interrupted"
        }
        Write-Json (Join-Path $runDir "performance-observation.json") @{
            rows = @($observationRow)
        }
        Write-Json (Join-Path $runDir "pair-001/manifest.resolved.json") @{
            container_image_digest = "sha256:request-observer-test"
        }
        Write-Json (Join-Path $runDir "run-status.json") @{
            phase = "completed"
            run_validity = "valid"
            diagnostic_comparison_enabled = $true
            exit_code = 0
            attempted_pairs = 1
            completed_pairs = 1
            final_aggregate_ready = $false
        }
        Write-Json (Join-Path $runDir "whale-binary-preflight-health.json") @{
            status = "pass"
            run_validity = "valid"
            whale_bin_resolved = $fixtureBinary
            whale_binary_sha256 = $fixtureBinarySha
            current_git_head = $repoCommit
            codex_source_latest_commit = @{ hash = $sourceCommit }
            build_attestation_path = $attestationPath
            build_attestation_status = "pass"
        }
        Write-Json (Join-Path $artifactDir "request-summary.json") @{
            first_input_tokens_per_request = 100
            model_request_count = $requests
        }

        if ($logicalMode -eq "standard") {
            Write-JsonLines (Join-Path $artifactDir "rollout.jsonl") @(
                (New-TokenBoundary "$arm-1")
            )
            Write-JsonLines (Join-Path $artifactDir "provider-wire-trace.jsonl") @(
                (New-WireShape "$arm-1" 1 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }) 0),
                (New-WireTerminal "$arm-1" 100 0)
            )
        } else {
            $controlArgs = '{"action":"initialize_and_execute","root":{"node_id":"root","goal":"task"},"work_nodes":[{"node_id":"work","goal":"work"}],"finish":{"node_id":"finish","goal":"finish"},"edges":[{"from":"root","to":"work"},{"from":"work","to":"finish"}],"actions":[{"node_id":"work","tool":"exec_command"}]}'
            Write-JsonLines (Join-Path $artifactDir "rollout.jsonl") @(
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "$arm-control"; rawPayload = @{ name = "taskspace_control"; arguments = $controlArgs } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call"; callId = "$arm-tool"; rawPayload = @{ name = "exec_command"; arguments = '{"cmd":"true"}' } } },
                (New-TokenBoundary "$arm-1"),
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "$arm-control"; toolSuccess = $true; rawPayload = @{ output = '{"success":true,"state_commit":true}' } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "function_call_output"; callId = "$arm-tool"; toolSuccess = $true; rawPayload = @{ output = "ok" } } },
                @{ type = "event_msg"; payload = @{ type = "map_runtime"; map_event_type = "task_context_event_recorded"; eventType = "message"; originalRole = "developer"; rawPayload = @{ type = "message"; role = "developer"; content = @(@{ type = "input_text"; text = '{"schema_version":"TaskSpaceResponseFinalReceiptV1"}' }) } } },
                (New-TokenBoundary "$arm-2")
            )
            $receiptHash = ("a" * 64) -join ""
            Write-JsonLines (Join-Path $artifactDir "provider-wire-trace.jsonl") @(
                (New-WireShape "$arm-1" 1 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }) 0),
                (New-WireTerminal "$arm-1" 100 0),
                (New-WireShape "$arm-2" 2 @(@{ index = 0; role = "system" }, @{ index = 1; role = "user" }, @{ index = 2; role = "assistant" }, @{ index = 3; role = "tool" }, @{ index = 4; role = "system" }) 2 @(
                    @{ message_index = 4; wire_role = "system"; control_call_id_sha256 = $receiptHash; reservation_revision_after = 1; canonical_revision = 2; revision_delta = 1; complete = $true }
                )),
                (New-WireTerminal "$arm-2" 200 20)
            )
        }
        $evidenceFact = Write-R7RunArtifactEvidenceManifest $runDir $logicalMode
        $runs.Add([pscustomobject]@{
                sample = "fixture"
                repeat = 1
                arm = $arm
                logical_mode = $logicalMode
                projection_policy = if ($logicalMode -eq "taskspace") { $arm.Substring(4) } else { "standard" }
                run_dir = $runDir
                exit_code = 0
                evidence_manifest_path = [string]$evidenceFact.path
                evidence_manifest_sha256 = [string]$evidenceFact.sha256
            })
    }
    Write-Json (Join-Path $runRoot "run-manifest.json") @{
        status = "completed"
        contract_id = "request-observer-test"
        repo_commit = $repoCommit
        whale_sha256 = $fixtureBinarySha
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
    $incomplete = @($summary | Where-Object arm -eq "map-always")[0]
    if ([string]$incomplete.observation_status -ne "incomplete" -or
        [string]$incomplete.comparison_eligible -ne "False" -or
        [string]$incomplete.business_success -ne "False") {
        throw "Matrix report removed or reclassified a validly observed Agent failure"
    }
    $append = @($summary | Where-Object arm -eq "map-append")[0]
    if ([int]$append.receipt_before_requests -ne 1 -or
        [double]$append.receipt_before_cache_hit_rate -ne 0.1 -or
        [string]$append.receipt_wire_roles -ne "system") {
        throw "Matrix report lost receipt/cache carrier facts"
    }
    if ([int]$trace.schema_version -ne 3 -or
        [string]$trace.input_artifact_provenance.status -ne "valid" -or
        [int]$trace.node_state_rejections.request_count -ne 0 -or
        [string]$trace.runs[2].request_path[1].primary_failure_class -ne "none") {
        throw "Trace analysis did not publish request-level taxonomy and provenance"
    }
    $sharedViolations = @(
        [pscustomobject]@{
            code = "node_state_invalid"
            subjects = @("reservation-a")
            node_id = "work"
            canonical_before_transaction = [pscustomobject]@{
                state = "ready"
                unsatisfied_predecessor_ids = @()
            }
            rejected_candidate_at_violation = [pscustomobject]@{
                state = "completed"
                allowed_states = @("ready")
                unsatisfied_predecessor_ids = @("left")
            }
        },
        [pscustomobject]@{
            code = "node_state_invalid"
            subjects = @("reservation-b")
            node_id = "work"
            canonical_before_transaction = [pscustomobject]@{
                state = "ready"
                unsatisfied_predecessor_ids = @("right")
            }
            rejected_candidate_at_violation = [pscustomobject]@{
                state = "completed"
                allowed_states = @("in_flight")
                unsatisfied_predecessor_ids = @()
            }
        },
        [pscustomobject]@{
            code = "node_state_invalid"
            subjects = @("reservation-c")
            node_id = "verify"
            canonical_before_transaction = [pscustomobject]@{
                node_present = $false
                state = ""
            }
            rejected_candidate_at_violation = [pscustomobject]@{
                state = "waiting"
                allowed_states = @("ready", "in_flight")
            }
        }
    )
    $stateSummary = Get-R7NodeStateRejectionSummary @(
        [pscustomobject]@{
            sample = "state-fixture"
            repeat = 1
            arm = "map-request"
            request_path = @(
                [pscustomobject]@{
                    request_index = 1
                    calls = @(
                        [pscustomobject]@{
                            call_id = "state-control"
                            zero_dispatch = $true
                            failure_copy_group_id = "provider_response:state"
                            failure_affected_call_ids = @(
                                "state-control",
                                "state-search"
                            )
                            violation_contexts = $sharedViolations
                        },
                        [pscustomobject]@{
                            call_id = "state-search"
                            zero_dispatch = $true
                            failure_copy_group_id = "provider_response:state"
                            failure_affected_call_ids = @(
                                "state-control",
                                "state-search"
                            )
                            violation_contexts = $sharedViolations
                        }
                    )
                },
                [pscustomobject]@{
                    request_index = 2
                    calls = @(
                        [pscustomobject]@{
                            tool = "taskspace_control"
                            control_action = "read_map"
                        }
                    )
                }
            )
        }
    )
    if ([int]$stateSummary.request_count -ne 1 -or
        [int]$stateSummary.violation_count -ne 3 -or
        [int]$stateSummary.next_read_map_request_count -ne 1 -or
        [int]$stateSummary.by_arm[0].request_count -ne 1 -or
        [int]$stateSummary.by_arm[0].violation_count -ne 3 -or
        [int]$stateSummary.by_arm[0].state_pairs.Count -ne 2 -or
        @(
            $stateSummary.by_arm[0].state_pairs |
                Where-Object {
                    [string]$_.canonical_state -eq "ready" -and
                    [string]$_.candidate_state -eq "completed" -and
                    [int]$_.violation_count -eq 2
                }
        ).Count -ne 1 -or
        @(
            $stateSummary.facts |
                Where-Object {
                    [string]$_.node_id -eq "work" -and
                    @($_.subjects).Count -eq 1
                }
        ).Count -ne 2) {
        throw "Node-state rejection summary did not preserve state pairs and follow-up actions: $($stateSummary | ConvertTo-Json -Compress -Depth 20)"
    }
    $finalProvenancePath = Join-Path $runRoot "artifact-provenance.json"
    $finalProvenance = Get-Content -Raw -Encoding UTF8 -LiteralPath $finalProvenancePath |
        ConvertFrom-Json -Depth 100
    $matrixStatusPath = Join-Path $runRoot "matrix-final-status.json"
    $matrixStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $matrixStatusPath |
        ConvertFrom-Json -Depth 100
    if ([int]$finalProvenance.schema_version -ne 2 -or
        [string]$finalProvenance.phase -ne "final" -or
        [string]$finalProvenance.status -ne "valid" -or
        [int]$finalProvenance.raw_artifact_count -ne 32 -or
        -not [bool]$matrixStatus.final_aggregate_ready) {
        throw "Matrix report did not publish sealed final aggregate provenance"
    }

    $appendRolloutPath = Join-Path $runRoot "map-append/artifacts/rollout.jsonl"
    $appendRollout = [IO.File]::ReadAllText($appendRolloutPath)
    [IO.File]::AppendAllText($appendRolloutPath, "{}$([Environment]::NewLine)")
    $rawMutationRejected = $false
    try {
        & (Join-Path $PSScriptRoot "report-r7-five-layer-matrix.ps1") -RunRoot $runRoot | Out-Null
    } catch {
        $rawMutationRejected = $_.Exception.Message -match "Matrix artifact provenance is invalid"
    }
    $rawMutationProvenance = Get-Content -Raw -Encoding UTF8 -LiteralPath $finalProvenancePath |
        ConvertFrom-Json -Depth 100
    if (-not $rawMutationRejected -or
        "run_evidence_artifact_hash_mismatch" -notin @($rawMutationProvenance.findings.code)) {
        throw "Matrix report accepted a post-seal raw artifact mutation"
    }
    [IO.File]::WriteAllText($appendRolloutPath, $appendRollout, [Text.UTF8Encoding]::new($false))
    & (Join-Path $PSScriptRoot "report-r7-five-layer-matrix.ps1") -RunRoot $runRoot | Out-Null

    $reportPath = Join-Path $runRoot "report.md"
    $reportContent = [IO.File]::ReadAllText($reportPath)
    [IO.File]::AppendAllText($reportPath, "`nmutation")
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runRoot "run-manifest.json") |
        ConvertFrom-Json -Depth 100
    $mutatedFinal = Get-R7MatrixArtifactProvenance `
        $repoRoot `
        (Join-Path $runRoot "run-manifest.json") `
        $manifest `
        (Join-Path $PSScriptRoot "report-r7-five-layer-matrix.ps1") `
        $matrixStatusPath
    if ([string]$mutatedFinal.status -ne "invalid" -or
        "matrix_final_output_hash_mismatch" -notin @($mutatedFinal.findings.code)) {
        throw "Final provenance accepted a mutated aggregate output"
    }
    [IO.File]::WriteAllText($reportPath, $reportContent, [Text.UTF8Encoding]::new($false))

    $invalidAttestation = Get-Content -Raw -Encoding UTF8 -LiteralPath $attestationPath | ConvertFrom-Json
    $invalidAttestation.status = "invalid"
    Write-Json $attestationPath $invalidAttestation
    $provenanceRejected = $false
    try {
        & (Join-Path $PSScriptRoot "report-r7-five-layer-matrix.ps1") -RunRoot $runRoot | Out-Null
    } catch {
        $provenanceRejected = $_.Exception.Message -match "Matrix artifact provenance is invalid"
    }
    $invalidProvenance = Get-Content -Raw -Encoding UTF8 -LiteralPath $finalProvenancePath |
        ConvertFrom-Json -Depth 100
    if (-not $provenanceRejected -or [string]$invalidProvenance.status -ne "invalid") {
        throw "Matrix report accepted an invalid binary attestation"
    }
    Write-Output "R7 request observability report passed."
} finally {
    if (Test-Path -LiteralPath $runRoot) {
        Remove-Item -Force -Recurse -LiteralPath $runRoot
    }
}
