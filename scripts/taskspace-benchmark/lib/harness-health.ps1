$script:TaskspaceInvalidHarnessExitCode = 3

function New-TaskspaceInfraSignature {
    param(
        [string]$Category = "harness_materialization_failure",
        [string]$Stage = "unknown",
        [string]$StableCode = "unknown",
        [string]$Message = "",
        [string]$Side = "",
        [string]$Artifact = ""
    )
    [pscustomobject]@{
        schema_version = 1
        category = $Category
        stage = $Stage
        stable_code = $StableCode
        normalized_message = $Message
        side = $Side
        artifact = $Artifact
        key = "$Category/$StableCode"
    }
}

function Test-TaskspaceFullyQualifiedPath {
    param([AllowEmptyString()][string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    if (-not [System.IO.Path]::IsPathRooted($Path)) { return $false }
    $root = [System.IO.Path]::GetPathRoot($Path)
    -not [string]::IsNullOrWhiteSpace($root)
}

function Test-TaskspaceResolvablePathFrom {
    param(
        [Parameter(Mandatory = $true)][string]$BaseDir,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $candidate = if (Test-TaskspaceFullyQualifiedPath $Path) { $Path } else { Join-Path $BaseDir $Path }
    Test-Path -LiteralPath $candidate
}

function Get-TaskspaceMinimumFreeBytes {
    $script:TaskspaceDiskThresholdError = ""
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_BYTES)) {
        try { return [int64]$env:TASKSPACE_MIN_FREE_BYTES } catch {
            $script:TaskspaceDiskThresholdError = "Invalid TASKSPACE_MIN_FREE_BYTES: $env:TASKSPACE_MIN_FREE_BYTES"
            return [int64](20GB)
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($env:TASKSPACE_MIN_FREE_GIB)) {
        try { return [int64]([double]$env:TASKSPACE_MIN_FREE_GIB * 1GB) } catch {
            $script:TaskspaceDiskThresholdError = "Invalid TASKSPACE_MIN_FREE_GIB: $env:TASKSPACE_MIN_FREE_GIB"
            return [int64](20GB)
        }
    }
    [int64](20GB)
}

function Get-TaskspaceExistingPathForDisk {
    param([AllowEmptyString()][string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return "" }
    $candidate = [System.IO.Path]::GetFullPath($Path)
    while (-not [string]::IsNullOrWhiteSpace($candidate) -and -not (Test-Path -LiteralPath $candidate)) {
        $parent = Split-Path -Parent $candidate
        if ($parent -eq $candidate) { break }
        $candidate = $parent
    }
    if (Test-Path -LiteralPath $candidate) { return $candidate }
    ""
}

function Get-TaskspaceDiskSpaceChecks {
    param([Parameter(Mandatory = $true)][string[]]$Paths)
    $minimum = Get-TaskspaceMinimumFreeBytes
    $checks = New-Object System.Collections.Generic.List[object]
    $seenRoots = @{}
    foreach ($path in @($Paths)) {
        $existing = Get-TaskspaceExistingPathForDisk $path
        if ([string]::IsNullOrWhiteSpace($existing)) { continue }
        $root = [System.IO.Path]::GetPathRoot($existing)
        if ([string]::IsNullOrWhiteSpace($root) -or $seenRoots.ContainsKey($root)) { continue }
        $seenRoots[$root] = $true
        try {
            $drive = New-Object System.IO.DriveInfo($root)
            $free = [int64]$drive.AvailableFreeSpace
            $checks.Add([pscustomobject]@{
                    root = $root
                    path = $existing
                    free_bytes = $free
                    required_free_bytes = $minimum
                    free_gib = [math]::Round($free / 1GB, 2)
                    required_free_gib = [math]::Round($minimum / 1GB, 2)
                    status = if ($free -lt $minimum) { "fail" } else { "pass" }
                })
        } catch {
            $checks.Add([pscustomobject]@{
                    root = $root
                    path = $existing
                    free_bytes = 0
                    required_free_bytes = $minimum
                    free_gib = 0
                    required_free_gib = [math]::Round($minimum / 1GB, 2)
                    status = "fail"
                    error = [string]$_.Exception.Message
                })
        }
    }
    @($checks.ToArray())
}

function Get-TaskspaceSupplementalDiskPaths {
    $paths = New-Object System.Collections.Generic.List[string]
    foreach ($candidate in @($env:UV_CACHE_DIR, $env:WHALE_TASKSPACE_UV_CACHE_DIR)) {
        if (-not [string]::IsNullOrWhiteSpace([string]$candidate)) { $paths.Add([string]$candidate) }
    }
    $dockerCommand = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerCommand -and -not [string]::IsNullOrWhiteSpace([string]$dockerCommand.Source)) { $paths.Add([string]$dockerCommand.Source) }
    if (Test-Path -LiteralPath "D:\whale-docker") { $paths.Add("D:\whale-docker") }
    @($paths.ToArray())
}

function Invoke-TaskspaceShortCommand {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSeconds = 20
    )
    $job = Start-Job -ScriptBlock {
        param([string]$InnerCommand, [string[]]$InnerArguments)
        $output = & $InnerCommand @InnerArguments 2>&1
        [pscustomobject]@{ exit_code = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }; output = @($output | ForEach-Object { [string]$_ }) }
    } -ArgumentList $Command, $Arguments
    if (-not (Wait-Job -Job $job -Timeout $TimeoutSeconds)) {
        Stop-Job -Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        return [pscustomobject]@{ exit_code = 124; output = @("timeout after $TimeoutSeconds seconds") }
    }
    $result = Receive-Job -Job $job
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    if ($null -eq $result) { return [pscustomobject]@{ exit_code = 1; output = @("no output") } }
    $result | Select-Object -First 1
}

function Get-TaskspaceDockerStorageChecks {
    param([int64]$MinimumFreeBytes = (Get-TaskspaceMinimumFreeBytes))
    $checks = New-Object System.Collections.Generic.List[object]
    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }
    if (-not (Get-Command wsl -ErrorAction SilentlyContinue)) { return @($checks.ToArray()) }
    $info = Invoke-TaskspaceShortCommand "wsl" @("-d", $distro, "--", "sh", "-lc", "docker info --format '{{.DockerRootDir}}' 2>/dev/null || true") 20
    $dockerRoot = (@($info.output) | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace([string]$dockerRoot)) { $dockerRoot = "/var/lib/docker" }
    $paths = @("/", [string]$dockerRoot) | Sort-Object -Unique
    foreach ($path in $paths) {
        $df = Invoke-TaskspaceShortCommand "wsl" @("-d", $distro, "--", "df", "-Pk", $path) 20
        $line = @($df.output | Where-Object { [string]$_ -match "^\S+\s+\d+\s+\d+\s+\d+\s+\d+%" } | Select-Object -Last 1)
        if ([int]$df.exit_code -ne 0 -or -not $line) {
            $checks.Add([pscustomobject]@{ kind = "wsl_df"; distro = $distro; path = $path; status = "fail"; free_bytes = 0; required_free_bytes = $MinimumFreeBytes; message = (@($df.output) -join " ") })
            continue
        }
        $parts = ([string]$line).Trim() -split "\s+"
        $free = [int64]$parts[3] * 1024
        $checks.Add([pscustomobject]@{
                kind = "wsl_df"
                distro = $distro
                path = $path
                docker_root = [string]$dockerRoot
                free_bytes = $free
                required_free_bytes = $MinimumFreeBytes
                free_gib = [math]::Round($free / 1GB, 2)
                required_free_gib = [math]::Round($MinimumFreeBytes / 1GB, 2)
                status = if ($free -lt $MinimumFreeBytes) { "fail" } else { "pass" }
            })
    }
    @($checks.ToArray())
}

function New-TaskspaceDiskHealth {
    param(
        [Parameter(Mandatory = $true)][string[]]$Paths,
        [string]$Stage = "preflight"
    )
    $minimum = Get-TaskspaceMinimumFreeBytes
    $hostChecks = Get-TaskspaceDiskSpaceChecks @($Paths + (Get-TaskspaceSupplementalDiskPaths))
    $dockerChecks = Get-TaskspaceDockerStorageChecks $minimum
    $findings = New-Object System.Collections.Generic.List[object]
    if (-not [string]::IsNullOrWhiteSpace($script:TaskspaceDiskThresholdError)) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "disk_space_threshold_invalid"; message = $script:TaskspaceDiskThresholdError; path = ""; stage = $Stage })
    }
    $allChecks = @($hostChecks) + @($dockerChecks)
    foreach ($space in @($allChecks | Where-Object { [string]$_.status -eq "fail" })) {
        $label = if ($space.PSObject.Properties.Name -contains "root") { [string]$space.root } else { "$($space.distro):$($space.path)" }
        $findings.Add([pscustomobject]@{
                severity = "fail"; stable_code = "disk_space_low"; stage = $Stage
                message = "Free disk space below TaskSpace preflight minimum on ${label}: $($space.free_gib) GiB available, $($space.required_free_gib) GiB required"
                path = [string]$space.path; root = $label; free_bytes = [int64]$space.free_bytes; required_free_bytes = [int64]$space.required_free_bytes
            })
    }
    $hardFindings = @($findings.ToArray())
    [pscustomobject]@{ schema_version = 1; status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }; run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }; findings = @($findings.ToArray()); disk_space_checks = @($hostChecks); docker_storage_checks = @($dockerChecks); generated_at = (Get-Date).ToString("o") }
}

function New-TaskspaceHarnessAbortSummaryLines {
    param([string]$Title, [string]$Phase, $Finding, $Signature, [string]$HealthPath)
    $lines = @("# $Title", "", "- run_validity: invalid_harness", "- abort_phase: $Phase", "- reason: $($Finding.message)", "- infra_signature: $($Signature.key)", "- harness_health: $HealthPath")
    if ([string]$Finding.stable_code -eq "disk_space_low") {
        $lines += @(
            "- failed_root: $($Finding.root)",
            "- failed_path: $($Finding.path)",
            "- free_gib: $([math]::Round([int64]$Finding.free_bytes / 1GB, 2))",
            "- required_gib: $([math]::Round([int64]$Finding.required_free_bytes / 1GB, 2))",
            "- override_bytes_env: TASKSPACE_MIN_FREE_BYTES",
            "- override_gib_env: TASKSPACE_MIN_FREE_GIB",
            "- likely_cleanup_paths: run root, repo target directories, Docker/WSL data root, D:\whale-docker"
        )
    } elseif ([string]$Finding.stable_code -eq "disk_space_threshold_invalid") {
        $lines += @(
            "- override_bytes_env: TASKSPACE_MIN_FREE_BYTES",
            "- override_gib_env: TASKSPACE_MIN_FREE_GIB"
        )
    }
    $lines
}

function Get-TaskspaceRepoPathLatestCommitInfo {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$PathSpec
    )
    try {
        $hash = (& git -C $RepoRoot log -1 --format=%H -- $PathSpec 2>$null)
        $epochText = (& git -C $RepoRoot log -1 --format=%ct -- $PathSpec 2>$null)
        if ([string]::IsNullOrWhiteSpace([string]$hash) -or [string]::IsNullOrWhiteSpace([string]$epochText)) {
            return $null
        }
        $epoch = [int64]([string]$epochText).Trim()
        [pscustomobject]@{
            hash = ([string]$hash).Trim()
            epoch = $epoch
            time_utc = ([DateTimeOffset]::FromUnixTimeSeconds($epoch).UtcDateTime.ToString("o"))
            pathspec = $PathSpec
        }
    } catch {
        $null
    }
}

function Get-TaskspaceWhaleBinaryAttestationPath {
    param([Parameter(Mandatory = $true)][string]$WhaleBin)
    "$WhaleBin.build-attestation.json"
}

function Get-TaskspaceSha256Text {
    param([AllowEmptyString()][string]$Text = "")
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        ([BitConverter]::ToString($hasher.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $hasher.Dispose()
    }
}

function Get-TaskspaceGitBuildIdentity {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    $head = ((& git -C $RepoRoot rev-parse HEAD 2>$null) | Select-Object -First 1).Trim()
    $headTree = ((& git -C $RepoRoot rev-parse "HEAD^{tree}" 2>$null) | Select-Object -First 1).Trim()
    $codexTree = ((& git -C $RepoRoot rev-parse "HEAD:third_party/codex-cli" 2>$null) | Select-Object -First 1).Trim()
    $dirty = @(& git -C $RepoRoot status --porcelain --untracked-files=all 2>$null)
    $codexDirty = @(
        & git -C $RepoRoot status --porcelain --untracked-files=all -- third_party/codex-cli 2>$null
    )
    if ($LASTEXITCODE -ne 0 -or
        $head -notmatch '^[0-9a-f]{40,64}$' -or
        $headTree -notmatch '^[0-9a-f]{40,64}$' -or
        $codexTree -notmatch '^[0-9a-f]{40,64}$') {
        throw "Cannot resolve clean Git build identity for $RepoRoot"
    }
    [pscustomobject]@{
        current_git_head = $head
        head_tree_id = $headTree
        codex_tree_id = $codexTree
        worktree_clean = $dirty.Count -eq 0
        codex_worktree_clean = $codexDirty.Count -eq 0
        dirty_paths = @($dirty)
        codex_dirty_paths = @($codexDirty)
    }
}

function Get-TaskspaceWhaleVersionProbe {
    param([Parameter(Mandatory = $true)][string]$WhaleBin)
    $result = Invoke-TaskspaceShortCommand $WhaleBin @("--version") 20
    $output = (@($result.output) -join "`n").Trim()
    [pscustomobject]@{
        argv = @($WhaleBin, "--version")
        exit_code = [int]$result.exit_code
        output = $output
        output_sha256 = Get-TaskspaceSha256Text $output
    }
}

function Get-TaskspaceWhaleBinaryAttestation {
    param(
        [Parameter(Mandatory = $true)][string]$WhaleBin,
        [Parameter(Mandatory = $true)][string]$ExpectedBinarySha256,
        [Parameter(Mandatory = $true)][string]$ExpectedSourceCommit,
        [Parameter(Mandatory = $true)][string]$RepoRoot
    )
    $path = Get-TaskspaceWhaleBinaryAttestationPath $WhaleBin
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return [pscustomobject]@{ status = "missing"; path = $path; reason = "attestation_missing"; marker = $null }
    }
    try {
        $marker = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{ status = "invalid"; path = $path; reason = "attestation_malformed"; marker = $null }
    }
    $repoRootFull = try { [System.IO.Path]::GetFullPath($RepoRoot) } catch { $RepoRoot }
    $markerRepoRoot = try { [System.IO.Path]::GetFullPath([string]$marker.repo_root) } catch { [string]$marker.repo_root }
    $binaryShaMatches = ([string]$marker.whale_binary_sha256).ToLowerInvariant() -eq ([string]$ExpectedBinarySha256).ToLowerInvariant()
    $sourceMatches = [string]$marker.codex_source_latest_commit -eq [string]$ExpectedSourceCommit
    $repoMatches = [string]::Equals($markerRepoRoot, $repoRootFull, [System.StringComparison]::OrdinalIgnoreCase)
    $schemaMatches = [int]$marker.schema_version -eq 2
    $statusMatches = [string]$marker.status -eq "pass"
    $gitIdentity = try { Get-TaskspaceGitBuildIdentity $RepoRoot } catch { $null }
    $gitMatches = $null -ne $gitIdentity -and
        [bool]$gitIdentity.codex_worktree_clean -and
        [bool]$marker.worktree_clean -and
        [string]$marker.codex_tree_id -eq [string]$gitIdentity.codex_tree_id
    $buildCommandMatches = -not [string]::IsNullOrWhiteSpace([string]$marker.build_command)
    $probe = try { Get-TaskspaceWhaleVersionProbe $WhaleBin } catch { $null }
    $markerProbe = $marker.executable_probe
    $markerProbeSha256 = if ($null -ne $markerProbe -and
        $markerProbe.PSObject.Properties.Name -contains "output_sha256") {
        [string]$markerProbe.output_sha256
    } elseif ($null -ne $markerProbe) {
        Get-TaskspaceSha256Text ([string]$markerProbe.output)
    } else {
        ""
    }
    $probeMatches = $null -ne $probe -and $null -ne $markerProbe -and
        [int]$probe.exit_code -eq 0 -and
        [int]$markerProbe.exit_code -eq 0 -and
        -not [string]::IsNullOrWhiteSpace([string]$probe.output) -and
        $markerProbeSha256 -eq [string]$probe.output_sha256
    if ($schemaMatches -and $statusMatches -and $binaryShaMatches -and $sourceMatches -and
        $repoMatches -and $gitMatches -and $buildCommandMatches -and $probeMatches) {
        return [pscustomobject]@{ status = "pass"; path = $path; reason = ""; marker = $marker }
    }
    $reasons = New-Object System.Collections.Generic.List[string]
    if (-not $schemaMatches) { $reasons.Add("schema_mismatch") }
    if (-not $statusMatches) { $reasons.Add("status_not_pass") }
    if (-not $binaryShaMatches) { $reasons.Add("binary_sha_mismatch") }
    if (-not $sourceMatches) { $reasons.Add("codex_source_commit_mismatch") }
    if (-not $repoMatches) { $reasons.Add("repo_root_mismatch") }
    if (-not $gitMatches) { $reasons.Add("git_build_identity_mismatch") }
    if (-not $buildCommandMatches) { $reasons.Add("build_command_missing") }
    if (-not $probeMatches) { $reasons.Add("executable_probe_mismatch") }
    [pscustomobject]@{ status = "invalid"; path = $path; reason = (@($reasons.ToArray()) -join ","); marker = $marker }
}

function Write-TaskspaceWhaleBinaryAttestation {
    param(
        [Parameter(Mandatory = $true)][string]$WhaleBin,
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [string]$BuildCommand = ""
    )
    if (-not (Test-Path -LiteralPath $WhaleBin -PathType Leaf)) {
        throw "Whale binary does not exist: $WhaleBin"
    }
    if ([string]::IsNullOrWhiteSpace($BuildCommand)) {
        throw "BuildCommand is required for a binary attestation."
    }
    $item = Get-Item -LiteralPath $WhaleBin
    $sourceInfo = Get-TaskspaceRepoPathLatestCommitInfo $RepoRoot "third_party/codex-cli"
    if (-not $sourceInfo) { throw "Cannot resolve latest third_party/codex-cli commit." }
    $gitIdentity = Get-TaskspaceGitBuildIdentity $RepoRoot
    if (-not [bool]$gitIdentity.worktree_clean) {
        throw "Cannot attest a binary from a dirty worktree."
    }
    $probe = Get-TaskspaceWhaleVersionProbe $item.FullName
    if ([int]$probe.exit_code -ne 0 -or [string]::IsNullOrWhiteSpace([string]$probe.output)) {
        throw "Whale binary executable probe failed: $($probe.exit_code)"
    }
    $binarySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash.ToLowerInvariant()
    $attestationPath = Get-TaskspaceWhaleBinaryAttestationPath $item.FullName
    [pscustomobject]@{
        schema_version = 2
        status = "pass"
        producer = "write-whale-binary-attestation.ps1"
        repo_root = [System.IO.Path]::GetFullPath($RepoRoot)
        current_git_head = [string]$gitIdentity.current_git_head
        head_tree_id = [string]$gitIdentity.head_tree_id
        codex_tree_id = [string]$gitIdentity.codex_tree_id
        worktree_clean = $true
        codex_source_latest_commit = [string]$sourceInfo.hash
        codex_source_latest_commit_time_utc = [string]$sourceInfo.time_utc
        whale_bin = $item.FullName
        whale_binary_sha256 = $binarySha256
        whale_binary_last_write_utc = $item.LastWriteTimeUtc.ToUniversalTime().ToString("o")
        build_command = $BuildCommand
        executable_probe = $probe
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $attestationPath -Encoding UTF8
    $attestationPath
}

function New-TaskspaceWhaleBinaryHealth {
    param(
        [Parameter(Mandatory = $true)][string]$WhaleBin,
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [switch]$AllowStale
    )
    $findings = New-Object System.Collections.Generic.List[object]
    $resolvedPath = ""
    $exists = Test-Path -LiteralPath $WhaleBin -PathType Leaf
    $binaryEpoch = 0L
    $binarySha256 = ""
    $binaryLastWriteUtc = ""
    if ($exists) {
        $item = Get-Item -LiteralPath $WhaleBin
        $resolvedPath = $item.FullName
        $binaryEpoch = [DateTimeOffset]$item.LastWriteTimeUtc.ToUniversalTime()
        $binaryEpoch = $binaryEpoch.ToUnixTimeSeconds()
        $binaryLastWriteUtc = $item.LastWriteTimeUtc.ToUniversalTime().ToString("o")
        try { $binarySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash.ToLowerInvariant() } catch { $binarySha256 = "" }
    } else {
        $findings.Add([pscustomobject]@{
                severity = "fail"
                stable_code = "whale_binary_missing"
                message = "Whale binary does not exist: $WhaleBin"
                path = $WhaleBin
                stage = "whale_binary_preflight"
            })
    }
    $sourceInfo = Get-TaskspaceRepoPathLatestCommitInfo $RepoRoot "third_party/codex-cli"
    $headHash = ""
    try { $headHash = ((& git -C $RepoRoot rev-parse HEAD 2>$null) | Select-Object -First 1).Trim() } catch { $headHash = "" }
    $stale = $false
    $attestation = $null
    if ($exists -and $sourceInfo) {
        $attestation = Get-TaskspaceWhaleBinaryAttestation $resolvedPath $binarySha256 ([string]$sourceInfo.hash) $RepoRoot
        if ([string]$attestation.status -ne "pass") {
            $findings.Add([pscustomobject]@{
                    severity = "fail"
                    stable_code = "whale_binary_attestation_invalid"
                    message = "Whale binary provenance is not proven by a matching build attestation."
                    path = $resolvedPath
                    stage = "whale_binary_preflight"
                    source_commit = [string]$sourceInfo.hash
                    attestation_path = [string]$attestation.path
                    attestation_status = [string]$attestation.status
                    attestation_reason = [string]$attestation.reason
                })
        }
    }
    if ($exists -and $sourceInfo -and $binaryEpoch -lt [int64]$sourceInfo.epoch) {
        if ($attestation -and [string]$attestation.status -eq "pass") {
            $findings.Add([pscustomobject]@{
                    severity = "info"
                    stable_code = "whale_binary_stale_mtime_attested"
                    message = "Whale binary mtime is older than codex source, but matching binary attestation proves the binary was built for the current codex source commit."
                    path = $resolvedPath
                    stage = "whale_binary_preflight"
                    binary_last_write_utc = $binaryLastWriteUtc
                    source_commit = [string]$sourceInfo.hash
                    source_commit_time_utc = [string]$sourceInfo.time_utc
                    attestation_path = [string]$attestation.path
                })
        } else {
            $stale = $true
            $attestationReason = if ($attestation) { [string]$attestation.reason } else { "attestation_unavailable" }
            $findings.Add([pscustomobject]@{
                    severity = "warn"
                    stable_code = "whale_binary_stale_for_codex_source"
                    message = "Whale binary mtime is older than the latest codex source commit; the provenance failure is reported by whale_binary_attestation_invalid."
                    path = $resolvedPath
                    stage = "whale_binary_preflight"
                    binary_last_write_utc = $binaryLastWriteUtc
                    source_commit = [string]$sourceInfo.hash
                    source_commit_time_utc = [string]$sourceInfo.time_utc
                    attestation_path = if ($attestation) { [string]$attestation.path } else { Get-TaskspaceWhaleBinaryAttestationPath $resolvedPath }
                    attestation_status = if ($attestation) { [string]$attestation.status } else { "unavailable" }
                    attestation_reason = $attestationReason
                })
        }
    }
    $hardFindings = @($findings.ToArray() | Where-Object { [string]$_.severity -eq "fail" })
    [pscustomobject]@{
        schema_version = 1
        status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }
        run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }
        whale_bin_requested = $WhaleBin
        whale_bin_resolved = $resolvedPath
        whale_binary_exists = $exists
        whale_binary_last_write_utc = $binaryLastWriteUtc
        whale_binary_sha256 = $binarySha256
        whale_binary_epoch = $binaryEpoch
        current_git_head = $headHash
        codex_source_latest_commit = $sourceInfo
        stale_for_codex_source = $stale
        stale_allowed = [bool]$AllowStale
        build_attestation_path = if ($attestation) { [string]$attestation.path } elseif ($exists) { Get-TaskspaceWhaleBinaryAttestationPath $resolvedPath } else { "" }
        build_attestation_status = if ($attestation) { [string]$attestation.status } else { "" }
        build_attestation_reason = if ($attestation) { [string]$attestation.reason } else { "" }
        findings = @($findings.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
}

function Get-TaskspaceHarnessTextSignature {
    param(
        [AllowEmptyString()][string]$Text = "",
        [string]$Stage = "validator_pretest",
        [string]$Side = "",
        [string]$Artifact = ""
    )
    if ($Text -match "docker command is required|Docker backend unavailable|Requested WSL Docker backend is unavailable|Requested native Docker backend is unavailable|Unsupported TASKSPACE_DOCKER_BACKEND|getpwnam\(root\) failed|getpwuid\(0\) failed|Wsl/Service/E_UNEXPECTED|I/O error @util\.cpp") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "docker_backend_unavailable" "Docker backend unavailable" $Side $Artifact
    }
    if ($Text -match "Resolve-Path|Cannot find path|PathNotFound") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "path_unresolvable" "Path resolution failed" $Side $Artifact
    }
    if ($Text -match "run-tests script not found|validator script not found") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "validator_source_missing" "Validator source missing" $Side $Artifact
    }
    if ($Text -match "uv[-_ ]cache|uv-x86_64|install\.sh") {
        return New-TaskspaceInfraSignature "harness_materialization_failure" $Stage "uv_cache_missing" "uv cache unavailable" $Side $Artifact
    }
    return $null
}

function Get-TaskspaceValidationText {
    param($Validation)
    $combined = ""
    foreach ($path in @($Validation.stdout_path, $Validation.stderr_path)) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            $combined += "`n" + (Get-Content -Raw -Encoding UTF8 -LiteralPath $path)
        }
    }
    $combined
}

function Get-TaskspaceValidatorProbeResult {
    param($Validation)
    $combined = Get-TaskspaceValidationText $Validation
    $probePath = ""
    $probeMatch = [regex]::Match($combined, "(?m)^validator_probe_result_path=(.+)$")
    if ($probeMatch.Success) { $probePath = $probeMatch.Groups[1].Value.Trim() }
    $json = if ($probePath -and (Test-Path -LiteralPath $probePath)) {
        try { Get-Content -Raw -Encoding UTF8 -LiteralPath $probePath | ConvertFrom-Json } catch { $null }
    } else { $null }
    [pscustomobject]@{
        path = $probePath
        json = $json
    }
}

function Get-TaskspaceValidationLifecycle {
    param($Validation)
    $combined = Get-TaskspaceValidationText $Validation
    $stages = @([regex]::Matches($combined, "(?m)^validator_lifecycle_stage=([^\r\n]+)\s*$") | ForEach-Object { $_.Groups[1].Value.Trim() })
    $stage = if ($stages.Count -gt 0) { [string]$stages[-1] } else { "unknown" }
    $timeoutPhaseMatch = [regex]::Match($combined, "(?m)^taskspace_validation_timeout_phase=([^\r\n]+)\s*$")
    $testsStartedAtMatch = [regex]::Match($combined, "(?m)^taskspace_tests_started_at=([^\r\n]+)\s*$")
    $testsCompletedAtMatch = [regex]::Match($combined, "(?m)^taskspace_tests_completed_at=([^\r\n]+)\s*$")
    [pscustomobject]@{
        tests_started_seen = ($combined -match "(?m)^validator_tests_started=true\s*$")
        tests_completed_seen = ($combined -match "(?m)^validator_tests_completed=true\s*$")
        validation_lifecycle_stage = $stage
        validation_timeout_phase = if ($timeoutPhaseMatch.Success) { $timeoutPhaseMatch.Groups[1].Value.Trim() } else { "" }
        tests_started_at = if ($testsStartedAtMatch.Success) { $testsStartedAtMatch.Groups[1].Value.Trim() } else { "" }
        tests_completed_at = if ($testsCompletedAtMatch.Success) { $testsCompletedAtMatch.Groups[1].Value.Trim() } else { "" }
    }
}

function Get-TaskspaceInfraSignatureFromMetrics {
    param($Metrics)
    if ($null -eq $Metrics) { return $null }
    if ($Metrics.PSObject.Properties.Name -contains "infra_signature" -and $Metrics.infra_signature) {
        return $Metrics.infra_signature
    }
    foreach ($failure in @($Metrics.validator_environment_failures)) {
        if ([string]::IsNullOrWhiteSpace([string]$failure)) { continue }
        if ([string]$failure -match "docker") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "docker_backend_unavailable" "Docker backend unavailable" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "path_unresolvable|Resolve-Path") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "path_unresolvable" "Path resolution failed" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "uv_cache") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "uv_cache_missing" "uv cache unavailable" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
        if ([string]$failure -match "validator_source") { return New-TaskspaceInfraSignature "harness_materialization_failure" "validator_pretest" "validator_source_missing" "Validator source missing" ([string]$Metrics.mode) ([string]$Metrics.validation_stderr_path) }
    }
    $null
}

function Test-TaskspaceHardInfraSignature {
    param($Signature)
    if ($null -eq $Signature) { return $false }
    [string]$Signature.stable_code -in @("relative_materialized_path", "path_unresolvable", "validator_source_missing", "uv_cache_missing", "docker_backend_unavailable", "runtime_manifest_missing", "validator_probe_failed", "workspace_baseline_git_failed", "workspace_fixture_copy_failed", "workspace_materialization_failed", "disk_space_low", "disk_space_threshold_invalid", "benchmark_artifact_scan_failed", "benchmark_artifact_limit_invalid", "benchmark_run_artifact_limit_exceeded", "benchmark_repository_artifact_limit_exceeded", "benchmark_nested_build_cache_detected")
}

function Get-TaskspaceSentinelAbortDecision {
    param(
        [Parameter(Mandatory = $true)]$StandardMetrics,
        [Parameter(Mandatory = $true)]$TaskspaceMetrics
    )
    $standardPretest = ($StandardMetrics.PSObject.Properties.Name -contains "pretest_failure" -and [bool]$StandardMetrics.pretest_failure)
    $taskspacePretest = ($TaskspaceMetrics.PSObject.Properties.Name -contains "pretest_failure" -and [bool]$TaskspaceMetrics.pretest_failure)
    $standardSig = Get-TaskspaceInfraSignatureFromMetrics $StandardMetrics
    $taskspaceSig = Get-TaskspaceInfraSignatureFromMetrics $TaskspaceMetrics
    $standardHard = $standardPretest -and (Test-TaskspaceHardInfraSignature $standardSig)
    $taskspaceHard = $taskspacePretest -and (Test-TaskspaceHardInfraSignature $taskspaceSig)
    $sameKey = ($standardSig -and $taskspaceSig -and [string]$standardSig.key -eq [string]$taskspaceSig.key)
    if ($standardHard -and $taskspaceHard -and $sameKey) {
        return [pscustomobject]@{ abort = $true; reason = "same_infra_signature_both_sides"; signature = $standardSig }
    }
    if ($standardHard) {
        return [pscustomobject]@{ abort = $true; reason = "standard_pretest_infra_failure"; signature = $standardSig }
    }
    if ($taskspaceHard) {
        return [pscustomobject]@{ abort = $true; reason = "taskspace_pretest_infra_failure"; signature = $taskspaceSig }
    }
    [pscustomobject]@{ abort = $false; reason = ""; signature = $null }
}

function Write-TaskspaceHarnessHealth {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Health
    )
    ($Health | ConvertTo-Json -Depth 20) | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-TaskspaceHarnessHealth {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$RunDir,
        [string]$ScenarioBaseDir = ""
    )
    $findings = New-Object System.Collections.Generic.List[object]
    $checked = New-Object System.Collections.Generic.List[object]
    foreach ($pathInfo in @(
        @{ name = "prompt_path"; path = [string]$Manifest.PromptPath; required = $true },
        @{ name = "fixture_dir"; path = [string]$Manifest.FixtureDir; required = $true }
    )) {
        $exists = -not [string]::IsNullOrWhiteSpace($pathInfo.path) -and (Test-Path -LiteralPath $pathInfo.path)
        $checked.Add([pscustomobject]@{ name = $pathInfo.name; path = $pathInfo.path; exists = $exists; fully_qualified = (Test-TaskspaceFullyQualifiedPath $pathInfo.path) })
        if ($pathInfo.required -and -not $exists) {
            $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "path_unresolvable"; message = "$($pathInfo.name) is missing"; path = $pathInfo.path })
        }
    }
    $external = $Manifest.ExternalBenchmark
    if ($external -and $external.PSObject.Properties.Name -contains "adapter_metadata") {
        $meta = $external.adapter_metadata
        foreach ($prop in @("uv_cache_root", "validator_source_dir", "fixture_source")) {
            if (-not ($meta.PSObject.Properties.Name -contains $prop)) { continue }
            $path = [string]$meta.$prop
            $exists = -not [string]::IsNullOrWhiteSpace($path) -and (Test-Path -LiteralPath $path)
            $fq = Test-TaskspaceFullyQualifiedPath $path
            $checked.Add([pscustomobject]@{ name = $prop; path = $path; exists = $exists; fully_qualified = $fq })
            if (-not $fq) {
                $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "relative_materialized_path"; message = "$prop must be absolute"; path = $path })
            } elseif (-not $exists) {
                $code = if ($prop -eq "uv_cache_root") { "uv_cache_missing" } elseif ($prop -eq "validator_source_dir") { "validator_source_missing" } else { "path_unresolvable" }
                $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = $code; message = "$prop is missing"; path = $path })
            }
        }
    }
    $spacePaths = New-Object System.Collections.Generic.List[string]
    $spacePaths.Add($RunDir)
    if (-not [string]::IsNullOrWhiteSpace($ScenarioBaseDir)) { $spacePaths.Add($ScenarioBaseDir) }
    foreach ($pathInfo in @($checked.ToArray())) {
        if (-not [string]::IsNullOrWhiteSpace([string]$pathInfo.path)) { $spacePaths.Add([string]$pathInfo.path) }
    }
    $diskHealth = New-TaskspaceDiskHealth @($spacePaths.ToArray()) "preflight"
    foreach ($finding in @($diskHealth.findings)) { $findings.Add($finding) }
    $hardFindings = @($findings.ToArray() | Where-Object { [string]$_.severity -eq "fail" })
    [pscustomobject]@{
        schema_version = 1
        status = if ($hardFindings.Count -gt 0) { "fail" } else { "pass" }
        run_validity = if ($hardFindings.Count -gt 0) { "invalid_harness" } else { "valid" }
        findings = @($findings.ToArray())
        checked_paths = @($checked.ToArray())
        disk_space_checks = @($diskHealth.disk_space_checks)
        docker_storage_checks = @($diskHealth.docker_storage_checks)
        generated_at = (Get-Date).ToString("o")
        run_dir = $RunDir
        scenario_base_dir = $ScenarioBaseDir
    }
}
