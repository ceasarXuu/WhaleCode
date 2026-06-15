param(
    [string]$Scenario = "single-file-fast-fix",
    [string]$ScenarioPath = "",
    [int]$Repeats = 1,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900, [int]$ValidationTimeoutSeconds = 420, [int]$ValidationPretestTimeoutSeconds = 120, [int]$ValidationTestTimeoutSeconds = 420,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [ValidateSet("deferred_materialization_allowed", "hard_sandbox_only")]
    [string]$OracleIsolationPolicy = "deferred_materialization_allowed",
    [string]$AuditReviewRoot = "",
    [switch]$EnableAggregate, [switch]$AllowNonE2Result, [switch]$ScoringMode, [switch]$RequireScoreValidity, [switch]$EnableDockerImageCache,
    [switch]$ResumeLatest,
    [string]$RunId = "",
    [string]$TaskListHash = "",
    [string]$SourceVersion = "",
    [string]$ProfileHash = "",
    [switch]$ForceRerun,
    [switch]$PlanOnly
)
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\bootstrap.ps1") -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot
if ($Repeats -lt 1) { throw "Repeats must be >= 1" }
if (-not $RunRoot) { $RunRoot = Get-NeutralTaskspaceBenchmarkRunRoot $repoRoot }
$manifest = Read-TaskspaceScenarioManifest $repoRoot $Scenario $ScenarioPath
$prompt = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifest.PromptPath
$promptGuardConfig = $manifest.PromptGuard
$allowedContextTerms = @()
$sourceSpans = @()
if ($null -ne $promptGuardConfig) {
    if ($promptGuardConfig.PSObject.Properties.Name -contains "allowed_domain_terms") {
        $allowedContextTerms = @($promptGuardConfig.allowed_domain_terms | ForEach-Object { [string]$_ })
    }
    if ($promptGuardConfig.PSObject.Properties.Name -contains "allowed_domain_regex") {
        $allowedContextTerms += @($promptGuardConfig.allowed_domain_regex | ForEach-Object { [string]$_ })
    }
    if ($promptGuardConfig.PSObject.Properties.Name -contains "source_spans") {
        $sourceSpans = @($promptGuardConfig.source_spans)
    }
}
$promptGuard = Invoke-TaskspacePromptGuard -PromptText $prompt -AllowedContextTerms $allowedContextTerms -SourceSpans $sourceSpans
if ($promptGuard.invalid_prompt) {
    throw "Scenario prompt leaks internal TaskSpace concepts: $(@($promptGuard.hard_hits) -join ', ')"
}
$commandLine = if ($MyInvocation.Line) { [string]$MyInvocation.Line } else { [Environment]::CommandLine }
$runDir = ""
if (-not [string]::IsNullOrWhiteSpace($RunId)) {
    $runDir = Join-Path (Join-Path $RunRoot $manifest.Id) $RunId
} elseif ($ResumeLatest) {
    $runDir = Find-TaskspaceLatestRunDir $RunRoot $manifest.Id
}
$resuming = (-not [string]::IsNullOrWhiteSpace($runDir) -and (Test-Path -LiteralPath $runDir) -and -not $ForceRerun)
if (-not $resuming) {
    $runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id
    Initialize-TaskspaceBenchmarkRunState $runDir $manifest.Id $Repeats $manifest.EvidenceTarget $commandLine | Out-Null
    Set-TaskspaceSampleStatus $runDir $manifest.Id "preflight" 0 0 "" "" "" "" $commandLine | Out-Null
} else {
    $existingStatus = Read-TaskspaceRunStatus $runDir
    $stale = Test-TaskspaceRunLockStale $existingStatus
    Write-TaskspaceRunEvent $runDir "resume_requested" @{ stale_lock = $stale; command_line = $commandLine }
    if ($existingStatus -and $existingStatus.PSObject.Properties.Name -contains "run_validity" -and [string]$existingStatus.run_validity -eq "invalid_harness" -and -not $ForceRerun) {
        throw "Run is invalid_harness and cannot be resumed without -ForceRerun: $runDir"
    }
    if (-not $stale -and $existingStatus.phase -notin @("completed", "ineligible")) {
        throw "Run appears active and is not stale: $runDir"
    }
    if ($stale) { Write-TaskspaceRunEvent $runDir "stale_lock_reclaimed" @{ previous_lock_owner = [string]$existingStatus.lock_owner } }
}
$promptCopy = Join-Path $runDir "prompt.txt"
Write-Text $promptCopy $prompt
Write-TaskspaceJson $promptGuard (Join-Path $runDir "prompt-guard.json")
Write-TaskspaceRunEvent $runDir "prompt_guard_completed" @{ invalid_prompt = [bool]$promptGuard.invalid_prompt; manual_review_required = [bool]$promptGuard.manual_review_required }
$whaleVersion = if (Test-Path -LiteralPath $WhaleBin) { (& $WhaleBin --version 2>&1) -join " " } else { "" }
$whaleSha = if (Test-Path -LiteralPath $WhaleBin) { Get-TaskspaceFileSha256 $WhaleBin } else { "" }
$fixtureSha = Get-TaskspaceDirectorySha256 $manifest.FixtureDir
$promptSha = Get-TaskspaceFileSha256 $manifest.PromptPath
$requiredProviderParams = @("model", "model_reasoning_effort", "sandbox_mode")
$providerParamStatus = [ordered]@{
    complete = $true
    required = $requiredProviderParams
    explicit = [ordered]@{
        model = $Model
        model_reasoning_effort = ""
        sandbox_mode = $SandboxMode
    }
    missing = @()
}
foreach ($override in @($ConfigOverride)) {
    if ($override -match '^model_reasoning_effort=') {
        $providerParamStatus.explicit.model_reasoning_effort = ($override -replace '^model_reasoning_effort=', '').Trim('"')
    }
}
if ([string]::IsNullOrWhiteSpace($providerParamStatus.explicit.model_reasoning_effort)) {
    $providerParamStatus.complete = $false
    $providerParamStatus.missing = @("model_reasoning_effort")
}
if ($PlanOnly) {
    Write-Host "RunDir: $runDir"
    Write-Host "PromptInvalid: $($promptGuard.invalid_prompt)"
    Write-Host "PromptManualReview: $($promptGuard.manual_review_required)"
    exit 0
}
$remoteAssets = @()
if ($null -ne $manifest.ExternalBenchmark -and
    $manifest.ExternalBenchmark.PSObject.Properties.Name -contains "adapter_metadata" -and
    $null -ne $manifest.ExternalBenchmark.adapter_metadata -and
    $manifest.ExternalBenchmark.adapter_metadata.PSObject.Properties.Name -contains "remote_assets") {
    $remoteAssets = @($manifest.ExternalBenchmark.adapter_metadata.remote_assets)
}
$remoteAssetIneligible = @($remoteAssets | Where-Object { $_.required_for_e3 -and -not [bool]$_.equivalence_proven })
if ($remoteAssets.Count -gt 0) {
    $assetPreflightPath = Join-Path $runDir "preflight.remote-assets.json"
    Write-TaskspaceJson ([pscustomobject]@{
            remote_assets = @($remoteAssets)
            ineligible_assets = @($remoteAssetIneligible)
            e3_eligible = ($remoteAssetIneligible.Count -eq 0)
        }) $assetPreflightPath
    Write-TaskspaceRunEvent $runDir "remote_asset_preflight_completed" @{ remote_asset_count = $remoteAssets.Count; ineligible_asset_count = $remoteAssetIneligible.Count; path = $assetPreflightPath }
    if ($remoteAssetIneligible.Count -gt 0) {
        Set-TaskspaceSampleStatus $runDir $manifest.Id "ineligible" 0 0 "environment_remote_asset_unavailable" "remote_asset_equivalence_unproven" "" $assetPreflightPath $commandLine | Out-Null
        Set-TaskspaceBenchmarkRunPhase $runDir "ineligible" 0 0 $false | Out-Null
        Write-Host "RunDir: $runDir"
        Write-Host "SampleStatus: $(Join-Path $runDir 'sample-status.json')"
        Write-Host "RemoteAssetPreflight: $assetPreflightPath"
        exit 2
    }
}
$harnessHealthPath = Join-Path $runDir "harness-health.json"
$harnessHealth = Get-TaskspaceHarnessHealth $manifest $runDir $manifest.ScenarioRoot
Write-TaskspaceHarnessHealth $harnessHealthPath $harnessHealth
Write-TaskspaceRunEvent $runDir "harness_health_completed" @{ status = [string]$harnessHealth.status; path = $harnessHealthPath }
if ([string]$harnessHealth.status -eq "fail") {
    $firstFinding = @($harnessHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $harnessHealthPath
    $abortPath = Join-Path $runDir "abort-summary.md"
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace Harness Abort" "preflight" $firstFinding $signature $harnessHealthPath | Set-Content -LiteralPath $abortPath -Encoding UTF8
    Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "preflight" ([string]$firstFinding.stable_code) $signature $harnessHealthPath $commandLine 0 0 | Out-Null
    Write-Host "RunDir: $runDir"
    Write-Host "HarnessHealth: $harnessHealthPath"
    Write-Host "AbortSummary: $abortPath"
    exit 3
}
if (-not (Test-Path -LiteralPath $WhaleBin)) { throw "Whale binary not found: $WhaleBin" }
$helpText = & $WhaleBin exec --help 2>&1
if (($helpText -join [Environment]::NewLine) -notmatch "--taskspace") {
    throw "Whale exec does not expose --taskspace."
}

$pairReports = New-Object System.Collections.Generic.List[object]
$probe = $null
for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
    $existingPairDir = Join-Path $runDir ("pair-{0:000}" -f $repeat)
    $existingPairReport = Join-Path $existingPairDir "pair-report.md"
    if ($resuming -and -not $ForceRerun -and (Test-Path -LiteralPath $existingPairReport)) {
        $classified = Get-TaskspacePairEvidenceFromArtifacts $existingPairDir $Repeats $promptGuard $EnableAggregate.IsPresent $AuditReviewRoot ([string]$manifest.EvidenceTarget) $probe
        Write-TaskspacePairReport $existingPairReport $manifest $promptGuard $classified.variable_control $classified.evidence $classified.left_metrics $classified.right_metrics $classified.pair $probe
        Write-TaskspaceRunEvent $runDir "pair_resumed_reclassified" @{
            repeat = $repeat
            pair_report = $existingPairReport
            reported_evidence_level = [string]$classified.evidence.reported_evidence_level
            included_in_utility_aggregate = [bool]$classified.evidence.included_in_utility_aggregate
            included_in_e3_aggregate = [bool]$classified.evidence.included_in_e3_aggregate
        }
        $pairReports.Add([pscustomobject]@{
                repeat = $repeat
                pair_dir = $existingPairDir
                pair_report = $existingPairReport
                evidence_target = $manifest.EvidenceTarget
                evidence = $classified.evidence
        })
        Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" $repeat $repeat "" "" "" $existingPairReport $commandLine | Out-Null
        if (($ScoringMode -or $RequireScoreValidity) -and $classified.evidence.PSObject.Properties.Name -contains "engineering_unclean" -and [bool]$classified.evidence.engineering_unclean) {
        $abort = Stop-TaskspaceScoringInvalidRun $runDir $manifest.Id $existingPairDir $existingPairReport $classified.evidence $commandLine $repeat $Repeats -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
            Write-Host "RunDir: $runDir"
            Write-Host "PairAbort: $($abort.abort_path)"
            exit 3
        }
        continue
    }
    Set-TaskspaceBenchmarkRunPhase $runDir "execute" ($repeat - 1) ($repeat - 1) | Out-Null
    Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" ($repeat - 1) ($repeat - 1) "" "" "" "" $commandLine | Out-Null
    $pairHealthPath = Join-Path $runDir ("harness-health-pair-{0:000}.json" -f $repeat)
    $pairHealth = New-TaskspaceDiskHealth @($runDir, $manifest.ScenarioRoot, $manifest.FixtureDir) ("pair_{0:000}_preflight" -f $repeat)
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
    $pairStartedAt = Get-Date
    Write-TaskspaceRunEvent $runDir "pair_started" @{ repeat = $repeat }
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
    $probeTimingBySide = @{}
    $probeStatusBySide = @{}
    if ([string]$manifest.EvidenceTarget -eq "E3" -or ($manifest.ExternalBenchmark -and $manifest.ExternalBenchmark.adapter_metadata -and [bool]$manifest.ExternalBenchmark.adapter_metadata.validator_probe_supported)) {
        foreach ($side in @($pair.Left, $pair.Right)) {
            $probeStdout = Join-Path $side.ArtifactDir "validator-probe.stdout.log"
            $probeStderr = Join-Path $side.ArtifactDir "validator-probe.stderr.log"
            $probeProofDir = Join-Path $side.ArtifactDir "vprobe"
            $probeStartedAt = Get-Date
            $probeExit = Invoke-TaskspaceValidationCommand $side.RepoDir $manifest.PublicValidation $probeStdout $probeStderr ([Math]::Min($ValidationPretestTimeoutSeconds, [Math]::Max(30, $ValidationTimeoutSeconds))) $probeProofDir @("-ProbeOnly")
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
        provider_param_status = $providerParamStatus
        config_overrides = @($ConfigOverride)
        sandbox_mode = $SandboxMode
        oracle_isolation_policy = $OracleIsolationPolicy
        logical_mode_map = @{ left = $pair.Left.LogicalMode; right = $pair.Right.LogicalMode }
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
        foreach ($side in @($pair.Left, $pair.Right)) {
            $jsonlPath = Join-Path $side.ArtifactDir "whale-exec.jsonl"
            $stderrPath = Join-Path $side.ArtifactDir "whale-exec.stderr.log"
            $lastMessagePath = Join-Path $side.ArtifactDir "last-message.md"
            $processTimingPath = Join-Path $side.ArtifactDir "process-timing.json"
            $stdinPath = Join-Path $side.ArtifactDir "user-prompt.txt"
            Write-Text $stdinPath $prompt
            $mount = $null
            try {
                $mount = Mount-TaskspaceExecutionAlias $side
                $executionRepoDir = [string]$mount.execution_repo_dir
                $args = New-TaskspaceWhaleArgv $side.LogicalMode $Model $executionRepoDir $lastMessagePath $SandboxMode $ConfigOverride
                $commonArgs = @($args | Where-Object { $_ -ne "--taskspace" })
                Write-TaskspaceJson ([pscustomobject]@{ logical_mode = $side.LogicalMode; argv = @($args); common_argv_without_treatment = @($commonArgs); treatment_delta = @("--taskspace"); execution_alias = $mount }) (Join-Path $side.ArtifactDir "whale-argv.json")
                $started = Get-Date
                $timedOut = $false
                try {
                    $exitCode = Invoke-RealProcess $WhaleBin $args $executionRepoDir $jsonlPath $stderrPath $TimeoutSeconds $stdinPath $processTimingPath
                } catch {
                    if ([string]$_.Exception.Message -notmatch "^Process timed out after ") { throw }
                    $exitCode = 124
                    $timedOut = $true
                    if (-not (Test-Path -LiteralPath $jsonlPath)) { Write-Text $jsonlPath "" }
                    $timeoutText = "Process timed out after $TimeoutSeconds seconds: $WhaleBin $($args -join ' ')`n$($_.Exception.Message)"
                    Write-Text $stderrPath $timeoutText
                }
                $finished = Get-Date
            } finally {
                Dismount-TaskspaceExecutionAlias $mount
            }
            $threadId = if (Test-Path -LiteralPath $jsonlPath) { Get-ThreadId (Get-Content -Raw -Encoding UTF8 -LiteralPath $jsonlPath) } else { "" }
            $obs = $null
            if ($side.LogicalMode -eq "taskspace") {
                $obs = Export-TaskspaceObservabilityIfAvailable $repoRoot $side.RepoDir $side.ArtifactDir $jsonlPath $started $threadId
            }
            $execBySide[$side.Name] = [pscustomobject]@{
                exit_code = $exitCode
                wall_time_ms = [int64](($finished - $started).TotalMilliseconds)
                timed_out = $timedOut
                jsonl_path = $jsonlPath
                stderr_path = $stderrPath
                last_message_path = $lastMessagePath
                process_timing_path = $processTimingPath
                process_launch_wait_ms = if (Test-Path -LiteralPath $processTimingPath) {
                    try {
                        $processTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath $processTimingPath | ConvertFrom-Json
                        if ($processTiming.PSObject.Properties.Name -contains "process_launch_wait_ms") { [int64]$processTiming.process_launch_wait_ms } else { $null }
                    } catch { $null }
                } else { $null }
            }
            $obsBySide[$side.Name] = $obs
        }
    } finally {
        $sourceGuard = Unprotect-TaskspaceExternalSensitiveSource $sourceGuard
    }
    $probe = Invoke-TaskspaceOracleIsolationProbe $WhaleBin $pair.Left.RepoDir $pair.PairDir $pair.CanaryPath $pair.CanaryText $Model $SandboxMode $ConfigOverride 180
    Materialize-TaskspacePrivateOracle $pair $manifest
    $metricsBySide = @{}
    $validationTimingBySide = @{}
    foreach ($side in @($pair.Left, $pair.Right)) {
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
                $validationExit = Invoke-TaskspaceValidationCommand $side.RepoDir $manifest.PublicValidation $validationStdout $validationStderr $effectiveValidationTimeout $validationProofDir @() $ValidationPretestTimeoutSeconds $ValidationTestTimeoutSeconds
            } finally { if ($null -eq $oldDockerImageCache) { Remove-Item Env:\TASKSPACE_DOCKER_IMAGE_CACHE -ErrorAction SilentlyContinue } else { $env:TASKSPACE_DOCKER_IMAGE_CACHE = $oldDockerImageCache } }
            $validationFinishedAt = Get-Date
            $oracleStartedAt = Get-Date
            $oracle = Invoke-TaskspaceHiddenOracle $side.RepoDir $side.ArtifactDir $pair.HiddenOraclePath "" -BypassSandbox:($SandboxMode -eq "bypass")
            $oracleFinishedAt = Get-Date
        }
        $validation = [pscustomobject]@{ exit_code = $validationExit; stdout_path = $validationStdout; stderr_path = $validationStderr }
        $metrics = Get-TaskspaceBenchmarkMetrics $side $exec $validation $oracle $obsBySide[$side.Name]
        $metrics.invalid_prompt = $promptGuard.invalid_prompt
        $modelTiming = Get-TaskspaceModelTimingAttribution $exec.jsonl_path
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
    $e3MinimumRepeats = 5
    if ($null -ne $manifest.E3 -and $manifest.E3.PSObject.Properties.Name -contains "minimum_repeats") {
        $e3MinimumRepeats = [Math]::Max(5, [int]$manifest.E3.minimum_repeats)
    }
    $pairReportPath = Join-Path $pair.PairDir "pair-report.md"
    $candidateEvidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $false $e3MinimumRepeats "" $false $externalProof $sideOutcomes $metricsTaints $environmentFailures
    Write-TaskspacePairReport $pairReportPath $manifest $promptGuard $variableControl $candidateEvidence $metricsBySide["left"] $metricsBySide["right"] $pair $probe
    $expectedClaimScope = if ($null -ne $manifest.E3 -and $manifest.E3.PSObject.Properties.Name -contains "claim_scope") { [string]$manifest.E3.claim_scope } else { "" }
    $auditReview = Get-TaskspaceAuditReview $pair.PairDir $AuditReviewRoot $repeat $expectedClaimScope
    $evidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $auditReview.completed $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes $metricsTaints $environmentFailures
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
    if (($ScoringMode -or $RequireScoreValidity) -and [bool]$auditManifest.engineering_unclean) {
        $abort = Stop-TaskspaceScoringInvalidRun $runDir $manifest.Id $pair.PairDir $pairReportPath $evidence $commandLine $repeat $Repeats -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
        Write-Host "RunDir: $runDir"
        Write-Host "PairAbort: $($abort.abort_path)"
        exit 3
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
            Write-TaskspaceRuntimeBottleneckReport -TimingPath $sampleTimingPath -ScoreValid $false | Out-Null
            Write-Host "RunDir: $runDir"
            Write-Host "PairAbort: $abortPath"
            exit 3
        }
    }
}

$runSummaryPath = Join-Path $runDir "run-summary.md"
Write-TaskspaceRunSummary -Path $runSummaryPath -Reports @($pairReports.ToArray())
$sampleTimingPath = Write-TaskspaceSampleTiming -RunDir $runDir -SampleId $manifest.Id -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
if ($EnableAggregate) {
    $aggregatePath = Join-Path $runDir "aggregate-report.md"
    Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @($pairReports.ToArray())
    $auditPending = @($pairReports.ToArray() | Where-Object { @($_.evidence.e3_gate_failures) -contains "e3_human_review_not_completed" }).Count -gt 0
    $finalPhase = if ($auditPending) { "audit_required" } else { "finalize" }
    Set-TaskspaceSampleStatus $runDir $manifest.Id $finalPhase $Repeats $Repeats "" "" $aggregatePath $runSummaryPath $commandLine | Out-Null
} else {
    Set-TaskspaceSampleStatus $runDir $manifest.Id "completed" $Repeats $Repeats "" "" "" $runSummaryPath $commandLine | Out-Null
}
$runFinalReady = $EnableAggregate.IsPresent -and -not ($manifest.EvidenceTarget -eq "E3" -and @($pairReports.ToArray() | Where-Object { @($_.evidence.e3_gate_failures) -contains "e3_human_review_not_completed" }).Count -gt 0)
Set-TaskspaceBenchmarkRunPhase $runDir "completed" $Repeats $Repeats $runFinalReady | Out-Null
Write-Host "RunDir: $runDir"
Write-Host "RunSummary: $runSummaryPath"
Write-Host "SampleTiming: $sampleTimingPath"
foreach ($report in $pairReports) { Write-Host "PairReport: $($report.pair_report)" }

$failedPairs = @(Get-TaskspaceFailedReports $pairReports ([string]$manifest.EvidenceTarget))
if ($failedPairs.Count -gt 0 -and -not $AllowNonE2Result) { exit 1 }
