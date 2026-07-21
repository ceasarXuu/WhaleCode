param()

$ErrorActionPreference = "Stop"
$sourceRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$runId = [DateTime]::UtcNow.ToString("yyyyMMddHHmmssfff")
$repo = Join-Path $sourceRoot "target/r7-toolchain/git-transaction-test/$runId"

function Invoke-TestGit {
    param([string[]]$Arguments, [switch]$AllowFailure)
    $output = & git -C $repo @Arguments 2>&1
    if ($LASTEXITCODE -ne 0 -and -not $AllowFailure) {
        throw "R7_TRANSACTION_TEST_GIT_FAILED args=$($Arguments -join ' ') detail=$($output -join "`n")"
    }
    @($output)
}

function Get-TestGitLine {
    param([string[]]$Arguments)
    $lines = @(Invoke-TestGit $Arguments)
    if ($lines.Count -ne 1) { throw "R7_TRANSACTION_TEST_GIT_LINE count=$($lines.Count)" }
    ([string]$lines[0]).Trim()
}

function Write-TestFile {
    param([string]$Name, [string]$Value)
    [System.IO.File]::WriteAllText((Join-Path $repo $Name), "$Value`n", [System.Text.UTF8Encoding]::new($false))
}

function Assert-TestClean {
    $status = @(Invoke-TestGit @("status", "--porcelain"))
    if ($status.Count -ne 0) { throw "R7_TRANSACTION_TEST_DIRTY detail=$($status -join ',')" }
}

[System.IO.Directory]::CreateDirectory($repo) | Out-Null
[void](Invoke-TestGit @("init", "--quiet", "--initial-branch=main"))
[void](Invoke-TestGit @("config", "user.name", "R7 Transaction Test"))
[void](Invoke-TestGit @("config", "user.email", "r7-transaction@invalid.local"))
Write-TestFile "tracked.txt" "baseline"
Write-TestFile "guard.txt" "guard"
Write-TestFile ".gitignore" "target/"
[void](Invoke-TestGit @("add", "--", ".gitignore", "tracked.txt", "guard.txt"))
[void](Invoke-TestGit @("commit", "--quiet", "-m", "test baseline"))

$previousRoot = $env:R7_REPO_ROOT
try {
    $env:R7_REPO_ROOT = $repo
    . (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")
    $baseline = Get-TestGitLine @("rev-parse", "HEAD")

    $transaction = Start-R7GitTransaction $baseline "publish"
    try {
        Write-TestFile "tracked.txt" "candidate"
        $candidate = New-R7ProspectiveCommit $transaction $baseline "candidate" @("tracked.txt")
        [void](Invoke-TestGit @("diff", "--cached", "--quiet", $baseline))
        if ($LASTEXITCODE -ne 0) { throw "R7_TRANSACTION_TEST_SHARED_INDEX_MUTATED" }
        Write-TestFile "guard.txt" "blocked"
        [void](Invoke-TestGit @("add", "--", "guard.txt") -AllowFailure)
        if ($LASTEXITCODE -eq 0) { throw "R7_TRANSACTION_TEST_SHARED_INDEX_LOCK_BYPASSED" }
        Write-TestFile "guard.txt" "guard"
        Publish-R7ProspectiveCommit $transaction $candidate
    } finally {
        Stop-R7GitTransaction $transaction
    }
    Assert-TestClean
    if ((Get-Content -Raw -Encoding UTF8 (Join-Path $repo "tracked.txt")).Trim() -cne "candidate") {
        throw "R7_TRANSACTION_TEST_PUBLISHED_CONTENT_DRIFT"
    }

    $head = Get-TestGitLine @("rev-parse", "HEAD")
    $deltaTransaction = Start-R7GitTransaction $head "delta-reject"
    try {
        Write-TestFile "tracked.txt" "delta"
        $threw = $false
        try {
            [void](New-R7ProspectiveCommit $deltaTransaction $head "bad delta" @("tracked.txt", "guard.txt"))
        } catch {
            $threw = $_.Exception.Message.Contains("R7_TRANSACTION_TREE_DELTA", [System.StringComparison]::Ordinal)
        }
        if (-not $threw) { throw "R7_TRANSACTION_TEST_DELTA_MISMATCH_ACCEPTED" }
    } finally {
        Restore-R7PathsFromCommit $head @("tracked.txt") "delta-reject-restore"
        Stop-R7GitTransaction $deltaTransaction
    }
    Assert-TestClean

    $casTransaction = Start-R7GitTransaction $head "cas-reject"
    try {
        Write-TestFile "tracked.txt" "cas candidate"
        $casCandidate = New-R7ProspectiveCommit $casTransaction $head "cas candidate" @("tracked.txt")
        $tree = Get-TestGitLine @("show", "-s", "--format=%T", $head)
        $concurrent = Get-TestGitLine @("commit-tree", $tree, "-p", $head, "-m", "concurrent")
        [void](Invoke-TestGit @("update-ref", "HEAD", $concurrent, $head))
        $threw = $false
        try { Publish-R7ProspectiveCommit $casTransaction $casCandidate } catch {
            $threw = $_.Exception.Message.Contains("R7_TRANSACTION_HEAD_DRIFT", [System.StringComparison]::Ordinal)
        }
        if (-not $threw) { throw "R7_TRANSACTION_TEST_CAS_DRIFT_ACCEPTED" }
        [void](Invoke-TestGit @("diff", "--cached", "--quiet", $concurrent))
        if ($LASTEXITCODE -ne 0) { throw "R7_TRANSACTION_TEST_CAS_INDEX_MUTATED" }
    } finally {
        Restore-R7PathsFromCommit $concurrent @("tracked.txt") "cas-reject-restore"
        Stop-R7GitTransaction $casTransaction
    }
    Assert-TestClean

    [pscustomobject][ordered]@{
        schema_version = 1
        test = "r7_git_transaction"
        passed = $true
        publish_commit = $candidate
        concurrent_commit = $concurrent
        cases = 4
    } | ConvertTo-Json -Compress
} finally {
    $env:R7_REPO_ROOT = $previousRoot
}
