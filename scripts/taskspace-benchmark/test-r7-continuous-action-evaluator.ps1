$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Code)
    if (-not $Condition) { throw $Code }
}

function New-Art {
    param([string]$Dir, [string]$Name, $Body)
    $path = "$Name.json"
    Write-R7JsonFile (Join-Path $Dir $path) $Body
    [pscustomobject]@{path = $path; sha256 = Get-R7Sha256File (Join-Path $Dir $path)}
}

function New-Run {
    param([string]$Dir, [string]$Arm, [string]$Sample, [int]$Repeat, [string]$WireHash)
    $prefix = "$Arm-$Sample-$Repeat"
    $carrier = "$prefix-carrier"
    [pscustomobject][ordered]@{
        run_id = $prefix; arm = $Arm; sample = $Sample; repeat = $Repeat
        attempt = 1; status = "complete"; held_out = $false; wire_sha256 = $WireHash
        summary = [pscustomobject]@{correct = $false; carrier_conserved = $false; evaluator_must_ignore = $true}
        artifacts = [pscustomobject][ordered]@{
            metrics = New-Art $Dir "$prefix-metrics" ([pscustomobject]@{started_at_ms = 0; finished_at_ms = 100})
            requests = New-Art $Dir "$prefix-requests" ([pscustomobject]@{events = @([pscustomobject]@{request_id = "$prefix-request"; input_tokens = 1000; output_tokens = 100; duration_ms = 80})})
            tools = New-Art $Dir "$prefix-tools" ([pscustomobject]@{events = @(
                [pscustomobject]@{event_kind = "handler_handoff_started"; carrier_id = $carrier},
                [pscustomobject]@{event_kind = "carrier_outcome"; carrier_id = $carrier},
                [pscustomobject]@{event_kind = "patch_input"; byte_exact = $true},
                [pscustomobject]@{event_kind = "carrier_output"; schema_exact = $true; payload_exact = $true},
                [pscustomobject]@{event_kind = "tool_finished"; duration_ms = 10}
            )})
            map = New-Art $Dir "$prefix-map" ([pscustomobject]@{events = @([pscustomobject]@{event_kind = "transition_accepted"; terminal = $false; reserved_tool_call_id = $carrier})})
            verdict = New-Art $Dir "$prefix-verdict" ([pscustomobject]@{correct = $true; error_codes = @()})
            cache = New-Art $Dir "$prefix-cache" ([pscustomobject]@{epochs = @([pscustomobject]@{status = "available"; input_tokens = 1000; cached_input_tokens = 500})})
        }
    }
}

function Copy-RunSet {
    param($Value)
    $Value | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100
}

function Invoke-Evaluation {
    param([string]$RunSetPath, [string]$OutputPath, [string]$Root)
    & pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "evaluate-r7-continuous-action-runset.ps1") -RunSetPath $RunSetPath -RunArtifactRoot $Root -OutputPath $OutputPath
    if ($LASTEXITCODE -ne 0) { throw "R7_EVALUATOR_CHILD_FAILED path=$RunSetPath" }
    Read-R7StrictJson $OutputPath (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-result-v1.schema.json")
}

$scratch = Join-Path $script:R7RepoRoot ("target/r7-continuous-action-evaluator-test/" + [DateTime]::UtcNow.ToString("yyyyMMddHHmmssfff"))
[System.IO.Directory]::CreateDirectory($scratch) | Out-Null
$contractPath = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"
$contract = Read-R7StrictJson $contractPath (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.schema.json")
$standardWire = Get-R7Sha256Text "standard-wire"
$runs = [System.Collections.Generic.List[object]]::new()
foreach ($sample in @($contract.sample_order)) {
    for ($repeat = 1; $repeat -le [int]$contract.samples.$sample.repeats; $repeat++) {
        foreach ($arm in @($contract.arm_order)) {
            $wire = if ($arm -eq "standard") { $standardWire } else { Get-R7Sha256Text "$arm-wire" }
            $runs.Add((New-Run $scratch $arm $sample $repeat $wire))
        }
    }
}
$fixtures = foreach ($sample in @($contract.sample_order)) {
    $source = $contract.samples.$sample
    [pscustomobject][ordered]@{
        sample = $sample; held_out = $false; fixture_path = $source.fixture_path; fixture_sha256 = $source.fixture_sha256
        directory_root = $source.directory_root; directory_manifest_sha256 = $source.directory_manifest_sha256
        prompt_path = $source.prompt_path; prompt_sha256 = $source.prompt_sha256
        fixture_root = $source.fixture_root; fixture_manifest_sha256 = $source.fixture_manifest_sha256
        oracle_path = $source.oracle_path; oracle_sha256 = $source.oracle_sha256
        source_set = $source.source_set; identity_mode = $source.identity_mode
    }
}
$runSet = [pscustomobject][ordered]@{
    schema_version = 1; artifact_role = "continuous_action_raw_run_set"; sealed = $true
    evaluation_id = "r7-fla3-5-continuous-action-v1"
    evaluation_contract = [pscustomobject]@{path = "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"; sha256 = Get-R7Sha256File $contractPath; contract_digest = $contract.contract_digest}
    identity = [pscustomobject]@{standard_commit = "1" * 40; sibling_baseline_commit = "2" * 40; candidate_commit = "3" * 40; docker_image_digest = "sha256:" + ("4" * 64); model = "deepseek-v4"; provider_endpoint_family = "deepseek"; tool_profile = "production"; standard_wire_sha256 = $standardWire}
    arm_order = @($contract.arm_order); sample_order = @($contract.sample_order); fixtures = @($fixtures); runs = $runs.ToArray()
}

$runSetPath = Join-Path $scratch "runset.json"
$resultPath = Join-Path $scratch "result.json"
Write-R7JsonFile $runSetPath $runSet
$result = Invoke-Evaluation $runSetPath $resultPath $scratch
Assert-True ([string]$result.decision -ceq "pass") "R7_EVALUATOR_VALID_RUNSET_FAILED"
Assert-True ([int]$result.run_count -eq 36 -and [int]$result.pair_count -eq 12) "R7_EVALUATOR_MATRIX_COUNT"
Assert-True ([double]$result.metrics_by_arm.fla3_5_candidate.correctness_rate -eq 1) "R7_EVALUATOR_IGNORED_SUMMARY_FAILED"

$duplicate = Copy-RunSet $runSet
$duplicate.runs = @($duplicate.runs) + @($duplicate.runs[0])
$duplicatePath = Join-Path $scratch "runset-duplicate.json"
Write-R7JsonFile $duplicatePath $duplicate
$duplicateResult = Invoke-Evaluation $duplicatePath (Join-Path $scratch "result-duplicate.json") $scratch
Assert-True ($duplicateResult.codes -contains "R7_DUPLICATE_ARM_SAMPLE_REPEAT") "R7_EVALUATOR_DUPLICATE_ACCEPTED"

$missing = Copy-RunSet $runSet
$missing.runs = @($missing.runs | Select-Object -Skip 1)
$missingPath = Join-Path $scratch "runset-missing.json"
Write-R7JsonFile $missingPath $missing
$missingResult = Invoke-Evaluation $missingPath (Join-Path $scratch "result-missing.json") $scratch
Assert-True ($missingResult.codes -contains "R7_MISSING_ARM_SAMPLE_REPEAT") "R7_EVALUATOR_MISSING_ACCEPTED"

$drift = Copy-RunSet $runSet
$drift.runs[2].artifacts.requests.sha256 = Get-R7Sha256Text "wrong-artifact"
$driftPath = Join-Path $scratch "runset-drift.json"
Write-R7JsonFile $driftPath $drift
$driftResult = Invoke-Evaluation $driftPath (Join-Path $scratch "result-drift.json") $scratch
Assert-True ($driftResult.codes -contains "R7_HASH_DRIFT") "R7_EVALUATOR_HASH_DRIFT_ACCEPTED"

$cache = Copy-RunSet $runSet
$cacheRef = New-Art $scratch "unavailable-cache" ([pscustomobject]@{epochs = @([pscustomobject]@{status = "unavailable"; input_tokens = 1000; cached_input_tokens = 0})})
$cache.runs[2].artifacts.cache = $cacheRef
$cachePath = Join-Path $scratch "runset-cache-unavailable.json"
Write-R7JsonFile $cachePath $cache
$cacheResult = Invoke-Evaluation $cachePath (Join-Path $scratch "result-cache-unavailable.json") $scratch
Assert-True ($cacheResult.codes -contains "R7_CACHE_UNAVAILABLE") "R7_EVALUATOR_CACHE_UNAVAILABLE_ACCEPTED"

$heldOut = Copy-RunSet $runSet
$heldOut.runs[0].held_out = $true
$heldOutPath = Join-Path $scratch "runset-heldout.json"
Write-R7JsonFile $heldOutPath $heldOut
& pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "evaluate-r7-continuous-action-runset.ps1") -RunSetPath $heldOutPath -RunArtifactRoot $scratch -OutputPath (Join-Path $scratch "result-heldout.json") 2>$null
Assert-True ($LASTEXITCODE -ne 0) "R7_EVALUATOR_HELDOUT_ACCEPTED"

Write-Output ([pscustomobject]@{schema_version = 1; test = "r7_continuous_action_evaluator"; passed = $true; runs = $runs.Count; negative_cases = 5} | ConvertTo-Json -Compress)
