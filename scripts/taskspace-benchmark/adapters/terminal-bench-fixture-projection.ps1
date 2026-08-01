function ConvertFrom-TerminalBenchDockerCopyToken {
    param([Parameter(Mandatory = $true)][string]$Token)
    $text = $Token.Trim()
    if (($text.StartsWith('"') -and $text.EndsWith('"')) -or ($text.StartsWith("'") -and $text.EndsWith("'"))) {
        return $text.Substring(1, $text.Length - 2)
    }
    $text
}

function Get-TerminalBenchAppProjectionDestination {
    param(
        [Parameter(Mandatory = $true)][string]$SourceRelativePath,
        [Parameter(Mandatory = $true)][string]$Destination
    )
    $sourceLeaf = Split-Path -Leaf ($SourceRelativePath.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
    $dest = (ConvertFrom-TerminalBenchDockerCopyToken $Destination).Replace("\", "/")
    if ([string]::IsNullOrWhiteSpace($dest)) { return "" }

    if ($dest -eq "." -or $dest -eq "./" -or $dest -eq "/app" -or $dest -eq "/app/") {
        return $sourceLeaf
    }
    if ($dest.StartsWith("/app/")) {
        $suffix = $dest.Substring(5).TrimStart("/")
        if ([string]::IsNullOrWhiteSpace($suffix)) { return $sourceLeaf }
        if ($suffix.EndsWith("/")) { return ($suffix + $sourceLeaf) }
        return $suffix
    }
    if ($dest.StartsWith("/")) { return "" }
    if ($dest.StartsWith("./")) { $dest = $dest.Substring(2) }
    if ([string]::IsNullOrWhiteSpace($dest) -or $dest -eq ".") { return $sourceLeaf }
    if ($dest.EndsWith("/")) { return ($dest + $sourceLeaf) }
    $dest
}

function Test-TerminalBenchDockerCopySource {
    param([Parameter(Mandatory = $true)][string]$SourceRelativePath)
    $source = (ConvertFrom-TerminalBenchDockerCopyToken $SourceRelativePath).Replace("\", "/")
    if ([string]::IsNullOrWhiteSpace($source)) { return $false }
    if ($source.StartsWith("/") -or $source.Contains("://")) { return $false }
    if ($source -match '[\*\?\[]') { return $false }
    $true
}

function Get-TerminalBenchAgentAppFixtureProjection {
    param([Parameter(Mandatory = $true)][string]$FixtureSource)
    $dockerfile = Join-Path $FixtureSource "Dockerfile"
    if (-not (Test-Path -LiteralPath $dockerfile)) { return @() }
    $rows = New-Object System.Collections.Generic.List[object]
    $lines = Get-Content -Encoding UTF8 -LiteralPath $dockerfile
    for ($index = 0; $index -lt @($lines).Count; $index++) {
        $line = [string]$lines[$index]
        if ($line.TrimStart().StartsWith("#")) { continue }
        if ($line -match '^\s*(COPY|ADD)\s+(?<flags>(--[^\s]+\s+)*)?(?<src>[^\s\[\],]+)\s+(?<dest>[^\s\[\],]+)\s*(?:#.*)?$') {
            $source = ConvertFrom-TerminalBenchDockerCopyToken $matches["src"]
            if (-not (Test-TerminalBenchDockerCopySource $source)) { continue }
            $destRel = Get-TerminalBenchAppProjectionDestination $source $matches["dest"]
            if ([string]::IsNullOrWhiteSpace($destRel)) { continue }
            $sourceRel = $source.Replace("\", "/").TrimStart("./")
            $destRel = $destRel.Replace("\", "/").TrimStart("/")
            if (-not (Test-TerminalBenchPublicFixtureRelativePath $sourceRel)) { continue }
            if (-not (Test-TerminalBenchPublicFixtureRelativePath $destRel)) { continue }
            $sourcePath = Join-Path $FixtureSource ($sourceRel.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
            if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) { continue }
            $rows.Add([pscustomobject]@{
                    source = $sourceRel
                    destination = $destRel
                    dockerfile_line = $index + 1
                    docker_instruction = $matches[1].ToUpperInvariant()
                })
        }
    }
    @($rows.ToArray())
}

function ConvertTo-TerminalBenchAgentAppFixture {
    param(
        [Parameter(Mandatory = $true)][string]$FixtureSource,
        [Parameter(Mandatory = $true)][string]$GeneratedDir,
        [Parameter(Mandatory = $true)][string]$SampleId
    )
    $projections = @(Get-TerminalBenchAgentAppFixtureProjection $FixtureSource)
    if ($projections.Count -eq 0) {
        return [pscustomobject]@{ fixture_source = $FixtureSource; projections = @(); projected = $false }
    }

    $projectionRoot = New-TaskspaceExternalDir (Join-Path $GeneratedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-app-fixture")
    Copy-TaskspaceExternalTreeContent $FixtureSource $projectionRoot
    foreach ($projection in $projections) {
        $src = Join-Path $projectionRoot ([string]$projection.source).Replace("/", [System.IO.Path]::DirectorySeparatorChar)
        $dst = Join-Path $projectionRoot ([string]$projection.destination).Replace("/", [System.IO.Path]::DirectorySeparatorChar)
        Copy-TaskspaceExternalFile $src $dst
    }
    [pscustomobject]@{ fixture_source = $projectionRoot; projections = @($projections); projected = $true }
}
