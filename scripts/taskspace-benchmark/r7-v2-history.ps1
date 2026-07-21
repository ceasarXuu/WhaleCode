function Get-R7FirstParentHistory {
    param([string]$TargetCommit, [string]$Path)
    $target = Get-R7GitLine @("rev-parse", $TargetCommit)
    @(Invoke-R7Git @("log", "--first-parent", "--reverse", "--format=%H", $target, "--", $Path))
}

function Assert-R7FirstParentAncestor {
    param([string]$Ancestor, [string]$Descendant, [string]$Code)
    $line = Get-R7GitLine @("rev-list", "--first-parent", "--ancestry-path", "--count", "$Ancestor..$Descendant")
    if ([int]$line -lt 1 -and $Ancestor -cne $Descendant) { throw "$Code ancestor=$Ancestor descendant=$Descendant" }
    $firstParent = @(Invoke-R7Git @("rev-list", "--first-parent", $Descendant))
    if ($Ancestor -cne $Descendant -and $firstParent -notcontains $Ancestor) { throw "$Code ancestor=$Ancestor descendant=$Descendant" }
}

function Get-R7TreeEntry {
    param([string]$Commit, [string]$Path)
    $lines = @(Invoke-R7Git @("ls-tree", $Commit, "--", $Path))
    if ($lines.Count -ne 1) { throw "R7_TREE_ENTRY_NOT_UNIQUE commit=$Commit path=$Path count=$($lines.Count)" }
    $match = [regex]::Match([string]$lines[0], '^(?<mode>[0-9]{6}) (?<type>[^ ]+) (?<object>[0-9a-f]{40})\t(?<path>.+)$')
    if (-not $match.Success) { throw "R7_TREE_ENTRY_INVALID commit=$Commit path=$Path" }
    [pscustomobject]@{
        mode = $match.Groups['mode'].Value
        type = $match.Groups['type'].Value
        object = $match.Groups['object'].Value
        path = $match.Groups['path'].Value
    }
}

function Assert-R7OrdinaryBlob {
    param([string]$Commit, [string]$Path, [string]$Code)
    $entry = Get-R7TreeEntry $Commit $Path
    if ([string]$entry.mode -cne "100644" -or [string]$entry.type -cne "blob") {
        throw "$Code commit=$Commit path=$Path mode=$($entry.mode) type=$($entry.type)"
    }
    $entry
}

function Get-R7FirstAddAnchor {
    param([string]$Path, [string]$Kind, [string]$TargetCommit = "HEAD")
    $target = Get-R7GitLine @("rev-parse", $TargetCommit)
    $history = @(Get-R7FirstParentHistory $target $Path)
    if ($history.Count -ne 1) { throw "R7_ANCHOR_NOT_IMMUTABLE path=$Path events=$($history.Count)" }
    $addCommit = [string]$history[0]
    $status = (Invoke-R7Git @("diff-tree", "--root", "--no-commit-id", "--name-status", "-r", $addCommit, "--", $Path)) -join "`n"
    if (-not $status.StartsWith("A`t", [System.StringComparison]::Ordinal)) { throw "R7_ANCHOR_NOT_FIRST_ADD path=$Path" }
    [void](Assert-R7OrdinaryBlob $addCommit $Path "R7_ANCHOR_MODE")
    $raw = Get-R7GitBlobText $addCommit $Path
    $scratchRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs"
    [System.IO.Directory]::CreateDirectory($scratchRoot) | Out-Null
    $scratch = Join-Path $scratchRoot "$addCommit-$([System.IO.Path]::GetFileName($Path))"
    [System.IO.File]::WriteAllText($scratch, $raw, [System.Text.UTF8Encoding]::new($false))
    $anchorSchema = if ([string]::IsNullOrWhiteSpace($env:R7_ANCHOR_SCHEMA_PATH)) {
        Join-Path $script:R7RepoRoot $script:R7AnchorSchemaPath
    } else {
        $env:R7_ANCHOR_SCHEMA_PATH
    }
    $anchor = Read-R7StrictJson $scratch $anchorSchema
    if ([string]$anchor.anchor_kind -cne $Kind) { throw "R7_ANCHOR_KIND_MISMATCH path=$Path" }
    $parent = Get-R7GitLine @("rev-parse", "$addCommit^1")
    if ([string]$anchor.anchored_parent_commit -cne $parent) { throw "R7_ANCHOR_PARENT_MISMATCH path=$Path" }
    Assert-R7FirstParentAncestor $addCommit $target "R7_ANCHOR_NOT_FIRST_PARENT_ANCESTOR"
    [pscustomobject]@{body = $anchor; raw = $raw; add_commit = $addCommit; parent_commit = $parent; target_commit = $target}
}

function Assert-R7AnchorArtifacts {
    param($Anchor)
    $roles = @($Anchor.body.artifacts | ForEach-Object { [string]$_.role })
    $paths = @($Anchor.body.artifacts | ForEach-Object { [string]$_.path })
    if (($roles | Sort-Object -Unique).Count -ne $roles.Count) { throw "R7_ANCHOR_DUPLICATE_ROLE" }
    if (($paths | Sort-Object -Unique).Count -ne $paths.Count) { throw "R7_ANCHOR_DUPLICATE_PATH" }
    foreach ($artifact in @($Anchor.body.artifacts)) {
        $path = [string]$artifact.path
        [void](Assert-R7OrdinaryBlob $Anchor.parent_commit $path "R7_ANCHOR_ARTIFACT_MODE")
        if ((Get-R7GitBlobSha256 $Anchor.parent_commit $path) -cne [string]$artifact.sha256) {
            throw "R7_ANCHOR_ARTIFACT_HASH role=$($artifact.role)"
        }
    }
}

function Get-R7BaselineAnchor {
    param([string]$TargetCommit = "HEAD")
    $paths = @(
        "benchmarks/taskspace/r7/continuous-action-ca0-baseline-v1.json",
        "benchmarks/taskspace/r7/continuous-action-ca0-baseline-v2.json",
        $script:R7BaselineAnchorPath
    )
    $anchors = @($paths | ForEach-Object { Get-R7FirstAddAnchor $_ "continuous_action_production_baseline" $TargetCommit })
    for ($index = 1; $index -lt $anchors.Count; $index++) {
        $previous = $anchors[$index - 1]
        $current = $anchors[$index]
        if ([string]$current.body.supersedes.path -cne $paths[$index - 1] -or
            [string]$current.body.supersedes.first_add_commit -cne $previous.add_commit -or
            [string]$current.body.supersedes.sha256 -cne (Get-R7Sha256Text $previous.raw) -or
            [string]$current.body.supersedes.reason -cne "phase_ownership_conflict") {
            throw "R7_BASELINE_SUPERSESSION_INVALID path=$($paths[$index])"
        }
        Assert-R7FirstParentAncestor $previous.add_commit $current.add_commit "R7_BASELINE_SUPERSESSION_ANCESTRY"
    }
    Assert-R7AnchorArtifacts $anchors[-1]
    $anchors[-1]
}

function Assert-R7NoPinnedHistoryDrift {
    param($Anchor, [string]$TargetCommit)
    foreach ($artifact in @($Anchor.body.artifacts)) {
        $path = [string]$artifact.path
        $events = @(Invoke-R7Git @("log", "--first-parent", "--format=%H", "$($Anchor.add_commit)..$TargetCommit", "--", $path))
        if ($events.Count -ne 0) { throw "R7_PINNED_HISTORY_DRIFT role=$($artifact.role) commits=$($events -join ',')" }
        [void](Assert-R7OrdinaryBlob $TargetCommit $path "R7_PINNED_TARGET_MODE")
        if ((Get-R7GitBlobSha256 $TargetCommit $path) -cne [string]$artifact.sha256) {
            throw "R7_PINNED_TARGET_HASH role=$($artifact.role)"
        }
    }
}

function Assert-R7ToolchainWorktree {
    param([string]$TargetCommit = "HEAD")
    $anchor = Get-R7FirstAddAnchor $script:R7ToolchainAnchorPath "continuous_action_v2_toolchain" $TargetCommit
    Assert-R7AnchorArtifacts $anchor
    Assert-R7NoPinnedHistoryDrift $anchor $anchor.target_commit
    foreach ($artifact in @($anchor.body.artifacts)) {
        $full = Join-Path $script:R7RepoRoot ([string]$artifact.path)
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "R7_TOOLCHAIN_FILE_MISSING role=$($artifact.role)" }
        if ((Get-Item -LiteralPath $full -Force).LinkType) { throw "R7_TOOLCHAIN_WORKTREE_LINK role=$($artifact.role)" }
        if ((Get-R7Sha256File $full) -cne [string]$artifact.sha256) { throw "R7_TOOLCHAIN_WORKTREE_DRIFT role=$($artifact.role)" }
    }
    $anchor
}
