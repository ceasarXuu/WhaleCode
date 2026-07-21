param(
    [Parameter(Mandatory = $true)]
    [string]$CandidateId,
    [string]$TargetCommit = "HEAD",
    [string]$RequireStatus = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Write-GitBlobScratch {
    param([string]$Commit, [string]$Path, [string]$Label)
    $scratchRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs"
    [System.IO.Directory]::CreateDirectory($scratchRoot) | Out-Null
    $scratch = Join-Path $scratchRoot "$Commit-$Label.json"
    [System.IO.File]::WriteAllBytes($scratch, (Get-R7GitBlobBytes $Commit $Path))
    $scratch
}

function Get-CommitJson {
    param([string]$Commit, [string]$Path, [string]$Schema, [string]$Label)
    Read-R7StrictJson (Write-GitBlobScratch $Commit $Path $Label) $Schema
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Code)
    if ($Actual -cne $Expected) { throw "$Code expected=$Expected actual=$Actual" }
}

function Assert-SourceReference {
    param($Reference, [string]$Commit, [string]$Label)
    $path = [string]$Reference.path
    if ([System.IO.Path]::IsPathRooted($path) -or $path.Contains("..")) { throw "R7_SOURCE_REFERENCE_PATH_INVALID label=$Label" }
    Assert-Equal (Get-R7GitBlobSha256 $Commit $path) ([string]$Reference.sha256) "R7_SOURCE_REFERENCE_HASH label=$Label"
    $entry = (Invoke-R7Git @("ls-tree", $Commit, "--", $path)) -join "`n"
    if (-not $entry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) { throw "R7_SOURCE_REFERENCE_MODE label=$Label" }
}

function Assert-PhaseOwnership {
    param($Ownership)
    $required = @{
        carrier_transport = "FLA-3.5"
        l4_action_schema = "FLA-4"
        l5_result_conformance = "FLA-5"
        tool_capability_experiments = "FLA-6"
        lifecycle_recovery_projection = "FLA-7"
        product_milestones = "FLA-7"
        formal_evaluation_and_default = "FLA-8"
        release_closeout = "R7 Phase H"
    }
    $domains = @($Ownership.domains)
    Assert-Equal (($domains | ForEach-Object domain | Sort-Object -Unique).Count) $domains.Count "R7_PHASE_OWNER_DUPLICATE_DOMAIN"
    foreach ($entry in $required.GetEnumerator()) {
        $matches = @($domains | Where-Object { [string]$_.domain -eq $entry.Key })
        Assert-Equal $matches.Count 1 "R7_PHASE_OWNER_MISSING"
        Assert-Equal ([string]$matches[0].owner_phase) ([string]$entry.Value) "R7_PHASE_OWNER_DRIFT"
    }
    foreach ($forbidden in @($Ownership.forbidden_parallel_owners)) {
        if (@($domains | Where-Object { [string]$_.owner_phase -eq [string]$forbidden }).Count -ne 0) {
            throw "R7_PHASE_ALIAS_OWNS_DOMAIN alias=$forbidden"
        }
    }
}

function Assert-PatchSet {
    param([object[]]$Operations, [string[]]$Allowed, [string]$Label)
    $paths = @($Operations | ForEach-Object { [string]$_.path })
    Assert-Equal ($paths | Sort-Object -Unique).Count $paths.Count "R7_PATCH_DUPLICATE_PATH label=$Label"
    foreach ($path in $paths) {
        if ($Allowed -notcontains $path) { throw "R7_PATCH_PATH_FORBIDDEN label=$Label path=$path" }
        foreach ($other in $paths) {
            if ($path -cne $other -and $other.StartsWith("$path/", [System.StringComparison]::Ordinal)) {
                throw "R7_PATCH_PATH_OVERLAP label=$Label parent=$path child=$other"
            }
        }
    }
    Assert-Equal (($paths | Sort-Object) -join "`n") (($Allowed | Sort-Object) -join "`n") "R7_PATCH_SET_INCOMPLETE label=$Label"
}

function Assert-StatusHistory {
    param([string]$ManifestPath, [string]$Target, [string]$Schema, [string]$ArtifactSchema)
    $commits = @(Invoke-R7Git @("log", "--first-parent", "--reverse", "--format=%H", $Target, "--", $ManifestPath))
    if ($commits.Count -lt 1) { throw "R7_CANDIDATE_HISTORY_EMPTY" }
    $previous = ""
    $allowed = @{
        evaluation_candidate = @("promotion_pending", "rejected")
        promotion_pending = @("promoted", "rejected")
        rejected = @()
        promoted = @("reverted")
        reverted = @()
    }
    foreach ($commit in $commits) {
        $event = Get-CommitJson $commit $ManifestPath $Schema "candidate-event-$CandidateId"
        $status = [string]$event.candidate_status
        if ([string]::IsNullOrWhiteSpace($previous)) {
            Assert-Equal $status "evaluation_candidate" "R7_CANDIDATE_INITIAL_STATUS"
        } elseif ($status -cne $previous -and $allowed[$previous] -notcontains $status) {
            throw "R7_CANDIDATE_ILLEGAL_TRANSITION from=$previous to=$status commit=$commit"
        }
        $changed = @(Invoke-R7Git @("diff-tree", "--no-commit-id", "--name-only", "-r", $commit))
        if ($status -eq "promoted" -and $status -cne $previous) {
            $expected = @($event.promotion.changed_paths | Sort-Object)
            Assert-Equal (($changed | Sort-Object) -join "`n") ($expected -join "`n") "R7_PROMOTION_CHANGED_PATHS"
        }
        if ($status -eq "reverted" -and $status -cne $previous) {
            $rollbackPath = [string]$event.artifact_hashes.rollback_manifest.path
            $rollback = Get-CommitJson $commit $rollbackPath $ArtifactSchema "rollback-event-$CandidateId"
            $runtimePaths = @($rollback.changed_path_inventory | Where-Object { [string]$_.rollback_action -ne "preserve" } | ForEach-Object { [string]$_.path })
            $expected = @($script:R7AuthorityPath, $script:R7ProductionPath, $ManifestPath) + $runtimePaths | Sort-Object -Unique
            Assert-Equal (($changed | Sort-Object) -join "`n") ($expected -join "`n") "R7_REVERT_CHANGED_PATHS"
        }
        $previous = $status
    }
}

function Assert-SupersessionBinding {
    param($Candidate, [string]$ManifestPath, [string]$Target, [string]$Schema)
    if ($null -eq $Candidate.psobject.Properties["superseded_by"]) { return }
    $successorId = [string]$Candidate.superseded_by.candidate_id
    $history = @(Invoke-R7Git @("log", "--first-parent", "--reverse", "--format=%H", $Target, "--", $ManifestPath))
    $firstBindingCommit = ""
    foreach ($commit in $history) {
        $event = Get-CommitJson $commit $ManifestPath $Schema "supersession-$CandidateId"
        if ($null -ne $event.psobject.Properties["superseded_by"]) { $firstBindingCommit = $commit; break }
    }
    if ([string]::IsNullOrWhiteSpace($firstBindingCommit)) { throw "R7_SUPERSESSION_HISTORY_MISSING" }
    $successorPath = "$script:R7CandidateRoot/$successorId/manifest.json"
    $successor = Get-CommitJson $firstBindingCommit $successorPath $Schema "successor-$successorId"
    Assert-Equal ([string]$successor.candidate_status) "promotion_pending" "R7_SUPERSESSION_NOT_AT_PENDING_EVENT"
    $successorHistory = @(Invoke-R7Git @("log", "--first-parent", "--reverse", "--format=%H", $firstBindingCommit, "--", $successorPath))
    $firstPending = ""
    foreach ($commit in $successorHistory) {
        $event = Get-CommitJson $commit $successorPath $Schema "successor-event-$successorId"
        if ([string]$event.candidate_status -eq "promotion_pending") { $firstPending = $commit; break }
    }
    Assert-Equal $firstPending $firstBindingCommit "R7_SUPERSESSION_COMMIT_NOT_ATOMIC"
}

if ($CandidateId -notmatch '^[0-9a-f]{64}$') { throw "R7_CANDIDATE_ID_INVALID" }
$target = Get-R7GitLine @("rev-parse", $TargetCommit)
$toolchain = Assert-R7ToolchainWorktree
$baseline = Get-R7FirstAddAnchor $script:R7BaselineAnchorPath "continuous_action_production_baseline"
Invoke-R7Git @("merge-base", "--is-ancestor", $baseline.add_commit, $target) -AllowFailure | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_BASELINE_NOT_ANCESTOR" }
Invoke-R7Git @("merge-base", "--is-ancestor", $toolchain.add_commit, $target) -AllowFailure | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_TOOLCHAIN_NOT_ANCESTOR" }

$candidatePath = Get-R7CandidatePath $CandidateId
$manifestRelative = "$($candidatePath.relative)/manifest.json"
$manifestSchema = if ([string]::IsNullOrWhiteSpace($env:R7_CANDIDATE_SCHEMA_PATH)) { Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json" } else { $env:R7_CANDIDATE_SCHEMA_PATH }
$artifactSchema = if ([string]::IsNullOrWhiteSpace($env:R7_ARTIFACT_SCHEMA_PATH)) { Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json" } else { $env:R7_ARTIFACT_SCHEMA_PATH }
$manifest = Get-CommitJson $target $manifestRelative $manifestSchema "candidate-manifest-$CandidateId"
Assert-Equal ([string]$manifest.candidate_id) $CandidateId "R7_CANDIDATE_ID_FIELD"
Assert-Equal ([string]$manifest.contract_id) "r7-continuous-action-candidate-$CandidateId" "R7_CANDIDATE_CONTRACT_ID"
if (-not [string]::IsNullOrWhiteSpace($RequireStatus)) { Assert-Equal ([string]$manifest.candidate_status) $RequireStatus "R7_CANDIDATE_REQUIRED_STATUS" }
Assert-Equal ([string]$manifest.baseline_anchor.first_add_commit) $baseline.add_commit "R7_CANDIDATE_BASELINE_ADD"
Assert-Equal ([string]$manifest.baseline_anchor.anchored_parent_commit) $baseline.parent_commit "R7_CANDIDATE_BASELINE_PARENT"
Assert-Equal ([string]$manifest.baseline_anchor.sha256) (Get-R7Sha256Text $baseline.raw) "R7_CANDIDATE_BASELINE_HASH"
Assert-Equal ([string]$manifest.toolchain_anchor.first_add_commit) $toolchain.add_commit "R7_CANDIDATE_TOOLCHAIN_ADD"
Assert-Equal ([string]$manifest.toolchain_anchor.anchored_parent_commit) $toolchain.parent_commit "R7_CANDIDATE_TOOLCHAIN_PARENT"
Assert-Equal ([string]$manifest.toolchain_anchor.sha256) (Get-R7Sha256Text $toolchain.raw) "R7_CANDIDATE_TOOLCHAIN_HASH"
Assert-Equal ([string]$manifest.active_authority.git_commit) $baseline.parent_commit "R7_AUTHORITY_SNAPSHOT_COMMIT"
Assert-Equal ([string]$manifest.active_production_manifest.git_commit) $baseline.parent_commit "R7_PRODUCTION_SNAPSHOT_COMMIT"
Assert-Equal ([string]$manifest.active_authority.sha256) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7AuthorityPath) "R7_AUTHORITY_SNAPSHOT_HASH"
Assert-Equal ([string]$manifest.active_production_manifest.sha256) (Get-R7GitBlobSha256 $baseline.parent_commit $script:R7ProductionPath) "R7_PRODUCTION_SNAPSHOT_HASH"

$artifactProperties = @($manifest.artifact_hashes.psobject.Properties)
Assert-Equal $artifactProperties.Count $script:R7ArtifactNames.Count "R7_ARTIFACT_ROLE_COUNT"
$artifactBodies = @{}
foreach ($role in $script:R7ArtifactNames.Keys) {
    $reference = $manifest.artifact_hashes.$role
    Assert-Equal ([string]$reference.artifact_role) $role "R7_ARTIFACT_ROLE_FIELD"
    Assert-Equal ([string]$reference.path) "$($candidatePath.relative)/$($script:R7ArtifactNames[$role])" "R7_ARTIFACT_PATH"
    Assert-Equal ([string]$reference.git_mode) "100644" "R7_ARTIFACT_MODE"
    Assert-Equal (Get-R7GitBlobSha256 ([string]$manifest.candidate_commit) ([string]$reference.path)) ([string]$reference.sha256) "R7_ARTIFACT_CANDIDATE_HASH"
    Assert-Equal (Get-R7GitBlobSha256 $target ([string]$reference.path)) ([string]$reference.sha256) "R7_ARTIFACT_TARGET_HASH"
    $history = @(Invoke-R7Git @("log", "--first-parent", "--reverse", "--format=%H", $target, "--", ([string]$reference.path)))
    Assert-Equal $history.Count 1 "R7_ARTIFACT_IMMUTABILITY"
    $artifactBodies[$role] = Get-CommitJson $target ([string]$reference.path) $artifactSchema "candidate-$CandidateId-$role"
    Assert-Equal ([string]$artifactBodies[$role].artifact_role) $role "R7_ARTIFACT_CONTENT_ROLE"
}

$identityHashes = [pscustomobject][ordered]@{}
foreach ($role in $script:R7ArtifactNames.Keys) { $identityHashes | Add-Member -NotePropertyName $role -NotePropertyValue ([string]$manifest.artifact_hashes.$role.sha256) }
$identity = [pscustomobject][ordered]@{
    baseline_anchor_sha256 = Get-R7Sha256Text $baseline.raw
    baseline_parent = $baseline.parent_commit
    toolchain_anchor_sha256 = Get-R7Sha256Text $toolchain.raw
    toolchain_parent = $toolchain.parent_commit
    activation_contract = "FLA-3.5|carrier_repair|typed_outcome_repair"
    artifact_hashes = $identityHashes
}
Assert-Equal (Get-R7CandidateId $identity) $CandidateId "R7_CANDIDATE_CONTENT_ID"
Invoke-R7Git @("merge-base", "--is-ancestor", ([string]$manifest.candidate_commit), $target) -AllowFailure | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_CANDIDATE_COMMIT_NOT_ANCESTOR" }

Assert-Equal (ConvertTo-R7CanonicalJson $artifactBodies.entry_closure.entries) (ConvertTo-R7CanonicalJson $artifactBodies.capability_matrix.entries) "R7_CAPABILITY_CLOSURE_ENTRY_DRIFT"
Assert-Equal ([string]$artifactBodies.capability_matrix.entry_closure_sha256) ([string]$manifest.artifact_hashes.entry_closure.sha256) "R7_CAPABILITY_CLOSURE_HASH_DRIFT"
$closureBindings = @($artifactBodies.entry_closure.source_inventory.bindings.psobject.Properties)
$bindingValues = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($binding in $closureBindings) {
    Assert-SourceReference $binding.Value ([string]$manifest.candidate_commit) "closure-binding-$($binding.Name)"
    [void]$bindingValues.Add((ConvertTo-R7CanonicalJson $binding.Value))
}
foreach ($source in @($artifactBodies.entry_closure.source_inventory.scanned_sources.psobject.Properties)) {
    $reference = [pscustomobject]@{path = $source.Name; sha256 = [string]$source.Value}
    Assert-SourceReference $reference ([string]$manifest.candidate_commit) "closure-source-$($source.Name)"
}
foreach ($entry in @($artifactBodies.entry_closure.entries)) {
    foreach ($pipelineBinding in @($entry.pipeline.psobject.Properties)) {
        if (-not $bindingValues.Contains((ConvertTo-R7CanonicalJson $pipelineBinding.Value))) {
            throw "R7_CLOSURE_PIPELINE_BINDING_UNKNOWN entry=$($entry.tool_name) stage=$($pipelineBinding.Name)"
        }
    }
}
foreach ($carrier in @($artifactBodies.l4_schema.carrier_specs)) {
    Assert-SourceReference $carrier.parser ([string]$manifest.candidate_commit) "l4-parser-$($carrier.wire_api)-$($carrier.tool_spec)"
}
$closureScratch = Join-Path $script:R7RepoRoot "target/r7-toolchain/regenerated-$CandidateId.json"
& cargo run --locked -q -p codex-tools --bin r7_carrier_entry_closure --manifest-path (Join-Path $script:R7RepoRoot "third_party/codex-cli/codex-rs/Cargo.toml") -- --repo-root $script:R7RepoRoot --output $closureScratch
if ($LASTEXITCODE -ne 0) { throw "R7_CLOSURE_REGENERATION_FAILED" }
Assert-Equal (Get-R7Sha256File $closureScratch) ([string]$manifest.artifact_hashes.entry_closure.sha256) "R7_CLOSURE_REGENERATION_DRIFT"

$evaluation = $artifactBodies.continuous_action_evaluation
if ($null -ne $evaluation.psobject.Properties["combined_control_plus_next_rate"]) { throw "R7_OLD_METRIC_FORBIDDEN" }
$digestObject = $evaluation | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100
$digestObject.psobject.Properties.Remove("contract_digest")
$digestText = (ConvertTo-R7CanonicalJson $digestObject) + "`n"
Assert-Equal (Get-R7Sha256Text $digestText) ([string]$evaluation.contract_digest) "R7_EVALUATION_DIGEST_DRIFT"
foreach ($sample in @($evaluation.samples.psobject.Properties)) {
    Assert-Equal (Get-R7GitBlobSha256 ([string]$manifest.candidate_commit) ([string]$sample.Value.fixture_path)) ([string]$sample.Value.fixture_sha256) "R7_EVALUATION_FIXTURE_DRIFT"
}
foreach ($scenario in @($artifactBodies.carrier_protocol_oracle.scenarios)) {
    Assert-Equal (Get-R7GitBlobSha256 ([string]$manifest.candidate_commit) ([string]$scenario.fixture_path)) ([string]$scenario.fixture_sha256) "R7_ORACLE_FIXTURE_DRIFT scenario=$($scenario.id)"
}

$ownershipPath = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/r7-phase-ownership-v1.json"
$ownership = Read-R7StrictJson $ownershipPath
Assert-PhaseOwnership $ownership
$authorityAllowed = @(
    "/contract_status",
    "/blocking_repairs/0/implementation_status",
    "/selected_targets/3/activation_phase",
    "/selected_targets/3/artifact",
    "/selected_targets/3/sha256",
    "/selected_targets/3/required_next_call",
    "/selected_targets/4/activation_phase",
    "/selected_targets/4/artifact",
    "/selected_targets/4/sha256"
)
$productionAllowed = @(
    "/manifest_version",
    "/activation_through",
    "/source_authority/sha256",
    "/promoted_candidate_id",
    "/layers/3/runtime_status",
    "/layers/3/selected_targets/0/artifact",
    "/layers/3/selected_targets/0/sha256",
    "/layers/3/selected_targets/0/activation_phase",
    "/layers/4/runtime_status",
    "/layers/4/selected_targets/0/artifact",
    "/layers/4/selected_targets/0/sha256",
    "/layers/4/selected_targets/0/activation_phase"
)
Assert-PatchSet @($manifest.promotion.authority_patch) @($authorityAllowed | Sort-Object -Unique) "authority"
Assert-PatchSet @($manifest.promotion.production_patch) @($productionAllowed | Sort-Object -Unique) "production"
Assert-PatchSet @($manifest.promotion.candidate_patch) @("/candidate_status") "candidate"
Assert-Equal ((@($manifest.promotion.changed_paths | Sort-Object)) -join "`n") ((@($script:R7AuthorityPath, $script:R7ProductionPath, $manifestRelative | Sort-Object)) -join "`n") "R7_PROMOTION_ALLOWLIST"

$status = [string]$manifest.candidate_status
$authorityHash = Get-R7GitBlobSha256 $target $script:R7AuthorityPath
$productionHash = Get-R7GitBlobSha256 $target $script:R7ProductionPath
if (@("evaluation_candidate", "promotion_pending", "rejected", "reverted") -contains $status) {
    Assert-Equal $authorityHash ([string]$manifest.active_authority.sha256) "R7_NONPROMOTED_AUTHORITY_DRIFT"
    Assert-Equal $productionHash ([string]$manifest.active_production_manifest.sha256) "R7_NONPROMOTED_PRODUCTION_DRIFT"
} else {
    Assert-Equal $status "promoted" "R7_CANDIDATE_STATUS_UNKNOWN"
    $baselineAuthority = (Get-R7GitBlobText $baseline.parent_commit $script:R7AuthorityPath) | ConvertFrom-Json -Depth 100
    $baselineProduction = (Get-R7GitBlobText $baseline.parent_commit $script:R7ProductionPath) | ConvertFrom-Json -Depth 100
    $expectedAuthority = Invoke-R7JsonPatch $baselineAuthority @($manifest.promotion.authority_patch)
    $expectedProduction = Invoke-R7JsonPatch $baselineProduction @($manifest.promotion.production_patch)
    $actualAuthority = (Get-R7GitBlobText $target $script:R7AuthorityPath) | ConvertFrom-Json -Depth 100
    $actualProduction = (Get-R7GitBlobText $target $script:R7ProductionPath) | ConvertFrom-Json -Depth 100
    Assert-Equal (ConvertTo-R7CanonicalJson $actualAuthority) (ConvertTo-R7CanonicalJson $expectedAuthority) "R7_PROMOTED_AUTHORITY_DRIFT"
    Assert-Equal (ConvertTo-R7CanonicalJson $actualProduction) (ConvertTo-R7CanonicalJson $expectedProduction) "R7_PROMOTED_PRODUCTION_DRIFT"
    Assert-Equal ([string]$actualProduction.promoted_candidate_id) $CandidateId "R7_ACTIVE_POINTER_DRIFT"
}

Assert-Equal (Get-R7GitBlobSha256 $target ([string]$manifest.status_evidence.evidence_path)) ([string]$manifest.status_evidence.evidence_sha256) "R7_STATUS_EVIDENCE_DRIFT"
Assert-StatusHistory $manifestRelative $target $manifestSchema $artifactSchema
Assert-SupersessionBinding $manifest $manifestRelative $target $manifestSchema
[pscustomobject][ordered]@{valid = $true; candidate_id = $CandidateId; target_commit = $target; status = $status; artifact_count = $artifactProperties.Count; closure_entries = @($artifactBodies.entry_closure.entries).Count} | ConvertTo-Json -Compress
