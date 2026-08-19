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
    param([string]$PairDir = "")
    $required = @(
        "manifest.resolved.json",
        "left/artifacts/metrics.json",
        "right/artifacts/metrics.json",
        "left/artifacts/whale-exec.jsonl",
        "right/artifacts/whale-exec.jsonl",
        "left/artifacts/validation.stdout.log",
        "right/artifacts/validation.stdout.log",
        "left/artifacts/git-diff.patch",
        "right/artifacts/git-diff.patch"
    )
    $externalPair = $false
    $externalBenchmarkName = ""
    if (-not [string]::IsNullOrWhiteSpace($PairDir)) {
        $manifestPath = Join-Path $PairDir "manifest.resolved.json"
        if (Test-Path -LiteralPath $manifestPath) {
            try {
                $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
                $externalPair = ($manifest.PSObject.Properties.Name -contains "external_benchmark" -and $null -ne $manifest.external_benchmark)
                $externalBenchmarkName = if ($externalPair -and $manifest.external_benchmark.PSObject.Properties.Name -contains "name") { [string]$manifest.external_benchmark.name } else { "" }
            } catch {}
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($PairDir)) {
        foreach ($observability in @("left/artifacts/observability/action-map-observability.json", "right/artifacts/observability/action-map-observability.json")) {
            if (Test-Path -LiteralPath (Join-Path $PairDir $observability)) { $required += $observability }
        }
        foreach ($proof in @("external-runtime-proof.json", "external-runner-equivalence-proof.json", "external-source-guard-proof.json", "external-isolation-proof.json", "external-e3-proof.json")) {
            if ($externalPair -or (Test-Path -LiteralPath (Join-Path $PairDir $proof))) { $required += $proof }
        }
        if ($externalBenchmarkName -eq "terminal-bench") {
            foreach ($side in @("left", "right")) {
                $runtimeDir = "$side/artifacts/vrun"
                if (-not (Test-Path -LiteralPath (Join-Path $PairDir $runtimeDir))) {
                    $runtimeDir = "$side/artifacts/external-validator-runtime"
                }
                $required += "$runtimeDir/terminal-bench-runtime-manifest.json"
                $required += "$runtimeDir/validation-cleanup-result.json"
            }
        }
    }
    $required
}

function Write-TaskspaceAuditReviewTemplate {
    param(
        [Parameter(Mandatory = $true)][string]$PairDir,
        [string]$ClaimScope = ""
    )
    $required = @(Get-TaskspaceRequiredAuditArtifacts $PairDir)
    $hashes = [ordered]@{}
    foreach ($relative in $required) {
        $path = Join-Path $PairDir $relative
        if (Test-Path -LiteralPath $path) {
            $hashes[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        }
    }
    $json = [ordered]@{
        reviewer = ""
        date = (Get-Date -Format "yyyy-MM-dd")
        artifact_basis = @($required)
        artifact_hashes = $hashes
        decision = ""
        claim_scope = $ClaimScope
        disagreement = $false
        attestations = [ordered]@{
            runtime_proof_reviewed = $false
            runner_equivalence_reviewed = $false
            isolation_proof_reviewed = $false
            source_guard_reviewed = $false
            source_pin_reviewed = $false
            hash_freshness_reviewed = $false
            side_outcomes_reviewed = $false
        }
        decision_rationale = ""
        notes = ""
    }
    $jsonPath = Join-Path $PairDir "audit-review.suggested.json"
    ($json | ConvertTo-Json -Depth 10) | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    $mdPath = Join-Path $PairDir "audit-review.template.md"
    $lines = @(
        "# TaskSpace E3 Artifact Audit Review",
        "",
        "- claim_scope: $ClaimScope",
        "- suggested_json: audit-review.suggested.json",
        "",
        "Inspect the pair artifacts before copying the suggested JSON to audit-review.json.",
        "The final pair-report.md is generated from these audited artifacts and is not part of the audit hash basis.",
        "",
        "Allowed decisions:",
        "- include_taskspace_better",
        "- include_standard_better",
        "- include_no_clear_delta",
        "- exclude_harness_failure",
        "- exclude_invalid_prompt",
        "- exclude_validator_unclear",
        "- exclude_privacy_or_sample_risk"
    )
    $lines | Set-Content -LiteralPath $mdPath -Encoding UTF8
    [pscustomobject]@{ markdown_path = $mdPath; json_path = $jsonPath }
}

function Get-TaskspaceAuditDecisionFromSideOutcomes {
    param([Parameter(Mandatory = $true)]$SideOutcomes)
    $standardSuccess = [bool]$SideOutcomes.standard_success
    $taskspaceSuccess = [bool]$SideOutcomes.taskspace_success
    if ($taskspaceSuccess -and -not $standardSuccess) { return "include_taskspace_better" }
    if ($standardSuccess -and -not $taskspaceSuccess) { return "include_standard_better" }
    if ($standardSuccess -and $taskspaceSuccess) { return "include_no_clear_delta" }
    "exclude_harness_failure"
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
    $artifactBasis = @(if ($review.PSObject.Properties.Name -contains "artifact_basis") { @($review.artifact_basis) })
    $decisionRationale = if ($review.PSObject.Properties.Name -contains "decision_rationale") { [string]$review.decision_rationale } else { "" }
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
    foreach ($required in Get-TaskspaceRequiredAuditArtifacts $PairDir) {
        if (-not $artifactSet.ContainsKey($required)) { $failures.Add("audit_required_artifact_missing:$required") }
    }
    $isExternalPair = $artifactSet.ContainsKey("external-e3-proof.json")
    if ($isExternalPair) {
        if ([string]::IsNullOrWhiteSpace($decisionRationale)) { $failures.Add("audit_decision_rationale_missing") }
        $attestations = if ($review.PSObject.Properties.Name -contains "attestations") { $review.attestations } else { $null }
        foreach ($name in @("runtime_proof_reviewed", "runner_equivalence_reviewed", "isolation_proof_reviewed", "source_guard_reviewed", "source_pin_reviewed", "hash_freshness_reviewed", "side_outcomes_reviewed")) {
            if ($null -eq $attestations -or -not ($attestations.PSObject.Properties.Name -contains $name) -or -not [bool]$attestations.$name) {
                $failures.Add("audit_attestation_missing_or_false:$name")
            }
        }
    }
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
