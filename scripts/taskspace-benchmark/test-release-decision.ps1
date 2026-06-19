param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\release-decision-selftest" }
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { [void]$script:failures.Add($Message) }
}

function Write-Json($Value, [string]$Path) {
    $dir = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-FixtureSha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-FixtureStableObjectHash($Value) {
    $json = $Value | ConvertTo-Json -Depth 30 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function New-FixtureReceiptLines([object[]]$Rows) {
    $previous = ""
    @($Rows | ForEach-Object {
            $row = [ordered]@{}
            foreach ($property in $_.PSObject.Properties) { $row[$property.Name] = $property.Value }
            $row["previous_event_hash"] = $previous
            $row["event_hash"] = Get-FixtureStableObjectHash $row
            $previous = $row["event_hash"]
            [pscustomobject]$row | ConvertTo-Json -Compress -Depth 20
        })
}

function New-FixtureRun([string]$Name, [string]$CostStatus, [bool]$ScoreValid, [int]$RoutingMistakes, [double]$ModelRequestRatio = 1.0, [string[]]$TaskListSamples = @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log"), [bool]$Attested = $false) {
    $dir = Join-Path $RunRoot $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Write-Json ([pscustomobject]@{
            status = $CostStatus
            ratios = [pscustomobject]@{
                direct_input_output_ratio = 1.5
                walltime_ratio = 1.2
                model_request_count_ratio = $ModelRequestRatio
            }
        }) (Join-Path $dir "suite-cost-gate.json")
    Write-Json ([pscustomobject]@{ availability = "measured" }) (Join-Path $dir "token-summary.json")
    Write-Json ([pscustomobject]@{ availability = "measured"; model_request_count = 1 }) (Join-Path $dir "request-summary.json")
    Write-Json ([pscustomobject]@{ availability = "measured" }) (Join-Path $dir "taskspace-control-usage.json")
    Set-Content -LiteralPath (Join-Path $dir "projection-events.jsonl") -Encoding UTF8 -Value "{}"
    ([pscustomobject]@{
        schema_version = "taskspace-output-ref-event-v1"
        source = "observability_timeline"
        kind = "output_ref.created"
        artifact_ref = "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        call_id = "call-fixture"
        timestamp_ms = 1
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "output-ref-events.jsonl") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $dir "compaction-events.jsonl") -Encoding UTF8 -Value "{}"
    ([pscustomobject]@{
        schema_version = "taskspace-provider-request-event-v1"
        request_id = "provider-request-1"
        request_phase = "model_sampling"
        task_id = "task-1"
        map_id = "map-1"
        node_id = "node-1"
        producer = "provider_lifecycle"
        provider_payload_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        status = "completed"
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "provider-request-events.jsonl") -Encoding UTF8
    ([pscustomobject]@{
        schema_version = "taskspace-budget-event-v1"
        status = "pass"
        budget_response_action_taken = $true
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "budget-events.jsonl") -Encoding UTF8
    ([pscustomobject]@{
        schema_version = "taskspace-budget-quality-impact-v1"
        sample_id = "processing-pipeline"
        budget_action = "within_budget"
        final_classification = "solved"
        score_eligible = $true
        missing_evidence_count = 0
        protected_item_miss_count = 0
        manual_override_used = $false
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "budget-quality-impact-events.jsonl") -Encoding UTF8
    Write-Json ([pscustomobject]@{
            budget_quality_impact_logged_for_every_budget_action = $true
            budget_quality_impact_missing_count = 0
            budget_induced_validation_skip_count = 0
            budget_induced_score_ineligible_solved_count = 0
            blocked_by_budget_samples_count = 0
            manual_override_used_count = 0
        }) (Join-Path $dir "budget_induced_quality_impact_summary.json")
    Write-Json ([pscustomobject]@{
            provider_request_hook_coverage = 99
            provider_request_terminal_coverage = 100
            request_phase_attribution_coverage = 95
            unknown_request_phase_ratio = 0
            provider_request_event_count = 1
            provider_request_distinct_count = 1
            provider_request_terminal_count = 1
            expected_model_request_count = 1
        }) (Join-Path $dir "request-phase-summary.json")
    ([pscustomobject]@{
        schema_version = "taskspace-exact-payload-scan-event-v1"
        scan_event_id = "scan-1"
        request_id = "provider-request-1"
        provider_payload_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        passed = $true
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "exact-payload-scan-events.jsonl") -Encoding UTF8
    Write-Json ([pscustomobject]@{
            provider_payload_available = $true
            request_id = "provider-request-1"
            provider_payload_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            exact_payload_scan_passed = $true
            exact_payload_scan_event_id = "scan-1"
            replacement_confirmed = $true
            legacy_taskspace_history_present = $false
            large_raw_output_tokens = 0
            protected_items_present = $true
        }) (Join-Path $dir "active-context-replacement-report.json")
    Write-Json ([pscustomobject]@{
            status = "pass"
            legacy_state_action_count = 0
            legacy_state_action_budget = 0
            state_commit_count = 1
        }) (Join-Path $dir "state-commit-displacement.json")
    Write-Json ([pscustomobject]@{ status = "pass"; spawn_agent_call_count = 0; max_spawn_agent_calls = 0 }) (Join-Path $dir "spawn-node-budget-summary.json")
    $profileHash = "profile-fixture-hash"
    $sourceVersion = "terminal-bench@fixture"
    $taskListHash = "task-list-fixture-hash"
    $sampleSetId = "terminal-bench_E3-P0_3_5"
    $sampleNames = @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log")
    $taskListPath = Join-Path $dir "task-list.jsonl"
    $taskListRows = @($TaskListSamples | ForEach-Object {
            [pscustomobject]@{
                sample_id = $_
                task_dir = (Join-Path $dir "tasks\$_")
                source_version = $sourceVersion
            } | ConvertTo-Json -Compress -Depth 8
        })
    foreach ($sample in $TaskListSamples) {
        New-Item -ItemType Directory -Path (Join-Path $dir "tasks\$sample") -Force | Out-Null
    }
    $taskListRows | Set-Content -LiteralPath $taskListPath -Encoding UTF8
    $runnerScriptSha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    $childRunnerSha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
    $taskListSha256 = Get-FixtureSha256 $taskListPath
    $gateEvidenceDir = Join-Path $dir "non-agent-evidence"
    New-Item -ItemType Directory -Path $gateEvidenceDir -Force | Out-Null
    $gateObject = {
        param([string]$Name)
        $evidencePath = Join-Path $gateEvidenceDir "$Name.txt"
        Set-Content -LiteralPath $evidencePath -Encoding UTF8 -Value "$Name pass"
        [pscustomobject]@{
            status = "pass"
            evidence_path = $evidencePath
            evidence_sha256 = Get-FixtureSha256 $evidencePath
            command = "selftest $Name"
            exit_code = 0
            generated_at = "2026-06-19T00:00:00.0000000Z"
            git_commit = "fixture-head"
            profile_hash = $profileHash
            task_list_hash = $taskListHash
            source_version = $sourceVersion
            producer = "test-release-decision.ps1"
        }
    }
    Write-Json ([pscustomobject]@{
            status = "pass"
            schema_version = 1
            gates = [pscustomobject]@{
                provider_request_hook = (& $gateObject "provider_request_hook")
                runtime_budget_response = (& $gateObject "runtime_budget_response")
                budget_quality_impact = (& $gateObject "budget_quality_impact")
                active_context_replacement = (& $gateObject "active_context_replacement")
                state_commit_displacement = (& $gateObject "state_commit_displacement")
                spawn_node_budget = (& $gateObject "spawn_node_budget")
                request_phase_attribution = (& $gateObject "request_phase_attribution")
                release_decision_fixture = (& $gateObject "release_decision_fixture")
                start_gate_fixture = (& $gateObject "start_gate_fixture")
            }
        }) (Join-Path $dir "v005-non-agent-gates.json")
    $codeCompletePath = Join-Path $dir "v005-code-complete.json"
    Write-Json ([pscustomobject]@{
            status = "pass"
            code_complete = $true
            task_list_hash = $taskListHash
            source_version = $sourceVersion
            profile_hash = $profileHash
            sample_set_id = $sampleSetId
        }) $codeCompletePath
    $userApprovalPath = Join-Path $dir "v005-user-approval.json"
    Write-Json ([pscustomobject]@{
            status = "pass"
            approved_command_category = "full_e3"
            approved_sample_set_id = $sampleSetId
            approval_source = "fixture-user-approval"
            approval_timestamp = (Get-Date).ToString("o")
            task_list_hash = $taskListHash
            source_version = $sourceVersion
            profile_hash = $profileHash
        }) $userApprovalPath
    $codeCompleteSha256 = Get-FixtureSha256 $codeCompletePath
    $approvalSha256 = Get-FixtureSha256 $userApprovalPath
    $suiteManifestPath = Join-Path $dir "suite-manifest.json"
    Write-Json ([pscustomobject]@{
            schema_version = 1
            artifact_origin = "real_suite"
            benchmark = "terminal-bench"
            source_version = $sourceVersion
            repeats = 5
            sample_set_id = $sampleSetId
            runner_entrypoint = "run-taskspace-e3-suite.ps1"
            task_list_path = $taskListPath
            runner_script_sha256 = $runnerScriptSha256
            child_runner_sha256 = $childRunnerSha256
            task_list_hash = $taskListHash
            task_list_sha256 = $taskListSha256
            profile_hash = $profileHash
            sample_set_derivation = [pscustomobject]@{
                formal_p0 = $true
                derivation_method = "task_list_content"
                sample_names = $sampleNames
            }
        }) $suiteManifestPath
    $suiteManifestSha256 = Get-FixtureSha256 $suiteManifestPath
    $suiteReceiptPath = Join-Path $dir "suite-receipt.jsonl"
    New-FixtureReceiptLines @(
        [pscustomobject]@{
            schema_version = 1
            event = "run_initialized"
            sample_set_id = $sampleSetId
            runner_script_sha256 = $runnerScriptSha256
            child_runner_sha256 = $childRunnerSha256
            task_list_sha256 = $taskListSha256
            profile_hash = $profileHash
            timestamp = "2026-06-19T00:00:00.0000000Z"
        },
        [pscustomobject]@{
            schema_version = 1
            event = "sample_scheduled"
            sample_id = "processing-pipeline"
            sample_index = 0
            timestamp = "2026-06-19T00:00:01.0000000Z"
        },
        [pscustomobject]@{
            schema_version = 1
            event = "sample_completed"
            sample_id = "processing-pipeline"
            sample_index = 0
            child_exit = 0
            completed_pairs = 5
            timestamp = "2026-06-19T00:00:02.0000000Z"
        },
        [pscustomobject]@{
            schema_version = 1
            event = "suite_finalized"
            status = "completed"
            exit_code = 0
            timestamp = "2026-06-19T00:00:03.0000000Z"
        }
    ) | Set-Content -LiteralPath $suiteReceiptPath -Encoding UTF8
    $suiteReceiptSha256 = Get-FixtureSha256 $suiteReceiptPath
    $suiteRunnerAttestationPath = Join-Path $dir "suite-runner-attestation.json"
    $suiteRunnerAttestationSha256 = ""
    if ($Attested) {
        Write-Json ([pscustomobject]@{
                schema_version = 1
                artifact_origin = "real_suite_runner"
                runner_entrypoint = "run-taskspace-e3-suite.ps1"
                runner_script_sha256 = $runnerScriptSha256
                child_runner_sha256 = $childRunnerSha256
                task_list_path = $taskListPath
                task_list_sha256 = $taskListSha256
                suite_manifest_sha256 = $suiteManifestSha256
                suite_receipt_sha256 = $suiteReceiptSha256
                profile_hash = $profileHash
                sample_set_id = $sampleSetId
                suite_root = (Split-Path -Parent $dir)
                process_id = $PID
                command_line = "powershell -File scripts\\taskspace-benchmark\\run-taskspace-e3-suite.ps1 -fixture"
                generated_at = "2026-06-19T00:00:04.0000000Z"
            }) $suiteRunnerAttestationPath
        $suiteRunnerAttestationSha256 = Get-FixtureSha256 $suiteRunnerAttestationPath
    }
    Write-Json ([pscustomobject]@{
            schema_version = "TaskShapeRouterV1"
            recommended_mode = "thin"
            status = "report_only"
        }) (Join-Path $dir "routing-decision.json")
    Write-Json ([pscustomobject]@{
            schema_version = "taskspace-cost-diagnostics-v1"
            root_cause = "active_profile_repeats_compact_taskspace_context_across_many_model_turns"
            drivers = @("rollout_request_count_over_partial_budget")
            ratios = [pscustomobject]@{
                rollout_trace_model_request_count_ratio = 18
                uncached_input_ratio = 11.2
                projection_token_share_of_taskspace_input = 0.0087
            }
        }) (Join-Path $dir "cost-diagnostics.json")
    New-Item -ItemType Directory -Path (Join-Path $dir "start-gate") -Force | Out-Null
    $gateDecision = [pscustomobject]@{
        schema_version = 1
        status = "pass"
        next_allowed_command_category = "full_e3"
        full_e3_allowed = $true
        speed_claim_allowed = $true
        calibration_gate_passed = $true
        v005_markers_passed = $true
        task_list_hash = $taskListHash
        source_version = $sourceVersion
        profile_hash = $profileHash
        generated_at = "2026-06-19T00:00:00.0000000Z"
    }
    Write-Json $gateDecision (Join-Path $dir "start-gate\gate-decision.json")
    Write-Json ([pscustomobject]@{
            schema_version = 1
            status = "pass"
            gate_decision = $gateDecision
        }) (Join-Path $dir "start-gate\e3-start-gate.json")
    Write-Json ([pscustomobject]@{
            run_validity = "valid"
            score_valid = $ScoreValid
            both_success = 1
            both_failed = 0
            excluded_pairs = 0
            excluded_by_reason = [pscustomobject]@{}
        }) (Join-Path $dir "aggregate.json")
    Write-Json ([pscustomobject]@{
            taskspace_projection_count = 1
            missing_taskspace_projection_count = 0
            taskspace_projection_protected_miss_count = 0
            active_projection_count = 1
            shadow_projection_count = 0
        }) (Join-Path $dir "context-projection-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            protected_miss_count = 0
        }) (Join-Path $dir "suite-map-management-summary.json")
    Write-Json ([pscustomobject]@{
            availability = "measured"
            routing_mistake_count = $RoutingMistakes
            recommended_mode = "thin"
            router_status = "report_only"
            verification_first_expected_format_count = 0
        }) (Join-Path $dir "suite-routing-summary.json")
    $pairEventRows = New-Object System.Collections.Generic.List[string]
    $pairCount = 15
    for ($pairIndex = 1; $pairIndex -le $pairCount; $pairIndex++) {
        $sampleIndex = [int][Math]::Floor(($pairIndex - 1) / 5)
        $sampleId = $sampleNames[$sampleIndex]
        $sampleRepeatIndex = (($pairIndex - 1) % 5) + 1
        $pairName = "pair-{0:D3}" -f $pairIndex
        Write-Json ([pscustomobject]@{
                logical_mode = "standard"
                large_output_replay_count = 0
                runtime_output_ref_created_count = 0
            }) (Join-Path $dir "$pairName\left\artifacts\metrics.json")
        Write-Json ([pscustomobject]@{
                logical_mode = "taskspace"
                large_output_replay_count = 0
                runtime_output_ref_created_count = 1
            }) (Join-Path $dir "$pairName\right\artifacts\metrics.json")
        ([pscustomobject]@{
            schema_version = "taskspace-output-ref-event-v1"
            source = "observability_timeline"
            kind = "output_ref.created"
            artifact_ref = "output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            call_id = "call-fixture-$pairIndex"
            timestamp_ms = $pairIndex
        } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $dir "$pairName\right\artifacts\output-ref-events.jsonl") -Encoding UTF8
        $pairReportPath = Join-Path $dir "$pairName\pair-report.md"
        Set-Content -LiteralPath $pairReportPath -Encoding UTF8 -Value "# Pair Report $pairIndex"
        [void]$pairEventRows.Add(([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = $pairIndex; sample_id = $sampleId; sample_repeat_index = $sampleRepeatIndex; standard_run_id = "standard-$pairIndex"; taskspace_run_id = "taskspace-$pairIndex"; pair_report = $pairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8))
    }
    Write-Json ([pscustomobject]@{
            schema_version = 1
            evidence_target = "E3"
            run_validity = "valid"
            diagnostic_comparison_enabled = $true
            final_aggregate_ready = $true
            completed_pairs = $pairCount
            sample_set_id = $sampleSetId
            sample_names = $sampleNames
            benchmark_family = "terminal-bench"
            runner_entrypoint = "run-taskspace-e3-suite.ps1"
            runner_profile_hash = $profileHash
            source_version = $sourceVersion
            task_list_hash = $taskListHash
            repeats_per_sample = 5
            artifact_origin = "real_suite"
            runner_script_sha256 = $runnerScriptSha256
            child_runner_sha256 = $childRunnerSha256
            task_list_sha256 = $taskListSha256
            suite_manifest_path = $suiteManifestPath
            suite_manifest_sha256 = $suiteManifestSha256
            suite_receipt_path = $suiteReceiptPath
            suite_receipt_sha256 = $suiteReceiptSha256
            suite_runner_attestation_path = if ($Attested) { $suiteRunnerAttestationPath } else { "" }
            suite_runner_attestation_sha256 = $suiteRunnerAttestationSha256
            approval_marker_sha256 = $approvalSha256
            code_complete_marker_sha256 = $codeCompleteSha256
        }) (Join-Path $dir "run-status.json")
    $eventRows = New-Object System.Collections.Generic.List[string]
    [void]$eventRows.Add(([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 5; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8))
    [void]$eventRows.Add(([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $dir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8))
    foreach ($row in @($pairEventRows.ToArray())) { [void]$eventRows.Add($row) }
    $eventRows.ToArray() | Set-Content -LiteralPath (Join-Path $dir "events.jsonl") -Encoding UTF8
    $dir
}

$passDir = New-FixtureRun "pass" "PASS" $true 0
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $passDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "synthetic PASS fixture did not fail release decision"
$passDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $passDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$passDecision.decision -ne "release_pass") "synthetic PASS fixture incorrectly wrote release_pass decision"
Assert-True (-not [bool]$passDecision.closeable) "synthetic PASS fixture incorrectly wrote closeable=true"
Assert-True (@($passDecision.blockers) -contains "suite_runner_attestation_gate_failed") "synthetic PASS fixture did not report attestation blocker"
Assert-True ([string]$passDecision.task_list_identity_source -eq "derived_from_task_list") "PASS fixture did not derive task list identity"
Assert-True ([bool]$passDecision.task_list_derivation_gate_pass) "PASS fixture did not pass task list derivation"
Assert-True ([bool]$passDecision.formal_p0_cost_clean_pass) "PASS fixture did not pass formal P0 clean cost gate"

$attestedSyntheticPassDir = New-FixtureRun "attested-synthetic-pass" "PASS" $true 0 1 @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log") $true
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $attestedSyntheticPassDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "attested synthetic PASS fixture did not fail release decision"
$attestedSyntheticPassDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $attestedSyntheticPassDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($attestedSyntheticPassDecision.blockers) -contains "suite_runner_attestation_gate_failed") "attested synthetic PASS fixture did not report attestation blocker"
Assert-True ([string]$attestedSyntheticPassDecision.decision -ne "release_pass") "attested synthetic PASS fixture incorrectly wrote release_pass"

$requestRatioDir = New-FixtureRun "request-ratio-high" "PASS" $true 0 10
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $requestRatioDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "request ratio fixture did not fail release decision"
$requestRatioDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $requestRatioDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($requestRatioDecision.blockers) -contains "formal_p0_request_ratio_gate_failed") "request ratio fixture did not report request ratio blocker"
Assert-True ([string]$requestRatioDecision.decision -ne "release_pass") "request ratio fixture incorrectly wrote release_pass"

$wrongTaskListDir = New-FixtureRun "wrong-task-list" "PASS" $true 0 1 @("processing-pipeline", "multi-source-data-merger", "hello-world")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $wrongTaskListDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "wrong task list fixture did not fail release decision"
$wrongTaskListDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $wrongTaskListDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($wrongTaskListDecision.blockers) -contains "formal_e3_task_list_derivation_failed") "wrong task list fixture did not report task-list derivation blocker"
Assert-True ([string]$wrongTaskListDecision.decision -ne "release_pass") "wrong task list fixture incorrectly wrote release_pass"

$syntheticOriginDir = New-FixtureRun "synthetic-origin" "PASS" $true 0
$syntheticRunStatusPath = Join-Path $syntheticOriginDir "run-status.json"
$syntheticRunStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $syntheticRunStatusPath | ConvertFrom-Json
$syntheticRunStatus.artifact_origin = "fixture_test"
Write-Json $syntheticRunStatus $syntheticRunStatusPath
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $syntheticOriginDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "synthetic origin fixture did not fail release decision"
$syntheticDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $syntheticOriginDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($syntheticDecision.blockers) -contains "formal_e3_provenance_gate_failed") "synthetic origin fixture did not report provenance blocker"
Assert-True ([string]$syntheticDecision.decision -ne "release_pass") "synthetic origin fixture incorrectly wrote release_pass"

$missingSuiteManifestDir = New-FixtureRun "missing-suite-manifest" "PASS" $true 0
$missingSuiteManifestRunStatusPath = Join-Path $missingSuiteManifestDir "run-status.json"
$missingSuiteManifestRunStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $missingSuiteManifestRunStatusPath | ConvertFrom-Json
Remove-Item -LiteralPath (Join-Path $missingSuiteManifestDir "suite-manifest.json") -Force
$missingSuiteManifestRunStatus.suite_manifest_path = ""
$missingSuiteManifestRunStatus.suite_manifest_sha256 = ""
Write-Json $missingSuiteManifestRunStatus $missingSuiteManifestRunStatusPath
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingSuiteManifestDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing suite manifest fixture did not fail release decision"
$missingSuiteManifestDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingSuiteManifestDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingSuiteManifestDecision.blockers) -contains "formal_e3_provenance_gate_failed") "missing suite manifest fixture did not report provenance blocker"

$badSuiteManifestHashDir = New-FixtureRun "bad-suite-manifest-hash" "PASS" $true 0
$badSuiteManifestRunStatusPath = Join-Path $badSuiteManifestHashDir "run-status.json"
$badSuiteManifestRunStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $badSuiteManifestRunStatusPath | ConvertFrom-Json
$badSuiteManifestRunStatus.suite_manifest_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
Write-Json $badSuiteManifestRunStatus $badSuiteManifestRunStatusPath
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $badSuiteManifestHashDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "bad suite manifest hash fixture did not fail release decision"
$badSuiteManifestDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $badSuiteManifestHashDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($badSuiteManifestDecision.blockers) -contains "formal_e3_provenance_gate_failed") "bad suite manifest hash fixture did not report provenance blocker"

$missingReceiptDir = New-FixtureRun "missing-suite-receipt" "PASS" $true 0
$missingReceiptRunStatusPath = Join-Path $missingReceiptDir "run-status.json"
$missingReceiptRunStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $missingReceiptRunStatusPath | ConvertFrom-Json
Remove-Item -LiteralPath (Join-Path $missingReceiptDir "suite-receipt.jsonl") -Force
$missingReceiptRunStatus.suite_receipt_path = ""
$missingReceiptRunStatus.suite_receipt_sha256 = ""
Write-Json $missingReceiptRunStatus $missingReceiptRunStatusPath
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingReceiptDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing suite receipt fixture did not fail release decision"
$missingReceiptDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingReceiptDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingReceiptDecision.blockers) -contains "suite_receipt_gate_failed") "missing suite receipt fixture did not report receipt blocker"

$brokenReceiptDir = New-FixtureRun "broken-suite-receipt-chain" "PASS" $true 0
$brokenReceiptPath = Join-Path $brokenReceiptDir "suite-receipt.jsonl"
$brokenReceiptRows = @(Get-Content -Encoding UTF8 -LiteralPath $brokenReceiptPath)
$brokenReceiptEvent = $brokenReceiptRows[1] | ConvertFrom-Json
$brokenReceiptEvent.previous_event_hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
$brokenReceiptRows[1] = ($brokenReceiptEvent | ConvertTo-Json -Compress -Depth 20)
$brokenReceiptRows | Set-Content -LiteralPath $brokenReceiptPath -Encoding UTF8
$brokenStatusPath = Join-Path $brokenReceiptDir "run-status.json"
$brokenStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $brokenStatusPath | ConvertFrom-Json
$brokenStatus.suite_receipt_sha256 = Get-FixtureSha256 $brokenReceiptPath
Write-Json $brokenStatus $brokenStatusPath
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $brokenReceiptDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "broken suite receipt chain fixture did not fail release decision"
$brokenReceiptDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $brokenReceiptDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($brokenReceiptDecision.blockers) -contains "suite_receipt_hash_chain_failed") "broken suite receipt chain fixture did not report hash-chain blocker"
Assert-True ([string]$brokenReceiptDecision.decision -ne "release_pass") "broken suite receipt chain fixture incorrectly wrote release_pass"

$badEvidenceHashDir = New-FixtureRun "bad-evidence-hash" "PASS" $true 0
$badEvidencePath = Join-Path $badEvidenceHashDir "non-agent-evidence\provider_request_hook.txt"
Set-Content -LiteralPath $badEvidencePath -Encoding UTF8 -Value "tampered evidence"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $badEvidenceHashDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "bad evidence hash fixture did not fail release decision"
$badEvidenceDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $badEvidenceHashDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($badEvidenceDecision.blockers) -contains "v005_non_agent_gates_failed") "bad evidence hash fixture did not report v005 non-agent blocker"
Assert-True ([string]$badEvidenceDecision.decision -ne "release_pass") "bad evidence hash fixture incorrectly wrote release_pass"

$partialDir = New-FixtureRun "partial" "PARTIAL" $true 0 1 @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log") $true
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $partialDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "synthetic PARTIAL fixture did not fail release decision"
$partialDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $partialDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$partialDecision.decision -ne "release_pass") "synthetic PARTIAL fixture incorrectly wrote release_pass decision"
Assert-True (@($partialDecision.blockers) -contains "suite_runner_attestation_gate_failed") "synthetic PARTIAL fixture did not report attestation blocker"
$partialMd = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $partialDir "release-decision.md")
Assert-True ($partialMd.Contains("suite_runner_attestation_gate_failed")) "synthetic PARTIAL markdown did not include attestation blocker"

$failDir = New-FixtureRun "fail" "FAIL" $true 1
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $failDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "FAIL fixture did not exit 1"
$failDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $failDir "release-decision.json") | ConvertFrom-Json
Assert-True ([string]$failDecision.decision -eq "fail") "FAIL fixture did not write fail decision"
Assert-True (-not [bool]$failDecision.closeable) "FAIL fixture incorrectly wrote closeable=true"
Assert-True (@($failDecision.blockers) -contains "cost_gate_failed") "FAIL fixture did not report cost blocker"
Assert-True (@($failDecision.blockers) -contains "routing_gate_failed") "FAIL fixture did not report routing blocker"
Assert-True ([string]$failDecision.cost_root_cause -eq "active_profile_repeats_compact_taskspace_context_across_many_model_turns") "FAIL fixture did not preserve cost root cause"
$failMd = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $failDir "release-decision.md")
Assert-True ($failMd.Contains("rollout_trace_model_request_count_ratio: 18")) "FAIL markdown did not include cost diagnostics"

$missingArtifactDir = New-FixtureRun "missing-artifact" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingArtifactDir "output-ref-events.jsonl") -Destination (Join-Path $missingArtifactDir "output-ref-events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingArtifactDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing artifact fixture did not exit 1"
$missingArtifactDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingArtifactDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingArtifactDecision.blockers) -contains "required_artifact_missing:output-ref-events.jsonl") "missing artifact fixture did not report required artifact blocker"

$missingProviderEventDir = New-FixtureRun "missing-provider-event" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingProviderEventDir "provider-request-events.jsonl") -Destination (Join-Path $missingProviderEventDir "provider-request-events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProviderEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provider event fixture did not exit 1"
$missingProviderEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProviderEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingProviderEventDecision.blockers) -contains "provider_request_event_missing") "missing provider event fixture did not report provider blocker"

$requestCoverageDir = New-FixtureRun "provider-request-coverage-gap" "PASS" $true 0
Write-Json ([pscustomobject]@{
        provider_request_hook_coverage = 50
        provider_request_terminal_coverage = 50
        request_phase_attribution_coverage = 95
        unknown_request_phase_ratio = 0
        provider_request_event_count = 1
        provider_request_distinct_count = 1
        provider_request_terminal_count = 1
        expected_model_request_count = 2
    }) (Join-Path $requestCoverageDir "request-phase-summary.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $requestCoverageDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "provider request coverage gap fixture did not exit 1"
$requestCoverageDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $requestCoverageDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($requestCoverageDecision.blockers) -contains "request_phase_attribution_missing") "provider request coverage gap fixture did not report request phase blocker"

$qualityImpactSkipDir = New-FixtureRun "budget-quality-validation-skip" "PASS" $true 0
Write-Json ([pscustomobject]@{
        budget_quality_impact_logged_for_every_budget_action = $true
        budget_quality_impact_missing_count = 0
        budget_induced_validation_skip_count = 1
        budget_induced_score_ineligible_solved_count = 0
        blocked_by_budget_samples_count = 0
        manual_override_used_count = 0
    }) (Join-Path $qualityImpactSkipDir "budget_induced_quality_impact_summary.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $qualityImpactSkipDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "budget quality validation skip fixture did not exit 1"
$qualityImpactSkipDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $qualityImpactSkipDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($qualityImpactSkipDecision.blockers) -contains "budget_quality_impact_gate_failed") "validation skip fixture did not report budget quality blocker"

$qualityImpactMismatchDir = New-FixtureRun "budget-quality-summary-mismatch" "PASS" $true 0
([pscustomobject]@{
        schema_version = "taskspace-budget-quality-impact-v1"
        sample_id = "processing-pipeline"
        budget_action = "hard_stop"
        final_classification = "blocked_by_budget"
        score_eligible = $false
        missing_evidence_count = 1
        protected_item_miss_count = 0
        manual_override_used = $false
    } | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $qualityImpactMismatchDir "budget-quality-impact-events.jsonl") -Encoding UTF8
Write-Json ([pscustomobject]@{
        budget_quality_impact_logged_for_every_budget_action = $true
        budget_quality_impact_missing_count = 0
        budget_induced_validation_skip_count = 0
        budget_induced_score_ineligible_solved_count = 0
        blocked_by_budget_samples_count = 0
        manual_override_used_count = 0
    }) (Join-Path $qualityImpactMismatchDir "budget_induced_quality_impact_summary.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $qualityImpactMismatchDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "budget quality summary mismatch fixture did not exit 1"
$qualityImpactMismatchDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $qualityImpactMismatchDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($qualityImpactMismatchDecision.blockers) -contains "budget_quality_impact_gate_failed") "summary mismatch fixture did not report budget quality blocker"
Assert-True (-not [bool]$qualityImpactMismatchDecision.budget_quality_impact_summary_matches_events) "summary mismatch fixture incorrectly matched derived event counts"
Assert-True ([int]$qualityImpactMismatchDecision.derived_blocked_by_budget_count -eq 1) "summary mismatch fixture did not derive blocked_by_budget from events"

$qualityImpactMissingDir = New-FixtureRun "missing-budget-quality-impact" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $qualityImpactMissingDir "budget-quality-impact-events.jsonl") -Destination (Join-Path $qualityImpactMissingDir "budget-quality-impact-events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $qualityImpactMissingDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing budget quality event fixture did not exit 1"
$qualityImpactMissingDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $qualityImpactMissingDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($qualityImpactMissingDecision.blockers) -contains "required_artifact_missing:budget-quality-impact-events.jsonl") "missing quality event fixture did not report required artifact blocker"
Assert-True (@($qualityImpactMissingDecision.blockers) -contains "budget_quality_impact_gate_failed") "missing quality event fixture did not report budget quality blocker"

$hashOnlyReplacementDir = New-FixtureRun "hash-only-active-replacement" "PASS" $true 0
$hashOnlyReplacement = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $hashOnlyReplacementDir "active-context-replacement-report.json") | ConvertFrom-Json
$hashOnlyReplacement.exact_payload_scan_passed = $false
$hashOnlyReplacement.exact_payload_scan_event_id = ""
$hashOnlyReplacement | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $hashOnlyReplacementDir "active-context-replacement-report.json") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $hashOnlyReplacementDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "hash-only active replacement fixture did not exit 1"
$hashOnlyReplacementDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $hashOnlyReplacementDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($hashOnlyReplacementDecision.blockers) -contains "active_context_replacement_gate_failed") "hash-only fixture did not report active replacement blocker"

$mismatchedScanDir = New-FixtureRun "mismatched-exact-scan" "PASS" $true 0
$scanRows = Get-Content -Encoding UTF8 -LiteralPath (Join-Path $mismatchedScanDir "exact-payload-scan-events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json }
$scanRows[0].provider_payload_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
($scanRows[0] | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $mismatchedScanDir "exact-payload-scan-events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $mismatchedScanDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "mismatched exact scan fixture did not exit 1"
$mismatchedScanDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $mismatchedScanDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($mismatchedScanDecision.blockers) -contains "active_context_replacement_gate_failed") "mismatched scan fixture did not report active replacement blocker"

$missingProviderPayloadJoinDir = New-FixtureRun "missing-provider-payload-join" "PASS" $true 0
$providerRows = Get-Content -Encoding UTF8 -LiteralPath (Join-Path $missingProviderPayloadJoinDir "provider-request-events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json }
$providerRows[0].provider_payload_sha256 = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
($providerRows[0] | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $missingProviderPayloadJoinDir "provider-request-events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProviderPayloadJoinDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provider payload join fixture did not exit 1"
$missingProviderPayloadJoinDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProviderPayloadJoinDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingProviderPayloadJoinDecision.blockers) -contains "active_context_replacement_gate_failed") "missing provider payload join fixture did not report active replacement blocker"
Assert-True ([int]$missingProviderPayloadJoinDecision.exact_payload_scan_matching_provider_event_count -eq 0) "missing provider payload join fixture still matched provider events"

$missingProviderProducerDir = New-FixtureRun "missing-provider-producer" "PASS" $true 0
$providerRows = Get-Content -Encoding UTF8 -LiteralPath (Join-Path $missingProviderProducerDir "provider-request-events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json }
$providerRows[0].PSObject.Properties.Remove("producer")
($providerRows[0] | ConvertTo-Json -Compress -Depth 8) | Set-Content -LiteralPath (Join-Path $missingProviderProducerDir "provider-request-events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProviderProducerDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provider producer fixture did not exit 1"
$missingProviderProducerDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProviderProducerDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingProviderProducerDecision.blockers) -contains "provider_request_event_missing") "missing provider producer fixture did not report provider request blocker"
Assert-True (@($missingProviderProducerDecision.blockers) -contains "active_context_replacement_gate_failed") "missing provider producer fixture did not break active replacement join"

$weakNonAgentDir = New-FixtureRun "weak-non-agent-gates" "PASS" $true 0
Write-Json ([pscustomobject]@{ status = "pass"; schema_version = 1 }) (Join-Path $weakNonAgentDir "v005-non-agent-gates.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $weakNonAgentDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "weak non-agent gates fixture did not exit 1"
$weakNonAgentDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $weakNonAgentDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($weakNonAgentDecision.blockers) -contains "v005_non_agent_gates_failed") "weak non-agent gates fixture did not report v005 gate blocker"

$missingApprovalTimestampDir = New-FixtureRun "missing-approval-timestamp" "PASS" $true 0
$missingApprovalMarker = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingApprovalTimestampDir "v005-user-approval.json") | ConvertFrom-Json
$missingApprovalMarker.PSObject.Properties.Remove("approval_timestamp")
$missingApprovalMarker | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $missingApprovalTimestampDir "v005-user-approval.json") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingApprovalTimestampDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing approval timestamp fixture did not exit 1"
$missingApprovalTimestampDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingApprovalTimestampDir "release-decision.json") | ConvertFrom-Json
Assert-True (@($missingApprovalTimestampDecision.blockers) -contains "v005_user_approval_marker_failed") "missing approval timestamp fixture did not report approval marker blocker"

$emptyOutputRefDir = New-FixtureRun "empty-output-ref-events" "PASS" $true 0
Set-Content -LiteralPath (Join-Path $emptyOutputRefDir "output-ref-events.jsonl") -Encoding UTF8 -Value "{}"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $emptyOutputRefDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "empty output-ref fixture did not exit 1"
$emptyOutputRefDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $emptyOutputRefDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$emptyOutputRefDecision.output_ref_gate_pass) "empty output-ref fixture incorrectly passed output-ref gate"

$missingMetricsDir = New-FixtureRun "missing-taskspace-metrics" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingMetricsDir "pair-001\right\artifacts\metrics.json") -Destination (Join-Path $missingMetricsDir "pair-001\right\artifacts\metrics.json.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingMetricsDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing taskspace metrics fixture did not exit 1"
$missingMetricsDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingMetricsDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingMetricsDecision.run_provenance_gate_pass) "missing metrics fixture incorrectly passed provenance gate"

$missingStandardMetricsDir = New-FixtureRun "missing-standard-metrics" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingStandardMetricsDir "pair-001\left\artifacts\metrics.json") -Destination (Join-Path $missingStandardMetricsDir "pair-001\left\artifacts\metrics.json.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingStandardMetricsDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing standard metrics fixture did not exit 1"
$missingStandardMetricsDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingStandardMetricsDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingStandardMetricsDecision.run_provenance_gate_pass) "missing standard metrics fixture incorrectly passed provenance gate"

$missingProvenanceDir = New-FixtureRun "missing-provenance" "PASS" $true 0
Move-Item -LiteralPath (Join-Path $missingProvenanceDir "events.jsonl") -Destination (Join-Path $missingProvenanceDir "events.jsonl.bak") -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $missingProvenanceDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "missing provenance fixture did not exit 1"
$missingProvenanceDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $missingProvenanceDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$missingProvenanceDecision.run_provenance_gate_pass) "missing provenance fixture incorrectly passed provenance gate"

$minimalPairEventDir = New-FixtureRun "minimal-pair-event" "PASS" $true 0
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $minimalPairEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $minimalPairEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "minimal pair event fixture did not exit 1"
$minimalPairEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $minimalPairEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$minimalPairEventDecision.run_provenance_gate_pass) "minimal pair event fixture incorrectly passed provenance gate"

$minimalRoutingEventDir = New-FixtureRun "minimal-routing-event" "PASS" $true 0
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = (Join-Path $minimalRoutingEventDir "pair-001\pair-report.md"); reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $minimalRoutingEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $minimalRoutingEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "minimal routing event fixture did not exit 1"
$minimalRoutingEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $minimalRoutingEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$minimalRoutingEventDecision.run_provenance_gate_pass) "minimal routing event fixture incorrectly passed provenance gate"

$duplicatePairEventDir = New-FixtureRun "duplicate-pair-event" "PASS" $true 0
Write-Json ([pscustomobject]@{
        schema_version = 1
        evidence_target = "E3"
        run_validity = "valid"
        diagnostic_comparison_enabled = $true
        final_aggregate_ready = $true
        completed_pairs = 2
    }) (Join-Path $duplicatePairEventDir "run-status.json")
$duplicatePairReportPath = Join-Path $duplicatePairEventDir "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 2; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $duplicatePairEventDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $duplicatePairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:03.0000000Z"; repeat = 2; pair_report = $duplicatePairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $duplicatePairEventDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $duplicatePairEventDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "duplicate pair event fixture did not exit 1"
$duplicatePairEventDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $duplicatePairEventDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$duplicatePairEventDecision.run_provenance_gate_pass) "duplicate pair event fixture incorrectly passed provenance gate"

$nestedPairDir = New-FixtureRun "nested-pair-dir" "PASS" $true 0
$nestedPairRoot = Join-Path $nestedPairDir "backup"
New-Item -ItemType Directory -Path $nestedPairRoot -Force | Out-Null
Move-Item -LiteralPath (Join-Path $nestedPairDir "pair-001") -Destination (Join-Path $nestedPairRoot "pair-001") -Force
$nestedPairReportPath = Join-Path $nestedPairRoot "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $nestedPairDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $nestedPairReportPath; reported_evidence_level = "E3" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $nestedPairDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $nestedPairDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "nested pair dir fixture did not exit 1"
$nestedPairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $nestedPairDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$nestedPairDecision.run_provenance_gate_pass) "nested pair dir fixture incorrectly passed provenance gate"

$nonE3PairDir = New-FixtureRun "non-e3-pair-level" "PASS" $true 0
$nonE3PairReportPath = Join-Path $nonE3PairDir "pair-001\pair-report.md"
@(
    ([pscustomobject]@{ event = "run_initialized"; schema_version = 1; timestamp = "2026-06-18T00:00:00.0000000Z"; scenario_id = "fixture"; repeats = 1; evidence_target = "E3" } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:01.0000000Z"; mode = "thin"; confidence = "high"; status = "report_only"; path = (Join-Path $nonE3PairDir "routing-decision.json") } | ConvertTo-Json -Compress -Depth 8),
    ([pscustomobject]@{ event = "pair_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:02.0000000Z"; repeat = 1; pair_report = $nonE3PairReportPath; reported_evidence_level = "E1" } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $nonE3PairDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $nonE3PairDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "non-E3 pair level fixture did not exit 1"
$nonE3PairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $nonE3PairDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$nonE3PairDecision.run_provenance_gate_pass) "non-E3 pair level fixture incorrectly passed provenance gate"

$sameSamplePairDir = New-FixtureRun "same-sample-pair-ledger" "PASS" $true 0
$sameSampleEvents = Get-Content -Encoding UTF8 -LiteralPath (Join-Path $sameSamplePairDir "events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json }
foreach ($event in @($sameSampleEvents | Where-Object { [string]$_.event -eq "pair_completed" })) {
    $event.sample_id = "processing-pipeline"
    $event.sample_repeat_index = [int]$event.repeat
    if ([int]$event.sample_repeat_index -gt 5) { $event.sample_repeat_index = 5 }
}
@($sameSampleEvents | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 8 }) | Set-Content -LiteralPath (Join-Path $sameSamplePairDir "events.jsonl") -Encoding UTF8
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $sameSamplePairDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "same sample pair ledger fixture did not exit 1"
$sameSamplePairDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $sameSamplePairDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$sameSamplePairDecision.formal_e3_pair_sample_ledger_pass) "same sample pair ledger fixture incorrectly passed sample ledger gate"
Assert-True (-not [bool]$sameSamplePairDecision.run_provenance_gate_pass) "same sample pair ledger fixture incorrectly passed provenance gate"

$staleRoutingDir = New-FixtureRun "stale-routing-event" "PASS" $true 0
Add-Content -LiteralPath (Join-Path $staleRoutingDir "events.jsonl") -Encoding UTF8 -Value (([pscustomobject]@{ event = "routing_decision_completed"; schema_version = 1; timestamp = "2026-06-18T00:00:03.0000000Z" } | ConvertTo-Json -Compress -Depth 8))
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $staleRoutingDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "stale routing event fixture did not exit 1"
$staleRoutingDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $staleRoutingDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$staleRoutingDecision.run_provenance_gate_pass) "stale routing event fixture incorrectly passed provenance gate"

$uncorrelatedOutputRefDir = New-FixtureRun "uncorrelated-output-ref" "PASS" $true 0
Set-Content -LiteralPath (Join-Path $uncorrelatedOutputRefDir "pair-001\right\artifacts\output-ref-events.jsonl") -Encoding UTF8 -Value "{}"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $uncorrelatedOutputRefDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "uncorrelated output-ref fixture did not exit 1"
$uncorrelatedOutputRefDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $uncorrelatedOutputRefDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$uncorrelatedOutputRefDecision.output_ref_gate_pass) "uncorrelated output-ref fixture incorrectly passed output-ref gate"

$stringFalseDir = New-FixtureRun "string-false-quality" "PASS" $true 0
Write-Json ([pscustomobject]@{
        run_validity = "valid"
        score_valid = "false"
        both_success = 1
        both_failed = 0
        excluded_pairs = 0
        excluded_by_reason = [pscustomobject]@{}
    }) (Join-Path $stringFalseDir "aggregate.json")
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-release-decision.ps1") -RunDir $stringFalseDir *> $null
Assert-True ($LASTEXITCODE -eq 1) "string false quality fixture did not exit 1"
$stringFalseDecision = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stringFalseDir "release-decision.json") | ConvertFrom-Json
Assert-True (-not [bool]$stringFalseDecision.quality_gate_pass) "string false quality fixture incorrectly passed quality gate"

if ($failures.Count -gt 0) {
    Write-Error ("Release decision self-test failed: " + (@($failures.ToArray()) -join "; "))
    exit 1
}
Write-Host "Release decision self-test: PASS"
Write-Host "RunRoot: $RunRoot"
