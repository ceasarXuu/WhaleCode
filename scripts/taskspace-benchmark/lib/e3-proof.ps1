function Get-TaskspaceFileTextIfPresent {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return "" }
    try {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    } catch {
        ""
    }
}

function Test-TaskspacePathUnderRoot {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )
    if (-not (Test-Path -LiteralPath $Path) -or -not (Test-Path -LiteralPath $Root)) { return $false }
    $resolvedPath = (Resolve-Path -LiteralPath $Path).Path.TrimEnd("\", "/")
    $resolvedRoot = (Resolve-Path -LiteralPath $Root).Path.TrimEnd("\", "/")
    $comparison = [System.StringComparison]::OrdinalIgnoreCase
    $resolvedPath.Equals($resolvedRoot, $comparison) -or $resolvedPath.StartsWith("$resolvedRoot\", $comparison)
}

function Get-TaskspaceBoolField {
    param($Object, [Parameter(Mandatory = $true)][string]$Name)
    if ($null -eq $Object -or -not ($Object.PSObject.Properties.Name -contains $Name)) { return $false }
    [bool]$Object.$Name
}

function Test-TaskspaceProofMarker {
    param([Parameter(Mandatory = $true)][string]$Text, [Parameter(Mandatory = $true)][string]$Pattern)
    $Text -match "(?m)^$Pattern`$"
}

function Get-TaskspaceProofMarkerValue {
    param([Parameter(Mandatory = $true)][string]$Text, [Parameter(Mandatory = $true)][string]$Name)
    $match = [regex]::Match($Text, "(?m)^$([regex]::Escape($Name))=(.+)$")
    if ($match.Success) { return $match.Groups[1].Value.Trim() }
    ""
}

function New-TaskspaceExternalEvidenceProof {
    param(
        [Parameter(Mandatory = $true)]$Pair,
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)]$MetricsBySide
    )
    if ($null -eq $Manifest.ExternalBenchmark) { return $null }

    $external = $Manifest.ExternalBenchmark
    $declared = if ($external.PSObject.Properties.Name -contains "validator_fidelity") { $external.validator_fidelity } else { $null }
    $runtimePath = Join-Path $Pair.PairDir "external-runtime-proof.json"
    $isolationPath = Join-Path $Pair.PairDir "external-isolation-proof.json"
    $combinedPath = Join-Path $Pair.PairDir "external-e3-proof.json"

    $runtimeRows = New-Object System.Collections.Generic.List[object]
    foreach ($sideName in @("left", "right")) {
        $metrics = $MetricsBySide[$sideName]
        $combinedLog = (Get-TaskspaceFileTextIfPresent $metrics.validation_stdout_path) + "`n" + (Get-TaskspaceFileTextIfPresent $metrics.validation_stderr_path)
        $runtimeRows.Add([pscustomobject]@{
            side = $sideName
            logical_mode = [string]$metrics.logical_mode
            validation_exit_code = [int]$metrics.public_validation_exit_code
            proof_nonce = (Get-TaskspaceProofMarkerValue $combinedLog "validator_proof_nonce")
            docker_wrapper_seen = (Test-TaskspaceProofMarker $combinedLog "validator_runtime_probe=terminal_bench_docker_wrapper")
            docker_app_runtime_seen = (Test-TaskspaceProofMarker $combinedLog "validator_runtime=terminal_bench_docker_app")
            container_workdir_app = (Test-TaskspaceProofMarker $combinedLog "container_workdir=/app")
            docker_inspect_seen = (Test-TaskspaceProofMarker $combinedLog "docker_inspect_available=True")
            test_dir_seen = (Test-TaskspaceProofMarker $combinedLog "test_dir=/tbench-validator/tests")
            validator_mount_seen = (Test-TaskspaceProofMarker $combinedLog "validator_mount=/tbench-validator")
            validator_mount_readonly = (Test-TaskspaceProofMarker $combinedLog "validator_mount_readonly=true")
            wrapper_sha = (Get-TaskspaceProofMarkerValue $combinedLog "validator_wrapper_sha256")
            entry_sha = (Get-TaskspaceProofMarkerValue $combinedLog "validator_entry_sha256")
            cleanup_container_exit = (Get-TaskspaceProofMarkerValue $combinedLog "validator_cleanup_container_exit")
            cleanup_image_exit = (Get-TaskspaceProofMarkerValue $combinedLog "validator_cleanup_image_exit")
        })
    }
    $runtimeOk = @($runtimeRows | Where-Object {
            -not ($_.docker_wrapper_seen -and $_.docker_app_runtime_seen -and $_.container_workdir_app -and $_.docker_inspect_seen -and
                $_.proof_nonce -match '^[0-9a-f]{32}$' -and $_.wrapper_sha -match '^[0-9a-f]{64}$' -and $_.entry_sha -match '^[0-9a-f]{64}$')
        }).Count -eq 0
    $mountOk = @($runtimeRows | Where-Object { -not ($_.test_dir_seen -and $_.validator_mount_seen -and $_.validator_mount_readonly) }).Count -eq 0
    $runtimeProof = [pscustomobject]@{
        benchmark = [string]$external.name
        adapter_version = [string]$external.adapter_version
        runtime_proven = $runtimeOk
        validator_mount_proven = $mountOk
        declared_official_runner_or_equivalent = (Get-TaskspaceBoolField $declared "official_runner_or_equivalent")
        declared_e3_eligible = (Get-TaskspaceBoolField $declared "e3_eligible")
        sides = @($runtimeRows.ToArray())
    }
    Write-TaskspaceJson $runtimeProof $runtimePath

    $validatorSourceRel = if ($external.PSObject.Properties.Name -contains "validator_source_dir") { [string]$external.validator_source_dir } else { "" }
    $validatorSourcePath = if ([string]::IsNullOrWhiteSpace($validatorSourceRel)) { "" } else { Join-Path $Manifest.ScenarioRoot $validatorSourceRel }
    $expectedValidatorSha = if ($Manifest.SampleOrigin.PSObject.Properties.Name -contains "original_validator_sha256") {
        [string]$Manifest.SampleOrigin.original_validator_sha256
    } elseif ($external.PSObject.Properties.Name -contains "original_validator_sha256") {
        [string]$external.original_validator_sha256
    } else {
        ""
    }
    $validatorSourceExists = (-not [string]::IsNullOrWhiteSpace($validatorSourcePath) -and (Test-Path -LiteralPath $validatorSourcePath))
    $validatorSourceSha = if ($validatorSourceExists) { Get-TaskspaceDirectorySha256 $validatorSourcePath } else { "" }
    $validatorSourceHashMatches = (-not [string]::IsNullOrWhiteSpace($expectedValidatorSha) -and $validatorSourceSha -eq $expectedValidatorSha)
    $isolationRows = New-Object System.Collections.Generic.List[object]
    foreach ($sideName in @("left", "right")) {
        $side = $Pair.$sideName
        $metrics = $MetricsBySide[$sideName]
        $agentText = (Get-TaskspaceFileTextIfPresent $metrics.jsonl_path) + "`n" +
            (Get-TaskspaceFileTextIfPresent $metrics.stderr_path) + "`n" +
            (Get-TaskspaceFileTextIfPresent $metrics.last_message_path)
        $sourceUnderRepo = if ($validatorSourcePath) { Test-TaskspacePathUnderRoot $validatorSourcePath $side.RepoDir } else { $true }
        $repoHits = @(Get-ChildItem -LiteralPath $side.RepoDir -Recurse -Force -ErrorAction SilentlyContinue |
            Where-Object { [string]$_.FullName -match "external-validator-source|tbench-validator" })
        $artifactHits = New-Object System.Collections.Generic.List[string]
        foreach ($token in @($validatorSourcePath, "external-validator-source", "/tbench-validator", "\tbench-validator")) {
            if (-not [string]::IsNullOrWhiteSpace($token) -and $agentText.Contains($token)) { $artifactHits.Add($token) }
        }
        $isolationRows.Add([pscustomobject]@{
            side = $sideName
            logical_mode = [string]$metrics.logical_mode
            validator_source_under_agent_repo = $sourceUnderRepo
            repo_validator_hits = @($repoHits | ForEach-Object { $_.FullName })
            agent_artifact_validator_tokens = @($artifactHits.ToArray())
            proven = ($validatorSourceExists -and $validatorSourceHashMatches -and -not $sourceUnderRepo -and $repoHits.Count -eq 0 -and $artifactHits.Count -eq 0)
        })
    }
    $isolationOk = @($isolationRows | Where-Object { -not $_.proven }).Count -eq 0
    $isolationProof = [pscustomobject]@{
        validator_source_dir = $validatorSourceRel
        validator_source_path = $validatorSourcePath
        validator_source_exists = $validatorSourceExists
        validator_source_sha256 = $validatorSourceSha
        expected_validator_source_sha256 = $expectedValidatorSha
        validator_source_hash_matches = $validatorSourceHashMatches
        validator_source_outside_repo_proven = $isolationOk
        agent_cannot_read_validator_source_proven = ($isolationOk -and (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source"))
        declared_agent_cannot_read_validator_source = (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source")
        sides = @($isolationRows.ToArray())
    }
    Write-TaskspaceJson $isolationProof $isolationPath

    $officialEquivalent = $runtimeOk -and $mountOk -and (Get-TaskspaceBoolField $declared "official_runner_or_equivalent")
    $sourceIsolated = $isolationOk -and (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source")
    $eligible = $officialEquivalent -and $sourceIsolated -and (Get-TaskspaceBoolField $declared "e3_eligible")
    $combinedProof = [pscustomobject]@{
        runtime_proof_path = $runtimePath
        isolation_proof_path = $isolationPath
        combined_proof_path = $combinedPath
        validator_fidelity = [pscustomobject]@{
            official_runner_or_equivalent = $officialEquivalent
            agent_cannot_read_validator_source = $sourceIsolated
            e3_eligible = $eligible
            runtime_proven = $runtimeOk
            validator_mount_proven = $mountOk
        }
    }
    Write-TaskspaceJson $combinedProof $combinedPath
    $combinedProof
}
