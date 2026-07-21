param(
    [Parameter(Mandatory = $true)][string]$TargetCommit,
    [Parameter(Mandatory = $true)][string]$ToolchainAddCommit,
    [Parameter(Mandatory = $true)][string]$RequiredCheckRunId,
    [Parameter(Mandatory = $true)][string]$RequiredCheckRunAttempt,
    [Parameter(Mandatory = $true)][string]$RequiredCheckName,
    [Parameter(Mandatory = $true)][string]$Repository,
    [Parameter(Mandatory = $true)][string]$WorkflowRef,
    [Parameter(Mandatory = $true)][string]$WorkflowSha,
    [Parameter(Mandatory = $true)][string]$EventName,
    [Parameter(Mandatory = $true)][string]$GitSha,
    [Parameter(Mandatory = $true)][string]$GitRef,
    [Parameter(Mandatory = $true)][string]$ExecutionImage,
    [Parameter(Mandatory = $true)][string]$PowerShellVersion,
    [Parameter(Mandatory = $true)][string]$ExportManifestPath,
    [Parameter(Mandatory = $true)][string]$AttestationPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Assert-Equal {
    param($Actual, $Expected, [string]$Code)
    if ($Actual -cne $Expected) { throw "$Code expected=$Expected actual=$Actual" }
}

function Read-CommitJson {
    param([string]$Commit, [string]$Path, [string]$Label, [string]$Schema = "")
    $scratch = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs/$Commit-$Label.json"
    [System.IO.Directory]::CreateDirectory((Split-Path $scratch -Parent)) | Out-Null
    [System.IO.File]::WriteAllBytes($scratch, (Get-R7GitBlobBytes $Commit $Path))
    Read-R7StrictJson $scratch $Schema
}

function Assert-GitReference {
    param($Reference, [string]$Commit, [string]$Label)
    $path = [string]$Reference.path
    $entry = (Invoke-R7Git @("ls-tree", $Commit, "--", $path)) -join "`n"
    if (-not $entry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) { throw "R7_COMPLETION_REFERENCE_MODE role=$Label" }
    Assert-Equal (Get-R7GitBlobSha256 $Commit $path) ([string]$Reference.sha256) "R7_COMPLETION_REFERENCE_HASH role=$Label"
}

function Invoke-PinnedCandidateVerifier {
    param([string]$Script, [string]$CandidateId, [string]$Commit, [string]$Status = "")
    $arguments = @("-NoLogo", "-NoProfile", "-File", $Script, "-CandidateId", $CandidateId, "-TargetCommit", $Commit)
    if (-not [string]::IsNullOrWhiteSpace($Status)) { $arguments += @("-RequireStatus", $Status) }
    & pwsh @arguments | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_CANDIDATE_VERIFIER_FAILED candidate=$CandidateId status=$Status" }
}

function Get-CandidateManifests {
    param([string]$Commit)
    @(Invoke-R7Git @("ls-tree", "-r", "--name-only", $Commit, "--", $script:R7CandidateRoot) | Where-Object { $_ -match '/manifest\.json$' })
}

if ($TargetCommit -notmatch '^[0-9a-f]{40}$' -or $ToolchainAddCommit -notmatch '^[0-9a-f]{40}$' -or $WorkflowSha -notmatch '^[0-9a-f]{40}$' -or $GitSha -notmatch '^[0-9a-f]{40}$') { throw "R7_COMPLETION_COMMIT_ID_INVALID" }
if ($RequiredCheckRunId -notmatch '^[0-9]+$' -or $RequiredCheckRunAttempt -notmatch '^[0-9]+$') { throw "R7_COMPLETION_RUN_ID_INVALID" }
Assert-Equal $RequiredCheckName "r7-continuous-action-completion" "R7_COMPLETION_CHECK_NAME"
Assert-Equal $Repository "ceasarXuu/WhaleCode" "R7_COMPLETION_REPOSITORY"
Assert-Equal $EventName "push" "R7_COMPLETION_EVENT"
if (-not $GitRef.StartsWith("refs/heads/", [System.StringComparison]::Ordinal)) { throw "R7_COMPLETION_GIT_REF value=$GitRef" }
Assert-Equal $GitSha $TargetCommit "R7_COMPLETION_GITHUB_SHA"
Assert-Equal $WorkflowSha $TargetCommit "R7_COMPLETION_WORKFLOW_SHA"
Assert-Equal $WorkflowRef "$Repository/.github/workflows/r7-continuous-action-completion.yml@$GitRef" "R7_COMPLETION_WORKFLOW_REF"
foreach ($binding in @(
    @("GITHUB_REPOSITORY", $Repository), @("GITHUB_WORKFLOW_REF", $WorkflowRef), @("GITHUB_WORKFLOW_SHA", $WorkflowSha),
    @("GITHUB_EVENT_NAME", $EventName), @("GITHUB_SHA", $GitSha), @("GITHUB_REF", $GitRef),
    @("GITHUB_RUN_ID", $RequiredCheckRunId), @("GITHUB_RUN_ATTEMPT", $RequiredCheckRunAttempt)
)) {
    $actual = [Environment]::GetEnvironmentVariable($binding[0])
    if (-not [string]::IsNullOrWhiteSpace($actual)) { Assert-Equal $actual $binding[1] "R7_COMPLETION_GITHUB_BINDING name=$($binding[0])" }
}

Assert-Equal (Get-R7GitLine @("rev-parse", "HEAD")) $TargetCommit "R7_COMPLETION_CHECKOUT_TARGET"
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
$byRole = @{}
foreach ($export in @($exportManifest.artifacts)) {
    $anchorArtifact = @($toolchain.body.artifacts | Where-Object { [string]$_.role -eq [string]$export.role })
    Assert-Equal $anchorArtifact.Count 1 "R7_EXPORT_ROLE_NOT_UNIQUE"
    Assert-Equal ([string]$export.source_path) ([string]$anchorArtifact[0].path) "R7_EXPORT_SOURCE_PATH"
    Assert-Equal ([string]$export.sha256) ([string]$anchorArtifact[0].sha256) "R7_EXPORT_DECLARED_HASH"
    Assert-Equal (Get-R7Sha256File ([string]$export.exported_path)) ([string]$anchorArtifact[0].sha256) "R7_EXPORT_FILE_HASH"
    $byRole[[string]$export.role] = [string]$export.exported_path
}

$candidateManifests = Get-CandidateManifests $TargetCommit
if ($candidateManifests.Count -lt 1) { throw "R7_COMPLETION_CANDIDATE_SET_EMPTY" }
foreach ($manifestPath in $candidateManifests) {
    Invoke-PinnedCandidateVerifier $byRole.candidate_verifier $manifestPath.Split('/')[-2] $TargetCommit
}
& pwsh -NoLogo -NoProfile -File $byRole.candidate_set_verifier -TargetCommit $TargetCommit | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_CANDIDATE_SET_FAILED" }

$production = Read-CommitJson $TargetCommit $script:R7ProductionPath "completion-production"
$promoted = @()
foreach ($path in $candidateManifests) {
    $body = Read-CommitJson $TargetCommit $path "completion-$($path.Split('/')[-2])"
    if ([string]$body.candidate_status -eq "promoted") { $promoted += $body }
}
$mode = if ($promoted.Count -eq 1 -and [string]$production.promoted_candidate_id -eq [string]$promoted[0].candidate_id) { "promotion" } else { "revert" }
$attestationDetails = $null

if ($mode -eq "promotion") {
    $candidate = $promoted[0]
    $candidateId = [string]$candidate.candidate_id
    $evidencePath = [string]$candidate.status_evidence.evidence_path
    $evidence = Read-CommitJson $TargetCommit $evidencePath "completion-evidence" $env:R7_COMPLETION_EVIDENCE_SCHEMA_PATH
    Assert-Equal ([string]$evidence.candidate_id) $candidateId "R7_COMPLETION_EVIDENCE_CANDIDATE"
    Assert-Equal ([string]$evidence.candidate_commit) ([string]$candidate.candidate_commit) "R7_COMPLETION_EVIDENCE_COMMIT"
    foreach ($name in @("evaluation_contract", "raw_run_set", "evaluation_result")) { Assert-GitReference $evidence.$name $TargetCommit $name }
    Assert-Equal ([string]$evidence.evaluation_contract.path) ([string]$candidate.artifact_hashes.continuous_action_evaluation.path) "R7_COMPLETION_EVALUATION_PATH"
    Assert-Equal ([string]$evidence.evaluation_contract.sha256) ([string]$candidate.artifact_hashes.continuous_action_evaluation.sha256) "R7_COMPLETION_EVALUATION_HASH"
    $runSet = Read-CommitJson $TargetCommit ([string]$evidence.raw_run_set.path) "raw-run-set" $env:R7_RUN_SET_SCHEMA_PATH
    $baseline = Get-R7BaselineAnchor $TargetCommit
    Assert-Equal ([string]$runSet.identity.candidate_commit) ([string]$candidate.candidate_commit) "R7_COMPLETION_RUNSET_CANDIDATE"
    Assert-Equal ([string]$runSet.identity.standard_commit) ([string]$candidate.candidate_commit) "R7_COMPLETION_RUNSET_STANDARD"
    Assert-Equal ([string]$runSet.identity.sibling_baseline_commit) $baseline.parent_commit "R7_COMPLETION_RUNSET_BASELINE"
    $evaluationOutput = Join-Path $script:R7RepoRoot "target/r7-toolchain/recomputed-evaluation-$TargetCommit.json"
    & pwsh -NoLogo -NoProfile -File $byRole.evaluation_launcher -RunSetPath (Join-Path $script:R7RepoRoot ([string]$evidence.raw_run_set.path)) -EvaluationContractPath (Join-Path $script:R7RepoRoot ([string]$evidence.evaluation_contract.path)) -RunArtifactRoot $script:R7RepoRoot -OutputPath $evaluationOutput
    if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_EVALUATOR_FAILED" }
    Assert-Equal (Get-R7Sha256File $evaluationOutput) ([string]$evidence.evaluation_result.sha256) "R7_COMPLETION_RESULT_RECOMPUTE"
    $evaluationResult = Read-R7StrictJson $evaluationOutput $env:R7_EVALUATION_RESULT_SCHEMA_PATH
    Assert-Equal ([string]$evaluationResult.decision) "pass" "R7_COMPLETION_EVALUATION_DECISION"

    $drillRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/rollback-drill-$RequiredCheckRunId-$RequiredCheckRunAttempt"
    & git clone --quiet --no-hardlinks $script:R7RepoRoot $drillRoot
    if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_DRILL_CLONE_FAILED" }
    & git -C $drillRoot checkout --quiet -b "r7-completion-drill-$RequiredCheckRunId" $TargetCommit
    if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_DRILL_CHECKOUT_FAILED" }
    $priorRoot = $env:R7_REPO_ROOT
    try {
        $env:R7_REPO_ROOT = $drillRoot
        & pwsh -NoLogo -NoProfile -File $byRole.transition_command -CandidateId $candidateId -ToStatus reverted -EvidencePath $evidencePath -ExpectedHead $TargetCommit | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "R7_COMPLETION_ROLLBACK_DRILL_FAILED" }
        $drillCommit = (& git -C $drillRoot rev-parse HEAD).Trim()
        Invoke-PinnedCandidateVerifier $byRole.candidate_verifier $candidateId $drillCommit "reverted"
    } finally { $env:R7_REPO_ROOT = $priorRoot }
    Assert-Equal (Get-R7Sha256File (Join-Path $drillRoot $script:R7AuthorityPath)) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7AuthorityPath) "R7_COMPLETION_DRILL_AUTHORITY"
    Assert-Equal (Get-R7Sha256File (Join-Path $drillRoot $script:R7ProductionPath)) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7ProductionPath) "R7_COMPLETION_DRILL_PRODUCTION"
    $attestationDetails = [pscustomobject][ordered]@{candidate_id = $candidateId; evidence_path = $evidencePath; evidence_sha256 = $candidate.status_evidence.evidence_sha256; evaluation_result_sha256 = $evidence.evaluation_result.sha256; rollback_drill_commit = $drillCommit}
} else {
    $parent = Get-R7GitLine @("rev-parse", "$TargetCommit^1")
    $changed = @(Invoke-R7Git @("diff", "--name-only", $parent, $TargetCommit, "--", $script:R7CandidateRoot) | Where-Object { $_ -match '/manifest\.json$' })
    if ($changed.Count -ne 1) { throw "R7_COMPLETION_REVERT_EVENT_COUNT count=$($changed.Count)" }
    $before = Read-CommitJson $parent $changed[0] "revert-before"
    $after = Read-CommitJson $TargetCommit $changed[0] "revert-after"
    Assert-Equal ([string]$before.candidate_status) "promoted" "R7_COMPLETION_REVERT_FROM"
    Assert-Equal ([string]$after.candidate_status) "reverted" "R7_COMPLETION_REVERT_TO"
    $baseline = Get-R7BaselineAnchor $TargetCommit
    Assert-Equal (Get-R7GitBlobSha256 $TargetCommit $script:R7AuthorityPath) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7AuthorityPath) "R7_COMPLETION_REVERT_AUTHORITY"
    Assert-Equal (Get-R7GitBlobSha256 $TargetCommit $script:R7ProductionPath) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7ProductionPath) "R7_COMPLETION_REVERT_PRODUCTION"
    Invoke-PinnedCandidateVerifier $byRole.candidate_verifier ([string]$after.candidate_id) $TargetCommit "reverted"
    $attestationDetails = [pscustomobject][ordered]@{candidate_id = $after.candidate_id; evidence_path = $after.status_evidence.evidence_path; evidence_sha256 = $after.status_evidence.evidence_sha256; reverted_from_commit = $parent}
}

$attestation = [pscustomobject][ordered]@{
    schema_version = 2; attestation_kind = "r7_continuous_action_completion"; verified = $true; event_kind = $mode
    target_commit = $TargetCommit; toolchain_add_commit = $ToolchainAddCommit; toolchain_parent_commit = $toolchain.parent_commit
    required_check = [pscustomobject][ordered]@{name = $RequiredCheckName; repository = $Repository; workflow_ref = $WorkflowRef; workflow_sha = $WorkflowSha; git_sha = $GitSha; git_ref = $GitRef; event_name = $EventName; run_id = $RequiredCheckRunId; run_attempt = $RequiredCheckRunAttempt; target_commit = $TargetCommit; execution_image = $ExecutionImage; powershell_version = $PowerShellVersion}
    details = $attestationDetails
    exported_artifacts = @($exportManifest.artifacts | ForEach-Object { [pscustomobject][ordered]@{role = $_.role; source_path = $_.source_path; sha256 = $_.sha256} })
}
[System.IO.Directory]::CreateDirectory((Split-Path $AttestationPath -Parent)) | Out-Null
Write-R7JsonFile $AttestationPath $attestation
[void](Read-R7StrictJson $AttestationPath)
Write-Output ($attestation | ConvertTo-Json -Depth 20 -Compress)
