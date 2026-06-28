function Get-TaskspaceFileTextIfPresent {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return "" }
    try {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    } catch {
        ""
    }
}

function Read-TaskspaceJsonIfPresent {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return $null }
    try { Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json } catch { $null }
}

function Get-TaskspaceSha256IfPresent {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return "" }
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Invoke-TaskspaceGitScalar {
    param([string]$RepoRoot, [string[]]$Arguments)
    if ([string]::IsNullOrWhiteSpace($RepoRoot) -or -not (Test-Path -LiteralPath $RepoRoot) -or -not (Get-Command git -ErrorAction SilentlyContinue)) { return "" }
    try {
        $output = & git -C $RepoRoot @Arguments 2>$null
        if ($LASTEXITCODE -ne 0) { return "" }
        ([string]$output).Trim()
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
    $Text -match "(?m)^$Pattern`r?`$"
}

function Get-TaskspaceProofMarkerValue {
    param([Parameter(Mandatory = $true)][string]$Text, [Parameter(Mandatory = $true)][string]$Name)
    $match = [regex]::Match($Text, "(?m)^$([regex]::Escape($Name))=(.+)$")
    if ($match.Success) { return $match.Groups[1].Value.Trim() }
    ""
}

function Find-TaskspaceValidatorSourceHitsInRepo {
    param([Parameter(Mandatory = $true)][string]$RepoDir)
    if ([string]::IsNullOrWhiteSpace($RepoDir) -or -not (Test-Path -LiteralPath $RepoDir)) { return @() }
    $skipDirs = @(".git", ".tbench-testing", ".venv", "venv", "node_modules", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache")
    $hits = New-Object System.Collections.Generic.List[string]
    $stack = New-Object System.Collections.Generic.Stack[System.IO.DirectoryInfo]
    $stack.Push([System.IO.DirectoryInfo]::new((Resolve-Path -LiteralPath $RepoDir).Path))
    $visited = 0
    $maxEntries = 50000
    while ($stack.Count -gt 0) {
        $dir = $stack.Pop()
        if ($skipDirs -contains $dir.Name) { continue }
        $visited++
        if ($visited -gt $maxEntries) {
            $hits.Add("__scan_truncated_after_$maxEntries`:$($dir.FullName)")
            break
        }
        try {
            foreach ($entry in $dir.EnumerateFileSystemInfos()) {
                if ($entry.Name -match "external-validator-source|tbench-validator") {
                    $hits.Add($entry.FullName)
                }
                if ($entry -is [System.IO.DirectoryInfo]) {
                    if ($skipDirs -notcontains $entry.Name) { $stack.Push($entry) }
                }
            }
        } catch {
            $hits.Add("__scan_error:$($dir.FullName):$($_.Exception.GetType().Name)")
        }
    }
    @($hits.ToArray())
}

function Get-TaskspaceExpectedWrapperSha {
    param($Manifest, $External)
    if ($Manifest.SampleOrigin.PSObject.Properties.Name -contains "generated_wrapper_sha256") { return [string]$Manifest.SampleOrigin.generated_wrapper_sha256 }
    if ($External.PSObject.Properties.Name -contains "generated_wrapper_sha256") { return [string]$External.generated_wrapper_sha256 }
    ""
}

function New-TaskspaceExternalEvidenceProof {
    param(
        [Parameter(Mandatory = $true)]$Pair,
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)]$MetricsBySide,
        $SourceGuard = $null
    )
    if ($null -eq $Manifest.ExternalBenchmark) { return $null }

    $external = $Manifest.ExternalBenchmark
    $declared = if ($external.PSObject.Properties.Name -contains "validator_fidelity") { $external.validator_fidelity } else { $null }
    $runtimePath = Join-Path $Pair.PairDir "external-runtime-proof.json"
    $runnerPath = Join-Path $Pair.PairDir "external-runner-equivalence-proof.json"
    $isolationPath = Join-Path $Pair.PairDir "external-isolation-proof.json"
    $combinedPath = Join-Path $Pair.PairDir "external-e3-proof.json"
    $expectedWrapperSha = Get-TaskspaceExpectedWrapperSha $Manifest $external

    $runtimeRows = New-Object System.Collections.Generic.List[object]
    foreach ($sideName in @("left", "right")) {
        $side = $Pair.$sideName
        $metrics = $MetricsBySide[$sideName]
        $combinedLog = (Get-TaskspaceFileTextIfPresent $metrics.validation_stdout_path) + "`n" + (Get-TaskspaceFileTextIfPresent $metrics.validation_stderr_path)
        $runtimeManifestPath = Get-TaskspaceProofMarkerValue $combinedLog "validator_runtime_manifest_path"
        $inspectPath = Get-TaskspaceProofMarkerValue $combinedLog "docker_inspect_path"
        $cleanupResultPath = Get-TaskspaceProofMarkerValue $combinedLog "validation_cleanup_result_path"
        $runtimeManifest = Read-TaskspaceJsonIfPresent $runtimeManifestPath
        $inspect = Read-TaskspaceJsonIfPresent $inspectPath
        $cleanupResult = Read-TaskspaceJsonIfPresent $cleanupResultPath
        $artifactRoot = if ($side.ArtifactDir -and (Test-Path -LiteralPath $side.ArtifactDir)) { (Resolve-Path -LiteralPath $side.ArtifactDir).Path } else { "" }
        $runtimeManifestUnderArtifact = (-not [string]::IsNullOrWhiteSpace($artifactRoot) -and
            -not [string]::IsNullOrWhiteSpace($runtimeManifestPath) -and
            (Test-TaskspacePathUnderRoot $runtimeManifestPath $artifactRoot))
        $inspectUnderArtifact = (-not [string]::IsNullOrWhiteSpace($artifactRoot) -and
            -not [string]::IsNullOrWhiteSpace($inspectPath) -and
            (Test-TaskspacePathUnderRoot $inspectPath $artifactRoot))
        $cleanupResultUnderArtifact = (-not [string]::IsNullOrWhiteSpace($artifactRoot) -and
            -not [string]::IsNullOrWhiteSpace($cleanupResultPath) -and
            (Test-TaskspacePathUnderRoot $cleanupResultPath $artifactRoot))
        $manifestWrapperSha = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "wrapper_sha256") { [string]$runtimeManifest.wrapper_sha256 } else { "" }
        $manifestWrapperPath = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "wrapper_path") { [string]$runtimeManifest.wrapper_path } else { "" }
        $actualWrapperSha = Get-TaskspaceSha256IfPresent $manifestWrapperPath
        $manifestEntrySha = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "entry_sha256") { [string]$runtimeManifest.entry_sha256 } else { "" }
        $manifestEntryPath = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "entry_script_path") { [string]$runtimeManifest.entry_script_path } else { "" }
        $actualEntrySha = Get-TaskspaceSha256IfPresent $manifestEntryPath
        $container = if ($inspect -is [array] -and @($inspect).Count -gt 0) { @($inspect)[0] } else { $inspect }
        $mounts = if ($container -and $container.PSObject.Properties.Name -contains "Mounts") { @($container.Mounts) } else { @() }
        $testsMount = @($mounts | Where-Object { [string]$_.Destination -eq "/tests" } | Select-Object -First 1)
        $appMount = @($mounts | Where-Object { [string]$_.Destination -eq "/app" } | Select-Object -First 1)
        $entryMount = @($mounts | Where-Object { [string]$_.Destination -eq "/tbench-entry.sh" } | Select-Object -First 1)
        $uvCacheMount = @($mounts | Where-Object { [string]$_.Destination -eq "/tbench-uv-cache" } | Select-Object -First 1)
        $workdir = if ($container -and $container.PSObject.Properties.Name -contains "Config" -and $container.Config.PSObject.Properties.Name -contains "WorkingDir") { [string]$container.Config.WorkingDir } else { "" }
        $runtimeCommand = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "validator_command") { [string]$runtimeManifest.validator_command } else { "" }
        $uvInstallerSha = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "uv_installer_sha256") { [string]$runtimeManifest.uv_installer_sha256 } else { "" }
        $uvArchiveSha = if ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "uv_archive_sha256") { [string]$runtimeManifest.uv_archive_sha256 } else { "" }
        $uvCacheDeclared = ($runtimeManifest -and $runtimeManifest.PSObject.Properties.Name -contains "uv_cache_mount")
        $uvCacheSourceMatches = (-not $uvCacheDeclared -or ($uvCacheMount.Count -gt 0 -and
                [string]$uvCacheMount[0].Source -eq [string]$runtimeManifest.uv_cache_mount))
        $uvCacheProven = (-not $uvCacheDeclared -or ($uvCacheMount.Count -gt 0 -and -not [bool]$uvCacheMount[0].RW -and
                $uvCacheSourceMatches -and
                $uvInstallerSha -match '^[0-9a-f]{64}$' -and $uvArchiveSha -match '^[0-9a-f]{64}$'))
        $proofNonce = Get-TaskspaceProofMarkerValue $combinedLog "validator_proof_nonce"
        $wrapperSha = Get-TaskspaceProofMarkerValue $combinedLog "validator_wrapper_sha256"
        $entrySha = Get-TaskspaceProofMarkerValue $combinedLog "validator_entry_sha256"
        $validationTimedOutAfterTestsStarted = (
            $metrics -and
            $metrics.PSObject.Properties.Name -contains "public_validation_exit_code" -and
            [int]$metrics.public_validation_exit_code -eq 124 -and
            $metrics.PSObject.Properties.Name -contains "tests_started_seen" -and
            [bool]$metrics.tests_started_seen
        )
        $validationSkippedAfterAgentTimeout = (
            $metrics -and
            $metrics.PSObject.Properties.Name -contains "public_validation_skipped" -and
            [bool]$metrics.public_validation_skipped -and
            $metrics.PSObject.Properties.Name -contains "public_validation_skip_reason" -and
            [string]$metrics.public_validation_skip_reason -eq "agent_exec_timeout" -and
            $metrics.PSObject.Properties.Name -contains "exec_timed_out" -and
            [bool]$metrics.exec_timed_out
        )
        $preAgentProbeProven = (
            $validationSkippedAfterAgentTimeout -and
            $metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_status" -and
            [string]$metrics.pre_agent_validator_probe_status -eq "passed" -and
            $metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_hash" -and
            [string]$metrics.pre_agent_validator_probe_hash -match '^[0-9a-f]{64}$'
        )
        $timeoutRuntimeMarkersProven = (
            $validationTimedOutAfterTestsStarted -and
            -not [string]::IsNullOrWhiteSpace($proofNonce) -and
            -not [string]::IsNullOrWhiteSpace($wrapperSha) -and
            -not [string]::IsNullOrWhiteSpace($entrySha) -and
            $runtimeManifestUnderArtifact -and
            $runtimeManifest -and
            $runtimeCommand -eq "bash /tests/run-tests.sh")
        $runtimeRows.Add([pscustomobject]@{
            side = $sideName
            logical_mode = [string]$metrics.logical_mode
            validation_exit_code = [int]$metrics.public_validation_exit_code
            validation_timeout_after_tests_started = $validationTimedOutAfterTestsStarted
            validation_skipped_after_agent_timeout = $validationSkippedAfterAgentTimeout
            pre_agent_validator_probe_proven = $preAgentProbeProven
            pre_agent_validator_probe_status = if ($metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_status") { [string]$metrics.pre_agent_validator_probe_status } else { "" }
            pre_agent_validator_probe_hash = if ($metrics.PSObject.Properties.Name -contains "pre_agent_validator_probe_hash") { [string]$metrics.pre_agent_validator_probe_hash } else { "" }
            timeout_runtime_markers_proven = $timeoutRuntimeMarkersProven
            proof_nonce = $proofNonce
            docker_wrapper_seen = ((Test-TaskspaceProofMarker $combinedLog "validator_runtime_probe=terminal_bench_docker_wrapper") -or
                (Test-TaskspaceProofMarker $combinedLog "validator_runtime_probe=terminal_bench_equivalent_wrapper"))
            docker_app_runtime_seen = ((Test-TaskspaceProofMarker $combinedLog "validator_runtime=terminal_bench_docker_app") -or
                (Test-TaskspaceProofMarker $combinedLog "validator_runtime=terminal_bench_equivalent_docker_app"))
            container_workdir_app = (Test-TaskspaceProofMarker $combinedLog "container_workdir=/app")
            docker_inspect_seen = (Test-TaskspaceProofMarker $combinedLog "docker_inspect_available=True")
            test_dir_seen = ((Test-TaskspaceProofMarker $combinedLog "test_dir=/tbench-validator/tests") -or
                (Test-TaskspaceProofMarker $combinedLog "test_dir=/tests"))
            validator_mount_seen = ((Test-TaskspaceProofMarker $combinedLog "validator_mount=/tbench-validator") -or
                (Test-TaskspaceProofMarker $combinedLog "validator_mount=/tests"))
            validator_mount_readonly = (Test-TaskspaceProofMarker $combinedLog "validator_mount_readonly=true")
            official_test_command_seen = (Test-TaskspaceProofMarker $combinedLog "validator_command=bash /tests/run-tests.sh")
            official_protocol_pre_agent_probe_proven = $preAgentProbeProven
            wrapper_sha = $wrapperSha
            entry_sha = $entrySha
            runtime_manifest_path = $runtimeManifestPath
            runtime_manifest_under_artifact = $runtimeManifestUnderArtifact
            docker_inspect_path = $inspectPath
            docker_inspect_under_artifact = $inspectUnderArtifact
            validation_cleanup_result_path = $cleanupResultPath
            validation_cleanup_result_under_artifact = $cleanupResultUnderArtifact
            validation_cleanup_classification = if ($cleanupResult -and $cleanupResult.PSObject.Properties.Name -contains "classification") { [string]$cleanupResult.classification } else { "" }
            validation_cleanup_identity_matched = if ($cleanupResult -and $cleanupResult.PSObject.Properties.Name -contains "identity_matched") { [bool]$cleanupResult.identity_matched } else { $false }
            uv_cache_mount_seen = ($uvCacheMount.Count -gt 0)
            uv_cache_mount_readonly = ($uvCacheMount.Count -gt 0 -and -not [bool]$uvCacheMount[0].RW)
            uv_cache_source_matches_manifest = $uvCacheSourceMatches
            uv_installer_sha256 = $uvInstallerSha
            uv_archive_sha256 = $uvArchiveSha
            uv_cache_proven = $uvCacheProven
            runtime_manifest_proven = ($runtimeManifestUnderArtifact -and $runtimeManifest -and
                $runtimeCommand -eq "bash /tests/run-tests.sh" -and
                $runtimeManifest.proof_nonce -eq $proofNonce -and
                $manifestWrapperSha -eq $wrapperSha -and
                $manifestWrapperSha -eq $expectedWrapperSha -and
                $actualWrapperSha -eq $wrapperSha -and
                $manifestEntrySha -eq $entrySha -and
                $actualEntrySha -eq $entrySha -and
                $uvCacheProven)
            docker_inspect_mounts_proven = ($inspectUnderArtifact -and $container -and
                $testsMount.Count -gt 0 -and -not [bool]$testsMount[0].RW -and
                $appMount.Count -gt 0 -and
                $entryMount.Count -gt 0 -and -not [bool]$entryMount[0].RW -and
                $workdir -eq "/app")
            cleanup_container_exit = (Get-TaskspaceProofMarkerValue $combinedLog "validator_cleanup_container_exit")
            cleanup_image_exit = (Get-TaskspaceProofMarkerValue $combinedLog "validator_cleanup_image_exit")
        })
    }
    $runtimeOk = @($runtimeRows | Where-Object {
            $fullRuntimeProof = ($_.docker_wrapper_seen -and $_.docker_app_runtime_seen -and $_.container_workdir_app -and $_.docker_inspect_seen -and
                $_.proof_nonce -match '^[0-9a-f]{32}$' -and $_.wrapper_sha -match '^[0-9a-f]{64}$' -and $_.entry_sha -match '^[0-9a-f]{64}$' -and
                $_.runtime_manifest_proven -and $_.docker_inspect_mounts_proven -and
                $_.validation_cleanup_result_under_artifact -and $_.validation_cleanup_classification -eq "ok")
            $timeoutAfterStartProof = ($_.validation_timeout_after_tests_started -and $_.timeout_runtime_markers_proven -and
                $_.docker_wrapper_seen -and $_.docker_app_runtime_seen -and $_.container_workdir_app -and
                $_.test_dir_seen -and $_.validator_mount_seen -and $_.validator_mount_readonly -and $_.official_test_command_seen)
            $skippedAfterAgentTimeoutProof = ($_.validation_skipped_after_agent_timeout -and $_.pre_agent_validator_probe_proven)
            -not ($fullRuntimeProof -or $timeoutAfterStartProof -or $skippedAfterAgentTimeoutProof)
        }).Count -eq 0
    $mountOk = @($runtimeRows | Where-Object {
            $fullMountProof = ($_.test_dir_seen -and $_.validator_mount_seen -and $_.validator_mount_readonly -and $_.docker_inspect_mounts_proven)
            $timeoutAfterStartMountProof = ($_.validation_timeout_after_tests_started -and $_.test_dir_seen -and $_.validator_mount_seen -and $_.validator_mount_readonly)
            $skippedAfterAgentTimeoutProof = ($_.validation_skipped_after_agent_timeout -and $_.pre_agent_validator_probe_proven)
            -not ($fullMountProof -or $timeoutAfterStartMountProof -or $skippedAfterAgentTimeoutProof)
        }).Count -eq 0
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

    $adapterMetadata = if ($external.PSObject.Properties.Name -contains "adapter_metadata") { $external.adapter_metadata } else { $null }
    $official = if ($null -ne $adapterMetadata -and $adapterMetadata.PSObject.Properties.Name -contains "official_equivalence") {
        $external.adapter_metadata.official_equivalence
    } else { $null }
    $sourceHashRows = New-Object System.Collections.Generic.List[object]
    $sourceHashOk = $false
    $sourceRevisionPinned = ($null -ne $official -and $official.PSObject.Properties.Name -contains "source_revision_pinned" -and [bool]$official.source_revision_pinned)
    $sourceRoot = if ($null -ne $official -and $official.PSObject.Properties.Name -contains "source_root") { [string]$official.source_root } else { "" }
    $sourceRevision = if ($null -ne $official -and $official.PSObject.Properties.Name -contains "source_revision") { [string]$official.source_revision } else { "" }
    if ($null -ne $official -and $official.PSObject.Properties.Name -contains "source_files") {
        $sourceHashOk = $true
        foreach ($row in @($official.source_files)) {
            $path = [string]$row.path
            $relative = [string]$row.relative_path
            $expectedSha = [string]$row.current_sha256
            $expectedPinnedBlob = [string]$row.pinned_blob_id
            $expectedCurrentBlob = [string]$row.current_blob_id
            $actualSha = Get-TaskspaceSha256IfPresent $path
            $actualCurrentBlob = ""
            $actualPinnedBlob = ""
            if (-not [string]::IsNullOrWhiteSpace($actualSha)) {
                $actualCurrentBlob = Invoke-TaskspaceGitScalar $sourceRoot @("hash-object", $path)
            }
            $actualPinnedBlob = Invoke-TaskspaceGitScalar $sourceRoot @("rev-parse", "$sourceRevision`:$relative")
            $matches = (
                -not [string]::IsNullOrWhiteSpace($actualSha) -and
                $actualSha -eq $expectedSha -and
                $actualCurrentBlob -eq $expectedCurrentBlob -and
                $actualPinnedBlob -eq $expectedPinnedBlob -and
                $actualCurrentBlob -eq $actualPinnedBlob)
            if (-not $matches) { $sourceHashOk = $false }
            $sourceHashRows.Add([pscustomobject]@{
                path = $path
                relative_path = $relative
                expected_sha256 = $expectedSha
                actual_sha256 = $actualSha
                expected_current_blob_id = $expectedCurrentBlob
                actual_current_blob_id = $actualCurrentBlob
                expected_pinned_blob_id = $expectedPinnedBlob
                actual_pinned_blob_id = $actualPinnedBlob
                matches = $matches
            })
        }
    }
    $officialProtocolOk = ($null -ne $official -and
        $official.PSObject.Properties.Name -contains "protocol" -and
        [string]$official.protocol -eq "terminal_bench_post_agent_tests_v1" -and
        $sourceRevisionPinned -and
        $sourceHashOk -and
        ($official.PSObject.Properties.Name -contains "source_files_match_pinned_revision" -and [bool]$official.source_files_match_pinned_revision) -and
        ($official.PSObject.Properties.Name -contains "task_worktree_dirty" -and -not [bool]$official.task_worktree_dirty) -and
        @($runtimeRows | Where-Object { -not ($_.official_test_command_seen -or $_.official_protocol_pre_agent_probe_proven) }).Count -eq 0)
    $remoteAssets = if ($null -ne $adapterMetadata -and $adapterMetadata.PSObject.Properties.Name -contains "remote_assets") {
        @($adapterMetadata.remote_assets)
    } else { @() }
    $remoteAssetsOk = @($remoteAssets | Where-Object { $_.required_for_e3 -and -not [bool]$_.equivalence_proven }).Count -eq 0
    $runnerProof = [pscustomobject]@{
        benchmark = [string]$external.name
        adapter_version = [string]$external.adapter_version
        protocol = if ($null -ne $official -and $official.PSObject.Properties.Name -contains "protocol") { [string]$official.protocol } else { "" }
        official_source_revision = $sourceRevision
        official_source_revision_pinned = $sourceRevisionPinned
        official_source_hashes_match = $sourceHashOk
        official_source_files_match_pinned_revision = if ($null -ne $official -and $official.PSObject.Properties.Name -contains "source_files_match_pinned_revision") { [bool]$official.source_files_match_pinned_revision } else { $false }
        task_worktree_dirty = if ($null -ne $official -and $official.PSObject.Properties.Name -contains "task_worktree_dirty") { [bool]$official.task_worktree_dirty } else { $true }
        official_protocol_source_proven = $officialProtocolOk
        remote_assets_equivalence_proven = $remoteAssetsOk
        remote_assets = @($remoteAssets)
        runtime_command_matches_official_protocol = @($runtimeRows | Where-Object { -not ($_.official_test_command_seen -or $_.official_protocol_pre_agent_probe_proven) }).Count -eq 0
        source_hashes = @($sourceHashRows.ToArray())
    }
    Write-TaskspaceJson $runnerProof $runnerPath

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
        $repoHits = @(Find-TaskspaceValidatorSourceHitsInRepo $side.RepoDir)
        $artifactHits = New-Object System.Collections.Generic.List[string]
        foreach ($token in @($validatorSourcePath, "external-validator-source", "/tbench-validator", "\tbench-validator")) {
            if (-not [string]::IsNullOrWhiteSpace($token) -and $agentText.Contains($token)) { $artifactHits.Add($token) }
        }
        $isolationRows.Add([pscustomobject]@{
            side = $sideName
            logical_mode = [string]$metrics.logical_mode
            validator_source_under_agent_repo = $sourceUnderRepo
            repo_validator_hits = @($repoHits)
            agent_artifact_validator_tokens = @($artifactHits.ToArray())
            proven = ($validatorSourceExists -and $validatorSourceHashMatches -and -not $sourceUnderRepo -and $repoHits.Count -eq 0 -and $artifactHits.Count -eq 0)
        })
    }
    $isolationOk = @($isolationRows | Where-Object { -not $_.proven }).Count -eq 0
    $guardPath = if ($null -ne $SourceGuard -and $SourceGuard.PSObject.Properties.Name -contains "proof_path") { [string]$SourceGuard.proof_path } else { Join-Path $Pair.PairDir "external-source-guard-proof.json" }
    $guardProof = Read-TaskspaceJsonIfPresent $guardPath
    $guardRows = if ($guardProof -and $guardProof.PSObject.Properties.Name -contains "files") { @($guardProof.files) } else { @() }
    $requiredProbeKinds = if ($guardProof -and $guardProof.PSObject.Properties.Name -contains "required_probe_kinds") { @($guardProof.required_probe_kinds) } else { @("current_powershell", "powershell_child", "cmd_child") }
    $guardFileRows = New-Object System.Collections.Generic.List[object]
    foreach ($row in $guardRows) {
        $probeKinds = @($row.probes_after_protect | ForEach-Object { [string]$_.kind })
        $missingProbes = @($requiredProbeKinds | Where-Object { $probeKinds -notcontains [string]$_ })
        $failedProbes = @($row.probes_after_protect | Where-Object { $_.available -and -not [bool]$_.read_denied })
        $releaseRow = @($guardProof.release_files | Where-Object { [string]$_.path -eq [string]$row.path } | Select-Object -First 1)
        $hashRestored = ($releaseRow.Count -gt 0 -and [string]$releaseRow[0].file_sha256_after_release -eq [string]$row.file_sha256_before_protect)
        $guardFileRows.Add([pscustomobject]@{
            path = [string]$row.path
            deny_exit_code = [int]$row.deny_exit_code
            missing_probe_kinds = @($missingProbes)
            failed_probe_kinds = @($failedProbes | ForEach-Object { [string]$_.kind })
            release_exit_code = if ($releaseRow.Count -gt 0) { [int]$releaseRow[0].remove_exit_code } else { -1 }
            hash_restored_after_release = $hashRestored
            proven = ([int]$row.deny_exit_code -eq 0 -and $missingProbes.Count -eq 0 -and $failedProbes.Count -eq 0 -and
                $releaseRow.Count -gt 0 -and [int]$releaseRow[0].remove_exit_code -eq 0 -and $hashRestored)
        })
    }
    $guardOk = ($guardProof -and
        [bool]$guardProof.active -and
        [int]$guardProof.protected_file_count -gt 0 -and
        [bool]$guardProof.all_reads_denied_after_protect -and
        [bool]$guardProof.all_denies_removed_after_release -and
        [bool]$guardProof.all_reads_restored_after_release -and
        @($guardFileRows | Where-Object { -not $_.proven }).Count -eq 0)
    $isolationProof = [pscustomobject]@{
        validator_source_dir = $validatorSourceRel
        validator_source_path = $validatorSourcePath
        validator_source_exists = $validatorSourceExists
        validator_source_sha256 = $validatorSourceSha
        expected_validator_source_sha256 = $expectedValidatorSha
        validator_source_hash_matches = $validatorSourceHashMatches
        validator_source_outside_repo_proven = $isolationOk
        source_guard_proof_path = $guardPath
        source_guard_proven = $guardOk
        source_guard_checks = @($guardFileRows.ToArray())
        agent_cannot_read_validator_source_proven = ($isolationOk -and $guardOk -and (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source"))
        declared_agent_cannot_read_validator_source = (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source")
        sides = @($isolationRows.ToArray())
    }
    Write-TaskspaceJson $isolationProof $isolationPath

    $officialEquivalent = $runtimeOk -and $mountOk -and $officialProtocolOk -and $remoteAssetsOk -and (Get-TaskspaceBoolField $declared "official_runner_or_equivalent")
    $sourceIsolated = $isolationOk -and $guardOk -and (Get-TaskspaceBoolField $declared "agent_cannot_read_validator_source")
    $eligible = $officialEquivalent -and $sourceIsolated -and (Get-TaskspaceBoolField $declared "e3_eligible")
    $combinedProof = [pscustomobject]@{
        runtime_proof_path = $runtimePath
        runner_equivalence_proof_path = $runnerPath
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
