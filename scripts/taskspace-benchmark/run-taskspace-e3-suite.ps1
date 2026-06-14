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
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
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
    [switch]$SkipStartGate,
    [switch]$AllowSkippedOnePairSmoke,
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
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")
if ($Repeats -lt 5) { throw "E3 suite requires Repeats >= 5." }
if (-not (Test-Path -LiteralPath $TaskListPath)) { Write-Error "TaskListPath not found: $TaskListPath"; exit 4 }
if (($ScoringMode -or $RequireScoreValidity) -and $SkipStartGate -and -not $PlanOnly) {
    [Console]::Error.WriteLine("SkipStartGate is not allowed for score-bearing E3 suite runs. Use PlanOnly for dry runs or provide start/calibration gate evidence.")
    exit 4
}
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-e3-suite-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$suiteRoot = Join-Path $RunRoot ("suite-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $suiteRoot -Force | Out-Null
$samplesRoot = Join-Path $suiteRoot "samples"
New-Item -ItemType Directory -Path $samplesRoot -Force | Out-Null
$suiteHealthPath = Join-Path $suiteRoot "suite-health.json"
$skippedPath = Join-Path $suiteRoot "skipped-samples.jsonl"
$taskListHash = Get-TaskspaceFileSha256 $TaskListPath
$profileIdentity = New-TaskspaceE3ProfileIdentity `
    -Benchmark $Benchmark `
    -SourceVersion $SourceVersion `
    -Model $Model `
    -Repeats $Repeats `
    -TimeoutSeconds $TimeoutSeconds `
    -ValidationTimeoutSeconds $ValidationTimeoutSeconds `
    -ValidationPretestTimeoutSeconds $ValidationPretestTimeoutSeconds `
    -ValidationTestTimeoutSeconds $ValidationTestTimeoutSeconds `
    -SandboxMode $SandboxMode `
    -ConfigOverride $ConfigOverride `
    -EnableDockerImageCache ([bool]$EnableDockerImageCache) `
    -MaxParallelSamples $MaxParallelSamples `
    -MaxParallelPairsPerSample $MaxParallelPairsPerSample `
    -MaxParallelValidationsPerPair $MaxParallelValidationsPerPair `
    -MaxDockerConcurrency $MaxDockerConcurrency `
    -MaxModelConcurrency $MaxModelConcurrency
$profileHash = [string]$profileIdentity.profile_hash

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
        suite_score_valid = $false
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $suiteHealthPath -Encoding UTF8
}

if (($ScoringMode -or $RequireScoreValidity) -and -not $PlanOnly -and -not $SkipStartGate) {
    $gate = Invoke-TaskspaceE3StartGate `
        -RepoRoot (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path `
        -BenchmarkRoot $PSScriptRoot `
        -OutputDir (Join-Path $suiteRoot "start-gate") `
        -RunRoot $suiteRoot `
        -TaskListPath $TaskListPath `
        -SourceVersion $SourceVersion `
        -ExpectedTaskListHash $taskListHash `
        -ExpectedProfileHash $profileHash `
        -OnePairSmokeRoot $OnePairSmokeRoot `
        -SerialCalibrationRoot $SerialCalibrationRoot `
        -ParallelEquivalencePath $ParallelEquivalencePath `
        -RunSelfTests `
        -AllowSkippedPathContract `
        -AllowSkippedOnePairSmoke:$AllowSkippedOnePairSmoke
    Write-Host "E3StartGate: $($gate.json_path)"
    Write-Host "E3StartGateReport: $($gate.markdown_path)"
    if ([int]$gate.exit_code -ne 0) {
        Write-SuiteStartGateAbortHealth $gate
        Write-Host "SuiteRoot: $suiteRoot"
        Write-Host "SuiteHealth: $suiteHealthPath"
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
        first_score_invalid_run = $scoreSummary.first_score_invalid_run
        suite_score_valid = $scoreSummary.suite_score_valid
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

$runner = if ([string]::IsNullOrWhiteSpace($RunnerPath)) { Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1" } else { [System.IO.Path]::GetFullPath($RunnerPath) }
if (-not (Test-Path -LiteralPath $runner)) { Write-Error "RunnerPath not found: $runner"; exit 4 }
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
        "-SandboxMode", $SandboxMode,
        "-TaskListHash", $taskListHash,
        "-ProfileHash", $profileHash,
        "-EnableAggregate"
    )
    foreach ($override in @($ConfigOverride)) { $childArgs += @("-ConfigOverride", $override) }
    if ($AuditReviewRoot) { $childArgs += @("-AuditReviewRoot", $AuditReviewRoot) }
    if ($PlanOnly) { $childArgs += "-PlanOnly" }
    if ($ScoringMode) { $childArgs += "-ScoringMode" }
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
    if (($ScoringMode -or $RequireScoreValidity) -and $aggregatePath) {
        try {
            $aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath.FullName | ConvertFrom-Json
            if ($aggregate.PSObject.Properties.Name -contains "score_valid" -and -not [bool]$aggregate.score_valid) {
                $status = New-TaskspaceSuiteChildFailureStatus $status ([string]$Row.sample_id) ([string]$Row.task_dir) 3 ([string]$aggregatePath.FullName) ([string]$Row.sample_root)
                $status.abort_phase = "score_validity"
                $status.abort_signature = "harness_materialization_failure/score_invalid"
                $status.abort_reason = if ($aggregate.PSObject.Properties.Name -contains "score_invalid_reason") { [string]$aggregate.score_invalid_reason } else { "score_invalid" }
                $ChildExit = 3
            }
        } catch {
            if ($RequireScoreValidity -or $ScoringMode) {
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
        if (($ScoringMode -or $RequireScoreValidity) -and -not $ContinueAfterInvalidHarness) { $shouldAbortSuite = $true }
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
        Update-SuiteAbortFromStatus $completed.status ([int]$completed.child_exit) ([int]$jobRow.row.index)
    }
}

$statusText = if ($suiteAbort) { "invalid_harness" } else { "completed" }
Write-SuiteHealth $statusText @($sampleStatuses.ToArray()) $signatureCounts $suiteAbort
$suiteTimingPath = Write-TaskspaceSuiteTiming -SuiteRoot $suiteRoot -SampleStatuses @($sampleStatuses.ToArray()) -TaskListHash $taskListHash -SourceVersion $SourceVersion -ProfileHash $profileHash
$runtimeBottleneckPath = Write-TaskspaceRuntimeBottleneckReport -TimingPath $suiteTimingPath -ScoreValid (-not [bool]$suiteAbort)
$gitCommit = ""
try { $gitCommit = (& git -C (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path rev-parse HEAD 2>$null) } catch { $gitCommit = "" }
$calibrationPath = Write-TaskspaceRuntimeCalibrationReport -TimingPath $suiteTimingPath -ScoreValid (-not [bool]$suiteAbort) -CommandLine ([Environment]::CommandLine) -GitCommit ([string]$gitCommit).Trim() -ParallelismPath $parallelismPath
Write-Host "SuiteRoot: $suiteRoot"
Write-Host "SuiteHealth: $suiteHealthPath"
Write-Host "SuiteTiming: $suiteTimingPath"
Write-Host "RuntimeBottleneck: $runtimeBottleneckPath"
Write-Host "RuntimeCalibration: $calibrationPath"
Write-Host "Parallelism: $parallelismPath"
if (Test-Path -LiteralPath $skippedPath) { Write-Host "SkippedSamples: $skippedPath" }
exit $exitCode
