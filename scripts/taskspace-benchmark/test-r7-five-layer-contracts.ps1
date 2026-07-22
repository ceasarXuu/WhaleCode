param(
    [ValidateSet("FLA-0", "FLA-1", "FLA-2", "FLA-3", "FLA-3.5-Scaffold", "FLA-3.5", "FLA-4-Repair-Baseline", "FLA-4", "FLA-5-Repair-Baseline", "FLA-5", "All")]
    [string]$Phase = "All"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$authorityPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
$authoritySchemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-contract-authority-v1.schema.json"
$manifestPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$manifestSchemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/taskspace-contract-manifest-v1.schema.json"
$taskspaceBasePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md"
$standardBasePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_standard.md"
$l1Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-l1-taskspace-base-section-v2.md"
$l2Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-l2-core-protocol-v2.md"
$productionL2Path = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v2.md"
$l3Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-l3-taskspace-advanced-v1.SKILL.md"
$productionL3Path = Join-Path $repoRoot "third_party/codex-cli/codex-rs/skills/src/assets/samples/taskspace-advanced/SKILL.md"
$l4Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json"
$l5ResultPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json"

. (Join-Path $PSScriptRoot "r7-contract-test-primitives.ps1")

function Assert-CandidateStateHistory {
    param([object]$Candidate, [string]$PreviousStatus, [object]$Authority)
    $currentStatus = [string]$Candidate.candidate_status
    if ([string]::IsNullOrWhiteSpace($PreviousStatus)) {
        Assert-Equal $currentStatus "evaluation_candidate" "A new candidate must start as evaluation_candidate"
        return
    }
    if ($PreviousStatus -cne $currentStatus) {
        Assert-CandidateTransition $PreviousStatus $currentStatus $Authority
    }
}

function Assert-CandidateActivationSnapshot {
    param(
        [object]$Candidate,
        [string]$Status,
        [string]$AuthorityRaw,
        [object]$AuthorityObject,
        [object]$ProductionManifest,
        [string]$ProductionRaw = "",
        [string]$AuthorityByteHash = "",
        [string]$ProductionByteHash = ""
    )
    $authorityHash = if ([string]::IsNullOrWhiteSpace($AuthorityByteHash)) { Get-TextSha256 $AuthorityRaw } else { $AuthorityByteHash }
    $productionHash = if ([string]::IsNullOrWhiteSpace($ProductionByteHash)) { Get-TextSha256 $ProductionRaw } else { $ProductionByteHash }
    $activeSnapshotHash = [string]$Candidate.active_authority.sha256
    Assert-Equal ([string]$ProductionManifest.source_authority.sha256) $authorityHash "Production manifest does not identify the authority in the same state event"
    if (@("evaluation_candidate", "promotion_pending", "rejected", "reverted") -contains $Status) {
        Assert-Equal $authorityHash $activeSnapshotHash "Non-promoted candidate event changed the active authority"
        Assert-True ([string]::IsNullOrWhiteSpace([string]$ProductionManifest.promoted_candidate_id)) "Non-promoted candidate event retained an active candidate pointer"
        Assert-True (-not [string]::IsNullOrWhiteSpace($ProductionRaw)) "Non-promoted candidate event omitted production manifest bytes"
        Assert-Equal $productionHash ([string]$Candidate.active_production_manifest.sha256) "Non-promoted candidate event did not restore the active production manifest bytes"
        return
    }
    Assert-Equal $Status "promoted" "Unknown candidate activation status"
    $expectedAuthority = Get-ExpectedPromotedAuthority $Candidate
    $expectedProduction = Get-ExpectedPromotedProduction $Candidate $authorityHash
    Assert-Equal (ConvertTo-CanonicalJson $AuthorityObject) (ConvertTo-CanonicalJson $expectedAuthority) "Promoted authority differs from baseline plus the exact candidate delta"
    Assert-Equal (ConvertTo-CanonicalJson $ProductionManifest) (ConvertTo-CanonicalJson $expectedProduction) "Promoted production manifest differs from baseline plus the exact candidate delta"
}

function Assert-CandidateSetIntegrity {
    param([object[]]$Candidates, [object]$ProductionManifest)
    $ids = @($Candidates | ForEach-Object { [string]$_.candidate_id })
    Assert-Equal @($ids | Sort-Object -Unique).Count $ids.Count "Candidate ids must be unique"
    $activeCandidates = @($Candidates | Where-Object { @("promotion_pending", "promoted") -contains [string]$_.candidate_status })
    Assert-True ($activeCandidates.Count -le 1) "At most one candidate may be promotion_pending or promoted"
    foreach ($candidate in @($Candidates | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.superseded_by) })) {
        Assert-True (@("rejected", "reverted") -contains [string]$candidate.candidate_status) "Only terminal candidates may have superseded_by"
        Assert-True ($ids -contains [string]$candidate.superseded_by) "Candidate superseded_by does not identify a retained candidate"
        Assert-True ([string]$candidate.superseded_by -cne [string]$candidate.candidate_id) "Candidate cannot supersede itself"
        $successor = @($Candidates | Where-Object { [string]$_.candidate_id -eq [string]$candidate.superseded_by })[0]
        Assert-True ([string]$successor.candidate_status -ne "evaluation_candidate") "Evaluation-only candidate cannot supersede a terminal authority claim"
    }
    $promoted = @($Candidates | Where-Object { [string]$_.candidate_status -eq "promoted" })
    $activePointer = [string]$ProductionManifest.promoted_candidate_id
    if ($promoted.Count -eq 1) {
        Assert-Equal $activePointer ([string]$promoted[0].candidate_id) "Production active pointer does not match promoted candidate"
    } else {
        Assert-True ([string]::IsNullOrWhiteSpace($activePointer)) "Production active pointer exists without one promoted candidate"
    }
}

function Assert-CandidateTransition {
    param([string]$From, [string]$To, [object]$Authority)
    $allowed = @($Authority.candidate_status_transitions.$From)
    Assert-True ($allowed -contains $To) "Illegal candidate status transition: $From -> $To"
}

. (Join-Path $PSScriptRoot "r7-candidate-semantic-contracts.ps1")
. (Join-Path $PSScriptRoot "r7-candidate-contract-helpers.ps1")

function Test-PhaseEnabled {
    param([string]$Name)
    $Phase -eq "All" -or $Phase -eq $Name
}

$authorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
Assert-StrictJson $authorityRaw "active authority"
Assert-True ($authorityRaw | Test-Json -SchemaFile $authoritySchemaPath -ErrorAction Stop) "Authority JSON does not match its schema"
$authority = $authorityRaw | ConvertFrom-Json -Depth 50
Assert-Equal $authority.contract_id "r7-five-layer-contract-authority-v1" "Unexpected authority contract"
Assert-Equal $authority.compatibility_policy "none" "Five-layer migration must not keep compatibility paths"

foreach ($document in @($authority.governing_documents)) {
    $path = Join-Path $repoRoot ([string]$document.path)
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Governing document missing: $($document.path)"
    Assert-Equal (Get-Sha256 $path) ([string]$document.sha256) "Governing document hash drifted: $($document.path)"
}

foreach ($target in @($authority.selected_targets)) {
    $path = Join-Path $repoRoot ([string]$target.artifact)
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Selected artifact missing: $($target.artifact)"
    Assert-Equal (Get-Sha256 $path) ([string]$target.sha256) "Selected artifact hash drifted: $($target.artifact)"
}

if (Test-PhaseEnabled "FLA-0") {
    $baseline = $authority.baseline
    foreach ($entry in @($baseline.taskspace_base, $baseline.tool_schema_source, $baseline.argument_parser, $baseline.result_formatter, $baseline.projection_contract)) {
        $frozenText = Get-GitBlobText ([string]$baseline.commit) ([string]$entry.path)
        Assert-Equal (Get-TextSha256 $frozenText) ([string]$entry.sha256) "Frozen baseline hash drifted: $($entry.path)"
    }
    & git -C $repoRoot cat-file -e "$($baseline.commit)^{commit}" 2>$null
    $baselineCommitExit = $LASTEXITCODE
    Assert-True ($baselineCommitExit -eq 0) "Frozen baseline commit is unavailable"
    Write-Output "FLA-0 frozen source contracts passed."
}

if ($Phase -eq "FLA-3.5" -or $Phase -eq "All") {
    $carrierSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/taskspace_carrier.rs"))
    $controlSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs"))
    $controlWireSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args_wire.rs"))
    $transitionSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_transition_args.rs"))
    $routerSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/router.rs"))
    $carrierRuntimeSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/taskspace_carrier.rs"))
    $registrySource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/tool_registry_plan.rs"))
    $turnSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/session/turn.rs"))
    $preflightSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs"))
    Assert-True $carrierSource.Contains('taskspace_action') "Action Tool decorator does not expose taskspace_action"
    Assert-True $carrierSource.Contains('required.push(ACTION_FIELD') "Action Tool decorator does not require taskspace_action"
    Assert-True $routerSource.Contains('extract_taskspace_action') "Router does not strip the action before ordinary Tool dispatch"
    Assert-True (-not $registrySource.Contains('decorate_taskspace_carrier_tool')) "Shared Tool registry still decorates Standard schemas"
    Assert-True $turnSource.Contains('.map(codex_tools::decorate_taskspace_carrier_tool)') "TaskSpace provider visibility does not decorate action Tools"
    Assert-True $carrierRuntimeSource.Contains('session.taskspace_active().await') "Runtime action contract is not scoped to canonical TaskSpace mode"
    Assert-True $carrierRuntimeSource.Contains('replace_function_output(response, header)') "Rejected action feedback does not replace the duplicate failure body"
    foreach ($variant in @("ContinueCurrent", "InitializeMap", "BindNode", "CompleteThenContinue")) {
        Assert-True $transitionSource.Contains($variant) "Carrier parser omits action variant: $variant"
    }
    foreach ($variant in @("InitializeMap", "BindNode", "CompleteThenContinue")) {
        Assert-True (-not $controlWireSource.Contains("Action::$variant")) "Standalone control still accepts transition variant: $variant"
    }
    Assert-True (-not $controlSource.Contains('required_next_call')) "Tool schema retains required_next_call"
    Assert-True (-not $preflightSource.Contains('TASKSPACE_REQUIRED_SIBLING')) "Response preflight retains sibling enforcement"
    Assert-Equal (Get-Sha256 $productionL2Path) (Get-Sha256 $l2Path) "Production L2 bytes differ from authority artifact"
    $repair = @($authority.blocking_repairs | Where-Object id -eq "FLA-3.5-continuous-action-regression-repair")[0]
    Assert-Equal ([string]$repair.implementation_status) "active_verified" "FLA-3.5 authority is not active_verified"
    Assert-Equal @($repair.blocks).Count 0 "FLA-3.5 still blocks later phases"
    $carrierManifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$carrierManifest.activation_through) "FLA-3.5" "Production manifest has not activated FLA-3.5"
    Assert-Equal ([string](@($carrierManifest.layers | Where-Object id -eq "L4")[0].runtime_status)) "carrier_repair_active" "L4 carrier is not active"
    Write-Output "FLA-3.5 action-carried lifecycle contracts passed."
}

if ((Test-PhaseEnabled "FLA-1") -or $Phase -eq "FLA-3.5-Scaffold") {
    Assert-True (Test-Path -LiteralPath $manifestPath -PathType Leaf) "Production contract manifest is missing"
    $manifestRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
    Assert-StrictJson $manifestRaw "production manifest"
    Assert-True ($manifestRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Production manifest JSON does not match its schema"
    $manifest = $manifestRaw | ConvertFrom-Json -Depth 50
    Assert-Equal $manifest.contract_id "r7-taskspace-five-layer-production-v1" "Unexpected production manifest"
    Assert-Equal $manifest.source_authority.contract_id $authority.contract_id "Manifest authority id drifted"
    Assert-Equal $manifest.source_authority.sha256 (Get-Sha256 $authorityPath) "Manifest authority hash drifted"
    Assert-Equal @($manifest.layers).Count 5 "Production manifest must own exactly five layers"
    Assert-Equal ((@($manifest.layers | ForEach-Object { [string]$_.id } | Sort-Object)) -join ",") "L1,L2,L3,L4,L5" "Layer ids drifted"
    Assert-Equal $manifest.wire_order.deepseek_chat[0] "L1" "L1 must be first on DeepSeek wire"
    Assert-Equal $manifest.wire_order.deepseek_chat[1] "L2" "L2 must be the second logical section"
    foreach ($layer in @($manifest.layers)) {
        Assert-True (-not [string]::IsNullOrWhiteSpace([string]$layer.owner)) "Layer owner missing: $($layer.id)"
        Assert-True (-not [string]::IsNullOrWhiteSpace([string]$layer.carrier)) "Layer carrier missing: $($layer.id)"
    }

    $invalidCandidate = $manifestRaw | ConvertFrom-Json -Depth 50
    $invalidCandidate.contract_status = "candidate_record"
    $invalidCandidateJson = $invalidCandidate | ConvertTo-Json -Depth 50
    $invalidCandidateAccepted = $invalidCandidateJson | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction SilentlyContinue
    Assert-True (-not $invalidCandidateAccepted) "Candidate manifest without candidate identity was accepted"

    $validCandidate = $manifestRaw | ConvertFrom-Json -Depth 50
    $validCandidate.contract_status = "candidate_record"
    $headCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
    $validCandidate | Add-Member -NotePropertyName candidate_commit -NotePropertyValue $headCommit
    $validCandidate | Add-Member -NotePropertyName candidate_status -NotePropertyValue "evaluation_candidate"
    $activeAuthorityBlob = Get-GitBlobText $headCommit "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
    $activeAuthorityHash = Get-GitBlobSha256 $headCommit "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
    $activeAuthorityBody = $activeAuthorityBlob | ConvertFrom-Json -Depth 50
    $validCandidate | Add-Member -NotePropertyName active_authority -NotePropertyValue ([pscustomobject]@{
            contract_id = "r7-five-layer-contract-authority-v1"
            path = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
            git_commit = $headCommit
            sha256 = $activeAuthorityHash
        })
    $activeProductionPath = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
    $activeProductionBlob = Get-GitBlobText $headCommit $activeProductionPath
    $validCandidate | Add-Member -NotePropertyName active_production_manifest -NotePropertyValue ([pscustomobject]@{
            contract_id = "r7-taskspace-five-layer-production-v1"
            path = $activeProductionPath
            git_commit = $headCommit
            sha256 = (Get-GitBlobSha256 $headCommit $activeProductionPath)
        })
    $validCandidate.source_authority.sha256 = $activeAuthorityHash
    $validCandidate | Add-Member -NotePropertyName artifact_hashes -NotePropertyValue ([pscustomobject]@{
            l4_schema = [pscustomobject]@{ artifact_role = "l4_schema"; path = ""; sha256 = (Get-TextSha256 "l4_schema fixture`n") }
            transition_schema = [pscustomobject]@{ artifact_role = "transition_schema"; path = ""; sha256 = (Get-TextSha256 "transition_schema fixture`n") }
            typed_outcome = [pscustomobject]@{ artifact_role = "typed_outcome"; path = ""; sha256 = (Get-TextSha256 "typed_outcome fixture`n") }
            carrier_protocol_oracle = [pscustomobject]@{ artifact_role = "carrier_protocol_oracle"; path = ""; sha256 = (Get-TextSha256 "carrier_protocol_oracle fixture`n") }
            entry_closure = [pscustomobject]@{ artifact_role = "entry_closure"; path = ""; sha256 = (Get-TextSha256 "entry_closure fixture`n") }
            capability_matrix = [pscustomobject]@{ artifact_role = "capability_matrix"; path = ""; sha256 = (Get-TextSha256 "capability_matrix fixture`n") }
            rollback_manifest = [pscustomobject]@{ artifact_role = "rollback_manifest"; path = ""; sha256 = (Get-TextSha256 "rollback_manifest fixture`n") }
            continuous_action_evaluation = [pscustomobject]@{ artifact_role = "continuous_action_evaluation"; path = ""; sha256 = (Get-TextSha256 "continuous_action_evaluation fixture`n") }
        })
    $projectionBaseline = @($activeAuthorityBody.selected_targets | Where-Object { [string]$_.layer -eq "L5-projection" })[0]
    $lifecycleBaseline = @($activeAuthorityBody.selected_targets | Where-Object { [string]$_.layer -eq "L5-lifecycle" })[0]
    $validCandidate | Add-Member -NotePropertyName activation_targets -NotePropertyValue ([pscustomobject]@{
            activation_through = "FLA-3.5"
            authority_contract_status = "production_active_through_fla3_5_with_carrier_repair"
            production_manifest_version = "1.0.8"
            promotion_commit_paths = @()
            blocking_repair = [pscustomobject]@{ id = "FLA-3.5-continuous-action-regression-repair"; implementation_status = "active_verified" }
            production_runtime_status = [pscustomobject]@{ L4 = "carrier_repair_active"; L5 = "carrier_result_repair_active_projection_baseline" }
            L4 = @([pscustomobject]@{ artifact_role = "l4_schema"; authority_layer = "L4"; implementation_status = "active_repair_verified"; path = ""; sha256 = $validCandidate.artifact_hashes.l4_schema.sha256; activation_phase = "FLA-3.5" })
            L5 = @(
                [pscustomobject]@{ artifact_role = "typed_outcome"; authority_layer = "L5-result"; implementation_status = "active_repair_verified"; path = ""; sha256 = $validCandidate.artifact_hashes.typed_outcome.sha256; activation_phase = "FLA-3.5" },
                [pscustomobject]@{ artifact_role = "projection_baseline"; authority_layer = "L5-projection"; implementation_status = "selected_baseline"; path = $projectionBaseline.artifact; sha256 = $projectionBaseline.sha256; activation_phase = $projectionBaseline.activation_phase },
                [pscustomobject]@{ artifact_role = "lifecycle_baseline"; authority_layer = "L5-lifecycle"; implementation_status = "selected_not_implemented"; path = $lifecycleBaseline.artifact; sha256 = $lifecycleBaseline.sha256; activation_phase = $lifecycleBaseline.activation_phase }
            )
        })
    $candidateId = Get-CandidateContentId $validCandidate
    $validCandidate.contract_id = "r7-taskspace-five-layer-candidate-$candidateId"
    $validCandidate | Add-Member -NotePropertyName candidate_id -NotePropertyValue $candidateId
    $candidatePrefix = "benchmarks/taskspace/r7/candidates/$candidateId"
    $validCandidate.activation_targets.promotion_commit_paths = @(
        "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json",
        "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json",
        "$candidatePrefix/manifest.json"
    )
    $artifactFileNames = @{
        l4_schema = "l4-schema.json"
        transition_schema = "transition-schema.json"
        typed_outcome = "typed-outcome.json"
        carrier_protocol_oracle = "carrier-protocol-oracle.json"
        entry_closure = "entry-closure.json"
        capability_matrix = "capability-matrix.json"
        rollback_manifest = "rollback-manifest.json"
        continuous_action_evaluation = "continuous-action-evaluation.json"
    }
    foreach ($artifact in $validCandidate.artifact_hashes.psobject.Properties) {
        $artifact.Value.path = "$candidatePrefix/$($artifactFileNames[[string]$artifact.Name])"
    }
    $validCandidate.activation_targets.L4[0].path = $validCandidate.artifact_hashes.l4_schema.path
    $typedActivation = @($validCandidate.activation_targets.L5 | Where-Object { [string]$_.artifact_role -eq "typed_outcome" })[0]
    $typedActivation.path = $validCandidate.artifact_hashes.typed_outcome.path
    $validCandidateJson = $validCandidate | ConvertTo-Json -Depth 50
    Assert-True ($validCandidateJson | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Valid candidate manifest mode was rejected"
    Assert-CandidateManifestIntegrity $validCandidate
    Assert-CandidateSetIntegrity @($validCandidate) $manifest
    Assert-CandidateStateHistory $validCandidate "" $authority
    Assert-CandidateActivationContract $validCandidate
    Assert-CandidateHistoryMetaContract $validCandidate $manifest

    $artifactSchemaPath = Join-Path $repoRoot ([string]$authority.candidate_registry.artifact_schema)
    Assert-CandidateArtifactSchemaContract $artifactSchemaPath
    Assert-Throws { Assert-StrictJson '{"schema_version":1,"schema_version":999}' "duplicate-key fixture" } "Strict JSON accepted duplicate property names"

    $mismatchedCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $mismatchedCandidate.contract_id = "r7-taskspace-five-layer-candidate-0000000000000000000000000000000000000000"
    Assert-Throws { Assert-CandidateManifestIntegrity $mismatchedCandidate } "Candidate id/contract mismatch was accepted"

    $fakeCommitCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $fakeCommitCandidate.candidate_commit = "0000000000000000000000000000000000000000"
    Assert-Throws { Assert-CandidateManifestIntegrity $fakeCommitCandidate } "Unavailable candidate commit was accepted"

    $sourceMismatchCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $sourceMismatchCandidate.source_authority.sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
    Assert-Throws { Assert-CandidateManifestIntegrity $sourceMismatchCandidate } "Candidate source/active authority mismatch was accepted"

    $pathEscapeCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $pathEscapeCandidate.artifact_hashes.l4_schema.path = "$candidatePrefix/../escape.json"
    Assert-Throws { Assert-CandidateManifestIntegrity $pathEscapeCandidate } "Candidate artifact path escape was accepted"

    $duplicatePathCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $duplicatePathCandidate.artifact_hashes.transition_schema.path = $duplicatePathCandidate.artifact_hashes.l4_schema.path
    Assert-Throws { Assert-CandidateManifestIntegrity $duplicatePathCandidate } "Candidate artifact roles sharing one path were accepted"

    $duplicateBlobCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $duplicateBlobCandidate.artifact_hashes.transition_schema.sha256 = $duplicateBlobCandidate.artifact_hashes.l4_schema.sha256
    $duplicateBlobId = Get-CandidateContentId $duplicateBlobCandidate
    $duplicateBlobCandidate.candidate_id = $duplicateBlobId
    $duplicateBlobCandidate.contract_id = "r7-taskspace-five-layer-candidate-$duplicateBlobId"
    foreach ($artifact in $duplicateBlobCandidate.artifact_hashes.psobject.Properties) {
        $artifact.Value.path = $artifact.Value.path.Replace($candidateId, $duplicateBlobId)
    }
    foreach ($layer in @("L4", "L5")) {
        foreach ($target in @($duplicateBlobCandidate.activation_targets.$layer)) {
            $target.path = $target.path.Replace($candidateId, $duplicateBlobId)
        }
    }
    Assert-Throws { Assert-CandidateManifestIntegrity $duplicateBlobCandidate } "Candidate artifact roles sharing one blob were accepted"

    $missingArtifactCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $missingArtifactCandidate.artifact_hashes.psobject.Properties.Remove("rollback_manifest")
    $missingArtifactJson = $missingArtifactCandidate | ConvertTo-Json -Depth 50
    $missingArtifactAccepted = $missingArtifactJson | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction SilentlyContinue
    Assert-True (-not $missingArtifactAccepted) "Candidate missing a required artifact role was accepted"

    $wrongRoleCandidate = $validCandidateJson | ConvertFrom-Json -Depth 50
    $wrongRoleCandidate.artifact_hashes.l4_schema.artifact_role = "transition_schema"
    $wrongRoleJson = $wrongRoleCandidate | ConvertTo-Json -Depth 50
    $wrongRoleAccepted = $wrongRoleJson | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction SilentlyContinue
    Assert-True (-not $wrongRoleAccepted) "Candidate artifact with the wrong role marker was accepted"

    $directPromoted = $validCandidateJson | ConvertFrom-Json -Depth 50
    $directPromoted.candidate_status = "promoted"
    Assert-Throws { Assert-CandidateStateHistory $directPromoted "" $authority } "A new directly promoted candidate was accepted"
    $directReverted = $validCandidateJson | ConvertFrom-Json -Depth 50
    $directReverted.candidate_status = "reverted"
    Assert-Throws { Assert-CandidateStateHistory $directReverted "" $authority } "A new directly reverted candidate was accepted"

    $promotedWithoutAuthority = $validCandidateJson | ConvertFrom-Json -Depth 50
    $promotedWithoutAuthority.candidate_status = "promoted"
    $productionWithPointer = $manifestRaw | ConvertFrom-Json -Depth 50
    $productionWithPointer | Add-Member -NotePropertyName promoted_candidate_id -NotePropertyValue $candidateId
    $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
    Assert-Throws { Assert-CandidateActivationSnapshot $promotedWithoutAuthority "promoted" $currentAuthorityRaw $authority $productionWithPointer $manifestRaw } "Promoted candidate without authority cutover was accepted"

    $revertedWithoutBaseline = $validCandidateJson | ConvertFrom-Json -Depth 50
    $revertedWithoutBaseline.candidate_status = "reverted"
    $revertedWithoutBaseline.active_authority.sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
    Assert-Throws { Assert-CandidateActivationSnapshot $revertedWithoutBaseline "reverted" $currentAuthorityRaw $authority $manifest $manifestRaw } "Reverted candidate without baseline restoration was accepted"

    $orphanPointerManifest = $manifestRaw | ConvertFrom-Json -Depth 50
    $orphanPointerManifest | Add-Member -NotePropertyName promoted_candidate_id -NotePropertyValue $candidateId
    Assert-Throws { Assert-CandidateSetIntegrity @() $orphanPointerManifest } "Production pointer without candidate directory was accepted"

    $promotedA = $validCandidateJson | ConvertFrom-Json -Depth 50
    $promotedA.candidate_status = "promoted"
    $promotedB = $validCandidateJson | ConvertFrom-Json -Depth 50
    $parentCommit = (& git -C $repoRoot rev-parse HEAD^1).Trim()
    $promotedB.candidate_id = $parentCommit
    $promotedB.candidate_commit = $parentCommit
    $promotedB.contract_id = "r7-taskspace-five-layer-candidate-$parentCommit"
    $promotedB.candidate_status = "promoted"
    Assert-Throws { Assert-CandidateSetIntegrity @($promotedA, $promotedB) $manifest } "Duplicate promoted candidates were accepted"
    Assert-CandidateTransition "evaluation_candidate" "promotion_pending" $authority
    Assert-CandidateTransition "promoted" "reverted" $authority
    Assert-Throws { Assert-CandidateTransition "promoted" "rejected" $authority } "Illegal promoted-to-rejected transition was accepted"

    $candidateRoot = Join-Path $repoRoot ([string]$authority.candidate_registry.root)
    $candidateManifests = @()
    $historicalCandidatePaths = @(& git -C $repoRoot log --first-parent --name-only --format= -- ([string]$authority.candidate_registry.root) |
            Where-Object { $_ -match "/manifest\.json$" } | Sort-Object -Unique)
    foreach ($historicalCandidatePath in $historicalCandidatePaths) {
        Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot $historicalCandidatePath) -PathType Leaf) "Candidate manifest history must not be deleted: $historicalCandidatePath"
    }
    if (Test-Path -LiteralPath $candidateRoot -PathType Container) {
        foreach ($candidateFile in @(Get-ChildItem -LiteralPath $candidateRoot -Recurse -File -Filter "manifest.json")) {
            $candidateRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $candidateFile.FullName
            Assert-StrictJson $candidateRaw "candidate manifest $($candidateFile.FullName)"
            Assert-True ($candidateRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Candidate manifest does not match schema: $($candidateFile.FullName)"
            $candidate = $candidateRaw | ConvertFrom-Json -Depth 50
            Assert-CandidateManifestIntegrity $candidate $candidateFile.FullName ((& git -C $repoRoot rev-parse HEAD).Trim()) $true
            Assert-CandidateHistoryIntegrity $candidateFile.FullName $candidateRaw $authority
            $candidateManifests += $candidate
        }
    }
    Assert-CandidateSetIntegrity $candidateManifests $manifest

    $contextModule = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/taskspace_contract.rs")
    $traceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs")
    Assert-True $contextModule.Contains("taskspace_contract_manifest_v1.json") "Context module does not own the production manifest"
    Assert-True $traceSource.Contains("taskspace_contract_manifest_identity") "Provider wire trace lacks manifest identity"
    if ($Phase -eq "FLA-3.5-Scaffold") {
        Write-Output "FLA-3.5 plan scaffold contracts passed; this is not a completion gate."
    } else {
        Write-Output "FLA-1 ownership and observability contracts passed."
    }
}

if (Test-PhaseEnabled "FLA-2") {
    Assert-True (Test-Path -LiteralPath $productionL2Path -PathType Leaf) "Production L2 artifact is missing"
    Assert-Equal (Get-Sha256 $productionL2Path) (Get-Sha256 $l2Path) "Production L2 bytes differ from authority artifact"

    $taskspaceBase = [System.IO.File]::ReadAllText($taskspaceBasePath)
    $standardBase = [System.IO.File]::ReadAllText($standardBasePath)
    $l1 = [System.IO.File]::ReadAllText($l1Path)
    $l2 = [System.IO.File]::ReadAllText($l2Path)
    $l1Start = $taskspaceBase.IndexOf("## TaskSpace work map", [System.StringComparison]::Ordinal)
    $l1End = $taskspaceBase.IndexOf("## Task execution", $l1Start, [System.StringComparison]::Ordinal)
    Assert-True ($l1Start -ge 0 -and $l1End -gt $l1Start) "TaskSpace L1 section boundaries are missing"
    $actualL1 = $taskspaceBase.Substring($l1Start, $l1End - $l1Start).TrimEnd("`r", "`n") + "`n"
    Assert-Equal $actualL1 $l1 "Production L1 section differs from authority artifact"
    Assert-Equal ([regex]::Matches($taskspaceBase, [regex]::Escape($l1)).Count) 1 "TaskSpace base must contain L1 exactly once"
    Assert-Equal ([regex]::Matches($taskspaceBase, [regex]::Escape($l2)).Count) 0 "L2 must not be embedded in TaskSpace base"
    Assert-Equal ([regex]::Matches($standardBase, "TaskSpace work map|taskspace_core_protocol").Count) 0 "Standard base contains TaskSpace content"
    foreach ($fragment in @('*** Begin Patch', '*** Update File:', '{"command"', '{"input"', '"arguments"')) {
        Assert-True (-not $standardBase.Contains($fragment)) "Standard Base embeds L4 Tool wire syntax: $fragment"
        Assert-True (-not $taskspaceBase.Contains($fragment)) "TaskSpace Base embeds L4 Tool wire syntax: $fragment"
    }

    $sessionSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/session/mod.rs")
    $traceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs")
    Assert-True $sessionSource.Contains("taskspace_core_protocol(map_runtime_mode)") "Session does not select L2 from runtime mode"
    Assert-True $sessionSource.Contains("developer_sections.push(core_protocol.to_string())") "L2 is not prepended to the stable developer bundle"
    Assert-True $traceSource.Contains("taskspace_core_protocol_identity") "Provider wire trace lacks L2 identity"

    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-True (@("FLA-2", "FLA-3", "FLA-3.5") -contains [string]$manifest.activation_through) "Production manifest regressed below FLA-2"
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L1")[0].runtime_status)) "active" "L1 is not active"
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L2")[0].runtime_status)) "active" "L2 is not active"
    Write-Output "FLA-2 L1/L2 production contracts passed."
}

if (Test-PhaseEnabled "FLA-3") {
    $l3Target = @($authority.selected_targets | Where-Object layer -eq "L3")[0]
    Assert-Equal ([string]$l3Target.implementation_status) "active_verified" "L3 activation status drifted"
    Assert-True (Test-Path -LiteralPath $productionL3Path -PathType Leaf) "Production L3 Skill is missing"
    Assert-Equal (Get-Sha256 $productionL3Path) (Get-Sha256 $l3Path) "Production L3 bytes differ from authority artifact"

    $skillsSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/skills/src/lib.rs"))
    $taskspaceSkillSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/taskspace_skill.rs"))
    $turnContextSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/session/turn_context.rs"))
    $protocolSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/protocol.rs"))
    Assert-True $skillsSource.Contains('TASKSPACE_ADVANCED_SKILL_VERSION: &str = "1.0.0"') "Production L3 version identity drifted"
    Assert-True $skillsSource.Contains('SYSTEM_SKILLS_SNAPSHOTS_DIR_NAME') "Production L3 lacks immutable snapshot storage"
    Assert-True $taskspaceSkillSource.Contains('TASKSPACE_SKILL_SNAPSHOT_MISSING') "Production L3 lacks factual missing-snapshot failure"
    Assert-True $taskspaceSkillSource.Contains('taskspace_active: bool') "L3 catalog binding is not gated by runtime mode"
    Assert-True $turnContextSource.Contains('taskspace_active,') "Turn Skill catalog does not pass runtime activation state"
    Assert-True $protocolSource.Contains('taskspace_skill_snapshot: Option<TaskSpaceSkillSnapshotIdentity>') "Session metadata does not persist the L3 identity"

    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-True (@("FLA-3", "FLA-3.5") -contains [string]$manifest.activation_through) "Production manifest has not activated FLA-3"
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L3")[0].runtime_status)) "active" "L3 is not active"
    Write-Output "FLA-3 advanced Skill lifecycle contracts passed."
}

if ($Phase -eq "FLA-4") {
    throw "FLA-4 has not been implemented; use FLA-4-Repair-Baseline to inspect the active FLA-3.5 carrier baseline"
}

if ($Phase -eq "All" -or $Phase -eq "FLA-4-Repair-Baseline") {
    $l4Target = @($authority.selected_targets | Where-Object layer -eq "L4")[0]
    Assert-Equal ([string]$l4Target.implementation_status) "active_repair_verified" "L4 repair activation status drifted"
    $selectedSchema = Get-Content -Raw -Encoding UTF8 -LiteralPath $l4Path | ConvertFrom-Json -Depth 50
    $selectedActions = @($selectedSchema.provider_tool.function.parameters.anyOf | ForEach-Object { [string]$_.properties.action.enum[0] })
    foreach ($action in @("block_node", "unblock_node", "rework_node")) {
        Assert-True ($selectedActions -contains $action) "Selected L4 schema omits direct action: $action"
    }
    Assert-True ($selectedActions -notcontains "bind_node") "Selected L4 control schema still exposes standalone bind_node"
    Assert-True ($selectedActions -notcontains "transition_node") "Selected L4 schema retains transition_node"

    $toolSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs"))
    $wireSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args_wire.rs"))
    Assert-True (-not $toolSource.Contains('"transition_node"')) "Provider Tool still exposes transition_node"
    Assert-True (-not $wireSource.Contains('TransitionNode')) "Argument wire still accepts transition_node"
    foreach ($action in @("block_node", "unblock_node", "rework_node")) {
        Assert-True ($toolSource.Contains('"' + $action + '"')) "Provider Tool source omits direct action: $action"
    }
    foreach ($variant in @("BlockNode", "UnblockNode", "ReworkNode")) {
        Assert-True $wireSource.Contains("Action::$variant") "Argument wire omits direct action variant: $variant"
    }
    $carrierTransitionSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_transition_args.rs"))
    Assert-True $carrierTransitionSource.Contains("ContinueCurrent") "Carrier action parser omits continue_current"
    Assert-True $carrierTransitionSource.Contains("BindNode") "Carrier action parser omits bind_node"
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L4")[0].runtime_status)) "carrier_repair_active" "Production manifest does not expose the L4 carrier repair"
    Write-Output "FLA-4 carrier repair baseline contracts passed; FLA-4 formalization remains pending."
}

if ($Phase -eq "FLA-5") {
    throw "FLA-5 is blocked until FLA-4 completion and its own executable conformance evidence are active; use FLA-5-Repair-Baseline only to inspect the current baseline"
}

if ($Phase -eq "All" -or $Phase -eq "FLA-5-Repair-Baseline") {
    $l5Target = @($authority.selected_targets | Where-Object layer -eq "L5-result")[0]
    Assert-Equal ([string]$l5Target.implementation_status) "active_repair_verified" "L5 result repair activation status drifted"
    $resultSchema = Get-Content -Raw -Encoding UTF8 -LiteralPath $l5ResultPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$resultSchema.properties.schema_version.const) "TaskSpaceControlResultV2" "Selected result schema version drifted"
    Assert-Equal ([bool]$resultSchema.properties.partial_commit.const) $false "partial_commit must remain false"

    $argsSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs"))
    $outputSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs"))
    $preflightSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs"))
    Assert-True $argsSource.Contains('TaskSpaceControlResultV2') "Production result version is not V2"
    Assert-True $outputSource.Contains('"partial_commit": false') "Production result formatter does not emit boolean partial_commit=false"
    Assert-True (-not $preflightSource.Contains('TASKSPACE_REQUIRED_SIBLING_MISSING')) "Control preflight retains the removed sibling error"
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L5")[0].runtime_status)) "carrier_result_repair_active_projection_baseline" "Production manifest does not expose the carrier-aware L5 result repair"
    Write-Output "FLA-5 result repair baseline contracts passed; FLA-5 completion remains blocked."
}

Write-Output "R7 five-layer contract validation passed for $Phase."
