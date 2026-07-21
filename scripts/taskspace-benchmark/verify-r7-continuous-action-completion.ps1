param(
    [Parameter(Mandatory = $true)]
    [string]$TargetCommit,
    [Parameter(Mandatory = $true)]
    [string]$ToolchainAddCommit,
    [Parameter(Mandatory = $true)]
    [string]$RequiredCheckRunId,
    [Parameter(Mandatory = $true)]
    [string]$RequiredCheckName,
    [Parameter(Mandatory = $true)]
    [string]$ExportManifestPath,
    [Parameter(Mandatory = $true)]
    [string]$AttestationPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Assert-Equal {
    param($Actual, $Expected, [string]$Code)
    if ($Actual -cne $Expected) { throw "$Code expected=$Expected actual=$Actual" }
}

function Read-TargetJson {
    param([string]$Path, [string]$Label)
    $scratch = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs/$TargetCommit-$Label.json"
    [System.IO.Directory]::CreateDirectory((Split-Path $scratch -Parent)) | Out-Null
    [System.IO.File]::WriteAllBytes($scratch, (Get-R7GitBlobBytes $TargetCommit $Path))
    Read-R7StrictJson $scratch
}

if ($TargetCommit -notmatch '^[0-9a-f]{40}$') { throw "R7_COMPLETION_TARGET_INVALID" }
if ($ToolchainAddCommit -notmatch '^[0-9a-f]{40}$') { throw "R7_COMPLETION_TOOLCHAIN_COMMIT_INVALID" }
if ($RequiredCheckRunId -notmatch '^[0-9]+$') { throw "R7_COMPLETION_RUN_ID_INVALID" }
Assert-Equal $RequiredCheckName "r7-continuous-action-completion" "R7_COMPLETION_CHECK_NAME"
if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_SHA)) { Assert-Equal $env:GITHUB_SHA $TargetCommit "R7_COMPLETION_GITHUB_SHA" }
if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_RUN_ID)) { Assert-Equal $env:GITHUB_RUN_ID $RequiredCheckRunId "R7_COMPLETION_GITHUB_RUN_ID" }

$head = (Invoke-R7Git @("rev-parse", "HEAD"))[0].Trim()
Assert-Equal $head $TargetCommit "R7_COMPLETION_CHECKOUT_TARGET"
Assert-R7CleanWorktree
$toolchain = Get-R7FirstAddAnchor $script:R7ToolchainAnchorPath "continuous_action_v2_toolchain"
Assert-Equal $toolchain.add_commit $ToolchainAddCommit "R7_COMPLETION_TOOLCHAIN_ADD"
[void](Assert-R7ToolchainWorktree)

$exportManifest = Read-R7StrictJson $ExportManifestPath
Assert-Equal ([string]$exportManifest.target_commit) $TargetCommit "R7_EXPORT_TARGET"
Assert-Equal ([string]$exportManifest.toolchain_add_commit) $ToolchainAddCommit "R7_EXPORT_TOOLCHAIN_ADD"
Assert-Equal ([string]$exportManifest.toolchain_parent_commit) $toolchain.parent_commit "R7_EXPORT_TOOLCHAIN_PARENT"
$expectedRoles = @($toolchain.body.artifacts | ForEach-Object { [string]$_.role } | Sort-Object)
$actualRoles = @($exportManifest.artifacts | ForEach-Object { [string]$_.role } | Sort-Object)
Assert-Equal ($actualRoles -join "`n") ($expectedRoles -join "`n") "R7_EXPORT_ROLE_SET"
foreach ($export in @($exportManifest.artifacts)) {
    $anchorArtifact = @($toolchain.body.artifacts | Where-Object { [string]$_.role -eq [string]$export.role })
    Assert-Equal $anchorArtifact.Count 1 "R7_EXPORT_ROLE_NOT_UNIQUE"
    Assert-Equal ([string]$export.source_path) ([string]$anchorArtifact[0].path) "R7_EXPORT_SOURCE_PATH"
    Assert-Equal ([string]$export.sha256) ([string]$anchorArtifact[0].sha256) "R7_EXPORT_DECLARED_HASH"
    Assert-Equal (Get-R7Sha256File ([string]$export.exported_path)) ([string]$anchorArtifact[0].sha256) "R7_EXPORT_FILE_HASH"
}

$production = Read-TargetJson $script:R7ProductionPath "completion-production"
$candidateId = [string]$production.promoted_candidate_id
if ($candidateId -notmatch '^[0-9a-f]{64}$') { throw "R7_COMPLETION_ACTIVE_CANDIDATE_MISSING" }
$candidateManifests = @(Invoke-R7Git @("ls-tree", "-r", "--name-only", $TargetCommit, "--", $script:R7CandidateRoot) | Where-Object { $_ -match '/manifest\.json$' })
if ($candidateManifests.Count -lt 1) { throw "R7_COMPLETION_CANDIDATE_SET_EMPTY" }
$promotedIds = [System.Collections.Generic.List[string]]::new()
foreach ($manifestPath in $candidateManifests) {
    $id = $manifestPath.Split('/')[-2]
    & pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "test-r7-continuous-action-candidate.ps1") -CandidateId $id -TargetCommit $TargetCommit | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_CANDIDATE_VERIFIER_FAILED candidate=$id" }
    $body = Read-TargetJson $manifestPath "candidate-set-$id"
    if ([string]$body.candidate_status -eq "promoted") { $promotedIds.Add($id) }
}
Assert-Equal $promotedIds.Count 1 "R7_COMPLETION_PROMOTED_COUNT"
Assert-Equal $promotedIds[0] $candidateId "R7_COMPLETION_ACTIVE_PROMOTED_DRIFT"
$candidate = Read-TargetJson "$script:R7CandidateRoot/$candidateId/manifest.json" "completion-candidate"
$evidence = Read-TargetJson ([string]$candidate.status_evidence.evidence_path) "completion-evidence"
Assert-Equal ([string]$evidence.artifact_role) "continuous_action_completion_evidence" "R7_COMPLETION_EVIDENCE_ROLE"
Assert-Equal ([string]$evidence.candidate_id) $candidateId "R7_COMPLETION_EVIDENCE_CANDIDATE"
Assert-Equal ([bool]$evidence.independent_docker) $true "R7_COMPLETION_DOCKER"
Assert-Equal ([bool]$evidence.rollback_drill_passed) $true "R7_COMPLETION_ROLLBACK_DRILL"
Assert-Equal ([bool]$evidence.production_tests_passed) $true "R7_COMPLETION_PRODUCTION_TESTS"
Assert-Equal ([bool]$evidence.logs_complete) $true "R7_COMPLETION_LOGS"
$requiredGates = @(
    "correctness_noninferior", "transition_carrier_rate", "carrier_conservation",
    "standalone_zero", "h003_zero", "patch_input_exact", "typed_output_exact",
    "request_noninferior", "token_noninferior", "time_noninferior", "cache_contract"
)
foreach ($gate in $requiredGates) {
    Assert-Equal ([bool]$evidence.hard_gates.$gate) $true "R7_COMPLETION_HARD_GATE gate=$gate"
}
foreach ($referenceName in @("three_arm_report", "production_trace", "rollback_report")) {
    $reference = $evidence.$referenceName
    if ([string]::IsNullOrWhiteSpace([string]$reference.path)) { throw "R7_COMPLETION_REFERENCE_PATH role=$referenceName" }
    Assert-Equal (Get-R7GitBlobSha256 $TargetCommit ([string]$reference.path)) ([string]$reference.sha256) "R7_COMPLETION_REFERENCE_HASH role=$referenceName"
}

$attestation = [pscustomobject][ordered]@{
    schema_version = 1
    attestation_kind = "r7_continuous_action_completion"
    verified = $true
    target_commit = $TargetCommit
    candidate_id = $candidateId
    toolchain_add_commit = $ToolchainAddCommit
    toolchain_parent_commit = $toolchain.parent_commit
    required_check = [pscustomobject][ordered]@{name = $RequiredCheckName; run_id = $RequiredCheckRunId; target_commit = $TargetCommit}
    completion_evidence = [pscustomobject][ordered]@{path = $candidate.status_evidence.evidence_path; sha256 = $candidate.status_evidence.evidence_sha256}
    exported_artifacts = @($exportManifest.artifacts | ForEach-Object { [pscustomobject][ordered]@{role = $_.role; source_path = $_.source_path; sha256 = $_.sha256} })
}
[System.IO.Directory]::CreateDirectory((Split-Path $AttestationPath -Parent)) | Out-Null
Write-R7JsonFile $AttestationPath $attestation
[void](Read-R7StrictJson $AttestationPath)
Write-Output ($attestation | ConvertTo-Json -Depth 20 -Compress)
