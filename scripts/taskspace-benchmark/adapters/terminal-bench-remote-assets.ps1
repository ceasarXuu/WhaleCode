function Get-TerminalBenchStringSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
}

function Test-TerminalBenchValidatorRuntimePath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    $path = $RelativePath.Replace("\", "/")
    $path -eq "run-tests.sh" -or $path -eq "verify.sh" -or $path -eq "test.sh" -or $path.StartsWith("tests/")
}

function Test-TerminalBenchRemoteAssetScanPath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    $path = $RelativePath.Replace("\", "/")
    if ($path -eq "solution.sh" -or $path -eq "solution.yaml") { return $false }
    $true
}

function Get-TerminalBenchRemoteAssetKind {
    param(
        [Parameter(Mandatory = $true)][string]$Line,
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$RelativePath
    )
    $trimmed = $Line.Trim()
    if ($trimmed.StartsWith("#")) { return "metadata_comment" }
    $uri = [uri]$Url
    $leaf = Split-Path -Leaf $uri.AbsolutePath
    $fileLikeLeaf = $leaf -match '(?i)\.(zip|tar|tgz|gz|xz|bz2|sqlite|db|csv|json|jsonl|parquet|whl|sh|py|bin|txt)$'
    if ($RelativePath -eq "Dockerfile" -and $trimmed -match '^(ENV|ARG)\b' -and -not $fileLikeLeaf) { return "registry_or_source_endpoint" }
    if ($RelativePath -eq "Dockerfile" -and $trimmed -match '^(LABEL)\b') { return "registry_or_source_endpoint" }
    $escaped = [regex]::Escape($Url)
    if ($Line -match "(?i)\b(curl|wget)\b.*\b(-o|-O|--output)\b.*$escaped") { return "materialized_file_asset" }
    if ($Line -match "(?i)\b(curl|wget)\b.*$escaped.*\b(-o|-O|--output)\b") { return "materialized_file_asset" }
    if ($RelativePath -eq "Dockerfile" -and $Line -match "(?i)^\s*(ADD|COPY)\s+[`"']?$escaped") { return "materialized_file_asset" }
    if ($Line -match "(?i)\b(curl|wget)\b" -or $fileLikeLeaf) { return "unknown_runtime_network_dependency" }
    "registry_or_source_endpoint"
}

function Get-TerminalBenchRemoteAssets {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$OutputRoot,
        [Parameter(Mandatory = $true)][string]$SourceVersion,
        [object]$UvCache = $null
    )
    $rows = New-Object System.Collections.Generic.List[object]
    $cacheRoot = Join-Path $OutputRoot "_asset-cache"
    $urlPattern = "https?://[^\s`"'<>\|]+"
    $coveredUrls = @{}
    if ($null -ne $UvCache) {
        foreach ($pair in @(
                @([string]$UvCache.installer_url, [string]$UvCache.installer_path, [string]$UvCache.installer_sha256, [int64]$UvCache.installer_size_bytes),
                @([string]$UvCache.installer_alias_url, [string]$UvCache.installer_path, [string]$UvCache.installer_sha256, [int64]$UvCache.installer_size_bytes),
                @([string]$UvCache.archive_url, [string]$UvCache.archive_path, [string]$UvCache.archive_sha256, [int64]$UvCache.archive_size_bytes)
            )) {
            if (-not [string]::IsNullOrWhiteSpace($pair[0])) {
                $coveredUrls[$pair[0]] = [pscustomobject]@{ path = $pair[1]; sha256 = $pair[2]; size = $pair[3] }
            }
        }
    }
    foreach ($file in @(Get-ChildItem -LiteralPath $TaskRoot -Recurse -File -Force -ErrorAction SilentlyContinue)) {
        $relative = $file.FullName.Substring($TaskRoot.Length).TrimStart("\", "/").Replace("\", "/")
        if (-not (Test-TerminalBenchRemoteAssetScanPath $relative)) { continue }
        $lines = @(Get-Content -Encoding UTF8 -LiteralPath $file.FullName -ErrorAction SilentlyContinue)
        for ($index = 0; $index -lt @($lines).Count; $index++) {
            $line = [string]$lines[$index]
            if ($line.TrimStart().StartsWith("#")) { continue }
            foreach ($match in [regex]::Matches($line, $urlPattern)) {
                $url = [string]$match.Value
                $assetKind = Get-TerminalBenchRemoteAssetKind $line $url $relative
                if ($assetKind -eq "metadata_comment") { continue }
                $key = Get-TerminalBenchStringSha256 $url
                $leaf = Split-Path -Leaf ([uri]$url).AbsolutePath
                if ([string]::IsNullOrWhiteSpace($leaf)) { $leaf = "asset.bin" }
                $cachePath = Join-Path $cacheRoot (Join-Path ($SampleId -replace '[^A-Za-z0-9_.-]', '_') (Join-Path $key $leaf))
                $validatorRuntime = Test-TerminalBenchValidatorRuntimePath $relative
                if ($validatorRuntime -and $coveredUrls.ContainsKey($url)) {
                    $covered = $coveredUrls[$url]
                    $cacheExists = Test-Path -LiteralPath ([string]$covered.path) -PathType Leaf
                    $actualSha = if ($cacheExists) { Get-TaskspaceExternalFileSha256 ([string]$covered.path) } else { "" }
                    $expectedSha = [string]$covered.sha256
                    $rows.Add([pscustomobject]@{
                        url = $url
                        asset_kind = "validator_dependency_cache"
                        source_path = $file.FullName
                        relative_source_path = $relative
                        source_line = $index + 1
                        source_revision = $SourceVersion
                        cache_key = $key
                        cache_path = [string]$covered.path
                        cache_exists = $cacheExists
                        expected_sha256 = $expectedSha
                        actual_sha256 = $actualSha
                        size_bytes = if ($cacheExists) { [int64]$covered.size } else { 0 }
                        license = "external-benchmark-license-see-source"
                        required_for_e3 = $false
                        injection_method = "covered_by_terminal_bench_uv_cache"
                        dockerfile_transform_diff = ""
                        post_injection_tree_sha256 = ""
                        equivalence_proven = ($cacheExists -and -not [string]::IsNullOrWhiteSpace($expectedSha) -and $actualSha -eq $expectedSha)
                    })
                    continue
                }
                $cacheExists = Test-Path -LiteralPath $cachePath -PathType Leaf
                $windowEnd = [Math]::Min(@($lines).Count - 1, $index + 5)
                $nearby = (($lines[$index..$windowEnd]) -join "`n")
                $expectedMatch = [regex]::Match($nearby, '(?i)\b[0-9a-f]{64}\b')
                if (-not $expectedMatch.Success) {
                    $expectedMatch = [regex]::Match((($lines | ForEach-Object { [string]$_ }) -join "`n"), '(?i)\b[0-9a-f]{64}\b')
                }
                $expectedSha = if ($expectedMatch.Success) { $expectedMatch.Value.ToLowerInvariant() } else { "" }
                $actualSha = if ($cacheExists) { Get-TaskspaceExternalFileSha256 $cachePath } else { "" }
                $rows.Add([pscustomobject]@{
                    url = $url
                    asset_kind = $assetKind
                    source_path = $file.FullName
                    relative_source_path = $relative
                    source_line = $index + 1
                    source_revision = $SourceVersion
                    cache_key = $key
                    cache_path = $cachePath
                    cache_exists = $cacheExists
                    expected_sha256 = $expectedSha
                    actual_sha256 = $actualSha
                    size_bytes = if ($cacheExists) { [int64](Get-Item -LiteralPath $cachePath).Length } else { 0 }
                    license = "external-benchmark-license-see-source"
                    required_for_e3 = ($assetKind -eq "materialized_file_asset" -or $assetKind -eq "unknown_runtime_network_dependency")
                    injection_method = "none"
                    dockerfile_transform_diff = ""
                    post_injection_tree_sha256 = ""
                    equivalence_proven = (-not [string]::IsNullOrWhiteSpace($expectedSha) -and $cacheExists -and $actualSha -eq $expectedSha)
                })
            }
        }
    }
    @($rows.ToArray())
}

function Initialize-TerminalBenchRemoteAssetInjection {
    param(
        [Parameter(Mandatory = $true)][string]$FixtureSource,
        [Parameter(Mandatory = $true)][string]$GeneratedDir,
        [Parameter(Mandatory = $true)]$RemoteAssets,
        [Parameter(Mandatory = $true)][string]$SampleId
    )
    $assets = @($RemoteAssets)
    if ($assets.Count -eq 0) { return [pscustomobject]@{ fixture_source = $FixtureSource; remote_assets = @(); injected = $false } }
    $requiredAssets = @($assets | Where-Object { [bool]$_.required_for_e3 })
    if ($requiredAssets.Count -eq 0) { return [pscustomobject]@{ fixture_source = $FixtureSource; remote_assets = @($assets); injected = $false } }
    $ready = @($requiredAssets | Where-Object { -not [bool]$_.equivalence_proven }).Count -eq 0
    if (-not $ready) { return [pscustomobject]@{ fixture_source = $FixtureSource; remote_assets = @($assets); injected = $false } }

    $injectedFixture = New-TaskspaceExternalDir (Join-Path $GeneratedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-remote-fixture")
    foreach ($item in Get-ChildItem -LiteralPath $FixtureSource -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $injectedFixture -Recurse -Force
    }
    $dockerfile = Join-Path $injectedFixture "Dockerfile"
    if (-not (Test-Path -LiteralPath $dockerfile)) {
        foreach ($asset in $assets) { $asset.equivalence_proven = $false }
        return [pscustomobject]@{ fixture_source = $FixtureSource; remote_assets = @($assets); injected = $false }
    }
    $dockerText = Get-Content -Raw -Encoding UTF8 -LiteralPath $dockerfile
    $originalDockerText = $dockerText
    foreach ($asset in $requiredAssets) {
        $assetRel = ".wra/$($asset.cache_key)/$(Split-Path -Leaf ([uri]$asset.url).AbsolutePath)"
        $assetDest = Join-Path $injectedFixture ($assetRel.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
        New-Item -ItemType Directory -Path (Split-Path -Parent $assetDest) -Force | Out-Null
        Copy-Item -LiteralPath ([string]$asset.cache_path) -Destination $assetDest -Force
        $linePattern = "(?m)^\s*RUN\s+curl\b.*?-o\s+([^\s]+)\s+[`"']?$([regex]::Escape([string]$asset.url))[`"']?.*$"
        $match = [regex]::Match($dockerText, $linePattern)
        if ($match.Success) {
            $destination = $match.Groups[1].Value.Trim('"', "'")
            $replacement = "COPY $assetRel $destination"
            $dockerText = [regex]::Replace($dockerText, $linePattern, $replacement, 1)
            $asset.injection_method = "dockerfile_copy_rewrite"
            $asset.dockerfile_transform_diff = "replace:$($match.Value)=>$replacement"
        } else {
            $asset.equivalence_proven = $false
        }
    }
    if ($dockerText -ne $originalDockerText) {
        Set-Content -LiteralPath $dockerfile -Encoding UTF8 -Value $dockerText
    }
    $treeSha = Get-TaskspaceExternalTreeSha256 $injectedFixture
    foreach ($asset in $requiredAssets) {
        if ([string]$asset.injection_method -eq "dockerfile_copy_rewrite") {
            $asset.post_injection_tree_sha256 = $treeSha
            $asset.equivalence_proven = $true
        }
    }
    [pscustomobject]@{ fixture_source = $injectedFixture; remote_assets = @($assets); injected = $true }
}
