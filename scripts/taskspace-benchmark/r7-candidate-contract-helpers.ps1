function Assert-CandidateManifestIntegrity {
    param([object]$Candidate, [string]$ManifestPath = "")
    $candidateId = [string]$Candidate.candidate_id
    Assert-Equal ([string]$Candidate.contract_id) "r7-taskspace-five-layer-candidate-$candidateId" "Candidate contract id does not match candidate id"
    Assert-Equal (Get-CandidateContentId $Candidate) $candidateId "Candidate content id does not match active snapshot and artifact hashes"
    $candidateCommit = [string]$Candidate.candidate_commit
    & git -C $repoRoot cat-file -e "$candidateCommit^{commit}" 2>$null
    Assert-True ($LASTEXITCODE -eq 0) "Candidate commit is unavailable: $candidateCommit"
    Assert-Equal ([string]$Candidate.active_authority.contract_id) "r7-five-layer-contract-authority-v1" "Candidate active authority id drifted"
    Assert-Equal ([string]$Candidate.active_authority.path) "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json" "Candidate active authority path drifted"
    $authorityBlob = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)
    Assert-Equal (Get-TextSha256 $authorityBlob) ([string]$Candidate.active_authority.sha256) "Candidate active authority snapshot hash drifted"
    Assert-Equal ([string]$Candidate.source_authority.contract_id) ([string]$Candidate.active_authority.contract_id) "Candidate source and active authority ids differ"
    Assert-Equal ([string]$Candidate.source_authority.path) ([string]$Candidate.active_authority.path) "Candidate source and active authority paths differ"
    Assert-Equal ([string]$Candidate.source_authority.sha256) ([string]$Candidate.active_authority.sha256) "Candidate source and active authority hashes differ"
    $namespaceRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId"))
    $namespacePrefix = $namespaceRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    $seenPaths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $seenHashes = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($artifact in $Candidate.artifact_hashes.psobject.Properties) {
        Assert-Equal ([string]$artifact.Value.artifact_role) ([string]$artifact.Name) "Candidate artifact role marker drifted"
        $relativePath = [string]$artifact.Value.path
        $expectedPrefix = "benchmarks/taskspace/r7/candidates/$candidateId/"
        Assert-True $relativePath.StartsWith($expectedPrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its namespace: $relativePath"
        $canonicalPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativePath))
        Assert-True $canonicalPath.StartsWith($namespacePrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its canonical namespace: $relativePath"
        Assert-True ($seenPaths.Add($canonicalPath)) "Candidate artifact paths must be unique: $relativePath"
        Assert-True ($seenHashes.Add([string]$artifact.Value.sha256)) "Candidate artifact roles must not reuse one blob hash"
        if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
            Assert-True (Test-Path -LiteralPath $canonicalPath -PathType Leaf) "Candidate artifact missing: $relativePath"
            $item = Get-Item -LiteralPath $canonicalPath -Force
            Assert-True (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) "Candidate artifact must not be a symlink: $relativePath"
            Assert-Equal (Get-Sha256 $canonicalPath) ([string]$artifact.Value.sha256) "Candidate artifact hash drifted: $relativePath"
            $artifactBlob = Get-GitBlobText $candidateCommit $relativePath
            Assert-Equal (Get-TextSha256 $artifactBlob) ([string]$artifact.Value.sha256) "Candidate artifact was not frozen by candidate commit: $relativePath"
            $treeEntry = (& git -C $repoRoot ls-tree $candidateCommit -- $relativePath).Trim()
            Assert-True $treeEntry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal) "Candidate artifact must be a regular non-executable Git blob: $relativePath"
            $artifactBody = Get-Content -Raw -Encoding UTF8 -LiteralPath $canonicalPath | ConvertFrom-Json -Depth 50
            Assert-Equal ([string]$artifactBody.artifact_role) ([string]$artifact.Name) "Candidate artifact content role drifted: $relativePath"
            Assert-True (-not [string]::IsNullOrWhiteSpace([string]$artifactBody.schema_version)) "Candidate artifact schema_version missing: $relativePath"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
        $expectedPath = Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId/manifest.json"
        Assert-Equal ([System.IO.Path]::GetFullPath($ManifestPath)) ([System.IO.Path]::GetFullPath($expectedPath)) "Candidate manifest path does not match candidate id"
    }
}

function Assert-CandidateHistoryIntegrity {
    param([string]$ManifestPath, [string]$CurrentRaw, [object]$Authority)
    $relativePath = [System.IO.Path]::GetRelativePath($repoRoot, $ManifestPath).Replace("\", "/")
    $historyCommits = @(& git -C $repoRoot log --first-parent --reverse --format=%H -- $relativePath)
    $previousStatus = ""
    $lastRaw = $null
    foreach ($commit in $historyCommits) {
        & git -C $repoRoot cat-file -e "${commit}:$relativePath" 2>$null
        Assert-True ($LASTEXITCODE -eq 0) "Candidate manifest history contains a deletion: $relativePath at $commit"
        $candidateRaw = Get-GitBlobText $commit $relativePath
        $candidate = $candidateRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $authorityRawAtCommit = Get-GitBlobText $commit "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
        $authorityAtCommit = $authorityRawAtCommit | ConvertFrom-Json -Depth 50
        $productionAtCommit = Get-GitBlobText $commit "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json" | ConvertFrom-Json -Depth 50
        Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $authorityRawAtCommit $authorityAtCommit $productionAtCommit
        $previousStatus = [string]$candidate.candidate_status
        $lastRaw = $candidateRaw
    }
    if ($null -eq $lastRaw -or (Get-TextSha256 $lastRaw) -cne (Get-TextSha256 $CurrentRaw)) {
        $candidate = $CurrentRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
        $currentProduction = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
        Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction
    }
    $currentCandidate = $CurrentRaw | ConvertFrom-Json -Depth 50
    if (@("evaluation_candidate", "promotion_pending", "promoted") -contains [string]$currentCandidate.candidate_status) {
        $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
        $currentProduction = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
        Assert-CandidateActivationSnapshot $currentCandidate ([string]$currentCandidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction
    }
}
