param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("deepswe", "terminal-bench")]
    [string]$Benchmark,
    [Parameter(Mandatory = $true)][string]$TaskListPath,
    [string]$SourceVersion = "",
    [int]$Repeats = 5,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [int]$ValidationTimeoutSeconds = 420,
    [int]$ValidationPretestTimeoutSeconds = 120,
    [int]$ValidationTestTimeoutSeconds = 420,
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AuditReviewRoot = "",
    [switch]$PlanOnly,
    [switch]$ScoringMode,
    [switch]$RequireScoreValidity,
    [switch]$EnableDockerImageCache,
    [switch]$ContinueAfterInvalidHarness,
    [string]$OnePairSmokeRoot = "",
    [string]$SerialCalibrationRoot = "",
    [string]$ParallelEquivalencePath = "",
    [string]$V005NonAgentGatesPath = "",
    [string]$V005CodeCompleteMarkerPath = "",
    [string]$V005UserApprovalMarkerPath = "",
    [ValidateSet("both", "left", "right")]
    [string]$RunSide = "both",
    [switch]$SkipStartGate,
    [switch]$AllowSkippedOnePairSmoke,
    [switch]$AllowSkippedCalibrationGate,
    [string]$RunnerPath = "",
    [int]$MaxParallelSamples = 1,
    [int]$MaxParallelPairsPerSample = 1,
    [int]$MaxParallelValidationsPerPair = 1,
    [int]$MaxDockerConcurrency = 1,
    [int]$MaxModelConcurrency = 1,
    [double]$DiskReserveGb = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\run-state.ps1")
. (Join-Path $PSScriptRoot "lib\suite-status.ps1")
. (Join-Path $PSScriptRoot "lib\timing.ps1")
. (Join-Path $PSScriptRoot "lib\runtime-bottleneck-report.ps1")
. (Join-Path $PSScriptRoot "lib\resource-governor.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\e3-identity.ps1")
. (Join-Path $PSScriptRoot "lib\calibration-selection.ps1")
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")
. (Join-Path $PSScriptRoot "lib\cost-instrumentation.ps1")
if ($Repeats -lt 5) { throw "E3 suite requires Repeats >= 5." }
if (-not (Test-Path -LiteralPath $TaskListPath)) { Write-Error "TaskListPath not found: $TaskListPath"; exit 4 }
$scoreValidityEnforced = ($ScoringMode -or $RequireScoreValidity -or -not $PlanOnly)
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-e3-suite-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$suiteRoot = Join-Path $RunRoot ("suite-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $suiteRoot -Force | Out-Null
if ($scoreValidityEnforced -and -not $PlanOnly -and $SkipStartGate) {
    Write-Host "SuiteRoot: $suiteRoot"
    Write-Host "SkipStartGate is not allowed when score validity is enforced."
    exit 4
}
$samplesRoot = Join-Path $suiteRoot "samples"
New-Item -ItemType Directory -Path $samplesRoot -Force | Out-Null
$suiteHealthPath = Join-Path $suiteRoot "suite-health.json"
$skippedPath = Join-Path $suiteRoot "skipped-samples.jsonl"
$taskListHash = Get-TaskspaceFileSha256 $TaskListPath
$runner = if ([string]::IsNullOrWhiteSpace($RunnerPath)) { Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1" } else { [System.IO.Path]::GetFullPath($RunnerPath) }
if (-not (Test-Path -LiteralPath $runner)) { Write-Error "RunnerPath not found: $runner"; exit 4 }
$suiteRunnerPath = $MyInvocation.MyCommand.Path
$runnerScriptSha256 = Get-TaskspaceFileSha256 $suiteRunnerPath
$childRunnerSha256 = Get-TaskspaceFileSha256 $runner
$taskListSha256 = Get-TaskspaceFileSha256 $TaskListPath
$approvalMarkerSha256 = Get-TaskspaceFileSha256 $V005UserApprovalMarkerPath
$codeCompleteMarkerSha256 = Get-TaskspaceFileSha256 $V005CodeCompleteMarkerPath
$sampleSetDerivation = Get-TaskspaceE3SampleSetDerivation -Benchmark $Benchmark -TaskListPath $TaskListPath -Repeats $Repeats
$sampleSetId = [string]$sampleSetDerivation.sample_set_id
$profileIdentity = New-TaskspaceE3ProfileIdentity `
    -Benchmark $Benchmark `
    -SourceVersion $SourceVersion `
    -Model $Model `
    -Repeats $Repeats `
    -TimeoutSeconds $TimeoutSeconds `
    -ValidationTimeoutSeconds $ValidationTimeoutSeconds `
    -ValidationPretestTimeoutSeconds $ValidationPretestTimeoutSeconds `
    -ValidationTestTimeoutSeconds $ValidationTestTimeoutSeconds `
    -ConfigOverride $ConfigOverride `
    -EnableDockerImageCache ([bool]$EnableDockerImageCache) `
    -MaxParallelSamples $MaxParallelSamples `
    -MaxParallelPairsPerSample $MaxParallelPairsPerSample `
    -MaxParallelValidationsPerPair $MaxParallelValidationsPerPair `
    -MaxDockerConcurrency $MaxDockerConcurrency `
    -MaxModelConcurrency $MaxModelConcurrency `
    -RunnerEntrypoint "run-taskspace-e3-suite.ps1" `
    -RunnerScriptSha256 $runnerScriptSha256 `
    -ChildRunnerSha256 $childRunnerSha256 `
    -TaskListSha256 $taskListSha256 `
    -SampleSetId $sampleSetId `
    -RunSide $RunSide `
    -ScoringMode ([bool]$scoreValidityEnforced)
$profileHash = [string]$profileIdentity.profile_hash
$suiteManifestPath = Join-Path $suiteRoot "suite-manifest.json"
$suiteRunnerAttestationPath = Join-Path $suiteRoot "suite-runner-attestation.json"
$suiteReceiptPath = Join-Path $suiteRoot "suite-receipt.jsonl"
$suiteRunnerNonce = [guid]::NewGuid().ToString("n")
[pscustomobject]@{
    schema_version = 1
    artifact_origin = "real_suite"
    suite_root = $suiteRoot
    benchmark = $Benchmark
    source_version = $SourceVersion
    repeats = $Repeats
    sample_set_id = $sampleSetId
    sample_set_derivation = $sampleSetDerivation
    runner_entrypoint = "run-taskspace-e3-suite.ps1"
    runner_script_path = $suiteRunnerPath
    runner_script_sha256 = $runnerScriptSha256
    child_runner_path = $runner
    child_runner_sha256 = $childRunnerSha256
    task_list_path = ([System.IO.Path]::GetFullPath($TaskListPath))
    task_list_hash = $taskListHash
    task_list_sha256 = $taskListSha256
    profile_hash = $profileHash
    scoring_mode = [bool]$scoreValidityEnforced
    run_side = $RunSide
    generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteManifestPath -Encoding UTF8

function Get-SuiteReceiptEventHash {
    param($Row)
    $json = ([pscustomobject]$Row | ConvertTo-Json -Compress -Depth 30)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-SuiteStableStringHash {
    param([string]$Value)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

$suiteReceiptLastEventHash = ""
function Write-SuiteReceiptEvent {
    param([string]$Event, [hashtable]$Fields = @{})
    $script:suiteReceiptLastEventHash = if (Test-Path -LiteralPath $suiteReceiptPath -PathType Leaf) {
        $lastLine = @(Get-Content -Encoding UTF8 -LiteralPath $suiteReceiptPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Last 1)
        if ($lastLine.Count -eq 1) {
            try { [string](($lastLine[0] | ConvertFrom-Json).event_hash) } catch { "" }
        } else { "" }
    } else { "" }
    $row = [ordered]@{
        schema_version = 1
        event = $Event
        previous_event_hash = $script:suiteReceiptLastEventHash
        suite_run_id = Split-Path -Leaf $suiteRoot
        suite_root = $suiteRoot
        sample_set_id = $sampleSetId
        runner_entrypoint = "run-taskspace-e3-suite.ps1"
        runner_script_sha256 = $runnerScriptSha256
        child_runner_sha256 = $childRunnerSha256
        task_list_sha256 = $taskListSha256
        profile_hash = $profileHash
        runner_nonce = $suiteRunnerNonce
        timestamp = (Get-Date).ToString("o")
    }
    foreach ($key in $Fields.Keys) { $row[$key] = $Fields[$key] }
    $row["event_hash"] = Get-SuiteReceiptEventHash $row
    $script:suiteReceiptLastEventHash = $row["event_hash"]
    ([pscustomobject]$row | ConvertTo-Json -Compress -Depth 20) | Add-Content -LiteralPath $suiteReceiptPath -Encoding UTF8
}

function Get-SuiteReceiptSha256 {
    if (-not (Test-Path -LiteralPath $suiteReceiptPath -PathType Leaf)) { return "" }
    (Get-FileHash -LiteralPath $suiteReceiptPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-SuiteRunnerAttestation {
    $receiptShaBeforeAttestation = Get-SuiteReceiptSha256
    $receiptEventHashBeforeAttestation = $script:suiteReceiptLastEventHash
    $commandLine = [Environment]::CommandLine
    $attestation = [ordered]@{
        schema_version = 1
        artifact_origin = "real_suite_runner"
        runner_entrypoint = "run-taskspace-e3-suite.ps1"
        runner_script_sha256 = $runnerScriptSha256
        child_runner_sha256 = $childRunnerSha256
        task_list_path = ([System.IO.Path]::GetFullPath($TaskListPath))
        task_list_sha256 = $taskListSha256
        suite_manifest_sha256 = (Get-FileHash -LiteralPath $suiteManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
        suite_receipt_sha256_before_attestation = $receiptShaBeforeAttestation
        suite_receipt_event_hash_before_attestation = $receiptEventHashBeforeAttestation
        profile_hash = $profileHash
        sample_set_id = $sampleSetId
        suite_root = $suiteRoot
        runner_nonce = $suiteRunnerNonce
        process_id = $PID
        command_line = $commandLine
        command_line_sha256 = Get-SuiteStableStringHash $commandLine
        generated_at = (Get-Date).ToString("o")
    }
    [pscustomobject]$attestation | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteRunnerAttestationPath -Encoding UTF8
    $attestationSha = (Get-FileHash -LiteralPath $suiteRunnerAttestationPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-SuiteReceiptEvent "runner_attestation_generated" @{
        runner_nonce = $suiteRunnerNonce
        process_id = $PID
        command_line_sha256 = Get-SuiteStableStringHash $commandLine
        suite_runner_attestation_path = $suiteRunnerAttestationPath
        suite_runner_attestation_sha256 = $attestationSha
        suite_receipt_event_hash_before_attestation = $receiptEventHashBeforeAttestation
    }
}

function Update-SampleRunReceiptFields {
    Write-SuiteRunnerAttestation
    $receiptSha = Get-SuiteReceiptSha256
    $attestationSha = (Get-FileHash -LiteralPath $suiteRunnerAttestationPath -Algorithm SHA256).Hash.ToLowerInvariant()
    foreach ($statusPath in @(Get-ChildItem -LiteralPath $samplesRoot -Filter "run-status.json" -Recurse -ErrorAction SilentlyContinue)) {
        try {
            Update-TaskspaceBenchmarkRunStatusFields (Split-Path -Parent $statusPath.FullName) @{
                suite_receipt_path = Join-Path (Split-Path -Parent $statusPath.FullName) "suite-receipt.jsonl"
                suite_receipt_sha256 = $receiptSha
                suite_runner_attestation_path = Join-Path (Split-Path -Parent $statusPath.FullName) "suite-runner-attestation.json"
                suite_runner_attestation_sha256 = $attestationSha
            } | Out-Null
            Copy-Item -LiteralPath $suiteReceiptPath -Destination (Join-Path (Split-Path -Parent $statusPath.FullName) "suite-receipt.jsonl") -Force
            Copy-Item -LiteralPath $suiteRunnerAttestationPath -Destination (Join-Path (Split-Path -Parent $statusPath.FullName) "suite-runner-attestation.json") -Force
        } catch {}
    }
}

Write-SuiteReceiptEvent "run_initialized" @{
    suite_manifest_path = $suiteManifestPath
    suite_manifest_sha256 = (Get-FileHash -LiteralPath $suiteManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-SuiteStartGateAbortHealth {
    param($Gate)
    [pscustomobject]@{
        schema_version = 1
        status = "invalid_harness"
        suite_root = $suiteRoot
        suite_abort_reason = "e3_start_gate_failed/$($Gate.first_failure_stable_code)"
        start_gate_status = $Gate.status
        start_gate_path = $Gate.json_path
        start_gate_report = $Gate.markdown_path
        first_failure_gate = $Gate.first_failure_gate
        first_failure_stable_code = $Gate.first_failure_stable_code
        invalid_harness_sample_count = 0
        remaining_samples_skipped = 0
        completed_child_processes = 0
        score_valid_child_runs = 0
        score_invalid_child_runs = 0
        score_pending_audit_child_runs = 0
        suite_score_ready = $false
        suite_score_valid = $false
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteHealthPath -Encoding UTF8
}

function Write-SuiteEarlyAbortArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$Reason,
        [string]$Phase = "pre_scheduling",
        $SampleStatuses = @()
    )
    $statuses = @($SampleStatuses)
    if ($statuses.Count -eq 0) {
        $statuses = @([pscustomobject]@{
                sample_id = "suite"
                phase = $Phase
                run_validity = "invalid_harness"
                exit_code = 3
                abort_scope = "suite"
                abort_phase = $Phase
                abort_signature = $Reason
                abort_reason = $Reason
                sample_root = $suiteRoot
            })
    }
    [pscustomobject]@{
        schema_version = 1
        status = "invalid_harness"
        suite_root = $suiteRoot
        signature_counts = @{}
        sample_statuses = @($statuses)
        suite_abort_reason = $Reason
        invalid_harness_sample_count = @($statuses | Where-Object { $_.PSObject.Properties.Name -contains "run_validity" -and [string]$_.run_validity -eq "invalid_harness" }).Count
        remaining_samples_skipped = @($statuses | Where-Object { $_.PSObject.Properties.Name -contains "skipped_reason" -and -not [string]::IsNullOrWhiteSpace([string]$_.skipped_reason) }).Count
        completed_child_processes = 0
        score_valid_child_runs = 0
        score_invalid_child_runs = 0
        score_pending_audit_child_runs = 0
        suite_score_ready = $false
        suite_score_valid = $false
        calibration_selection_path = (Join-Path $suiteRoot "calibration-selection.json")
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteHealthPath -Encoding UTF8
    $suiteTimingPath = Write-TaskspaceSuiteTiming -SuiteRoot $suiteRoot -SampleStatuses $statuses -TaskListHash $taskListHash -SourceVersion $SourceVersion -ProfileHash $profileHash
    $suiteCost = Write-TaskspaceCostAggregateArtifacts -RootDir $suiteRoot -Scope "suite"
    $runtimeBottleneckPath = Write-TaskspaceRuntimeBottleneckReport -TimingPath $suiteTimingPath -ScoreValid $false
    $calibrationPath = Write-TaskspaceRuntimeCalibrationReport -TimingPath $suiteTimingPath -ScoreValid $false -CommandLine ([Environment]::CommandLine) -ParallelismPath $(if (Get-Variable -Name parallelismPath -Scope Script -ErrorAction SilentlyContinue) { $script:parallelismPath } else { "" })
    Write-SuiteReceiptEvent "suite_finalized" @{
        status = "invalid_harness"
        exit_code = 3
        abort_reason = $Reason
        phase = $Phase
        suite_health_path = $suiteHealthPath
        suite_timing_path = $suiteTimingPath
        suite_cost_gate_path = $suiteCost.suite_cost_gate_path
        runtime_bottleneck_path = $runtimeBottleneckPath
        runtime_calibration_path = $calibrationPath
        sample_status_count = @($statuses).Count
    }
    [pscustomobject]@{ suite_timing_path = $suiteTimingPath; suite_cost_gate_path = [string]$suiteCost.suite_cost_gate_path; runtime_bottleneck_path = $runtimeBottleneckPath; runtime_calibration_path = $calibrationPath }
}

if ($scoreValidityEnforced -and -not $PlanOnly -and -not $SkipStartGate) {
    $gate = Invoke-TaskspaceE3StartGate `
        -RepoRoot (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path `
        -BenchmarkRoot $PSScriptRoot `
        -OutputDir (Join-Path $suiteRoot "start-gate") `
        -RunRoot $suiteRoot `
        -TaskListPath $TaskListPath `
        -SourceVersion $SourceVersion `
        -ExpectedTaskListHash $taskListHash `
        -ExpectedProfileHash $profileHash `
        -Benchmark $Benchmark `
        -Repeats $Repeats `
        -ExpectedSampleSetId $sampleSetId `
        -OnePairSmokeRoot $OnePairSmokeRoot `
        -SerialCalibrationRoot $SerialCalibrationRoot `
        -ParallelEquivalencePath $ParallelEquivalencePath `
        -V005NonAgentGatesPath $V005NonAgentGatesPath `
        -V005CodeCompleteMarkerPath $V005CodeCompleteMarkerPath `
        -V005UserApprovalMarkerPath $V005UserApprovalMarkerPath `
        -RunSelfTests `
        -AllowSkippedPathContract `
        -AllowSkippedOnePairSmoke:$AllowSkippedOnePairSmoke `
        -AllowSkippedCalibrationGate:$AllowSkippedCalibrationGate
    Write-Host "E3StartGate: $($gate.json_path)"
    Write-Host "E3StartGateReport: $($gate.markdown_path)"
    if ([int]$gate.exit_code -ne 0) {
        Write-SuiteStartGateAbortHealth $gate
        $early = Write-SuiteEarlyAbortArtifacts -Reason "e3_start_gate_failed/$($gate.first_failure_stable_code)" -Phase "e3_start_gate"
        Write-Host "SuiteRoot: $suiteRoot"
        Write-Host "SuiteHealth: $suiteHealthPath"
        Write-Host "SuiteTiming: $($early.suite_timing_path)"
        exit 3
    }
    if (-not [bool]$gate.gate_decision.full_e3_allowed) {
        Write-SuiteStartGateAbortHealth $gate
        $reason = "e3_start_gate_blocked/$($gate.gate_decision.next_allowed_command_category)"
        $early = Write-SuiteEarlyAbortArtifacts -Reason $reason -Phase "e3_start_gate"
        Write-Host "SuiteRoot: $suiteRoot"
        Write-Host "SuiteHealth: $suiteHealthPath"
        Write-Host "SuiteTiming: $($early.suite_timing_path)"
        exit 3
    }
}

$resourceConfig = New-TaskspaceResourceGovernorConfig `
    -MaxParallelSamples $MaxParallelSamples `
    -MaxParallelPairsPerSample $MaxParallelPairsPerSample `
    -MaxParallelValidationsPerPair $MaxParallelValidationsPerPair `
    -MaxDockerConcurrency $MaxDockerConcurrency `
    -MaxModelConcurrency $MaxModelConcurrency `
    -DiskReserveGb $DiskReserveGb
$serialGuard = Test-TaskspaceResourceGovernorSerialOnly $resourceConfig
$diskReservation = Test-TaskspaceDiskReservation @($suiteRoot, $samplesRoot, (Split-Path -Parent $TaskListPath)) ([int64]$resourceConfig.disk_reserve_bytes)
$parallelismPath = Write-TaskspaceParallelismArtifact $suiteRoot $resourceConfig $serialGuard $diskReservation (New-TaskspaceResourceWaitSnapshot)
if (-not [bool]$resourceConfig.valid) {
    [Console]::Error.WriteLine("Invalid resource governor configuration: " + (@($resourceConfig.errors) -join "; "))
    exit 4
}
if ([string]$diskReservation.status -eq "fail") {
    [Console]::Error.WriteLine("Disk reservation failed before suite scheduling. See $parallelismPath")
    $diskFailure = @($diskReservation.failures | Select-Object -First 1)[0]
    $reason = if ($diskFailure) { "harness_materialization_failure/$($diskFailure.stable_code)" } else { "harness_materialization_failure/disk_reservation_failed" }
    $early = Write-SuiteEarlyAbortArtifacts -Reason $reason -Phase "disk_reservation"
    Write-Host "SuiteRoot: $suiteRoot"
    Write-Host "SuiteHealth: $suiteHealthPath"
    Write-Host "SuiteTiming: $($early.suite_timing_path)"
    exit 3
}
if (@($serialGuard.unsupported_parallel_fields).Count -gt 0) {
    [Console]::Error.WriteLine("Parallel E3 execution flags are not implemented yet. Unsupported fields: " + (@($serialGuard.unsupported_parallel_fields) -join ", ") + ". See $parallelismPath")
    exit 4
}

function Read-TaskspaceSuiteList {
    param([string]$Path)
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    if ($raw.TrimStart().StartsWith("[")) { return @($raw | ConvertFrom-Json) }
    @(Get-Content -Encoding UTF8 -LiteralPath $Path | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
}

function Write-SuiteHealth {
    param($Status, $SampleStatuses, $SignatureCounts, [string]$AbortReason = "")
    $skipped = @($SampleStatuses | Where-Object { $_.PSObject.Properties.Name -contains "skipped_reason" -and -not [string]::IsNullOrWhiteSpace([string]$_.skipped_reason) })
    $invalid = @($SampleStatuses | Where-Object { $_.PSObject.Properties.Name -contains "run_validity" -and [string]$_.run_validity -eq "invalid_harness" })
    $skippedPairs = Get-TaskspaceSuiteRemainingSkippedPairs $suiteRoot
    $scoreSummary = Get-TaskspaceSuiteScoreValiditySummary $SampleStatuses $Repeats
    $timeSaved = Get-TaskspaceSuiteExpectedTimeSaved $suiteRoot $SampleStatuses $Repeats
    [pscustomobject]@{
        schema_version = 1
        status = $Status
        suite_root = $suiteRoot
        signature_counts = $SignatureCounts
        sample_statuses = @($SampleStatuses)
        suite_abort_reason = $AbortReason
        invalid_harness_sample_count = $invalid.Count
        remaining_samples_skipped = $skipped.Count
        remaining_pairs_skipped = $skippedPairs
        completed_child_processes = $scoreSummary.completed_child_processes
        score_valid_child_runs = $scoreSummary.score_valid_child_runs
        score_invalid_child_runs = $scoreSummary.score_invalid_child_runs
        score_pending_audit_child_runs = $scoreSummary.score_pending_audit_child_runs
        first_score_invalid_run = $scoreSummary.first_score_invalid_run
        suite_score_ready = $scoreSummary.suite_score_ready
        suite_score_valid = $scoreSummary.suite_score_valid
        calibration_selection_path = (Join-Path $suiteRoot "calibration-selection.json")
        expected_time_saved_minutes = $timeSaved.expected_time_saved_minutes
        expected_time_saved_basis = $timeSaved.expected_time_saved_basis
        skipped_pair_equivalent_count = $timeSaved.skipped_pair_equivalent_count
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteHealthPath -Encoding UTF8
}

try {
    $tasks = @(Read-TaskspaceSuiteList $TaskListPath)
} catch {
    Write-Error "Malformed TaskListPath: $($_.Exception.Message)"
    exit 4
}
if ($tasks.Count -eq 0) { Write-Error "TaskListPath contains no samples."; exit 4 }
$suiteSampleNames = @($tasks | ForEach-Object {
        if ($_.PSObject.Properties.Name -contains "sample_id" -and -not [string]::IsNullOrWhiteSpace([string]$_.sample_id)) { [string]$_.sample_id }
        elseif ($_.PSObject.Properties.Name -contains "task_dir" -and -not [string]::IsNullOrWhiteSpace([string]$_.task_dir)) { Split-Path -Leaf ([string]$_.task_dir) }
    })
$calibrationSelectionPath = Join-Path $suiteRoot "calibration-selection.json"
$calibrationSelection = New-TaskspaceCalibrationSelection -TaskListPath $TaskListPath -OutputPath $calibrationSelectionPath -Benchmark $Benchmark -SelectionCount 3
Write-Host "CalibrationSelection: $calibrationSelectionPath"

$sampleStatuses = New-Object System.Collections.Generic.List[object]
$signatureCounts = @{}
$suiteAbort = ""
$exitCode = 0
$maxSampleWorkers = [Math]::Max(1, [int]$resourceConfig.max_parallel_samples)

function New-SuiteSampleRow {
    param($Task, [int]$Index)
    $taskDir = if ($Task.PSObject.Properties.Name -contains "task_dir") { [string]$Task.task_dir } else { "" }
    $sampleId = if ($Task.PSObject.Properties.Name -contains "sample_id") { [string]$Task.sample_id } else { "sample-$($Index + 1)" }
    $recordSourceVersion = if ($Task.PSObject.Properties.Name -contains "source_version" -and -not [string]::IsNullOrWhiteSpace([string]$Task.source_version)) { [string]$Task.source_version } else { $SourceVersion }
    if ([string]::IsNullOrWhiteSpace($taskDir) -or [string]::IsNullOrWhiteSpace($recordSourceVersion)) {
        throw "Suite sample requires task_dir and source_version/default SourceVersion: $sampleId"
    }
    [pscustomobject]@{
        index = $Index
        task_dir = $taskDir
        sample_id = $sampleId
        source_version = $recordSourceVersion
        sample_root = (Join-Path $samplesRoot $sampleId)
    }
}

function New-SuiteSkippedSample {
    param($Row, [string]$AbortSignature)
    $skipReason = if ([string]$AbortSignature -match "disk_space_low|disk_space_threshold_invalid") { "suite_global_disk_guard" } else { "suite_repeated_infra_signature" }
    $skippedSampleRoot = [string]$Row.sample_root
    $status = [pscustomobject]@{
        sample_id = [string]$Row.sample_id
        task_dir = [string]$Row.task_dir
        phase = "skipped"
        run_validity = "invalid_harness"
        exit_code = 3
        abort_scope = "suite"
        abort_phase = "suite_circuit_breaker"
        abort_signature = $AbortSignature
        skipped_reason = $skipReason
        sample_root = $skippedSampleRoot
    }
    New-Item -ItemType Directory -Path $skippedSampleRoot -Force | Out-Null
    $status | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $skippedSampleRoot "sample-status.json") -Encoding UTF8
    ($status | ConvertTo-Json -Compress) | Add-Content -LiteralPath $skippedPath -Encoding UTF8
    $status
}

function Test-SuiteSampleDiskPreflight {
    param($Row)
    $sampleDiskHealthPath = Join-Path $suiteRoot ("suite-disk-health-{0:000}.json" -f ([int]$Row.index + 1))
    $sampleDiskHealth = New-TaskspaceDiskHealth @($suiteRoot, [string]$Row.sample_root, [string]$Row.task_dir) "suite_sample_preflight"
    Write-TaskspaceHarnessHealth $sampleDiskHealthPath $sampleDiskHealth
    if ([string]$sampleDiskHealth.status -ne "fail") { return $null }
    $firstFinding = @($sampleDiskHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
    $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "suite_sample_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $sampleDiskHealthPath
    [pscustomobject]@{
        sample_id = [string]$Row.sample_id
        task_dir = [string]$Row.task_dir
        run_validity = "invalid_harness"
        exit_code = 3
        abort_scope = "sample"
        abort_phase = "suite_sample_preflight"
        abort_signature = $signature.key
        abort_reason = [string]$firstFinding.stable_code
        first_failure_artifact = $sampleDiskHealthPath
        sample_root = [string]$Row.sample_root
    }
}

function New-SuiteChildArgs {
    param($Row)
    $childArgs = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
        "-Benchmark", $Benchmark,
        "-TaskDir", [string]$Row.task_dir,
        "-SampleId", [string]$Row.sample_id,
        "-SourceVersion", [string]$Row.source_version,
        "-Repeats", $Repeats,
        "-RunRoot", [string]$Row.sample_root,
        "-WhaleBin", $WhaleBin,
        "-Model", $Model,
        "-TimeoutSeconds", $TimeoutSeconds,
        "-ValidationTimeoutSeconds", $ValidationTimeoutSeconds,
        "-ValidationPretestTimeoutSeconds", $ValidationPretestTimeoutSeconds,
        "-ValidationTestTimeoutSeconds", $ValidationTestTimeoutSeconds,
        "-TaskListHash", $taskListHash,
        "-ProfileHash", $profileHash,
        "-SampleSetId", $sampleSetId,
        "-SuiteRunnerEntrypoint", "run-taskspace-e3-suite.ps1",
        "-ArtifactOrigin", "real_suite",
        "-RunnerScriptSha256", $runnerScriptSha256,
        "-ChildRunnerSha256", $childRunnerSha256,
        "-TaskListSha256", $taskListSha256,
        "-SuiteManifestPath", $suiteManifestPath,
        "-SuiteReceiptPath", $suiteReceiptPath,
        "-SuiteReceiptSha256", (Get-SuiteReceiptSha256),
        "-RunSide", $RunSide,
        "-EnableAggregate"
    )
    $optionalStringArgs = [ordered]@{
        ApprovalMarkerSha256 = $approvalMarkerSha256
        CodeCompleteMarkerSha256 = $codeCompleteMarkerSha256
        V005NonAgentGatesPath = $V005NonAgentGatesPath
        V005CodeCompleteMarkerPath = $V005CodeCompleteMarkerPath
        V005UserApprovalMarkerPath = $V005UserApprovalMarkerPath
    }
    foreach ($entry in $optionalStringArgs.GetEnumerator()) {
        $value = [string]$entry.Value
        if (-not [string]::IsNullOrWhiteSpace($value)) {
            $childArgs += @("-$($entry.Key)", $value)
        }
    }
    $nonEmptySuiteSampleNames = @($suiteSampleNames | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($nonEmptySuiteSampleNames.Count -gt 0) {
        $childArgs += @("-SampleNames", (@($nonEmptySuiteSampleNames) -join ","))
    }
    foreach ($override in @($ConfigOverride)) { $childArgs += @("-ConfigOverride", $override) }
    if ($AuditReviewRoot) { $childArgs += @("-AuditReviewRoot", $AuditReviewRoot) }
    if ($PlanOnly) { $childArgs += "-PlanOnly" }
    if ($scoreValidityEnforced) { $childArgs += "-ScoringMode" }
    if ($RequireScoreValidity) { $childArgs += "-RequireScoreValidity" }
    if ($EnableDockerImageCache) { $childArgs += "-EnableDockerImageCache" }
    $childArgs
}

function Complete-SuiteSampleStatus {
    param($Row, [int]$ChildExit)
    $statusPath = Get-ChildItem -LiteralPath ([string]$Row.sample_root) -Filter "sample-status.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $status = if ($statusPath) { Get-Content -Raw -Encoding UTF8 -LiteralPath $statusPath.FullName | ConvertFrom-Json } else { [pscustomobject]@{ sample_id = [string]$Row.sample_id; run_validity = if ($ChildExit -eq 3) { "invalid_harness" } else { "unknown" }; exit_code = $ChildExit } }
    if (-not ($status.PSObject.Properties.Name -contains "sample_root")) { $status | Add-Member -NotePropertyName sample_root -NotePropertyValue ([string]$Row.sample_root) -Force }
    $alreadyInvalidHarness = $status.PSObject.Properties.Name -contains "run_validity" -and [string]$status.run_validity -eq "invalid_harness"
    $completedDiagnosticRun = Test-TaskspaceSuiteChildStatusComplete $status $Repeats
    if ($ChildExit -ne 0 -and -not $alreadyInvalidHarness -and -not $completedDiagnosticRun) {
        $status = New-TaskspaceSuiteChildFailureStatus $status ([string]$Row.sample_id) ([string]$Row.task_dir) $ChildExit $(if ($statusPath) { [string]$statusPath.FullName } else { "" }) ([string]$Row.sample_root)
    }
    $aggregatePath = Get-ChildItem -LiteralPath ([string]$Row.sample_root) -Filter "aggregate.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($scoreValidityEnforced -and $aggregatePath) {
        try {
            $aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath.FullName | ConvertFrom-Json
            if ($aggregate.PSObject.Properties.Name -contains "score_valid" -and -not [bool]$aggregate.score_valid) {
                $scoreBlockReason = if ($aggregate.PSObject.Properties.Name -contains "score_block_reason") { [string]$aggregate.score_block_reason } else { "" }
                $scoreInvalidReason = if ($aggregate.PSObject.Properties.Name -contains "score_invalid_reason") { [string]$aggregate.score_invalid_reason } else { "" }
                if ($scoreBlockReason -eq "audit_required" -and [string]::IsNullOrWhiteSpace($scoreInvalidReason)) {
                    foreach ($property in @(
                            @{ name = "score_ready"; value = $false },
                            @{ name = "score_valid"; value = $false },
                            @{ name = "score_block_reason"; value = "audit_required" },
                            @{ name = "score_status"; value = "pending_audit" },
                            @{ name = "score_artifact"; value = [string]$aggregatePath.FullName }
                        )) {
                        $status | Add-Member -NotePropertyName $property.name -NotePropertyValue $property.value -Force
                    }
                    Write-TaskspaceRunEvent $suiteRoot "suite_score_pending_audit" @{
                        suite_run_id = (Split-Path -Leaf $suiteRoot)
                        child_run_id = if ($status.PSObject.Properties.Name -contains "sample_root") { [string]$status.sample_root } else { "" }
                        sample_id = if ($status.PSObject.Properties.Name -contains "sample_id") { [string]$status.sample_id } else { [string]$Row.sample_id }
                        aggregate_path = [string]$aggregatePath.FullName
                    }
                } else {
                    $status = New-TaskspaceSuiteChildFailureStatus $status ([string]$Row.sample_id) ([string]$Row.task_dir) 3 ([string]$aggregatePath.FullName) ([string]$Row.sample_root)
                    $status.abort_phase = "score_validity"
                    $status.abort_signature = "harness_materialization_failure/score_invalid"
                    $status.abort_reason = if (-not [string]::IsNullOrWhiteSpace($scoreInvalidReason)) { $scoreInvalidReason } elseif (-not [string]::IsNullOrWhiteSpace($scoreBlockReason)) { $scoreBlockReason } else { "score_invalid" }
                    $ChildExit = 3
                }
            }
        } catch {
            if ($scoreValidityEnforced) {
                $status = New-TaskspaceSuiteChildFailureStatus $status ([string]$Row.sample_id) ([string]$Row.task_dir) 3 ([string]$aggregatePath.FullName) ([string]$Row.sample_root)
                $status.abort_phase = "score_validity"
                $status.abort_reason = "aggregate_score_validity_parse_failed"
                $ChildExit = 3
            }
        }
    }
    [pscustomobject]@{ status = $status; child_exit = $ChildExit }
}

function Update-SuiteAbortFromStatus {
    param($Status, [int]$ChildExit, [int]$Index)
    if ($ChildExit -eq 1 -and $script:exitCode -eq 0) { $script:exitCode = 1 }
    if ($ChildExit -eq 2 -and $script:exitCode -eq 0) { $script:exitCode = 2 }
    if ($ChildExit -eq 3 -or ($Status.PSObject.Properties.Name -contains "run_validity" -and [string]$Status.run_validity -eq "invalid_harness")) {
        $sig = if ($Status.PSObject.Properties.Name -contains "abort_signature" -and -not [string]::IsNullOrWhiteSpace([string]$Status.abort_signature)) { [string]$Status.abort_signature } else { "harness_materialization_failure/unknown" }
        if (-not $signatureCounts.ContainsKey($sig)) { $signatureCounts[$sig] = 0 }
        $signatureCounts[$sig]++
        if ($script:exitCode -eq 0) { $script:exitCode = 3 }
        $global = $sig -match "docker_backend_unavailable|uv_cache_missing|validator_source_missing|disk_space_low|disk_space_threshold_invalid"
        $shouldAbortSuite = $false
        if ($scoreValidityEnforced -and -not $ContinueAfterInvalidHarness) { $shouldAbortSuite = $true }
        if (-not $ContinueAfterInvalidHarness -and ($global -or $signatureCounts[$sig] -ge 2)) { $shouldAbortSuite = $true }
        if ($shouldAbortSuite -and -not $script:suiteAbort) {
            $script:suiteAbort = $sig
            $script:exitCode = 3
            Write-TaskspaceRunEvent $suiteRoot "suite_score_invalidated" @{
                suite_run_id = (Split-Path -Leaf $suiteRoot)
                child_run_id = if ($Status.PSObject.Properties.Name -contains "sample_root") { [string]$Status.sample_root } else { "" }
                sample_id = if ($Status.PSObject.Properties.Name -contains "sample_id") { [string]$Status.sample_id } else { "" }
                reason = $sig
                remaining_samples_skipped = [Math]::Max(0, $tasks.Count - $Index - 1)
            }
        }
    }
}

for ($index = 0; $index -lt $tasks.Count; ) {
    $batch = New-Object System.Collections.Generic.List[object]
    while ($index -lt $tasks.Count -and $batch.Count -lt $maxSampleWorkers) {
        $row = New-SuiteSampleRow $tasks[$index] $index
        Write-SuiteReceiptEvent "sample_scheduled" @{
            sample_id = [string]$row.sample_id
            sample_index = [int]$row.index
            sample_root = [string]$row.sample_root
            task_dir = [string]$row.task_dir
        }
        $batch.Add($row)
        $index++
    }
    $jobs = New-Object System.Collections.Generic.List[object]
    foreach ($row in @($batch.ToArray())) {
        if ($suiteAbort) {
            $sampleStatuses.Add((New-SuiteSkippedSample $row $suiteAbort))
            continue
        }
        $preflightStatus = Test-SuiteSampleDiskPreflight $row
        if ($preflightStatus) {
            $status = $preflightStatus
            $sampleStatuses.Add($status)
            Update-SuiteAbortFromStatus $status 3 ([int]$row.index)
            continue
        }
        $childArgs = New-SuiteChildArgs $row
        if ($maxSampleWorkers -eq 1) {
            & powershell @childArgs
            $result = [pscustomobject]@{ row = $row; exit_code = $LASTEXITCODE; output = @() }
        } else {
            $job = Start-Job -ScriptBlock {
                param([string[]]$ChildArgs)
                $output = & powershell @ChildArgs 2>&1
                [pscustomobject]@{ exit_code = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }; output = @($output | ForEach-Object { [string]$_ }) }
            } -ArgumentList (, [string[]]$childArgs)
            $result = [pscustomobject]@{ row = $row; job = $job }
        }
        $jobs.Add($result)
    }
    foreach ($jobRow in @($jobs.ToArray() | Sort-Object { [int]$_.row.index })) {
        if ($jobRow.PSObject.Properties.Name -contains "job") {
            Wait-Job -Job $jobRow.job | Out-Null
            $jobResult = Receive-Job -Job $jobRow.job | Select-Object -First 1
            Remove-Job -Job $jobRow.job -Force -ErrorAction SilentlyContinue
            foreach ($line in @($jobResult.output)) { Write-Host $line }
            $childExit = [int]$jobResult.exit_code
        } else {
            $childExit = [int]$jobRow.exit_code
        }
        $completed = Complete-SuiteSampleStatus $jobRow.row $childExit
        $sampleStatuses.Add($completed.status)
        Write-SuiteReceiptEvent "sample_completed" @{
            sample_id = [string]$jobRow.row.sample_id
            sample_index = [int]$jobRow.row.index
            sample_root = [string]$jobRow.row.sample_root
            child_exit = [int]$completed.child_exit
            run_validity = if ($completed.status.PSObject.Properties.Name -contains "run_validity") { [string]$completed.status.run_validity } else { "" }
            completed_pairs = if ($completed.status.PSObject.Properties.Name -contains "completed_pairs") { [int]$completed.status.completed_pairs } else { 0 }
        }
        Update-SuiteAbortFromStatus $completed.status ([int]$completed.child_exit) ([int]$jobRow.row.index)
    }
}

$finalScoreSummary = Get-TaskspaceSuiteScoreValiditySummary @($sampleStatuses.ToArray()) $Repeats
$statusText = if ($suiteAbort) { "invalid_harness" } elseif ([int]$finalScoreSummary.score_pending_audit_child_runs -gt 0) { "audit_required" } else { "completed" }
Write-SuiteHealth $statusText @($sampleStatuses.ToArray()) $signatureCounts $suiteAbort
$suiteTimingPath = Write-TaskspaceSuiteTiming -SuiteRoot $suiteRoot -SampleStatuses @($sampleStatuses.ToArray()) -TaskListHash $taskListHash -SourceVersion $SourceVersion -ProfileHash $profileHash
$suiteCost = Write-TaskspaceCostAggregateArtifacts -RootDir $suiteRoot -Scope "suite"
$runtimeBottleneckPath = Write-TaskspaceRuntimeBottleneckReport -TimingPath $suiteTimingPath -ScoreValid ([bool]$finalScoreSummary.suite_score_valid)
$gitCommit = ""
try { $gitCommit = (& git -C (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path rev-parse HEAD 2>$null) } catch { $gitCommit = "" }
$calibrationPath = Write-TaskspaceRuntimeCalibrationReport -TimingPath $suiteTimingPath -ScoreValid ([bool]$finalScoreSummary.suite_score_valid) -CommandLine ([Environment]::CommandLine) -GitCommit ([string]$gitCommit).Trim() -ParallelismPath $parallelismPath
Write-SuiteReceiptEvent "suite_finalized" @{
    status = $statusText
    exit_code = $exitCode
    suite_health_path = $suiteHealthPath
    suite_timing_path = $suiteTimingPath
    suite_cost_gate_path = $suiteCost.suite_cost_gate_path
    runtime_bottleneck_path = $runtimeBottleneckPath
    runtime_calibration_path = $calibrationPath
    sample_status_count = @($sampleStatuses.ToArray()).Count
}
Update-SampleRunReceiptFields
Write-Host "SuiteRoot: $suiteRoot"
Write-Host "SuiteHealth: $suiteHealthPath"
Write-Host "SuiteTiming: $suiteTimingPath"
Write-Host "SuiteCostGate: $($suiteCost.suite_cost_gate_path)"
Write-Host "RuntimeBottleneck: $runtimeBottleneckPath"
Write-Host "RuntimeCalibration: $calibrationPath"
Write-Host "Parallelism: $parallelismPath"
if (Test-Path -LiteralPath $skippedPath) { Write-Host "SkippedSamples: $skippedPath" }
exit $exitCode
