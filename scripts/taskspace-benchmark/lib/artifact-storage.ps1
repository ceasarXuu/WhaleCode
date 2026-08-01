function Get-TaskspaceArtifactBuildCacheNames {
    @("debug", "release", "dev", "dev-small", "dist", "ci-test", "doc", "package")
}

function Get-TaskspaceArtifactLimit {
    param(
        [Parameter(Mandatory = $true)][string]$BytesEnvironmentVariable,
        [Parameter(Mandatory = $true)][string]$GibEnvironmentVariable,
        [Parameter(Mandatory = $true)][int64]$DefaultBytes
    )
    $bytesValue = [Environment]::GetEnvironmentVariable($BytesEnvironmentVariable)
    $gibValue = [Environment]::GetEnvironmentVariable($GibEnvironmentVariable)
    try {
        if (-not [string]::IsNullOrWhiteSpace($bytesValue)) {
            $value = [int64]$bytesValue
            if ($value -lt 1) { throw "must be positive" }
            return [pscustomobject]@{ valid = $true; bytes = $value; source = $BytesEnvironmentVariable; error = "" }
        }
        if (-not [string]::IsNullOrWhiteSpace($gibValue)) {
            $value = [int64]([double]$gibValue * 1GB)
            if ($value -lt 1) { throw "must be positive" }
            return [pscustomobject]@{ valid = $true; bytes = $value; source = $GibEnvironmentVariable; error = "" }
        }
    } catch {
        return [pscustomobject]@{
            valid = $false
            bytes = $DefaultBytes
            source = if (-not [string]::IsNullOrWhiteSpace($bytesValue)) { $BytesEnvironmentVariable } else { $GibEnvironmentVariable }
            error = [string]$_.Exception.Message
        }
    }
    [pscustomobject]@{ valid = $true; bytes = $DefaultBytes; source = "default"; error = "" }
}

function Get-TaskspacePathStorageFacts {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ bytes = [int64]0; latest_write_time_utc = [datetime]::MinValue }
    }
    $rootItem = Get-Item -Force -LiteralPath $Path
    [int64]$bytes = 0
    $latest = $rootItem.LastWriteTimeUtc
    if (-not $rootItem.PSIsContainer) { $bytes = [int64]$rootItem.Length }
    if ($rootItem.PSIsContainer) {
        Get-ChildItem -Force -Recurse -LiteralPath $Path -ErrorAction Stop | ForEach-Object {
            $item = $_
            if (-not $item.PSIsContainer) { $bytes += [int64]$item.Length }
            if ($item.LastWriteTimeUtc -gt $latest) { $latest = $item.LastWriteTimeUtc }
        }
    }
    [pscustomobject]@{ bytes = $bytes; latest_write_time_utc = $latest }
}

function Get-TaskspacePathSizeBytes {
    param([Parameter(Mandatory = $true)][string]$Path)
    [int64](Get-TaskspacePathStorageFacts $Path).bytes
}

function Get-TaskspaceRepositoryArtifactUsage {
    param([Parameter(Mandatory = $true)][string]$TargetRoot)
    $protectedNames = @(Get-TaskspaceArtifactBuildCacheNames)
    [int64]$artifactBytes = 0
    $artifactEntries = [Collections.Generic.List[object]]::new()
    $protectedEntries = [Collections.Generic.List[object]]::new()
    if (Test-Path -LiteralPath $TargetRoot -PathType Container) {
        foreach ($entry in @(Get-ChildItem -Force -LiteralPath $TargetRoot)) {
            if ($protectedNames -contains $entry.Name -or $entry.Name -in @(".rustc_info.json", "CACHEDIR.TAG", ".storage-reports")) {
                $protectedEntries.Add([pscustomobject]@{ name = $entry.Name; path = $entry.FullName; reason = "build_cache_or_storage_metadata" })
                continue
            }
            $entryBytes = Get-TaskspacePathSizeBytes $entry.FullName
            $artifactBytes += $entryBytes
            $artifactEntries.Add([pscustomobject]@{ name = $entry.Name; path = $entry.FullName; bytes = $entryBytes })
        }
    }
    [pscustomobject]@{
        target_root = [IO.Path]::GetFullPath($TargetRoot)
        artifact_bytes = $artifactBytes
        artifact_entries = @($artifactEntries.ToArray())
        protected_entries = @($protectedEntries.ToArray())
    }
}

function Find-TaskspaceNestedBuildCaches {
    param([Parameter(Mandatory = $true)][string]$RunRoot)
    if (-not (Test-Path -LiteralPath $RunRoot -PathType Container)) { return @() }
    $root = [IO.Path]::GetFullPath($RunRoot).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    $cachePaths = [Collections.Generic.List[string]]::new()
    Get-ChildItem -Force -Directory -Recurse -LiteralPath $root -ErrorAction Stop | ForEach-Object {
        $directory = $_
        $relative = [IO.Path]::GetRelativePath($root, $directory.FullName).Replace('\', '/')
        if ($relative -match '(^|/)third_party/codex-cli/codex-rs/target$') {
            $cachePaths.Add($directory.FullName)
        }
    }
    @($cachePaths.ToArray() | Sort-Object -Unique)
}

function New-TaskspaceArtifactStorageHealth {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [string]$Stage = "preflight"
    )
    $runLimit = Get-TaskspaceArtifactLimit "TASKSPACE_MAX_RUN_ARTIFACT_BYTES" "TASKSPACE_MAX_RUN_ARTIFACT_GIB" ([int64](24GB))
    $repoLimit = Get-TaskspaceArtifactLimit "TASKSPACE_MAX_REPO_ARTIFACT_BYTES" "TASKSPACE_MAX_REPO_ARTIFACT_GIB" ([int64](64GB))
    $targetRoot = Join-Path $RepoRoot "target"
    $findings = [Collections.Generic.List[object]]::new()
    try {
        $runBytes = Get-TaskspacePathSizeBytes $RunRoot
        $repoUsage = Get-TaskspaceRepositoryArtifactUsage $targetRoot
        $nestedBuildCaches = @(Find-TaskspaceNestedBuildCaches $RunRoot)
    } catch {
        $runBytes = 0
        $repoUsage = [pscustomobject]@{ target_root = $targetRoot; artifact_bytes = 0; artifact_entries = @(); protected_entries = @() }
        $nestedBuildCaches = @()
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "benchmark_artifact_scan_failed"; message = [string]$_.Exception.Message; path = $RunRoot; stage = $Stage })
    }
    foreach ($limit in @($runLimit, $repoLimit) | Where-Object { -not [bool]$_.valid }) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "benchmark_artifact_limit_invalid"; message = "Invalid artifact limit in $($limit.source): $($limit.error)"; path = $limit.source; stage = $Stage })
    }
    if ($runBytes -gt [int64]$runLimit.bytes) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "benchmark_run_artifact_limit_exceeded"; message = "Benchmark run artifacts exceed the configured limit"; path = $RunRoot; stage = $Stage; actual_bytes = $runBytes; limit_bytes = [int64]$runLimit.bytes })
    }
    if ([int64]$repoUsage.artifact_bytes -gt [int64]$repoLimit.bytes) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "benchmark_repository_artifact_limit_exceeded"; message = "Repository benchmark artifacts exceed the configured limit"; path = $targetRoot; stage = $Stage; actual_bytes = [int64]$repoUsage.artifact_bytes; limit_bytes = [int64]$repoLimit.bytes })
    }
    foreach ($path in $nestedBuildCaches) {
        $findings.Add([pscustomobject]@{ severity = "fail"; stable_code = "benchmark_nested_build_cache_detected"; message = "Benchmark artifacts contain a copied Cargo target tree"; path = $path; stage = $Stage })
    }
    [pscustomobject]@{
        schema_version = 1
        status = if ($findings.Count -gt 0) { "fail" } else { "pass" }
        run_validity = if ($findings.Count -gt 0) { "invalid_harness" } else { "valid" }
        stage = $Stage
        run_root = [IO.Path]::GetFullPath($RunRoot)
        run_artifact_bytes = $runBytes
        run_limit = $runLimit
        repository_usage = $repoUsage
        repository_limit = $repoLimit
        nested_build_cache_paths = $nestedBuildCaches
        findings = @($findings.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
}

function New-TaskspaceArtifactCleanupPlan {
    param(
        [Parameter(Mandatory = $true)][string]$TargetRoot,
        [string[]]$KeepName = @(),
        [int]$MinimumAgeDays = 0
    )
    $protectedNames = @(Get-TaskspaceArtifactBuildCacheNames) + @(".rustc_info.json", "CACHEDIR.TAG", ".storage-reports") + @($KeepName)
    $cutoff = (Get-Date).AddDays(-1 * $MinimumAgeDays)
    $entries = [Collections.Generic.List[object]]::new()
    if (Test-Path -LiteralPath $TargetRoot -PathType Container) {
        foreach ($entry in @(Get-ChildItem -Force -LiteralPath $TargetRoot | Sort-Object Name)) {
            $protected = $protectedNames -contains $entry.Name
            $facts = Get-TaskspacePathStorageFacts $entry.FullName
            $latestWriteTime = [datetime]$facts.latest_write_time_utc
            $oldEnough = $MinimumAgeDays -eq 0 -or $latestWriteTime -lt $cutoff.ToUniversalTime()
            $entries.Add([pscustomobject]@{
                    name = $entry.Name
                    path = $entry.FullName
                    bytes = [int64]$facts.bytes
                    latest_write_time = $latestWriteTime.ToString("o")
                    action = if ($protected) { "protect" } elseif (-not $oldEnough) { "retain_recent" } else { "remove" }
                    reason = if ($protected) { "build_cache_metadata_or_explicit_keep" } elseif (-not $oldEnough) { "younger_than_minimum_age" } else { "benchmark_artifact" }
                })
        }
    }
    [pscustomobject]@{
        schema_version = 1
        target_root = [IO.Path]::GetFullPath($TargetRoot)
        minimum_age_days = $MinimumAgeDays
        keep_names = @($KeepName)
        entries = @($entries.ToArray())
        removable_bytes = [int64](($entries | Where-Object action -eq "remove" | Measure-Object bytes -Sum).Sum)
        generated_at = (Get-Date).ToString("o")
    }
}
