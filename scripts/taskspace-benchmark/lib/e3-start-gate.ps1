$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "calibration-gate.ps1")

function Invoke-TaskspaceGateCommand {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Command,
        [int]$TimeoutSeconds = 120
    )
    $started = Get-Date
    $job = Start-Job -ScriptBlock {
        param([string]$WorkDir, [string]$InnerCommand)
        Set-Location -LiteralPath $WorkDir
        $output = & powershell -NoProfile -ExecutionPolicy Bypass -Command $InnerCommand 2>&1
        [pscustomobject]@{
            exit_code = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
            output = @($output | ForEach-Object { [string]$_ })
        }
    } -ArgumentList $RepoRoot, $Command
    $timedOut = -not (Wait-Job -Job $job -Timeout $TimeoutSeconds)
    if ($timedOut) {
        Stop-Job -Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        return [pscustomobject]@{ command = $Command; exit_code = 124; timed_out = $true; duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds); output_tail = @("timeout after $TimeoutSeconds seconds") }
    }
    $result = Receive-Job -Job $job | Select-Object -First 1
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    [pscustomobject]@{
        command = $Command
        exit_code = if ($result) { [int]$result.exit_code } else { 1 }
        timed_out = $false
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
        output_tail = @($result.output | Select-Object -Last 20)
    }
}

function New-TaskspaceE3StartGateMarkdown {
    param($Gate)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace E3 Start Gate")
    $lines.Add("")
    $lines.Add("- status: $($Gate.status)")
    $lines.Add("- run_validity: $($Gate.run_validity)")
    $lines.Add("- first_failure_gate: $($Gate.first_failure_gate)")
    $lines.Add("- first_failure_stable_code: $($Gate.first_failure_stable_code)")
    $lines.Add("- first_failure_artifact: $($Gate.first_failure_artifact)")
    $lines.Add("")
    $lines.Add("## Gates")
    foreach ($gateRow in @($Gate.gates)) {
        $lines.Add("- $($gateRow.name): $($gateRow.status) $(if ($gateRow.reason) { '(' + $gateRow.reason + ')' } else { '' })")
    }
    $lines.Add("")
    $lines.Add("## Self Tests")
    if (@($Gate.self_tests).Count -eq 0) { $lines.Add("- skipped") } else {
        foreach ($test in @($Gate.self_tests)) {
            $commandText = [string]$test.command
            $lines.Add("- ``$commandText``: exit=$($test.exit_code) timeout=$($test.timed_out)")
        }
    }
    @($lines.ToArray())
}

function New-TaskspaceE3GateRow {
    param([string]$Name, [string]$Status, [string]$Reason = "", [string]$StableCode = "", [string]$Message = "")
    [pscustomobject]@{ name = $Name; status = $Status; reason = $Reason; stable_code = $StableCode; message = $Message }
}

function New-TaskspaceE3GateDecision {
    param($Gate, [string]$Phase = "R1", [string]$TaskListHash = "", [string]$SourceVersion = "", [string]$ProfileHash = "")
    $passed = ($Gate -and [string]$Gate.status -eq "pass")
    $nextCategory = if ($passed) { "serial_calibration" } else { "fixture_tests" }
    $calibrationPass = $false
    if ($Gate -and $Gate.calibration_gate) {
        $calibrationPass = ([string]$Gate.calibration_gate.status -eq "pass")
    }
    [pscustomobject]@{
        schema_version = 1
        status = if ($passed) { "pass" } else { "blocked" }
        phase = $Phase
        next_allowed_command_category = $nextCategory
        full_e3_allowed = $false
        speed_claim_allowed = $false
        calibration_gate_passed = $calibrationPass
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
        blocking_reasons = @(@($Gate.gates | Where-Object { [string]$_.status -eq "fail" } | ForEach-Object { [string]$_.stable_code }))
        generated_at = (Get-Date).ToString("o")
    }
}

function Convert-TaskspaceCalibrationGateRow {
    param($Row)
    $stableCode = if ([string]::IsNullOrWhiteSpace([string]$Row.reason)) { "" } else { [string]$Row.reason }
    New-TaskspaceE3GateRow "calibration_$($Row.name)" ([string]$Row.status) ([string]$Row.reason) $stableCode ([string]$Row.artifact)
}

function Get-TaskspaceE3TaskListGate {
    param([string]$TaskListPath, [string]$DefaultSourceVersion = "")
    if ([string]::IsNullOrWhiteSpace($TaskListPath)) { return New-TaskspaceE3GateRow "task_list" "skipped" "TaskListPath not set" "task_list_not_provided" "TaskListPath was not provided." }
    if (-not (Test-Path -LiteralPath $TaskListPath)) { return New-TaskspaceE3GateRow "task_list" "fail" "task_list_missing" "task_list_missing" "TaskListPath not found: $TaskListPath" }
    try {
        $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $TaskListPath
        $items = if ($raw.TrimStart().StartsWith("[")) {
            @($raw | ConvertFrom-Json)
        } else {
            @(Get-Content -Encoding UTF8 -LiteralPath $TaskListPath | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_ | ConvertFrom-Json })
        }
    } catch {
        return New-TaskspaceE3GateRow "task_list" "fail" "task_list_malformed" "task_list_malformed" ([string]$_.Exception.Message)
    }
    if (@($items).Count -eq 0) { return New-TaskspaceE3GateRow "task_list" "fail" "task_list_empty" "task_list_empty" "TaskListPath contains no samples." }
    foreach ($item in @($items)) {
        $taskDir = if ($item.PSObject.Properties.Name -contains "task_dir") { [string]$item.task_dir } else { "" }
        $sourceVersion = if ($item.PSObject.Properties.Name -contains "source_version") { [string]$item.source_version } else { "" }
        if ([string]::IsNullOrWhiteSpace($taskDir)) { return New-TaskspaceE3GateRow "task_list" "fail" "task_dir_missing" "task_dir_missing" "Task list sample is missing task_dir." }
        if (-not (Test-Path -LiteralPath $taskDir)) { return New-TaskspaceE3GateRow "task_list" "fail" "task_dir_missing" "task_dir_missing" "Task list task_dir not found: $taskDir" }
        if ([string]::IsNullOrWhiteSpace($sourceVersion) -and [string]::IsNullOrWhiteSpace($DefaultSourceVersion)) {
            return New-TaskspaceE3GateRow "task_list" "fail" "source_version_missing" "source_version_missing" "Task list sample is missing source_version and no default SourceVersion was provided."
        }
    }
    New-TaskspaceE3GateRow "task_list" "pass"
}

function Get-TaskspaceE3OnePairSmokeGate {
    param([string]$OnePairSmokeRoot)
    if ([string]::IsNullOrWhiteSpace($OnePairSmokeRoot)) {
        return New-TaskspaceE3GateRow "one_pair_smoke" "skipped" "OnePairSmokeRoot not set" "one_pair_smoke_not_provided" "One-pair smoke artifact root was not provided."
    }
    if (-not (Test-Path -LiteralPath $OnePairSmokeRoot)) {
        return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_smoke_missing" "one_pair_smoke_missing" "OnePairSmokeRoot not found: $OnePairSmokeRoot"
    }
    $aggregatePath = Get-ChildItem -LiteralPath $OnePairSmokeRoot -Filter "aggregate.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($aggregatePath) {
        try {
            $aggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath.FullName | ConvertFrom-Json
            if ($aggregate.PSObject.Properties.Name -contains "score_valid" -and [bool]$aggregate.score_valid) {
                return New-TaskspaceE3GateRow "one_pair_smoke" "pass" "" "" ([string]$aggregatePath.FullName)
            }
            if ($aggregate.PSObject.Properties.Name -contains "run_validity" -and [string]$aggregate.run_validity -eq "invalid_harness") {
                return New-TaskspaceE3GateRow "one_pair_smoke" "pass" "classified_invalid_harness" "" ([string]$aggregatePath.FullName)
            }
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_score_invalid_unclassified" "one_pair_score_invalid_unclassified" ([string]$aggregatePath.FullName)
        } catch {
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_aggregate_malformed" "one_pair_aggregate_malformed" ([string]$_.Exception.Message)
        }
    }
    $suiteHealthPath = Get-ChildItem -LiteralPath $OnePairSmokeRoot -Filter "suite-health.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($suiteHealthPath) {
        try {
            $suiteHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath $suiteHealthPath.FullName | ConvertFrom-Json
            if ([string]$suiteHealth.status -eq "invalid_harness" -and -not [string]::IsNullOrWhiteSpace([string]$suiteHealth.suite_abort_reason)) {
                return New-TaskspaceE3GateRow "one_pair_smoke" "pass" "classified_invalid_harness" "" ([string]$suiteHealthPath.FullName)
            }
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_suite_health_not_classified" "one_pair_suite_health_not_classified" ([string]$suiteHealthPath.FullName)
        } catch {
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_suite_health_malformed" "one_pair_suite_health_malformed" ([string]$_.Exception.Message)
        }
    }
    $sampleStatusPath = Get-ChildItem -LiteralPath $OnePairSmokeRoot -Filter "sample-status.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($sampleStatusPath) {
        try {
            $sampleStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $sampleStatusPath.FullName | ConvertFrom-Json
            if ([string]$sampleStatus.run_validity -eq "invalid_harness" -and -not [string]::IsNullOrWhiteSpace([string]$sampleStatus.abort_signature)) {
                return New-TaskspaceE3GateRow "one_pair_smoke" "pass" "classified_invalid_harness" "" ([string]$sampleStatusPath.FullName)
            }
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_sample_status_not_classified" "one_pair_sample_status_not_classified" ([string]$sampleStatusPath.FullName)
        } catch {
            return New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_sample_status_malformed" "one_pair_sample_status_malformed" ([string]$_.Exception.Message)
        }
    }
    New-TaskspaceE3GateRow "one_pair_smoke" "fail" "one_pair_smoke_artifact_missing" "one_pair_smoke_artifact_missing" "No aggregate.json, suite-health.json, or sample-status.json found under OnePairSmokeRoot."
}

function New-TaskspaceE3SetupFailureGate {
    param([string]$OutputDir, [string]$Message, [string]$StableCode = "start_gate_setup_failed")
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    $jsonPath = Join-Path $OutputDir "e3-start-gate.json"
    $markdownPath = Join-Path $OutputDir "e3-start-gate.md"
    $gate = [pscustomobject]@{
        schema_version = 1
        status = "fail"
        run_validity = "invalid_harness"
        exit_code = 3
        gates = @((New-TaskspaceE3GateRow "setup" "fail" $StableCode $StableCode $Message))
        self_tests = @()
        first_failure_gate = "setup"
        first_failure_stable_code = $StableCode
        first_failure_message = $Message
        first_failure_artifact = $jsonPath
        generated_at = (Get-Date).ToString("o")
    }
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    New-TaskspaceE3StartGateMarkdown $gate | Set-Content -LiteralPath $markdownPath -Encoding UTF8
    $gate | Add-Member -NotePropertyName json_path -NotePropertyValue $jsonPath -Force
    $gate | Add-Member -NotePropertyName markdown_path -NotePropertyValue $markdownPath -Force
    $gate
}

function Invoke-TaskspaceE3StartGate {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$BenchmarkRoot,
        [Parameter(Mandatory = $true)][string]$OutputDir,
        [string]$Scenario = "",
        [string]$ScenarioPath = "",
        [string]$RunRoot = "",
        [string]$TaskListPath = "",
        [string]$SourceVersion = "",
        [string]$ExpectedTaskListHash = "",
        [string]$ExpectedProfileHash = "",
        [string]$OnePairSmokeRoot = "",
        [string]$SerialCalibrationRoot = "",
        [string]$ParallelEquivalencePath = "",
        [switch]$RunSelfTests,
        [switch]$AllowSkippedPathContract,
        [switch]$AllowSkippedSelfTests,
        [switch]$AllowSkippedOnePairSmoke,
        [switch]$AllowSkippedCalibrationGate,
        [string[]]$SelfTestCommands = @(
            ".\scripts\taskspace-benchmark\test-e3-score-validity.ps1",
            ".\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1",
            ".\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1",
            ".\scripts\taskspace-benchmark\test-e3-proof-harness.ps1",
            ".\scripts\taskspace-benchmark\test-harness.ps1"
        )
    )
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    if ([string]::IsNullOrWhiteSpace($RunRoot)) { $RunRoot = Join-Path $OutputDir "runs" }
    $RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
    $paths = New-Object System.Collections.Generic.List[string]
    $paths.Add($OutputDir); $paths.Add($RunRoot); $paths.Add($RepoRoot)
    if ($TaskListPath) { $paths.Add($TaskListPath) }
    $manifest = $null
    $manifestHealth = $null
    if (-not [string]::IsNullOrWhiteSpace($Scenario) -or -not [string]::IsNullOrWhiteSpace($ScenarioPath)) {
        try {
            $manifest = Read-TaskspaceScenarioManifest $RepoRoot $Scenario $ScenarioPath
        } catch {
            return New-TaskspaceE3SetupFailureGate $OutputDir ([string]$_.Exception.Message) "scenario_manifest_invalid"
        }
        $paths.Add($manifest.ScenarioRoot)
        $manifestHealth = Get-TaskspaceHarnessHealth $manifest $RunRoot $manifest.ScenarioRoot
    }
    $diskHealth = New-TaskspaceDiskHealth @($paths.ToArray()) "e3_start_gate"
    $gates = New-Object System.Collections.Generic.List[object]
    $gates.Add((New-TaskspaceE3GateRow "disk_preflight" ([string]$diskHealth.status) $(if ([string]$diskHealth.status -eq "pass") { "" } else { "disk_space_low" }) $(if ([string]$diskHealth.status -eq "pass") { "" } else { "disk_space_low" })))
    $dockerFailures = @($diskHealth.docker_storage_checks | Where-Object { [string]$_.status -eq "fail" })
    $dockerStatus = if (@($diskHealth.docker_storage_checks).Count -eq 0) { "fail" } elseif ($dockerFailures.Count -eq 0) { "pass" } else { "fail" }
    $dockerReason = if (@($diskHealth.docker_storage_checks).Count -eq 0) { "docker_storage_unverified" } elseif ($dockerFailures.Count -eq 0) { "" } else { "docker_storage_low" }
    $gates.Add((New-TaskspaceE3GateRow "docker_storage" $dockerStatus $dockerReason $dockerReason))
    if ($manifestHealth) {
        $pathFailures = @($manifestHealth.findings | Where-Object { [string]$_.stable_code -in @("relative_materialized_path", "path_unresolvable", "uv_cache_missing", "validator_source_missing") })
        $hardManifestFailures = @($manifestHealth.findings | Where-Object { [string]$_.severity -eq "fail" })
        $manifestReason = if ($pathFailures.Count -gt 0) { [string]$pathFailures[0].stable_code } elseif ($hardManifestFailures.Count -gt 0) { [string]$hardManifestFailures[0].stable_code } else { "" }
        $gates.Add((New-TaskspaceE3GateRow "path_contract" $(if ($manifestReason) { "fail" } else { "pass" }) $manifestReason $manifestReason))
    } else {
        $gates.Add((New-TaskspaceE3GateRow "path_contract" $(if ($AllowSkippedPathContract) { "skipped_allowed" } else { "fail" }) "no_scenario_manifest" "path_contract_not_checked"))
    }
    $taskListGate = Get-TaskspaceE3TaskListGate $TaskListPath $SourceVersion
    if ([string]$taskListGate.status -ne "skipped") { $gates.Add($taskListGate) }
    $smokeGate = Get-TaskspaceE3OnePairSmokeGate $OnePairSmokeRoot
    if ([string]$smokeGate.status -eq "skipped" -and $AllowSkippedOnePairSmoke) {
        $gates.Add((New-TaskspaceE3GateRow "one_pair_smoke" "skipped_allowed" "OnePairSmokeRoot not set" "one_pair_smoke_not_provided"))
    } elseif ([string]$smokeGate.status -eq "skipped") {
        $gates.Add((New-TaskspaceE3GateRow "one_pair_smoke" "fail" "OnePairSmokeRoot not set" "one_pair_smoke_not_provided"))
    } else {
        $gates.Add($smokeGate)
    }
    if ($AllowSkippedCalibrationGate) {
        $gates.Add((New-TaskspaceE3GateRow "calibration_gate" "skipped_allowed" "calibration gate explicitly skipped" "calibration_gate_skipped_allowed"))
        $calibrationGate = $null
    } else {
        $calibrationGate = Invoke-TaskspaceCalibrationGate `
            -OnePairSmokeRoot $OnePairSmokeRoot `
            -SerialCalibrationRoot $SerialCalibrationRoot `
            -ParallelEquivalencePath $ParallelEquivalencePath `
            -ExpectedTaskListHash $ExpectedTaskListHash `
            -ExpectedSourceVersion $SourceVersion `
            -ExpectedProfileHash $ExpectedProfileHash `
            -OutputPath (Join-Path $OutputDir "calibration-gate.json")
        foreach ($calibrationRow in @($calibrationGate.gates)) {
            $gates.Add((Convert-TaskspaceCalibrationGateRow $calibrationRow))
        }
    }
    $selfTests = @()
    $preSelfTestFailures = @($gates.ToArray() | Where-Object { [string]$_.status -eq "fail" })
    if ($RunSelfTests -and $preSelfTestFailures.Count -eq 0) {
        $selfTests = @($SelfTestCommands | ForEach-Object { Invoke-TaskspaceGateCommand $RepoRoot $_ 180 })
        $failedTest = @($selfTests | Where-Object { [int]$_.exit_code -ne 0 } | Select-Object -First 1)[0]
        $gates.Add((New-TaskspaceE3GateRow "cheap_self_tests" $(if ($failedTest) { "fail" } else { "pass" }) $(if ($failedTest) { "self_test_failed" } else { "" }) $(if ($failedTest) { "self_test_failed" } else { "" }) $(if ($failedTest) { [string]$failedTest.command } else { "" })))
    } elseif ($RunSelfTests) {
        $gates.Add((New-TaskspaceE3GateRow "cheap_self_tests" "skipped" "previous_gate_failed" "self_tests_skipped_after_previous_failure"))
    } else {
        $gates.Add((New-TaskspaceE3GateRow "cheap_self_tests" $(if ($AllowSkippedSelfTests) { "skipped_allowed" } else { "fail" }) "RunSelfTests not set" "self_tests_not_run"))
    }
    $failed = @($gates.ToArray() | Where-Object { [string]$_.status -eq "fail" })
    $firstFailure = @($failed | Select-Object -First 1)[0]
    $jsonPath = Join-Path $OutputDir "e3-start-gate.json"
    $markdownPath = Join-Path $OutputDir "e3-start-gate.md"
    $gate = [pscustomobject]@{
        schema_version = 1
        status = if ($failed.Count -eq 0) { "pass" } else { "fail" }
        run_validity = if ($failed.Count -eq 0) { "valid" } else { "invalid_harness" }
        exit_code = if ($failed.Count -eq 0) { 0 } else { 3 }
        gates = @($gates.ToArray())
        self_tests = @($selfTests)
        disk_health = $diskHealth
        manifest_health = $manifestHealth
        calibration_gate = $calibrationGate
        skipped_gate_policy = [pscustomobject]@{ allow_skipped_path_contract = [bool]$AllowSkippedPathContract; allow_skipped_self_tests = [bool]$AllowSkippedSelfTests; allow_skipped_one_pair_smoke = [bool]$AllowSkippedOnePairSmoke; allow_skipped_calibration_gate = [bool]$AllowSkippedCalibrationGate }
        first_failure_gate = if ($firstFailure) { [string]$firstFailure.name } else { "" }
        first_failure_stable_code = if ($firstFailure) { [string]$firstFailure.stable_code } else { "" }
        first_failure_message = if ($firstFailure) { [string]$firstFailure.message } else { "" }
        first_failure_command = if ($firstFailure -and [string]$firstFailure.name -eq "cheap_self_tests") { [string]$firstFailure.message } else { "" }
        first_failure_output_tail = if ($firstFailure -and [string]$firstFailure.name -eq "cheap_self_tests") { @($selfTests | Where-Object { [string]$_.command -eq [string]$firstFailure.message } | Select-Object -First 1 | ForEach-Object { @($_.output_tail) }) } else { @() }
        first_failure_artifact = if ($failed.Count -eq 0) { "" } else { $jsonPath }
        generated_at = (Get-Date).ToString("o")
    }
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    New-TaskspaceE3StartGateMarkdown $gate | Set-Content -LiteralPath $markdownPath -Encoding UTF8
    $gateDecisionPath = Join-Path $OutputDir "gate-decision.json"
    New-TaskspaceE3GateDecision $gate "R1" $ExpectedTaskListHash $SourceVersion $ExpectedProfileHash | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $gateDecisionPath -Encoding UTF8
    $gate | Add-Member -NotePropertyName json_path -NotePropertyValue $jsonPath -Force
    $gate | Add-Member -NotePropertyName markdown_path -NotePropertyValue $markdownPath -Force
    $gate | Add-Member -NotePropertyName gate_decision_path -NotePropertyValue $gateDecisionPath -Force
    $gate
}
