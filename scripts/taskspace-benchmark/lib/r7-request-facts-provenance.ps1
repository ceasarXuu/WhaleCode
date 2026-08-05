function Get-R7RequestFactsAnalyzerIdentity {
    $benchmarkRoot = Split-Path -Parent $PSScriptRoot
    $relativePaths = @(
        "request_facts.py",
        "request_fact_availability.py",
        "request_fact_diagnostics.py",
        "request_fact_summary.py",
        "request_fact_validation.py"
    )
    $files = @($relativePaths | ForEach-Object {
            $path = Join-Path $benchmarkRoot $_
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                throw "Request facts analyzer source is missing: $path"
            }
            [pscustomobject]@{
                path = $_
                sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
            }
        })
    $mainSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $benchmarkRoot "request_facts.py")
    $versionMatch = [regex]::Match($mainSource, '(?m)^ANALYZER_VERSION = "([^"]+)"$')
    if (-not $versionMatch.Success) { throw "Request facts analyzer version is missing" }
    $joined = @($files | ForEach-Object { "$($_.path):$($_.sha256)" }) -join "`n"
    $bytes = [Text.Encoding]::UTF8.GetBytes($joined)
    $sha = [Security.Cryptography.SHA256]::HashData($bytes)
    [pscustomobject]@{
        version = $versionMatch.Groups[1].Value
        sha256 = [Convert]::ToHexString($sha).ToLowerInvariant()
        files = $files
    }
}

function Get-R7RequestFactsIdentity {
    param(
        [Parameter(Mandatory = $true)][string]$RequestFactsPath,
        [Parameter(Mandatory = $true)][string]$ArtifactDir
    )
    if (-not (Test-Path -LiteralPath $RequestFactsPath -PathType Leaf)) {
        throw "Request facts artifact is missing: $RequestFactsPath"
    }
    $facts = Get-Content -Raw -Encoding UTF8 -LiteralPath $RequestFactsPath |
        ConvertFrom-Json -Depth 100
    $analyzer = Get-R7RequestFactsAnalyzerIdentity
    if ([string]$facts.schema_version -ne "whalecode-request-facts-v1" -or
        [string]$facts.analyzer_version -ne [string]$analyzer.version) {
        throw "Request facts analyzer identity is stale"
    }
    $artifactRoot = [IO.Path]::GetFullPath($ArtifactDir).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $sources = [ordered]@{}
    foreach ($name in @("rollout", "wire", "boundary")) {
        $source = $facts.sources.$name
        $status = [string]$source.status
        if ($status -notin @("read", "unavailable")) {
            throw "Request facts source status is invalid: $name"
        }
        $path = if ([string]::IsNullOrWhiteSpace([string]$source.path)) {
            ""
        } else {
            [IO.Path]::GetFullPath([string]$source.path)
        }
        if ($path -and -not $path.StartsWith("$artifactRoot$([IO.Path]::DirectorySeparatorChar)", [StringComparison]::Ordinal)) {
            throw "Request facts source is outside the artifact directory: $name"
        }
        $exists = $path -and (Test-Path -LiteralPath $path -PathType Leaf)
        if (($status -eq "read") -ne [bool]$exists) {
            throw "Request facts source availability is stale: $name"
        }
        $sha256 = if ($exists) {
            (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        } else { $null }
        if ($exists -and [string]$source.sha256 -ne $sha256) {
            throw "Request facts source hash is stale: $name"
        }
        $sources[$name] = [pscustomobject]@{
            status = $status
            path = $path
            sha256 = $sha256
        }
    }
    [pscustomobject]@{
        schema_version = [string]$facts.schema_version
        artifact_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $RequestFactsPath).Hash.ToLowerInvariant()
        analyzer = $analyzer
        sources = [pscustomobject]$sources
        availability = $facts.availability
    }
}

function Test-R7RequestFactsFreshness {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[object]]$Findings
    )
    $factsPath = Join-Path $ArtifactDir "request-facts.json"
    try {
        $identity = Get-R7RequestFactsIdentity $factsPath $ArtifactDir
        $facts = Get-Content -Raw -Encoding UTF8 -LiteralPath $factsPath | ConvertFrom-Json -Depth 100
    } catch {
        $code = if ([string]$_.Exception.Message -match "source (hash|availability) is stale") {
            "request_facts_stale"
        } else { "request_facts_identity_invalid" }
        Add-R7EvidenceFinding $Findings $code ([string]$_.Exception.Message) $factsPath
        return $null
    }
    $arguments = @{}
    foreach ($name in @("rollout", "wire", "boundary")) {
        $source = $identity.sources.$name
        if ([string]$source.status -eq "read") {
            $parameter = switch ($name) {
                "rollout" { "RolloutJsonlPath" }
                "wire" { "WireTracePath" }
                "boundary" { "BoundaryEventsPath" }
            }
            $arguments[$parameter] = [string]$source.path
        }
    }
    $rebuilt = Invoke-TaskspaceRequestFactsGenerator @arguments
    $expected = $facts | ConvertTo-Json -Depth 100 -Compress
    $actual = $rebuilt | ConvertTo-Json -Depth 100 -Compress
    if ($actual -cne $expected) {
        Add-R7EvidenceFinding $Findings "request_facts_stale" "Request facts do not match their sealed sources." $factsPath
        return $null
    }
    $facts
}

function Get-R7RequestFactsManifestRoles {
    param($Evidence)
    $roles = @(
        "run_status", "binary_health", "performance_observation", "resolved_manifest",
        "rollout", "provider_wire_trace", "request_summary", "request_facts", "binary_attestation"
    )
    if ([string]$Evidence.request_facts_identity.sources.boundary.status -eq "read") {
        $roles += "provider_boundary_events"
    }
    $roles
}

function Test-R7RequestFactsManifestIdentity {
    param(
        $Evidence,
        [string]$ArtifactDir,
        [System.Collections.Generic.List[object]]$Findings,
        [string]$EvidencePath,
        [string]$RunDir
    )
    try {
        $current = Get-R7RequestFactsIdentity (Join-Path $ArtifactDir "request-facts.json") $ArtifactDir
        $sealed = $Evidence.request_facts_identity
        if (($current | ConvertTo-Json -Depth 20 -Compress) -cne
            ($sealed | ConvertTo-Json -Depth 20 -Compress)) {
            Add-R7ProvenanceFinding $Findings "run_request_facts_identity_mismatch" $EvidencePath $RunDir
        }
    } catch {
        Add-R7ProvenanceFinding $Findings "run_request_facts_identity_invalid" (Join-Path $ArtifactDir "request-facts.json") $RunDir
    }
}
