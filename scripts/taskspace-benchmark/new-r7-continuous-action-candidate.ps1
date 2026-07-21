param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactSourceDirectory,
    [string]$CommitMessagePrefix = "test(r7): add continuous action candidate"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Get-UniqueIndex {
    param([object[]]$Items, [scriptblock]$Predicate, [string]$Label)
    $matches = @()
    for ($index = 0; $index -lt $Items.Count; $index++) {
        if (& $Predicate $Items[$index]) { $matches += $index }
    }
    if ($matches.Count -ne 1) { throw "R7_CANDIDATE_INDEX_NOT_UNIQUE label=$Label count=$($matches.Count)" }
    [int]$matches[0]
}

function New-ArtifactReference {
    param([string]$Role, [string]$CandidateId, [string]$Hash)
    [pscustomobject][ordered]@{
        artifact_role = $Role
        path = "$script:R7CandidateRoot/$CandidateId/$($script:R7ArtifactNames[$Role])"
        sha256 = $Hash
        git_mode = "100644"
    }
}

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

function New-PromotionContract {
    param($Authority, $Production, [string]$CandidateId, $Artifacts)
    $authorityOperations = [System.Collections.Generic.List[object]]::new()
    $repairIndex = Get-UniqueIndex @($Authority.blocking_repairs) { param($item) [string]$item.id -eq "FLA-3.5-continuous-action-regression-repair" } "blocking repair"
    $l4Index = Get-UniqueIndex @($Authority.selected_targets) { param($item) [string]$item.layer -eq "L4" } "authority L4"
    $l5Index = Get-UniqueIndex @($Authority.selected_targets) { param($item) [string]$item.layer -eq "L5-result" } "authority L5-result"
    $authorityOperations.Add((New-R7PatchOperation "replace" "/contract_status" $Authority.contract_status "production_active_through_fla3_5_with_carrier_repair"))
    $authorityOperations.Add((New-R7PatchOperation "replace" "/blocking_repairs/$repairIndex/implementation_status" $Authority.blocking_repairs[$repairIndex].implementation_status "active_verified"))
    foreach ($change in @(
        @($l4Index, "activation_phase", $Authority.selected_targets[$l4Index].activation_phase, "FLA-3.5"),
        @($l4Index, "artifact", $Authority.selected_targets[$l4Index].artifact, $Artifacts.l4_schema.path),
        @($l4Index, "sha256", $Authority.selected_targets[$l4Index].sha256, $Artifacts.l4_schema.sha256),
        @($l5Index, "activation_phase", $Authority.selected_targets[$l5Index].activation_phase, "FLA-3.5"),
        @($l5Index, "artifact", $Authority.selected_targets[$l5Index].artifact, $Artifacts.typed_outcome.path),
        @($l5Index, "sha256", $Authority.selected_targets[$l5Index].sha256, $Artifacts.typed_outcome.sha256)
    )) {
        $authorityOperations.Add((New-R7PatchOperation "replace" "/selected_targets/$($change[0])/$($change[1])" $change[2] $change[3]))
    }
    if ($null -ne $Authority.selected_targets[$l4Index].psobject.Properties["required_next_call"]) {
        $authorityOperations.Add((New-R7PatchOperation "remove" "/selected_targets/$l4Index/required_next_call" $Authority.selected_targets[$l4Index].required_next_call $null))
    }
    $expectedAuthority = Invoke-R7JsonPatch $Authority $authorityOperations.ToArray()
    $scratchAuthority = Join-Path $script:R7RepoRoot "target/r7-toolchain/expected-authority-$CandidateId.json"
    [System.IO.Directory]::CreateDirectory((Split-Path $scratchAuthority -Parent)) | Out-Null
    Write-R7JsonFile $scratchAuthority $expectedAuthority
    $expectedAuthorityHash = Get-R7Sha256File $scratchAuthority

    $productionOperations = [System.Collections.Generic.List[object]]::new()
    $productionL4 = Get-UniqueIndex @($Production.layers) { param($item) [string]$item.id -eq "L4" } "production L4"
    $productionL5 = Get-UniqueIndex @($Production.layers) { param($item) [string]$item.id -eq "L5" } "production L5"
    $resultTarget = Get-UniqueIndex @($Production.layers[$productionL5].selected_targets) { param($item) [string]$item.artifact -eq "benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json" } "production L5-result"
    foreach ($operation in @(
        New-R7PatchOperation "replace" "/manifest_version" $Production.manifest_version "1.0.5",
        New-R7PatchOperation "replace" "/activation_through" $Production.activation_through "FLA-3.5",
        New-R7PatchOperation "replace" "/source_authority/sha256" $Production.source_authority.sha256 $expectedAuthorityHash,
        New-R7PatchOperation "add" "/promoted_candidate_id" $null $CandidateId,
        New-R7PatchOperation "replace" "/layers/$productionL4/runtime_status" $Production.layers[$productionL4].runtime_status "carrier_repair_active",
        New-R7PatchOperation "replace" "/layers/$productionL4/selected_targets/0/artifact" $Production.layers[$productionL4].selected_targets[0].artifact $Artifacts.l4_schema.path,
        New-R7PatchOperation "replace" "/layers/$productionL4/selected_targets/0/sha256" $Production.layers[$productionL4].selected_targets[0].sha256 $Artifacts.l4_schema.sha256,
        New-R7PatchOperation "replace" "/layers/$productionL4/selected_targets/0/activation_phase" $Production.layers[$productionL4].selected_targets[0].activation_phase "FLA-3.5",
        New-R7PatchOperation "replace" "/layers/$productionL5/runtime_status" $Production.layers[$productionL5].runtime_status "carrier_result_repair_active_projection_baseline",
        New-R7PatchOperation "replace" "/layers/$productionL5/selected_targets/$resultTarget/artifact" $Production.layers[$productionL5].selected_targets[$resultTarget].artifact $Artifacts.typed_outcome.path,
        New-R7PatchOperation "replace" "/layers/$productionL5/selected_targets/$resultTarget/sha256" $Production.layers[$productionL5].selected_targets[$resultTarget].sha256 $Artifacts.typed_outcome.sha256,
        New-R7PatchOperation "replace" "/layers/$productionL5/selected_targets/$resultTarget/activation_phase" $Production.layers[$productionL5].selected_targets[$resultTarget].activation_phase "FLA-3.5"
    )) { $productionOperations.Add($operation) }
    [pscustomobject][ordered]@{
        changed_paths = @($script:R7AuthorityPath, $script:R7ProductionPath, "$script:R7CandidateRoot/$CandidateId/manifest.json")
        authority_patch = $authorityOperations.ToArray()
        production_patch = $productionOperations.ToArray()
        candidate_patch = @(
            New-R7PatchOperation "replace" "/candidate_status" "promotion_pending" "promoted"
        )
    }
}

Assert-R7CleanWorktree
$toolchain = Assert-R7ToolchainWorktree
$baseline = Get-R7FirstAddAnchor $script:R7BaselineAnchorPath "continuous_action_production_baseline"
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

$identity = [pscustomobject][ordered]@{
    baseline_anchor_sha256 = Get-R7Sha256Text $baseline.raw
    baseline_parent = $baseline.parent_commit
    toolchain_anchor_sha256 = Get-R7Sha256Text $toolchain.raw
    toolchain_parent = $toolchain.parent_commit
    activation_contract = "FLA-3.5|carrier_repair|typed_outcome_repair"
    artifact_hashes = [pscustomobject][ordered]@{}
}
foreach ($role in $stageFiles.Keys) { $identity.artifact_hashes | Add-Member -NotePropertyName $role -NotePropertyValue $stageFiles[$role].sha256 }
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
    artifact_hashes = $identity.artifact_hashes
}
$creationEvidencePath = Join-Path $candidatePath.full "creation-evidence.json"
Write-R7JsonFile $creationEvidencePath $creationEvidence
Invoke-R7Git @("add", "--", $candidatePath.relative) | Out-Null
Invoke-R7Git @("commit", "-m", "$CommitMessagePrefix artifacts $candidateId") | Out-Null
$candidateCommit = Get-R7GitLine @("rev-parse", "HEAD")

$authorityRaw = Get-R7GitBlobText $baseline.parent_commit $script:R7AuthorityPath
$productionRaw = Get-R7GitBlobText $baseline.parent_commit $script:R7ProductionPath
$authority = $authorityRaw | ConvertFrom-Json -Depth 100
$production = $productionRaw | ConvertFrom-Json -Depth 100
$artifactRefs = [pscustomobject][ordered]@{}
foreach ($role in $stageFiles.Keys) { $artifactRefs | Add-Member -NotePropertyName $role -NotePropertyValue (New-ArtifactReference $role $candidateId $stageFiles[$role].sha256) }
$projection = @($authority.selected_targets | Where-Object layer -eq "L5-projection")[0]
$lifecycle = @($authority.selected_targets | Where-Object layer -eq "L5-lifecycle")[0]
$promotion = New-PromotionContract $authority $production $candidateId $artifactRefs
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
    activation_targets = [pscustomobject][ordered]@{
        activation_through = "FLA-3.5"
        authority_contract_status = "production_active_through_fla3_5_with_carrier_repair"
        production_manifest_version = "1.0.5"
        blocking_repair = [pscustomobject][ordered]@{id = "FLA-3.5-continuous-action-regression-repair"; implementation_status = "active_verified"}
        production_runtime_status = [pscustomobject][ordered]@{L4 = "carrier_repair_active"; L5 = "carrier_result_repair_active_projection_baseline"}
        L4 = @([pscustomobject][ordered]@{artifact_role = "l4_schema"; authority_layer = "L4"; implementation_status = "active_repair_verified"; path = $artifactRefs.l4_schema.path; sha256 = $artifactRefs.l4_schema.sha256; activation_phase = "FLA-3.5"})
        L5 = @(
            [pscustomobject][ordered]@{artifact_role = "typed_outcome"; authority_layer = "L5-result"; implementation_status = "active_repair_verified"; path = $artifactRefs.typed_outcome.path; sha256 = $artifactRefs.typed_outcome.sha256; activation_phase = "FLA-3.5"},
            [pscustomobject][ordered]@{artifact_role = "projection_baseline"; authority_layer = "L5-projection"; implementation_status = $projection.implementation_status; path = $projection.artifact; sha256 = $projection.sha256; activation_phase = $projection.activation_phase},
            [pscustomobject][ordered]@{artifact_role = "lifecycle_baseline"; authority_layer = "L5-lifecycle"; implementation_status = $lifecycle.implementation_status; path = $lifecycle.artifact; sha256 = $lifecycle.sha256; activation_phase = $lifecycle.activation_phase}
        )
    }
    artifact_hashes = $artifactRefs
    promotion = $promotion
    status_evidence = [pscustomobject][ordered]@{event_kind = "candidate_created"; evidence_path = "$($candidatePath.relative)/creation-evidence.json"; evidence_sha256 = Get-R7Sha256File $creationEvidencePath}
}
$manifestPath = Join-Path $candidatePath.full "manifest.json"
Write-R7JsonFile $manifestPath $manifest
$manifestSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
[void](Read-R7StrictJson $manifestPath $manifestSchema)
Invoke-R7Git @("add", "--", "$($candidatePath.relative)/manifest.json") | Out-Null
Invoke-R7Git @("commit", "-m", "$CommitMessagePrefix manifest $candidateId") | Out-Null

[pscustomobject][ordered]@{candidate_id = $candidateId; candidate_commit = $candidateCommit; manifest_commit = Get-R7GitLine @("rev-parse", "HEAD"); manifest_path = "$($candidatePath.relative)/manifest.json"} | ConvertTo-Json -Compress
