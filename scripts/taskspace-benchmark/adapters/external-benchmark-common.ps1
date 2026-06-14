function New-TaskspaceExternalDir {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
    (Resolve-Path -LiteralPath $Path).Path
}

function Write-TaskspaceExternalJson {
    param(
        [Parameter(Mandatory = $true)]$Value,
        [Parameter(Mandatory = $true)][string]$Path
    )
    ($Value | ConvertTo-Json -Depth 30) | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-TaskspaceExternalFileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-TaskspaceExternalTreeSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    $root = (Resolve-Path -LiteralPath $Path).Path
    $rows = Get-ChildItem -LiteralPath $root -Recurse -File |
        Sort-Object FullName |
        ForEach-Object {
            $relative = $_.FullName.Substring($root.Length).TrimStart("\", "/").Replace("\", "/")
            "$relative=$((Get-TaskspaceExternalFileSha256 $_.FullName))"
        }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($rows -join "`n"))
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
}

function Test-TaskspaceExternalLeakyName {
    param([Parameter(Mandatory = $true)][string]$Name)
    $lower = $Name.ToLowerInvariant()
    foreach ($pattern in @("solution*", "solutions", "answer*", "answers", "gold*", "*.patch", "private*", "hidden*")) {
        if ($lower -like $pattern) { return $true }
    }
    return $false
}

function Copy-TaskspaceExternalFixture {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDir,
        [Parameter(Mandatory = $true)][string]$DestinationDir
    )
    $dest = New-TaskspaceExternalDir $DestinationDir
    $sourceLeaks = @(Get-ChildItem -LiteralPath $SourceDir -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object { Test-TaskspaceExternalLeakyName $_.Name })
    if ($sourceLeaks.Count -gt 0) {
        throw "External fixture source contains solution/gold/private/hidden files: $(@($sourceLeaks | ForEach-Object { $_.FullName }) -join ', ')"
    }
    foreach ($item in Get-ChildItem -LiteralPath $SourceDir -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $dest -Recurse -Force
    }
    $leaks = @(Get-ChildItem -LiteralPath $dest -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object { Test-TaskspaceExternalLeakyName $_.Name })
    if ($leaks.Count -gt 0) {
        throw "External fixture contains solution/gold/private files: $(@($leaks | ForEach-Object { $_.FullName }) -join ', ')"
    }
    $dest
}

function Copy-TaskspaceExternalShellScript {
    param(
        [Parameter(Mandatory = $true)][string]$SourcePath,
        [Parameter(Mandatory = $true)][string]$DestinationPath
    )
    $bytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $SourcePath).Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        $bytes = if ($bytes.Length -eq 3) { [byte[]]@() } else { [byte[]]$bytes[3..($bytes.Length - 1)] }
    }
    $text = [System.Text.Encoding]::UTF8.GetString($bytes).Replace("`r`n", "`n").Replace("`r", "`n")
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $DestinationPath) | Out-Null
    [System.IO.File]::WriteAllText($DestinationPath, $text, [System.Text.UTF8Encoding]::new($false))
}

function New-TaskspaceExternalScenario {
    param(
        [Parameter(Mandatory = $true)][string]$ScenarioDir,
        [Parameter(Mandatory = $true)][string]$ScenarioId,
        [Parameter(Mandatory = $true)][string]$Benchmark,
        [Parameter(Mandatory = $true)][string]$AdapterVersion,
        [Parameter(Mandatory = $true)][string]$InstructionPath,
        [Parameter(Mandatory = $true)][string]$FixtureSourceDir,
        [Parameter(Mandatory = $true)][string]$ValidatorScriptPath,
        [Parameter(Mandatory = $true)][string]$ValidatorSourceDir,
        [Parameter(Mandatory = $true)][string]$OriginalValidatorSha256,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$SourceVersion,
        [Parameter(Mandatory = $true)][string]$SourceUrl,
        [Parameter(Mandatory = $true)][string]$License,
        [Parameter(Mandatory = $true)][string]$DataPolicy,
        [string]$ClaimScope = "",
        $ValidatorFidelity = $null,
        $AdapterMetadata = $null,
        $PromptGuard = $null
    )
    foreach ($required in @($SourceVersion, $SourceUrl, $License, $DataPolicy, $OriginalValidatorSha256)) {
        if ([string]::IsNullOrWhiteSpace($required)) {
            throw "External scenario source/version/license/validator metadata must be non-empty."
        }
    }
    $scenarioRoot = New-TaskspaceExternalDir $ScenarioDir
    $promptPath = Join-Path $scenarioRoot "prompt.txt"
    Copy-Item -LiteralPath $InstructionPath -Destination $promptPath -Force
    $fixtureDir = Copy-TaskspaceExternalFixture $FixtureSourceDir (Join-Path $scenarioRoot "fixture")
    $validatorSourceDest = New-TaskspaceExternalDir (Join-Path $scenarioRoot "external-validator-source")
    foreach ($item in Get-ChildItem -LiteralPath $ValidatorSourceDir -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $validatorSourceDest -Recurse -Force
    }
    $validatorDest = Join-Path $scenarioRoot "external-validator.ps1"
    Copy-Item -LiteralPath $ValidatorScriptPath -Destination $validatorDest -Force
    $promptSha = Get-TaskspaceExternalFileSha256 $promptPath
    $wrapperSha = Get-TaskspaceExternalFileSha256 $validatorDest
    if ($null -eq $ValidatorFidelity) {
        $ValidatorFidelity = [ordered]@{
            official_runner_or_equivalent = $false
            docker_runtime = $false
            container_workdir = ""
            validator_runtime = "unverified"
            agent_cannot_read_validator_source = $false
            e3_eligible = $false
            downgrade_reason = "validator fidelity has not been proven"
        }
    }
    if ($null -eq $AdapterMetadata) { $AdapterMetadata = [ordered]@{} }
    $scenario = [ordered]@{
        id = $ScenarioId
        level = "L3"
        evidence_target = "E3"
        prompt_file = "prompt.txt"
        fixture_dir = "fixture"
        narrative_contract = "external benchmark original instruction preserved"
        mode_delta_contract = "only --taskspace differs"
        oracle = [ordered]@{
            hidden_strategy = "external-validator-v1"
            public_validation = [ordered]@{
                command = "powershell"
                args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $validatorDest)
            }
        }
        expected = [ordered]@{
            max_taskspace_nodes = 200
            max_taskspace_spawn_agent_calls = 50
        }
        thresholds = [ordered]@{
            taskspace_tool_call_ratio_warn = 10
            taskspace_wall_time_ratio_warn = 10
        }
        sample_origin = [ordered]@{
            type = "external_benchmark"
            source = $Benchmark
            source_version = $SourceVersion
            source_url = $SourceUrl
            license = $License
            data_policy = $DataPolicy
            sample_id = $SampleId
            original_prompt_sha256 = $promptSha
            original_validator_sha256 = $OriginalValidatorSha256
            generated_wrapper_sha256 = $wrapperSha
        }
        external_benchmark = [ordered]@{
            name = $Benchmark
            adapter_version = $AdapterVersion
            original_instruction_file = "prompt.txt"
            validator_source_dir = "external-validator-source"
            original_validator_sha256 = $OriginalValidatorSha256
            generated_wrapper_sha256 = $wrapperSha
            validator_command = @("powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $validatorDest)
            validator_fidelity = $ValidatorFidelity
            adapter_metadata = $AdapterMetadata
        }
        prompt_guard = $PromptGuard
        human_review_required = $true
        e3 = [ordered]@{
            minimum_repeats = 5
            manual_review_template = "docs/testing/templates/taskspace-e3-human-review.md"
            claim_scope = $ClaimScope
        }
    }
    Write-TaskspaceExternalJson $scenario (Join-Path $scenarioRoot "scenario.json")
    [pscustomobject]@{
        scenario_dir = $scenarioRoot
        scenario_id = $ScenarioId
        fixture_dir = $fixtureDir
        prompt_sha256 = $promptSha
        original_validator_sha256 = $OriginalValidatorSha256
        generated_wrapper_sha256 = $wrapperSha
    }
}
