$ErrorActionPreference = "Stop"
$repoRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-authority-$([guid]::NewGuid().ToString('N'))"
$binaryRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-authority-bin-$([guid]::NewGuid().ToString('N'))"
. (Join-Path $PSScriptRoot "lib/harness-health.ps1")
. (Join-Path $PSScriptRoot "lib/r7-evaluation-authority.ps1")

function Invoke-TestGit {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & git -C $repoRoot @Arguments | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Fixture git command failed: $($Arguments -join ' ')"
    }
}

try {
    New-Item -ItemType Directory -Force -Path (
        Join-Path $repoRoot "third_party/codex-cli"
    ) | Out-Null
    New-Item -ItemType Directory -Force -Path $binaryRoot | Out-Null
    [IO.File]::WriteAllText(
        (Join-Path $repoRoot "third_party/codex-cli/source.txt"),
        "source`n",
        [Text.UTF8Encoding]::new($false)
    )
    [IO.File]::WriteAllText(
        (Join-Path $repoRoot "tracked.txt"),
        "committed`n",
        [Text.UTF8Encoding]::new($false)
    )
    Invoke-TestGit init -q
    Invoke-TestGit config user.email "fixture@example.invalid"
    Invoke-TestGit config user.name "R7 Fixture"
    Invoke-TestGit add .
    Invoke-TestGit commit -q -m "fixture"
    $commit = ((& git -C $repoRoot rev-parse HEAD) | Select-Object -First 1).Trim()
    $trackedPath = Join-Path $repoRoot "tracked.txt"
    if (-not (Test-R7TrackedFileMatchesCommit $repoRoot $commit $trackedPath)) {
        throw "Committed file bytes did not match their Git blob"
    }
    [IO.File]::WriteAllText(
        $trackedPath,
        "modified`n",
        [Text.UTF8Encoding]::new($false)
    )
    if (Test-R7TrackedFileMatchesCommit $repoRoot $commit $trackedPath) {
        throw "Modified file bytes matched the committed Git blob"
    }
    [IO.File]::WriteAllText(
        $trackedPath,
        "committed`n",
        [Text.UTF8Encoding]::new($false)
    )

    $binaryPath = Join-Path $binaryRoot "fixture-whale"
    Copy-Item -LiteralPath (Get-Command git).Source -Destination $binaryPath
    & chmod +x $binaryPath
    $identity = Get-TaskspaceGitBuildIdentity $repoRoot
    $probe = Get-TaskspaceWhaleVersionProbe $binaryPath
    $binarySha = (
        Get-FileHash -Algorithm SHA256 -LiteralPath $binaryPath
    ).Hash.ToLowerInvariant()
    $attestationPath = Get-TaskspaceWhaleBinaryAttestationPath $binaryPath
    [pscustomobject]@{
        schema_version = 2
        status = "pass"
        repo_root = $repoRoot
        current_git_head = $identity.current_git_head
        head_tree_id = $identity.head_tree_id
        codex_tree_id = $identity.codex_tree_id
        worktree_clean = $true
        codex_source_latest_commit = $commit
        whale_bin = $binaryPath
        whale_binary_sha256 = $binarySha
        build_command = "fixture"
        executable_probe = $probe
    } | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 -LiteralPath $attestationPath
    $clean = Get-TaskspaceWhaleBinaryAttestation `
        -WhaleBin $binaryPath `
        -ExpectedBinarySha256 $binarySha `
        -ExpectedSourceCommit $commit `
        -RepoRoot $repoRoot
    if ([string]$clean.status -ne "pass") {
        throw "Clean fixture attestation was rejected: $($clean.reason)"
    }
    [IO.File]::WriteAllText(
        (Join-Path $repoRoot "untracked.txt"),
        "dirty`n",
        [Text.UTF8Encoding]::new($false)
    )
    $dirty = Get-TaskspaceWhaleBinaryAttestation `
        -WhaleBin $binaryPath `
        -ExpectedBinarySha256 $binarySha `
        -ExpectedSourceCommit $commit `
        -RepoRoot $repoRoot
    if ([string]$dirty.status -ne "invalid" -or
        [string]$dirty.reason -notmatch "git_build_identity_mismatch") {
        throw "Dirty current worktree reused a clean binary attestation"
    }
    Write-Output "R7 candidate authority passed."
} finally {
    foreach ($path in @($repoRoot, $binaryRoot)) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -Force -Recurse -LiteralPath $path
        }
    }
}
