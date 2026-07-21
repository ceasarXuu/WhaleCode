param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactSourceDirectory,
    [string]$CommitMessagePrefix = "test(r7): add continuous action candidate"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function New-RollbackArtifact {
    param($Baseline, $Toolchain)
    $baselineCommit = [string]$Baseline.parent_commit
    $head = Get-R7GitLine @("rev-parse", "HEAD")
    $changedPaths = @(Invoke-R7Git @("diff", "--name-only", "$baselineCommit..$head") | Sort-Object -Unique)
    if ($changedPaths.Count -eq 0) { throw "R7_ROLLBACK_INVENTORY_EMPTY" }
    $pinned = @($Toolchain.body.artifacts | ForEach-Object { [string]$_.path })
    $inventory = foreach ($path in $changedPaths) {
        $candidateHash = Get-R7GitBlobSha256 $head $path
        $baselineProbe = Invoke-R7Git @("cat-file", "-e", "${baselineCommit}:$path") -AllowFailure
        $existsAtBaseline = $LASTEXITCODE -eq 0
        $preserve = $pinned -contains $path -or
            $path.StartsWith("docs/", [System.StringComparison]::Ordinal) -or
            $path.StartsWith("benchmarks/", [System.StringComparison]::Ordinal) -or
            $path.StartsWith("scripts/", [System.StringComparison]::Ordinal) -or
            $path.StartsWith(".github/", [System.StringComparison]::Ordinal)
        $entry = [ordered]@{
            path = $path
            rollback_action = if ($preserve) { "preserve" } elseif ($existsAtBaseline) { "restore" } else { "remove" }
            candidate_sha256 = $candidateHash
            git_mode = "100644"
        }
        if ($existsAtBaseline) { $entry.baseline_sha256 = Get-R7GitBlobSha256 $baselineCommit $path }
        [pscustomobject]$entry
    }
    $baselineArtifacts = @($Baseline.body.artifacts)
    [pscustomobject][ordered]@{
        schema_version = 2
        artifact_role = "rollback_manifest"
        baseline_commit = $baselineCommit
        baseline_authority_sha256 = [string](@($baselineArtifacts | Where-Object role -eq "active_authority")[0].sha256)
        baseline_production_sha256 = [string](@($baselineArtifacts | Where-Object role -eq "active_production_manifest")[0].sha256)
        changed_paths = $changedPaths
        changed_path_inventory = @($inventory)
        restore_targets = @(
            [pscustomobject][ordered]@{path = $script:R7AuthorityPath; sha256 = [string](@($baselineArtifacts | Where-Object role -eq "active_authority")[0].sha256); git_mode = "100644"},
            [pscustomobject][ordered]@{path = $script:R7ProductionPath; sha256 = [string](@($baselineArtifacts | Where-Object role -eq "active_production_manifest")[0].sha256); git_mode = "100644"}
        )
        commands = @(
            "pwsh scripts/taskspace-benchmark/test-r7-continuous-action-candidate.ps1 -CandidateId <id>",
            "pwsh scripts/taskspace-benchmark/set-r7-continuous-action-candidate-status.ps1 -CandidateId <id> -ToStatus reverted -EvidencePath <path>"
        )
    }
}

Assert-R7CleanWorktree
$toolchain = Assert-R7ToolchainWorktree
$baseline = Get-R7BaselineAnchor
Invoke-R7Git @("merge-base", "--is-ancestor", $toolchain.add_commit, "HEAD") -AllowFailure | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "R7_TOOLCHAIN_ANCHOR_NOT_ANCESTOR"
}
$sourceRoot = (Resolve-Path -LiteralPath $ArtifactSourceDirectory).Path
$schemaPath = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json"
$stageRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/candidate-stage"
[System.IO.Directory]::CreateDirectory($stageRoot) | Out-Null

$bodies = [ordered]@{}
$rawSources = [ordered]@{}
foreach ($role in @("l4_schema", "transition_schema", "typed_outcome", "carrier_protocol_oracle")) {
    $path = Join-Path $sourceRoot $script:R7ArtifactNames[$role]
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "R7_CANDIDATE_SOURCE_MISSING role=$role" }
    $body = Read-R7StrictJson $path $schemaPath
    if ([string]$body.artifact_role -cne $role) { throw "R7_CANDIDATE_SOURCE_ROLE_MISMATCH role=$role" }
    $bodies[$role] = $body
    $rawSources[$role] = $path
}

$closurePath = Join-Path $stageRoot "entry-closure.json"
& cargo run --locked -q -p codex-tools --bin r7_carrier_entry_closure --manifest-path (Join-Path $script:R7RepoRoot "third_party/codex-cli/codex-rs/Cargo.toml") -- --repo-root $script:R7RepoRoot --output $closurePath
if ($LASTEXITCODE -ne 0) { throw "R7_CLOSURE_GENERATION_FAILED" }
$closure = Read-R7StrictJson $closurePath $schemaPath
$bodies.entry_closure = $closure
$rawSources.entry_closure = $closurePath
$bodies.capability_matrix = [pscustomobject][ordered]@{
    schema_version = 2
    artifact_role = "capability_matrix"
    entry_closure_sha256 = Get-R7Sha256File $closurePath
    entries = @($closure.entries)
}
$evaluationPath = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"
$bodies.continuous_action_evaluation = Read-R7StrictJson $evaluationPath $schemaPath
$rawSources.continuous_action_evaluation = $evaluationPath
$bodies.rollback_manifest = New-RollbackArtifact $baseline $toolchain

$stageFiles = [ordered]@{}
foreach ($role in $script:R7ArtifactNames.Keys) {
    $path = Join-Path $stageRoot $script:R7ArtifactNames[$role]
    if ($rawSources.Contains($role)) {
        [System.IO.File]::WriteAllBytes($path, [System.IO.File]::ReadAllBytes([string]$rawSources[$role]))
    } else {
        Write-R7JsonFile $path $bodies[$role]
    }
    [void](Read-R7StrictJson $path $schemaPath)
    $stageFiles[$role] = [pscustomobject]@{path = $path; sha256 = Get-R7Sha256File $path}
}

$artifactHashValues = [pscustomobject][ordered]@{}
foreach ($role in $stageFiles.Keys) {
    $artifactHashValues | Add-Member -NotePropertyName $role -NotePropertyValue $stageFiles[$role].sha256
}
$authorityRaw = Get-R7GitBlobText $baseline.parent_commit $script:R7AuthorityPath
$productionRaw = Get-R7GitBlobText $baseline.parent_commit $script:R7ProductionPath
$authority = $authorityRaw | ConvertFrom-Json -Depth 100
$production = $productionRaw | ConvertFrom-Json -Depth 100
$identity = New-R7CandidateIdentity $baseline $toolchain $artifactHashValues $authority $production
$candidateId = Get-R7CandidateId $identity
$candidatePath = Get-R7CandidatePath $candidateId
if (Test-Path -LiteralPath $candidatePath.full) { throw "R7_CANDIDATE_ALREADY_EXISTS id=$candidateId" }
[System.IO.Directory]::CreateDirectory($candidatePath.full) | Out-Null
foreach ($role in $stageFiles.Keys) {
    [System.IO.File]::WriteAllBytes((Join-Path $candidatePath.full $script:R7ArtifactNames[$role]), [System.IO.File]::ReadAllBytes($stageFiles[$role].path))
}

$creationEvidence = [pscustomobject][ordered]@{
    schema_version = 1
    event_kind = "candidate_created"
    candidate_id = $candidateId
    baseline_anchor_first_add_commit = $baseline.add_commit
    toolchain_anchor_first_add_commit = $toolchain.add_commit
    source_head = Get-R7GitLine @("rev-parse", "HEAD")
    artifact_hashes = $artifactHashValues
}
$creationEvidencePath = Join-Path $candidatePath.full "creation-evidence.json"
Write-R7JsonFile $creationEvidencePath $creationEvidence
Invoke-R7Git @("add", "--", $candidatePath.relative) | Out-Null
Invoke-R7Git @("commit", "-m", "$CommitMessagePrefix artifacts $candidateId") | Out-Null
$candidateCommit = Get-R7GitLine @("rev-parse", "HEAD")

$artifactRefs = New-R7ArtifactReferences $candidateId $artifactHashValues
$activationTargets = New-R7ActivationTargets $candidateId $artifactRefs $authority
$promotionContract = New-R7ExpectedPromotionContract $authority $production $candidateId $artifactRefs
$manifest = [pscustomobject][ordered]@{
    schema_version = 2
    contract_id = "r7-continuous-action-candidate-$candidateId"
    contract_status = "candidate_record"
    candidate_id = $candidateId
    candidate_commit = $candidateCommit
    candidate_status = "evaluation_candidate"
    baseline_anchor = [pscustomobject][ordered]@{path = $script:R7BaselineAnchorPath; first_add_commit = $baseline.add_commit; anchored_parent_commit = $baseline.parent_commit; sha256 = Get-R7Sha256Text $baseline.raw}
    toolchain_anchor = [pscustomobject][ordered]@{path = $script:R7ToolchainAnchorPath; first_add_commit = $toolchain.add_commit; anchored_parent_commit = $toolchain.parent_commit; sha256 = Get-R7Sha256Text $toolchain.raw}
    active_authority = [pscustomobject][ordered]@{contract_id = $authority.contract_id; path = $script:R7AuthorityPath; git_commit = $baseline.parent_commit; sha256 = Get-R7Sha256Text $authorityRaw; git_mode = "100644"}
    active_production_manifest = [pscustomobject][ordered]@{contract_id = $production.contract_id; path = $script:R7ProductionPath; git_commit = $baseline.parent_commit; sha256 = Get-R7Sha256Text $productionRaw; git_mode = "100644"}
    activation_targets = $activationTargets
    artifact_hashes = $artifactRefs
    promotion = $promotionContract.promotion
    status_evidence = [pscustomobject][ordered]@{event_kind = "candidate_created"; evidence_path = "$($candidatePath.relative)/creation-evidence.json"; evidence_sha256 = Get-R7Sha256File $creationEvidencePath}
}
$manifestPath = Join-Path $candidatePath.full "manifest.json"
Write-R7JsonFile $manifestPath $manifest
$manifestSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
[void](Read-R7StrictJson $manifestPath $manifestSchema)
Invoke-R7Git @("add", "--", "$($candidatePath.relative)/manifest.json") | Out-Null
Invoke-R7Git @("commit", "-m", "$CommitMessagePrefix manifest $candidateId") | Out-Null

[pscustomobject][ordered]@{candidate_id = $candidateId; candidate_commit = $candidateCommit; manifest_commit = Get-R7GitLine @("rev-parse", "HEAD"); manifest_path = "$($candidatePath.relative)/manifest.json"} | ConvertTo-Json -Compress
