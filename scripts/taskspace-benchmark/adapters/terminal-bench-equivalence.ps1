function Invoke-TerminalBenchGitQuiet {
    param(
        [Parameter(Mandatory = $true)][string]$GitRoot,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )
    $oldPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "SilentlyContinue"
        $output = & git -C $GitRoot @Arguments 2>$null
        if ($LASTEXITCODE -ne 0) { return "" }
        return (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    } finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Get-TerminalBenchOfficialEquivalence {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$SourceVersion
    )
    $required = @(
        "terminal_bench\harness\harness.py",
        "terminal_bench\terminal\docker_compose_manager.py",
        "terminal_bench\handlers\trial_handler.py",
        "terminal_bench\parsers\pytest_parser.py"
    )
    $gitRoot = ""
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $root = & git -C $TaskRoot rev-parse --show-toplevel 2>$null
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($root)) { $gitRoot = $root.Trim() }
    }
    $sourceFiles = New-Object System.Collections.Generic.List[object]
    $allPresent = -not [string]::IsNullOrWhiteSpace($gitRoot)
    $allPinned = $allPresent
    if ($allPresent) {
        foreach ($relative in $required) {
            $path = Join-Path $gitRoot $relative
            $relativeUnix = $relative.Replace("\", "/")
            $pinnedBlob = if (Get-Command git -ErrorAction SilentlyContinue) {
                Invoke-TerminalBenchGitQuiet $gitRoot @("rev-parse", "$SourceVersion`:$relativeUnix")
            } else { "" }
            if (-not (Test-Path -LiteralPath $path)) {
                $allPresent = $false
            } else {
                $currentBlob = Invoke-TerminalBenchGitQuiet $gitRoot @("hash-object", $path)
                $resolved = (Resolve-Path -LiteralPath $path).Path
                $matchesPinned = (-not [string]::IsNullOrWhiteSpace($pinnedBlob) -and $pinnedBlob -eq $currentBlob)
                if (-not $matchesPinned) { $allPinned = $false }
                $sourceFiles.Add([pscustomobject]@{
                    path = $resolved
                    relative_path = $relativeUnix
                    current_sha256 = Get-TaskspaceExternalFileSha256 $resolved
                    pinned_blob_id = $pinnedBlob
                    current_blob_id = $currentBlob
                    matches_pinned_revision = $matchesPinned
                })
            }
        }
    }
    $revisionPinned = $SourceVersion -match '^[0-9a-fA-F]{40}$'
    $taskRelative = ""
    $taskDirty = $true
    if ($allPresent) {
        $taskRelative = (Resolve-Path -LiteralPath $TaskRoot).Path.Substring((Resolve-Path -LiteralPath $gitRoot).Path.Length).TrimStart("\", "/").Replace("\", "/")
        $status = & git -C $gitRoot status --porcelain -- $taskRelative 2>$null
        $taskDirty = @($status | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0
    }
    [ordered]@{
        protocol = "terminal_bench_post_agent_tests_v1"
        source_root = $gitRoot
        source_revision = $SourceVersion
        source_revision_pinned = $revisionPinned
        source_files_present = $allPresent
        source_files_match_pinned_revision = $allPinned
        task_relative_path = $taskRelative
        task_worktree_dirty = $taskDirty
        source_files = @($sourceFiles.ToArray())
        proven = ($revisionPinned -and $allPresent -and $allPinned -and -not $taskDirty)
    }
}
