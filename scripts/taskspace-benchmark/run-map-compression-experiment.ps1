param(
    [Parameter(Mandatory = $true)][string]$Phase,
    [Parameter(Mandatory = $true)][string]$CandidateWhaleBin,
    [string]$ContractPath = "",
    [string]$RunRoot = "",
    [int]$Repeats = 3,
    [ValidateRange(1, 8)][int]$MaxParallel = 3,
    [switch]$SkipStandard,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\bootstrap.ps1") -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot

function Resolve-RepoRelativePath {
    param([Parameter(Mandatory = $true)][string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return (Resolve-Path -LiteralPath $Path).Path
    }
    (Resolve-Path -LiteralPath (Join-Path $repoRoot $Path)).Path
}

function Get-LowerSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Import-LocalCredentialIfNeeded {
    param([Parameter(Mandatory = $true)][string]$Name)
    if (-not [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name))) { return }
    $envPath = Join-Path $repoRoot ".env.local"
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) { return }
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $envPath) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) { continue }
        $separator = $trimmed.IndexOf("=")
        if ($separator -lt 1 -or $trimmed.Substring(0, $separator).Trim() -ne $Name) { continue }
        $value = $trimmed.Substring($separator + 1).Trim()
        if ($value.Length -ge 2 -and (($value.StartsWith('"') -and $value.EndsWith('"')) -or
            ($value.StartsWith("'") -and $value.EndsWith("'")))) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        [Environment]::SetEnvironmentVariable($Name, $value, "Process")
        return
    }
}

function Assert-ExpectedSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $actual = Get-LowerSha256 $Path
    if ($actual -ne $Expected.ToLowerInvariant()) {
        throw "$Label sha256 mismatch: expected $Expected, got $actual"
    }
}

function Get-ArmOrder {
    param([int]$Repeat, [string[]]$ArmIds)
    if ($ArmIds.Count -le 1) { return $ArmIds }
    $offset = ($Repeat - 1) % $ArmIds.Count
    if ($offset -eq 0) { return $ArmIds }
    @($ArmIds[$offset..($ArmIds.Count - 1)] + $ArmIds[0..($offset - 1)])
}

function Find-SelectedMetrics {
    param([string]$Root, [string]$SelectedSide)
    $suffix = [System.IO.Path]::Combine($SelectedSide, "artifacts", "metrics.json")
    $matches = @(Get-ChildItem -LiteralPath $Root -Filter "metrics.json" -File -Recurse |
        Where-Object { $_.FullName.EndsWith($suffix, [StringComparison]::OrdinalIgnoreCase) })
    if ($matches.Count -ne 1) {
        return ""
    }
    $matches[0].FullName
}

if ($Repeats -lt 1) { throw "Repeats must be >= 1" }
if ([string]::IsNullOrWhiteSpace($ContractPath)) {
    $ContractPath = Join-Path $repoRoot "benchmarks/taskspace/map-compression/experiment-contract.json"
}
$ContractPath = Resolve-RepoRelativePath $ContractPath
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $ContractPath | ConvertFrom-Json
foreach ($credentialName in @($contract.provider.credential_env_names)) {
    Import-LocalCredentialIfNeeded ([string]$credentialName)
}
$baselineBin = Resolve-RepoRelativePath ([string]$contract.baseline_binary.repo_relative_path)
$baselineAttestation = Resolve-RepoRelativePath ([string]$contract.baseline_binary.attestation_repo_relative_path)
$CandidateWhaleBin = Resolve-RepoRelativePath $CandidateWhaleBin
$candidateAttestation = "$CandidateWhaleBin.build-attestation.json"
if (-not (Test-Path -LiteralPath $candidateAttestation -PathType Leaf)) {
    throw "candidate attestation missing: $candidateAttestation"
}

Assert-ExpectedSha256 $baselineBin ([string]$contract.baseline_binary.sha256) "B0 binary"
Assert-ExpectedSha256 $baselineAttestation ([string]$contract.baseline_binary.attestation_sha256) "B0 attestation"
Assert-ExpectedSha256 (Resolve-RepoRelativePath ([string]$contract.container.contract_repo_relative_path)) ([string]$contract.container.contract_sha256) "container contract"
Assert-ExpectedSha256 (Resolve-RepoRelativePath ([string]$contract.runner.entrypoint)) ([string]$contract.runner.entrypoint_sha256) "benchmark runner"
Assert-ExpectedSha256 (Resolve-RepoRelativePath ([string]$contract.runner.observer)) ([string]$contract.runner.observer_sha256) "benchmark observer"

$candidateSha = Get-LowerSha256 $CandidateWhaleBin
$candidateAttestationJson = Get-Content -Raw -Encoding UTF8 -LiteralPath $candidateAttestation | ConvertFrom-Json
if ([string]$candidateAttestationJson.whale_binary_sha256 -ne $candidateSha) {
    throw "candidate attestation does not match candidate binary"
}
$orchestratorCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
$candidateCodexCommit = (& git -C $repoRoot log -1 --format=%H -- third_party/codex-cli).Trim()
if ([string]$candidateAttestationJson.codex_source_latest_commit -ne $candidateCodexCommit) {
    throw "candidate attestation Codex source mismatch: rebuild and attest current Codex source"
}
$candidateCommit = [string]$candidateAttestationJson.current_git_head

foreach ($sampleName in @("simple", "complex")) {
    $sample = $contract.samples.$sampleName
    $scenarioRoot = Join-Path $repoRoot "benchmarks/taskspace/scenarios/$([string]$sample.scenario)"
    Assert-ExpectedSha256 (Join-Path $scenarioRoot "prompt.txt") ([string]$sample.prompt_sha256) "$sampleName prompt"
    if ($sample.PSObject.Properties.Name -contains "prelude_prompt_repo_relative_path") {
        Assert-ExpectedSha256 (Resolve-RepoRelativePath ([string]$sample.prelude_prompt_repo_relative_path)) ([string]$sample.prelude_prompt_sha256) "$sampleName prelude prompt"
    }
    $fixtureSha = Get-TaskspaceDirectorySha256 (Join-Path $scenarioRoot "fixture")
    if ($fixtureSha -ne [string]$sample.fixture_sha256) {
        throw "$sampleName fixture sha256 mismatch: expected $($sample.fixture_sha256), got $fixtureSha"
    }
}

if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
    $RunRoot = Join-Path $repoRoot "target/r5-map-compression/$Phase/$stamp"
}
$RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null

$arms = [ordered]@{}
if (-not $SkipStandard) {
    $arms["STD"] = [pscustomobject]@{ mode = "standard"; side = "left"; binary = $CandidateWhaleBin; commit = $candidateCommit; allow_stale = $false }
}
$arms["B0"] = [pscustomobject]@{ mode = "taskspace"; side = "right"; binary = $baselineBin; commit = [string]$contract.baseline_source_commit; allow_stale = $true }
$arms["C"] = [pscustomobject]@{ mode = "taskspace"; side = "right"; binary = $CandidateWhaleBin; commit = $candidateCommit; allow_stale = $false }

$tasks = [System.Collections.Generic.List[object]]::new()
foreach ($sampleName in @("simple", "complex")) {
    $sample = $contract.samples.$sampleName
    $scenario = [string]$sample.scenario
    for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
        foreach ($armId in @(Get-ArmOrder $repeat @($arms.Keys))) {
            $arm = $arms[$armId]
            $armRoot = Join-Path $RunRoot "$sampleName/repeat-$('{0:000}' -f $repeat)/$armId"
            New-Item -ItemType Directory -Force -Path $armRoot | Out-Null
            $arguments = @(
                "-NoProfile", "-File", (Join-Path $PSScriptRoot "run-taskspace-benchmark.ps1"),
                "-Scenario", $scenario,
                "-Repeats", "1",
                "-RunRoot", $armRoot,
                "-WhaleBin", [string]$arm.binary,
                "-Model", [string]$contract.provider.model,
                "-SourceVersion", [string]$arm.commit,
                "-SampleSetId", "r5-map-compression-$Phase",
                "-SampleNames", $scenario,
                "-BenchmarkFamily", "r5-map-compression",
                "-RunnerEntrypoint", "scripts/taskspace-benchmark/run-map-compression-experiment.ps1",
                "-ArtifactOrigin", "r5-map-compression-$Phase",
                "-SuiteManifestPath", $ContractPath,
                "-RunSide", [string]$arm.side,
                "-EnableDockerImageCache",
                "-AllowNonE2Result"
            )
            if ([bool]$arm.allow_stale) { $arguments += "-AllowStaleWhaleBin" }
            if ($sample.PSObject.Properties.Name -contains "prelude_prompt_repo_relative_path") {
                $arguments += @("-PreludePromptPath", (Resolve-RepoRelativePath ([string]$sample.prelude_prompt_repo_relative_path)))
            }
            $additionalOverrides = @($contract.provider.config_overrides |
                ForEach-Object { [string]$_ } |
                Where-Object { $_ -ne 'model_reasoning_effort="max"' })
            if ($additionalOverrides.Count -gt 1) {
                throw "map compression runner supports one additional config override, got $($additionalOverrides.Count)"
            }
            if ($additionalOverrides.Count -eq 1) {
                $arguments += @("-AdditionalConfigOverride", [string]$additionalOverrides[0])
            }
            $tasks.Add([pscustomobject]@{
                sample_class = $sampleName
                scenario = $scenario
                repeat = $repeat
                arm = $armId
                logical_mode = [string]$arm.mode
                selected_side = [string]$arm.side
                binary_sha256 = Get-LowerSha256 ([string]$arm.binary)
                source_commit = [string]$arm.commit
                run_root = $armRoot
                stdout_path = Join-Path $armRoot "super-run.stdout.log"
                stderr_path = Join-Path $armRoot "super-run.stderr.log"
                arguments = $arguments
            })
        }
    }
}

if ($PlanOnly) {
    $planPath = Join-Path $RunRoot "run-plan.json"
    [ordered]@{
        schema_version = "taskspace-map-compression-run-plan-v1"
        phase = $Phase
        orchestrator_commit = $orchestratorCommit
        candidate_commit = $candidateCommit
        candidate_binary_sha256 = $candidateSha
        baseline_binary_sha256 = [string]$contract.baseline_binary.sha256
        provider_config_overrides = @($contract.provider.config_overrides | ForEach-Object { [string]$_ })
        repeats = $Repeats
        max_parallel = $MaxParallel
        tasks = @($tasks | Select-Object sample_class, scenario, repeat, arm, logical_mode, selected_side, binary_sha256, source_commit, run_root)
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $planPath -Encoding UTF8
    Write-Host "MapCompressionRunPlan: $planPath"
    exit 0
}

$pending = [System.Collections.Queue]::new()
foreach ($task in $tasks) { $pending.Enqueue($task) }
$running = @{}
$results = [System.Collections.Generic.List[object]]::new()
while ($pending.Count -gt 0 -or $running.Count -gt 0) {
    while ($pending.Count -gt 0 -and $running.Count -lt $MaxParallel) {
        $task = $pending.Dequeue()
        $job = Start-Job -ScriptBlock {
            param($Task)
            & pwsh @($Task.arguments) 1> $Task.stdout_path 2> $Task.stderr_path
            [pscustomobject]@{ exit_code = $LASTEXITCODE; task = $Task }
        } -ArgumentList $task
        $running[[string]$job.Id] = $job
    }
    if ($running.Count -eq 0) { continue }
    $completed = Wait-Job -Job @($running.Values) -Any -Timeout 5
    if ($null -eq $completed) { continue }
    $result = Receive-Job -Job $completed
    $task = $result.task
    $metricsPath = Find-SelectedMetrics ([string]$task.run_root) ([string]$task.selected_side)
    $results.Add([pscustomobject]@{
        sample_class = [string]$task.sample_class
        scenario = [string]$task.scenario
        repeat = [int]$task.repeat
        arm = [string]$task.arm
        logical_mode = [string]$task.logical_mode
        selected_side = [string]$task.selected_side
        binary_sha256 = [string]$task.binary_sha256
        source_commit = [string]$task.source_commit
        exit_code = [int]$result.exit_code
        run_root = [string]$task.run_root
        metrics_path = $metricsPath
        stdout_path = [string]$task.stdout_path
        stderr_path = [string]$task.stderr_path
    })
    Remove-Job -Job $completed -Force
    $running.Remove([string]$completed.Id)
}

$index = [ordered]@{
    schema_version = "taskspace-map-compression-run-index-v1"
    phase = $Phase
    orchestrator_commit = $orchestratorCommit
    super_runner_sha256 = Get-LowerSha256 $PSCommandPath
    experiment_observer_sha256 = Get-LowerSha256 (Join-Path $PSScriptRoot "observe-map-compression-experiment.ps1")
    contract_path = $ContractPath
    contract_sha256 = Get-LowerSha256 $ContractPath
    candidate_commit = $candidateCommit
    candidate_binary_sha256 = $candidateSha
    baseline_binary_sha256 = [string]$contract.baseline_binary.sha256
    provider_config_overrides = @($contract.provider.config_overrides | ForEach-Object { [string]$_ })
    p0_alias_of = "B0"
    repeats = $Repeats
    max_parallel = $MaxParallel
    started_order_balanced = $true
    results = @($results | Sort-Object sample_class, repeat, arm)
}
$indexPath = Join-Path $RunRoot "run-index.json"
$index | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $indexPath -Encoding UTF8
Write-Host "MapCompressionRunIndex: $indexPath"

$failed = @($results | Where-Object { $_.exit_code -ne 0 -or [string]::IsNullOrWhiteSpace($_.metrics_path) })
if ($failed.Count -gt 0) { exit 1 }
