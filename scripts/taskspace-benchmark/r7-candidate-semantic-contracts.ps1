function New-AuthorityTargetFromActivation {
    param([object]$Target)
    [pscustomobject][ordered]@{
        layer = [string]$Target.authority_layer
        implementation_status = [string]$Target.implementation_status
        activation_phase = [string]$Target.activation_phase
        artifact = [string]$Target.path
        sha256 = [string]$Target.sha256
    }
}

function Get-ExpectedPromotedAuthority {
    param([object]$Candidate)
    $expected = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path) |
        ConvertFrom-Json -Depth 100
    $replacements = @{}
    foreach ($target in @($Candidate.activation_targets.L4) + @($Candidate.activation_targets.L5)) {
        $replacements[[string]$target.authority_layer] = New-AuthorityTargetFromActivation $target
    }
    $expected.selected_targets = @($expected.selected_targets | ForEach-Object {
        $layer = [string]$_.layer
        if ($replacements.ContainsKey($layer)) { $replacements[$layer] } else { $_ }
    })
    $repairId = [string]$Candidate.activation_targets.blocking_repair.id
    $repairs = @($expected.blocking_repairs | Where-Object { [string]$_.id -eq $repairId })
    Assert-Equal $repairs.Count 1 "Promoted baseline must contain exactly one designated blocking repair"
    $repairs[0].implementation_status = [string]$Candidate.activation_targets.blocking_repair.implementation_status
    $expected
}

function Get-ExpectedPromotedProduction {
    param([object]$Candidate, [string]$AuthoritySha256)
    $expected = Get-GitBlobText ([string]$Candidate.active_production_manifest.git_commit) ([string]$Candidate.active_production_manifest.path) |
        ConvertFrom-Json -Depth 100
    $expected.source_authority.sha256 = $AuthoritySha256
    $expected | Add-Member -NotePropertyName promoted_candidate_id -NotePropertyValue ([string]$Candidate.candidate_id) -Force
    $expected.activation_through = [string]$Candidate.activation_targets.activation_through
    foreach ($layerId in @("L4", "L5")) {
        $layers = @($expected.layers | Where-Object { [string]$_.id -eq $layerId })
        Assert-Equal $layers.Count 1 "Production baseline must contain exactly one $layerId layer"
        $layers[0].runtime_status = [string]$Candidate.activation_targets.production_runtime_status.$layerId
        $layers[0].selected_targets = @($Candidate.activation_targets.$layerId | ForEach-Object {
            [pscustomobject][ordered]@{
                artifact = [string]$_.path
                sha256 = [string]$_.sha256
                activation_phase = [string]$_.activation_phase
            }
        })
    }
    $expected
}

function Get-EntryClosureDigest {
    param([object]$Closure)
    $payload = [pscustomobject][ordered]@{
        source_inventory = $Closure.source_inventory
        source_hashes = $Closure.source_hashes
        entries = $Closure.entries
    }
    Get-TextSha256 ("r7-entry-closure-v1`n$(ConvertTo-CanonicalJson $payload)`n")
}

function Assert-CandidateArtifactSemantics {
    param([object]$Candidate, [hashtable]$Bodies, [string]$CandidateCommit)
    $rollback = $Bodies.rollback_manifest
    Assert-Equal ([string]$rollback.baseline_authority_sha256) ([string]$Candidate.active_authority.sha256) "Rollback authority baseline is not candidate-bound"
    Assert-Equal ([string]$rollback.baseline_production_sha256) ([string]$Candidate.active_production_manifest.sha256) "Rollback production baseline is not candidate-bound"
    $authorityRestore = @($rollback.restore_targets | Where-Object { [string]$_.target_role -eq "authority" })[0]
    $productionRestore = @($rollback.restore_targets | Where-Object { [string]$_.target_role -eq "production_manifest" })[0]
    Assert-Equal ([string]$authorityRestore.path) ([string]$Candidate.active_authority.path) "Rollback authority path is not candidate-bound"
    Assert-Equal ([string]$authorityRestore.sha256) ([string]$Candidate.active_authority.sha256) "Rollback authority hash is not candidate-bound"
    Assert-Equal ([string]$productionRestore.path) ([string]$Candidate.active_production_manifest.path) "Rollback production path is not candidate-bound"
    Assert-Equal ([string]$productionRestore.sha256) ([string]$Candidate.active_production_manifest.sha256) "Rollback production hash is not candidate-bound"

    $closure = $Bodies.entry_closure
    $matrix = $Bodies.capability_matrix
    Assert-Equal ([string]$matrix.entry_closure.path) ([string]$Candidate.artifact_hashes.entry_closure.path) "Capability matrix entry closure path drifted"
    Assert-Equal ([string]$matrix.entry_closure.sha256) ([string]$Candidate.artifact_hashes.entry_closure.sha256) "Capability matrix entry closure hash drifted"
    Assert-Equal (ConvertTo-CanonicalJson $matrix.entries) (ConvertTo-CanonicalJson $closure.entries) "Capability matrix and generated entry closure differ"
    $sourcePaths = [ordered]@{
        tool_spec = [string]$closure.source_inventory.tool_spec_source
        tool_payload = [string]$closure.source_inventory.tool_payload_source
        router = [string]$closure.source_inventory.router_source
        registry = [string]$closure.source_inventory.registry_source
        code_mode = [string]$closure.source_inventory.code_mode_source
    }
    foreach ($property in $sourcePaths.GetEnumerator()) {
        Assert-Equal ([string]$closure.source_hashes.($property.Key)) (Get-GitBlobSha256 $CandidateCommit $property.Value) "Entry closure source hash drifted: $($property.Key)"
    }
    Assert-Equal ([string]$closure.generation_digest) (Get-EntryClosureDigest $closure) "Entry closure generation digest drifted"

    $evaluation = $Bodies.continuous_action_evaluation
    $sampleIds = @($evaluation.samples.psobject.Properties.Name)
    Assert-Equal (@($sampleIds | Sort-Object -Unique).Count) $sampleIds.Count "Continuous-action sample identities are not unique"
    Assert-Equal (@($evaluation.sample_order | Sort-Object -Unique).Count) @($evaluation.sample_order).Count "Continuous-action sample order is not unique"
    Assert-Equal ((@($evaluation.sample_order | Sort-Object)) -join ",") ((@($sampleIds | Sort-Object)) -join ",") "Continuous-action sample order does not cover frozen samples"

    $fla8 = $Bodies.fla8_evaluation_v2
    Assert-Equal @($fla8.held_out_identity.sample_ids).Count ([int]$fla8.held_out_identity.sample_count) "FLA-8 held-out sample count drifted"
    Assert-Equal @($fla8.held_out_identity.sample_ids | Sort-Object -Unique).Count @($fla8.held_out_identity.sample_ids).Count "FLA-8 held-out sample identities are not unique"
}
