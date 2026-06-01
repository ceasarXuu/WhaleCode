function Get-TaskspaceAuditReviewPath {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [string]$AuditReviewRoot = "",
        [int]$Repeat = 0
    )
    $local = Join-Path $PairDir "audit-review.json"
    if (Test-Path -LiteralPath $local) { return (Resolve-Path -LiteralPath $local).Path }
    if ([string]::IsNullOrWhiteSpace($AuditReviewRoot)) { return "" }
    $candidateNames = @()
    if ($Repeat -gt 0) { $candidateNames += ("pair-{0:000}\audit-review.json" -f $Repeat) }
    foreach ($name in $candidateNames) {
        $candidate = Join-Path $AuditReviewRoot $name
        if (Test-Path -LiteralPath $candidate) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    ""
}

function Test-TaskspaceAuditArtifactPath {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][string]$ArtifactPath
    )
    if ([System.IO.Path]::IsPathRooted($ArtifactPath)) { return $false }
    if ($ArtifactPath -match '(^|[\\/])\.\.([\\/]|$)') { return $false }
    $candidate = Join-Path $PairDir $ArtifactPath
    Test-Path -LiteralPath $candidate
}

function Get-TaskspaceAuditArtifactSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [Parameter(Mandatory = $true)][string]$ArtifactPath
    )
    $candidate = Join-Path $PairDir $ArtifactPath
    (Get-FileHash -Algorithm SHA256 -LiteralPath $candidate).Hash.ToLowerInvariant()
}

function Get-TaskspaceRequiredAuditArtifacts {
    @(
        "manifest.resolved.json",
        "pair-report.md",
        "left/artifacts/metrics.json",
        "right/artifacts/metrics.json",
        "left/artifacts/whale-exec.jsonl",
        "right/artifacts/whale-exec.jsonl",
        "left/artifacts/validation.stdout.log",
        "right/artifacts/validation.stdout.log",
        "left/artifacts/git-diff.patch",
        "right/artifacts/git-diff.patch"
    )
}

function Get-TaskspaceAuditReview {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [string]$AuditReviewRoot = "",
        [int]$Repeat = 0,
        [string]$ExpectedClaimScope = ""
    )
    $path = Get-TaskspaceAuditReviewPath $PairDir $AuditReviewRoot $Repeat
    $failures = New-Object System.Collections.Generic.List[string]
    if ([string]::IsNullOrWhiteSpace($path)) {
        return [pscustomobject]@{
            completed = $false
            decision = ""
            disagreement = $false
            reviewer = ""
            claim_scope = ""
            source_path = ""
            failures = @("audit_review_missing")
        }
    }
    try {
        $review = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    } catch {
        $failures.Add("audit_review_json_invalid")
        return [pscustomobject]@{
            completed = $false
            decision = ""
            disagreement = $false
            reviewer = ""
            claim_scope = ""
            source_path = $path
            failures = @($failures.ToArray())
        }
    }
    $decision = if ($review.PSObject.Properties.Name -contains "decision") { [string]$review.decision } else { "" }
    $reviewer = if ($review.PSObject.Properties.Name -contains "reviewer") { [string]$review.reviewer } else { "" }
    $date = if ($review.PSObject.Properties.Name -contains "date") { [string]$review.date } else { "" }
    $claimScope = if ($review.PSObject.Properties.Name -contains "claim_scope") { [string]$review.claim_scope } else { "" }
    $disagreement = ($review.PSObject.Properties.Name -contains "disagreement" -and [bool]$review.disagreement)
    $artifactBasis = if ($review.PSObject.Properties.Name -contains "artifact_basis") { @($review.artifact_basis) } else { @() }
    $artifactHashes = @{}
    if ($review.PSObject.Properties.Name -contains "artifact_hashes") {
        foreach ($property in $review.artifact_hashes.PSObject.Properties) {
            $artifactHashes[$property.Name.Replace("\", "/")] = [string]$property.Value
        }
    }
    if ([string]::IsNullOrWhiteSpace($decision)) { $failures.Add("audit_decision_missing") }
    if ([string]::IsNullOrWhiteSpace($reviewer)) { $failures.Add("audit_reviewer_missing") }
    if ([string]::IsNullOrWhiteSpace($date)) { $failures.Add("audit_date_missing") }
    if ([string]::IsNullOrWhiteSpace($claimScope)) { $failures.Add("audit_claim_scope_missing") }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedClaimScope) -and $claimScope -ne $ExpectedClaimScope) {
        $failures.Add("audit_claim_scope_mismatch")
    }
    if ($artifactBasis.Count -eq 0) { $failures.Add("audit_artifact_basis_missing") }
    if ($artifactHashes.Count -eq 0) { $failures.Add("audit_artifact_hashes_missing") }
    $artifactSet = @{}
    foreach ($artifact in $artifactBasis) {
        $artifactText = [string]$artifact
        if ([string]::IsNullOrWhiteSpace($artifactText)) {
            $failures.Add("audit_artifact_basis_blank")
        } elseif ([System.IO.Path]::IsPathRooted($artifactText) -or $artifactText -match '(^|[\\/])\.\.([\\/]|$)') {
            $failures.Add("audit_artifact_path_not_pair_relative:$artifactText")
        } elseif (-not (Test-TaskspaceAuditArtifactPath $PairDir $artifactText)) {
            $failures.Add("audit_artifact_missing:$artifactText")
        } else {
            $normalizedArtifact = $artifactText.Replace("\", "/")
            $artifactSet[$normalizedArtifact] = $true
            if (-not $artifactHashes.ContainsKey($normalizedArtifact)) {
                $failures.Add("audit_artifact_hash_missing:$normalizedArtifact")
            } else {
                $actualHash = Get-TaskspaceAuditArtifactSha256 $PairDir $artifactText
                $expectedHash = ([string]$artifactHashes[$normalizedArtifact]).ToLowerInvariant()
                if ($actualHash -ne $expectedHash) {
                    $failures.Add("audit_artifact_hash_mismatch:$normalizedArtifact")
                }
            }
        }
    }
    foreach ($required in Get-TaskspaceRequiredAuditArtifacts) {
        if (-not $artifactSet.ContainsKey($required)) { $failures.Add("audit_required_artifact_missing:$required") }
    }
    $hasTaskspaceObservability = @($artifactSet.Keys | Where-Object { $_ -like "*/artifacts/observability/action-map-observability.json" }).Count -gt 0
    if (-not $hasTaskspaceObservability) { $failures.Add("audit_taskspace_observability_missing") }
    [pscustomobject]@{
        completed = ($failures.Count -eq 0)
        decision = $decision
        disagreement = $disagreement
        reviewer = $reviewer
        date = $date
        claim_scope = $claimScope
        source_path = $path
        artifact_basis = @($artifactBasis)
        artifact_hashes = $artifactHashes
        failures = @($failures.ToArray())
    }
}
