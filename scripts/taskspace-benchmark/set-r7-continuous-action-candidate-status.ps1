param(
    [Parameter(Mandatory = $true)]
    [string]$CandidateId,
    [Parameter(Mandatory = $true)]
    [ValidateSet("promotion_pending", "rejected", "promoted", "reverted")]
    [string]$ToStatus,
    [Parameter(Mandatory = $true)]
    [string]$EvidencePath,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedHead
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Read-WorktreeCandidate {
    param([string]$Id)
    $candidatePath = Get-R7CandidatePath $Id
    $manifestPath = Join-Path $candidatePath.full "manifest.json"
    $schema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
    [pscustomobject]@{path = $candidatePath; manifest_path = $manifestPath; body = Read-R7StrictJson $manifestPath $schema}
}

function Assert-Evidence {
    param([string]$RelativePath, [string]$Head)
    if ([System.IO.Path]::IsPathRooted($RelativePath) -or $RelativePath.Contains("..")) { throw "R7_TRANSITION_EVIDENCE_PATH_INVALID" }
    $entry = (Invoke-R7Git @("ls-tree", $Head, "--", $RelativePath)) -join "`n"
    if (-not $entry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) { throw "R7_TRANSITION_EVIDENCE_NOT_FROZEN path=$RelativePath" }
    $scratch = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs/$Head-evidence.json"
    [System.IO.Directory]::CreateDirectory((Split-Path $scratch -Parent)) | Out-Null
    [System.IO.File]::WriteAllBytes($scratch, (Get-R7GitBlobBytes $Head $RelativePath))
    [void](Read-R7StrictJson $scratch)
    Get-R7GitBlobSha256 $Head $RelativePath
}

function Set-StatusEvidence {
    param($Manifest, [string]$Status, [string]$Path, [string]$Hash)
    $eventKind = switch ($Status) {
        "promotion_pending" { "evaluation_passed" }
        "rejected" { "evaluation_failed" }
        "reverted" { "post_promotion_failed" }
        default { throw "R7_TRANSITION_EVIDENCE_STATUS_INVALID status=$Status" }
    }
    $Manifest.candidate_status = $Status
    $Manifest.status_evidence = [pscustomobject][ordered]@{
        event_kind = $eventKind
        evidence_path = $Path
        evidence_sha256 = $Hash
    }
}

function Set-TerminalSupersessions {
    param([string]$SuccessorId)
    $manifestPaths = @(Invoke-R7Git @("ls-tree", "-r", "--name-only", "HEAD", "--", $script:R7CandidateRoot) | Where-Object { $_ -match '/manifest\.json$' })
    $changed = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $manifestPaths) {
        if ($path -eq "$script:R7CandidateRoot/$SuccessorId/manifest.json") { continue }
        $id = $path.Split('/')[-2]
        $candidate = Read-WorktreeCandidate $id
        if (@("rejected", "reverted") -notcontains [string]$candidate.body.candidate_status) { continue }
        if ($null -ne $candidate.body.psobject.Properties["superseded_by"]) {
            if ([string]$candidate.body.superseded_by.candidate_id -cne $SuccessorId) { throw "R7_SUPERSESSION_ALREADY_BOUND candidate=$id" }
            continue
        }
        $candidate.body | Add-Member -NotePropertyName superseded_by -NotePropertyValue ([pscustomobject]@{candidate_id = $SuccessorId})
        Write-R7JsonFile $candidate.manifest_path $candidate.body
        $changed.Add($path)
    }
    $changed.ToArray()
}

Assert-R7CleanWorktree
[void](Assert-R7ToolchainWorktree)
$head = (Invoke-R7Git @("rev-parse", "HEAD"))[0].Trim()
if ($head -cne $ExpectedHead) { throw "R7_TRANSITION_HEAD_DRIFT expected=$ExpectedHead actual=$head" }
$evidenceHash = Assert-Evidence $EvidencePath $head
$candidate = Read-WorktreeCandidate $CandidateId
$fromStatus = [string]$candidate.body.candidate_status
$allowed = @{
    evaluation_candidate = @("promotion_pending", "rejected")
    promotion_pending = @("promoted", "rejected")
    rejected = @()
    promoted = @("reverted")
    reverted = @()
}
if ($allowed[$fromStatus] -notcontains $ToStatus) { throw "R7_TRANSITION_ILLEGAL from=$fromStatus to=$ToStatus" }

& pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "test-r7-continuous-action-candidate.ps1") -CandidateId $CandidateId -TargetCommit $head -RequireStatus $fromStatus | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_TRANSITION_PREFLIGHT_FAILED" }

$expectedChanged = [System.Collections.Generic.List[string]]::new()
$expectedChanged.Add("$($candidate.path.relative)/manifest.json")
switch ($ToStatus) {
    "promotion_pending" {
        Set-StatusEvidence $candidate.body $ToStatus $EvidencePath $evidenceHash
        foreach ($path in @(Set-TerminalSupersessions $CandidateId)) { $expectedChanged.Add($path) }
        Write-R7JsonFile $candidate.manifest_path $candidate.body
    }
    "rejected" {
        Set-StatusEvidence $candidate.body $ToStatus $EvidencePath $evidenceHash
        Write-R7JsonFile $candidate.manifest_path $candidate.body
    }
    "promoted" {
        if ([string]$candidate.body.status_evidence.evidence_path -cne $EvidencePath -or
            [string]$candidate.body.status_evidence.evidence_sha256 -cne $evidenceHash) {
            throw "R7_PROMOTION_EVIDENCE_NOT_PENDING_EVIDENCE"
        }
        $baseline = Get-R7FirstAddAnchor $script:R7BaselineAnchorPath "continuous_action_production_baseline"
        $authority = (Get-R7GitBlobText $baseline.parent_commit $script:R7AuthorityPath) | ConvertFrom-Json -Depth 100
        $production = (Get-R7GitBlobText $baseline.parent_commit $script:R7ProductionPath) | ConvertFrom-Json -Depth 100
        $promotedAuthority = Invoke-R7JsonPatch $authority @($candidate.body.promotion.authority_patch)
        $promotedProduction = Invoke-R7JsonPatch $production @($candidate.body.promotion.production_patch)
        $promotedCandidate = Invoke-R7JsonPatch $candidate.body @($candidate.body.promotion.candidate_patch)
        Write-R7JsonFile (Join-Path $script:R7RepoRoot $script:R7AuthorityPath) $promotedAuthority
        Write-R7JsonFile (Join-Path $script:R7RepoRoot $script:R7ProductionPath) $promotedProduction
        Write-R7JsonFile $candidate.manifest_path $promotedCandidate
        $expectedChanged.Add($script:R7AuthorityPath)
        $expectedChanged.Add($script:R7ProductionPath)
    }
    "reverted" {
        $baseline = Get-R7FirstAddAnchor $script:R7BaselineAnchorPath "continuous_action_production_baseline"
        $artifactSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json"
        $rollbackPath = Join-Path $script:R7RepoRoot ([string]$candidate.body.artifact_hashes.rollback_manifest.path)
        $rollback = Read-R7StrictJson $rollbackPath $artifactSchema
        foreach ($entry in @($rollback.changed_path_inventory)) {
            $action = [string]$entry.rollback_action
            if ($action -eq "preserve") { continue }
            $relative = [string]$entry.path
            $full = Join-Path $script:R7RepoRoot $relative
            if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "R7_ROLLBACK_CANDIDATE_FILE_MISSING path=$relative" }
            if ((Get-R7Sha256File $full) -cne [string]$entry.candidate_sha256) { throw "R7_ROLLBACK_CANDIDATE_DRIFT path=$relative" }
            if ($action -eq "restore") {
                [System.IO.Directory]::CreateDirectory((Split-Path $full -Parent)) | Out-Null
                [System.IO.File]::WriteAllBytes($full, (Get-R7GitBlobBytes ([string]$rollback.baseline_commit) $relative))
            } elseif ($action -eq "remove") {
                Invoke-R7Git @("rm", "--", $relative) | Out-Null
            } else {
                throw "R7_ROLLBACK_ACTION_INVALID path=$relative action=$action"
            }
            $expectedChanged.Add($relative)
        }
        [System.IO.File]::WriteAllBytes((Join-Path $script:R7RepoRoot $script:R7AuthorityPath), (Get-R7GitBlobBytes $baseline.parent_commit $script:R7AuthorityPath))
        [System.IO.File]::WriteAllBytes((Join-Path $script:R7RepoRoot $script:R7ProductionPath), (Get-R7GitBlobBytes $baseline.parent_commit $script:R7ProductionPath))
        Set-StatusEvidence $candidate.body $ToStatus $EvidencePath $evidenceHash
        Write-R7JsonFile $candidate.manifest_path $candidate.body
        $expectedChanged.Add($script:R7AuthorityPath)
        $expectedChanged.Add($script:R7ProductionPath)
    }
}

$actualChanged = @(Invoke-R7Git @("diff", "--name-only") | Sort-Object)
$expected = @($expectedChanged.ToArray() | Sort-Object -Unique)
if (($actualChanged -join "`n") -cne ($expected -join "`n")) {
    throw "R7_TRANSITION_CHANGED_PATHS expected=$($expected -join ',') actual=$($actualChanged -join ',')"
}
Invoke-R7Git (@("add", "-A", "--") + $expected) | Out-Null
Invoke-R7Git @("commit", "-m", "state(r7): $fromStatus to $ToStatus for $CandidateId") | Out-Null
$eventCommit = (Invoke-R7Git @("rev-parse", "HEAD"))[0].Trim()
& pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "test-r7-continuous-action-candidate.ps1") -CandidateId $CandidateId -TargetCommit $eventCommit -RequireStatus $ToStatus | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_TRANSITION_POSTCOMMIT_FAILED commit=$eventCommit" }
[pscustomobject][ordered]@{candidate_id = $CandidateId; from = $fromStatus; to = $ToStatus; event_commit = $eventCommit; evidence_sha256 = $evidenceHash} | ConvertTo-Json -Compress
