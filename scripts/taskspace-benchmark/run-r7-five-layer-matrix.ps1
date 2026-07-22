param(
    [ValidateSet("initial", "extended")][string]$Stage = "initial",
    [int]$Repeats = 0,
    [string]$RunRoot = "",
    [string]$ExecutionRoot = "",
    [string]$WhaleBin = "third_party/codex-cli/codex-rs/target/debug/whale",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 1800,
    [int]$ThrottleLimit = 3,
    [switch]$AllowExtended,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$contractPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-evaluation-contract-v1.json"
$baseRunner = Join-Path $repoRoot "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath | ConvertFrom-Json -Depth 50

function Write-Utf8Json {
    param([Parameter(Mandatory = $true)]$Value, [Parameter(Mandatory = $true)][string]$Path)
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    [IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth 50) + "`n"), [Text.UTF8Encoding]::new($false))
}

function Import-DeepSeekCredential {
    if (-not [string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)) { return }
    $envPath = Join-Path $repoRoot ".env.local"
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) {
        throw "DEEPSEEK_API_KEY is missing and .env.local does not exist."
    }
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $envPath) {
        if ($line -match '^\s*DEEPSEEK_API_KEY\s*=\s*(.+?)\s*$') {
            $value = [string]$Matches[1]
            if (($value.StartsWith('"') -and $value.EndsWith('"')) -or ($value.StartsWith("'") -and $value.EndsWith("'"))) {
                $value = $value.Substring(1, $value.Length - 2)
            }
            $env:DEEPSEEK_API_KEY = $value
            break
        }
    }
    if ([string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)) {
        throw "DEEPSEEK_API_KEY is missing from .env.local."
    }
}

if ($Stage -eq "extended" -and -not $AllowExtended) {
    throw "Extended repeat-10 evaluation requires explicit -AllowExtended after the initial report."
}
$stageContract = if ($Stage -eq "initial") { $contract.run_design.initial_observation } else { $contract.run_design.extended_observation }
$expectedRepeats = [int]$stageContract.repeats_per_arm_per_sample
if ($Repeats -eq 0) { $Repeats = $expectedRepeats }
if ($Repeats -ne $expectedRepeats) {
    throw "$Stage evaluation requires exactly $expectedRepeats repeats per arm per sample."
}
if ($ThrottleLimit -lt 1) { throw "ThrottleLimit must be at least 1." }

$samples = if ($Stage -eq "initial") {
    @($contract.samples.development_smoke | ForEach-Object { [string]$_ })
} else {
    @($contract.samples.development_smoke + $contract.samples.held_out_formal | ForEach-Object { [string]$_ })
}
$armOrders = if ($Stage -eq "initial") { @($stageContract.arm_order) } else { @($stageContract.arm_order_cycle) }
$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
$runId = Get-Date -Format "yyyyMMdd-HHmmss-fff"
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target/r7-five-layer-matrix/$($contract.contract_id)/$commit/$runId"
} elseif (-not [IO.Path]::IsPathRooted($RunRoot)) {
    $RunRoot = Join-Path $repoRoot $RunRoot
}
New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null
if ([string]::IsNullOrWhiteSpace($ExecutionRoot)) {
    $ExecutionRoot = if ($PlanOnly) {
        Join-Path $RunRoot "_execution-plan"
    } else {
        Join-Path $repoRoot "target/r7-five-layer-eval-data/$commit/$runId"
    }
} elseif (-not [IO.Path]::IsPathRooted($ExecutionRoot)) {
    $ExecutionRoot = Join-Path $repoRoot $ExecutionRoot
}
New-Item -ItemType Directory -Force -Path $ExecutionRoot | Out-Null

$plans = [Collections.Generic.List[object]]::new()
$armCodes = @{ standard = "a0"; "map-always" = "a1"; "map-append" = "a2"; "map-request" = "a3" }
foreach ($sample in $samples) {
    for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
        $order = @($armOrders[($repeat - 1) % $armOrders.Count] | ForEach-Object { [string]$_ })
        foreach ($arm in $order) {
            $logicalMode = if ($arm -eq "standard") { "standard" } else { "taskspace" }
            $plans.Add([pscustomobject]@{
                    sample = $sample
                    repeat = $repeat
                    arm = $arm
                    logical_mode = $logicalMode
                    run_side = if ($logicalMode -eq "standard") { "left" } else { "right" }
                    projection_policy = if ($logicalMode -eq "standard") { "map-request" } else { $arm }
                    arm_run_root = Join-Path $ExecutionRoot "raw/$sample/r-$repeat/$($armCodes[$arm])"
                })
        }
    }
}

$manifestPath = Join-Path $RunRoot "run-manifest.json"
$manifest = [ordered]@{
    schema_version = 1
    contract_id = [string]$contract.contract_id
    stage = $Stage
    status = if ($PlanOnly) { "planned" } else { "running" }
    generated_at = (Get-Date).ToString("o")
    repo_commit = $commit
    whale_bin = $WhaleBin
    whale_sha256 = if (-not $PlanOnly -and (Test-Path -LiteralPath $WhaleBin -PathType Leaf)) { (Get-FileHash -Algorithm SHA256 -LiteralPath $WhaleBin).Hash.ToLowerInvariant() } else { "" }
    model = $Model
    execution = "docker"
    execution_root = $ExecutionRoot
    repeats_per_arm_per_sample = $Repeats
    samples = $samples
    arms = @("standard", "map-always", "map-append", "map-request")
    throttle_limit = $ThrottleLimit
    planned_run_count = $plans.Count
    completed_run_count = 0
    runs = @($plans)
}
Write-Utf8Json $manifest $manifestPath
if ($PlanOnly) {
    Write-Output "R7FiveLayerRunRoot: $RunRoot"
    Write-Output "R7FiveLayerManifest: $manifestPath"
    return
}

Import-DeepSeekCredential
if (-not [IO.Path]::IsPathRooted($WhaleBin)) { $WhaleBin = Join-Path $repoRoot $WhaleBin }
if (-not (Test-Path -LiteralPath $WhaleBin -PathType Leaf)) { throw "Whale binary not found: $WhaleBin" }
& docker info *> $null
if ($LASTEXITCODE -ne 0) { throw "Docker daemon is unavailable." }

$groups = @($plans | Group-Object { "$($_.sample)|$($_.repeat)" })
$worker = {
    param($GroupPlansJson, $BaseRunner, $WhaleBin, $Model, $TimeoutSeconds, $ApiKey)
    $ErrorActionPreference = "Stop"
    $env:DEEPSEEK_API_KEY = $ApiKey
    $GroupPlans = @($GroupPlansJson | ConvertFrom-Json -Depth 20)
    $records = [Collections.Generic.List[object]]::new()
    foreach ($plan in @($GroupPlans)) {
        New-Item -ItemType Directory -Force -Path $plan.arm_run_root | Out-Null
        $runnerLog = Join-Path $plan.arm_run_root "runner.log"
        $arguments = @(
            "-NoProfile", "-File", $BaseRunner,
            "-Scenario", $plan.sample,
            "-Repeats", "1",
            "-RunRoot", $plan.arm_run_root,
            "-WhaleBin", $WhaleBin,
            "-Model", $Model,
            "-TimeoutSeconds", [string]$TimeoutSeconds,
            "-TaskSpaceProjectionPolicy", $plan.projection_policy,
            "-RunSide", $plan.run_side,
            "-AllowNonE2Result",
            "-EnableDockerImageCache"
        )
        $startedAt = Get-Date
        $output = @(& pwsh @arguments 2>&1)
        $exitCode = $LASTEXITCODE
        $output | Set-Content -Encoding UTF8 -LiteralPath $runnerLog
        $scenarioRoot = Join-Path $plan.arm_run_root $plan.sample
        $runDir = @(Get-ChildItem -LiteralPath $scenarioRoot -Directory -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).FullName
        $records.Add([pscustomobject]@{
                sample = $plan.sample
                repeat = $plan.repeat
                arm = $plan.arm
                logical_mode = $plan.logical_mode
                projection_policy = $plan.projection_policy
                run_side = $plan.run_side
                exit_code = $exitCode
                started_at = $startedAt.ToString("o")
                finished_at = (Get-Date).ToString("o")
                run_dir = [string]$runDir
                runner_log = $runnerLog
            })
        if ($exitCode -ne 0) { throw "Base runner failed for $($plan.sample) repeat $($plan.repeat) $($plan.arm): exit $exitCode" }
    }
    $resultPath = Join-Path (Split-Path -Parent $GroupPlans[0].arm_run_root) "group-result.json"
    [IO.File]::WriteAllText($resultPath, (($records | ConvertTo-Json -Depth 20) + "`n"), [Text.UTF8Encoding]::new($false))
    $resultPath
}

for ($offset = 0; $offset -lt $groups.Count; $offset += $ThrottleLimit) {
    $batch = @($groups[$offset..([Math]::Min($groups.Count - 1, $offset + $ThrottleLimit - 1))])
    $jobs = @($batch | ForEach-Object {
            $groupJson = @($_.Group) | ConvertTo-Json -Depth 20 -Compress
            Start-Job -ScriptBlock $worker -ArgumentList @($groupJson, $baseRunner, $WhaleBin, $Model, $TimeoutSeconds, $env:DEEPSEEK_API_KEY)
        })
    $jobs | Wait-Job | Out-Null
    foreach ($job in $jobs) {
        $jobOutput = @(Receive-Job -Job $job -ErrorAction Continue)
        if ($job.State -ne "Completed") {
            throw "Matrix worker failed: $($job.ChildJobs[0].JobStateInfo.Reason.Message)"
        }
        $jobOutput | ForEach-Object { Write-Output $_ }
        Remove-Job -Job $job
    }
}

$completed = [Collections.Generic.List[object]]::new()
foreach ($group in $groups) {
    $firstPlan = @($group.Group)[0]
    $resultPath = Join-Path (Split-Path -Parent $firstPlan.arm_run_root) "group-result.json"
    foreach ($record in @(Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json -Depth 20)) {
        $completed.Add($record)
    }
}
$manifest.status = "completed"
$manifest.completed_at = (Get-Date).ToString("o")
$manifest.completed_run_count = $completed.Count
$manifest.runs = @($completed)
Write-Utf8Json $manifest $manifestPath
Write-Output "R7FiveLayerRunRoot: $RunRoot"
Write-Output "R7FiveLayerManifest: $manifestPath"
