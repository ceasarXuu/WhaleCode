param(
    [string]$Scenario = "single-file-fast-fix",
    [string]$ScenarioPath = "",
    [int]$Repeats = 1,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [int]$ValidationTimeoutSeconds = 420,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [ValidateSet("deferred_materialization_allowed", "hard_sandbox_only")]
    [string]$OracleIsolationPolicy = "deferred_materialization_allowed",
    [string]$AuditReviewRoot = "",
    [switch]$EnableAggregate,
    [switch]$AllowNonE2Result,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $repoRoot "scripts\action-map-graph-health-lib.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\prompt-guard.ps1")
. (Join-Path $PSScriptRoot "lib\workspace.ps1")
. (Join-Path $PSScriptRoot "lib\oracle-runner.ps1")
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")
. (Join-Path $PSScriptRoot "lib\audit-report.ps1")
. (Join-Path $PSScriptRoot "lib\e3-proof.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\source-guard.ps1")

if ($Repeats -lt 1) { throw "Repeats must be >= 1" }
if (-not $RunRoot) { $RunRoot = Get-NeutralTaskspaceBenchmarkRunRoot $repoRoot }

$manifest = Read-TaskspaceScenarioManifest $repoRoot $Scenario $ScenarioPath
$prompt = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifest.PromptPath
$promptGuard = Invoke-TaskspacePromptGuard $prompt
if ($promptGuard.invalid_prompt) {
    throw "Scenario prompt leaks internal TaskSpace concepts: $(@($promptGuard.hard_hits) -join ', ')"
}

$runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id
$promptCopy = Join-Path $runDir "prompt.txt"
Write-Text $promptCopy $prompt
Write-TaskspaceJson $promptGuard (Join-Path $runDir "prompt-guard.json")
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
if (-not (Test-Path -LiteralPath $WhaleBin)) { throw "Whale binary not found: $WhaleBin" }
$helpText = & $WhaleBin exec --help 2>&1
if (($helpText -join [Environment]::NewLine) -notmatch "--taskspace") {
    throw "Whale exec does not expose --taskspace."
}

$pairReports = New-Object System.Collections.Generic.List[object]
for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
    $pair = New-TaskspacePairWorkspace $manifest $runDir $repeat
    foreach ($side in @($pair.Left, $pair.Right)) {
        if (-not (Test-TaskspaceNeutralCwd $side.RepoDir)) {
            throw "Non-neutral cwd for $($side.Name): $($side.RepoDir)"
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
                    $exitCode = Invoke-RealProcess $WhaleBin $args $executionRepoDir $jsonlPath $stderrPath $TimeoutSeconds $stdinPath
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
            }
            $obsBySide[$side.Name] = $obs
        }
    } finally {
        $sourceGuard = Unprotect-TaskspaceExternalSensitiveSource $sourceGuard
    }
    $probe = Invoke-TaskspaceOracleIsolationProbe $WhaleBin $pair.Left.RepoDir $pair.PairDir $pair.CanaryPath $pair.CanaryText $Model $SandboxMode $ConfigOverride 180
    Materialize-TaskspacePrivateOracle $pair $manifest

    $metricsBySide = @{}
    foreach ($side in @($pair.Left, $pair.Right)) {
        $validationStdout = Join-Path $side.ArtifactDir "validation.stdout.log"
        $validationStderr = Join-Path $side.ArtifactDir "validation.stderr.log"
        $validationProofDir = Join-Path $side.ArtifactDir "external-validator-runtime"
        $effectiveValidationTimeout = [Math]::Max(30, $ValidationTimeoutSeconds)
        $validationExit = Invoke-TaskspaceValidationCommand $side.RepoDir $manifest.PublicValidation $validationStdout $validationStderr $effectiveValidationTimeout $validationProofDir
        $oracle = Invoke-TaskspaceHiddenOracle $side.RepoDir $side.ArtifactDir $pair.HiddenOraclePath "" -BypassSandbox:($SandboxMode -eq "bypass")
        $exec = $execBySide[$side.Name]
        $validation = [pscustomobject]@{ exit_code = $validationExit; stdout_path = $validationStdout; stderr_path = $validationStderr }
        $metrics = Get-TaskspaceBenchmarkMetrics $side $exec $validation $oracle $obsBySide[$side.Name]
        $metrics.invalid_prompt = $promptGuard.invalid_prompt
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
    $candidateEvidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $false $e3MinimumRepeats "" $false $externalProof $sideOutcomes
    Write-TaskspacePairReport $pairReportPath $manifest $promptGuard $variableControl $candidateEvidence $metricsBySide["left"] $metricsBySide["right"] $pair $probe
    $expectedClaimScope = if ($null -ne $manifest.E3 -and $manifest.E3.PSObject.Properties.Name -contains "claim_scope") { [string]$manifest.E3.claim_scope } else { "" }
    $auditReview = Get-TaskspaceAuditReview $pair.PairDir $AuditReviewRoot $repeat $expectedClaimScope
    $evidence = Get-TaskspaceEvidenceGate $Repeats $promptGuard $pairOracleLevel $manifestResolved.provider_param_status $variableControl.invalid_pair $businessSuccess $false $EnableAggregate $OracleIsolationPolicy $manifest.EvidenceTarget $manifest.SampleOrigin $manifest.ExternalBenchmark $manifest.E3 $manifest.HumanReviewRequired $auditReview.completed $e3MinimumRepeats $auditReview.decision $auditReview.disagreement $externalProof $sideOutcomes
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
    Write-TaskspacePairReport $pairReportPath $manifest $promptGuard $variableControl $evidence $metricsBySide["left"] $metricsBySide["right"] $pair $probe
    if ([string]$manifest.EvidenceTarget -eq "E3") {
        Write-TaskspaceAuditReviewTemplate $pair.PairDir $expectedClaimScope | Out-Null
    }
    $pairReports.Add([pscustomobject]@{ repeat = $repeat; pair_dir = $pair.PairDir; pair_report = $pairReportPath; evidence_target = $manifest.EvidenceTarget; evidence = $evidence })
}

$runSummaryPath = Join-Path $runDir "run-summary.md"
Write-TaskspaceRunSummary -Path $runSummaryPath -Reports @($pairReports.ToArray())
if ($EnableAggregate) {
    Write-TaskspaceAggregateReport -Path (Join-Path $runDir "aggregate-report.md") -Reports @($pairReports.ToArray())
}
Write-Host "RunDir: $runDir"
Write-Host "RunSummary: $runSummaryPath"
foreach ($report in $pairReports) { Write-Host "PairReport: $($report.pair_report)" }

$failedPairs = @(Get-TaskspaceFailedReports $pairReports ([string]$manifest.EvidenceTarget))
if ($failedPairs.Count -gt 0 -and -not $AllowNonE2Result) { exit 1 }
