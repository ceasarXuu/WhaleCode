param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\harness-health.ps1")
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-start-gate-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

function New-GateScenario {
    param([string]$Root, [bool]$RelativeUv = $false)
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "fixture") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "validator") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "uv-cache") | Out-Null
    "prompt" | Set-Content -LiteralPath (Join-Path $Root "prompt.txt") -Encoding UTF8
    $uv = if ($RelativeUv) { "relative\uv-cache" } else { Join-Path $Root "uv-cache" }
    [pscustomobject]@{
        id = "gate-fixture"
        level = "E3"
        evidence_target = "E3"
        prompt_file = "prompt.txt"
        fixture_dir = "fixture"
        narrative_contract = "fixture contract"
        mode_delta_contract = "fixture delta"
        oracle = [pscustomobject]@{ hidden_strategy = "fixture"; public_validation = [pscustomobject]@{} }
        expected = [pscustomobject]@{ max_taskspace_nodes = 10 }
        thresholds = [pscustomobject]@{ taskspace_tool_call_ratio_warn = 10 }
        external_benchmark = [pscustomobject]@{
            adapter_metadata = [pscustomobject]@{
                uv_cache_root = $uv
                validator_source_dir = Join-Path $Root "validator"
                fixture_source = Join-Path $Root "fixture"
            }
        }
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $Root "scenario.json") -Encoding UTF8
}

function New-CalibrationFixtures {
    param([string]$Root)
    $onePairRoot = Join-Path $Root "one-pair"
    $serialRoot = Join-Path $Root "serial"
    New-Item -ItemType Directory -Force -Path $onePairRoot, $serialRoot | Out-Null
    [pscustomobject]@{ score_valid = $true; run_validity = "valid"; clean_comparable_pair_count = 1 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "aggregate.json") -Encoding UTF8
    [pscustomobject]@{
        agent_duration_ms = 1000
        public_validation_duration_ms = 2000
        bottleneck_classification = "validator_bound"
        runtime_optimization_status = "ready"
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "pair-timing.json") -Encoding UTF8
    [pscustomobject]@{ sample_id = "calibration-one-pair" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $onePairRoot "sample-timing.json") -Encoding UTF8
    "# Runtime Bottleneck`n" | Set-Content -LiteralPath (Join-Path $onePairRoot "runtime-bottleneck.md") -Encoding UTF8
    [pscustomobject]@{
        sample_count = 3
        timing_quality = "complete"
        runtime_optimization_status = "ready"
        bottleneck_classification = "mixed_or_unclassified"
        wait_attribution_status = "complete"
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $serialRoot "suite-timing.json") -Encoding UTF8
    "# Runtime Calibration`n" | Set-Content -LiteralPath (Join-Path $serialRoot "runtime-calibration-report.md") -Encoding UTF8
    $equivalencePath = Join-Path $Root "serial-vs-parallel-equivalence.json"
    [pscustomobject]@{
        comparable = $true
        parallel_smoke_score_drift = $false
        drift_count = 0
        compared_sample_ids = @("sample-a", "sample-b", "sample-c")
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $equivalencePath -Encoding UTF8
    [pscustomobject]@{ one_pair_root = $onePairRoot; serial_root = $serialRoot; equivalence_path = $equivalencePath }
}

$oldMinFreeBytes = $env:TASKSPACE_MIN_FREE_BYTES
$oldMinFreeGib = $env:TASKSPACE_MIN_FREE_GIB
try {
    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue

    $scenarioDir = Join-Path $runDir "scenario-pass"
    New-GateScenario $scenarioDir
    $gate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-pass") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$gate.status -eq "pass" -and [int]$gate.exit_code -eq 0) "start gate did not pass clean fixture"
    Assert-True (Test-Path -LiteralPath $gate.json_path) "start gate did not write json artifact"
    Assert-True (Test-Path -LiteralPath $gate.markdown_path) "start gate did not write markdown artifact"

    $noSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-smoke") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$noSmokeGate.status -eq "fail" -and @($noSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "fail" -and [string]$_.stable_code -eq "one_pair_smoke_not_provided" }).Count -eq 1) "start gate allowed missing one-pair smoke without explicit allow"

    $smokeRoot = Join-Path $runDir "one-pair-smoke"
    New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null
    [pscustomobject]@{ score_valid = $true; run_validity = "valid"; clean_comparable_pair_count = 1 } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $smokeRoot "aggregate.json") -Encoding UTF8
    $smokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $smokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$smokeGate.status -eq "pass" -and @($smokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate did not accept valid one-pair smoke artifact when calibration gate is explicitly skipped"
    $aggregateOnlyCalibrationGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-aggregate-only-calibration-fail") -ScenarioPath $scenarioDir -OnePairSmokeRoot $smokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$aggregateOnlyCalibrationGate.status -eq "fail" -and @($aggregateOnlyCalibrationGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "start gate allowed aggregate-only one-pair root without timing calibration artifacts"

    $calibration = New-CalibrationFixtures (Join-Path $runDir "calibration-fixtures")
    $calibratedGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-calibration-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $calibration.one_pair_root -SerialCalibrationRoot $calibration.serial_root -ParallelEquivalencePath $calibration.equivalence_path -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -SelfTestCommands @()
    Assert-True ([string]$calibratedGate.status -eq "pass" -and @($calibratedGate.gates | Where-Object { [string]$_.name -eq "calibration_parallel_smoke" -and [string]$_.status -eq "pass" }).Count -eq 1) "start gate did not pass complete calibration evidence"

    $classifiedSmokeRoot = Join-Path $runDir "one-pair-classified-smoke"
    New-Item -ItemType Directory -Force -Path $classifiedSmokeRoot | Out-Null
    [pscustomobject]@{ run_validity = "invalid_harness"; abort_signature = "harness_materialization_failure/docker_run_failure" } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $classifiedSmokeRoot "sample-status.json") -Encoding UTF8
    $classifiedSmokeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-classified-smoke-pass") -ScenarioPath $scenarioDir -OnePairSmokeRoot $classifiedSmokeRoot -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$classifiedSmokeGate.status -eq "pass" -and @($classifiedSmokeGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.reason -eq "classified_invalid_harness" }).Count -eq 1) "start gate did not accept classified invalid one-pair sample-status artifact"

    $noSelfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-no-selftests") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$noSelfTestGate.status -eq "fail" -and [string]$noSelfTestGate.first_failure_stable_code -eq "self_tests_not_run") "start gate allowed skipped self-tests without explicit allow"

    $relativeScenario = Join-Path $runDir "scenario-relative"
    New-GateScenario $relativeScenario $true
    $relativeGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-relative") -ScenarioPath $relativeScenario -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$relativeGate.status -eq "fail" -and [int]$relativeGate.exit_code -eq 3) "start gate did not fail relative path contract"
    Assert-True (@($relativeGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "fail" }).Count -eq 1) "start gate did not identify path_contract failure"

    $env:TASKSPACE_MIN_FREE_BYTES = ([int64]::MaxValue).ToString()
    $diskGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-disk") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$diskGate.status -eq "fail" -and [string]$diskGate.run_validity -eq "invalid_harness") "start gate did not fail impossible disk threshold"

    $env:TASKSPACE_MIN_FREE_BYTES = "1"
    $selfTestGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-selftest") -ScenarioPath $scenarioDir -RunRoot (Join-Path $runDir "runs") -RunSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @("exit 7")
    Assert-True ([string]$selfTestGate.status -eq "fail" -and [int]$selfTestGate.exit_code -eq 3) "start gate did not fail failing self-test command"
    Assert-True ([string]$selfTestGate.first_failure_gate -eq "cheap_self_tests" -and [string]$selfTestGate.first_failure_command -eq "exit 7") "start gate did not record first failing self-test command"

    $missingScenarioGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-missing-scenario") -ScenarioPath (Join-Path $runDir "missing-scenario") -RunRoot (Join-Path $runDir "runs") -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$missingScenarioGate.status -eq "fail" -and [int]$missingScenarioGate.exit_code -eq 3 -and (Test-Path -LiteralPath $missingScenarioGate.json_path)) "start gate did not write artifacts for missing scenario"

    $taskListMissingGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-missing") -TaskListPath (Join-Path $runDir "missing-tasks.jsonl") -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListMissingGate.status -eq "fail" -and [string]$taskListMissingGate.first_failure_stable_code -eq "task_list_missing") "start gate did not fail missing task list"

    $emptyTaskList = Join-Path $runDir "empty-tasks.jsonl"
    "" | Set-Content -LiteralPath $emptyTaskList -Encoding UTF8
    $taskListEmptyGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-empty") -TaskListPath $emptyTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListEmptyGate.status -eq "fail" -and [string]$taskListEmptyGate.first_failure_stable_code -eq "task_list_empty") "start gate did not fail empty task list"

    $badTaskList = Join-Path $runDir "bad-tasks.jsonl"
    "{not-json" | Set-Content -LiteralPath $badTaskList -Encoding UTF8
    $taskListBadGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-bad") -TaskListPath $badTaskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListBadGate.status -eq "fail" -and [string]$taskListBadGate.first_failure_stable_code -eq "task_list_malformed") "start gate did not fail malformed task list"

    $taskList = Join-Path $runDir "tasks.jsonl"
    ([pscustomobject]@{ task_dir = $scenarioDir; source_version = "fixture-source" } | ConvertTo-Json -Compress) | Set-Content -LiteralPath $taskList -Encoding UTF8
    $taskListGate = Invoke-TaskspaceE3StartGate -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot -OutputDir (Join-Path $runDir "gate-tasklist-pass") -TaskListPath $taskList -RunRoot (Join-Path $runDir "runs") -AllowSkippedPathContract -AllowSkippedSelfTests -AllowSkippedOnePairSmoke -AllowSkippedCalibrationGate -SelfTestCommands @()
    Assert-True ([string]$taskListGate.status -eq "pass" -and @($taskListGate.gates | Where-Object { [string]$_.name -eq "path_contract" -and [string]$_.status -eq "skipped_allowed" }).Count -eq 1) "start gate did not require explicit skipped path-contract allow"

    $suiteGateRoot = Join-Path $runDir "suite-start-gate"
    $suiteGateOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot $suiteGateRoot -ScoringMode 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite start gate did not fail closed before scoring run"
    $suiteRootLine = @($suiteGateOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteRunRoot = ([string]$suiteRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteHealthPath = Join-Path $suiteRunRoot "suite-health.json"
    $suiteStartGatePath = Join-Path $suiteRunRoot "start-gate\e3-start-gate.json"
    Assert-True ((Test-Path -LiteralPath $suiteHealthPath) -and (Test-Path -LiteralPath $suiteStartGatePath)) "suite start gate did not write health and gate artifacts"
    $suiteHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteHealthPath | ConvertFrom-Json
    $suiteStartGate = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteStartGatePath | ConvertFrom-Json
    $sampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $suiteRunRoot "samples") -Directory -ErrorAction SilentlyContinue)
    Assert-True ([string]$suiteHealth.status -eq "invalid_harness" -and -not [bool]$suiteHealth.suite_score_valid) "suite start gate health did not mark invalid_harness"
    Assert-True ([string]$suiteStartGate.status -eq "fail" -and @($suiteStartGate.gates | Where-Object { [string]$_.name -eq "one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "suite start gate did not preserve one-pair smoke failure"
    Assert-True (@($suiteStartGate.gates | Where-Object { [string]$_.name -eq "cheap_self_tests" -and [string]$_.status -eq "skipped" -and [string]$_.stable_code -eq "self_tests_skipped_after_previous_failure" }).Count -eq 1) "suite start gate ran self-tests after an earlier gate failure"
    Assert-True ($sampleDirs.Count -eq 0) "suite start gate created sample runs after gate failure"

    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $skipScoringOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $runDir "suite-skip-scoring") -ScoringMode -SkipStartGate 2>&1
    $skipScoringExit = $LASTEXITCODE
    $skipRequireOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot (Join-Path $runDir "suite-skip-require") -RequireScoreValidity -SkipStartGate 2>&1
    $skipRequireExit = $LASTEXITCODE
    $ErrorActionPreference = $oldErrorActionPreference
    Assert-True ($skipScoringExit -eq 4 -and ($skipScoringOutput -join "`n") -match "SkipStartGate is not allowed") "suite allowed SkipStartGate for ScoringMode run"
    Assert-True ($skipRequireExit -eq 4 -and ($skipRequireOutput -join "`n") -match "SkipStartGate is not allowed") "suite allowed SkipStartGate for RequireScoreValidity run"

    $suiteCalibrationRoot = Join-Path $runDir "suite-calibration-gate"
    $suiteCalibrationOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1") -Benchmark terminal-bench -TaskListPath $taskList -SourceVersion selftest -Repeats 5 -RunRoot $suiteCalibrationRoot -ScoringMode -OnePairSmokeRoot $smokeRoot 2>&1
    Assert-True ($LASTEXITCODE -eq 3) "suite start gate did not fail closed when calibration artifacts were missing"
    $suiteCalibrationRootLine = @($suiteCalibrationOutput | Where-Object { [string]$_ -match "^SuiteRoot:" } | Select-Object -First 1)[0]
    $suiteCalibrationRunRoot = ([string]$suiteCalibrationRootLine) -replace "^SuiteRoot:\s*", ""
    $suiteCalibrationStartGate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $suiteCalibrationRunRoot "start-gate\e3-start-gate.json") | ConvertFrom-Json
    $suiteCalibrationSampleDirs = @(Get-ChildItem -LiteralPath (Join-Path $suiteCalibrationRunRoot "samples") -Directory -ErrorAction SilentlyContinue)
    Assert-True (@($suiteCalibrationStartGate.gates | Where-Object { [string]$_.name -eq "calibration_one_pair_smoke" -and [string]$_.status -eq "fail" }).Count -eq 1) "suite start gate did not preserve calibration one-pair timing failure"
    Assert-True ($suiteCalibrationSampleDirs.Count -eq 0) "suite calibration gate created sample runs after gate failure"
} finally {
    if ($null -eq $oldMinFreeBytes) { Remove-Item Env:TASKSPACE_MIN_FREE_BYTES -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_BYTES = $oldMinFreeBytes }
    if ($null -eq $oldMinFreeGib) { Remove-Item Env:TASKSPACE_MIN_FREE_GIB -ErrorAction SilentlyContinue } else { $env:TASKSPACE_MIN_FREE_GIB = $oldMinFreeGib }
}

if ($failures.Count -gt 0) {
    Write-Host "E3 start gate self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "E3 start gate self-test: PASS"
Write-Host "RunRoot: $runDir"
