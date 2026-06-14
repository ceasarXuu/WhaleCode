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
    Assert-True ([string]$suiteHealth.status -eq "invalid_harness" -and -not [bool]$suiteHealth.suite_score_valid) "suite health did not record invalid suite score"
    Assert-True ([int]$suiteHealth.remaining_samples_skipped -eq 1 -and [int]$suiteHealth.score_invalid_child_runs -eq 2) "suite health did not count skipped invalid sample"
    Assert-True ($null -eq $suiteHealth.expected_time_saved_minutes -and [string]$suiteHealth.expected_time_saved_basis -eq "no_serial_baseline") "suite health did not explain missing time-saved baseline"
    Assert-True ([string]$skippedStatus.phase -eq "skipped" -and [string]$skippedStatus.abort_phase -eq "suite_circuit_breaker") "skipped sample status did not record suite circuit breaker"
} finally {
    if ($null -eq $oldSuiteMinFreeBytes) { Remove-Item Env:TASKSPACE_MIN_FREE_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_BYTES = $oldSuiteMinFreeBytes }
    if ($null -eq $oldSuiteMinFreeGib) { Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_GIB = $oldSuiteMinFreeGib }
}

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
        validator_environment_failures = @(); docker_build_duration_ms = 1600; docker_run_duration_ms = 200; docker_cleanup_duration_ms = 100; docker_cache_key = "cache-a"
    }
    right = [pscustomobject]@{
        logical_mode = "taskspace"; wall_time_ms = 2000; exec_exit_code = 0; exec_timed_out = $false
        validator_environment_failures = @(); docker_build_duration_ms = 100; docker_run_duration_ms = 200; docker_cleanup_duration_ms = 100; docker_cache_key = "cache-a"
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

$suiteRoot = Join-Path $runDir "timing-suite"
$suiteSampleRoot = Join-Path $suiteRoot "samples\timing-sample"
New-Item -ItemType Directory -Force -Path $suiteSampleRoot | Out-Null
Copy-Item -LiteralPath (Join-Path $timingRun "sample-timing.json") -Destination (Join-Path $suiteSampleRoot "sample-timing.json") -Force
Write-TaskspaceSuiteTiming -SuiteRoot $suiteRoot -SampleStatuses @([pscustomobject]@{ sample_root = $suiteSampleRoot }) | Out-Null
$suiteTiming = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRoot "suite-timing.json") | ConvertFrom-Json
Assert-True ([int64]$suiteTiming.docker_build_duration_ms -eq 2600) "suite timing did not aggregate docker build duration"
Assert-True ([int]$suiteTiming.bottleneck_counts.docker_build_bound -eq 1) "suite timing did not aggregate bottleneck count"
Assert-True ([int]$suiteTiming.docker_cache_key_counts."cache-a" -eq 2 -and [string]$suiteTiming.repeated_docker_cache_keys[0] -eq "cache-a") "suite timing did not aggregate repeated Docker cache keys"

$timingAggregatePath = Join-Path $suiteRoot "aggregate-report.md"
Write-TaskspaceAggregateReport -Path $timingAggregatePath -Reports @([pscustomobject]@{ repeat = 1; pair_dir = $pairTimingDir; pair_report = "pair-report.md"; evidence_target = "E3"; evidence = $evidence })
$timingAggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteRoot "aggregate.json") | ConvertFrom-Json
$timingAggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $timingAggregatePath
Assert-True ([string]$timingAggregate.timing_summary.bottleneck_classification -eq "mixed_or_unclassified") "aggregate JSON did not include suite timing bottleneck"
Assert-True ($timingAggregateText -match "## Timing Summary" -and $timingAggregateText -match "top_span:" -and $timingAggregateText -match "total_median_ms") "aggregate report did not render timing summary"
Assert-True ($timingAggregateText -match "repeated_docker_cache_keys: cache-a") "aggregate report did not render repeated Docker cache keys"

if ($failures.Count -gt 0) {
    Write-Host "E3 harness guardrails self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 harness guardrails self-test: PASS"
Write-Host "RunRoot: $runDir"
