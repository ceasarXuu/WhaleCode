function Get-R7ProvenanceProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

if (-not (Get-Command Get-TaskspaceGitBuildIdentity -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "harness-health.ps1")
}

function Get-R7ProvenanceFileFact {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[object]]$Findings,
        [Parameter(Mandatory = $true)][string]$MissingCode,
        [string]$RunDir = ""
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Add-R7ProvenanceFinding $Findings $MissingCode $Path $RunDir
        return $null
    }
    New-R7ProvenanceFileFact $Path
}

function New-R7ProvenanceFileFact {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$Role = ""
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Cannot seal missing R7 artifact: $Path"
    }
    $item = Get-Item -LiteralPath $Path
    [pscustomobject]@{
        role = $Role
        path = $item.FullName
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash.ToLowerInvariant()
        bytes = [int64]$item.Length
    }
}

function Test-R7ProvenanceFileFact {
    param($Fact)
    if ($null -eq $Fact -or
        -not (Test-Path -LiteralPath ([string]$Fact.path) -PathType Leaf)) {
        return $false
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath ([string]$Fact.path)).Hash.ToLowerInvariant()
    $item = Get-Item -LiteralPath ([string]$Fact.path)
    $actual -eq ([string]$Fact.sha256).ToLowerInvariant() -and
        [int64]$item.Length -eq [int64]$Fact.bytes
}

function Read-R7ProvenanceJson {
    param(
        [string]$Path,
        [System.Collections.Generic.List[object]]$Findings,
        [string]$MissingCode,
        [string]$InvalidCode
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        $Findings.Add([pscustomobject]@{ code = $MissingCode; path = $Path })
        return $null
    }
    try {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json -Depth 100
    } catch {
        $Findings.Add([pscustomobject]@{ code = $InvalidCode; path = $Path })
        $null
    }
}

function Add-R7ProvenanceFinding {
    param(
        [System.Collections.Generic.List[object]]$Findings,
        [string]$Code,
        [string]$Path,
        [string]$RunDir = ""
    )
    $Findings.Add([pscustomobject]@{
            code = $Code
            path = $Path
            run_dir = $RunDir
        })
}

function Write-R7RunArtifactEvidenceManifest {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][string]$LogicalMode
    )
    $observationPath = Join-Path $RunDir "performance-observation.json"
    if (-not (Test-Path -LiteralPath $observationPath -PathType Leaf)) {
        throw "Cannot seal R7 run without performance observation: $RunDir"
    }
    $observation = Get-Content -Raw -Encoding UTF8 -LiteralPath $observationPath |
        ConvertFrom-Json -Depth 100
    $rows = @(
        Get-R7ProvenanceProperty $observation "rows" @() |
            Where-Object {
                [string]$_.observation_status -in @("complete", "incomplete") -and
                [string]$_.logical_mode -eq $LogicalMode
            }
    )
    if ($rows.Count -ne 1) {
        throw "Cannot seal R7 run without exactly one observed $LogicalMode side: $RunDir"
    }
    $artifactDir = [string]$rows[0].artifact_dir
    $healthPath = Join-Path $RunDir "whale-binary-preflight-health.json"
    $health = Get-Content -Raw -Encoding UTF8 -LiteralPath $healthPath |
        ConvertFrom-Json -Depth 100
    $artifactPaths = [ordered]@{
        run_status = Join-Path $RunDir "run-status.json"
        binary_health = $healthPath
        performance_observation = $observationPath
        resolved_manifest = Join-Path $RunDir "pair-001/manifest.resolved.json"
        rollout = Join-Path $artifactDir "rollout.jsonl"
        provider_wire_trace = Join-Path $artifactDir "provider-wire-trace.jsonl"
        request_summary = Join-Path $artifactDir "request-summary.json"
        binary_attestation = [string]$health.build_attestation_path
    }
    $facts = @(
        foreach ($entry in $artifactPaths.GetEnumerator()) {
            New-R7ProvenanceFileFact ([string]$entry.Value) ([string]$entry.Key)
        }
    )
    $manifestPath = Join-Path $RunDir "r7-artifact-evidence-manifest.json"
    $manifest = [ordered]@{
        schema_version = 1
        status = "sealed"
        run_dir = [IO.Path]::GetFullPath($RunDir)
        logical_mode = $LogicalMode
        artifact_dir = [IO.Path]::GetFullPath($artifactDir)
        artifacts = $facts
        sealed_at = (Get-Date).ToString("o")
    }
    [IO.File]::WriteAllText(
        $manifestPath,
        (($manifest | ConvertTo-Json -Depth 50) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )
    New-R7ProvenanceFileFact $manifestPath "run_evidence_manifest"
}

function Get-R7MatrixArtifactProvenance {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$ManifestPath,
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$ReportScriptPath,
        [string]$MatrixStatusPath = ""
    )
    $findings = [System.Collections.Generic.List[object]]::new()
    $runFacts = [System.Collections.Generic.List[object]]::new()
    $binaryHashes = @{}
    $repoCommit = [string](Get-R7ProvenanceProperty $Manifest "repo_commit" "")
    $manifestBinarySha = ([string](Get-R7ProvenanceProperty $Manifest "whale_sha256" "")).ToLowerInvariant()
    $gitIdentity = try { Get-TaskspaceGitBuildIdentity $RepoRoot } catch { $null }

    if ($repoCommit -notmatch '^[0-9a-f]{40,64}$') {
        Add-R7ProvenanceFinding $findings "matrix_repo_commit_invalid" $ManifestPath
    }
    if ($null -eq $gitIdentity -or [string]$gitIdentity.current_git_head -ne $repoCommit) {
        Add-R7ProvenanceFinding $findings "matrix_repo_commit_not_current_head" $ManifestPath
    }
    if ($manifestBinarySha -notmatch '^[0-9a-f]{64}$') {
        Add-R7ProvenanceFinding $findings "matrix_binary_sha_invalid" $ManifestPath
    }
    $runs = @(Get-R7ProvenanceProperty $Manifest "runs" @())
    if ([int](Get-R7ProvenanceProperty $Manifest "completed_run_count" -1) -ne $runs.Count) {
        Add-R7ProvenanceFinding $findings "matrix_completed_run_count_mismatch" $ManifestPath
    }
    $uniqueRunDirs = @($runs | ForEach-Object { [string]$_.run_dir } | Sort-Object -Unique)
    if ($uniqueRunDirs.Count -ne $runs.Count) {
        Add-R7ProvenanceFinding $findings "matrix_run_dir_duplicate" $ManifestPath
    }

    foreach ($run in $runs) {
        $runDir = [string]$run.run_dir
        $runStatusPath = Join-Path $runDir "run-status.json"
        $healthPath = Join-Path $runDir "whale-binary-preflight-health.json"
        $observationPath = Join-Path $runDir "performance-observation.json"
        $resolvedManifestPath = Join-Path $runDir "pair-001/manifest.resolved.json"
        $runStatus = Read-R7ProvenanceJson $runStatusPath $findings "run_status_missing" "run_status_invalid"
        $health = Read-R7ProvenanceJson $healthPath $findings "run_binary_health_missing" "run_binary_health_invalid"
        $observation = Read-R7ProvenanceJson $observationPath $findings "run_observation_missing" "run_observation_invalid"

        if ([int](Get-R7ProvenanceProperty $run "exit_code" -1) -ne 0) {
            Add-R7ProvenanceFinding $findings "matrix_run_exit_nonzero" $ManifestPath $runDir
        }
        if ($runStatus) {
            if ([string]$runStatus.phase -ne "completed") {
                Add-R7ProvenanceFinding $findings "run_status_not_completed" $runStatusPath $runDir
            }
            if ([string]$runStatus.run_validity -ne "valid" -or
                [bool]$runStatus.diagnostic_comparison_enabled -ne $true -or
                [int]$runStatus.exit_code -ne 0) {
                Add-R7ProvenanceFinding $findings "run_status_not_comparable" $runStatusPath $runDir
            }
            if ([int]$runStatus.attempted_pairs -lt 1 -or
                [int]$runStatus.attempted_pairs -ne [int]$runStatus.completed_pairs) {
                Add-R7ProvenanceFinding $findings "run_pair_count_incomplete" $runStatusPath $runDir
            }
        }

        $binaryPath = ""
        $sourceCommit = ""
        $attestationPath = ""
        if ($health) {
            $healthBinarySha = ([string]$health.whale_binary_sha256).ToLowerInvariant()
            $binaryPath = [string]$health.whale_bin_resolved
            $sourceCommit = [string]$health.codex_source_latest_commit.hash
            $attestationPath = [string]$health.build_attestation_path
            if ([string]$health.status -ne "pass" -or
                [string]$health.run_validity -ne "valid" -or
                [string]$health.build_attestation_status -ne "pass") {
                Add-R7ProvenanceFinding $findings "run_binary_health_not_attested" $healthPath $runDir
            }
            if ($healthBinarySha -ne $manifestBinarySha) {
                Add-R7ProvenanceFinding $findings "run_binary_sha_mismatch" $healthPath $runDir
            }
            if ([string]$health.current_git_head -ne $repoCommit) {
                Add-R7ProvenanceFinding $findings "run_repo_commit_mismatch" $healthPath $runDir
            }
            if ($sourceCommit -notmatch '^[0-9a-f]{40,64}$') {
                Add-R7ProvenanceFinding $findings "run_source_commit_invalid" $healthPath $runDir
            }
            if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
                Add-R7ProvenanceFinding $findings "run_binary_missing" $binaryPath $runDir
            } else {
                if (-not $binaryHashes.ContainsKey($binaryPath)) {
                    $binaryHashes[$binaryPath] = (Get-FileHash -Algorithm SHA256 -LiteralPath $binaryPath).Hash.ToLowerInvariant()
                }
                if ([string]$binaryHashes[$binaryPath] -ne $healthBinarySha) {
                    Add-R7ProvenanceFinding $findings "run_binary_content_mismatch" $binaryPath $runDir
                }
            }
        }

        $attestation = if ($attestationPath) {
            Read-R7ProvenanceJson $attestationPath $findings "run_attestation_missing" "run_attestation_invalid"
        } else {
            Add-R7ProvenanceFinding $findings "run_attestation_path_missing" $healthPath $runDir
            $null
        }
        if ($attestation) {
            $attestedRepoRoot = try { [IO.Path]::GetFullPath([string]$attestation.repo_root) } catch { "" }
            $attestedBinary = try { [IO.Path]::GetFullPath([string]$attestation.whale_bin) } catch { "" }
            $resolvedBinary = try { [IO.Path]::GetFullPath($binaryPath) } catch { "" }
            $probe = if ($binaryPath -and (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
                try { Get-TaskspaceWhaleVersionProbe $binaryPath } catch { $null }
            } else {
                $null
            }
            $markerProbe = $attestation.executable_probe
            if ([int]$attestation.schema_version -ne 2 -or
                [string]$attestation.status -ne "pass" -or
                ([string]$attestation.whale_binary_sha256).ToLowerInvariant() -ne $manifestBinarySha -or
                [string]$attestation.current_git_head -ne $repoCommit -or
                [string]$attestation.codex_source_latest_commit -ne $sourceCommit -or
                -not [bool]$attestation.worktree_clean -or
                $null -eq $gitIdentity -or
                [string]$attestation.head_tree_id -ne [string]$gitIdentity.head_tree_id -or
                [string]$attestation.codex_tree_id -ne [string]$gitIdentity.codex_tree_id -or
                [string]::IsNullOrWhiteSpace([string]$attestation.build_command) -or
                $null -eq $probe -or [int]$probe.exit_code -ne 0 -or
                $null -eq $markerProbe -or [int]$markerProbe.exit_code -ne 0 -or
                [string]$markerProbe.output_sha256 -ne [string]$probe.output_sha256 -or
                -not [string]::Equals($attestedBinary, $resolvedBinary, [StringComparison]::OrdinalIgnoreCase) -or
                -not [string]::Equals($attestedRepoRoot, [IO.Path]::GetFullPath($RepoRoot), [StringComparison]::OrdinalIgnoreCase)) {
                Add-R7ProvenanceFinding $findings "run_attestation_identity_mismatch" $attestationPath $runDir
            }
        }

        $artifactDir = ""
        if ($observation) {
            $matchingRows = @(
                Get-R7ProvenanceProperty $observation "rows" @() |
                    Where-Object {
                        [string]$_.observation_status -in @("complete", "incomplete") -and
                        [string]$_.logical_mode -eq [string]$run.logical_mode
                    }
            )
            if ($matchingRows.Count -ne 1) {
                Add-R7ProvenanceFinding $findings "run_observation_row_mismatch" $observationPath $runDir
            } else {
                $artifactDir = [string]$matchingRows[0].artifact_dir
            }
        }
        $expectedEvidencePath = Join-Path $runDir "r7-artifact-evidence-manifest.json"
        $evidencePath = [string](Get-R7ProvenanceProperty $run "evidence_manifest_path" "")
        $evidenceSha = ([string](Get-R7ProvenanceProperty $run "evidence_manifest_sha256" "")).ToLowerInvariant()
        $evidence = $null
        if ([string]::IsNullOrWhiteSpace($evidencePath) -or
            -not [string]::Equals(
                [IO.Path]::GetFullPath($evidencePath),
                [IO.Path]::GetFullPath($expectedEvidencePath),
                [StringComparison]::OrdinalIgnoreCase
            )) {
            Add-R7ProvenanceFinding $findings "run_evidence_manifest_path_mismatch" $evidencePath $runDir
        } elseif (-not (Test-Path -LiteralPath $evidencePath -PathType Leaf)) {
            Add-R7ProvenanceFinding $findings "run_evidence_manifest_missing" $evidencePath $runDir
        } elseif ($evidenceSha -notmatch '^[0-9a-f]{64}$' -or
            (Get-FileHash -Algorithm SHA256 -LiteralPath $evidencePath).Hash.ToLowerInvariant() -ne $evidenceSha) {
            Add-R7ProvenanceFinding $findings "run_evidence_manifest_hash_mismatch" $evidencePath $runDir
        } else {
            $evidence = Read-R7ProvenanceJson `
                $evidencePath `
                $findings `
                "run_evidence_manifest_missing" `
                "run_evidence_manifest_invalid"
        }
        $rawFacts = @()
        if ($evidence) {
            $rawFacts = @(Get-R7ProvenanceProperty $evidence "artifacts" @())
            $requiredRoles = @(
                "run_status",
                "binary_health",
                "performance_observation",
                "resolved_manifest",
                "rollout",
                "provider_wire_trace",
                "request_summary",
                "binary_attestation"
            )
            $actualRoles = @($rawFacts | ForEach-Object { [string]$_.role })
            $evidenceRunDir = try { [IO.Path]::GetFullPath([string]$evidence.run_dir) } catch { "" }
            $evidenceArtifactDir = try { [IO.Path]::GetFullPath([string]$evidence.artifact_dir) } catch { "" }
            if ([int]$evidence.schema_version -ne 1 -or
                [string]$evidence.status -ne "sealed" -or
                [string]$evidence.logical_mode -ne [string]$run.logical_mode -or
                -not [string]::Equals($evidenceRunDir, [IO.Path]::GetFullPath($runDir), [StringComparison]::OrdinalIgnoreCase) -or
                -not [string]::Equals($evidenceArtifactDir, [IO.Path]::GetFullPath($artifactDir), [StringComparison]::OrdinalIgnoreCase) -or
                (Compare-Object $requiredRoles $actualRoles)) {
                Add-R7ProvenanceFinding $findings "run_evidence_manifest_identity_mismatch" $evidencePath $runDir
            }
            foreach ($fact in $rawFacts) {
                if (-not (Test-R7ProvenanceFileFact $fact)) {
                    Add-R7ProvenanceFinding $findings "run_evidence_artifact_hash_mismatch" ([string]$fact.path) $runDir
                }
            }
        }
        $runFacts.Add([pscustomobject]@{
                run_dir = $runDir
                run_status_path = $runStatusPath
                binary_health_path = $healthPath
                binary_path = $binaryPath
                binary_sha256 = if ($health) { [string]$health.whale_binary_sha256 } else { "" }
                source_commit = $sourceCommit
                attestation_path = $attestationPath
                artifact_dir = $artifactDir
                evidence_manifest_path = $evidencePath
                evidence_manifest_sha256 = $evidenceSha
                raw_artifacts = @($rawFacts)
                final_aggregate_ready = if ($runStatus) { [bool]$runStatus.final_aggregate_ready } else { $false }
                status = if (@($findings | Where-Object run_dir -eq $runDir).Count) { "invalid" } else { "valid" }
            })
    }

    $matrixStatus = $null
    $matrixStatusFact = $null
    if (-not [string]::IsNullOrWhiteSpace($MatrixStatusPath)) {
        $matrixStatus = Read-R7ProvenanceJson `
            $MatrixStatusPath `
            $findings `
            "matrix_final_status_missing" `
            "matrix_final_status_invalid"
        $matrixStatusFact = Get-R7ProvenanceFileFact `
            $MatrixStatusPath `
            $findings `
            "matrix_final_status_missing"
        if ($matrixStatus) {
            $inputFacts = @(Get-R7ProvenanceProperty $matrixStatus "inputs" @())
            $outputFacts = @(Get-R7ProvenanceProperty $matrixStatus "outputs" @())
            $requiredNames = @("summary.csv", "aggregate.csv", "trace-analysis.json", "report.md")
            $actualNames = @($outputFacts | ForEach-Object { Split-Path -Leaf ([string]$_.path) })
            $manifestInputs = @($inputFacts | Where-Object { [string]$_.role -eq "run_manifest" })
            if ([int]$matrixStatus.schema_version -ne 1 -or
                [string]$matrixStatus.status -ne "finalized" -or
                -not [bool]$matrixStatus.final_aggregate_ready -or
                [string]$matrixStatus.repo_commit -ne $repoCommit -or
                [int]$matrixStatus.run_count -ne $runs.Count -or
                $inputFacts.Count -ne 1 -or
                $manifestInputs.Count -ne 1 -or
                -not [string]::Equals(
                    [IO.Path]::GetFullPath([string]$manifestInputs[0].path),
                    [IO.Path]::GetFullPath($ManifestPath),
                    [StringComparison]::OrdinalIgnoreCase
                ) -or
                (Compare-Object $requiredNames $actualNames)) {
                Add-R7ProvenanceFinding $findings "matrix_final_status_identity_mismatch" $MatrixStatusPath
            }
            foreach ($inputFact in $inputFacts) {
                if (-not (Test-R7ProvenanceFileFact $inputFact)) {
                    Add-R7ProvenanceFinding `
                        $findings `
                        "matrix_final_input_hash_mismatch" `
                        ([string]$inputFact.path)
                }
            }
            foreach ($outputFact in $outputFacts) {
                if (-not (Test-R7ProvenanceFileFact $outputFact)) {
                    Add-R7ProvenanceFinding `
                        $findings `
                        "matrix_final_output_hash_mismatch" `
                        ([string]$outputFact.path)
                }
            }
        }
    }

    [pscustomobject]@{
        schema_version = 2
        status = if ($findings.Count) { "invalid" } else { "valid" }
        phase = if ($matrixStatus) { "final" } else { "input" }
        repo_commit = $repoCommit
        whale_binary_sha256 = $manifestBinarySha
        manifest_path = $ManifestPath
        manifest_sha256 = if (Test-Path -LiteralPath $ManifestPath) {
            (Get-FileHash -Algorithm SHA256 -LiteralPath $ManifestPath).Hash.ToLowerInvariant()
        } else { "" }
        report_script_path = $ReportScriptPath
        report_script_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $ReportScriptPath).Hash.ToLowerInvariant()
        run_count = $runs.Count
        runs = @($runFacts)
        raw_artifact_count = (
            $runFacts | ForEach-Object { @($_.raw_artifacts).Count } | Measure-Object -Sum
        ).Sum
        matrix_final_status = $matrixStatus
        matrix_final_status_fact = $matrixStatusFact
        findings = @($findings)
        generated_at = (Get-Date).ToString("o")
    }
}
