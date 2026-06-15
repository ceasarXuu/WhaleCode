param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\run-state.ps1")
. (Join-Path $PSScriptRoot "lib\failure-taxonomy.ps1")
. (Join-Path $PSScriptRoot "lib\aggregate-report.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\suite-status.ps1")
. (Join-Path $PSScriptRoot "lib\timing.ps1")
. (Join-Path $PSScriptRoot "lib\runtime-bottleneck-report.ps1")
. (Join-Path $PSScriptRoot "lib\resource-governor.ps1")
. (Join-Path $PSScriptRoot "lib\parallel-diff.ps1")
. (Join-Path $PSScriptRoot "lib\calibration-gate.ps1")
. (Join-Path $PSScriptRoot "lib\calibration-selection.ps1")
. (Join-Path $PSScriptRoot "lib\runtime-reconstruction.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-guardrails-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

$stdout = Join-Path $runDir "validation.stdout.log"
$stderr = Join-Path $runDir "validation.stderr.log"
@"
validator_lifecycle_stage=entry_started
validator_lifecycle_stage=tests_started
validator_tests_started=true
validator_lifecycle_stage=tests_completed
validator_tests_completed=true
"@ | Set-Content -LiteralPath $stdout -Encoding UTF8
"" | Set-Content -LiteralPath $stderr -Encoding UTF8
$validation = [pscustomobject]@{ exit_code = 7; stdout_path = $stdout; stderr_path = $stderr }
$lifecycle = Get-TaskspaceValidationLifecycle $validation
Assert-True ([bool]$lifecycle.tests_started_seen) "lifecycle parser missed tests_started marker"
Assert-True ([bool]$lifecycle.tests_completed_seen) "lifecycle parser missed tests_completed marker"
Assert-True ([string]$lifecycle.validation_lifecycle_stage -eq "tests_completed") "lifecycle parser did not keep last stage"

$legacyRoot = Join-Path $runDir "legacy-runtime-root"
$legacyRun = Join-Path $legacyRoot "runs\terminal_bench__hello-world\20260606-014926-073"
New-Item -ItemType Directory -Force -Path (Join-Path $legacyRun "pair-001\left"), (Join-Path $legacyRun "pair-001\right"), (Join-Path $legacyRun "pair-002\left"), (Join-Path $legacyRun "pair-002\right") | Out-Null
$legacyReconstruction = Write-TaskspaceRuntimeReconstruction -SuiteRoot $legacyRoot -OutputRoot (Join-Path $runDir "legacy-runtime-reconstruction")
$legacyArtifact = $legacyReconstruction.artifact
Assert-True ([string]$legacyArtifact.suite_root -eq [System.IO.Path]::GetFullPath($legacyRoot)) "legacy reconstruction did not preserve source suite root"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$legacyArtifact.legacy_import_path)) "legacy reconstruction did not emit import artifact path"
Assert-True ([string]$legacyArtifact.bottleneck_classification -eq "unknown") "legacy reconstruction should classify missing timing as unknown"
Assert-True (@($legacyArtifact.sample_rows).Count -eq 1 -and [string]$legacyArtifact.sample_rows[0].sample_id -eq "hello-world") "legacy reconstruction did not import sample rows"
Assert-True (@($legacyArtifact.missing_fields | Where-Object { [string]$_ -eq "legacy_timing_unavailable:pair_timing" }).Count -eq 1) "legacy reconstruction did not expose missing pair timing"

$resourceConfig = New-TaskspaceResourceGovernorConfig
$serialGuard = Test-TaskspaceResourceGovernorSerialOnly $resourceConfig
Assert-True ([bool]$resourceConfig.valid -and [bool]$serialGuard.serial_only) "resource governor default config is not serial-valid"
$sampleParallelConfig = New-TaskspaceResourceGovernorConfig -MaxParallelSamples 2
$sampleParallelGuard = Test-TaskspaceResourceGovernorSerialOnly $sampleParallelConfig
Assert-True (-not [bool]$sampleParallelGuard.serial_only -and [bool]$sampleParallelGuard.sample_parallel_enabled -and @($sampleParallelGuard.unsupported_parallel_fields).Count -eq 0) "resource governor did not allow sample-level parallelism only"
$parallelConfig = New-TaskspaceResourceGovernorConfig -MaxParallelSamples 2 -MaxDockerConcurrency 2
$parallelGuard = Test-TaskspaceResourceGovernorSerialOnly $parallelConfig
Assert-True (-not [bool]$parallelGuard.serial_only -and @($parallelGuard.unsupported_parallel_fields).Count -eq 1 -and @($parallelGuard.unsupported_parallel_fields) -contains "MaxDockerConcurrency") "resource governor did not reject unsupported parallel fields"
$waitSnapshot = New-TaskspaceResourceWaitSnapshot -DockerTokenWaitMs 2 -ValidationTokenWaitMs 3 -DiskReservationWaitMs 5 -CacheLockWaitMs 7
Assert-True ([int64]$waitSnapshot.resource_wait_ms_total -eq 17) "resource governor wait snapshot did not aggregate waits"
$diskReservationPass = Test-TaskspaceDiskReservation @($runDir) 0
Assert-True ([string]$diskReservationPass.status -eq "pass") "resource governor zero-byte disk reservation should pass"
$diskReservationFail = Test-TaskspaceDiskReservation @($runDir) ([int64]::MaxValue)
Assert-True ([string]$diskReservationFail.status -eq "fail" -and [string]$diskReservationFail.failures[0].stable_code -eq "disk_reservation_insufficient") "resource governor low-disk fixture did not fail closed"
$parallelismFixtureRoot = Join-Path $runDir "parallelism-fixture"
New-Item -ItemType Directory -Force -Path $parallelismFixtureRoot | Out-Null
$parallelismPath = Write-TaskspaceParallelismArtifact $parallelismFixtureRoot $parallelConfig $parallelGuard $diskReservationFail $waitSnapshot
$parallelism = Get-Content -Raw -Encoding UTF8 -LiteralPath $parallelismPath | ConvertFrom-Json
Assert-True ([string]$parallelism.resource_governor_status -eq "blocked") "parallelism artifact did not block unsupported/low-disk fixture"
Assert-True ([int64]$parallelism.wait.resource_wait_ms_total -eq 17) "parallelism artifact did not persist wait totals"

$selectionRoot = Join-Path $runDir "calibration-selection"
New-Item -ItemType Directory -Force -Path $selectionRoot | Out-Null
$selectionTaskList = Join-Path $selectionRoot "tasks.jsonl"
@(
    ([pscustomobject]@{ sample_id = "b-1"; task_dir = (Join-Path $selectionRoot "family-b\one"); source_version = "selftest"; task_family = "b" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "a-1"; task_dir = (Join-Path $selectionRoot "family-a\one"); source_version = "selftest"; task_family = "a" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "a-2"; task_dir = (Join-Path $selectionRoot "family-a\two"); source_version = "selftest"; task_family = "a" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "c-1"; task_dir = (Join-Path $selectionRoot "family-c\one"); source_version = "selftest"; task_family = "c" } | ConvertTo-Json -Compress)
) | Set-Content -LiteralPath $selectionTaskList -Encoding UTF8
$selection = New-TaskspaceCalibrationSelection -TaskListPath $selectionTaskList -OutputPath (Join-Path $selectionRoot "calibration-selection.json") -Benchmark "terminal-bench"
Assert-True (($selection.selected_task_ids -join ",") -eq "a-1,b-1,c-1") "calibration selection did not choose one task per sorted family"
Assert-True ([string]$selection.source_task_list_hash -eq (Get-TaskspaceFileSha256 $selectionTaskList)) "calibration selection did not record source task-list hash"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$selection.subset_task_list_hash)) "calibration selection did not record subset hash"
Assert-True (@($selection.excluded_tasks | Where-Object { [string]$_.sample_id -eq "a-2" -and [string]$_.reason -eq "not_selected_after_family_coverage_limit" }).Count -eq 1) "calibration selection did not record excluded task rationale"

$reconstructRoot = Join-Path $runDir "runtime-reconstruct-fixture"
$reconstructSamples = Join-Path $reconstructRoot "samples"
New-Item -ItemType Directory -Force -Path (Join-Path $reconstructSamples "sample-a"), (Join-Path $reconstructSamples "sample-b"), (Join-Path $reconstructSamples "sample-c") | Out-Null
@{ total_pair_duration_ms = 1000; agent_duration_ms = 100; public_validation_duration_ms = 900; docker_build_duration_ms = 0; docker_run_duration_ms = 0; docker_cleanup_duration_ms = 0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reconstructSamples "sample-a\sample-timing.json") -Encoding UTF8
@{ total_pair_duration_ms = 2000; agent_duration_ms = 1500; public_validation_duration_ms = 500; docker_build_duration_ms = 0; docker_run_duration_ms = 0; docker_cleanup_duration_ms = 0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reconstructSamples "sample-b\sample-timing.json") -Encoding UTF8
[pscustomobject]@{
    status = "invalid_harness"
    sample_statuses = @(
        [pscustomobject]@{ sample_id = "sample-a"; run_validity = "invalid_harness"; sample_root = (Join-Path $reconstructSamples "sample-a") },
        [pscustomobject]@{ sample_id = "sample-b"; run_validity = "valid"; sample_root = (Join-Path $reconstructSamples "sample-b") },
        [pscustomobject]@{ sample_id = "sample-c"; run_validity = "invalid_harness"; skipped_reason = "previous_engineering_unclean"; sample_root = (Join-Path $reconstructSamples "sample-c") }
    )
} | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $reconstructRoot "suite-health.json") -Encoding UTF8
[pscustomobject]@{ total_pair_duration_ms = 3000; agent_duration_ms = 1600; public_validation_duration_ms = 1400; docker_build_duration_ms = 0; docker_run_duration_ms = 0; docker_cleanup_duration_ms = 0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reconstructRoot "suite-timing.json") -Encoding UTF8
$reconstructOutput = Join-Path $reconstructRoot "runtime-reconstruction\selftest"
$reconstruction = Write-TaskspaceRuntimeReconstruction -SuiteRoot $reconstructRoot -OutputRoot $reconstructOutput
Assert-True ((Test-Path -LiteralPath $reconstruction.json_path) -and (Test-Path -LiteralPath $reconstruction.markdown_path)) "runtime reconstruction did not write artifacts"
Assert-True ([int64]$reconstruction.artifact.time_after_first_invalid_ms -eq 2000) "runtime reconstruction did not compute first-invalid waste"
Assert-True ([string]$reconstruction.artifact.bottleneck_classification -eq "invalid_waste_bound") "runtime reconstruction did not classify invalid waste"
Assert-True (([System.IO.Path]::GetFullPath($reconstruction.artifact.output_root)).StartsWith([System.IO.Path]::GetFullPath((Join-Path $reconstructRoot "runtime-reconstruction")))) "runtime reconstruction did not write under isolated output root"

$pretestStderr = Join-Path $runDir "pretest.stderr.log"
"Resolve-Path : Cannot find path 'target\bad-uv-cache'" | Set-Content -LiteralPath $pretestStderr -Encoding UTF8
$sig = Get-TaskspaceHarnessTextSignature (Get-Content -Raw -Encoding UTF8 -LiteralPath $pretestStderr) "validator_pretest" "left" $pretestStderr
Assert-True ($sig -and [string]$sig.stable_code -eq "path_unresolvable") "stderr fallback did not classify path_unresolvable"

$wslStderr = Join-Path $runDir "wsl.stderr.log"
"external-validator.ps1 : <3>WSL (1148 - Relay) ERROR: CreateProcessParseCommon:1014: getpwnam(root) failed 5" | Set-Content -LiteralPath $wslStderr -Encoding UTF8
$wslSig = Get-TaskspaceHarnessTextSignature (Get-Content -Raw -Encoding UTF8 -LiteralPath $wslStderr) "validator_pretest" "left" $wslStderr
Assert-True ($wslSig -and [string]$wslSig.stable_code -eq "docker_backend_unavailable") "stderr fallback did not classify WSL root lookup as docker_backend_unavailable"

$standardMetrics = [pscustomobject]@{
    mode = "left"
    logical_mode = "standard"
    public_validation_exit_code = 1
    pretest_failure = $true
    validation_stderr_path = $pretestStderr
    validator_environment_failures = @("path_unresolvable")
    infra_signature = $sig
}
$taskspaceMetrics = [pscustomobject]@{
    mode = "right"
    logical_mode = "taskspace"
    public_validation_exit_code = 1
    pretest_failure = $true
    validation_stderr_path = $pretestStderr
    validator_environment_failures = @("path_unresolvable")
    infra_signature = $sig
}
$sentinel = Get-TaskspaceSentinelAbortDecision $standardMetrics $taskspaceMetrics
Assert-True ([bool]$sentinel.abort -and [string]$sentinel.reason -eq "same_infra_signature_both_sides") "sentinel did not abort same infra signature"

$afterTestsMetrics = [pscustomobject]@{
    mode = "right"
    logical_mode = "taskspace"
    public_validation_exit_code = 1
    pretest_failure = $false
    tests_started_seen = $true
    validator_environment_failures = @()
    infra_signature = $null
}
$standardAfterTestsMetrics = [pscustomobject]@{
    mode = "left"
    logical_mode = "standard"
    public_validation_exit_code = 1
    pretest_failure = $false
    tests_started_seen = $true
    validator_environment_failures = @()
    infra_signature = $null
}
$noAbort = Get-TaskspaceSentinelAbortDecision $standardAfterTestsMetrics $afterTestsMetrics
Assert-True (-not [bool]$noAbort.abort) "sentinel aborted a failure that reached tests_started"

$manifest = [pscustomobject]@{
    PromptPath = Join-Path $runDir "prompt.txt"
    FixtureDir = Join-Path $runDir "fixture"
    ExternalBenchmark = [pscustomobject]@{
        adapter_metadata = [pscustomobject]@{
            uv_cache_root = "relative\uv-cache"
            validator_source_dir = Join-Path $runDir "validator-source"
            fixture_source = Join-Path $runDir "fixture"
        }
    }
}
"prompt" | Set-Content -LiteralPath $manifest.PromptPath -Encoding UTF8
New-Item -ItemType Directory -Force -Path $manifest.FixtureDir | Out-Null
New-Item -ItemType Directory -Force -Path $manifest.ExternalBenchmark.adapter_metadata.validator_source_dir | Out-Null
$health = Get-TaskspaceHarnessHealth $manifest $runDir $runDir
Assert-True ([string]$health.status -eq "fail") "preflight health did not fail relative materialized path"
Assert-True (@($health.findings | Where-Object { [string]$_.stable_code -eq "relative_materialized_path" }).Count -gt 0) "preflight health did not name relative_materialized_path"

$manifest.ExternalBenchmark.adapter_metadata.uv_cache_root = Join-Path $runDir "uv-cache"
New-Item -ItemType Directory -Force -Path $manifest.ExternalBenchmark.adapter_metadata.uv_cache_root | Out-Null
$oldMinFreeBytes = $env:TASKSPACE_MIN_FREE_BYTES
$oldMinFreeGib = $env:TASKSPACE_MIN_FREE_GIB
try {
    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue
    $spacePassHealth = Get-TaskspaceHarnessHealth $manifest $runDir $runDir
    Assert-True ([string]$spacePassHealth.status -eq "pass") "disk preflight failed with a 1 byte minimum"
    Assert-True (@($spacePassHealth.disk_space_checks).Count -gt 0) "disk preflight did not record checked disks"

    $env:TASKSPACE_MIN_FREE_BYTES = ([int64]::MaxValue).ToString()
    $spaceFailHealth = Get-TaskspaceHarnessHealth $manifest $runDir $runDir
    $spaceFinding = @($spaceFailHealth.findings | Where-Object { [string]$_.stable_code -eq "disk_space_low" } | Select-Object -First 1)[0]
    Assert-True ([string]$spaceFailHealth.status -eq "fail") "disk preflight did not fail with impossible minimum"
    Assert-True ($spaceFinding -and [int64]$spaceFinding.required_free_bytes -eq [int64]::MaxValue) "disk preflight did not record disk_space_low finding details"
    $spaceSig = New-TaskspaceInfraSignature "harness_materialization_failure" "preflight" "disk_space_low" "Disk space low" "" $runDir
    Assert-True (Test-TaskspaceHardInfraSignature $spaceSig) "disk_space_low was not treated as a hard infra signature"
} finally {
    if ($null -eq $oldMinFreeBytes) { Remove-Item Env:TASKSPACE_MIN_FREE_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_BYTES = $oldMinFreeBytes }
    if ($null -eq $oldMinFreeGib) { Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_GIB = $oldMinFreeGib }
}

$suiteSkipRoot = Join-Path $runDir "suite-skip"
$suiteTaskA = Join-Path $suiteSkipRoot "task-a"
$suiteTaskB = Join-Path $suiteSkipRoot "task-b"
New-Item -ItemType Directory -Force -Path $suiteTaskA, $suiteTaskB | Out-Null
$suiteTaskList = Join-Path $suiteSkipRoot "tasks.jsonl"
@(
    ([pscustomobject]@{ sample_id = "sample-a"; task_dir = $suiteTaskA; source_version = "selftest" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "sample-b"; task_dir = $suiteTaskB; source_version = "selftest" } | ConvertTo-Json -Compress)
) | Set-Content -LiteralPath $suiteTaskList -Encoding UTF8
$oldSuiteMinFreeBytes = $env:TASKSPACE_MIN_FREE_BYTES
$oldSuiteMinFreeGib = $env:TASKSPACE_MIN_FREE_GIB
try {
    $env:TASKSPACE_MIN_FREE_BYTES = ([int64]::MaxValue).ToString()
    Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue
    $suiteOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $suiteTaskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $suiteSkipRoot "runs") -PlanOnly -ScoringMode 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite disk guard did not exit invalid_harness"
    $suiteRootLine = @($suiteOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteRunRoot = ([string]$suiteRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRunRoot "suite-health.json") | ConvertFrom-Json
    $skippedStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRunRoot "samples\sample-b\sample-status.json") | ConvertFrom-Json
    $suiteEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $suiteRunRoot "events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json })
    $scoreInvalidatedEvents = @($suiteEvents | Where-Object { [string]$_.event -eq "suite_score_invalidated" })
    Assert-True ([string]$suiteHealth.status -eq "invalid_harness" -and -not [bool]$suiteHealth.suite_score_valid) "suite health did not record invalid suite score"
    Assert-True ([int]$suiteHealth.remaining_samples_skipped -eq 1 -and [int]$suiteHealth.score_invalid_child_runs -eq 2) "suite health did not count skipped invalid sample"
    Assert-True ($null -eq $suiteHealth.expected_time_saved_minutes -and [string]$suiteHealth.expected_time_saved_basis -eq "no_serial_baseline") "suite health did not explain missing time-saved baseline"
    Assert-True ($scoreInvalidatedEvents.Count -eq 1 -and [int]$scoreInvalidatedEvents[0].remaining_samples_skipped -eq 1) "suite did not emit one score invalidated event with skipped sample count"
    Assert-True ([string]$skippedStatus.phase -eq "skipped" -and [string]$skippedStatus.abort_phase -eq "suite_circuit_breaker") "skipped sample status did not record suite circuit breaker"
} finally {
    if ($null -eq $oldSuiteMinFreeBytes) { Remove-Item Env:TASKSPACE_MIN_FREE_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_BYTES = $oldSuiteMinFreeBytes }
    if ($null -eq $oldSuiteMinFreeGib) { Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_GIB = $oldSuiteMinFreeGib }
}

$suiteChildInvalidRoot = Join-Path $runDir "suite-child-invalid"
$suiteChildTaskA = Join-Path $suiteChildInvalidRoot "task-a"
$suiteChildTaskB = Join-Path $suiteChildInvalidRoot "task-b"
$suiteChildStubRunner = Join-Path $suiteChildInvalidRoot "stub-runner.ps1"
New-Item -ItemType Directory -Force -Path $suiteChildTaskA, $suiteChildTaskB | Out-Null
$suiteChildTaskList = Join-Path $suiteChildInvalidRoot "tasks.jsonl"
@(
    ([pscustomobject]@{ sample_id = "sample-a"; task_dir = $suiteChildTaskA; source_version = "selftest" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "sample-b"; task_dir = $suiteChildTaskB; source_version = "selftest" } | ConvertTo-Json -Compress)
) | Set-Content -LiteralPath $suiteChildTaskList -Encoding UTF8
@'
param(
    [string]$Benchmark,
    [string]$TaskDir,
    [string]$SampleId,
    [string]$SourceVersion,
    [int]$Repeats,
    [string]$RunRoot,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null
[pscustomobject]@{
    schema_version = 1
    sample_id = $SampleId
    phase = "invalid_harness"
    run_validity = "invalid_harness"
    exit_code = 3
    abort_scope = "sample"
    abort_phase = "score_validity"
    abort_signature = "harness_materialization_failure/stub_score_invalid"
    abort_reason = "stub_score_invalid"
    first_failure_artifact = $TaskDir
    sample_root = $RunRoot
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $RunRoot "sample-status.json") -Encoding UTF8
exit 3
'@ | Set-Content -LiteralPath $suiteChildStubRunner -Encoding UTF8
$suiteChildOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $suiteChildTaskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $suiteChildInvalidRoot "runs") -RunnerPath $suiteChildStubRunner -PlanOnly -ScoringMode -SkipStartGate 2>&1
Assert-True ($LASTEXITCODE -eq 3) "suite child invalid fixture did not exit invalid_harness"
$suiteChildRootLine = @($suiteChildOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
$suiteChildRunRoot = ([string]$suiteChildRootLine) -replace "^SuiteRoot:\s*", ""
$suiteChildHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteChildRunRoot "suite-health.json") | ConvertFrom-Json
$suiteChildSkipped = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteChildRunRoot "samples\sample-b\sample-status.json") | ConvertFrom-Json
$suiteChildEvents = @(Get-Content -Encoding UTF8 -LiteralPath (Join-Path $suiteChildRunRoot "events.jsonl") | ForEach-Object { $_ | ConvertFrom-Json })
$suiteChildInvalidEvents = @($suiteChildEvents | Where-Object { [string]$_.event -eq "suite_score_invalidated" })
Assert-True ([string]$suiteChildHealth.status -eq "invalid_harness" -and -not [bool]$suiteChildHealth.suite_score_valid) "suite child invalid fixture did not invalidate suite score"
Assert-True ([int]$suiteChildHealth.remaining_samples_skipped -eq 1 -and [int]$suiteChildHealth.score_invalid_child_runs -eq 2) "suite child invalid fixture did not count skipped sample"
Assert-True ([string]$suiteChildSkipped.phase -eq "skipped" -and [string]$suiteChildSkipped.abort_signature -eq "harness_materialization_failure/stub_score_invalid") "suite child invalid fixture did not skip second sample with first invalid signature"
Assert-True ($suiteChildInvalidEvents.Count -eq 1 -and [int]$suiteChildInvalidEvents[0].remaining_samples_skipped -eq 1) "suite child invalid fixture did not emit score invalidated event"

$parallelSuiteRoot = Join-Path $runDir "suite-parallel"
$parallelTaskList = Join-Path $parallelSuiteRoot "tasks.jsonl"
$parallelStubRunner = Join-Path $parallelSuiteRoot "stub-runner.ps1"
$parallelTaskA = Join-Path $parallelSuiteRoot "task-a"
$parallelTaskB = Join-Path $parallelSuiteRoot "task-b"
$parallelTaskC = Join-Path $parallelSuiteRoot "task-c"
New-Item -ItemType Directory -Force -Path $parallelTaskA, $parallelTaskB, $parallelTaskC | Out-Null
@(
    ([pscustomobject]@{ sample_id = "sample-a"; task_dir = $parallelTaskA; source_version = "selftest" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "sample-b"; task_dir = $parallelTaskB; source_version = "selftest" } | ConvertTo-Json -Compress),
    ([pscustomobject]@{ sample_id = "sample-c"; task_dir = $parallelTaskC; source_version = "selftest" } | ConvertTo-Json -Compress)
) | Set-Content -LiteralPath $parallelTaskList -Encoding UTF8
@'
param(
    [string]$Benchmark,
    [string]$TaskDir,
    [string]$SampleId,
    [string]$SourceVersion,
    [int]$Repeats,
    [string]$RunRoot,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null
if ($SampleId -eq "sample-a") { Start-Sleep -Seconds 2 }
[pscustomobject]@{
    sample_id = $SampleId
    task_dir = $TaskDir
    phase = "completed"
    run_validity = "valid"
    exit_code = 0
    attempted_pairs = $Repeats
    completed_pairs = $Repeats
    sample_root = $RunRoot
    source_version = $SourceVersion
    benchmark = $Benchmark
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $RunRoot "sample-status.json") -Encoding UTF8
exit 0
'@ | Set-Content -LiteralPath $parallelStubRunner -Encoding UTF8
$serialOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $parallelTaskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $parallelSuiteRoot "serial-runs") -PlanOnly -ScoringMode -SkipStartGate -RunnerPath $parallelStubRunner 2>&1
Assert-True ($LASTEXITCODE -eq 0) "serial suite baseline for parallel smoke failed: $($serialOutput -join ' | ')"
$serialSuiteRootLine = @($serialOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
$serialRunRoot = ([string]$serialSuiteRootLine) -replace "^SuiteRoot:\s*", ""
$serialSelectionPath = Join-Path $serialRunRoot "calibration-selection.json"
Assert-True (Test-Path -LiteralPath $serialSelectionPath) "suite did not write calibration-selection artifact"
$serialSelection = Get-Content -Raw -Encoding UTF8 -LiteralPath $serialSelectionPath | ConvertFrom-Json
Assert-True (@($serialSelection.selected_task_ids).Count -eq 3 -and -not [string]::IsNullOrWhiteSpace([string]$serialSelection.source_task_list_hash)) "suite calibration-selection artifact is incomplete"
$parallelOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $parallelTaskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $parallelSuiteRoot "runs") -PlanOnly -ScoringMode -SkipStartGate -RunnerPath $parallelStubRunner -MaxParallelSamples 2 2>&1
Assert-True ($LASTEXITCODE -eq 0) "sample-level parallel suite smoke failed: $($parallelOutput -join ' | ')"
$parallelSuiteRootLine = @($parallelOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
$parallelRunRoot = ([string]$parallelSuiteRootLine) -replace "^SuiteRoot:\s*", ""
$parallelHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $parallelRunRoot "suite-health.json") | ConvertFrom-Json
$parallelismSmoke = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $parallelRunRoot "parallelism.json") | ConvertFrom-Json
$mergedSampleIds = @($parallelHealth.sample_statuses | ForEach-Object { [string]$_.sample_id })
Assert-True ([string]$parallelHealth.status -eq "completed" -and [bool]$parallelHealth.suite_score_valid) "parallel suite smoke did not complete as score-valid"
Assert-True (($mergedSampleIds -join ",") -eq "sample-a,sample-b,sample-c") "parallel suite merge order was not deterministic"
Assert-True ([string]$parallelismSmoke.serial_only_status -eq "sample_parallel_supported" -and [bool]$parallelismSmoke.sample_parallel_enabled -and [int]$parallelismSmoke.configured.max_parallel_samples -eq 2) "parallelism artifact did not record sample-level parallel mode"
Assert-True ([int]$parallelismSmoke.observed.max_parallel_samples -eq 2) "parallelism artifact did not record observed sample-level parallelism"
Assert-True ((Test-Path -LiteralPath (Join-Path $parallelRunRoot "samples\sample-a\sample-status.json")) -and (Test-Path -LiteralPath (Join-Path $parallelRunRoot "samples\sample-b\sample-status.json")) -and (Test-Path -LiteralPath (Join-Path $parallelRunRoot "samples\sample-c\sample-status.json"))) "parallel suite smoke did not isolate sample artifacts"
$equivalencePath = Join-Path $parallelSuiteRoot "serial-vs-parallel-equivalence.json"
$equivalence = Write-TaskspaceSuiteScoreEquivalence -SerialSuiteHealthPath (Join-Path $serialRunRoot "suite-health.json") -ParallelSuiteHealthPath (Join-Path $parallelRunRoot "suite-health.json") -OutputPath $equivalencePath -TaskListHash "task-list-a" -SourceVersion "source-a" -ProfileHash "profile-a" -RequiredSampleFields @("run_validity", "phase")
Assert-True ([bool]$equivalence.comparable -and -not [bool]$equivalence.parallel_smoke_score_drift -and [int]$equivalence.drift_count -eq 0) "serial-vs-parallel equivalence reported unexpected drift"
Assert-True ([string]$equivalence.task_list_hash -eq "task-list-a" -and [string]$equivalence.source_version -eq "source-a" -and [string]$equivalence.profile_hash -eq "profile-a") "serial-vs-parallel equivalence did not preserve identity fields"
$driftFixture = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $parallelRunRoot "suite-health.json") | ConvertFrom-Json
$driftFixture.sample_statuses[1].run_validity = "invalid_harness"
$driftResult = Compare-TaskspaceSuiteScoreEquivalence (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $serialRunRoot "suite-health.json") | ConvertFrom-Json) $driftFixture
Assert-True (-not [bool]$driftResult.comparable -and [bool]$driftResult.parallel_smoke_score_drift -and @($driftResult.drifts | Where-Object { [string]$_.scope -eq "sample:sample-b" -and [string]$_.field -eq "run_validity" }).Count -eq 1) "serial-vs-parallel equivalence did not detect sample score drift"
$requiredFieldResult = Compare-TaskspaceSuiteScoreEquivalence (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $serialRunRoot "suite-health.json") | ConvertFrom-Json) (Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $parallelRunRoot "suite-health.json") | ConvertFrom-Json) @("prompt_hash", "config_hash", "proof_status")
Assert-True (-not [bool]$requiredFieldResult.comparable -and [bool]$requiredFieldResult.parallel_smoke_score_drift -and @($requiredFieldResult.drifts | Where-Object { [string]$_.serial -eq "<missing>" -or [string]$_.parallel -eq "<missing>" }).Count -gt 0) "serial-vs-parallel equivalence silently passed missing required sample fields"

$stateRun = Join-Path $runDir "invalid-state"
New-Item -ItemType Directory -Force -Path $stateRun | Out-Null
Initialize-TaskspaceBenchmarkRunState $stateRun "sample" 5 "E3" "selftest" | Out-Null
Set-TaskspaceInvalidHarnessStatus $stateRun "sample" "sentinel_pair" "same_infra_signature_both_sides" $sig $pretestStderr "selftest" 1 1 | Out-Null
$runStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stateRun "run-status.json") | ConvertFrom-Json
$sampleStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $stateRun "sample-status.json") | ConvertFrom-Json
Assert-True ([string]$runStatus.run_validity -eq "invalid_harness" -and [int]$runStatus.exit_code -eq 3) "invalid run status did not record exit code 3"
Assert-True ([string]$sampleStatus.run_validity -eq "invalid_harness" -and [int]$sampleStatus.exit_code -eq 3) "invalid sample status did not record invalid_harness exit code 3"
Assert-True (-not [bool]$sampleStatus.resume_allowed -and [string]$sampleStatus.abort_phase -eq "sentinel_pair") "invalid sample status did not block resume"

$completeDiagnosticStatus = [pscustomobject]@{
    sample_id = "diagnostic-complete"
    run_validity = "valid"
    phase = "audit_required"
    attempted_pairs = 5
    completed_pairs = 5
}
Assert-True (Test-TaskspaceSuiteChildStatusComplete $completeDiagnosticStatus 5) "suite status helper did not preserve completed audit_required diagnostic run"
$incompleteStatus = [pscustomobject]@{
    sample_id = "diagnostic-incomplete"
    run_validity = "valid"
    phase = "execute"
    attempted_pairs = 3
    completed_pairs = 3
}
Assert-True (-not (Test-TaskspaceSuiteChildStatusComplete $incompleteStatus 5)) "suite status helper accepted incomplete child status"
$childFailure = New-TaskspaceSuiteChildFailureStatus $incompleteStatus "diagnostic-incomplete" $runDir 1 "" $runDir
Assert-True ([string]$childFailure.run_validity -eq "invalid_harness" -and [string]$childFailure.abort_signature -eq "harness_materialization_failure/child_process_failed") "suite status helper did not synthesize child process failure"

$externalExitRoot = Join-Path $runDir "external-exit-propagation"
$externalTask = Join-Path $externalExitRoot "task"
$stubRunner = Join-Path $externalExitRoot "stub-runner.ps1"
New-Item -ItemType Directory -Force -Path $externalTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $externalTask "task.yaml") -Encoding UTF8
@'
FROM scratch
'@ | Set-Content -LiteralPath (Join-Path $externalTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $externalTask "run-tests.sh") -Encoding UTF8
@'
param(
    [string]$ScenarioPath,
    [int]$Repeats,
    [string]$RunRoot,
    [string]$SourceVersion
)
$ErrorActionPreference = "Stop"
$childRoot = Join-Path $RunRoot "stub-child"
New-Item -ItemType Directory -Force -Path $childRoot | Out-Null
[pscustomobject]@{
    schema_version = 1
    sample_id = "external-exit"
    phase = "invalid_harness"
    run_validity = "invalid_harness"
    exit_code = 3
    abort_scope = "sample"
    abort_phase = "score_validity"
    abort_signature = "harness_materialization_failure/stub_invalid"
    abort_reason = "stub_invalid"
    first_failure_artifact = $ScenarioPath
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $childRoot "sample-status.json") -Encoding UTF8
exit 3
'@ | Set-Content -LiteralPath $stubRunner -Encoding UTF8
$externalOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $externalTask -SampleId "external-exit" -SourceVersion selftest -Repeats 1 -RunRoot (Join-Path $externalExitRoot "runs") -RunnerPath $stubRunner -ScoringMode -EnableAggregate 2>&1
Assert-True ($LASTEXITCODE -eq 3) "external benchmark wrapper did not propagate invalid_harness child exit code 3"
$externalStatusPath = Get-ChildItem -LiteralPath (Join-Path $externalExitRoot "runs") -Filter "sample-status.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Assert-True ($null -ne $externalStatusPath) "external benchmark wrapper test did not leave child sample-status evidence"
$externalStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $externalStatusPath.FullName | ConvertFrom-Json
Assert-True ([string]$externalStatus.run_validity -eq "invalid_harness" -and [int]$externalStatus.exit_code -eq 3) "external benchmark wrapper child evidence did not preserve invalid_harness exit code"

$aggregatePath = Join-Path $runDir "aggregate-report.md"
$evidence = [pscustomobject]@{
    reported_evidence_level = "E3-candidate"
    included_in_utility_aggregate = $false
    included_in_e3_aggregate = $false
    evidence_gate_failures = @()
    e3_gate_failures = @("path_unresolvable")
    failure_taxonomy = @("harness_materialization_failure")
    utility_direction = "inconclusive"
    human_review_completed = $false
    human_review_decision = ""
    human_review_disagreement = $false
}
Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @([pscustomobject]@{ repeat = 1; pair_dir = $runDir; pair_report = "pair-report.md"; evidence_target = "E3"; evidence = $evidence })
$aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "aggregate.json") | ConvertFrom-Json
$aggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath
Assert-True ([string]$aggregate.run_validity -eq "invalid_harness") "aggregate did not mark invalid_harness"
Assert-True (-not [bool]$aggregate.diagnostic_comparison_enabled) "aggregate did not disable diagnostic comparison"
Assert-True ($aggregateText -match "diagnostic_comparison_enabled: False") "aggregate report did not render comparison-disabled status"
Assert-True ($aggregateText -notmatch "taskspace_better|standard_better|regressed|worse") "invalid harness aggregate rendered directional comparison wording"

$timingRun = Join-Path $runDir "timing"
New-Item -ItemType Directory -Force -Path $timingRun | Out-Null
$pairTimingDir = Join-Path $timingRun "pair-001"
New-Item -ItemType Directory -Force -Path $pairTimingDir | Out-Null
$pairStart = [datetime]"2026-06-14T00:00:00Z"
$pairEnd = $pairStart.AddSeconds(10)
$timingMetrics = @{
    left = [pscustomobject]@{
        logical_mode = "standard"; wall_time_ms = 2000; exec_exit_code = 0; exec_timed_out = $false
        validator_environment_failures = @(); docker_build_duration_ms = 1600; docker_run_duration_ms = 200; docker_cleanup_duration_ms = 100; docker_cache_key = "cache-a"; model_request_duration_ms = 700; docker_cache_lock_wait_ms = 3
    }
    right = [pscustomobject]@{
        logical_mode = "taskspace"; wall_time_ms = 2000; exec_exit_code = 0; exec_timed_out = $false
        validator_environment_failures = @(); docker_build_duration_ms = 100; docker_run_duration_ms = 200; docker_cleanup_duration_ms = 100; docker_cache_key = "cache-a"; model_request_duration_ms = 300; docker_cache_lock_wait_ms = 7
    }
}
$timingValidation = @{
    left = [pscustomobject]@{ logical_mode = "standard"; validation_started_at = $pairStart.AddSeconds(4); validation_finished_at = $pairStart.AddSeconds(4.5); validation_exit_code = 0; oracle_started_at = $pairStart.AddSeconds(4.5); oracle_finished_at = $pairStart.AddSeconds(4.75); oracle_exit_code = 0; engineering_unclean_reasons = @() }
    right = [pscustomobject]@{ logical_mode = "taskspace"; validation_started_at = $pairStart.AddSeconds(5); validation_finished_at = $pairStart.AddSeconds(5.5); validation_exit_code = 0; oracle_started_at = $pairStart.AddSeconds(5.5); oracle_finished_at = $pairStart.AddSeconds(5.75); oracle_exit_code = 0; engineering_unclean_reasons = @() }
}
Write-TaskspacePairTiming -PairDir $pairTimingDir -Repeat 1 -PairStartedAt $pairStart -PairFinishedAt $pairEnd -Manifest ([pscustomobject]@{ Id = "timing-sample" }) -Pair $null -MetricsBySide $timingMetrics -ValidationTimingBySide $timingValidation | Out-Null
$pairTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $pairTimingDir "pair-timing.json") | ConvertFrom-Json
Assert-True ([string]$pairTiming.bottleneck_classification -eq "docker_build_bound") "pair timing did not classify docker build bottleneck"
Assert-True ([double]$pairTiming.timing_breakdown.subtotal_percentages.docker_build -eq 17.0) "pair timing docker build percentage was not deterministic"
Assert-True ([string]$pairTiming.timing_breakdown.largest_span.name -eq "agent") "pair timing largest span did not identify agent subtotal"
Assert-True ([string]$pairTiming.timing_breakdown.top_spans[0].name -eq "agent") "pair timing did not render sorted top spans"
Assert-True (@($pairTiming.docker_cache_keys).Count -eq 1 -and [string]$pairTiming.docker_cache_keys[0] -eq "cache-a") "pair timing did not record unique Docker cache key"
Assert-True ([int64]$pairTiming.model_request_duration_ms -eq 1000 -and @($pairTiming.wait_attribution_missing_fields | Where-Object { [string]$_ -eq "model_request_duration_ms" }).Count -eq 0) "pair timing did not aggregate observed model request duration"
Assert-True ([int64]$pairTiming.cache_lock_wait_ms -eq 10 -and [int64]$pairTiming.resource_wait_ms_total -eq 10 -and [string]$pairTiming.resource_wait_attribution_mode -eq "serial_with_cache_lock_observed") "pair timing did not aggregate observed cache lock wait"

$unclean = Get-TaskspaceTimingBottleneck 10000 1000 1000 0 0 0 0 @("docker_run_failure")
Assert-True ([string]$unclean.classification -eq "engineering_unclean_slow") "timing bottleneck did not prioritize engineering unclean"
$agentBound = Get-TaskspaceTimingBottleneck 10000 7000 1000 0 0 0 0 @()
Assert-True ([string]$agentBound.classification -eq "agent_bound") "timing bottleneck did not classify agent_bound"
$validatorBound = Get-TaskspaceTimingBottleneck 10000 1000 3000 0 0 0 0 @()
Assert-True ([string]$validatorBound.classification -eq "validator_bound") "timing bottleneck did not classify validator_bound"
$cleanupBound = Get-TaskspaceTimingBottleneck 10000 1000 1000 0 0 500 0 @()
Assert-True ([string]$cleanupBound.classification -eq "cleanup_bound") "timing bottleneck did not classify cleanup_bound"

$pairTimingDir2 = Join-Path $timingRun "pair-002"
New-Item -ItemType Directory -Force -Path $pairTimingDir2 | Out-Null
$timingMetrics.left.docker_build_duration_ms = 800
$timingMetrics.right.docker_build_duration_ms = 100
Write-TaskspacePairTiming -PairDir $pairTimingDir2 -Repeat 2 -PairStartedAt $pairStart -PairFinishedAt $pairStart.AddSeconds(20) -Manifest ([pscustomobject]@{ Id = "timing-sample" }) -Pair $null -MetricsBySide $timingMetrics -ValidationTimingBySide $timingValidation | Out-Null

Write-TaskspaceSampleTiming $timingRun "timing-sample" | Out-Null
$sampleTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $timingRun "sample-timing.json") | ConvertFrom-Json
Assert-True ([int64]$sampleTiming.docker_build_duration_ms -eq 2600) "sample timing did not aggregate docker build duration"
Assert-True ([int]$sampleTiming.bottleneck_counts.docker_build_bound -eq 1) "sample timing did not aggregate bottleneck count"
Assert-True ([int64]$sampleTiming.phase_distributions.total.median_ms -eq 10000 -and [int64]$sampleTiming.phase_distributions.total.p95_ms -eq 20000) "sample timing did not compute deterministic median/p95"
Assert-True ([int]$sampleTiming.docker_cache_key_counts."cache-a" -eq 2 -and [string]$sampleTiming.repeated_docker_cache_keys[0] -eq "cache-a") "sample timing did not detect repeated Docker cache key"
Assert-True ([int64]$sampleTiming.model_request_duration_ms -eq 2000 -and @($sampleTiming.wait_attribution_missing_fields | Where-Object { [string]$_ -eq "model_request_duration_ms" }).Count -eq 0) "sample timing did not aggregate observed model request duration"
Assert-True ([int64]$sampleTiming.resource_wait_ms_total -eq 20 -and @($sampleTiming.wait_attribution_missing_fields | Where-Object { [string]$_ -eq "resource_wait_ms_total" }).Count -eq 0) "sample timing did not aggregate observed cache lock wait"

$suiteRoot = Join-Path $runDir "timing-suite"
$suiteSampleRoot = Join-Path $suiteRoot "samples\timing-sample"
New-Item -ItemType Directory -Force -Path $suiteSampleRoot | Out-Null
Copy-Item -LiteralPath (Join-Path $timingRun "sample-timing.json") -Destination (Join-Path $suiteSampleRoot "sample-timing.json") -Force
Write-TaskspaceSuiteTiming -SuiteRoot $suiteRoot -SampleStatuses @([pscustomobject]@{ sample_root = $suiteSampleRoot }) -TaskListHash "task-list-a" -SourceVersion "source-a" -ProfileHash "profile-a" | Out-Null
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRoot "suite-timing.json") | ConvertFrom-Json
Assert-True ([int64]$suiteTiming.docker_build_duration_ms -eq 2600) "suite timing did not aggregate docker build duration"
Assert-True ([int]$suiteTiming.bottleneck_counts.docker_build_bound -eq 1) "suite timing did not aggregate bottleneck count"
Assert-True ([string]$suiteTiming.task_list_hash -eq "task-list-a" -and [string]$suiteTiming.source_version -eq "source-a" -and [string]$suiteTiming.profile_hash -eq "profile-a") "suite timing did not preserve identity fields"
Assert-True ([int]$suiteTiming.docker_cache_key_counts."cache-a" -eq 2 -and [string]$suiteTiming.repeated_docker_cache_keys[0] -eq "cache-a") "suite timing did not aggregate repeated Docker cache keys"
Assert-True ([int64]$suiteTiming.model_request_duration_ms -eq 2000 -and @($suiteTiming.wait_attribution_missing_fields | Where-Object { [string]$_ -eq "model_request_duration_ms" }).Count -eq 0) "suite timing did not aggregate observed model request duration"
Assert-True ([int64]$suiteTiming.resource_wait_ms_total -eq 20 -and @($suiteTiming.wait_attribution_missing_fields | Where-Object { [string]$_ -eq "resource_wait_ms_total" }).Count -eq 0) "suite timing did not aggregate observed cache lock wait"
$runtimeBottleneckPath = Write-TaskspaceRuntimeBottleneckReport -TimingPath (Join-Path $suiteRoot "suite-timing.json") -ScoreValid $true
$runtimeBottleneckText = Get-Content -Raw -Encoding UTF8 -LiteralPath $runtimeBottleneckPath
$runtimeBottleneckJson = Get-Content -Raw -Encoding UTF8 -LiteralPath ([System.IO.Path]::ChangeExtension($runtimeBottleneckPath, ".json")) | ConvertFrom-Json
Assert-True ($runtimeBottleneckText -match "speedup_decision: speedup_blocked_instrumentation") "runtime bottleneck report did not block speedup when wait attribution is missing"
Assert-True ($runtimeBottleneckText -notmatch "wait_attribution_missing_fields: .*model_queue_wait_ms") "runtime bottleneck report treated unavailable model queue attribution as missing"
Assert-True ($runtimeBottleneckText -match "wait_attribution_unavailable_fields: .*model_queue_wait_ms=whale_jsonl_provider_queue_retry_telemetry_unavailable") "runtime bottleneck report did not render unavailable wait attribution fields"
Assert-True ($runtimeBottleneckText -notmatch "wait_attribution_missing_fields: .*resource_wait_ms_total") "runtime bottleneck report treated serial resource wait as missing"
Assert-True ([string]$runtimeBottleneckJson.speedup_decision -eq "speedup_blocked_instrumentation") "runtime bottleneck JSON did not record speedup decision"
Assert-True (-not [bool]$runtimeBottleneckJson.speedup_evidence_valid -and $runtimeBottleneckText -match "speedup_evidence_valid: False") "runtime bottleneck report did not block speedup evidence validity"
Assert-True ([string]$runtimeBottleneckJson.resource_wait_attribution_mode -eq "serial_with_cache_lock_observed") "runtime bottleneck JSON did not record observed cache lock attribution mode"
Assert-True ([string]$runtimeBottleneckJson.wait_attribution_unavailable_fields.model_retry_backoff_ms -eq "whale_jsonl_provider_queue_retry_telemetry_unavailable") "runtime bottleneck JSON did not record unavailable retry attribution reason"
$calibrationParallelismPath = Write-TaskspaceParallelismArtifact $suiteRoot (New-TaskspaceResourceGovernorConfig) $null (Test-TaskspaceDiskReservation @($suiteRoot) 0) (New-TaskspaceResourceWaitSnapshot)
$calibrationPath = Write-TaskspaceRuntimeCalibrationReport -TimingPath (Join-Path $suiteRoot "suite-timing.json") -ScoreValid $true -CommandLine "synthetic calibration" -GitCommit "test-commit" -ProfileHash "test-profile" -ParallelismPath $calibrationParallelismPath
$calibrationText = Get-Content -Raw -Encoding UTF8 -LiteralPath $calibrationPath
$calibrationJson = Get-Content -Raw -Encoding UTF8 -LiteralPath ([System.IO.Path]::ChangeExtension($calibrationPath, ".json")) | ConvertFrom-Json
Assert-True ($calibrationText -match "# TaskSpace Runtime Calibration Report" -and $calibrationText -match "speedup_decision: speedup_blocked_instrumentation") "runtime calibration report did not render speedup decision"
Assert-True (-not [bool]$calibrationJson.speedup_evidence_valid -and $calibrationText -match "speedup_evidence_valid: False") "runtime calibration report did not render blocked speedup evidence validity"
Assert-True ($calibrationText -match "resource_governor_status: pass" -and $calibrationText -match "profile_hash: test-profile") "runtime calibration report did not render parallelism/profile metadata"
Assert-True ([string]$calibrationJson.speedup_decision -eq "speedup_blocked_instrumentation" -and [string]$calibrationJson.parallelism.resource_governor_status -eq "pass") "runtime calibration JSON did not preserve decision and parallelism status"

$timingAggregatePath = Join-Path $suiteRoot "aggregate-report.md"
Write-TaskspaceAggregateReport -Path $timingAggregatePath -Reports @([pscustomobject]@{ repeat = 1; pair_dir = $pairTimingDir; pair_report = "pair-report.md"; evidence_target = "E3"; evidence = $evidence })
$timingAggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRoot "aggregate.json") | ConvertFrom-Json
$timingAggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $timingAggregatePath
Assert-True ([string]$timingAggregate.timing_summary.bottleneck_classification -eq "mixed_or_unclassified") "aggregate JSON did not include suite timing bottleneck"
Assert-True ([string]$timingAggregate.timing_summary.speedup_decision -eq "speedup_blocked_instrumentation") "aggregate JSON did not include runtime speedup decision"
Assert-True (-not [bool]$timingAggregate.timing_summary.speedup_evidence_valid) "aggregate JSON did not include blocked speedup evidence validity"
Assert-True ([string]$timingAggregate.timing_summary.wait_attribution_unavailable_fields.model_queue_wait_ms -eq "whale_jsonl_provider_queue_retry_telemetry_unavailable") "aggregate JSON did not include unavailable wait attribution reason"
Assert-True ($timingAggregateText -match "## Timing Summary" -and $timingAggregateText -match "top_span:" -and $timingAggregateText -match "total_median_ms") "aggregate report did not render timing summary"
Assert-True ($timingAggregateText -match "speedup_decision: speedup_blocked_instrumentation") "aggregate report did not render speedup decision"
Assert-True ($timingAggregateText -match "speedup_evidence_valid: False") "aggregate report did not render speedup evidence validity"
Assert-True ($timingAggregateText -match "wait_attribution_unavailable_fields: .*model_queue_wait_ms=whale_jsonl_provider_queue_retry_telemetry_unavailable") "aggregate report did not render unavailable wait attribution reason"
Assert-True ($timingAggregateText -match "repeated_docker_cache_keys: cache-a") "aggregate report did not render repeated Docker cache keys"

$calibrationGateRoot = Join-Path $runDir "calibration-gate"
$onePairRoot = Join-Path $calibrationGateRoot "one-pair"
$serialCalibrationRoot = Join-Path $calibrationGateRoot "serial-calibration"
New-Item -ItemType Directory -Force -Path $onePairRoot, $serialCalibrationRoot | Out-Null
$onePairTimingPath = Join-Path $onePairRoot "pair-timing.json"
$onePairReportPath = Join-Path $onePairRoot "runtime-bottleneck.md"
$onePairReportJsonPath = Join-Path $onePairRoot "runtime-bottleneck.json"
[pscustomobject]@{
    agent_duration_ms = 1000
    public_validation_duration_ms = 2000
    bottleneck_classification = "validator_bound"
    runtime_optimization_status = "ready"
    task_list_hash = "task-list-a"
    source_version = "source-a"
    profile_hash = "profile-a"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $onePairTimingPath -Encoding UTF8
[pscustomobject]@{ sample_id = "calibration-one-pair" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Encoding UTF8
"# Runtime Bottleneck`n" | Set-Content -LiteralPath $onePairReportPath -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    timing_path = $onePairTimingPath
    report_path = $onePairReportPath
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_validator_or_docker"
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $onePairReportJsonPath -Encoding UTF8
$serialSuiteTimingPath = Join-Path $serialCalibrationRoot "suite-timing.json"
$serialReportPath = Join-Path $serialCalibrationRoot "runtime-calibration-report.md"
$serialReportJsonPath = Join-Path $serialCalibrationRoot "runtime-calibration-report.json"
[pscustomobject]@{
    sample_count = 3
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    bottleneck_classification = "mixed_or_unclassified"
    wait_attribution_status = "complete"
    task_list_hash = "task-list-a"
    source_version = "source-a"
    profile_hash = "profile-a"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $serialSuiteTimingPath -Encoding UTF8
"# Runtime Calibration`n" | Set-Content -LiteralPath $serialReportPath -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    report_path = $serialReportPath
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_parallelism"
    timing_path = $serialSuiteTimingPath
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $serialReportJsonPath -Encoding UTF8
$calibrationGatePath = Join-Path $calibrationGateRoot "calibration-gate.json"
$calibrationGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath -ExpectedTaskListHash "task-list-a" -ExpectedSourceVersion "source-a" -ExpectedProfileHash "profile-a" -OutputPath $calibrationGatePath
Assert-True ([string]$calibrationGate.status -eq "pass" -and [bool]$calibrationGate.full_e3_allowed -and [bool]$calibrationGate.speed_claim_allowed) "calibration gate did not pass complete timing/equivalence evidence"
Assert-True (Test-Path -LiteralPath $calibrationGatePath) "calibration gate did not write its artifact"
Assert-True ([bool]$calibrationGate.gates[1].details.speedup_evidence_valid -and [string]$calibrationGate.gates[1].details.wait_attribution_status -eq "complete") "calibration gate did not expose serial speed evidence details"
$identityMismatchGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath -ExpectedTaskListHash "task-list-b" -ExpectedSourceVersion "source-a" -ExpectedProfileHash "profile-a"
Assert-True ([string]$identityMismatchGate.status -eq "fail" -and [string]$identityMismatchGate.first_failure.reason -eq "one_pair_smoke_identity_mismatch:task_list_hash") "calibration gate did not block identity-mismatched one-pair timing evidence"
$missingOnePairGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot (Join-Path $calibrationGateRoot "missing-one-pair") -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$missingOnePairGate.status -eq "fail" -and -not [bool]$missingOnePairGate.full_e3_allowed -and [string]$missingOnePairGate.first_failure.reason -eq "one_pair_root_missing") "calibration gate did not block missing one-pair smoke evidence"
$blockedOnePairRoot = Join-Path $calibrationGateRoot "one-pair-blocked-decision"
New-Item -ItemType Directory -Force -Path $blockedOnePairRoot | Out-Null
$blockedOnePairTimingPath = Join-Path $blockedOnePairRoot "pair-timing.json"
$blockedOnePairReportPath = Join-Path $blockedOnePairRoot "runtime-bottleneck.md"
Copy-Item -LiteralPath $onePairTimingPath -Destination $blockedOnePairTimingPath -Force
Copy-Item -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Destination (Join-Path $blockedOnePairRoot "sample-timing.json") -Force
"# Runtime Bottleneck`n" | Set-Content -LiteralPath $blockedOnePairReportPath -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    timing_path = $blockedOnePairTimingPath
    report_path = $blockedOnePairReportPath
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_blocked_instrumentation"
    speedup_decision_reason = "runtime_optimization_status_blocked"
    runtime_optimization_blockers = @("missing_wait_attribution:model_request_duration_ms")
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $blockedOnePairRoot "runtime-bottleneck.json") -Encoding UTF8
$blockedOnePairGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $blockedOnePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$blockedOnePairGate.status -eq "pass" -and [bool]$blockedOnePairGate.full_e3_allowed -and -not [bool]$blockedOnePairGate.speed_claim_allowed) "calibration gate did not decouple one-pair instrumentation speed block from full E3 eligibility"
Assert-True ([string]$blockedOnePairGate.gates[0].details.speedup_decision_reason -eq "runtime_optimization_status_blocked" -and @($blockedOnePairGate.gates[0].details.runtime_optimization_blockers).Count -eq 1) "calibration gate did not expose one-pair speed blocker details"
$dirtyOnePairRoot = Join-Path $calibrationGateRoot "one-pair-dirty-timing"
New-Item -ItemType Directory -Force -Path $dirtyOnePairRoot | Out-Null
$dirtyOnePairTimingPath = Join-Path $dirtyOnePairRoot "pair-timing.json"
$dirtyOnePairReportPath = Join-Path $dirtyOnePairRoot "runtime-bottleneck.md"
[pscustomobject]@{
    agent_duration_ms = 1000
    public_validation_duration_ms = 2000
    bottleneck_classification = "engineering_unclean_slow"
    runtime_optimization_status = "blocked"
    engineering_unclean = $true
    engineering_unclean_reasons = @("public_validation_timeout")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $dirtyOnePairTimingPath -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Destination (Join-Path $dirtyOnePairRoot "sample-timing.json") -Force
"# Runtime Bottleneck`n" | Set-Content -LiteralPath $dirtyOnePairReportPath -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    timing_path = $dirtyOnePairTimingPath
    report_path = $dirtyOnePairReportPath
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_validator_or_docker"
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $dirtyOnePairRoot "runtime-bottleneck.json") -Encoding UTF8
$dirtyOnePairGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $dirtyOnePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$dirtyOnePairGate.status -eq "fail" -and [string]$dirtyOnePairGate.first_failure.reason -eq "one_pair_smoke_engineering_unclean") "calibration gate trusted report score_valid over dirty one-pair timing"
foreach ($dirtyCase in @(
        [pscustomobject]@{ name = "flag-only"; fields = [pscustomobject]@{ engineering_unclean = $true }; expected = "one_pair_smoke_engineering_unclean" },
        [pscustomobject]@{ name = "reasons-only"; fields = [pscustomobject]@{ engineering_unclean_reasons = @("docker_run_failure") }; expected = "one_pair_smoke_engineering_unclean_reasons_present" },
        [pscustomobject]@{ name = "bottleneck-only"; fields = [pscustomobject]@{ bottleneck_classification = "engineering_unclean_slow" }; expected = "one_pair_smoke_engineering_unclean_slow" }
    )) {
    $caseRoot = Join-Path $calibrationGateRoot "one-pair-dirty-$($dirtyCase.name)"
    New-Item -ItemType Directory -Force -Path $caseRoot | Out-Null
    $caseTimingPath = Join-Path $caseRoot "pair-timing.json"
    $caseReportPath = Join-Path $caseRoot "runtime-bottleneck.md"
    $caseTiming = [pscustomobject]@{
        agent_duration_ms = 1000
        public_validation_duration_ms = 2000
        bottleneck_classification = "validator_bound"
        runtime_optimization_status = "ready"
    }
    foreach ($property in $dirtyCase.fields.PSObject.Properties) {
        $caseTiming | Add-Member -NotePropertyName $property.Name -NotePropertyValue $property.Value -Force
    }
    $caseTiming | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $caseTimingPath -Encoding UTF8
    Copy-Item -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Destination (Join-Path $caseRoot "sample-timing.json") -Force
    "# Runtime Bottleneck`n" | Set-Content -LiteralPath $caseReportPath -Encoding UTF8
    [pscustomobject]@{
        schema_version = 1
        timing_path = $caseTimingPath
        report_path = $caseReportPath
        score_valid = $true
        speedup_evidence_valid = $true
        speedup_decision = "speedup_candidate_validator_or_docker"
        timing_quality = "complete"
        runtime_optimization_status = "ready"
        wait_attribution_status = "complete"
        generated_at = "2026-06-15T00:00:00.0000000Z"
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $caseRoot "runtime-bottleneck.json") -Encoding UTF8
    $caseGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $caseRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
    Assert-True ([string]$caseGate.status -eq "fail" -and [string]$caseGate.first_failure.reason -eq [string]$dirtyCase.expected) "calibration gate missed isolated one-pair dirty indicator $($dirtyCase.name)"
}
$invalidOnePairRoot = Join-Path $calibrationGateRoot "one-pair-invalid-run"
New-Item -ItemType Directory -Force -Path $invalidOnePairRoot | Out-Null
$invalidOnePairTimingPath = Join-Path $invalidOnePairRoot "pair-timing.json"
$invalidOnePairReportPath = Join-Path $invalidOnePairRoot "runtime-bottleneck.md"
Copy-Item -LiteralPath $onePairTimingPath -Destination $invalidOnePairTimingPath -Force
Copy-Item -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Destination (Join-Path $invalidOnePairRoot "sample-timing.json") -Force
"# Runtime Bottleneck`n" | Set-Content -LiteralPath $invalidOnePairReportPath -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    timing_path = $invalidOnePairTimingPath
    report_path = $invalidOnePairReportPath
    score_valid = $false
    speedup_evidence_valid = $false
    speedup_decision = "speedup_blocked_invalid_run"
    timing_quality = "complete"
    runtime_optimization_status = "blocked"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $invalidOnePairRoot "runtime-bottleneck.json") -Encoding UTF8
$invalidOnePairGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $invalidOnePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$invalidOnePairGate.status -eq "fail" -and [string]$invalidOnePairGate.first_failure.reason -eq "one_pair_smoke_field_invalid:score_valid") "calibration gate allowed invalid one-pair score evidence"
$placeholderOnePairRoot = Join-Path $calibrationGateRoot "one-pair-placeholder-json"
New-Item -ItemType Directory -Force -Path $placeholderOnePairRoot | Out-Null
Copy-Item -LiteralPath $onePairTimingPath -Destination (Join-Path $placeholderOnePairRoot "pair-timing.json") -Force
Copy-Item -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Destination (Join-Path $placeholderOnePairRoot "sample-timing.json") -Force
"# Runtime Bottleneck`n" | Set-Content -LiteralPath (Join-Path $placeholderOnePairRoot "runtime-bottleneck.md") -Encoding UTF8
[pscustomobject]@{
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_validator_or_docker"
    timing_quality = "complete"
    runtime_optimization_status = "ready"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $placeholderOnePairRoot "runtime-bottleneck.json") -Encoding UTF8
$placeholderOnePairGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $placeholderOnePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$placeholderOnePairGate.status -eq "fail" -and [string]$placeholderOnePairGate.first_failure.reason -eq "one_pair_smoke_field_missing:schema_version") "calibration gate allowed placeholder one-pair JSON without generated metadata"
$placeholderSerialRoot = Join-Path $calibrationGateRoot "serial-placeholder"
New-Item -ItemType Directory -Force -Path $placeholderSerialRoot | Out-Null
Copy-Item -LiteralPath $serialSuiteTimingPath -Destination (Join-Path $placeholderSerialRoot "suite-timing.json") -Force
"# Placeholder Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $placeholderSerialRoot "runtime-calibration-report.md") -Encoding UTF8
$placeholderSerialGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $placeholderSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$placeholderSerialGate.status -eq "fail" -and [string]$placeholderSerialGate.first_failure.reason -eq "serial_calibration_artifact_missing") "calibration gate allowed placeholder calibration report without JSON evidence"
$blockedSerialRoot = Join-Path $calibrationGateRoot "serial-blocked"
New-Item -ItemType Directory -Force -Path $blockedSerialRoot | Out-Null
Copy-Item -LiteralPath $serialSuiteTimingPath -Destination (Join-Path $blockedSerialRoot "suite-timing.json") -Force
Copy-Item -LiteralPath (Join-Path $serialCalibrationRoot "runtime-calibration-report.md") -Destination (Join-Path $blockedSerialRoot "runtime-calibration-report.md") -Force
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $blockedSerialRoot "runtime-calibration-report.md")
    score_valid = $true
    speedup_evidence_valid = $false
    speedup_decision = "speedup_blocked_instrumentation"
    timing_path = (Join-Path $blockedSerialRoot "suite-timing.json")
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $blockedSerialRoot "runtime-calibration-report.json") -Encoding UTF8
$blockedSerialGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $blockedSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$blockedSerialGate.status -eq "pass" -and [bool]$blockedSerialGate.full_e3_allowed -and -not [bool]$blockedSerialGate.speed_claim_allowed) "calibration gate did not decouple serial speed evidence from full E3 eligibility"
$dirtySerialRoot = Join-Path $calibrationGateRoot "serial-dirty-timing"
New-Item -ItemType Directory -Force -Path $dirtySerialRoot | Out-Null
$dirtySerialTimingPath = Join-Path $dirtySerialRoot "suite-timing.json"
[pscustomobject]@{
    sample_count = 3
    timing_quality = "complete"
    runtime_optimization_status = "blocked"
    bottleneck_classification = "engineering_unclean_slow"
    wait_attribution_status = "complete"
    engineering_unclean_reasons = @("child_engineering_unclean_slow")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $dirtySerialTimingPath -Encoding UTF8
"# Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $dirtySerialRoot "runtime-calibration-report.md") -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $dirtySerialRoot "runtime-calibration-report.md")
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_parallelism"
    timing_path = $dirtySerialTimingPath
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $dirtySerialRoot "runtime-calibration-report.json") -Encoding UTF8
$dirtySerialGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $dirtySerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$dirtySerialGate.status -eq "fail" -and [string]$dirtySerialGate.first_failure.reason -eq "serial_calibration_engineering_unclean_reasons_present") "calibration gate trusted report score_valid over dirty serial timing"
$blockedDecisionSerialRoot = Join-Path $calibrationGateRoot "serial-blocked-decision"
New-Item -ItemType Directory -Force -Path $blockedDecisionSerialRoot | Out-Null
Copy-Item -LiteralPath $serialSuiteTimingPath -Destination (Join-Path $blockedDecisionSerialRoot "suite-timing.json") -Force
"# Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $blockedDecisionSerialRoot "runtime-calibration-report.md") -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $blockedDecisionSerialRoot "runtime-calibration-report.md")
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_blocked_instrumentation"
    speedup_decision_reason = "runtime_optimization_status_blocked"
    runtime_optimization_blockers = @("unavailable_wait_attribution:model_queue_wait_ms")
    timing_path = (Join-Path $blockedDecisionSerialRoot "suite-timing.json")
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $blockedDecisionSerialRoot "runtime-calibration-report.json") -Encoding UTF8
$blockedDecisionSerialGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $blockedDecisionSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$blockedDecisionSerialGate.status -eq "pass" -and [bool]$blockedDecisionSerialGate.full_e3_allowed -and -not [bool]$blockedDecisionSerialGate.speed_claim_allowed) "calibration gate did not decouple serial instrumentation decision from full E3 eligibility"
Assert-True ([string]$blockedDecisionSerialGate.gates[1].details.speedup_decision_reason -eq "runtime_optimization_status_blocked" -and @($blockedDecisionSerialGate.gates[1].details.runtime_optimization_blockers).Count -eq 1) "calibration gate did not expose serial speed blocker details"
$invalidSerialRoot = Join-Path $calibrationGateRoot "serial-invalid-run"
New-Item -ItemType Directory -Force -Path $invalidSerialRoot | Out-Null
Copy-Item -LiteralPath $serialSuiteTimingPath -Destination (Join-Path $invalidSerialRoot "suite-timing.json") -Force
"# Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $invalidSerialRoot "runtime-calibration-report.md") -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $invalidSerialRoot "runtime-calibration-report.md")
    score_valid = $false
    speedup_evidence_valid = $false
    speedup_decision = "speedup_blocked_invalid_run"
    timing_path = (Join-Path $invalidSerialRoot "suite-timing.json")
    timing_quality = "complete"
    runtime_optimization_status = "blocked"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $invalidSerialRoot "runtime-calibration-report.json") -Encoding UTF8
$invalidSerialGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $invalidSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$invalidSerialGate.status -eq "fail" -and [string]$invalidSerialGate.first_failure.reason -eq "serial_calibration_field_invalid:score_valid") "calibration gate allowed invalid serial score evidence"
$missingWaitSerialRoot = Join-Path $calibrationGateRoot "serial-missing-wait"
New-Item -ItemType Directory -Force -Path $missingWaitSerialRoot | Out-Null
$missingWaitSuiteTimingPath = Join-Path $missingWaitSerialRoot "suite-timing.json"
[pscustomobject]@{
    sample_count = 3
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    bottleneck_classification = "mixed_or_unclassified"
    wait_attribution_status = "missing"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $missingWaitSuiteTimingPath -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $serialCalibrationRoot "runtime-calibration-report.md") -Destination (Join-Path $missingWaitSerialRoot "runtime-calibration-report.md") -Force
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $missingWaitSerialRoot "runtime-calibration-report.md")
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_parallelism"
    timing_path = $missingWaitSuiteTimingPath
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "missing"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $missingWaitSerialRoot "runtime-calibration-report.json") -Encoding UTF8
$missingWaitGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $missingWaitSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$missingWaitGate.status -eq "pass" -and [bool]$missingWaitGate.full_e3_allowed -and -not [bool]$missingWaitGate.speed_claim_allowed) "calibration gate did not decouple missing wait attribution from full E3 eligibility"
$missingTimingPathSerialRoot = Join-Path $calibrationGateRoot "serial-missing-timing-path"
New-Item -ItemType Directory -Force -Path $missingTimingPathSerialRoot | Out-Null
Copy-Item -LiteralPath $serialSuiteTimingPath -Destination (Join-Path $missingTimingPathSerialRoot "suite-timing.json") -Force
"# Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $missingTimingPathSerialRoot "runtime-calibration-report.md") -Encoding UTF8
[pscustomobject]@{
    schema_version = 1
    report_path = (Join-Path $missingTimingPathSerialRoot "runtime-calibration-report.md")
    score_valid = $true
    speedup_evidence_valid = $true
    speedup_decision = "speedup_candidate_parallelism"
    timing_quality = "complete"
    runtime_optimization_status = "ready"
    wait_attribution_status = "complete"
    generated_at = "2026-06-15T00:00:00.0000000Z"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $missingTimingPathSerialRoot "runtime-calibration-report.json") -Encoding UTF8
$missingTimingPathGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $missingTimingPathSerialRoot -ParallelEquivalencePath $equivalencePath
Assert-True ([string]$missingTimingPathGate.status -eq "fail" -and [string]$missingTimingPathGate.first_failure.reason -eq "serial_calibration_field_missing:timing_path") "calibration gate allowed calibration JSON without timing_path binding"
$missingRequiredFieldsEquivalencePath = Join-Path $calibrationGateRoot "missing-required-fields-equivalence.json"
[pscustomobject]@{
    parallel_smoke_score_drift = $false
    comparable = $true
    drift_count = 0
    compared_sample_ids = @("sample-a")
    task_list_hash = "task-list-a"
    source_version = "source-a"
    profile_hash = "profile-a"
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $missingRequiredFieldsEquivalencePath -Encoding UTF8
$missingRequiredFieldsGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $missingRequiredFieldsEquivalencePath
Assert-True ([string]$missingRequiredFieldsGate.status -eq "fail" -and [string]$missingRequiredFieldsGate.first_failure.reason -eq "parallel_required_sample_fields_missing") "calibration gate did not block parallel equivalence missing required sample fields"
$driftEquivalencePath = Join-Path $calibrationGateRoot "drift-equivalence.json"
[pscustomobject]@{
    parallel_smoke_score_drift = $true
    compared_sample_ids = @("sample-a")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $driftEquivalencePath -Encoding UTF8
$driftCalibrationGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $driftEquivalencePath
Assert-True ([string]$driftCalibrationGate.status -eq "fail" -and [string]$driftCalibrationGate.first_failure.reason -eq "parallel_score_drift") "calibration gate did not block parallel score drift"
$notComparableEquivalencePath = Join-Path $calibrationGateRoot "not-comparable-equivalence.json"
[pscustomobject]@{
    comparable = $false
    parallel_smoke_score_drift = $false
    drift_count = 0
    compared_sample_ids = @("sample-a")
    required_sample_fields = @("run_validity", "phase")
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $notComparableEquivalencePath -Encoding UTF8
$notComparableCalibrationGate = Invoke-TaskspaceCalibrationGate -OnePairSmokeRoot $onePairRoot -SerialCalibrationRoot $serialCalibrationRoot -ParallelEquivalencePath $notComparableEquivalencePath
Assert-True ([string]$notComparableCalibrationGate.status -eq "fail" -and [string]$notComparableCalibrationGate.first_failure.reason -eq "parallel_not_comparable") "calibration gate did not block non-comparable parallel equivalence"

if ($failures.Count -gt 0) {
    Write-Host "E3 harness guardrails self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 harness guardrails self-test: PASS"
Write-Host "RunRoot: $runDir"
