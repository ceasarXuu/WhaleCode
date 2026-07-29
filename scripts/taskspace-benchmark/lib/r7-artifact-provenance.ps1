function Get-R7ProvenanceProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
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

function Get-R7MatrixArtifactProvenance {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$ManifestPath,
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$ReportScriptPath
    )
    $findings = [System.Collections.Generic.List[object]]::new()
    $runFacts = [System.Collections.Generic.List[object]]::new()
    $binaryHashes = @{}
    $repoCommit = [string](Get-R7ProvenanceProperty $Manifest "repo_commit" "")
    $manifestBinarySha = ([string](Get-R7ProvenanceProperty $Manifest "whale_sha256" "")).ToLowerInvariant()

    if ($repoCommit -notmatch '^[0-9a-f]{40,64}$') {
        Add-R7ProvenanceFinding $findings "matrix_repo_commit_invalid" $ManifestPath
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
        $runStatus = Read-R7ProvenanceJson $runStatusPath $findings "run_status_missing" "run_status_invalid"
        $health = Read-R7ProvenanceJson $healthPath $findings "run_binary_health_missing" "run_binary_health_invalid"

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
            if ([int]$attestation.schema_version -ne 1 -or
                [string]$attestation.status -ne "pass" -or
                ([string]$attestation.whale_binary_sha256).ToLowerInvariant() -ne $manifestBinarySha -or
                [string]$attestation.current_git_head -ne $repoCommit -or
                [string]$attestation.codex_source_latest_commit -ne $sourceCommit -or
                -not [string]::Equals($attestedRepoRoot, [IO.Path]::GetFullPath($RepoRoot), [StringComparison]::OrdinalIgnoreCase)) {
                Add-R7ProvenanceFinding $findings "run_attestation_identity_mismatch" $attestationPath $runDir
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
                final_aggregate_ready = if ($runStatus) { [bool]$runStatus.final_aggregate_ready } else { $false }
                status = if (@($findings | Where-Object run_dir -eq $runDir).Count) { "invalid" } else { "valid" }
            })
    }

    [pscustomobject]@{
        schema_version = 1
        status = if ($findings.Count) { "invalid" } else { "valid" }
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
        findings = @($findings)
        generated_at = (Get-Date).ToString("o")
    }
}
