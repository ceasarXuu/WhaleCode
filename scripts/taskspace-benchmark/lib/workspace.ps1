function New-TaskspaceBenchmarkRun {
    param(
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$ScenarioId
    )
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
    New-Dir (Join-Path $RunRoot "$ScenarioId\$stamp")
}

function Get-NeutralTaskspaceBenchmarkRunRoot {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    Join-Path $RepoRoot "target\paired-bench-runs"
}

function Get-TaskspaceModeMapping {
    param([Parameter(Mandatory = $true)][int]$Repeat)
    if ($Repeat % 2 -eq 1) {
        return [ordered]@{ left = "standard"; right = "taskspace" }
    }
    [ordered]@{ left = "taskspace"; right = "standard" }
}

function Initialize-TaskspaceRepoBaseline {
    param([Parameter(Mandatory = $true)][string]$RepoDir)
    Push-Location $RepoDir
    try {
        git init | Out-Null
        git config user.email "taskspace-benchmark@example.local" | Out-Null
        git config user.name "TaskSpace Benchmark" | Out-Null
        git add . | Out-Null
        git commit -m "baseline fixture" | Out-Null
        if ((git status --porcelain) -ne $null) {
            $status = git status --porcelain
            if ($status) { throw "Fixture repo is dirty after baseline commit: $status" }
        }
    } finally {
        Pop-Location
    }
}

function New-TaskspacePairWorkspace {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][int]$Repeat
    )
    $repeatDir = New-Dir (Join-Path $RunDir ("pair-{0:000}" -f $Repeat))
    $reviewerOracleDir = New-Dir (Join-Path $repeatDir "reviewer-only\private-oracle")
    Copy-Item -LiteralPath $Manifest.HiddenOraclePath -Destination (Join-Path $reviewerOracleDir "oracle.py") -Force
    $mapping = Get-TaskspaceModeMapping $Repeat
    $sides = @{}
    foreach ($side in @("left", "right")) {
        $repoDir = New-Dir (Join-Path $repeatDir "$side\repo")
        $artifactDir = New-Dir (Join-Path $repeatDir "$side\artifacts")
        Copy-Item -Path (Join-Path $Manifest.FixtureDir "*") -Destination $repoDir -Recurse -Force
        Initialize-TaskspaceRepoBaseline $repoDir
        $sides[$side] = [pscustomobject]@{
            Name = $side
            LogicalMode = [string]$mapping[$side]
            RepoDir = $repoDir
            ArtifactDir = $artifactDir
        }
    }
    $mapPath = Join-Path $repeatDir "logical-mode-map.json"
    ([pscustomobject]@{ repeat = $Repeat; left = $mapping.left; right = $mapping.right } |
        ConvertTo-Json -Depth 5) | Set-Content -LiteralPath $mapPath -Encoding UTF8
    [pscustomobject]@{
        Repeat = $Repeat
        PairDir = $repeatDir
        ReviewerOracleDir = $reviewerOracleDir
        HiddenOraclePath = Join-Path $reviewerOracleDir "oracle.py"
        LogicalModeMapPath = $mapPath
        Left = $sides["left"]
        Right = $sides["right"]
    }
}

function Test-TaskspaceNeutralCwd {
    param([Parameter(Mandatory = $true)][string]$Path)
    $resolved = if (Test-Path -LiteralPath $Path) { (Resolve-Path -LiteralPath $Path).Path } else { $Path }
    $lower = $resolved.ToLowerInvariant()
    foreach ($forbidden in @("standard", "taskspace", "action-map", " map", "\map", "/map", "node", "subagent")) {
        if ($lower.Contains($forbidden)) { return $false }
    }
    return $true
}

function New-TaskspaceWhaleArgv {
    param(
        [Parameter(Mandatory = $true)][string]$LogicalMode,
        [Parameter(Mandatory = $true)][string]$Model,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$LastMessagePath
    )
    $args = @("exec", "--json")
    if ($LogicalMode -eq "taskspace") { $args += "--taskspace" }
    $args += @("-m", $Model, "-C", $RepoDir, "--dangerously-bypass-approvals-and-sandbox", "--output-last-message", $LastMessagePath, "-")
    @($args)
}

function Get-NormalizedTaskspaceWhaleArgv {
    param([Parameter(Mandatory = $true)][string[]]$Argv)
    $normalized = @()
    for ($i = 0; $i -lt $Argv.Count; $i++) {
        $arg = $Argv[$i]
        if ($arg -eq "-C") {
            $normalized += $arg
            $normalized += "<repo>"
            $i++
        } elseif ($arg -eq "--output-last-message") {
            $normalized += $arg
            $normalized += "<last-message>"
            $i++
        } else {
            $normalized += $arg
        }
    }
    @($normalized)
}
