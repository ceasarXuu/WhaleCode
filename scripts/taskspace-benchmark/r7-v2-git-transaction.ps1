function Get-R7TransactionPath {
    param([string]$Root, [string]$TransactionId, [string]$Name)
    $safeId = $TransactionId -replace '[^A-Za-z0-9._-]', '-'
    Join-Path $script:R7RepoRoot "$Root/$safeId/$Name"
}

function Start-R7GitTransaction {
    param([string]$ExpectedHead, [string]$TransactionId)
    $head = Get-R7GitLine @("rev-parse", "HEAD")
    if ($head -cne $ExpectedHead) {
        throw "R7_TRANSACTION_HEAD_DRIFT expected=$ExpectedHead actual=$head"
    }
    $indexPath = Get-R7GitLine @("rev-parse", "--path-format=absolute", "--git-path", "index")
    $lockPath = "$indexPath.lock"
    [System.IO.Directory]::CreateDirectory((Split-Path $indexPath -Parent)) | Out-Null
    try {
        $stream = [System.IO.File]::Open(
            $lockPath,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::ReadWrite,
            [System.IO.FileShare]::None
        )
        $stream.Dispose()
    } catch {
        throw "R7_TRANSACTION_INDEX_BUSY path=$lockPath"
    }
    try {
        $head = Get-R7GitLine @("rev-parse", "HEAD")
        if ($head -cne $ExpectedHead) {
            throw "R7_TRANSACTION_HEAD_DRIFT expected=$ExpectedHead actual=$head"
        }
        Assert-R7CleanWorktree
        $privateIndex = Get-R7TransactionPath "target/r7-toolchain/private-indexes" $TransactionId "index"
        [System.IO.Directory]::CreateDirectory((Split-Path $privateIndex -Parent)) | Out-Null
        [pscustomobject]@{
            expected_head = $ExpectedHead
            transaction_id = $TransactionId
            shared_index = $indexPath
            lock_path = $lockPath
            private_index = $privateIndex
            published = $false
        }
    } catch {
        Move-R7TransactionFile $lockPath "target/r7-toolchain/failed-transactions" $TransactionId "index.lock"
        throw
    }
}

function Invoke-R7PrivateIndex {
    param($Transaction, [scriptblock]$Operation)
    $previous = $env:GIT_INDEX_FILE
    try {
        $env:GIT_INDEX_FILE = [string]$Transaction.private_index
        & $Operation
    } finally {
        $env:GIT_INDEX_FILE = $previous
    }
}

function Assert-R7TransactionOpen {
    param($Transaction)
    if ([bool]$Transaction.published -or -not (Test-Path -LiteralPath $Transaction.lock_path -PathType Leaf)) {
        throw "R7_TRANSACTION_NOT_OPEN id=$($Transaction.transaction_id)"
    }
    $head = Get-R7GitLine @("rev-parse", "HEAD")
    if ($head -cne [string]$Transaction.expected_head) {
        throw "R7_TRANSACTION_HEAD_DRIFT expected=$($Transaction.expected_head) actual=$head"
    }
}

function New-R7ProspectiveCommit {
    param(
        $Transaction,
        [string]$ParentCommit,
        [string]$Message,
        [string[]]$Paths
    )
    Assert-R7TransactionOpen $Transaction
    $expected = @($Paths | Sort-Object -Unique)
    if ($expected.Count -eq 0) { throw "R7_TRANSACTION_PATHS_EMPTY" }
    foreach ($path in $expected) {
        if ([System.IO.Path]::IsPathRooted($path) -or $path.Contains("..")) {
            throw "R7_TRANSACTION_PATH_INVALID path=$path"
        }
    }
    Invoke-R7PrivateIndex $Transaction {
        Invoke-R7Git @("read-tree", $ParentCommit) | Out-Null
        Invoke-R7Git (@("add", "-A", "--") + $expected) | Out-Null
        $actual = @(Invoke-R7Git @("diff", "--cached", "--name-only", $ParentCommit) | Sort-Object -Unique)
        if (($actual -join "`n") -cne ($expected -join "`n")) {
            throw "R7_TRANSACTION_TREE_DELTA expected=$($expected -join ',') actual=$($actual -join ',')"
        }
        Invoke-R7Git @("diff", "--cached", "--check", $ParentCommit) | Out-Null
        $tree = Get-R7GitLine @("write-tree")
        Get-R7GitLine @("commit-tree", $tree, "-p", $ParentCommit, "-m", $Message)
    }
}

function Publish-R7ProspectiveCommit {
    param($Transaction, [string]$Commit)
    Assert-R7TransactionOpen $Transaction
    $privateTree = Invoke-R7PrivateIndex $Transaction { Get-R7GitLine @("write-tree") }
    $commitTree = Get-R7GitLine @("show", "-s", "--format=%T", $Commit)
    if ($privateTree -cne $commitTree) {
        throw "R7_TRANSACTION_INDEX_TREE_DRIFT expected=$commitTree actual=$privateTree"
    }
    Invoke-R7Git @("update-ref", "HEAD", $Commit, ([string]$Transaction.expected_head)) | Out-Null
    $backup = Get-R7TransactionPath "target/r7-toolchain/completed-transactions" $Transaction.transaction_id "previous-index"
    [System.IO.Directory]::CreateDirectory((Split-Path $backup -Parent)) | Out-Null
    try {
        if (Test-Path -LiteralPath $Transaction.shared_index -PathType Leaf) {
            [System.IO.File]::Replace($Transaction.private_index, $Transaction.shared_index, $backup)
        } else {
            Move-Item -LiteralPath $Transaction.private_index -Destination $Transaction.shared_index
        }
        $Transaction.published = $true
        Move-R7TransactionFile $Transaction.lock_path "target/r7-toolchain/completed-transactions" $Transaction.transaction_id "index.lock"
    } catch {
        Invoke-R7Git @("update-ref", "HEAD", ([string]$Transaction.expected_head), $Commit) -AllowFailure | Out-Null
        throw "R7_TRANSACTION_INDEX_PUBLISH_FAILED id=$($Transaction.transaction_id) detail=$($_.Exception.Message)"
    }
    $actual = Get-R7GitLine @("rev-parse", "HEAD")
    if ($actual -cne $Commit) {
        throw "R7_TRANSACTION_PUBLISH_FAILED expected=$Commit actual=$actual"
    }
    Assert-R7CleanWorktree
}

function Move-R7TransactionFile {
    param([string]$Source, [string]$Root, [string]$TransactionId, [string]$Name)
    if (-not (Test-Path -LiteralPath $Source)) { return }
    $destination = Get-R7TransactionPath $Root $TransactionId $Name
    [System.IO.Directory]::CreateDirectory((Split-Path $destination -Parent)) | Out-Null
    Move-Item -LiteralPath $Source -Destination $destination -Force
}

function Stop-R7GitTransaction {
    param($Transaction)
    if ($null -eq $Transaction -or [bool]$Transaction.published) { return }
    Move-R7TransactionFile $Transaction.private_index "target/r7-toolchain/failed-transactions" $Transaction.transaction_id "private-index"
    Move-R7TransactionFile $Transaction.lock_path "target/r7-toolchain/failed-transactions" $Transaction.transaction_id "index.lock"
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
    foreach ($path in @($Paths | Sort-Object -Unique)) {
        $probe = Invoke-R7Git @("cat-file", "-e", "${Commit}:$path") -AllowFailure
        $exists = $LASTEXITCODE -eq 0
        $full = Join-Path $script:R7RepoRoot $path
        Backup-R7Path $path $TransactionId
        if ($exists) {
            [System.IO.Directory]::CreateDirectory((Split-Path $full -Parent)) | Out-Null
            [System.IO.File]::WriteAllBytes($full, (Get-R7GitBlobBytes $Commit $path))
        }
    }
    $status = @(Invoke-R7Git @("status", "--porcelain"))
    if ($status.Count -ne 0) {
        throw "R7_TRANSACTION_RESTORE_DIRTY`n$($status -join "`n")"
    }
}
