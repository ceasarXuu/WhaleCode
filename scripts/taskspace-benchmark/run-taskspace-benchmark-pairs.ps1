for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
    $existingPairDir = Join-Path $runDir ("pair-{0:000}" -f $repeat); $existingPairReport = Join-Path $existingPairDir "pair-report.md"
    if ($resuming -and -not $ForceRerun -and (Test-Path -LiteralPath $existingPairReport)) {
        $classified = Get-TaskspacePairEvidenceFromArtifacts $existingPairDir $Repeats $promptGuard $EnableAggregate.IsPresent $AuditReviewRoot ([string]$manifest.EvidenceTarget) $probe
        Write-TaskspacePairReport $existingPairReport $manifest $promptGuard $classified.variable_control $classified.evidence $classified.left_metrics $classified.right_metrics $classified.pair $probe
        Write-TaskspaceRunEvent $runDir "pair_resumed_reclassified" @{
            repeat = $repeat; pair_report = $existingPairReport
            reported_evidence_level = [string]$classified.evidence.reported_evidence_level
            included_in_utility_aggregate = [bool]$classified.evidence.included_in_utility_aggregate
            included_in_e3_aggregate = [bool]$classified.evidence.included_in_e3_aggregate
        }
        $pairReports.Add([pscustomobject]@{
                repeat = $repeat; pair_dir = $existingPairDir
                pair_report = $existingPairReport; evidence_target = $manifest.EvidenceTarget
                evidence = $classified.evidence
        })
        Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" $repeat $repeat "" "" "" $existingPairReport $commandLine | Out-Null
        if ($RunSide -eq "both" -and ($ScoringMode -or $RequireScoreValidity) -and $classified.evidence.PSObject.Properties.Name -contains "engineering_unclean" -and [bool]$classified.evidence.engineering_unclean) {
            $abort = Stop-TaskspaceScoringInvalidRun $runDir $manifest.Id $existingPairDir $existingPairReport $classified.evidence $commandLine $repeat $Repeats -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
            Write-Host "RunDir: $runDir"
            Write-Host "PairAbort: $($abort.abort_path)"
            exit 3
        }
        continue
    }
    Set-TaskspaceBenchmarkRunPhase $runDir "execute" ($repeat - 1) ($repeat - 1) | Out-Null
    Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" ($repeat - 1) ($repeat - 1) "" "" "" "" $commandLine | Out-Null
    $pairHealthPath = Join-Path $runDir ("harness-health-pair-{0:000}.json" -f $repeat); $pairHealth = New-TaskspaceDiskHealth @($runDir, $manifest.ScenarioRoot, $manifest.FixtureDir) ("pair_{0:000}_preflight" -f $repeat)
    Write-TaskspaceHarnessHealth $pairHealthPath $pairHealth
    Write-TaskspaceRunEvent $runDir "pair_disk_health_completed" @{ repeat = $repeat; status = [string]$pairHealth.status; path = $pairHealthPath }
    if ([string]$pairHealth.status -eq "fail") {
        $firstFinding = @($pairHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
        $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "pair_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $pairHealthPath
        $abortPath = Join-Path $runDir ("pair-{0:000}-preflight-abort.md" -f $repeat)
        New-TaskspaceHarnessAbortSummaryLines "TaskSpace Pair Preflight Abort" "pair_preflight" $firstFinding $signature $pairHealthPath | Set-Content -LiteralPath $abortPath -Encoding UTF8
        Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "pair_preflight" ([string]$firstFinding.stable_code) $signature $pairHealthPath $commandLine ($repeat - 1) ($repeat - 1) | Out-Null
        Write-Host "RunDir: $runDir"
        Write-Host "HarnessHealth: $pairHealthPath"
        Write-Host "AbortSummary: $abortPath"
        exit 3
    }
    $pairStartedAt = Get-Date; Write-TaskspaceRunEvent $runDir "pair_started" @{ repeat = $repeat }
    try {
        $pair = New-TaskspacePairWorkspace $manifest $runDir $repeat
    } catch {
        $message = [string]$_.Exception.Message
        $stableCode = if ($message -match "workspace_baseline_git_failed|workspace_baseline_dirty") {
            "workspace_baseline_git_failed"
        } elseif ($message -match "workspace_fixture_copy_failed") {
            "workspace_fixture_copy_failed"
        } else {
            "workspace_materialization_failed"
        }
        $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "workspace_materialization" $stableCode $message "" $runDir
        $abortPath = Join-Path $existingPairDir "pair-abort.json"
        Write-TaskspaceJson ([pscustomobject]@{
                abort_scope = "sample"
                abort_phase = "workspace_materialization"
                reason = $stableCode
                infra_signature = $signature
                first_failure_artifact = $abortPath
            }) $abortPath
        Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "workspace_materialization" $stableCode $signature $abortPath $commandLine ($repeat - 1) ($repeat - 1) | Out-Null
        Write-Host "RunDir: $runDir"
        Write-Host "PairAbort: $abortPath"
        exit 3
    }
    foreach ($side in @($pair.Left, $pair.Right)) {
        if (-not (Test-TaskspaceNeutralCwd $side.RepoDir)) {
            throw "Non-neutral cwd for $($side.Name): $($side.RepoDir)"
        }
    }
    $allSides = @($pair.Left, $pair.Right)
    $selectedSides = @($allSides | Where-Object { Test-TaskspaceRunSideSelected ([string]$_.Name) $RunSide })
    $skippedSides = @($allSides | Where-Object { -not (Test-TaskspaceRunSideSelected ([string]$_.Name) $RunSide) })
    Write-TaskspaceRunEvent $runDir "pair_side_selection_completed" @{
        repeat = $repeat
        run_side = $RunSide
        selected_sides = @($selectedSides | ForEach-Object { [string]$_.Name })
        skipped_sides = @($skippedSides | ForEach-Object { [string]$_.Name })
    }
    $probeTimingBySide = @{}
    $probeStatusBySide = @{}
    if ([string]$manifest.EvidenceTarget -eq "E3" -or ($manifest.ExternalBenchmark -and $manifest.ExternalBenchmark.adapter_metadata -and [bool]$manifest.ExternalBenchmark.adapter_metadata.validator_probe_supported)) {
        foreach ($side in $selectedSides) {
            $probeRoot = if ($side.PSObject.Properties.Name -contains "RunnerPrivateDir" -and -not [string]::IsNullOrWhiteSpace([string]$side.RunnerPrivateDir)) {
                New-Dir (Join-Path ([string]$side.RunnerPrivateDir) "vprobe")
            } else {
                New-Dir (Join-Path $pair.PairDir "_runner-private\$($side.Name)\vprobe")
            }
            $probeStdout = Join-Path $probeRoot "validator-probe.stdout.log"
            $probeStderr = Join-Path $probeRoot "validator-probe.stderr.log"
            $probeProofDir = $probeRoot
            $probeStartedAt = Get-Date
            $probeResult = Invoke-TaskspaceDockerValidation $containerRunId $manifest.Id ("pair-{0:000}" -f $repeat) $side $containerImage $containerContract $manifest.PublicValidation ([Math]::Min($ValidationPretestTimeoutSeconds, [Math]::Max(30, $ValidationTimeoutSeconds))) @("-ProbeOnly") $probeRoot
            $probeExit = [int]$probeResult.exit_code
            $probeStdout = [string]$probeResult.stdout_path
            $probeStderr = [string]$probeResult.stderr_path
            $probeFinishedAt = Get-Date
            $probeTimingBySide[$side.Name] = [int64](($probeFinishedAt - $probeStartedAt).TotalMilliseconds)
            $probeValidation = [pscustomobject]@{ exit_code = $probeExit; stdout_path = $probeStdout; stderr_path = $probeStderr }
            $probeLifecycle = Get-TaskspaceValidationLifecycle $probeValidation
            $probeText = Get-TaskspaceValidationText $probeValidation
            $probeSignature = Get-TaskspaceHarnessTextSignature $probeText "probe" $side.Name $probeStderr
            if ($probeExit -ne 0) {
                if ($null -eq $probeSignature) { $probeSignature = New-TaskspaceInfraSignature "harness_materialization_failure" "probe" "validator_probe_failed" "Validator probe failed" $side.Name $probeStderr }
                $abortPath = Join-Path $pair.PairDir "pair-abort.json"
                Write-TaskspaceJson ([pscustomobject]@{
                        abort_scope = "sample"
                        abort_phase = "probe"
                        reason = "validator_probe_failed"
                        infra_signature = $probeSignature
                        first_failure_artifact = $probeStderr
                    }) $abortPath
                Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "probe" "validator_probe_failed" $probeSignature $abortPath $commandLine ($repeat - 1) ($repeat - 1) | Out-Null
                Write-Host "RunDir: $runDir"
                Write-Host "PairAbort: $abortPath"
                exit 3
            }
            if ([bool]$probeLifecycle.tests_started_seen) {
                $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "probe" "validator_probe_failed" "Probe unexpectedly reached tests_started" $side.Name $probeStdout
                $abortPath = Join-Path $pair.PairDir "pair-abort.json"
                Write-TaskspaceJson ([pscustomobject]@{ abort_scope = "sample"; abort_phase = "probe"; reason = "probe_reached_tests"; infra_signature = $signature; first_failure_artifact = $probeStdout }) $abortPath
                Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "probe" "probe_reached_tests" $signature $abortPath $commandLine ($repeat - 1) ($repeat - 1) | Out-Null
                exit 3
            }
            $probeHashSource = "$probeText`n$probeExit"
            $probeHashBytes = [System.Text.Encoding]::UTF8.GetBytes($probeHashSource)
            $probeHash = [System.BitConverter]::ToString([System.Security.Cryptography.SHA256]::Create().ComputeHash($probeHashBytes)).Replace("-", "").ToLowerInvariant()
            $probeStatusBySide[$side.Name] = [pscustomobject]@{
                status = "passed"
                exit_code = $probeExit
                stdout_path = $probeStdout
                stderr_path = $probeStderr
                proof_dir = $probeProofDir
                hash = $probeHash
            }
        }
    }
    $manifestResolved = [ordered]@{
        scenario = $manifest.Id
        repeat = $repeat
        prompt_sha256_left = $promptSha
        prompt_sha256_right = $promptSha
        fixture_sha256_left = $fixtureSha
        fixture_sha256_right = $fixtureSha
        whale_bin_left = (Resolve-Path -LiteralPath $WhaleBin).Path
        whale_bin_right = (Resolve-Path -LiteralPath $WhaleBin).Path
        whale_sha256_left = $whaleSha
        whale_sha256_right = $whaleSha
        whale_version = $whaleVersion
        model_left = $Model
        model_right = $Model
        timeout_seconds_left = $TimeoutSeconds
        timeout_seconds_right = $TimeoutSeconds
        validation_timeout_seconds = $ValidationTimeoutSeconds
        validation_pretest_timeout_seconds = $ValidationPretestTimeoutSeconds
        validation_test_timeout_seconds = $ValidationTestTimeoutSeconds
        docker_image_cache_enabled = [bool]$EnableDockerImageCache
        execution_substrate = "docker"
        container_image_digest = [string]$containerImage.image_digest
        container_base_image = [string]$containerImage.base_image
        container_resource_contract = $containerContract.resources
        provider_param_status = $providerParamStatus
        provider_request_hard_limit = $ProviderRequestHardLimit
        provider_routing = $providerRouting
        config_overrides = @($effectiveConfigOverrides)
        taskspace_projection_policy = $TaskSpaceProjectionPolicy
        stop_on_any_side_failure = [bool]$StopOnAnySideFailure
        sandbox_mode = "docker_hard_boundary"
        oracle_isolation_policy = $OracleIsolationPolicy
        logical_mode_map = @{ left = $pair.Left.LogicalMode; right = $pair.Right.LogicalMode }
        run_side = $RunSide
        selected_sides = @($selectedSides | ForEach-Object { [string]$_.Name })
        skipped_sides = @($skippedSides | ForEach-Object { [string]$_.Name })
        sample_origin = $manifest.SampleOrigin
        external_benchmark = $manifest.ExternalBenchmark
        human_review_required = $manifest.HumanReviewRequired
        e3 = $manifest.E3
    }
    Write-TaskspaceJson $manifestResolved (Join-Path $pair.PairDir "manifest.resolved.json")
    $execBySide = @{}
    $obsBySide = @{}
    $sourceGuard = $null
    try {
        $sourceGuard = Protect-TaskspaceExternalSensitiveSource $manifest $pair.PairDir
        foreach ($side in $selectedSides) {
            $jsonlPath = Join-Path $side.ArtifactDir "whale-exec.jsonl"
            $stderrPath = Join-Path $side.ArtifactDir "whale-exec.stderr.log"
            $lastMessagePath = Join-Path $side.ArtifactDir "last-message.md"
            $processTimingPath = Join-Path $side.ArtifactDir "process-timing.json"
            $stdinPath = Join-Path $side.ArtifactDir "user-prompt.txt"
            $sidePrompt = if ($side.LogicalMode -eq "taskspace") { $taskspacePrompt } else { $prompt }
            Write-Text $stdinPath $sidePrompt
            $executionRepoDir = [string]$containerContract.paths.workspace
            $containerLastMessagePath = Join-Path ([string]$containerContract.paths.artifacts) "last-message.md"
            $args = New-TaskspaceWhaleArgv $side.LogicalMode $Model $executionRepoDir $containerLastMessagePath $effectiveConfigOverrides $TaskSpaceProjectionPolicy
            $commonArgs = Get-TaskspaceCommonArgvWithoutTreatment $args
            $childEnvironment = @{
                WHALE_PROVIDER_WIRE_TRACE_PATH = "/artifacts/provider-wire-trace.jsonl"
            }
            if ($ProviderRequestHardLimit -gt 0) {
                $childEnvironment["WHALE_PROVIDER_BOUNDARY"] = "docker-isolated-proxy-v1"
            }
            if ($side.LogicalMode -eq "taskspace") {
                $childEnvironment["WHALE_TASKSPACE_ROUTE_MODE"] = [string]$routingDecision.recommended_mode
                $childEnvironment["WHALE_TASKSPACE_PROFILE_NAME"] = "taskspace-v005-$($routingDecision.recommended_mode)"
                $childEnvironment["RUST_LOG"] = "error,codex_core::taskspace_exec=info"
            }
            $treatmentDelta = if ($side.LogicalMode -eq "taskspace") {
                @("--taskspace", "-c taskspace_projection_policy=`"$TaskSpaceProjectionPolicy`"")
            } else {
                @()
            }
            Write-TaskspaceJson ([pscustomobject]@{ logical_mode = $side.LogicalMode; argv = @($args); common_argv_without_treatment = @($commonArgs); treatment_delta = $treatmentDelta; execution_substrate = "docker"; container_workdir = $executionRepoDir; child_environment = $childEnvironment; provider_routing = $providerRouting }) (Join-Path $side.ArtifactDir "whale-argv.json")
            $started = Get-Date
            $containerExec = Invoke-TaskspaceDockerAgent $containerRunId $manifest.Id ("pair-{0:000}" -f $repeat) $side $containerImage $containerContract $WhaleBin $args $childEnvironment $env:DEEPSEEK_API_KEY $ProviderRequestHardLimit $Model $TimeoutSeconds
            $exitCode = [int]$containerExec.exit_code
            $timedOut = [bool]$containerExec.timed_out
            $finished = Get-Date
            $threadId = if (Test-Path -LiteralPath $jsonlPath) { Get-ThreadId (Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonlPath) } else { "" }
            $obs = $null
            if ($side.LogicalMode -eq "taskspace") {
                $obs = Export-TaskspaceObservabilityIfAvailable $repoRoot $side.RepoDir $side.ArtifactDir $jsonlPath $started $WhaleBin $threadId
            }
            $execBySide[$side.Name] = [pscustomobject]@{
                exit_code = $exitCode
                wall_time_ms = [int64]$containerExec.wall_time_ms
                timed_out = $timedOut
                jsonl_path = $jsonlPath
                stderr_path = $stderrPath
                last_message_path = $lastMessagePath
                process_timing_path = $processTimingPath
                process_launch_wait_ms = 0
                execution_substrate = "docker"
                container_id = [string]$containerExec.container_id
            }
            $obsBySide[$side.Name] = $obs
        }
    } finally {
        $sourceGuard = Unprotect-TaskspaceExternalSensitiveSource $sourceGuard
    }
    $probe = if ($RunSide -eq "both") {
        Get-TaskspaceDockerOracleIsolationProbe $pair.Left $pair.CanaryPath $pair.CanaryText
    } else {
        Write-TaskspaceRunEvent $runDir "oracle_isolation_probe_skipped" @{ repeat = $repeat; run_side = $RunSide; reason = "side_selection" }
        $null
    }
    Materialize-TaskspacePrivateOracle $pair $manifest
    $metricsBySide = @{}
    $validationTimingBySide = @{}
    foreach ($side in @($pair.Left, $pair.Right)) {
        if (-not (Test-TaskspaceRunSideSelected ([string]$side.Name) $RunSide)) {
            $skipTimestamp = Get-Date
            $metrics = New-TaskspaceSideSelectionSkipMetrics $side $RunSide $skipTimestamp
            $validationTimingBySide[$side.Name] = [pscustomobject]@{
                logical_mode = [string]$side.LogicalMode
                validation_started_at = $skipTimestamp
                validation_finished_at = $skipTimestamp
                validation_exit_code = 0
                validation_skipped = $true
                validation_skip_reason = "side_selection"
                validation_process_launch_wait_ms = 0
                probe_duration_ms = 0
                oracle_started_at = $skipTimestamp
                oracle_finished_at = $skipTimestamp
                oracle_exit_code = 0
                engineering_unclean_reasons = @($metrics.metrics_taints)
            }
            Write-TaskspaceJson $metrics (Join-Path $side.ArtifactDir "metrics.json")
            $metricsBySide[$side.Name] = $metrics
            Write-TaskspaceRunEvent $runDir "side_execution_skipped" @{
                repeat = $repeat
                side = [string]$side.Name
                logical_mode = [string]$side.LogicalMode
                run_side = $RunSide
            }
            continue
        }
        $validationStdout = Join-Path $side.ArtifactDir "validation.stdout.log"
        $validationStderr = Join-Path $side.ArtifactDir "validation.stderr.log"
        $validationProofDir = Join-Path $side.ArtifactDir "vrun"
        $validationProcessTimingPath = Join-Path $validationProofDir "validation-process-timing.json"
        $exec = $execBySide[$side.Name]
        $probeStatus = if ($probeStatusBySide.ContainsKey($side.Name)) { $probeStatusBySide[$side.Name] } else { $null }
        $probePassed = ($probeStatus -and [string]$probeStatus.status -eq "passed" -and [string]$probeStatus.hash -match '^[0-9a-f]{64}$')
        $skipValidationAfterExecTimeout = ($exec -and $exec.PSObject.Properties.Name -contains "timed_out" -and [bool]$exec.timed_out -and $probePassed)
        if ($skipValidationAfterExecTimeout) {
            $validationStartedAt = Get-Date
            $skipRecord = [pscustomobject]@{
                public_validation_skipped = $true
                public_validation_skip_reason = "agent_exec_timeout"
                pre_agent_validator_probe_status = if ($probeStatus) { [string]$probeStatus.status } else { "missing" }
                pre_agent_validator_probe_hash = if ($probeStatus) { [string]$probeStatus.hash } else { "" }
                pre_agent_validator_probe_stdout_path = if ($probeStatus) { [string]$probeStatus.stdout_path } else { "" }
                pre_agent_validator_probe_stderr_path = if ($probeStatus) { [string]$probeStatus.stderr_path } else { "" }
                validation_skip_recorded_at = $validationStartedAt.ToString("o")
            }
            Write-TaskspaceJson $skipRecord (Join-Path $side.ArtifactDir "validation-skip.json")
            Write-Text $validationStdout "public_validation_skipped=true`npublic_validation_skip_reason=agent_exec_timeout`npre_agent_validator_probe_status=$($skipRecord.pre_agent_validator_probe_status)`n"
            Write-Text $validationStderr ""
            $validationExit = 0
            $validationFinishedAt = $validationStartedAt
            $oracleStartedAt = Get-Date
            $oracle = [pscustomobject]@{
                exit_code = 0
                stdout_path = Join-Path $side.ArtifactDir "hidden-oracle.stdout.log"
                stderr_path = Join-Path $side.ArtifactDir "hidden-oracle.stderr.log"
                oracle_isolation_level = "skipped_after_agent_exec_timeout"
            }
            Write-Text $oracle.stdout_path "hidden_oracle_skipped=true`nhidden_oracle_skip_reason=agent_exec_timeout`n"
            Write-Text $oracle.stderr_path ""
            $oracleFinishedAt = $oracleStartedAt
        } else {
            $effectiveValidationTimeout = [Math]::Max(30, $ValidationTimeoutSeconds)
            $validationStartedAt = Get-Date
            $oldDockerImageCache = $env:TASKSPACE_DOCKER_IMAGE_CACHE
            try { if ($EnableDockerImageCache) { $env:TASKSPACE_DOCKER_IMAGE_CACHE = "1" } else { Remove-Item Env:\TASKSPACE_DOCKER_IMAGE_CACHE -ErrorAction SilentlyContinue }
                $validationResult = Invoke-TaskspaceDockerValidation $containerRunId $manifest.Id ("pair-{0:000}" -f $repeat) $side $containerImage $containerContract $manifest.PublicValidation $effectiveValidationTimeout
                $validationExit = [int]$validationResult.exit_code
                $validationStdout = [string]$validationResult.stdout_path
                $validationStderr = [string]$validationResult.stderr_path
            } finally { if ($null -eq $oldDockerImageCache) { Remove-Item Env:\TASKSPACE_DOCKER_IMAGE_CACHE -ErrorAction SilentlyContinue } else { $env:TASKSPACE_DOCKER_IMAGE_CACHE = $oldDockerImageCache } }
            $validationFinishedAt = Get-Date
            $oracleStartedAt = Get-Date
            $oracle = Invoke-TaskspaceDockerOracle $containerRunId $manifest.Id ("pair-{0:000}" -f $repeat) $side $containerImage $containerContract $pair.HiddenOraclePath
            $oracleFinishedAt = Get-Date
        }
        $validation = [pscustomobject]@{ exit_code = $validationExit; stdout_path = $validationStdout; stderr_path = $validationStderr }
        $metrics = Get-TaskspaceBenchmarkMetrics $side $exec $validation $oracle $obsBySide[$side.Name]
        $metrics.invalid_prompt = $promptGuard.invalid_prompt
        $modelTimingPath = Get-TaskspaceModelTimingSourcePath $side.ArtifactDir $exec.jsonl_path
        $modelTiming = Get-TaskspaceModelTimingAttribution $modelTimingPath
        $metrics | Add-Member -NotePropertyName model_queue_wait_ms -NotePropertyValue $modelTiming.model_queue_wait_ms -Force
        $metrics | Add-Member -NotePropertyName model_retry_backoff_ms -NotePropertyValue $modelTiming.model_retry_backoff_ms -Force
        $metrics | Add-Member -NotePropertyName model_request_duration_ms -NotePropertyValue $modelTiming.model_request_duration_ms -Force
        $metrics | Add-Member -NotePropertyName model_timing_event_count -NotePropertyValue $modelTiming.model_timing_event_count -Force
        $metrics | Add-Member -NotePropertyName model_timing_source_status -NotePropertyValue $modelTiming.model_timing_source_status -Force
        $metrics | Add-Member -NotePropertyName model_timing_source_path -NotePropertyValue $modelTiming.model_timing_source_path -Force
        $metrics | Add-Member -NotePropertyName model_timing_parse_errors -NotePropertyValue $modelTiming.model_timing_parse_errors -Force
        $metrics | Add-Member -NotePropertyName process_launch_wait_ms -NotePropertyValue $exec.process_launch_wait_ms -Force
        $metrics | Add-Member -NotePropertyName public_validation_skipped -NotePropertyValue ([bool]$skipValidationAfterExecTimeout) -Force
        $metrics | Add-Member -NotePropertyName public_validation_skip_reason -NotePropertyValue $(if ($skipValidationAfterExecTimeout) { "agent_exec_timeout" } else { "" }) -Force
        $metrics | Add-Member -NotePropertyName pre_agent_validator_probe_status -NotePropertyValue $(if ($probeStatus) { [string]$probeStatus.status } else { "" }) -Force
        $metrics | Add-Member -NotePropertyName pre_agent_validator_probe_hash -NotePropertyValue $(if ($probeStatus) { [string]$probeStatus.hash } else { "" }) -Force
        Set-TaskspaceLifecycleClassification $metrics | Out-Null
        $validationTimingBySide[$side.Name] = [pscustomobject]@{
            logical_mode = [string]$side.LogicalMode
            validation_started_at = $validationStartedAt
            validation_finished_at = $validationFinishedAt
            validation_exit_code = $validationExit
            validation_skipped = [bool]$skipValidationAfterExecTimeout
            validation_skip_reason = if ($skipValidationAfterExecTimeout) { "agent_exec_timeout" } else { "" }
            validation_process_launch_wait_ms = if (Test-Path -LiteralPath $validationProcessTimingPath) {
                try {
                    $validationProcessTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $validationProcessTimingPath | ConvertFrom-Json
                    if ($validationProcessTiming.PSObject.Properties.Name -contains "process_launch_wait_ms") { [int64]$validationProcessTiming.process_launch_wait_ms } else { $null }
                } catch { $null }
            } else { $null }
            probe_duration_ms = if ($probeTimingBySide.ContainsKey($side.Name)) { [int64]$probeTimingBySide[$side.Name] } else { $null }
            oracle_started_at = $oracleStartedAt
            oracle_finished_at = $oracleFinishedAt
            oracle_exit_code = if ($oracle -and $oracle.PSObject.Properties.Name -contains "exit_code") { [int]$oracle.exit_code } else { 0 }
            engineering_unclean_reasons = @($metrics.validator_environment_failures)
        }
        Add-TaskspaceMetricTimingFields $metrics $validationTimingBySide[$side.Name] | Out-Null
        Write-TaskspaceJson $metrics (Join-Path $side.ArtifactDir "metrics.json")
        $metricsBySide[$side.Name] = $metrics
    }
    $taskspaceMetricSide = @($pair.Left, $pair.Right) | Where-Object { $_.LogicalMode -eq "taskspace" } | Select-Object -First 1
    if ($taskspaceMetricSide -and
        $manifest.Expected -and
        $manifest.Expected.PSObject.Properties.Name -contains "min_taskspace_runtime_output_ref_created_count") {
        $taskspaceMetrics = $metricsBySide[$taskspaceMetricSide.Name]
        $expectedOutputRefs = [int]$manifest.Expected.min_taskspace_runtime_output_ref_created_count
        $actualOutputRefs = if ($taskspaceMetrics.PSObject.Properties.Name -contains "runtime_output_ref_created_count") {
            [int]$taskspaceMetrics.runtime_output_ref_created_count
        } else {
            0
        }
        if ($actualOutputRefs -lt $expectedOutputRefs) {
            $existingTaints = @()
            if ($taskspaceMetrics.PSObject.Properties.Name -contains "metrics_taints") {
                $existingTaints = @($taskspaceMetrics.metrics_taints)
            }
            $taint = "scenario_expected_runtime_output_ref_created_count_not_met:$actualOutputRefs<$expectedOutputRefs"
            $taskspaceMetrics | Add-Member -NotePropertyName metrics_taints -NotePropertyValue @(@($existingTaints) + $taint) -Force
            Write-TaskspaceJson $taskspaceMetrics (Join-Path $taskspaceMetricSide.ArtifactDir "metrics.json")
        }
    }
    $variableControl = Compare-TaskspacePairVariables $manifestResolved $metricsBySide["left"] $metricsBySide["right"]
    $externalProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide $sourceGuard
    $oracleLevels = @($metricsBySide["left"].oracle_isolation_level, $metricsBySide["right"].oracle_isolation_level)
    $oracleLevels += $probe.oracle_isolation_level
    $pairOracleLevel = if ($oracleLevels -contains "failed") {
        "failed"
    } elseif ($oracleLevels -contains "soft_denylist") {
        "soft_denylist"
    } elseif ($oracleLevels -contains "hard_deferred_materialization") {
        "hard_deferred_materialization"
    } else {
        "hard_sandbox"
    }
    $businessSuccess = [bool]($metricsBySide["left"].business_success -or $metricsBySide["right"].business_success)
    $metricsTaints = @($metricsBySide.Values | ForEach-Object { @($_.metrics_taints) } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    $environmentFailures = @($metricsBySide.Values | ForEach-Object { @($_.validator_environment_failures) } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    $standardMetrics = @($metricsBySide["left"], $metricsBySide["right"]) | Where-Object { $_.logical_mode -eq "standard" } | Select-Object -First 1
    $taskspaceMetrics = @($metricsBySide["left"], $metricsBySide["right"]) | Where-Object { $_.logical_mode -eq "taskspace" } | Select-Object -First 1
    $sideOutcomes = [pscustomobject]@{
        standard_success = ($standardMetrics -and [bool]$standardMetrics.business_success)
        taskspace_success = ($taskspaceMetrics -and [bool]$taskspaceMetrics.business_success)
        exec_timeouts = @($metricsBySide.Values | Where-Object { $_.PSObject.Properties.Name -contains "exec_timed_out" -and [bool]$_.exec_timed_out } | ForEach-Object { "$($_.mode)/$($_.logical_mode)" })
    }
    $agentLifecycleEligible = @($metricsBySide.Values | Where-Object {
            $_.PSObject.Properties.Name -contains "agent_completion_status" -and [string]$_.agent_completion_status -eq "complete" -and
            $_.PSObject.Properties.Name -contains "sampling_interrupted" -and -not [bool]$_.sampling_interrupted
        }).Count -eq 2
    $e3MinimumRepeats = 5
    if ($null -ne $manifest.E3 -and $manifest.E3.PSObject.Properties.Name -contains "minimum_repeats") {
        $e3MinimumRepeats = [Math]::Max(5, [int]$manifest.E3.minimum_repeats)
    }
    $pairReportPath = Join-Path $pair.PairDir "pair-report.md"
    $candidateEvidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $false $e3MinimumRepeats "" $false $externalProof $sideOutcomes $metricsTaints $environmentFailures $agentLifecycleEligible
    Write-TaskspacePairReport $pairReportPath $manifest $promptGuard $variableControl $candidateEvidence $metricsBySide["left"] $metricsBySide["right"] $pair $probe
    $expectedClaimScope = if ($null -ne $manifest.E3 -and $manifest.E3.PSObject.Properties.Name -contains "claim_scope") { [string]$manifest.E3.claim_scope } else { "" }
    $auditReview = Get-TaskspaceAuditReview $pair.PairDir $AuditReviewRoot $repeat $expectedClaimScope
    $evidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $auditReview.completed $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes $metricsTaints $environmentFailures $agentLifecycleEligible
    $evidence | Add-Member -NotePropertyName audit_review_source_path -NotePropertyValue $auditReview.source_path -Force
    $evidence | Add-Member -NotePropertyName audit_review_failures -NotePropertyValue @($auditReview.failures) -Force
    if ($externalProof) {
        $evidence | Add-Member -NotePropertyName external_runtime_proof_path -NotePropertyValue $externalProof.runtime_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_runner_equivalence_proof_path -NotePropertyValue $externalProof.runner_equivalence_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_isolation_proof_path -NotePropertyValue $externalProof.isolation_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_combined_proof_path -NotePropertyValue $externalProof.combined_proof_path -Force
        $evidence | Add-Member -NotePropertyName external_proof_official_runner_or_equivalent -NotePropertyValue $externalProof.validator_fidelity.official_runner_or_equivalent -Force
        $evidence | Add-Member -NotePropertyName external_proof_agent_cannot_read_validator_source -NotePropertyValue $externalProof.validator_fidelity.agent_cannot_read_validator_source -Force
        $evidence | Add-Member -NotePropertyName external_proof_validator_e3_eligible -NotePropertyValue $externalProof.validator_fidelity.e3_eligible -Force
    }
    $auditManifest = Write-TaskspaceAuditManifest $pair.PairDir $manifestResolved $metricsBySide["left"] $metricsBySide["right"] $evidence $variableControl $auditReview
    $evidence | Add-Member -NotePropertyName audit_manifest_path -NotePropertyValue $auditManifest.json_path -Force
    $evidence | Add-Member -NotePropertyName failure_taxonomy -NotePropertyValue @($auditManifest.failure_taxonomy) -Force
    $evidence | Add-Member -NotePropertyName utility_direction -NotePropertyValue $auditManifest.utility_direction -Force
    $evidence | Add-Member -NotePropertyName run_score_ready -NotePropertyValue ([bool]$auditManifest.run_score_ready) -Force
    $evidence | Add-Member -NotePropertyName run_score_valid -NotePropertyValue ([bool]$auditManifest.run_score_valid) -Force
    $evidence | Add-Member -NotePropertyName audit_required -NotePropertyValue ([bool]$auditManifest.audit_required) -Force
    $evidence | Add-Member -NotePropertyName engineering_unclean -NotePropertyValue ([bool]$auditManifest.engineering_unclean) -Force
    $evidence | Add-Member -NotePropertyName engineering_unclean_reasons -NotePropertyValue @($auditManifest.engineering_unclean_reasons) -Force
    $evidence | Add-Member -NotePropertyName outcome_standard -NotePropertyValue ([string]$auditManifest.outcome_standard) -Force
    $evidence | Add-Member -NotePropertyName outcome_taskspace -NotePropertyValue ([string]$auditManifest.outcome_taskspace) -Force
    $pairFinishedAt = Get-Date
    $pairTimingPath = Write-TaskspacePairTiming -PairDir $pair.PairDir -Repeat $repeat -PairStartedAt $pairStartedAt -PairFinishedAt $pairFinishedAt -Manifest $manifest -Pair $pair -MetricsBySide $metricsBySide -ValidationTimingBySide $validationTimingBySide -EngineeringUncleanReasons @($auditManifest.engineering_unclean_reasons) -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
    $evidence | Add-Member -NotePropertyName pair_timing_path -NotePropertyValue $pairTimingPath -Force
    Write-TaskspacePairReport $pairReportPath $manifest $promptGuard $variableControl $evidence $metricsBySide["left"] $metricsBySide["right"] $pair $probe
    if ([string]$manifest.EvidenceTarget -eq "E3") {
        Write-TaskspaceAuditReviewTemplate $pair.PairDir $expectedClaimScope | Out-Null
        Write-TaskspaceRunEvent $runDir "audit_draft_written" @{ repeat = $repeat; pair_dir = $pair.PairDir }
    }
    $pairReports.Add([pscustomobject]@{ repeat = $repeat; pair_dir = $pair.PairDir; pair_report = $pairReportPath; evidence_target = $manifest.EvidenceTarget; evidence = $evidence })
    Write-TaskspaceRunEvent $runDir "pair_completed" @{ repeat = $repeat; pair_report = $pairReportPath; reported_evidence_level = [string]$evidence.reported_evidence_level }
    Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" $repeat $repeat "" "" "" $pairReportPath $commandLine | Out-Null
    Assert-TaskspaceArtifactStorage ("pair_{0:000}_completed" -f $repeat)
    if ($RunSide -eq "both" -and ($ScoringMode -or $RequireScoreValidity) -and [bool]$auditManifest.engineering_unclean) {
        $abort = Stop-TaskspaceScoringInvalidRun $runDir $manifest.Id $pair.PairDir $pairReportPath $evidence $commandLine $repeat $Repeats -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
        Write-Host "RunDir: $runDir"
        Write-Host "PairAbort: $($abort.abort_path)"
        exit 3
    }
    $stopDecision = Get-TaskspacePairStopDecision ([bool]$StopOnAnySideFailure) @($selectedSides | ForEach-Object { $metricsBySide[[string]$_.Name] })
    if ([bool]$stopDecision.stop) {
        $stopConditionTriggered = $true
        $stopConditionReason = [string]$stopDecision.code
        $stopConditionArtifact = Join-Path $pair.PairDir "pair-stop-condition.json"
        $skippedRepeats = if ($repeat -lt $Repeats) { @(($repeat + 1)..$Repeats) } else { @() }
        Write-TaskspaceJson ([pscustomobject]@{
                schema_version = "taskspace-pair-stop-condition-v1"
                reason = $stopConditionReason
                repeat = $repeat
                failed_sides = @($stopDecision.failed_sides)
                skipped_repeats = $skippedRepeats
            }) $stopConditionArtifact
        Write-TaskspaceRunEvent $runDir "run_stop_condition_triggered" @{
            reason = $stopConditionReason
            repeat = $repeat
            artifact = $stopConditionArtifact
            skipped_repeats = $skippedRepeats
        }
        break
    }
    if ([string]$manifest.EvidenceTarget -eq "E3" -and $repeat -eq 1 -and $Repeats -gt 1) {
        $sentinel = Get-TaskspaceSentinelAbortDecision $standardMetrics $taskspaceMetrics
        Write-TaskspaceRunEvent $runDir "sentinel_pair_completed" @{ repeat = $repeat; abort = [bool]$sentinel.abort; reason = [string]$sentinel.reason }
        if ([bool]$sentinel.abort) {
            $abortPath = Join-Path $pair.PairDir "pair-abort.json"
            Write-TaskspaceJson ([pscustomobject]@{
                    abort_scope = "sample"
                    abort_phase = "sentinel_pair"
                    reason = [string]$sentinel.reason
                    infra_signature = $sentinel.signature
                    first_failure_artifact = if ($sentinel.signature) { [string]$sentinel.signature.artifact } else { $pairReportPath }
                    skipped_repeats = @((($repeat + 1)..$Repeats) | Where-Object { $_ -le $Repeats })
            }) $abortPath
            Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "sentinel_pair" ([string]$sentinel.reason) $sentinel.signature $abortPath $commandLine $repeat $repeat | Out-Null
            $sampleTimingPath = Write-TaskspaceSampleTiming -RunDir $runDir -SampleId $manifest.Id -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
            Write-TaskspaceCostAggregateArtifacts -RootDir $runDir -Scope "sample" | Out-Null
            Write-TaskspaceRuntimeBottleneckReport -TimingPath $sampleTimingPath -ScoreValid $false | Out-Null
            Write-Host "RunDir: $runDir"
            Write-Host "PairAbort: $abortPath"
            exit 3
        }
    }
}
