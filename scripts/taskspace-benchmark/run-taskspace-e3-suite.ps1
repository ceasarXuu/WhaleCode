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
    [switch]$ContinueAfterInvalidHarness
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\suite-status.ps1")
. (Join-Path $PSScriptRoot "lib\timing.ps1")
if ($Repeats -lt 5) { throw "E3 suite requires Repeats >= 5." }
if (-not (Test-Path -LiteralPath $TaskListPath)) { Write-Error "TaskListPath not found: $TaskListPath"; exit 4 }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-e3-suite-runs" }
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$suiteRoot = Join-Path $RunRoot ("suite-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $suiteRoot -Force | Out-Null
$samplesRoot = Join-Path $suiteRoot "samples"
New-Item -ItemType Directory -Path $samplesRoot -Force | Out-Null
$suiteHealthPath = Join-Path $suiteRoot "suite-health.json"
$skippedPath = Join-Path $suiteRoot "skipped-samples.jsonl"

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

$runner = Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1"
$sampleStatuses = New-Object System.Collections.Generic.List[object]
$signatureCounts = @{}
$suiteAbort = ""
$exitCode = 0

for ($index = 0; $index -lt $tasks.Count; $index++) {
    $task = $tasks[$index]
    $taskDir = if ($task.PSObject.Properties.Name -contains "task_dir") { [string]$task.task_dir } else { "" }
    $sampleId = if ($task.PSObject.Properties.Name -contains "sample_id") { [string]$task.sample_id } else { "sample-$($index + 1)" }
    $recordSourceVersion = if ($task.PSObject.Properties.Name -contains "source_version" -and -not [string]::IsNullOrWhiteSpace([string]$task.source_version)) { [string]$task.source_version } else { $SourceVersion }
    if ([string]::IsNullOrWhiteSpace($taskDir) -or [string]::IsNullOrWhiteSpace($recordSourceVersion)) {
        Write-Error "Suite sample requires task_dir and source_version/default SourceVersion: $sampleId"
        exit 4
    }
    if ($suiteAbort) {
        $skipReason = if ([string]$suiteAbort -match "disk_space_low|disk_space_threshold_invalid") { "suite_global_disk_guard" } else { "suite_repeated_infra_signature" }
        $row = [pscustomobject]@{
            sample_id = $sampleId
            task_dir = $taskDir
            run_validity = "invalid_harness"
            abort_scope = "suite"
            abort_phase = "suite_circuit_breaker"
            abort_signature = $suiteAbort
            skipped_reason = $skipReason
            sample_root = Join-Path $samplesRoot $sampleId
        }
        ($row | ConvertTo-Json -Compress) | Add-Content -LiteralPath $skippedPath -Encoding UTF8
        $sampleStatuses.Add($row)
        continue
    }
    $sampleRoot = Join-Path $samplesRoot $sampleId
    $sampleDiskHealthPath = Join-Path $suiteRoot ("suite-disk-health-{0:000}.json" -f ($index + 1))
    $sampleDiskHealth = New-TaskspaceDiskHealth @($suiteRoot, $sampleRoot, $taskDir) "suite_sample_preflight"
    Write-TaskspaceHarnessHealth $sampleDiskHealthPath $sampleDiskHealth
    if ([string]$sampleDiskHealth.status -eq "fail") {
        $firstFinding = @($sampleDiskHealth.findings | Where-Object { [string]$_.severity -eq "fail" } | Select-Object -First 1)[0]
        $signature = New-TaskspaceInfraSignature "harness_materialization_failure" "suite_sample_preflight" ([string]$firstFinding.stable_code) ([string]$firstFinding.message) "" $sampleDiskHealthPath
        $status = [pscustomobject]@{
            sample_id = $sampleId
            task_dir = $taskDir
            run_validity = "invalid_harness"
            exit_code = 3
            abort_scope = "sample"
            abort_phase = "suite_sample_preflight"
            abort_signature = $signature.key
            abort_reason = [string]$firstFinding.stable_code
            first_failure_artifact = $sampleDiskHealthPath
            sample_root = $sampleRoot
        }
        $sampleStatuses.Add($status)
        if (-not $signatureCounts.ContainsKey($signature.key)) { $signatureCounts[$signature.key] = 0 }
        $signatureCounts[$signature.key]++
        $suiteAbort = $signature.key
        $exitCode = 3
        continue
    }
    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
        "-Benchmark", $Benchmark,
        "-TaskDir", $taskDir,
        "-SampleId", $sampleId,
        "-SourceVersion", $recordSourceVersion,
        "-Repeats", $Repeats,
        "-RunRoot", $sampleRoot,
        "-WhaleBin", $WhaleBin,
        "-Model", $Model,
        "-TimeoutSeconds", $TimeoutSeconds,
        "-ValidationTimeoutSeconds", $ValidationTimeoutSeconds,
        "-ValidationPretestTimeoutSeconds", $ValidationPretestTimeoutSeconds,
        "-ValidationTestTimeoutSeconds", $ValidationTestTimeoutSeconds,
        "-SandboxMode", $SandboxMode,
        "-EnableAggregate"
    )
    foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
    if ($AuditReviewRoot) { $args += @("-AuditReviewRoot", $AuditReviewRoot) }
    if ($PlanOnly) { $args += "-PlanOnly" }
    if ($ScoringMode) { $args += "-ScoringMode" }
    if ($RequireScoreValidity) { $args += "-RequireScoreValidity" }
    & powershell @args
    $childExit = $LASTEXITCODE
    $statusPath = Get-ChildItem -LiteralPath $sampleRoot -Filter "sample-status.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $status = if ($statusPath) { Get-Content -Raw -Encoding UTF8 -LiteralPath $statusPath.FullName | ConvertFrom-Json } else { [pscustomobject]@{ sample_id = $sampleId; run_validity = if ($childExit -eq 3) { "invalid_harness" } else { "unknown" }; exit_code = $childExit } }
    if (-not ($status.PSObject.Properties.Name -contains "sample_root")) { $status | Add-Member -NotePropertyName sample_root -NotePropertyValue $sampleRoot -Force }
    $alreadyInvalidHarness = $status.PSObject.Properties.Name -contains "run_validity" -and [string]$status.run_validity -eq "invalid_harness"
    $completedDiagnosticRun = Test-TaskspaceSuiteChildStatusComplete $status $Repeats
    if ($childExit -ne 0 -and -not $alreadyInvalidHarness -and -not $completedDiagnosticRun) {
        $status = New-TaskspaceSuiteChildFailureStatus $status $sampleId $taskDir $childExit $(if ($statusPath) { [string]$statusPath.FullName } else { "" }) $sampleRoot
    }
    $aggregatePath = Get-ChildItem -LiteralPath $sampleRoot -Filter "aggregate.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (($ScoringMode -or $RequireScoreValidity) -and $aggregatePath) {
        try {
            $aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath.FullName | ConvertFrom-Json
            if ($aggregate.PSObject.Properties.Name -contains "score_valid" -and -not [bool]$aggregate.score_valid) {
                $status = New-TaskspaceSuiteChildFailureStatus $status $sampleId $taskDir 3 ([string]$aggregatePath.FullName) $sampleRoot
                $status.abort_phase = "score_validity"
                $status.abort_signature = "harness_materialization_failure/score_invalid"
                $status.abort_reason = if ($aggregate.PSObject.Properties.Name -contains "score_invalid_reason") { [string]$aggregate.score_invalid_reason } else { "score_invalid" }
                $alreadyInvalidHarness = $true
                $childExit = 3
            }
        } catch {
            if ($RequireScoreValidity -or $ScoringMode) {
                $status = New-TaskspaceSuiteChildFailureStatus $status $sampleId $taskDir 3 ([string]$aggregatePath.FullName) $sampleRoot
                $status.abort_phase = "score_validity"
                $status.abort_reason = "aggregate_score_validity_parse_failed"
                $alreadyInvalidHarness = $true
                $childExit = 3
            }
        }
    }
    $sampleStatuses.Add($status)
    if ($childExit -eq 1 -and $exitCode -eq 0) { $exitCode = 1 }
    if ($childExit -eq 2 -and $exitCode -eq 0) { $exitCode = 2 }
    if ($childExit -eq 3 -or ($status.PSObject.Properties.Name -contains "run_validity" -and [string]$status.run_validity -eq "invalid_harness")) {
        $sig = if ($status.PSObject.Properties.Name -contains "abort_signature" -and -not [string]::IsNullOrWhiteSpace([string]$status.abort_signature)) { [string]$status.abort_signature } else { "harness_materialization_failure/unknown" }
        if (-not $signatureCounts.ContainsKey($sig)) { $signatureCounts[$sig] = 0 }
        $signatureCounts[$sig]++
        if ($exitCode -eq 0) { $exitCode = 3 }
        $global = $sig -match "docker_backend_unavailable|uv_cache_missing|validator_source_missing|disk_space_low|disk_space_threshold_invalid"
        if (($ScoringMode -or $RequireScoreValidity) -and -not $ContinueAfterInvalidHarness) {
            $suiteAbort = $sig
            $exitCode = 3
        }
        if (-not $ContinueAfterInvalidHarness -and ($global -or $signatureCounts[$sig] -ge 2)) {
            $suiteAbort = $sig
            $exitCode = 3
        }
    }
}

$statusText = if ($suiteAbort) { "invalid_harness" } else { "completed" }
Write-SuiteHealth $statusText @($sampleStatuses.ToArray()) $signatureCounts $suiteAbort
$suiteTimingPath = Write-TaskspaceSuiteTiming $suiteRoot @($sampleStatuses.ToArray())
Write-Host "SuiteRoot: $suiteRoot"
Write-Host "SuiteHealth: $suiteHealthPath"
Write-Host "SuiteTiming: $suiteTimingPath"
if (Test-Path -LiteralPath $skippedPath) { Write-Host "SkippedSamples: $skippedPath" }
exit $exitCode
