function New-R7ProspectiveCommit {
    param(
        [string]$ExpectedHead,
        [string]$ParentCommit,
        [string]$Message
    )
    $head = Get-R7GitLine @("rev-parse", "HEAD")
    if ($head -cne $ExpectedHead) {
        throw "R7_TRANSACTION_HEAD_DRIFT expected=$ExpectedHead actual=$head"
    }
    $tree = Get-R7GitLine @("write-tree")
    Get-R7GitLine @("commit-tree", $tree, "-p", $ParentCommit, "-m", $Message)
}

function Publish-R7ProspectiveCommit {
    param([string]$Commit, [string]$ExpectedHead)
    Invoke-R7Git @("update-ref", "HEAD", $Commit, $ExpectedHead) | Out-Null
    $actual = Get-R7GitLine @("rev-parse", "HEAD")
    if ($actual -cne $Commit) {
        throw "R7_TRANSACTION_PUBLISH_FAILED expected=$Commit actual=$actual"
    }
}

function Reset-R7IndexToCommit {
    param([string]$Commit)
    Invoke-R7Git @("read-tree", "--reset", $Commit) | Out-Null
}

function Backup-R7Path {
    param([string]$RelativePath, [string]$TransactionId)
    $source = Join-Path $script:R7RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $source)) { return }
    $backupRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/failed-transactions/$TransactionId"
    $destination = Join-Path $backupRoot $RelativePath
    [System.IO.Directory]::CreateDirectory((Split-Path $destination -Parent)) | Out-Null
    Move-Item -LiteralPath $source -Destination $destination -Force
}

function Restore-R7PathsFromCommit {
    param([string]$Commit, [string[]]$Paths, [string]$TransactionId)
    Reset-R7IndexToCommit $Commit
    foreach ($path in @($Paths | Sort-Object -Unique)) {
        $probe = Invoke-R7Git @("cat-file", "-e", "${Commit}:$path") -AllowFailure
        $exists = $LASTEXITCODE -eq 0
        $full = Join-Path $script:R7RepoRoot $path
        if ($exists) {
            Backup-R7Path $path $TransactionId
            [System.IO.Directory]::CreateDirectory((Split-Path $full -Parent)) | Out-Null
            [System.IO.File]::WriteAllBytes($full, (Get-R7GitBlobBytes $Commit $path))
        } else {
            Backup-R7Path $path $TransactionId
        }
    }
    $status = @(Invoke-R7Git @("status", "--porcelain"))
    if ($status.Count -ne 0) {
        throw "R7_TRANSACTION_RESTORE_DIRTY`n$($status -join "`n")"
    }
}
