function Get-TerminalBenchStringSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
}

function Get-TerminalBenchRemoteAssets {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$SampleId,
        [Parameter(Mandatory = $true)][string]$OutputRoot,
        [Parameter(Mandatory = $true)][string]$SourceVersion
    )
    $rows = New-Object System.Collections.Generic.List[object]
    $cacheRoot = Join-Path $OutputRoot "_asset-cache"
    foreach ($file in @(Get-ChildItem -LiteralPath $TaskRoot -Recurse -File -Force -ErrorAction SilentlyContinue)) {
        $relative = $file.FullName.Substring($TaskRoot.Length).TrimStart("\", "/").Replace("\", "/")
        $lines = Get-Content -Encoding UTF8 -LiteralPath $file.FullName -ErrorAction SilentlyContinue
        for ($index = 0; $index -lt @($lines).Count; $index++) {
            foreach ($match in [regex]::Matches([string]$lines[$index], 'https?://[^\s"''<>]+')) {
                $url = [string]$match.Value
                $key = Get-TerminalBenchStringSha256 $url
                $leaf = Split-Path -Leaf ([uri]$url).AbsolutePath
                if ([string]::IsNullOrWhiteSpace($leaf)) { $leaf = "asset.bin" }
                $cachePath = Join-Path $cacheRoot (Join-Path ($SampleId -replace '[^A-Za-z0-9_.-]', '_') (Join-Path $key $leaf))
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
                    required_for_e3 = $true
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
    $ready = @($assets | Where-Object { -not [bool]$_.equivalence_proven }).Count -eq 0
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
    foreach ($asset in $assets) {
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
    foreach ($asset in $assets) {
        if ([string]$asset.injection_method -eq "dockerfile_copy_rewrite") {
            $asset.post_injection_tree_sha256 = $treeSha
            $asset.equivalence_proven = $true
        }
    }
    [pscustomobject]@{ fixture_source = $injectedFixture; remote_assets = @($assets); injected = $true }
}
