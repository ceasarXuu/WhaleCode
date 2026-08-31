param(
    [string]$Scenario = "single-file-fast-fix",
    [string]$ScenarioPath = "",
    [int]$Repeats = 1,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900, [int]$ValidationTimeoutSeconds = 420, [int]$ValidationPretestTimeoutSeconds = 120, [int]$ValidationTestTimeoutSeconds = 420,
    [int]$ProviderRequestHardLimit = 0,
    [int]$ProviderInputTokenHardLimit = 0,
    [int]$ProviderOutputTokenHardLimit = 0,
    [double]$ProviderEstimatedCostHardLimit = 0,
    [string]$ProviderBudgetCurrency = "",
    [double]$ProviderCachedInputRatePerMillion = 0,
    [double]$ProviderUncachedInputRatePerMillion = 0,
    [double]$ProviderOutputRatePerMillion = 0,
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AdditionalConfigOverride = "",
    [ValidateSet("map-always", "map-append", "map-request")]
    [string]$TaskSpaceProjectionPolicy = "map-request",
    [ValidateSet("deferred_materialization_allowed", "hard_sandbox_only")]
    [string]$OracleIsolationPolicy = "deferred_materialization_allowed",
    [string]$AuditReviewRoot = "",
    [switch]$EnableAggregate, [switch]$AllowNonE2Result, [switch]$ScoringMode, [switch]$RequireScoreValidity, [switch]$EnableDockerImageCache,
    [switch]$ResumeLatest,
    [string]$RunId = "",
    [string]$TaskListHash = "",
    [string]$SourceVersion = "",
    [string]$ProfileHash = "",
    [string]$SampleSetId = "",
    [string[]]$SampleNames = @(),
    [string]$BenchmarkFamily = "",
    [string]$RunnerEntrypoint = "",
    [string]$ArtifactOrigin = "",
    [string]$RunnerScriptSha256 = "",
    [string]$ChildRunnerSha256 = "",
    [string]$TaskListSha256 = "",
    [string]$SuiteManifestPath = "",
    [string]$SuiteReceiptPath = "",
    [string]$SuiteReceiptSha256 = "",
    [string]$ApprovalMarkerSha256 = "",
    [string]$CodeCompleteMarkerSha256 = "",
    [string]$V005NonAgentGatesPath = "",
    [string]$V005CodeCompleteMarkerPath = "",
    [string]$V005UserApprovalMarkerPath = "",
    [ValidateSet("both", "left", "right")]
    [string]$RunSide = "both",
    [ValidateSet("both", "standard", "taskspace")]
    [string]$RunLogicalMode = "both",
    [switch]$ForceRerun,
    [switch]$StopOnAnySideFailure,
    [switch]$AllowStaleWhaleBin,
    [switch]$PlanOnly
)
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\bootstrap.ps1") -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot

function ConvertTo-TaskspaceSampleNameList {
    param([string[]]$Names)
    @($Names | ForEach-Object { ([string]$_) -split "," } | ForEach-Object { ([string]$_).Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
}

if ($Repeats -lt 1) { throw "Repeats must be >= 1" }
if ($ProviderRequestHardLimit -lt 0) { throw "ProviderRequestHardLimit must be >= 0" }
foreach ($value in @($ProviderInputTokenHardLimit, $ProviderOutputTokenHardLimit, $ProviderEstimatedCostHardLimit, $ProviderCachedInputRatePerMillion, $ProviderUncachedInputRatePerMillion, $ProviderOutputRatePerMillion)) {
    if ($value -lt 0) { throw "Provider budget limits and rates must be >= 0" }
}
$providerTokenBudgetEnabled = $ProviderInputTokenHardLimit -gt 0 -or $ProviderOutputTokenHardLimit -gt 0 -or $ProviderEstimatedCostHardLimit -gt 0
if ($providerTokenBudgetEnabled -and ($ProviderRequestHardLimit -le 0 -or [string]::IsNullOrWhiteSpace($ProviderBudgetCurrency) -or $ProviderInputTokenHardLimit -le 0 -or $ProviderOutputTokenHardLimit -le 0 -or $ProviderEstimatedCostHardLimit -le 0 -or $ProviderUncachedInputRatePerMillion -le 0 -or $ProviderOutputRatePerMillion -le 0)) {
    throw "provider_budget_contract_incomplete: token and cost limits require a complete provider boundary budget"
}
if ($ProviderRequestHardLimit -gt 0 -and $Model -notmatch '^deepseek-') {
    throw "provider_boundary_model_mismatch: provider boundary requires a DeepSeek model, got $Model"
}
$SampleNames = @(ConvertTo-TaskspaceSampleNameList $SampleNames)
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
    $runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id $RunId
    Initialize-TaskspaceBenchmarkRunState $runDir $manifest.Id $Repeats $manifest.EvidenceTarget $commandLine | Out-Null
    Set-TaskspaceSampleStatus $runDir $manifest.Id "preflight" 0 0 "" "" "" "" $commandLine | Out-Null
} else {
    $existingStatus = Read-TaskspaceRunStatus $runDir
    $stale = Test-TaskspaceRunLockStale $existingStatus
    Write-TaskspaceRunEvent $runDir "resume_requested" @{ stale_lock = $stale; command_line = $commandLine }
    if ($existingStatus -and $existingStatus.PSObject.Properties.Name -contains "run_validity" -and [string]$existingStatus.run_validity -eq "invalid_harness" -and -not $ForceRerun) {
        throw "Run is invalid_harness and cannot be resumed without -ForceRerun: $runDir"
    }
    if ($existingStatus -and [string]$existingStatus.phase -eq "stopped" -and -not $ForceRerun) {
        throw "Run was stopped by an executable stop condition and cannot be resumed without -ForceRerun: $runDir"
    }
    if (-not $stale -and $existingStatus.phase -notin @("completed", "ineligible")) {
        throw "Run appears active and is not stale: $runDir"
    }
    if ($stale) { Write-TaskspaceRunEvent $runDir "stale_lock_reclaimed" @{ previous_lock_owner = [string]$existingStatus.lock_owner } }
}
function Assert-TaskspaceArtifactStorage {
    param([Parameter(Mandatory = $true)][string]$Stage)
    $safeStage = $Stage -replace '[^A-Za-z0-9_.-]', '_'
    $healthPath = Join-Path $runDir "artifact-storage-$safeStage.json"
    $health = New-TaskspaceArtifactStorageHealth $repoRoot $runDir $Stage
    Write-TaskspaceJson $health $healthPath
    Write-TaskspaceRunEvent $runDir "artifact_storage_checked" @{ stage = $Stage; status = [string]$health.status; path = $healthPath; run_artifact_bytes = [int64]$health.run_artifact_bytes; repository_artifact_bytes = [int64]$health.repository_usage.artifact_bytes }
    if ([string]$health.status -eq "fail") {
        $finding = @($health.findings | Select-Object -First 1)[0]
        $signature = New-TaskspaceInfraSignature "harness_materialization_failure" $Stage ([string]$finding.stable_code) ([string]$finding.message) "" $healthPath
        Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id $Stage ([string]$finding.stable_code) $signature $healthPath $commandLine 0 0 | Out-Null
        Write-Host "RunDir: $runDir"
        Write-Host "ArtifactStorageHealth: $healthPath"
        exit $script:TaskspaceInvalidHarnessExitCode
    }
}
Assert-TaskspaceArtifactStorage "run_preflight"
if (-not [string]::IsNullOrWhiteSpace($V005NonAgentGatesPath) -and (Test-Path -LiteralPath $V005NonAgentGatesPath -PathType Leaf)) {
    Copy-Item -LiteralPath $V005NonAgentGatesPath -Destination (Join-Path $runDir "v005-non-agent-gates.json") -Force
}
if (-not [string]::IsNullOrWhiteSpace($V005CodeCompleteMarkerPath) -and (Test-Path -LiteralPath $V005CodeCompleteMarkerPath -PathType Leaf)) {
    Copy-Item -LiteralPath $V005CodeCompleteMarkerPath -Destination (Join-Path $runDir "v005-code-complete.json") -Force
}
if (-not [string]::IsNullOrWhiteSpace($V005UserApprovalMarkerPath) -and (Test-Path -LiteralPath $V005UserApprovalMarkerPath -PathType Leaf)) {
    Copy-Item -LiteralPath $V005UserApprovalMarkerPath -Destination (Join-Path $runDir "v005-user-approval.json") -Force
}
$suiteManifestCopyPath = ""
if (-not [string]::IsNullOrWhiteSpace($SuiteManifestPath) -and (Test-Path -LiteralPath $SuiteManifestPath -PathType Leaf)) {
    $suiteManifestCopyPath = Join-Path $runDir "suite-manifest.json"
    Copy-Item -LiteralPath $SuiteManifestPath -Destination $suiteManifestCopyPath -Force
}
$suiteReceiptCopyPath = ""
if (-not [string]::IsNullOrWhiteSpace($SuiteReceiptPath) -and (Test-Path -LiteralPath $SuiteReceiptPath -PathType Leaf)) {
    $suiteReceiptCopyPath = Join-Path $runDir "suite-receipt.jsonl"
    Copy-Item -LiteralPath $SuiteReceiptPath -Destination $suiteReceiptCopyPath -Force
}
Update-TaskspaceBenchmarkRunStatusFields $runDir @{
    sample_set_id = $SampleSetId
    sample_names = @($SampleNames)
    benchmark_family = $BenchmarkFamily
    runner_entrypoint = $RunnerEntrypoint
    runner_profile_hash = $ProfileHash
    source_version = $SourceVersion
    task_list_hash = $TaskListHash
    repeats_per_sample = $Repeats
    artifact_origin = $ArtifactOrigin
    runner_script_sha256 = $RunnerScriptSha256
    child_runner_sha256 = $ChildRunnerSha256
    task_list_sha256 = $TaskListSha256
    suite_manifest_path = $suiteManifestCopyPath
    suite_manifest_sha256 = if ($suiteManifestCopyPath) { (Get-FileHash -Algorithm SHA256 -LiteralPath $suiteManifestCopyPath).Hash.ToLowerInvariant() } else { "" }
    suite_receipt_path = $suiteReceiptCopyPath
    suite_receipt_sha256 = if ($SuiteReceiptSha256) { $SuiteReceiptSha256 } elseif ($suiteReceiptCopyPath) { (Get-FileHash -Algorithm SHA256 -LiteralPath $suiteReceiptCopyPath).Hash.ToLowerInvariant() } else { "" }
    approval_marker_sha256 = $ApprovalMarkerSha256
    code_complete_marker_sha256 = $CodeCompleteMarkerSha256
    run_side = $RunSide
    run_logical_mode = $RunLogicalMode
    taskspace_projection_policy = $TaskSpaceProjectionPolicy
    stop_on_any_side_failure = [bool]$StopOnAnySideFailure
} | Out-Null
$promptCopy = Join-Path $runDir "prompt.txt"
Write-Text $promptCopy $prompt
Write-TaskspaceJson $promptGuard (Join-Path $runDir "prompt-guard.json")
Write-TaskspaceRunEvent $runDir "prompt_guard_completed" @{ invalid_prompt = [bool]$promptGuard.invalid_prompt; manual_review_required = [bool]$promptGuard.manual_review_required }
$routingDecision = New-TaskspaceRoutingDecision $manifest $prompt
$taskspacePrompt = $prompt + (New-TaskspaceRoutingPrompt $routingDecision)
$routingDecisionPath = Join-Path $runDir "routing-decision.json"
Write-TaskspaceJson $routingDecision $routingDecisionPath
Write-TaskspaceRunEvent $runDir "routing_decision_completed" @{ mode = [string]$routingDecision.recommended_mode; confidence = [string]$routingDecision.confidence; status = [string]$routingDecision.status; path = $routingDecisionPath }
$whaleVersion = ""
$whaleSha = ""
$fixtureSha = Get-TaskspaceDirectorySha256 $manifest.FixtureDir
$promptSha = Get-TaskspaceFileSha256 $manifest.PromptPath
$requiredProviderParams = @("model", "model_reasoning_effort", "sandbox_mode")
$providerParamStatus = [ordered]@{
    complete = $true
    required = $requiredProviderParams
    explicit = [ordered]@{
        model = $Model
        model_reasoning_effort = ""
        sandbox_mode = "docker_hard_boundary"
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
    $selectionPlan = @(
        for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
            $mapping = Get-TaskspaceModeMapping $repeat
            foreach ($sideName in @("left", "right")) {
                $side = [pscustomobject]@{
                    Name = $sideName
                    LogicalMode = [string]$mapping[$sideName]
                }
                [pscustomobject]@{
                    repeat = $repeat
                    side = $sideName
                    logical_mode = [string]$side.LogicalMode
                    selected = [bool](Test-TaskspaceRunSelection $side $RunSide $RunLogicalMode)
                }
            }
        }
    )
    $selectionPlanPath = Join-Path $runDir "execution-selection-plan.json"
    Write-TaskspaceJson ([pscustomobject]@{
            schema_version = 1
            run_side = $RunSide
            run_logical_mode = $RunLogicalMode
            selected_execution_count = @($selectionPlan | Where-Object { $_.selected }).Count
            executions = $selectionPlan
        }) $selectionPlanPath
    Write-Host "RunDir: $runDir"
    Write-Host "SelectionPlan: $selectionPlanPath"
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
$binaryHealthPath = Join-Path $runDir "whale-binary-preflight-health.json"
$binaryHealth = New-TaskspaceWhaleBinaryHealth $WhaleBin $repoRoot -AllowStale:$AllowStaleWhaleBin
Write-TaskspaceHarnessHealth $binaryHealthPath $binaryHealth
Write-TaskspaceRunEvent $runDir "whale_binary_preflight_completed" @{ status = [string]$binaryHealth.status; path = $binaryHealthPath; stale_for_codex_source = [bool]$binaryHealth.stale_for_codex_source }
if ([string]$binaryHealth.status -eq "fail") {
    $firstFinding = @($binaryHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "whale_binary_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $binaryHealthPath
    $abortPath = Join-Path $runDir "abort-summary.md"
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace Whale Binary Abort" "whale_binary_preflight" $firstFinding $signature $binaryHealthPath | Set-Content -LiteralPath $abortPath -Encoding UTF8
    Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "whale_binary_preflight" ([string]$firstFinding.stable_code) $signature $binaryHealthPath $commandLine 0 0 | Out-Null
    Write-Host "RunDir: $runDir"
    Write-Host "WhaleBinaryHealth: $binaryHealthPath"
    Write-Host "AbortSummary: $abortPath"
    exit 3
}
$providerCredentialHealthPath = Join-Path $runDir "provider-credential-preflight-health.json"
$providerCredentialFindings = @()
if ([string]$Model -match '^deepseek' -and [string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)) {
    $providerCredentialFindings = @([pscustomobject]@{
            severity = "fail"
            stable_code = "provider_credential_missing"
            message = "DEEPSEEK_API_KEY is required for DeepSeek benchmark model execution."
            path = "env:DEEPSEEK_API_KEY"
        })
}
$providerCredentialHealth = [pscustomobject]@{
    schema_version = 1
    status = if ($providerCredentialFindings.Count -gt 0) { "fail" } else { "pass" }
    run_validity = if ($providerCredentialFindings.Count -gt 0) { "invalid_harness" } else { "valid" }
    model = $Model
    findings = @($providerCredentialFindings)
    generated_at = (Get-Date).ToString("o")
}
Write-TaskspaceHarnessHealth $providerCredentialHealthPath $providerCredentialHealth
Write-TaskspaceRunEvent $runDir "provider_credential_preflight_completed" @{ status = [string]$providerCredentialHealth.status; path = $providerCredentialHealthPath; model = $Model }
if ([string]$providerCredentialHealth.status -eq "fail") {
    $firstFinding = @($providerCredentialHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "provider_credential_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $providerCredentialHealthPath
    $abortPath = Join-Path $runDir "abort-summary.md"
    New-TaskspaceHarnessAbortSummaryLines "TaskSpace Provider Credential Abort" "provider_credential_preflight" $firstFinding $signature $providerCredentialHealthPath | Set-Content -LiteralPath $abortPath -Encoding UTF8
    Set-TaskspaceInvalidHarnessStatus $runDir $manifest.Id "provider_credential_preflight" ([string]$firstFinding.stable_code) $signature $providerCredentialHealthPath $commandLine 0 0 | Out-Null
    Write-Host "RunDir: $runDir"
    Write-Host "ProviderCredentialHealth: $providerCredentialHealthPath"
    Write-Host "AbortSummary: $abortPath"
    exit 3
}
if (-not (Test-Path -LiteralPath $WhaleBin)) { throw "Whale binary not found: $WhaleBin" }
$WhaleBin = (Resolve-Path -LiteralPath $WhaleBin).Path
$null = & $WhaleBin exec --taskspace --help 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "Whale exec does not accept the hidden --taskspace benchmark switch."
}
$whaleVersion = (& $WhaleBin --version 2>&1) -join " "
$whaleSha = [string]$binaryHealth.whale_binary_sha256
$containerContract = Read-TaskspaceContainerContract $repoRoot
$containerImage = Resolve-TaskspaceContainerImage $repoRoot $containerContract
$containerRunId = Split-Path -Leaf $runDir
$effectiveConfigOverrides = @($ConfigOverride)
if (-not [string]::IsNullOrWhiteSpace($AdditionalConfigOverride)) {
    $effectiveConfigOverrides += $AdditionalConfigOverride
}
$effectiveConfigOverrides += @($containerContract.agent_config_overrides | ForEach-Object { [string]$_ })
$providerRouting = $null
$providerBudget = @{
    input_token_limit = $ProviderInputTokenHardLimit
    output_token_limit = $ProviderOutputTokenHardLimit
    estimated_cost_limit = $ProviderEstimatedCostHardLimit
    currency = $ProviderBudgetCurrency
    cached_input_rate_per_million = $ProviderCachedInputRatePerMillion
    uncached_input_rate_per_million = $ProviderUncachedInputRatePerMillion
    output_rate_per_million = $ProviderOutputRatePerMillion
}
if ($ProviderRequestHardLimit -gt 0) {
    $effectiveConfigOverrides += @(Get-TaskspaceProviderBoundaryConfigOverrides $containerContract)
    $providerRouting = Get-TaskspaceProviderBoundaryRouteEvidence $containerContract
}
Write-TaskspaceRunEvent $runDir "container_image_ready" @{
    image_digest = [string]$containerImage.image_digest
    docker_server_version = [string]$containerImage.docker_server_version
    build_duration_ms = [int64]$containerImage.build_duration_ms
}

$pairReports = New-Object System.Collections.Generic.List[object]
$probe = $null
$stopConditionTriggered = $false
$stopConditionReason = ""
$stopConditionArtifact = ""
. (Join-Path $PSScriptRoot "run-taskspace-benchmark-pairs.ps1")

$completedPairCount = @($pairReports.ToArray()).Count
$runSummaryPath = Join-Path $runDir "run-summary.md"
Write-TaskspaceRunSummary -Path $runSummaryPath -Reports @($pairReports.ToArray())
$sampleTimingPath = Write-TaskspaceSampleTiming -RunDir $runDir -SampleId $manifest.Id -TaskListHash $TaskListHash -SourceVersion $SourceVersion -ProfileHash $ProfileHash
Write-TaskspaceCostAggregateArtifacts -RootDir $runDir -Scope "sample" | Out-Null
Write-TaskspaceSuiteRoutingSummary -RunDir $runDir | Out-Null
if ($EnableAggregate) {
    $aggregatePath = Join-Path $runDir "aggregate-report.md"
    Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @($pairReports.ToArray())
    $auditPending = @($pairReports.ToArray() | Where-Object { @($_.evidence.e3_gate_failures) -contains "e3_human_review_not_completed" }).Count -gt 0
    $finalPhase = if ($stopConditionTriggered) { "stopped" } elseif ($auditPending) { "audit_required" } else { "finalize" }
    Set-TaskspaceSampleStatus $runDir $manifest.Id $finalPhase $completedPairCount $completedPairCount "" "" $aggregatePath $runSummaryPath $commandLine | Out-Null
} else {
    $finalPhase = if ($stopConditionTriggered) { "stopped" } else { "completed" }
    Set-TaskspaceSampleStatus $runDir $manifest.Id $finalPhase $completedPairCount $completedPairCount "" "" "" $runSummaryPath $commandLine | Out-Null
}
$runFinalReady = -not $stopConditionTriggered -and $EnableAggregate.IsPresent -and -not ($manifest.EvidenceTarget -eq "E3" -and @($pairReports.ToArray() | Where-Object { @($_.evidence.e3_gate_failures) -contains "e3_human_review_not_completed" }).Count -gt 0)
$runFinalPhase = if ($stopConditionTriggered) { "stopped" } else { "completed" }
Set-TaskspaceBenchmarkRunPhase $runDir $runFinalPhase $completedPairCount $completedPairCount $runFinalReady | Out-Null
if ($stopConditionTriggered) {
    Update-TaskspaceBenchmarkRunStatusFields $runDir @{
        stop_condition_triggered = $true
        stop_condition_reason = $stopConditionReason
        stop_condition_artifact = $stopConditionArtifact
    } | Out-Null
}
Write-Host "RunDir: $runDir"
Write-Host "RunSummary: $runSummaryPath"
Write-Host "SampleTiming: $sampleTimingPath"
foreach ($report in $pairReports) { Write-Host "PairReport: $($report.pair_report)" }

$failedPairs = @(Get-TaskspaceFailedReports $pairReports ([string]$manifest.EvidenceTarget))
if ($stopConditionTriggered) { exit 1 }
if ($failedPairs.Count -gt 0 -and -not $AllowNonE2Result) { exit 1 }
