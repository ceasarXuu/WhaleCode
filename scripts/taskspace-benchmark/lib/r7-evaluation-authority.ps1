if (-not (Get-Command Read-TaskspaceScenarioManifest -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "scenario-manifest.ps1")
}

function Get-R7EvaluationProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

function Test-R7ExactSequence {
    param([object[]]$Expected, [object[]]$Actual)
    if ($Expected.Count -ne $Actual.Count) { return $false }
    for ($index = 0; $index -lt $Expected.Count; $index++) {
        if ([string]$Expected[$index] -cne [string]$Actual[$index]) { return $false }
    }
    $true
}

function Test-R7TrackedFileMatchesCommit {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Commit,
        [Parameter(Mandatory = $true)][string]$Path
    )
    if ($Commit -notmatch '^[0-9a-fA-F]{40,64}$' -or
        -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }
    $repoPath = [IO.Path]::GetFullPath($RepoRoot).TrimEnd("\", "/")
    $filePath = [IO.Path]::GetFullPath($Path)
    $prefix = $repoPath + [IO.Path]::DirectorySeparatorChar
    if (-not $filePath.StartsWith($prefix, [StringComparison]::Ordinal)) {
        return $false
    }
    $relative = $filePath.Substring($prefix.Length).Replace("\", "/")
    $workingBlob = @(
        & git -C $repoPath hash-object --no-filters -- $relative 2>$null
    )
    if ($LASTEXITCODE -ne 0 -or $workingBlob.Count -ne 1) { return $false }
    $committedBlob = @(
        & git -C $repoPath rev-parse "${Commit}:$relative" 2>$null
    )
    if ($LASTEXITCODE -ne 0 -or $committedBlob.Count -ne 1) { return $false }
    [string]$workingBlob[0] -ceq [string]$committedBlob[0]
}

function Get-R7CanonicalScenarioDirectorySha256 {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$ScenarioRoot
    )
    $repoPath = [IO.Path]::GetFullPath($RepoRoot).TrimEnd("\", "/")
    $scenarioPath = [IO.Path]::GetFullPath($ScenarioRoot)
    $paths = [string[]]@(
        Get-ChildItem -LiteralPath $scenarioPath -Recurse -File |
            ForEach-Object {
                $_.FullName.Substring($repoPath.Length).TrimStart("\", "/").Replace("\", "/")
            }
    )
    [Array]::Sort($paths, [StringComparer]::Ordinal)
    $rows = foreach ($relativePath in $paths) {
        $fullPath = Join-Path $repoPath $relativePath
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fullPath).Hash.ToLowerInvariant()
        "$hash  $relativePath"
    }
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes((($rows -join "`n") + "`n"))
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        ([BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-R7EvaluationAuthority {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [ValidateSet("initial", "extended")][string]$Stage
    )
    $contractRelative = "benchmarks/taskspace/r7/five-layer-evaluation-contract-v1.json"
    $authorityRelative = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
    $contractPath = Join-Path $RepoRoot $contractRelative
    $authorityPath = Join-Path $RepoRoot $authorityRelative
    $contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath |
        ConvertFrom-Json -Depth 100
    $productionAuthority = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath |
        ConvertFrom-Json -Depth 100
    $contractHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $contractPath).Hash.ToLowerInvariant()
    $targets = @(
        Get-R7EvaluationProperty $productionAuthority "selected_targets" @() |
            Where-Object {
                [string]$_.layer -eq "evaluation" -and
                [string]$_.artifact -eq $contractRelative
            }
    )
    if ($targets.Count -ne 1 -or
        ([string]$targets[0].sha256).ToLowerInvariant() -ne $contractHash) {
        throw "Canonical evaluation contract is not anchored by production authority"
    }
    if ([string]$contract.contract_id -ne "r7-five-layer-evaluation-contract-v1" -or
        [string]$contract.status -ne "active_initial_evaluation_authority") {
        throw "Canonical evaluation contract identity is not active"
    }

    $scenarioById = @{}
    foreach ($frozen in @($contract.samples.frozen_artifacts)) {
        $sampleId = [string]$frozen.id
        $expectedPath = "benchmarks/taskspace/scenarios/$sampleId"
        if ([string]::IsNullOrWhiteSpace($sampleId) -or
            $scenarioById.ContainsKey($sampleId) -or
            [string]$frozen.path -ne $expectedPath) {
            throw "Canonical evaluation contract has an invalid frozen scenario identity"
        }
        $scenarioRoot = Join-Path $RepoRoot $expectedPath
        $scenarioJsonPath = Join-Path $scenarioRoot "scenario.json"
        $scenarioJsonHash = (
            Get-FileHash -Algorithm SHA256 -LiteralPath $scenarioJsonPath
        ).Hash.ToLowerInvariant()
        $directoryHash = Get-R7CanonicalScenarioDirectorySha256 $RepoRoot $scenarioRoot
        if ($scenarioJsonHash -ne ([string]$frozen.scenario_json_sha256).ToLowerInvariant() -or
            $directoryHash -ne ([string]$frozen.directory_manifest_sha256).ToLowerInvariant()) {
            throw "Frozen scenario bytes do not match evaluation authority: $sampleId"
        }
        $scenario = Read-TaskspaceScenarioManifest -RepoRoot $RepoRoot -Scenario $sampleId
        $scenarioById[$sampleId] = [pscustomobject]@{
            id = $sampleId
            path = $expectedPath
            scenario_json_sha256 = $scenarioJsonHash
            directory_manifest_sha256 = $directoryHash
            prompt_sha256 = Get-TaskspaceFileSha256 $scenario.PromptPath
            fixture_sha256 = Get-TaskspaceDirectorySha256 $scenario.FixtureDir
        }
    }

    $stageContract = if ($Stage -eq "initial") {
        $contract.run_design.initial_observation
    } else {
        $contract.run_design.extended_observation
    }
    $samples = if ($Stage -eq "initial") {
        @($contract.samples.development_smoke | ForEach-Object { [string]$_ })
    } else {
        @(
            $contract.samples.development_smoke + $contract.samples.held_out_formal |
                ForEach-Object { [string]$_ }
        )
    }
    foreach ($sample in $samples) {
        if (-not $scenarioById.ContainsKey($sample)) {
            throw "Evaluation stage references an unfrozen scenario: $sample"
        }
    }
    $environment = $contract.environment
    $profiles = Get-R7EvaluationProperty $environment "tool_capability_profiles"
    foreach ($mode in @("standard", "taskspace")) {
        $profile = Get-R7EvaluationProperty $profiles $mode
        if ([string](Get-R7EvaluationProperty $profile "tools_hash" "") -notmatch
                '^[a-fA-F0-9]{64}$' -or
            [int64](Get-R7EvaluationProperty $profile "tools_count" 0) -lt 1) {
            throw "Evaluation authority has an invalid $mode tool capability profile"
        }
    }
    if ([string]$environment.docker_image_digest -notmatch '^sha256:[a-fA-F0-9]{64}$' -or
        [string]::IsNullOrWhiteSpace([string]$environment.model_identifier) -or
        [string]::IsNullOrWhiteSpace([string]$environment.model_reasoning_effort) -or
        [string]::IsNullOrWhiteSpace([string]$environment.sandbox_mode) -or
        [string]::IsNullOrWhiteSpace([string]$environment.provider_wire_api) -or
        [string]::IsNullOrWhiteSpace([string]$environment.provider_transport)) {
        throw "Evaluation authority has an incomplete execution identity"
    }
    [pscustomobject]@{
        contract = $contract
        contract_path = [IO.Path]::GetFullPath($contractPath)
        contract_relative_path = $contractRelative
        contract_sha256 = $contractHash
        production_authority_path = [IO.Path]::GetFullPath($authorityPath)
        production_authority_sha256 = (
            Get-FileHash -Algorithm SHA256 -LiteralPath $authorityPath
        ).Hash.ToLowerInvariant()
        stage = $Stage
        samples = $samples
        repeats = [int]$stageContract.repeats_per_arm_per_sample
        arms = @($contract.arms.post_refactor_four_arm | ForEach-Object { [string]$_ })
        scenarios = $scenarioById
        execution = "docker"
        model = [string]$environment.model_identifier
        reasoning_effort = [string]$environment.model_reasoning_effort
        sandbox_mode = [string]$environment.sandbox_mode
        container_image_digest = [string]$environment.docker_image_digest
        provider_wire_api = [string]$environment.provider_wire_api
        provider_transport = [string]$environment.provider_transport
        tool_capability_profiles = $profiles
    }
}

function Get-R7MatrixEvaluationAuthorityCheck {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$ManifestPath,
        [Parameter(Mandatory = $true)]$Manifest
    )
    $findings = [Collections.Generic.List[string]]::new()
    $authority = try {
        Get-R7EvaluationAuthority $RepoRoot ([string]$Manifest.stage)
    } catch {
        $findings.Add("evaluation_authority_invalid")
        $null
    }
    if ($null -eq $authority) {
        return [pscustomobject]@{ status = "invalid"; authority = $null; findings = @($findings) }
    }
    $manifestContractPath = [string](Get-R7EvaluationProperty $Manifest "evaluation_contract_path" "")
    $manifestContractHash = (
        [string](Get-R7EvaluationProperty $Manifest "evaluation_contract_sha256" "")
    ).ToLowerInvariant()
    if ($manifestContractPath -ne $authority.contract_relative_path -or
        $manifestContractHash -ne $authority.contract_sha256 -or
        [string]$Manifest.contract_id -ne [string]$authority.contract.contract_id) {
        $findings.Add("matrix_evaluation_contract_identity_mismatch")
    }
    $repoCommit = [string](Get-R7EvaluationProperty $Manifest "repo_commit" "")
    if (-not (Test-R7TrackedFileMatchesCommit `
            $RepoRoot `
            $repoCommit `
            $authority.contract_path)) {
        $findings.Add("matrix_evaluation_contract_not_committed")
    }
    if (-not (Test-R7TrackedFileMatchesCommit `
            $RepoRoot `
            $repoCommit `
            $authority.production_authority_path)) {
        $findings.Add("matrix_production_authority_not_committed")
    }
    if (-not (Test-R7ExactSequence @($authority.samples) @($Manifest.samples)) -or
        -not (Test-R7ExactSequence @($authority.arms) @($Manifest.arms)) -or
        [int]$Manifest.repeats_per_arm_per_sample -ne $authority.repeats) {
        $findings.Add("matrix_evaluation_design_mismatch")
    }
    if ([string]$Manifest.execution -ne $authority.execution -or
        [string]$Manifest.model -ne $authority.model -or
        [string](Get-R7EvaluationProperty $Manifest "model_reasoning_effort" "") -ne
            $authority.reasoning_effort -or
        [string](Get-R7EvaluationProperty $Manifest "sandbox_mode" "") -ne
            $authority.sandbox_mode -or
        [string](Get-R7EvaluationProperty $Manifest "expected_container_image_digest" "") -ne
            $authority.container_image_digest -or
        [string](Get-R7EvaluationProperty $Manifest "provider_wire_api" "") -ne
            $authority.provider_wire_api -or
        [string](Get-R7EvaluationProperty $Manifest "provider_transport" "") -ne
            $authority.provider_transport) {
        $findings.Add("matrix_evaluation_environment_mismatch")
    }
    $expectedKeys = [Collections.Generic.List[string]]::new()
    foreach ($sample in $authority.samples) {
        foreach ($repeat in 1..$authority.repeats) {
            foreach ($arm in $authority.arms) {
                $expectedKeys.Add("$sample|$repeat|$arm")
            }
        }
    }
    $actualKeys = [Collections.Generic.List[string]]::new()
    foreach ($run in @(Get-R7EvaluationProperty $Manifest "runs" @())) {
        $sample = [string]$run.sample
        $repeat = [int]$run.repeat
        $arm = [string]$run.arm
        $mode = if ($arm -eq "standard") { "standard" } else { "taskspace" }
        $side = if ($mode -eq "standard") { "left" } else { "right" }
        $projection = if ($mode -eq "standard") { "map-request" } else { $arm }
        $actualKeys.Add("$sample|$repeat|$arm")
        if ([string]$run.logical_mode -ne $mode -or
            [string]$run.run_side -ne $side -or
            [string]$run.projection_policy -ne $projection) {
            $findings.Add("matrix_run_mode_identity_mismatch")
        }
    }
    $expected = [string[]]@($expectedKeys)
    $actual = [string[]]@($actualKeys)
    [Array]::Sort($expected, [StringComparer]::Ordinal)
    [Array]::Sort($actual, [StringComparer]::Ordinal)
    if (-not (Test-R7ExactSequence $expected $actual)) {
        $findings.Add("matrix_run_set_mismatch")
    }
    [pscustomobject]@{
        status = if ($findings.Count) { "invalid" } else { "valid" }
        authority = $authority
        findings = @($findings | Sort-Object -Unique)
        manifest_path = $ManifestPath
    }
}

function Get-R7ProviderWireCapabilityIdentity {
    param([Parameter(Mandatory = $true)][string]$WireTracePath)
    $shapeRows = @(
        foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $WireTracePath) {
            if ([string]::IsNullOrWhiteSpace($line)) { continue }
            $event = $line | ConvertFrom-Json -Depth 100
            if ($event.PSObject.Properties.Name -contains "request_index") { $event }
        }
    )
    if (-not $shapeRows.Count) { throw "Provider wire trace has no request-shape rows" }
    $fields = @(
        "schema_version", "provider_wire_api", "transport", "tools_hash", "tools_count"
    )
    $identity = [ordered]@{}
    foreach ($field in $fields) {
        $values = @(
            $shapeRows |
                ForEach-Object { Get-R7EvaluationProperty $_ $field } |
                Sort-Object -Unique
        )
        if ($values.Count -ne 1) {
            throw "Provider wire capability is unstable or missing: $field"
        }
        $identity[$field] = $values[0]
    }
    if ([string]$identity.schema_version -ne "provider-chat-wire-trace-v11" -or
        [string]$identity.tools_hash -notmatch '^[a-fA-F0-9]{64}$' -or
        $identity.tools_count -isnot [int64] -or [int64]$identity.tools_count -lt 1) {
        throw "Provider wire capability identity is malformed"
    }
    [pscustomobject]$identity
}
