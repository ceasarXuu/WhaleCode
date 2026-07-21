function Get-R7UniqueIndex {
    param([object[]]$Items, [scriptblock]$Predicate, [string]$Label)
    $matches = [System.Collections.Generic.List[int]]::new()
    for ($index = 0; $index -lt $Items.Count; $index++) {
        if (& $Predicate $Items[$index]) { $matches.Add($index) }
    }
    if ($matches.Count -ne 1) {
        throw "R7_CANDIDATE_INDEX_NOT_UNIQUE label=$Label count=$($matches.Count)"
    }
    $matches[0]
}

function New-R7ArtifactReference {
    param([string]$Role, [string]$CandidateId, [string]$Hash)
    [pscustomobject][ordered]@{
        artifact_role = $Role
        path = "$script:R7CandidateRoot/$CandidateId/$($script:R7ArtifactNames[$Role])"
        sha256 = $Hash
        git_mode = "100644"
    }
}

function New-R7ArtifactReferences {
    param([string]$CandidateId, $ArtifactHashes)
    $references = [pscustomobject][ordered]@{}
    foreach ($role in $script:R7ArtifactNames.Keys) {
        $hash = if ($ArtifactHashes -is [System.Collections.IDictionary]) {
            [string]$ArtifactHashes[$role]
        } else {
            [string]$ArtifactHashes.$role
        }
        $references | Add-Member -NotePropertyName $role -NotePropertyValue (
            New-R7ArtifactReference $role $CandidateId $hash
        )
    }
    $references
}

function New-R7ActivationTargets {
    param([string]$CandidateId, $Artifacts, $Authority)
    $projection = @($Authority.selected_targets | Where-Object layer -eq "L5-projection")
    $lifecycle = @($Authority.selected_targets | Where-Object layer -eq "L5-lifecycle")
    if ($projection.Count -ne 1 -or $lifecycle.Count -ne 1) {
        throw "R7_ACTIVATION_BASELINE_TARGET_NOT_UNIQUE"
    }
    [pscustomobject][ordered]@{
        activation_through = "FLA-3.5"
        authority_contract_status = "production_active_through_fla3_5_with_carrier_repair"
        production_manifest_version = "1.0.5"
        blocking_repair = [pscustomobject][ordered]@{
            id = "FLA-3.5-continuous-action-regression-repair"
            implementation_status = "active_verified"
        }
        production_runtime_status = [pscustomobject][ordered]@{
            L4 = "carrier_repair_active"
            L5 = "carrier_result_repair_active_projection_baseline"
        }
        L4 = @([pscustomobject][ordered]@{
            artifact_role = "l4_schema"
            authority_layer = "L4"
            implementation_status = "active_repair_verified"
            path = $Artifacts.l4_schema.path
            sha256 = $Artifacts.l4_schema.sha256
            activation_phase = "FLA-3.5"
        })
        L5 = @(
            [pscustomobject][ordered]@{
                artifact_role = "typed_outcome"
                authority_layer = "L5-result"
                implementation_status = "active_repair_verified"
                path = $Artifacts.typed_outcome.path
                sha256 = $Artifacts.typed_outcome.sha256
                activation_phase = "FLA-3.5"
            },
            [pscustomobject][ordered]@{
                artifact_role = "projection_baseline"
                authority_layer = "L5-projection"
                implementation_status = $projection[0].implementation_status
                path = $projection[0].artifact
                sha256 = $projection[0].sha256
                activation_phase = $projection[0].activation_phase
            },
            [pscustomobject][ordered]@{
                artifact_role = "lifecycle_baseline"
                authority_layer = "L5-lifecycle"
                implementation_status = $lifecycle[0].implementation_status
                path = $lifecycle[0].artifact
                sha256 = $lifecycle[0].sha256
                activation_phase = $lifecycle[0].activation_phase
            }
        )
    }
}

function New-R7ExpectedPromotionContract {
    param($Authority, $Production, [string]$CandidateId, $Artifacts)
    $authorityOperations = [System.Collections.Generic.List[object]]::new()
    $repairIndex = Get-R7UniqueIndex @($Authority.blocking_repairs) {
        param($item) [string]$item.id -eq "FLA-3.5-continuous-action-regression-repair"
    } "blocking repair"
    $l4Index = Get-R7UniqueIndex @($Authority.selected_targets) {
        param($item) [string]$item.layer -eq "L4"
    } "authority L4"
    $l5Index = Get-R7UniqueIndex @($Authority.selected_targets) {
        param($item) [string]$item.layer -eq "L5-result"
    } "authority L5-result"
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
    $productionL4 = Get-R7UniqueIndex @($Production.layers) {
        param($item) [string]$item.id -eq "L4"
    } "production L4"
    $productionL5 = Get-R7UniqueIndex @($Production.layers) {
        param($item) [string]$item.id -eq "L5"
    } "production L5"
    $resultTarget = Get-R7UniqueIndex @($Production.layers[$productionL5].selected_targets) {
        param($item) [string]$item.artifact -eq "benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json"
    } "production L5-result"
    foreach ($change in @(
        @("replace", "/manifest_version", $Production.manifest_version, "1.0.5"),
        @("replace", "/activation_through", $Production.activation_through, "FLA-3.5"),
        @("replace", "/source_authority/sha256", $Production.source_authority.sha256, $expectedAuthorityHash),
        @("add", "/promoted_candidate_id", $null, $CandidateId),
        @("replace", "/layers/$productionL4/runtime_status", $Production.layers[$productionL4].runtime_status, "carrier_repair_active"),
        @("replace", "/layers/$productionL4/selected_targets/0/artifact", $Production.layers[$productionL4].selected_targets[0].artifact, $Artifacts.l4_schema.path),
        @("replace", "/layers/$productionL4/selected_targets/0/sha256", $Production.layers[$productionL4].selected_targets[0].sha256, $Artifacts.l4_schema.sha256),
        @("replace", "/layers/$productionL4/selected_targets/0/activation_phase", $Production.layers[$productionL4].selected_targets[0].activation_phase, "FLA-3.5"),
        @("replace", "/layers/$productionL5/runtime_status", $Production.layers[$productionL5].runtime_status, "carrier_result_repair_active_projection_baseline"),
        @("replace", "/layers/$productionL5/selected_targets/$resultTarget/artifact", $Production.layers[$productionL5].selected_targets[$resultTarget].artifact, $Artifacts.typed_outcome.path),
        @("replace", "/layers/$productionL5/selected_targets/$resultTarget/sha256", $Production.layers[$productionL5].selected_targets[$resultTarget].sha256, $Artifacts.typed_outcome.sha256),
        @("replace", "/layers/$productionL5/selected_targets/$resultTarget/activation_phase", $Production.layers[$productionL5].selected_targets[$resultTarget].activation_phase, "FLA-3.5")
    )) {
        $productionOperations.Add((New-R7PatchOperation $change[0] $change[1] $change[2] $change[3]))
    }
    $expectedProduction = Invoke-R7JsonPatch $Production $productionOperations.ToArray()
    [pscustomobject][ordered]@{
        promotion = [pscustomobject][ordered]@{
            changed_paths = @(
                $script:R7AuthorityPath,
                $script:R7ProductionPath,
                "$script:R7CandidateRoot/$CandidateId/manifest.json"
            )
            authority_patch = $authorityOperations.ToArray()
            production_patch = $productionOperations.ToArray()
            candidate_patch = @(
                New-R7PatchOperation "replace" "/candidate_status" "promotion_pending" "promoted"
            )
        }
        expected_authority = $expectedAuthority
        expected_production = $expectedProduction
    }
}

function New-R7CandidateIdentity {
    param($Baseline, $Toolchain, $ArtifactHashes, $Authority, $Production)
    $placeholder = "f" * 64
    $references = New-R7ArtifactReferences $placeholder $ArtifactHashes
    $contract = New-R7ExpectedPromotionContract $Authority $Production $placeholder $references
    [pscustomobject][ordered]@{
        baseline_anchor_sha256 = Get-R7Sha256Text $Baseline.raw
        baseline_parent = $Baseline.parent_commit
        toolchain_anchor_sha256 = Get-R7Sha256Text $Toolchain.raw
        toolchain_parent = $Toolchain.parent_commit
        artifact_hashes = $ArtifactHashes
        activation_targets_template = New-R7ActivationTargets $placeholder $references $Authority
        promotion_template = $contract.promotion
    }
}
